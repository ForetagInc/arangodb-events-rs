use std::any::Any;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use crate::api::DocumentOperation;

#[derive(Eq, PartialEq, Hash)]
pub enum HandlerEvent {
	InsertOrReplace,
	Remove,
}

pub struct HandlerContext<T: ?Sized>(Arc<Box<T>>);

impl<T: ?Sized> HandlerContext<T> {
	pub(crate) fn new(data: Box<T>) -> Self {
		Self(Arc::new(data))
	}
}

impl<T: ?Sized> HandlerContext<T> {
	pub fn get_ref(&self) -> &T {
		self.0.as_ref()
	}

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

pub trait Handler: 'static {
	type Context;

	fn call(ctx: &Self::Context, doc: &DocumentOperation);

	fn dispatch(ctx: HandlerContext<dyn Any>, doc: &DocumentOperation) {
		if let Some(data) = ctx.downcast_ref::<Self::Context>() {
			Self::call(&data, doc);
		}
	}
}

pub struct HandlerContextFactory;

// This factory makes it possible to return `dyn Any` type directly, to avoid needing for
// specifying type on the library use code
impl HandlerContextFactory {
	pub fn from<T: Any>(data: T) -> HandlerContext<dyn Any> {
		HandlerContext::new(Box::new(data))
	}
}

pub(crate) struct Subscription {
	callback: fn(HandlerContext<dyn Any>, &DocumentOperation),
	context: HandlerContext<dyn Any>,
}

pub(crate) struct SubscriptionMap {
	map: HashMap<HandlerEvent, Vec<Subscription>>,
}

impl SubscriptionMap {
	pub(crate) fn empty() -> Self {
		Self {
			map: HashMap::new(),
		}
	}

	pub(crate) fn insert<H: Handler>(&mut self, ev: HandlerEvent, ctx: HandlerContext<dyn Any>) {
		let subscription = Subscription {
			callback: H::dispatch,
			context: ctx,
		};

		if let Some(v) = self.map.get_mut(&ev) {
			v.push(subscription);
		} else {
			self.map.insert(ev, vec![subscription]);
		}
	}

	pub(crate) fn get(&self, ev: &HandlerEvent) -> Option<&Vec<Subscription>> {
		self.map.get(ev)
	}
}

pub(crate) struct SubscriptionManager {
	collection_subscriptions: HashMap<String, SubscriptionMap>,
	subscriptions: SubscriptionMap,
}

impl SubscriptionManager {
	pub(crate) fn new() -> Self {
		Self {
			collection_subscriptions: HashMap::new(),
			subscriptions: SubscriptionMap::empty(),
		}
	}

	pub(crate) fn insert<H: Handler>(&mut self, ev: HandlerEvent, ctx: HandlerContext<dyn Any>) {
		self.subscriptions.insert::<H>(ev, ctx)
	}

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

	pub(crate) fn call(&self, ev: HandlerEvent, doc: &DocumentOperation, collection: Option<&str>) {
		fn dispatch_event(e: &HandlerEvent, map: &SubscriptionMap, doc: &DocumentOperation) {
			if let Some(subs) = map.get(e) {
				for sub in subs {
					(sub.callback)(sub.context.clone(), doc)
				}
			}
		}

		// Call generic subscriptions with no collection attached
		dispatch_event(&ev, &self.subscriptions, doc);

		// Call subscriptions for specific collection if matches
		if let Some(col) = collection {
			self.collection_subscriptions
				.get(col)
				.map(|m| dispatch_event(&ev, m, doc));
		}
	}
}
