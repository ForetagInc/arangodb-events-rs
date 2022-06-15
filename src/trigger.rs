use std::str::FromStr;

use hyper::{body::Buf, Client as HTTPClient, Request};
use serde::Deserialize;

use crate::TriggerConfig;

trait EventEmitter {
	fn on(&self, event: &str, listener: Box<dyn Fn(&str)>);
}

pub struct Trigger {
	pub host: String,
	pub database: Option<String>,
	pub collections: Option<Vec<String>>,

	// Properties
	started: bool,
}

impl Default for Trigger {
	fn default() -> Self {
		Self {
			host: String::from("http://localhost:8529/"),
			database: Some(String::from("_system")),
			collections: Some(Vec::new()),
			started: false,
		}
	}
}

impl Trigger {
	pub fn new(host: &str, database: &str) -> Self {
		Self {
			host: host.to_string(),
			database: Some(database.to_string()),
			..Default::default()
		}
	}

	async fn start_logger_state(mut self) -> Result<(), ()> {
		let logger_state_path = hyper::Uri::from_str(
			format!(
				"{}/_db${}/_api/replication/logger-state",
				&self.host,
				&self.database.as_ref().unwrap()
			)
			.as_str(),
		);

		let logger_follow_path = hyper::Uri::from_str(
			format!(
				"{}/_db${}/_api/replication/logger-follow",
				&self.host,
				&self.database.as_ref().unwrap()
			)
			.as_str(),
		);

		let client = HTTPClient::new();

		// let logger_request = client.get(logger_state_path.unwrap_or_default()).await?;

		// let logger_response = hyper::body::aggregate(logger_request).await?;

		// let res = serde_json::from_reader(logger_response.reader())?;

		Ok(())
	}

	pub async fn start(mut self) {
		self.started = true;
		self.start_logger_state().await;
	}

	pub fn stop(mut self) {
		self.started = false;
	}

	pub fn subscribe(&self, config: Vec<TriggerConfig>) {}

	pub fn unsubscribe(&self, config: Vec<TriggerConfig>) {}
}
