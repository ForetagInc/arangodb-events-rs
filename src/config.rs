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

	pub fn new_with_events(collection: &str, events: Vec<&'static str>) -> Self {
		Self {
			collection: collection.to_string(),
			events: Some(events),
			keys: None,
		}
	}

	pub fn new_with_events_and_keys(
		collection: &str,
		events: Vec<&'static str>,
		keys: Vec<&'static str>,
	) -> Self {
		Self {
			collection: collection.to_string(),
			events: Some(events),
			keys: Some(keys),
		}
	}
}
