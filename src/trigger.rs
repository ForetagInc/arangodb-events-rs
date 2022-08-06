use hyper::http::request::Builder as HttpRequestBuilder;
use hyper::{Body, Client, Request, Response, StatusCode, Uri};

use crate::{ArangoDBError, Error, Result};

pub struct Trigger {
	host: String,
	database: String,
	auth: Option<TriggerAuthentication>,
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
	pub async fn new(
		host: &str,
		database: &str,
		auth: Option<TriggerAuthentication>,
	) -> Result<Self> {
		let trigger = Self {
			host: host.to_string(),
			database: database.to_string(),
			auth,
		};

		trigger.init().await?;

		Ok(trigger)
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

	async fn init(&self) -> Result<()> {
		let client = Client::new();

		let logger_state_uri = self.get_uri("/_api/replication/logger-state")?;
		let req = self
			.get_new_request(logger_state_uri)
			.body(Body::empty())
			.map_err::<Error, _>(|e| e.into())?;

		let response: Response<Body> = client
			.request(req)
			.await
			.map_err::<Error, _>(|e| e.into())?;

		match response.status() {
			StatusCode::UNAUTHORIZED => Err(ArangoDBError::Unauthorized.into()),
			StatusCode::METHOD_NOT_ALLOWED => Err(ArangoDBError::MethodNotAllowed.into()),
			StatusCode::INTERNAL_SERVER_ERROR => Err(ArangoDBError::ServerError.into()),
			StatusCode::OK => {
				println!("{:?}", response);

				Ok(())
			}
			_ => unreachable!(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	pub async fn it_inits() -> Result<()> {
		let trigger = Trigger::new(
			"http://localhost:8529/",
			"_system",
			Some(TriggerAuthentication::new("root", "root")),
		)
		.await?;

		Ok(())
	}
}
