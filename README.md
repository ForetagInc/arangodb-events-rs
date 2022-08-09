# ArangoDB Events

A library to add triggers to your ArangoDB database, when events occur (insert, update, delete etc.) on your
collections.

## Usage

```rust
use arangodb_events_rs::api::DocumentOperation;
use arangodb_events_rs::{Handler, Trigger, HandlerContextFactory};

pub struct Example;

pub struct ExampleContext {
    pub data: u8,
}

impl Handler for Example {
    type Context = ExampleContext;

    fn call(ctx: &ExampleContext, doc: &DocumentOperation) {
        println!("{}", ctx.data);
    }
}

#[tokio::main]
async fn main() {
    let mut trigger = Trigger::new_auth(
        "http://localhost:8529/",
        "database",
        TriggerAuthentication::new("user", "password"),
    );

    trigger.subscribe::<Example>(
        HandlerEvent::InsertOrReplace,
        HandlerContextFactory::from(ExampleContext {
            data: 10,
        })
    );

    trigger.init().await.unwrap();

    loop {
        trigger.listen().await.unwrap();
    }
}
```