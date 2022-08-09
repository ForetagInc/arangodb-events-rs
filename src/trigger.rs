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

const LAST_LOG_HEADER: &str = "X-Arango-Replication-Lastincluded";

pub struct Trigger {
	host: String,
	database: String,
	auth: Option<TriggerAuthentication>,
	last_log_tick: String,
	transactions: HashMap<String, Transaction>,
	subscriptions: SubscriptionManager,
}

pub struct TriggerAuthentication {
	user: String,
	password: String,
}

impl TriggerAuthentication {
	pub fn new(user: &str, password: &str) -> Self {
		Self {
			user: user.to_string(),
			password: password.to_string(),
		}
	}
}

impl Trigger {
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

	pub fn new_auth(host: &str, database: &str, auth: TriggerAuthentication) -> Self {
		let mut instance = Self::new(host, database);
		instance.auth = Some(auth);
		instance
	}

	fn get_uri(&self, endpoint: &str) -> Result<Uri> {
		format!("{}/_db/{}{}", self.host, self.database, endpoint)
			.parse()
			.map_err(|e: hyper::http::uri::InvalidUri| e.into())
	}

	fn get_authorization_value(&self, auth: &TriggerAuthentication) -> String {
		format!(
			"Basic {}",
			base64::encode(format!("{}:{}", auth.user, auth.password))
		)
	}

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

					println!("----------{}----------------", current_tick);

					while let Some(line) = deserializer.read_line().await? {
						self.process_line(line)?;
					}

					println!("---------------------------------")
				}

				Ok(())
			}
			s => Err(s.into()),
		}
	}

	fn process_line(&mut self, line: String) -> Result<()> {
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

					self.transactions.insert(tid.clone(), Transaction::new(tid));
				}
				LogType::RemoveDocument | LogType::InsertOrReplaceDocument => {
					let tid = get_tid(line.as_str())?;

					// TODO: Process unique operations with transaction id = 0

					// If the transaction's id is not 0 and it's not on already started transactions
					// we just ignore the operation as it shouldn't get parsed
					if let Some(t) = self.transactions.get_mut(tid.as_str()) {
						t.operations
							.push(if matches!(log_type, LogType::RemoveDocument) {
								TransactionOperation::RemoveDocument(
									serde_json::from_str(line.as_str()).map_crate_err()?,
								)
							} else {
								TransactionOperation::InsertOrReplaceDocument(
									serde_json::from_str(line.as_str()).map_crate_err()?,
								)
							})
					}
				}
				LogType::CommitTransaction => {
					for (_, transaction) in &self.transactions {
						for operation in &transaction.operations {
							self.execute_operation(operation)
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

	fn execute_operation(&self, op: &TransactionOperation) {
		match op {
			TransactionOperation::InsertOrReplaceDocument(ref doc) => {
				self.subscriptions.call(HandlerEvent::InsertOrReplace, doc)
			}
			TransactionOperation::RemoveDocument(ref doc) => {
				self.subscriptions.call(HandlerEvent::Remove, doc)
			}
		}
	}

	pub fn subscribe<H: Handler>(&mut self, event: HandlerEvent, ctx: HandlerContext<dyn Any>) {
		self.subscriptions.insert::<H>(event, ctx)
	}

	pub fn subscribe_to<H: Handler>(
		&mut self,
		event: HandlerEvent,
		collection: String,
		ctx: HandlerContext<dyn Any>,
	) {
		self.subscriptions.insert_to::<H>(event, collection, ctx)
	}
}

pub(crate) struct Transaction {
	id: String,
	operations: Vec<TransactionOperation>,
}

impl Transaction {
	pub(crate) fn new(id: String) -> Self {
		Self {
			id,
			operations: Vec::new(),
		}
	}
}

pub(crate) enum TransactionOperation {
	InsertOrReplaceDocument(DocumentOperation),
	RemoveDocument(DocumentOperation),
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	pub async fn it_inits() -> Result<()> {
		let mut trigger = Trigger::new_auth(
			"http://localhost:8529/",
			"_system",
			TriggerAuthentication::new("root", "root"),
		);

		trigger.init().await?;

		Ok(())
	}
}
