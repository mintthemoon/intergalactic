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
	Body, Method, Request, Response, Uri,
};
use jsonrpsee::types::{Id, RequestSer};
use serde_json::{value::to_raw_value, Value as JsonValue};
use tower::{Layer, Service};

#[derive(Debug, Clone)]
pub struct ProxyGetRequestLayer {
	path: String,
	method: String,
	params: Vec<String>,
}

impl ProxyGetRequestLayer {
	pub fn new(path: impl Into<String>, method: impl Into<String>, params: Vec<String>) -> Result<Self, jsonrpsee::core::Error> {
		let path = path.into();
		if !path.starts_with('/') {
			return Err(jsonrpsee::core::Error::Custom("ProxyGetRequestLayer path must start with `/`".to_string()));
		}
		Ok(Self { path, method: method.into(), params })
	}
}

impl<S> Layer<S> for ProxyGetRequestLayer {
	type Service = ProxyGetRequest<S>;

	fn layer(&self, inner: S) -> Self::Service {
		ProxyGetRequest::new(inner, &self.path, &self.method, &self.params)
			.expect("Path already validated in ProxyGetRequestLayer; qed")
	}
}

#[derive(Debug, Clone)]
pub struct ProxyGetRequest<S> {
	inner: S,
	path: Arc<str>,
	method: Arc<str>,
	params: Arc<Vec<String>>,
}

impl<S> ProxyGetRequest<S> {
	pub fn new(inner: S, path: &str, method: &str, params: &Vec<String>) -> Result<Self, jsonrpsee::core::Error> {
		if !path.starts_with('/') {
			return Err(jsonrpsee::core::Error::Custom(format!("ProxyGetRequest path must start with `/`, got: {}", path)));
		}

		Ok(Self { inner, path: Arc::from(path), method: Arc::from(method), params: Arc::from(params.clone()) })
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
		let modify = self.path.as_ref() == req.uri().path() && req.method() == Method::GET;
		if modify {
			let req_params: HashMap<String, JsonValue> = req
				.uri()
				.query()
				.map(|v| url::form_urlencoded::parse(v.as_bytes())
					.into_owned()
					.map(|(k, v)| (k, serde_json::to_value(v).expect("valid query param")))
					.collect()
				)
				.unwrap_or_else(HashMap::new);
			*req.method_mut() = Method::POST;
			*req.uri_mut() = Uri::from_static("/");
			req.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
			req.headers_mut().insert(ACCEPT, HeaderValue::from_static("application/json"));
			let params: JsonValue = self.params
				.iter()
				.map(|p| req_params.get(p).unwrap_or(&JsonValue::Null).clone())
				.collect::<Vec<JsonValue>>()
				.into();
			let params_raw = to_raw_value(&params).expect("valid params");
			let body = Body::from(
				serde_json::to_string(&RequestSer::borrowed(&Id::Number(0), &self.method, Some(params_raw.as_ref())))
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
