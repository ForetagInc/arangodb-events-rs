use std::fmt;

pub(crate) type StdResult<T, E> = std::result::Result<T, E>;

pub type Result<T> = StdResult<T, Error>;

type Cause = Box<dyn std::error::Error>;

pub struct Error {
	inner: Box<ErrorImpl>,
}

struct ErrorImpl {
	kind: Kind,
	cause: Option<Cause>,
}

#[derive(Debug)]
pub(super) enum Kind {
	Http,
	ArangoDB(ArangoDBError),
	Io(Io),
}

#[derive(Debug)]
pub(super) enum Io {
	Serialize,
	Other,
}

#[derive(Debug)]
pub(super) enum ArangoDBError {
	HttpError(hyper::StatusCode),
}

pub trait MapCrateError<T, E: Into<Error>> {
	fn map_crate_err(self) -> Result<T>;
}

impl<T, E: Into<Error>> MapCrateError<T, E> for StdResult<T, E> {
	fn map_crate_err(self) -> Result<T> {
		self.map_err::<Error, _>(|e| e.into())
	}
}

impl Error {
	pub(super) fn new(kind: Kind) -> Error {
		Error {
			inner: Box::new(ErrorImpl { kind, cause: None }),
		}
	}

	pub(super) fn with<C: Into<Cause>>(mut self, cause: C) -> Error {
		self.inner.cause = Some(cause.into());
		self
	}

	pub(super) fn kind(&self) -> &Kind {
		&self.inner.kind
	}

	fn description(&self) -> String {
		match self.inner.kind {
			Kind::Http => "HTTP client error".to_string(),
			Kind::ArangoDB(ArangoDBError::HttpError(s)) => {
				format!("ArangoDB HTTP API error: {}", s.as_str())
			}
			Kind::Io(Io::Serialize) => "Error while serializing/deserializing data".to_string(),
			Kind::Io(Io::Other) => "I/O Error".to_string(),
		}
	}
}

impl fmt::Debug for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut f = f.debug_tuple("rust_arango_trigger::Error");
		f.field(&self.inner.kind);
		if let Some(ref cause) = self.inner.cause {
			f.field(cause);
		}
		f.finish()
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if let Some(ref cause) = self.inner.cause {
			write!(f, "{}: {}", self.description(), cause)
		} else {
			f.write_str(self.description().as_str())
		}
	}
}

impl std::error::Error for Error {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		self.inner
			.cause
			.as_ref()
			.map(|cause| &**cause as &(dyn std::error::Error + 'static))
	}
}

impl From<hyper::StatusCode> for Error {
	fn from(s: hyper::StatusCode) -> Self {
		Error::new(Kind::ArangoDB(ArangoDBError::HttpError(s)))
	}
}

macro_rules! err_from {
	($err:path, $new:expr) => {
		impl From<$err> for Error {
			fn from(_: $err) -> Self {
				Error::new($new)
			}
		}
	};
	(+ $err:path, $new:expr) => {
		impl From<$err> for Error {
			fn from(e: $err) -> Self {
				Error::new($new).with(e)
			}
		}
	};
}

err_from!(+ hyper::http::uri::InvalidUri, Kind::Http);
err_from!(+ hyper::http::Error, Kind::Http);
err_from!(+ hyper::Error, Kind::Http);
err_from!(+ std::io::Error, Kind::Io(Io::Other));
err_from!(+ serde_json::Error, Kind::Io(Io::Serialize));
err_from!(+ hyper::header::ToStrError, Kind::Io(Io::Serialize));
err_from!(+ std::num::ParseIntError, Kind::Io(Io::Serialize));
