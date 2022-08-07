use hyper::http::request::Builder as HttpRequestBuilder;
use hyper::{Body, Client, Request, Response, StatusCode, Uri};

use crate::api::{LogType, LoggerStateData};
use crate::deserialize::Deserializer;
use crate::{utils, ArangoDBError, Error, Io, Kind, MapCrateError, Result};

const LAST_LOG_HEADER: &str = "X-Arango-Replication-Lastincluded";

pub struct Trigger {
	host: String,
	database: String,
	auth: Option<TriggerAuthentication>,
	last_log_tick: String,
}

pub struct TriggerAuthentication {
	user: String,
	password: String,
}

impl TriggerAuthentication {
	pub fn new(user: &str, password: &str) -> Self {
		Self {
			user: user.to_string(),
			password: password.to_string(),
		}
	}
}

impl Trigger {
	pub fn new(host: &str, database: &str) -> Self {
		Self {
			host: host.to_string(),
			database: database.to_string(),
			auth: None,
			last_log_tick: "0".to_string(),
		}
	}

	pub fn new_auth(host: &str, database: &str, auth: TriggerAuthentication) -> Self {
		Self {
			host: host.to_string(),
			database: database.to_string(),
			auth: Some(auth),
			last_log_tick: "0".to_string(),
		}
	}

	fn get_uri(&self, endpoint: &str) -> Result<Uri> {
		format!("{}/_db/{}{}", self.host, self.database, endpoint)
			.parse()
			.map_err(|e: hyper::http::uri::InvalidUri| e.into())
	}

	fn get_authorization_value(&self, auth: &TriggerAuthentication) -> String {
		format!(
			"Basic {}",
			base64::encode(format!("{}:{}", auth.user, auth.password))
		)
	}

	fn get_new_request(&self, uri: Uri) -> HttpRequestBuilder {
		let mut req = Request::builder().uri(uri);

		if let Some(auth) = &self.auth {
			req = req.header(
				hyper::header::AUTHORIZATION,
				self.get_authorization_value(auth),
			);
		}

		req
	}

	pub async fn init(&mut self) -> Result<()> {
		let client = Client::new();

		let logger_state_uri = self.get_uri("/_api/replication/logger-state")?;
		let req = self
			.get_new_request(logger_state_uri)
			.body(Body::empty())
			.map_crate_err()?;

		let response: Response<Body> = client.request(req).await.map_crate_err()?;

		match response.status() {
			StatusCode::UNAUTHORIZED => Err(ArangoDBError::Unauthorized.into()),
			StatusCode::METHOD_NOT_ALLOWED => Err(ArangoDBError::MethodNotAllowed.into()),
			StatusCode::INTERNAL_SERVER_ERROR => Err(ArangoDBError::ServerError.into()),
			StatusCode::OK => {
				let bytes = hyper::body::to_bytes(response.into_body()).await?;
				let data: LoggerStateData =
					serde_json::from_slice(bytes.as_ref()).map_crate_err()?;

				self.last_log_tick = data.state.last_log_tick;

				Ok(())
			}
			_ => unreachable!("Unexpected {} status code", response.status()),
		}
	}

	pub async fn listen(&mut self) -> Result<()> {
		let current_tick = self.last_log_tick.clone();

		let client = Client::new();

		let logger_state_uri = self.get_uri(
			format!(
				"/_api/replication/logger-follow?from={}",
				current_tick.as_str()
			)
			.as_str(),
		)?;

		let req = self
			.get_new_request(logger_state_uri)
			.body(Body::empty())
			.map_crate_err()?;

		let response: Response<Body> = client.request(req).await.map_crate_err()?;

		match response.status() {
			StatusCode::UNAUTHORIZED => Err(ArangoDBError::Unauthorized.into()),
			StatusCode::METHOD_NOT_ALLOWED => Err(ArangoDBError::MethodNotAllowed.into()),
			StatusCode::INTERNAL_SERVER_ERROR => Err(ArangoDBError::ServerError.into()),
			StatusCode::BAD_REQUEST => Err(ArangoDBError::BadRequest.into()),
			StatusCode::NOT_IMPLEMENTED => Err(ArangoDBError::NotImplemented.into()),
			StatusCode::OK | StatusCode::NO_CONTENT => {
				let next_log_tick = if let Some(v) = response.headers().get(LAST_LOG_HEADER) {
					let value = v.to_str().map_crate_err()?;

					if value == "0" {
						tokio::time::sleep(std::time::Duration::from_millis(500)).await;

						current_tick.as_str()
					} else {
						value
					}
				} else {
					current_tick.as_str()
				};

				self.last_log_tick = next_log_tick.to_string();

				// If there's no change on tick value, call again process_log_tick
				if !next_log_tick.eq(&current_tick) {
					let mut deserializer = Deserializer::new(response.into_body());

					println!("----------{}----------------", current_tick);

					while let Some(line) = deserializer.read_line().await? {
						self.process_line(line);
					}

					println!("---------------------------------")
				}

				Ok(())
			}
			_ => unreachable!("Unexpected {} status code", response.status()),
		}
	}

	// {"tick":"6181901","type":2300,"database":"664279","tid":"1368501","cid":"664397","cname":"accounts","data":{"_key":"664537","_id":"accounts/664537","_rev":"_eljTnkS--A","firstName":"Chirus","age":20}}
	fn process_line(&self, line: String) -> Result<()> {
		let type_idx = line
			.find("\"type\":")
			.ok_or(Error::new(Kind::Io(Io::Serialize)))?
			+ 7; // 7 = "type": length

		let log_type: u16 = utils::get_string_between(line.as_str(), type_idx, 4)
			.parse()
			.map_crate_err()?;

		match log_type.try_into() {
			Ok(LogType::StartTransaction) => {
				println!("Start transaction")
			}
			Ok(LogType::InsertOrReplaceDocument) => {
				println!("Insert or replace")
			}
			Ok(LogType::CommitTransaction) => {
				println!("Commit transaction")
			}
			_ => {}
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	pub async fn it_inits() -> Result<()> {
		let mut trigger = Trigger::new_auth(
			"http://localhost:8529/",
			"_system",
			TriggerAuthentication::new("root", "root"),
		);

		trigger.init().await?;

		Ok(())
	}
}
