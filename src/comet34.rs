use std::{collections::{HashMap, HashSet}, net::SocketAddr, time::Duration};
use anyhow::{anyhow, Result, Error};
use hyper::{Body, Request, Method};
use jsonrpsee::{
	core::{client::ClientT, params::ArrayParams, Error as RpcError},
	http_client::{HttpClientBuilder, HttpClient},
	server::{RpcModule, ServerBuilder},
    rpc_params,
	types::{error::CallError, Params},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::signal::ctrl_c;
use tower::ServiceBuilder;
use tower_http::cors::{CorsLayer, Any};
use crate::config::Config;
use crate::proxy::{ProxyGetRequestParamsLayer, ProxyGetRequestCustomLayer};

#[derive(Deserialize, Serialize, Clone)]
pub struct Comet34NodeInfo {
	pub protocol_version: Comet34ProtocolVersion,
	pub id: String,
	pub listen_addr: String,
	pub network: String,
	pub version: String,
	pub channels: String,
	pub moniker: String,
	pub other: Comet34Other,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Comet34ProtocolVersion {
	pub p2p: String,
	pub block: String,
	pub app: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Comet34Other {
	pub tx_index: String,
	pub rpc_address: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Comet34SyncInfo {
	pub latest_block_hash: String,
	pub latest_app_hash: String,
	pub latest_block_height: String,
	pub latest_block_time: String,
	pub earliest_block_hash: String,
	pub earliest_app_hash: String,
	pub earliest_block_height: String,
	pub earliest_block_time: String,
	pub catching_up: bool,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Comet34ValidatorInfo {
	pub address: String,
	pub pub_key: Comet34PubKey,
	pub voting_power: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Comet34PubKey {
	#[serde(rename = "type")]
	pub type_: String,
	pub value: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Comet34Status {
	pub node_info: Comet34NodeInfo,
	pub sync_info: Comet34SyncInfo,
	pub validator_info: Comet34ValidatorInfo,
}

impl Comet34Status {
	pub fn strip_sensitive_info(&self) -> Self {
		let mut status = self.clone();
		status.node_info.moniker = "".to_string();
		status.node_info.listen_addr = "".to_string();
		status.node_info.other.rpc_address = "".to_string();
		status.node_info.version = "".to_string();
		status
	}
}

pub type Comet34Params = Vec<String>;

pub fn make_params(params: Vec<impl Into<String>>) -> Comet34Params {
	params.into_iter().map(Into::into).collect()
}

pub struct Comet34Route {
	pub method: String,
	pub params: Comet34Params,
}

impl Comet34Route {
	pub fn new(method: String, params: Comet34Params) -> Self {
		Self {
			method,
			params,
		}
	}

	pub fn proxy_get_layer(&self, path: String) -> Result<ProxyGetRequestParamsLayer, RpcError> {
		ProxyGetRequestParamsLayer::new(path, self.method.clone(), self.params.clone())
	}

	pub fn register_method(&'static self, backend: &'static Comet34Backend, module: &mut RpcModule<()>) -> Result<(), RpcError> {
		if !backend.blocked_routes.contains(&self.method) {
			if self.method == "status" {
				module.register_async_method(&self.method, |_, _| backend.status())?;
			} else {
				module.register_async_method(&self.method, |p, _| backend.proxy_call(&self.method, p))?;
			}
		}
		Ok(())
	}
}

pub struct Comet34Backend {
	pub blocked_routes: HashSet<String>,
	pub listen_addr: SocketAddr,
	pub http: HttpClient,
	pub routes: HashMap<String, Comet34Route>,
	pub url: String,
	pub max_connections: u32,
    pub max_subscriptions_per_connection: u32,
    pub max_request_body_size_bytes: u32,
    pub max_response_body_size_bytes: u32,
    pub ws_ping_interval_seconds: u32,
}

impl TryFrom<Config> for Comet34Backend {
	type Error = Error;

	fn try_from(config: Config) -> Result<Self> {
		Self::new(
			&config.rpc_addr,
			&config.listen_addr,
			&config.blocked_routes,
			config.max_connections,
			config.max_subscriptions_per_connection,
			config.max_request_body_size_bytes,
			config.max_response_body_size_bytes,
			config.ws_ping_interval_seconds,
		)
	}
}

impl Comet34Backend {
	pub fn new(
		url: &str,
		listen_addr: &str,
		blocked_routes: &HashSet<String>,
		max_connections: u32,
    	max_subscriptions_per_connection: u32,
    	max_request_body_size_bytes: u32,
    	max_response_body_size_bytes: u32,
   		ws_ping_interval_seconds: u32,
	) -> Result<Self> {
		let mut backend = Self {
			blocked_routes: blocked_routes.clone(),
			listen_addr: listen_addr.parse()?,
			http: HttpClientBuilder::default().build(&url)?,
			routes: HashMap::new(),
			url: url.to_string(),
			max_connections,
			max_subscriptions_per_connection,
			max_request_body_size_bytes,
			max_response_body_size_bytes,
			ws_ping_interval_seconds,
		};
		backend.add_proxy_route("/abci_info", "abci_info", vec![]);
		backend.add_proxy_route("/abci_query", "abci_query", make_params(vec!["path", "data", "height", "prove"]));
		backend.add_proxy_route("/block", "block", make_params(vec!["height"]));
		backend.add_proxy_route("/block_by_hash", "block_by_hash", make_params(vec!["hash"]));
		backend.add_proxy_route("/block_results", "block_results", make_params(vec!["height"]));
		backend.add_proxy_route("/block_search", "block_search", make_params(vec!["query", "page", "per_page", "order_by", "match_events"]));
		backend.add_proxy_route("/blockchain", "blockchain", make_params(vec!["minHeight", "maxHeight"]));
		backend.add_proxy_route("/broadcast_evidence", "broadcast_evidence", make_params(vec!["evidence"]));
		backend.add_proxy_route("/broadcast_tx_async", "broadcast_tx_async", make_params(vec!["tx"]));
		backend.add_proxy_route("/broadcast_tx_commit", "broadcast_tx_commit", make_params(vec!["tx"]));
		backend.add_proxy_route("/broadcast_tx_sync", "broadcast_tx_sync", make_params(vec!["tx"]));
		backend.add_proxy_route("/check_tx", "check_tx", make_params(vec!["tx"]));
		backend.add_proxy_route("/commit", "commit", make_params(vec!["height"]));
		backend.add_proxy_route("/consensus_params", "consensus_params", make_params(vec!["height"]));
		backend.add_proxy_route("/consensus_state", "consensus_state", vec![]);
		backend.add_proxy_route("/dump_consensus_state", "dump_consensus_state", vec![]);
		backend.add_proxy_route("/genesis", "genesis", vec![]);
		backend.add_proxy_route("/genesis_chunked", "genesis_chunked", make_params(vec!["chunk"]));
		backend.add_proxy_route("/health", "health", vec![]);
		backend.add_proxy_route("/net_info", "net_info", vec![]);
		backend.add_proxy_route("/num_unconfirmed_txs", "num_unconfirmed_txs", vec![]);
		backend.add_proxy_route("/status", "status", vec![]);
		backend.add_proxy_route("/subscribe", "subscribe", make_params(vec!["query"]));
		backend.add_proxy_route("/tx", "tx", make_params(vec!["hash", "prove"]));
		backend.add_proxy_route("/tx_search", "tx_search", make_params(vec!["query", "page", "per_page", "order_by", "match_events"]));
		backend.add_proxy_route("/unconfirmed_txs", "unconfirmed_txs", make_params(vec!["limit"]));
		backend.add_proxy_route("/unsubscribe_all", "unsubscribe_all", vec![]);
		backend.add_proxy_route("/unsubscribe", "unsubscribe", make_params(vec!["query"]));
		backend.add_proxy_route("/validators", "validators", make_params(vec!["height", "page", "per_page"]));
		Ok(backend)
	}

	pub fn add_proxy_route(&mut self, path: impl Into<String>, method: impl Into<String>, params: Comet34Params) {
		self.routes.insert(path.into(), Comet34Route::new(method.into(), params));
	}

	pub fn route_proxy_layer(&self, path: impl Into<String>) -> Result<ProxyGetRequestParamsLayer> {
		let path = path.into();
		let route = self.routes.get(&path).ok_or(anyhow!("route not found: {}", path))?;
		route.proxy_get_layer(path).map_err(Error::from)
	}

	pub async fn start(&'static self) -> Result<()> {
		let service_builder = ServiceBuilder::default()
			.layer(self.route_proxy_layer("/abci_info")?)
			.layer(self.route_proxy_layer("/abci_query")?)
			.layer(self.route_proxy_layer("/block")?)
			.layer(self.route_proxy_layer("/block_by_hash")?)
			.layer(self.route_proxy_layer("/block_results")?)
			.layer(self.route_proxy_layer("/block_search")?)
			.layer(self.route_proxy_layer("/blockchain")?)
			.layer(self.route_proxy_layer("/broadcast_evidence")?)
			.layer(self.route_proxy_layer("/broadcast_tx_async")?)
			.layer(self.route_proxy_layer("/broadcast_tx_commit")?)
			.layer(self.route_proxy_layer("/broadcast_tx_sync")?)
			.layer(self.route_proxy_layer("/check_tx")?)
			.layer(self.route_proxy_layer("/commit")?)
			.layer(self.route_proxy_layer("/consensus_params")?)
			.layer(self.route_proxy_layer("/consensus_state")?)
			.layer(self.route_proxy_layer("/dump_consensus_state")?)
			.layer(self.route_proxy_layer("/genesis")?)
			.layer(self.route_proxy_layer("/genesis_chunked")?)
			.layer(self.route_proxy_layer("/health")?)
			.layer(self.route_proxy_layer("/net_info")?)
			.layer(self.route_proxy_layer("/num_unconfirmed_txs")?)
			.layer(self.route_proxy_layer("/status")?)
			.layer(self.route_proxy_layer("/subscribe")?)
			.layer(self.route_proxy_layer("/tx")?)
			.layer(self.route_proxy_layer("/tx_search")?)
			.layer(self.route_proxy_layer("/unconfirmed_txs")?)
			.layer(self.route_proxy_layer("/unsubscribe")?)
			.layer(self.route_proxy_layer("/unsubscribe_all")?)
			.layer(self.route_proxy_layer("/validators")?)
			.layer(ProxyGetRequestCustomLayer::new("/", &root_html_proxy_call)?)
			.layer(CorsLayer::new().allow_methods(vec![Method::GET, Method::POST]).allow_origin(Any).allow_headers(Any));
		let server = ServerBuilder::default()
			.max_connections(self.max_connections)
			.max_subscriptions_per_connection(self.max_subscriptions_per_connection)
			.max_request_body_size(self.max_request_body_size_bytes)
			.max_response_body_size(self.max_response_body_size_bytes)
			.ping_interval(Duration::from_secs(self.ws_ping_interval_seconds.into()))
			.set_middleware(service_builder)
			.build(self.listen_addr).await?;
		let mut module = RpcModule::new(());
		self.routes
			.iter()
			.map(|(_, route)| route.register_method(&self, &mut module))
			.collect::<Result<Vec<_>, RpcError>>()?;
		let handle = server.start(module)?;
		tracing::info!("server started");
		ctrl_c().await?;
		tracing::info!("received SIGINT, shutting down...");
		handle.stop().map_err(Error::from)
	}

	pub async fn status(&'static self) -> Result<JsonValue, RpcError> {
		let res = self.http.request("status", rpc_params![]).await?;
		let status: Comet34Status = serde_json::from_value(res)?;
		serde_json::to_value(status.strip_sensitive_info()).map_err(RpcError::from)
	}

	pub async fn proxy_call(&'static self, method: &str, params: Params<'static>) -> Result<JsonValue, RpcError> {
		let params_json: JsonValue = params.parse()?;
		let method_params = &self.routes.get(&format!("/{}", method))
			.ok_or(RpcError::MethodNotFound(method.to_string()))?.params;
		let params = match params_json {
			JsonValue::Object(o) => {
				let mut params = ArrayParams::new();
				method_params.iter()
					.map(|p| params.insert(o.get(p)
					.unwrap_or(&JsonValue::Null)))
					.collect::<Result<Vec<()>, serde_json::Error>>()?;
				params
			},
			JsonValue::Array(a) => {
				if a.len() != method_params.len() {
					return Err(RpcError::Call(CallError::InvalidParams(
						anyhow!("expected {} parameter(s) [{}], got {}", method_params.len(), method_params.join(", "), a.len())
					)));
				}
				let mut params = ArrayParams::new();
				a.iter().map(|p| params.insert(p)).collect::<Result<Vec<()>, serde_json::Error>>()?;
				params
			},
			_ => ArrayParams::new(),
		};
		self.http.request(method, params).await
	}
}

pub fn root_html_proxy_call(req: &Request<Body>) -> String {
	let host: &str = req.headers().get("Host").map(|v| v.to_str().unwrap_or_default()).unwrap_or_default();
	root_html(&format!("//{}", host))
}

pub fn root_html(base: &str) -> String {
	format!(
		r#"<html><body><br>Available endpoints:<br><br>Endpoints that require arguments:<br><a href="{base_url}/abci_info?">{base_url}/abci_info?</a></br><a href="{base_url}/abci_query?path=_&data=_&height=_&prove=_">{base_url}/abci_query?path=_&data=_&height=_&prove=_</a></br><a href="{base_url}/block?height=_">{base_url}/block?height=_</a></br><a href="{base_url}/block_by_hash?hash=_">{base_url}/block_by_hash?hash=_</a></br><a href="{base_url}/block_results?height=_">{base_url}/block_results?height=_</a></br><a href="{base_url}/block_search?query=_&page=_&per_page=_&order_by=_&match_events=_">{base_url}/block_search?query=_&page=_&per_page=_&order_by=_&match_events=_</a></br><a href="{base_url}/blockchain?minHeight=_&maxHeight=_">{base_url}/blockchain?minHeight=_&maxHeight=_</a></br><a href="{base_url}/broadcast_evidence?evidence=_">{base_url}/broadcast_evidence?evidence=_</a></br><a href="{base_url}/broadcast_tx_async?tx=_">{base_url}/broadcast_tx_async?tx=_</a></br><a href="{base_url}/broadcast_tx_commit?tx=_">{base_url}/broadcast_tx_commit?tx=_</a></br><a href="{base_url}/broadcast_tx_sync?tx=_">{base_url}/broadcast_tx_sync?tx=_</a></br><a href="{base_url}/check_tx?tx=_">{base_url}/check_tx?tx=_</a></br><a href="{base_url}/commit?height=_">{base_url}/commit?height=_</a></br><a href="{base_url}/consensus_params?height=_">{base_url}/consensus_params?height=_</a></br><a href="{base_url}/consensus_state?">{base_url}/consensus_state?</a></br><a href="{base_url}/dump_consensus_state?">{base_url}/dump_consensus_state?</a></br><a href="{base_url}/genesis?">{base_url}/genesis?</a></br><a href="{base_url}/genesis_chunked?chunk=_">{base_url}/genesis_chunked?chunk=_</a></br><a href="{base_url}/health?">{base_url}/health?</a></br><a href="{base_url}/net_info?">{base_url}/net_info?</a></br><a href="{base_url}/num_unconfirmed_txs?">{base_url}/num_unconfirmed_txs?</a></br><a href="{base_url}/status?">{base_url}/status?</a></br><a href="{base_url}/subscribe?query=_">{base_url}/subscribe?query=_</a></br><a href="{base_url}/tx?hash=_&prove=_">{base_url}/tx?hash=_&prove=_</a></br><a href="{base_url}/tx_search?query=_&prove=_&page=_&per_page=_&order_by=_&match_events=_">{base_url}/tx_search?query=_&prove=_&page=_&per_page=_&order_by=_&match_events=_</a></br><a href="{base_url}/unconfirmed_txs?limit=_">{base_url}/unconfirmed_txs?limit=_</a></br><a href="{base_url}/unsubscribe?query=_">{base_url}/unsubscribe?query=_</a></br><a href="{base_url}/unsubscribe_all?">{base_url}/unsubscribe_all?</a></br><a href="{base_url}/validators?height=_&page=_&per_page=_">{base_url}/validators?height=_&page=_&per_page=_</a></br></body></html>"#,
		base_url = base,
	)
}