use futures_util::stream::IntoAsyncRead;
use futures_util::{AsyncBufReadExt, AsyncRead, Stream, TryStreamExt};
use hyper::body::Bytes;
use hyper::Body;

use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{MapCrateError, Result, StdResult};

pub(crate) struct Deserializer<R: AsyncRead> {
	inner: R,
}

pub(crate) struct BodyStream {
	inner: Body,
}

impl BodyStream {
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
	pub(crate) fn new(body: Body) -> Self {
		Self {
			inner: BodyStream::new(body).into_async_read(),
		}
	}

	pub(crate) async fn read_line(&mut self) -> Result<Option<String>> {
		let mut str = String::new();

		let b = self.inner.read_line(&mut str).await.map_crate_err()?;

		if b == 0 {
			return Ok(None);
		}

		Ok(Some(str))
	}
}
