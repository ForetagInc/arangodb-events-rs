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
	Unauthorized,
	MethodNotAllowed,
	ServerError,
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

	fn description(&self) -> &str {
		match self.inner.kind {
			Kind::Http => "HTTP client error",
			Kind::ArangoDB(ArangoDBError::Unauthorized) => {
				"ArangoDB error: not authorized to execute this request"
			}
			Kind::ArangoDB(ArangoDBError::MethodNotAllowed) => {
				"ArangoDB error: method not supported"
			}
			Kind::ArangoDB(ArangoDBError::ServerError) => "ArangoDB error: internal server error",
			Kind::Io(Io::Serialize) => "Error while serializing/deserializing data",
			Kind::Io(Io::Other) => "I/O Error",
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
			f.write_str(self.description())
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

impl From<ArangoDBError> for Error {
	fn from(e: ArangoDBError) -> Self {
		Error::new(Kind::ArangoDB(e))
	}
}

impl From<hyper::http::uri::InvalidUri> for Error {
	fn from(e: hyper::http::uri::InvalidUri) -> Self {
		Error::new(Kind::Http).with(e)
	}
}

impl From<hyper::http::Error> for Error {
	fn from(e: hyper::http::Error) -> Self {
		Error::new(Kind::Http).with(e)
	}
}

impl From<hyper::Error> for Error {
	fn from(e: hyper::Error) -> Self {
		Error::new(Kind::Http).with(e)
	}
}

impl From<std::io::Error> for Error {
	fn from(e: std::io::Error) -> Self {
		Error::new(Kind::Io(Io::Other)).with(e)
	}
}

impl From<serde_json::Error> for Error {
	fn from(e: serde_json::Error) -> Self {
		Error::new(Kind::Io(Io::Serialize)).with(e)
	}
}

impl From<hyper::header::ToStrError> for Error {
	fn from(e: hyper::header::ToStrError) -> Self {
		Error::new(Kind::Io(Io::Serialize)).with(e)
	}
}
