use std::net::SocketAddr;
use anyhow::{Result, Error};
use hyper::{Body, Request};
use jsonrpsee::{
	core::{client::ClientT,params::ArrayParams},
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

pub struct Tendermint34Backend {
	pub listen_addr: SocketAddr,
	pub http: HttpClient,
	pub url: String,
}

impl Tendermint34Backend {
	pub fn new(url: &str, listen_addr: &str) -> Result<Self> {
		Ok(Self {
			listen_addr: listen_addr.parse::<SocketAddr>()?,
			http: HttpClientBuilder::default().build(&url)?,
			url: url.to_string(),
		})
	}

	pub async fn start(&'static self) -> Result<()> {
		
		let service_builder = ServiceBuilder::default()
			.layer(ProxyGetRequestParamsLayer::new("/abci_info", "acbi_info", vec![])?)
			.layer(ProxyGetRequestParamsLayer::new("/abci_query", "acbi_query", vec!["path".to_string(), "data".to_string(), "height".to_string(), "prove".to_string()])?)
			.layer(ProxyGetRequestParamsLayer::new("/block", "block", vec!["height".to_string()])?)
			.layer(ProxyGetRequestParamsLayer::new("/block_by_hash", "block_by_hash", vec!["hash".to_string()])?)
			.layer(ProxyGetRequestParamsLayer::new("/block_results", "block_results", vec!["height".to_string()])?)
			.layer(ProxyGetRequestParamsLayer::new("/block_search", "block_search", vec!["query".to_string(), "page".to_string(), "per_page".to_string(), "order_by".to_string(), "match_events".to_string()])?)
			.layer(ProxyGetRequestParamsLayer::new("/blockchain", "blockchain", vec!["minHeight".to_string(), "maxHeight".to_string()])?)
			.layer(ProxyGetRequestParamsLayer::new("/broadcast_evidence", "broadcast_evidence", vec!["evidence".to_string()])?)
			.layer(ProxyGetRequestParamsLayer::new("/broadcast_tx_async", "broadcast_tx_async", vec!["tx".to_string()])?)
			.layer(ProxyGetRequestParamsLayer::new("/broadcast_tx_commit", "broadcast_tx_commit", vec!["tx".to_string()])?)
			.layer(ProxyGetRequestParamsLayer::new("/broadcast_tx_sync", "broadcast_tx_sync", vec!["tx".to_string()])?)
			.layer(ProxyGetRequestParamsLayer::new("/check_tx", "check_tx", vec!["tx".to_string()])?)
			.layer(ProxyGetRequestParamsLayer::new("/commit", "commit", vec!["height".to_string()])?)
			.layer(ProxyGetRequestParamsLayer::new("/consensus_params", "consensus_params", vec!["height".to_string()])?)
			.layer(ProxyGetRequestParamsLayer::new("/consensus_state", "consensus_state", vec![])?)
			.layer(ProxyGetRequestParamsLayer::new("/dump_consensus_state", "dump_consensus_state", vec![])?)
			.layer(ProxyGetRequestParamsLayer::new("/genesis", "genesis", vec![])?)
			.layer(ProxyGetRequestParamsLayer::new("/genesis_chunked", "genesis_chunked", vec!["chunk".to_string()])?)
			.layer(ProxyGetRequestParamsLayer::new("/health", "health", vec![])?)
			.layer(ProxyGetRequestParamsLayer::new("/net_info", "net_info", vec![])?)
			.layer(ProxyGetRequestParamsLayer::new("/num_unconfirmed_txs", "num_unconfirmed_txs", vec![])?)
			.layer(ProxyGetRequestParamsLayer::new("/status", "status", vec![])?)
			.layer(ProxyGetRequestParamsLayer::new("/subscribe", "subscribe", vec!["query".to_string()])?)
			.layer(ProxyGetRequestParamsLayer::new("/tx", "tx", vec!["hash".to_string(), "prove".to_string()])?)
			.layer(ProxyGetRequestParamsLayer::new("/tx_search", "tx_search", vec!["query".to_string(), "prove".to_string(), "page".to_string(), "per_page".to_string(), "order_by".to_string(), "match_events".to_string()])?)
			.layer(ProxyGetRequestParamsLayer::new("/unconfirmed_txs", "unconfirmed_txs", vec!["limit".to_string()])?)
			.layer(ProxyGetRequestParamsLayer::new("/unsubscribe", "unsubscribe", vec!["query".to_string()])?)
			.layer(ProxyGetRequestParamsLayer::new("/unsubscribe_all", "unsubscribe_all", vec![])?)
			.layer(ProxyGetRequestParamsLayer::new("/validators", "validators", vec!["height".to_string(), "page".to_string(), "per_page".to_string()])?)
			.layer(ProxyGetRequestCustomLayer::new("/", &root_html_proxy_call)?);
		let server = ServerBuilder::default()
			.set_middleware(service_builder)
			.build(self.listen_addr).await?;
		let mut module = RpcModule::new(());
		module.register_async_method("abci_info", |p, _| self.proxy_call("abci_info", p))?;
		module.register_async_method("abci_query", |p, _| self.proxy_call("abci_query", p))?;
		module.register_async_method("block", |p, _| self.proxy_call("block", p))?;
		module.register_async_method("block_by_hash", |p, _| self.proxy_call("block_by_hash", p))?;
		module.register_async_method("block_results", |p, _| self.proxy_call("block_results", p))?;
		module.register_method("block_search", |_, _| blocked_call())?;
		module.register_async_method("blockchain", |p, _| self.proxy_call("blockchain", p))?;
		module.register_async_method("broadcast_evidence", |p, _| self.proxy_call("broadcast_evidence", p))?;
		module.register_async_method("broadcast_tx_async", |p, _| self.proxy_call("broadcast_tx_async", p))?;
		module.register_async_method("broadcast_tx_commit", |p, _| self.proxy_call("broadcast_tx_commit", p))?;
		module.register_async_method("broadcast_tx_sync", |p, _| self.proxy_call("broadcast_tx_sync", p))?;
		module.register_async_method("check_tx", |p, _| self.proxy_call("check_tx", p))?;
		module.register_async_method("commit", |p, _| self.proxy_call("commit", p))?;
		module.register_async_method("consensus_params", |p, _| self.proxy_call("consensus_params", p))?;
		module.register_async_method("consensus_state", |p, _| self.proxy_call("consensus_state", p))?;
		module.register_async_method("dump_consensus_state", |p, _| self.proxy_call("dump_consensus_state", p))?;
		module.register_async_method("genesis", |p, _| self.proxy_call("genesis", p))?;
		module.register_async_method("genesis_chunked", |p, _| self.proxy_call("genesis_chunked", p))?;
		module.register_async_method("health", |p, _| self.proxy_call("health", p))?;
		module.register_async_method("net_info", |p, _| self.proxy_call("net_info", p))?;
		module.register_async_method("num_unconfirmed_txs", |p, _| self.proxy_call("num_unconfirmed_txs", p))?;
		module.register_async_method("status", |_, _| self.status())?;
		module.register_async_method("subscribe", |p, _| self.proxy_call("subscribe", p))?;
		module.register_async_method("tx", |p, _| self.proxy_call("tx", p))?;
		module.register_method("tx_search", |_, _| blocked_call())?;
		module.register_async_method("unconfirmed_txs", |p, _| self.proxy_call("unconfirmed_txs", p))?;
		module.register_async_method("unsubscribe", |p, _| self.proxy_call("unsubscribe", p))?;
		module.register_async_method("unsubscribe_all", |p, _| self.proxy_call("unsubscribe_all", p))?;
		module.register_async_method("validators", |p, _| self.proxy_call("validators", p))?;
		let handle = server.start(module)?;
		tracing::info!("server started");
		ctrl_c().await?;
		tracing::info!("received SIGINT, shutting down...");
		handle.stop().map_err(Error::from)
	}

	pub async fn status(&'static self) -> Result<JsonValue, jsonrpsee::core::Error> {
		let res = self.http.request("status", rpc_params![]).await?;
		let status: Tendermint34Status = serde_json::from_value(res)?;
		serde_json::to_value(status.strip_sensitive_info()).map_err(jsonrpsee::core::Error::from)
	}

	pub async fn proxy_call(&'static self, method: &str, params: Params<'static>) -> Result<JsonValue, jsonrpsee::core::Error> {
		let params_json: Vec<JsonValue> = params.parse()?;
		let mut params_arr = ArrayParams::new();
		params_json.iter().map(|p| params_arr.insert(p)).collect::<Result<Vec<()>, serde_json::Error>>()?;
		self.http.request(method, params_arr).await
	}
}

pub fn blocked_call() -> Result<JsonValue, jsonrpsee::core::Error> {
	Err(jsonrpsee::core::Error::Custom("method not supported".to_string()))
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