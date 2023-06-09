use std::{
	collections::HashMap,
	error::Error,
	future::Future,
	pin::Pin,
    sync::Arc,
	task::{Context, Poll},
};
use hyper::{
	header::{ACCEPT, CONTENT_TYPE},
	http::HeaderValue,
	Body, Method, Request, Response, Uri, StatusCode,
};
use jsonrpsee::types::{Id, RequestSer};
use serde_json::{value::to_raw_value, Value as JsonValue};
use tower::{Layer, Service};

#[derive(Debug, Clone)]
pub struct ProxyGetRequestParamsLayer {}

impl ProxyGetRequestParamsLayer {
	pub fn new() -> Self {
		Self {}
	}
}

impl<S> Layer<S> for ProxyGetRequestParamsLayer {
	type Service = ProxyGetRequestParams<S>;

	fn layer(&self, inner: S) -> Self::Service {
		ProxyGetRequestParams::new(inner)
	}
}

#[derive(Debug, Clone)]
pub struct ProxyGetRequestParams<S> {
	inner: S,
}

impl<S> ProxyGetRequestParams<S> {
	pub fn new(inner: S) -> Self {
		Self { inner }
	}
}

impl<S> Service<Request<Body>> for ProxyGetRequestParams<S>
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
		let modify = req.uri().path() != "/" && req.method() == Method::GET;
		if modify {
			let params_map: HashMap<String, JsonValue> = req
				.uri()
				.query()
				.map(|v| url::form_urlencoded::parse(v.as_bytes())
					.into_owned()
					.map(|(k, v)| (k, serde_json::to_value(v.trim_matches('"')).expect("valid query param")))
					.collect()
				)
				.unwrap_or_else(HashMap::new);
			let params_raw = to_raw_value(&params_map).expect("valid params");
			let method = req.uri().path().trim_start_matches('/').to_string();
			*req.method_mut() = Method::POST;
			*req.uri_mut() = Uri::from_static("/");
			req.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
			req.headers_mut().insert(ACCEPT, HeaderValue::from_static("application/json"));
			let body = Body::from(
				serde_json::to_string(&RequestSer::borrowed(&Id::Number(0), &method, Some(params_raw.as_ref())))
					.expect("valid request"),
			);
			req = req.map(|_| body);
		}
		let fut = self.inner.call(req);
		let res_fut = async move {
			Ok(fut.await.map_err(|err| err.into())?)
        };
        Box::pin(res_fut)
    }
}

#[derive(Clone)]
pub struct ProxyGetRequestCustomLayer {
	path: String,
	func: &'static dyn Fn(&Request<Body>) -> String,
}

unsafe impl Send for ProxyGetRequestCustomLayer {}

impl ProxyGetRequestCustomLayer {
	pub fn new(path: impl Into<String>, func: &'static impl Fn(&Request<Body>) -> String) -> Result<Self, jsonrpsee::core::Error> {
		let path = path.into();
		if !path.starts_with('/') {
			return Err(jsonrpsee::core::Error::Custom("ProxyGetRequestCustomLayer path must start with `/`".to_string()));
		}
		Ok(Self { path, func })
	}
}

impl<S> Layer<S> for ProxyGetRequestCustomLayer {
	type Service = ProxyGetRequestCustom<S>;

	fn layer(&self, inner: S) -> Self::Service {
		ProxyGetRequestCustom::new(inner, &self.path, self.func)
			.expect("Path already validated in ProxyGetRequestCustomLayer; qed")
	}
}

#[derive(Clone)]
pub struct ProxyGetRequestCustom<S> {
	inner: S,
	path: Arc<str>,
	func: &'static dyn Fn(&Request<Body>) -> String,
}

unsafe impl<S> Send for ProxyGetRequestCustom<S> {}

impl<S> ProxyGetRequestCustom<S> {
	pub fn new(inner: S, path: &str, func: &'static dyn Fn(&Request<Body>) -> String) -> Result<Self, jsonrpsee::core::Error> {
		if !path.starts_with('/') {
			return Err(jsonrpsee::core::Error::Custom(format!("ProxyGetRequestCustom path must start with `/`, got: {}", path)));
		}

		Ok(Self { inner, path: Arc::from(path), func })
	}
}

impl<S> Service<Request<Body>> for ProxyGetRequestCustom<S>
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

	fn call(&mut self, req: Request<Body>) -> Self::Future {
		let modify = self.path.as_ref() == req.uri().path() && req.method() == Method::GET;
		if modify {
			let content = (self.func)(&req);
			let res_fut = async move {
				Response::builder()
					.status(StatusCode::OK)
					.body(Body::from(content))
					.map_err(Into::into)
			};
			return Box::pin(res_fut);
		}
		let fut = self.inner.call(req);
		let res_fut = async move {
			Ok(fut.await.map_err(|err| err.into())?)
        };
        Box::pin(res_fut)
    }
}
