# ArangoDB Events

A library to add triggers to your ArangoDB database, when events occur (insert, update, delete etc.) on your
collections.

## Usage

```rust
use arangodb_events_rs::api::DocumentOperation;
use arangodb_events_rs::{Handler, Trigger, HandlerContextFactory};

pub struct GlobalHandler;

pub struct GlobalHandlerContext {
    pub data: u8,
}

impl Handler for GlobalHandler {
    type Context = GlobalHandlerContext;

    fn call(ctx: &GlobalHandlerContext, doc: &DocumentOperation) {
        println!("{}", ctx.data); // 10
    }
}

#[tokio::main]
async fn main() {
    let mut trigger = Trigger::new_auth(
        "http://localhost:8529/",
        "database",
        TriggerAuthentication::new("user", "password"),
    );

    trigger.subscribe::<GlobalHandler>(
        HandlerEvent::InsertOrReplace,
        HandlerContextFactory::from(GlobalHandlerContext {
            data: 10,
        })
    ); // This subscribes for all Insert or Replace operations on the database

    trigger.subscribe_to::<AccountHandler>(
        HandlerEvent::Remove,
        "accounts",
        HandlerContextFactory::from(AccountHandlerContext {
            data: 50,
        })
    ); // This is gonna listen only for Remove operations over accounts table

    trigger
        .init()
        .await
        .expect("Error initializing ArangoDB Trigger");

    loop {
        trigger
            .listen()
            .await
            .expect("Error on Trigger listener loop");
    }
}
```