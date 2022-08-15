use hyper::http::request::Builder as HttpRequestBuilder;
use hyper::{Body, Client, Request, Response, StatusCode, Uri};
use std::any::Any;
use std::collections::HashMap;

use crate::api::{DocumentOperation, LogType, LoggerStateData};
use crate::deserialize::Deserializer;
use crate::{
	utils, Error, Handler, HandlerContext, HandlerEvent, Io, Kind, MapCrateError, Result,
	SubscriptionManager,
};

/// ArangoDB HTTP Header. From the ArangoDB docs: it's the tick value of the last included value in
/// the result. In incremental log fetching, this value can be used as the from value for the
/// following request. Note that if the result is empty, the value will be `0`. This value should
/// not be used as from value by clients in the next request (otherwise the server would return the
/// log events from the start of the log again).
const LAST_LOG_HEADER: &str = "X-Arango-Replication-Lastincluded";

/// ArangoDB event trigger. This structs contains server-related information and its
/// [`Subscription`]s. Use it in order to listen to ArangoDB events.
///
/// # Examples
/// There are two ways of initializing a Trigger. With basic authentication or without it. This
/// refers to ArangoDB server authentication.
///
/// ## Authenticated ArangoDB instance
/// ```
/// use arangodb_events_rs::{Trigger, TriggerAuthentication};
///
/// let mut trigger = Trigger::new_auth(
///    	"http://localhost:8529/",
///    	"alchemy",
///    	TriggerAuthentication::new("user", "password"),
///	);
/// ```
///
/// ## Unauthenticated ArangoDB instance
/// ```
/// use arangodb_events_rs::Trigger;
///
/// let mut trigger = Trigger::new(
///    	"http://localhost:8529/",
///    	"alchemy",
///	);
/// ```
pub struct Trigger {
	host: String,
	database: String,
	auth: Option<TriggerAuthentication>,
	last_log_tick: String,
	transactions: HashMap<String, Transaction>,
	subscriptions: SubscriptionManager,
}

/// ArangoDB struct holding basic HTTP ArangoDB server authentication
pub struct TriggerAuthentication {
	user: String,
	password: String,
}

impl TriggerAuthentication {
	/// Creates a new instance of [`TriggerAuthentication`] that holds data for Basic HTTP
	/// Authentication for ArangoDB server
	///
	/// # Arguments
	///
	/// * `user`: The ArangoDB server username
	/// * `password`: The ArangoDB server password
	///
	/// returns: [`TriggerAuthentication`]
	///
	/// # Examples
	///
	/// ```
	/// use arangodb_events_rs::{Trigger, TriggerAuthentication};
	///
	/// let mut trigger = Trigger::new_auth(
	///     "http://localhost:8529/",
	///     "alchemy",
	///     TriggerAuthentication::new("user", "password"),
	/// );
	/// ```
	pub fn new(user: &str, password: &str) -> Self {
		Self {
			user: user.to_string(),
			password: password.to_string(),
		}
	}
}

impl Trigger {
	/// Creates a new [`Trigger`] instance that connects to an ArangoDB HTTP Server by the given
	/// details.
	///
	/// # Arguments
	///
	/// * `host`: The ArangoDB server instance host
	/// * `database`: The ArangoDB server instance database name
	///
	/// returns: [`Trigger`]
	///
	/// # Examples
	///
	/// ```
	/// use arangodb_events_rs::Trigger;
	///
	/// let mut trigger = Trigger::new(
	///    	"http://localhost:8529/",
	///    	"alchemy",
	///	);
	/// ```
	pub fn new(host: &str, database: &str) -> Self {
		Self {
			host: host.to_string(),
			database: database.to_string(),
			auth: None,
			last_log_tick: "0".to_string(),
			transactions: HashMap::new(),
			subscriptions: SubscriptionManager::new(),
		}
	}

	/// Creates a new [`Trigger`] instance that connects to an ArangoDB HTTP Server by the given
	/// details with HTTP Basic authentication.
	///
	/// # Arguments
	///
	/// * `host`: The ArangoDB server instance host
	/// * `database`: The ArangoDB server instance database name
	/// * `auth`: The ArangoDB HTTP basic authentication details held on a [`TriggerAuthentication`]
	/// struct
	///
	/// returns: [`Trigger`]
	///
	/// # Examples
	///
	/// ```
	/// use arangodb_events_rs::Trigger;
	///
	/// let mut trigger = Trigger::new(
	///    	"http://localhost:8529/",
	///    	"alchemy",
	///	);
	/// ```
	pub fn new_auth(host: &str, database: &str, auth: TriggerAuthentication) -> Self {
		let mut instance = Self::new(host, database);
		instance.auth = Some(auth);
		instance
	}

	/// Gets HTTP URI for the given endpoint with the host and database stored
	fn get_uri(&self, endpoint: &str) -> Result<Uri> {
		format!("{}/_db/{}{}", self.host, self.database, endpoint)
			.parse()
			.map_err(|e: hyper::http::uri::InvalidUri| e.into())
	}

	/// Retrieves `Authorization` HTTP Header value by a [`TriggerAuthentication`]
	fn get_authorization_value(&self, auth: &TriggerAuthentication) -> String {
		format!(
			"Basic {}",
			base64::encode(format!("{}:{}", auth.user, auth.password))
		)
	}

	/// Creates a [`HttpRequestBuilder`] with the given [`Uri`]
	fn get_new_request(&self, uri: Uri) -> HttpRequestBuilder {
		let mut req = Request::builder().uri(uri);

		if let Some(auth) = &self.auth {
			req = req.header(
				hyper::header::AUTHORIZATION,
				self.get_authorization_value(auth),
			);
		}

		req
	}

	/// Initializes a [`Trigger`]. This method calls **`GET /_api/replication/logger-state`**
	/// endpoint on the ArangoDB server to store the last log tick from ArangoDB Replication API on
	/// the [`Trigger`] instance to then be used on the [`listen`] method.
	///
	/// [`listen`]: #method.listen
	///
	/// returns: `Result<()>`
	///
	/// # Examples
	///
	/// ```
	/// use arangodb_events_rs::Trigger;
	///
	/// let mut trigger = Trigger::new(
	///    	"http://localhost:8529/",
	///    	"alchemy",
	///	);
	///
	/// trigger.init().await.expect("Error initializing ArangoDB event trigger");
	/// ```
	pub async fn init(&mut self) -> Result<()> {
		let client = Client::new();

		let logger_state_uri = self.get_uri("/_api/replication/logger-state")?;
		let req = self
			.get_new_request(logger_state_uri)
			.body(Body::empty())
			.map_crate_err()?;

		let response: Response<Body> = client.request(req).await.map_crate_err()?;

		match response.status() {
			StatusCode::OK => {
				let bytes = hyper::body::to_bytes(response.into_body()).await?;
				let data: LoggerStateData =
					serde_json::from_slice(bytes.as_ref()).map_crate_err()?;

				self.last_log_tick = data.state.last_log_tick;

				Ok(())
			}
			s => Err(s.into()),
		}
	}

	/// Listens to the ArangoDB Replication API calling to **`GET /_api/replication/logger-state`**
	/// giving a query variable `from` the value of the last log tick stored on the [`Trigger`]
	/// instance. This method doesn't keep listening to the server, but rather it returns whenever
	/// the response of the HTTP request is already processed. To Keep this process running use this
	/// method inside a loop and for multi-threading create a thread wrapping the loop, mind that
	/// there shouldn't be any problems with [`HandlerContext`] data as they're [`std::sync::Arc`]
	/// wrappers.
	///
	/// # Examples
	/// ```
	/// use arangodb_events_rs::Trigger;
	///
	/// let mut trigger = Trigger::new(
	///    	"http://localhost:8529/",
	///    	"alchemy",
	///	);
	///
	/// trigger.init().await.expect("Error initializing ArangoDB event trigger");
	///
	/// loop {
	/// 	// Note that as this is a user-controlled loop, this whole listening process can be
	/// 	// interrupted any time doing instead of a loop a while or any other systems
	/// 	trigger.listen().await.unwrap();
	/// }
	/// ```
	pub async fn listen(&mut self) -> Result<()> {
		let current_tick = self.last_log_tick.clone();

		let client = Client::new();

		let logger_state_uri = self.get_uri(
			format!(
				"/_api/replication/logger-follow?from={}",
				current_tick.as_str()
			)
			.as_str(),
		)?;

		let req = self
			.get_new_request(logger_state_uri)
			.body(Body::empty())
			.map_crate_err()?;

		let response: Response<Body> = client.request(req).await.map_crate_err()?;

		match response.status() {
			StatusCode::OK | StatusCode::NO_CONTENT => {
				let next_log_tick = if let Some(v) = response.headers().get(LAST_LOG_HEADER) {
					let value = v.to_str().map_crate_err()?;

					if value == "0" {
						tokio::time::sleep(std::time::Duration::from_millis(500)).await;

						return Ok(());
					} else {
						value
					}
				} else {
					current_tick.as_str()
				};

				self.last_log_tick = next_log_tick.to_string();

				// If there's no change on tick value, call again process_log_tick
				if !next_log_tick.eq(&current_tick) {
					let mut deserializer = Deserializer::new(response.into_body());

					while let Some(line) = deserializer.read_line().await? {
						self.process_line(line).await?;
					}
				}

				Ok(())
			}
			s => Err(s.into()),
		}
	}

	/// Processes one logger line
	async fn process_line(&mut self, line: String) -> Result<()> {
		// We do this kind of parsing with indexes and characters instead of serializing or
		// deserializing JSON directly using `serde_json` because it'd consume a lot of resources
		// for some operations that may not be needed to be parsed.

		// Get index after search on line
		fn find_idx(line: &str, search: &str) -> Result<usize> {
			Ok(line
				.find(search)
				.ok_or(Error::new(Kind::Io(Io::Serialize)))?
				+ search.len())
		}

		let type_idx = find_idx(line.as_str(), "\"type\":")?;

		let log_type_str: u16 = utils::get_string_between(line.as_str(), type_idx, 4)
			.parse()
			.map_crate_err()?;

		// Get transaction id
		fn get_tid(line: &str) -> Result<String> {
			let tid_idx = find_idx(line, "\"tid\":\"")?;

			Ok(utils::get_string_until(line, tid_idx, '"'))
		}

		if let Ok(log_type) = log_type_str.try_into() {
			match log_type {
				LogType::StartTransaction => {
					let tid = get_tid(line.as_str())?;

					self.transactions.insert(tid.clone(), Transaction::empty());
				}
				LogType::RemoveDocument | LogType::InsertOrReplaceDocument => {
					let tid = get_tid(line.as_str())?;

					fn create_operation(
						line: &str,
						log_type: LogType,
					) -> Result<TransactionOperation> {
						Ok(if matches!(log_type, LogType::RemoveDocument) {
							TransactionOperation::RemoveDocument(
								serde_json::from_str(line).map_crate_err()?,
							)
						} else {
							TransactionOperation::InsertOrReplaceDocument(
								serde_json::from_str(line).map_crate_err()?,
							)
						})
					}

					// The field tid might contain the value “0” to identify a single operation
					// that is not part of a multi-document transaction
					if tid == "0" {
						let single_op = create_operation(line.as_str(), log_type)?;

						self.execute_operation(&single_op).await;
					} else {
						// If the transaction's id is not 0 and it's not on already started
						// transactions we just ignore the operation as it shouldn't get parsed
						if let Some(t) = self.transactions.get_mut(tid.as_str()) {
							t.operations
								.push(create_operation(line.as_str(), log_type)?)
						}
					}
				}
				LogType::CommitTransaction => {
					let tid = get_tid(line.as_str())?;

					if let Some(t) = self.transactions.get(tid.as_str()) {
						for operation in &t.operations {
							self.execute_operation(operation).await
						}
					}
				}
				LogType::AbortTransaction => {
					let tid = get_tid(line.as_str())?;

					self.transactions.remove(tid.as_str());
				}
				_ => {}
			}
		}

		Ok(())
	}

	/// Executes a [`TransactionOperation`]
	async fn execute_operation(&self, op: &TransactionOperation) {
		match op {
			TransactionOperation::InsertOrReplaceDocument(ref doc) => {
				self.subscriptions
					.call(
						HandlerEvent::InsertOrReplace,
						doc,
						Some(doc.collection.as_str()),
					)
					.await
			}
			TransactionOperation::RemoveDocument(ref doc) => {
				self.subscriptions
					.call(HandlerEvent::Remove, doc, Some(doc.collection.as_str()))
					.await
			}
		}
	}

	/// Subscribes [`Handler`] to a [`HandlerEvent`] with a given [`HandlerContext`]
	///
	/// # Arguments
	///
	/// * `ev`: The [`HandlerEvent`] the [`Handler`] is gonna listen to
	/// * `ctx`: The [`Handler::Context`]. Note that you could pass here any [`HandlerContext`]
	/// with any type, but note that if its type it's not the same as the [`Handler::Context`] one
	/// the [`Handler::call`] method is never gonna be called as downcasting will fail
	///
	/// # Examples
	/// ```
	/// use arangodb_events_rs::api::DocumentOperation;
	/// use arangodb_events_rs::{Handler, HandlerContextFactory, HandlerEvent, Trigger};
	///
	/// pub struct ExampleHandler;
	///
	/// pub struct MyContext {
	///     pub data: u8,
	/// }
	///
	/// impl Handler for GlobalHandler {
	///     type Context = GlobalHandlerContext;
	///
	///     fn call(ctx: &GlobalHandlerContext, doc: &DocumentOperation) {
	///         println!("{}", ctx.data); // 10
	///     }
	/// }
	///
	/// let mut trigger = Trigger::new(
	/// 	"http://localhost:8529/",
	/// 	"alchemy",
	/// );
	///
	///	trigger.subscribe::<Example>(
	/// 	HandlerEvent::InsertOrReplace,
	/// 	HandlerContextFactory::from(MyContext {
	///         data: 10,
	///  	})
	/// );
	///
	///	trigger
	///		.init()
	///		.await
	///		.expect("Error initializing ArangoDB Trigger");
	///
	///  loop {
	///		trigger.listen().await.unwrap();
	///  }
	/// ```
	pub fn subscribe<H: Handler>(&mut self, event: HandlerEvent, ctx: HandlerContext<dyn Any>) {
		self.subscriptions.insert::<H>(event, ctx)
	}

	/// Subscribes [`Handler`] to a [`HandlerEvent`] with a given [`HandlerContext`] for all
	/// document operations that affects given collection name
	///
	/// # Arguments
	///
	/// * `ev`: The [`HandlerEvent`] the [`Handler`] is gonna listen to
	/// * `collection`: The ArangoDB collection name
	/// * `ctx`: The [`Handler::Context`]. Note that you could pass here any [`HandlerContext`]
	/// with any type, but note that if its type it's not the same as the [`Handler::Context`] one
	/// the [`Handler::call`] method is never gonna be called as downcasting will fail
	///
	/// # Examples
	/// ```
	/// use arangodb_events_rs::api::DocumentOperation;
	/// use arangodb_events_rs::{Handler, HandlerContextFactory, HandlerEvent, Trigger};
	///
	/// pub struct AccountHandler;
	///
	/// pub struct AccountContext {
	///     pub data: u8,
	/// }
	///
	/// impl Handler for AccountHandler {
	///     type Context = AccountContext;
	///
	///     fn call(ctx: &AccountContext, doc: &DocumentOperation) {
	///         println!("{}", ctx.data); // 10
	///     }
	/// }
	///
	/// let mut trigger = Trigger::new(
	/// 	"http://localhost:8529/",
	/// 	"alchemy",
	/// );
	///
	///	trigger.subscribe_to::<AccountHandler>(
	/// 	HandlerEvent::InsertOrReplace,
	/// 	"accounts",
	/// 	HandlerContextFactory::from(AccountContext {
	///         data: 10,
	///  	})
	/// );
	///
	///	trigger
	///		.init()
	///		.await
	///		.expect("Error initializing ArangoDB Trigger");
	///
	///  loop {
	///		trigger.listen().await.unwrap();
	///  }
	/// ```
	pub fn subscribe_to<H: Handler>(
		&mut self,
		event: HandlerEvent,
		collection: &str,
		ctx: HandlerContext<dyn Any>,
	) {
		self.subscriptions.insert_to::<H>(event, collection, ctx)
	}
}

/// Transactions are those who specify in themselves multiple [`TransactionOperation`] this is used
/// because ArangoDB works in the way that they start transactions, add operations to that
/// transaction and then abort or commit the transaction. We need to keep track of which
/// [`TransactionOperation`] belongs to which [`Transaction`] so that we can abort them or execute
/// them on commit.
pub(crate) struct Transaction {
	operations: Vec<TransactionOperation>,
}

impl Transaction {
	/// Creates a new empty [`Transaction`]
	pub(crate) fn empty() -> Self {
		Self {
			operations: Vec::new(),
		}
	}
}

/// Insert or Replace/Remove operations that can or not belong to a [`Transaction`] if they don't
/// belong to a [`Transaction`] it gets executed at the same moment it is parsed.
pub(crate) enum TransactionOperation {
	InsertOrReplaceDocument(DocumentOperation),
	RemoveDocument(DocumentOperation),
}
