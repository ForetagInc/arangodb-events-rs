# ArangoDB Events

A library to add triggers to your ArangoDB database, when events occur (insert, update, delete etc.) on your
collections.

[![crates.io](https://img.shields.io/crates/v/arangodb_events_rs?label=latest&logo=rust)](https://crates.io/crates/arangodb_events_rs)
![Downloads](https://img.shields.io/crates/d/arangodb_events_rs.svg)
[![Documentation](https://docs.rs/arangodb_events_rs/badge.svg?version=latest)](https://docs.rs/arangodb_events_rs/latest)

## Documentation

- [API Documentation](https://docs.rs/arangodb_events_rs/)

## Features
- `async` Enables asynchronous `Handler::call` method

## Installation

Add the crate to your `Cargo.toml`:
```toml
arangodb_events_rs = "0.1.5"
```

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