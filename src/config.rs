pub struct TriggerConfig {
	collection: String,
	events: Option<Vec<String>>,
	keys: Option<Vec<String>>,
}

impl TriggerConfig {
	pub fn new(collection: &str) -> Self {
		Self {
			collection: collection.to_string(),
			events: None,
			keys: None,
		}
	}

	pub fn new_with_events(collection: &str, events: Option<Vec<String>>) -> Self {
		Self {
			collection: collection.to_string(),
			events,
			keys: None,
		}
	}

	pub fn new_with_events_and_keys(
		collection: &str,
		events: Option<Vec<String>>,
		keys: Option<Vec<String>>,
	) -> Self {
		Self {
			collection: collection.to_string(),
			events,
			keys,
		}
	}
}
