use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use crate::Result;

#[derive(Eq, PartialEq, Hash)]
pub enum HandlerEvent {
	InsertOrReplace,
	Remove,
}

pub struct HandlerContext<T: ?Sized>(Arc<T>);

impl<T> HandlerContext<T> {
	pub fn new(data: T) -> Self {
		Self(Arc::new(data))
	}
}

impl<T: ?Sized> Clone for HandlerContext<T> {
	fn clone(&self) -> HandlerContext<T> {
		HandlerContext(Arc::clone(&self.0))
	}
}

pub trait Handler<T: ?Sized> {
	fn call(ctx: &HandlerContext<T>) -> Result<()>;
}

pub(crate) struct Subscription<T: ?Sized> {
	callback: fn(&HandlerContext<T>) -> Result<()>,
	context: HandlerContext<T>,
}

pub(crate) struct SubscriptionMap {
	map: HashMap<HandlerEvent, Vec<Box<dyn Any>>>,
}

impl SubscriptionMap {
	pub(crate) fn empty() -> Self {
		Self {
			map: HashMap::new(),
		}
	}

	pub(crate) fn insert<T: ?Sized + 'static, H: Handler<T>>(
		&mut self,
		ev: HandlerEvent,
		ctx: HandlerContext<T>,
	) {
		let subscription = Box::new(Subscription {
			callback: H::call,
			context: ctx,
		});

		if let Some(v) = self.map.get_mut(&ev) {
			v.push(subscription);
		} else {
			self.map.insert(ev, vec![subscription]);
		}
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

	pub(crate) fn insert<T: ?Sized + 'static, H: Handler<T>>(
		&mut self,
		ev: HandlerEvent,
		ctx: HandlerContext<T>,
	) {
		self.subscriptions.insert::<T, H>(ev, ctx)
	}

	pub(crate) fn insert_to<T: ?Sized + 'static, H: Handler<T>>(
		&mut self,
		ev: HandlerEvent,
		collection: String,
		ctx: HandlerContext<T>,
	) {
		if let Some(subs) = self.collection_subscriptions.get_mut(&collection) {
			subs.insert::<T, H>(ev, ctx)
		} else {
			let mut map = SubscriptionMap::empty();
			let ctx = map.insert::<T, H>(ev, ctx);

			self.collection_subscriptions.insert(collection, map);

			ctx
		}
	}
}
