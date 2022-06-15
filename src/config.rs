pub struct TriggerConfig<'a> {
	collection: String,
	events: Option<Vec<&'a str>>,
	keys: Option<Vec<&'a str>>,
}

impl TriggerConfig<'_> {
	pub fn new(collection: &str) -> Self {
		Self {
			collection: collection.to_string(),
			events: None,
			keys: None,
		}
	}

	pub fn new_with_events(collection: &str, events: Option<Vec<&'static str>>) -> Self {
		Self {
			collection: collection.to_string(),
			events,
			keys: None,
		}
	}

	pub fn new_with_events_and_keys(
		collection: &str,
		events: Option<Vec<&'static str>>,
		keys: Option<Vec<&'static str>>,
	) -> Self {
		Self {
			collection: collection.to_string(),
			events,
			keys,
		}
	}
}
