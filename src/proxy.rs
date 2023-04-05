use hyper::header::{ACCEPT, CONTENT_TYPE};
use hyper::http::HeaderValue;
use hyper::{Body, Method, Request, Response, Uri};
use jsonrpsee::types::{Id, RequestSer};
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

#[derive(Debug, Clone)]
pub struct ProxyGetRequestLayer {
	path: String,
	method: String,
}

impl ProxyGetRequestLayer {
	pub fn new(path: impl Into<String>, method: impl Into<String>) -> Result<Self, jsonrpsee::core::Error> {
		let path = path.into();
		if !path.starts_with('/') {
			return Err(jsonrpsee::core::Error::Custom("ProxyGetRequestLayer path must start with `/`".to_string()));
		}
		Ok(Self { path, method: method.into() })
	}
}

impl<S> Layer<S> for ProxyGetRequestLayer {
	type Service = ProxyGetRequest<S>;

	fn layer(&self, inner: S) -> Self::Service {
		ProxyGetRequest::new(inner, &self.path, &self.method)
			.expect("Path already validated in ProxyGetRequestLayer; qed")
	}
}

#[derive(Debug, Clone)]
pub struct ProxyGetRequest<S> {
	inner: S,
	path: Arc<str>,
	method: Arc<str>,
}

impl<S> ProxyGetRequest<S> {
	pub fn new(inner: S, path: &str, method: &str) -> Result<Self, jsonrpsee::core::Error> {
		if !path.starts_with('/') {
			return Err(jsonrpsee::core::Error::Custom(format!("ProxyGetRequest path must start with `/`, got: {}", path)));
		}

		Ok(Self { inner, path: Arc::from(path), method: Arc::from(method) })
	}
}

impl<S> Service<Request<Body>> for ProxyGetRequest<S>
where
	S: Service<Request<Body>, Response = Response<Body>>,
	S::Response: 'static,
	S::Error: Into<Box<dyn Error + Send + Sync>> + 'static,
	S::Future: Send + 'static,
{
	type Response = S::Response;
	type Error = Box<dyn Error + Send + Sync + 'static>;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

	#[inline]
	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx).map_err(Into::into)
	}

	fn call(&mut self, mut req: Request<Body>) -> Self::Future {
		let modify = self.path.as_ref() == req.uri() && req.method() == Method::GET;

		// Proxy the request to the appropriate method call.
		if modify {
			// RPC methods are accessed with `POST`.
			*req.method_mut() = Method::POST;
			// Precautionary remove the URI.
			*req.uri_mut() = Uri::from_static("/");

			// Requests must have the following headers:
			req.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
			req.headers_mut().insert(ACCEPT, HeaderValue::from_static("application/json"));

			// Adjust the body to reflect the method call.
			let body = Body::from(
				serde_json::to_string(&RequestSer::borrowed(&Id::Number(0), &self.method, None))
					.expect("Valid request; qed"),
			);
			req = req.map(|_| body);
		}

        // Call the inner service and get a future that resolves to the response.
		let fut = self.inner.call(req);

		let res_fut = async move {
			Ok(fut.await.map_err(|err| err.into())?)
        };
        Box::pin(res_fut)
    }
}
