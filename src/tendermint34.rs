use std::{collections::{HashMap, HashSet}, net::SocketAddr};
use anyhow::{anyhow, Result, Error};
use hyper::{Body, Request};
use jsonrpsee::{
	core::{client::ClientT, params::ArrayParams, Error as RpcError},
	http_client::{HttpClientBuilder, HttpClient},
	server::{RpcModule, ServerBuilder},
    rpc_params,
	types::Params,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::signal::ctrl_c;
use tower::ServiceBuilder;
use crate::proxy::{ProxyGetRequestParamsLayer, ProxyGetRequestCustomLayer};

#[derive(Deserialize, Serialize, Clone)]
pub struct Tendermint34NodeInfo {
	pub protocol_version: Tendermint34ProtocolVersion,
	pub id: String,
	pub listen_addr: String,
	pub network: String,
	pub version: String,
	pub channels: String,
	pub moniker: String,
	pub other: Tendermint34Other,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Tendermint34ProtocolVersion {
	pub p2p: String,
	pub block: String,
	pub app: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Tendermint34Other {
	pub tx_index: String,
	pub rpc_address: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Tendermint34SyncInfo {
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
pub struct Tendermint34ValidatorInfo {
	pub address: String,
	pub pub_key: Tendermint34PubKey,
	pub voting_power: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Tendermint34PubKey {
	#[serde(rename = "type")]
	pub type_: String,
	pub value: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Tendermint34Status {
	pub node_info: Tendermint34NodeInfo,
	pub sync_info: Tendermint34SyncInfo,
	pub validator_info: Tendermint34ValidatorInfo,
}

impl Tendermint34Status {
	pub fn strip_sensitive_info(&self) -> Self {
		let mut status = self.clone();
		status.node_info.moniker = "".to_string();
		status.node_info.listen_addr = "".to_string();
		status.node_info.other.rpc_address = "".to_string();
		status.node_info.version = "".to_string();
		status
	}
}

pub type Tendermint34Params = Vec<String>;

pub fn make_params(params: Vec<impl Into<String>>) -> Tendermint34Params {
	params.into_iter().map(Into::into).collect()
}

pub struct Tendermint34Route {
	pub method: String,
	pub params: Tendermint34Params,
}

impl Tendermint34Route {
	pub fn new(method: String, params: Tendermint34Params) -> Self {
		Self {
			method,
			params,
		}
	}

	pub fn proxy_get_layer(&self, path: String) -> Result<ProxyGetRequestParamsLayer, RpcError> {
		ProxyGetRequestParamsLayer::new(path, self.method.clone(), self.params.clone())
	}

	pub fn register_method(&'static self, backend: &'static Tendermint34Backend, module: &mut RpcModule<()>) -> Result<(), RpcError> {
		if backend.blocked_routes.contains(&self.method) {
			module.register_method(&self.method, |_, _| Err::<JsonValue, RpcError>(RpcError::Custom("method not supported".to_string())))?;
		} else if self.method == "status" {
			module.register_async_method(&self.method, |_, _| backend.status())?;
		} else {
			module.register_async_method(&self.method, |p, _| backend.proxy_call(&self.method, p))?;
		}
		Ok(())
	}
}

pub struct Tendermint34Backend {
	pub blocked_routes: HashSet<String>,
	pub listen_addr: SocketAddr,
	pub http: HttpClient,
	pub routes: HashMap<String, Tendermint34Route>,
	pub url: String,
}

impl Tendermint34Backend {
	pub fn new(url: &str, listen_addr: &str, blocked_routes: &Vec<&str>) -> Result<Self> {
		let mut backend = Self {
			blocked_routes: HashSet::from_iter(blocked_routes.iter().map(|s| s.to_string())),
			listen_addr: listen_addr.parse()?,
			http: HttpClientBuilder::default().build(&url)?,
			routes: HashMap::new(),
			url: url.to_string(),
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

	pub fn add_proxy_route(&mut self, path: impl Into<String>, method: impl Into<String>, params: Tendermint34Params) {
		self.routes.insert(path.into(), Tendermint34Route::new(method.into(), params));
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
			.layer(ProxyGetRequestCustomLayer::new("/", &root_html_proxy_call)?);
		let server = ServerBuilder::default()
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
		let status: Tendermint34Status = serde_json::from_value(res)?;
		serde_json::to_value(status.strip_sensitive_info()).map_err(RpcError::from)
	}

	pub async fn proxy_call(&'static self, method: &str, params: Params<'static>) -> Result<JsonValue, RpcError> {
		let params_json: Vec<JsonValue> = params.parse()?;
		let mut params_arr = ArrayParams::new();
		params_json.iter().map(|p| params_arr.insert(p)).collect::<Result<Vec<()>, serde_json::Error>>()?;
		self.http.request(method, params_arr).await
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