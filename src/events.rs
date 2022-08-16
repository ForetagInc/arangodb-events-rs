use std::any::Any;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use crate::api::DocumentOperation;

/// Defines the type of event the handler will be listening to
#[derive(Eq, PartialEq, Hash)]
pub enum HandlerEvent {
	/// Insert or replace document. Triggered by
	/// [`InsertOrReplaceDocument`](`crate::api::LogType::InsertOrReplaceDocument`) event
	InsertOrReplace,
	/// Insert or replace document. Triggered by
	/// [`RemoveDocument`](`crate::api::LogType::RemoveDocument`) event
	Remove,
}

/// Handler context wrapper and extractor.
///
/// Note that `HandlerContext` is cheap to clone; internally, it uses an `Arc`.
pub struct HandlerContext<T: ?Sized>(Arc<Box<T>>);

impl<T: ?Sized> HandlerContext<T> {
	/// Create new `HandlerContext` instance.
	pub(crate) fn new(data: Box<T>) -> Self {
		Self(Arc::new(data))
	}
}

impl<T: ?Sized> HandlerContext<T> {
	/// Returns reference to inner `T`.
	pub fn get_ref(&self) -> &T {
		self.0.as_ref()
	}

	/// Unwraps to the internal `Arc<T>`
	pub fn into_inner(self) -> Arc<Box<T>> {
		self.0
	}
}

impl<T: ?Sized> Deref for HandlerContext<T> {
	type Target = Arc<Box<T>>;

	fn deref(&self) -> &Arc<Box<T>> {
		&self.0
	}
}

impl<T: ?Sized> Clone for HandlerContext<T> {
	fn clone(&self) -> HandlerContext<T> {
		HandlerContext(Arc::clone(&self.0))
	}
}

/// Event handler
///
/// This trait is implemented for structs to then subscribe to a [`Trigger`](`crate::Trigger`)
/// and get its [`call`] method executed with access to the given context at subscribe.
///
/// [`call`]: Handler::call
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
/// impl Handler for ExampleHandler {
///     type Context = MyContext;
///
///     fn call(ctx: &MyContext, doc: &DocumentOperation) {
///         println!("{}", ctx.data); // 10
///     }
/// }
///
/// let mut trigger = Trigger::new(
/// 	"http://localhost:8529/",
/// 	"alchemy",
/// );
///
///	trigger.subscribe::<ExampleHandler>(
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
pub trait Handler: 'static {
	type Context;

	#[cfg(feature = "async")]
	/// Method called when the [`HandlerEvent`] the Handler is subscribed to gets dispatched from
	/// the application [`Trigger`]
	///
	/// Note: with `async` feature enabled, this method returns [`AsyncHandlerOutput`]
	fn call<'a>(ctx: &'a Self::Context, doc: &'a DocumentOperation) -> AsyncHandlerOutput<'a>;

	#[cfg(not(feature = "async"))]
	/// Method called when the [`HandlerEvent`] the Handler is subscribed to gets dispatched from
	/// the application [`Trigger`]
	fn call(ctx: &Self::Context, doc: &DocumentOperation);

	/// Dispatch the event, this method basically downcast the dynamic [`HandlerContext`] into
	/// [`HandlerContext<Self::Context>`]
	///
	/// Note: with `async` feature enabled, this method returns [`Option<AsyncHandlerOutput>`] so we
	/// encapsulate the [`Handler::call`] into a pinned box.
	fn dispatch<'a>(
		ctx: &'a HandlerContext<dyn Any>,
		doc: &'a DocumentOperation,
	) -> Option<AsyncHandlerOutput<'a>> {
		if let Some(c) = ctx.downcast_ref::<Self::Context>() {
			#[cfg(feature = "async")]
			return Some(Self::call(c, doc));

			#[cfg(not(feature = "async"))]
			return Some(Box::pin(async move { Self::call(c, doc) }));
		}

		None
	}
}

/// Type alias for [`Handler::call`] method output
pub type AsyncHandlerOutput<'a> = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'a>>;

/// Factory to create HandlerContext.
///
/// Note that this is the way to create `HandlerContext` instead of using its `new` method because
/// this helps with automatic typings as this already returns `HandlerContext<dyn Any>` instead of
/// `HandlerContext<T>` which then can lead to some typing problems while subscribing to the
/// [`Trigger`]. That is why [`HandlerContext::new`] is not exposed publicly
pub struct HandlerContextFactory;

// This factory makes it possible to return `dyn Any` type directly, to avoid needing for
// specifying type on the library use code
impl HandlerContextFactory {
	/// Creates a new `HandlerContext`
	///
	/// # Arguments
	///
	/// * `data`: The inner data to be wrapped
	///
	/// returns: `HandlerContext<dyn Any>`
	///
	/// # Examples
	///
	/// ```
	/// use arangodb_events_rs::{HandlerContextFactory, Trigger, HandlerEvent};
	///
	/// struct MyContext(pub String);
	///
	/// let mut trigger = Trigger::new(
	/// 	"http://localhost:8529/",
	/// 	"alchemy",
	/// );
	///
	///	trigger.subscribe::<ExistingHandler>(
	///		HandlerEvent::InsertOrReplace,
	///		HandlerContextFactory::from(
	///			MyContext(String::from("This is the context data"))
	/// 	)
	///	);
	/// ```
	pub fn from<T: Any>(data: T) -> HandlerContext<dyn Any> {
		HandlerContext::new(Box::new(data))
	}
}

/// Event subscription
pub(crate) struct Subscription {
	name: String,
	callback: for<'a> fn(
		&'a HandlerContext<dyn Any>,
		&'a DocumentOperation,
	) -> Option<AsyncHandlerOutput<'a>>,
	context: HandlerContext<dyn Any>,
}

/// Event subscription map
pub(crate) struct SubscriptionMap {
	map: HashMap<HandlerEvent, Vec<Subscription>>,
}

impl SubscriptionMap {
	/// Creates a new empty instance of `SubscriptionMap`
	///
	/// returns: [`SubscriptionMap`]
	pub(crate) fn empty() -> Self {
		Self {
			map: HashMap::new(),
		}
	}

	/// Inserts into the inner map an instance of [`Subscription`] with the given handler's dispatch
	/// method as the callback of the [`Subscription`]
	///
	/// # Arguments
	///
	/// * `ev`: The [`HandlerEvent`] the [`Handler`] is gonna listen to
	/// * `ctx`: The [`Handler`]'s [`HandlerContext`]. Note that you could pass here any [`HandlerContext`]
	/// with any type, but note that if its type it's not the same as the [`Handler::Context`] one the
	/// [`Handler::call`] function is never gonna be executed as downcasting will fail.
	pub(crate) fn insert<H: Handler>(&mut self, ev: HandlerEvent, ctx: HandlerContext<dyn Any>) {
		let subscription = Subscription {
			name: std::any::type_name::<H>().to_string(),
			callback: H::dispatch,
			context: ctx,
		};

		if let Some(v) = self.map.get_mut(&ev) {
			v.push(subscription);
		} else {
			self.map.insert(ev, vec![subscription]);
		}
	}

	/// Get all the [`Subscription`] instances attached to a [`HandlerEvent`]
	///
	/// # Arguments
	///
	/// * `ev`: The [`HandlerEvent`]
	///
	/// returns: `Option<&Vec<Subscription, Global>>`
	pub(crate) fn get(&self, ev: &HandlerEvent) -> Option<&Vec<Subscription>> {
		self.map.get(ev)
	}
}

/// Subscription manager that will hold a general [`SubscriptionMap`] for all the events triggered
/// and one [`SubscriptionManager`] per each collection-attached [`Subscription`] indexed by the
/// collection string
pub(crate) struct SubscriptionManager {
	collection_subscriptions: HashMap<String, SubscriptionMap>,
	subscriptions: SubscriptionMap,
}

impl SubscriptionManager {
	/// Creates a new instance of `SubscriptionManager`
	///
	/// returns: [`SubscriptionManager`]
	pub(crate) fn new() -> Self {
		Self {
			collection_subscriptions: HashMap::new(),
			subscriptions: SubscriptionMap::empty(),
		}
	}

	/// Inserts into the global [`SubscriptionMap`] a [`Handler`] to subscribe to a [`HandlerEvent`]
	/// with the given [`HandlerContext`]
	///
	/// # Arguments
	///
	/// * `ev`: The [`HandlerEvent`] the [`Handler`] is gonna listen to
	/// * `ctx`: The [`Handler`]'s [`HandlerContext`]. Note that you could pass here any [`HandlerContext`]
	/// with any type, but note that if its type it's not the same as the [`Handler::Context`] one the
	/// [`Handler::call`] function is never gonna be executed as downcasting will fail
	pub(crate) fn insert<H: Handler>(&mut self, ev: HandlerEvent, ctx: HandlerContext<dyn Any>) {
		self.subscriptions.insert::<H>(ev, ctx)
	}

	/// Inserts (or creates if doesn't exist) a [`Handler`] into the [`SubscriptionMap`] attached to
	/// a collection to subscribe to a [`HandlerEvent`] with the given [`HandlerContext`]
	///
	/// # Arguments
	///
	/// * `ev`: The [`HandlerEvent`] the [`Handler`] is gonna listen for
	/// * `collection`: The collection name the [`Handler`] is gonna listen for
	/// * `ctx`: The [`Handler`]'s [`HandlerContext`]. Note that you could pass here any [`HandlerContext`]
	/// with any type, but note that if its type it's not the same as the [`Handler::Context`] one the
	/// [`Handler::call`] function is never gonna be executed as downcasting will fail
	pub(crate) fn insert_to<H: Handler>(
		&mut self,
		ev: HandlerEvent,
		collection: &str,
		ctx: HandlerContext<dyn Any>,
	) {
		if let Some(subs) = self.collection_subscriptions.get_mut(collection) {
			subs.insert::<H>(ev, ctx)
		} else {
			let mut map = SubscriptionMap::empty();
			let ctx = map.insert::<H>(ev, ctx);

			self.collection_subscriptions
				.insert(collection.to_string(), map);

			ctx
		}
	}

	/// Triggers all the [`Subscription`]s for a [`HandlerEvent`] with the possibility of also
	/// triggering all the [`Subscription`]s for the same [`HandlerEvent`] and a specific
	/// collection. It will also give to the [`Subscription`] callback the [`DocumentOperation`]
	/// data of the event
	///
	/// # Arguments
	///
	/// * `ev`: The [`HandlerEvent`] to be triggered
	/// * `doc`: The [`DocumentOperation`] data
	/// * `collection`: [`Some`] to trigger collection-attached [`Subscription`]  callbacks or
	/// [`None`] to trigger only global [`Subscription`] callbacks
	pub(crate) async fn call(
		&self,
		ev: HandlerEvent,
		doc: &DocumentOperation,
		collection: Option<&str>,
	) {
		async fn dispatch_event(e: &HandlerEvent, map: &SubscriptionMap, doc: &DocumentOperation) {
			if let Some(subs) = map.get(e) {
				for sub in subs {
					if let Some(cb) = (sub.callback)(&sub.context, doc) {
						cb.await
					} else {
						println!(
							"arangodb_events_rs: warn: unable to downcast context for {:?} handler",
							sub.name
						)
					}
				}
			}
		}

		// Call generic subscriptions with no collection attached
		dispatch_event(&ev, &self.subscriptions, doc).await;

		// Call subscriptions for specific collection if matches
		if let Some(col) = collection {
			if let Some(map) = self.collection_subscriptions.get(col) {
				dispatch_event(&ev, map, doc).await;
			}
		}
	}
}
