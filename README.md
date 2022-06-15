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
		TriggerConfig::new("users", ["insert/update"])
	]);

	// Subscribe to only delete events on the users collection with the key "252525"
	trigger.subscribe(vec![
		TriggerConfig::new("users", ["insert/update"], ["252525"]),
	]);

	// Unsubscribe from all events on the users collection
	trigger.unsubscribe(vec![
		TriggerConfig::new("users")
	]);

	// Unsubscribe from only delete events on the users collection
	trigger.unsubscribe(vec![
		TriggerConfig::new("users", ["delete"])
	]);

	// Unsubscribe from only delete events on the users collection with the key "252525"
	trigger.unsubscribe(vec![
		TriggerConfig::new("users", ["delete"], ["252525"])
	]);

	trigger.start();
}
```