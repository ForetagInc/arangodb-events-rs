pub struct TriggerConfig {
	collection: String,
	events: Option<Vec<String>>,
	keys: Option<Vec<String>>,
}

impl TriggerConfig {
	fn new(collection: String, events: Option<Vec<String>>, keys: Option<Vec<String>>) -> Self {
		Self {
			collection,
			events,
			keys,
		}
	}
}
