use futures_util::stream::IntoAsyncRead;
use futures_util::{AsyncBufReadExt, AsyncRead, Stream, TryStreamExt};
use hyper::body::Bytes;
use hyper::Body;

use std::pin::Pin;
use std::result::Result as StdResult;
use std::task::{Context, Poll};

use crate::{MapCrateError, Result};

/// Line deserializer for [`AsyncRead`] trait
pub(crate) struct Deserializer<R: AsyncRead> {
	inner: R,
}

/// Wrapper for [`Body`] to implement [`Stream`] trait with Std Error as Result to help typings and
/// prevent error mapping on the [`Deserializer`] constructor
pub(crate) struct BodyStream {
	inner: Body,
}

impl BodyStream {
	/// Creates a new BodyStream wrapper for [`Body`]
	///
	/// # Arguments
	///
	/// * `body`: The response body to be wrapped
	///
	/// returns: [`BodyStream`]
	pub(crate) fn new(body: Body) -> Self {
		Self { inner: body }
	}
}

impl Stream for BodyStream {
	type Item = StdResult<Bytes, std::io::Error>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		Pin::new(&mut self.inner)
			.poll_next(cx)
			.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{}", e)))
	}
}

impl Deserializer<IntoAsyncRead<BodyStream>> {
	/// Creates a new Deserializer instance with which you'll be able to read lines in a Streamed way
	/// for [`Body`] HTTP responses.
	///
	/// # Arguments
	///
	/// * `body`: The response body to be deserialized
	///
	/// returns: [`Deserializer<IntoAsyncRead<BodyStream>>`]
	pub(crate) fn new(body: Body) -> Self {
		Self {
			inner: BodyStream::new(body).into_async_read(),
		}
	}

	/// Reads a new line from the Asynchronous [`BodyStream`] wrapper returning [`None`] if there
	/// are no more lines to read
	///
	/// returns: [`Result<Option<String>>`]
	pub(crate) async fn read_line(&mut self) -> Result<Option<String>> {
		let mut str = String::new();

		let b = self.inner.read_line(&mut str).await.map_crate_err()?;

		if b == 0 {
			return Ok(None);
		}

		Ok(Some(str))
	}
}
