# (Rust) Arango Triggers

A library to add triggers to your ArangoDB database, when events occur (insert, update, delete etc.) on your collections.

## Usage

```rust
use rust_arango_triggers::{Trigger, TriggerConfig};

fn main() {
	let trigger = Trigger::new("http://localhost:8529", "database");

	/**
	 *  Possible events:
	 *  - insert/update
	 *  - delete
	 */

	// Subscribe to all the events on the users collection
	trigger.subscribe(vec![
		TriggerConfig::new("users")
	]);

	// Subscribe to only insert/update events on the users collection
	trigger.subscribe(vec![
		TriggerConfig::new_with_events("users", vec!["insert/update"])
	]);

	// Subscribe to only delete events on the users collection with the key "252525"
	trigger.subscribe(vec![
		TriggerConfig::new_with_events_and_keys("users", vec!["insert/update"], vec!["252525"]),
	]);

	// Unsubscribe from all events on the users collection
	trigger.unsubscribe(vec![
		TriggerConfig::new("users")
	]);

	// Unsubscribe from only delete events on the users collection
	trigger.unsubscribe(vec![
		TriggerConfig::new_with_events("users", vec!["delete"])
	]);

	// Unsubscribe from only delete events on the users collection with the key "252525"
	trigger.unsubscribe(vec![
		TriggerConfig::new_with_events_and_keys("users", vec!["delete"], vec!["252525"])
	]);

	trigger.start();
}
```