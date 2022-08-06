use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

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
}

#[derive(Debug)]
pub(super) enum ArangoDBError {
	Unauthorized,
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
