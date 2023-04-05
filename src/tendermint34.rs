use std::{net::SocketAddr, sync::Arc};
use anyhow::{Result, Error};
use jsonrpsee::{
	core::client::ClientT,
	http_client::{HttpClientBuilder, HttpClient},
	server::{RpcModule, ServerBuilder},
    rpc_params,
	types::Params,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::signal::ctrl_c;
use tower::ServiceBuilder;
use crate::proxy::ProxyGetRequestLayer;

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
	pub http: Arc<HttpClient>,
	pub url: String,
}

impl Tendermint34Backend {
	pub fn new(url: &str, listen_addr: &str) -> Result<Self> {
		Ok(Self {
			listen_addr: listen_addr.parse::<SocketAddr>()?,
			http: Arc::new(HttpClientBuilder::default().build(&url)?),
			url: url.to_string(),
		})
	}

	pub async fn start(&'static self) -> Result<()> {
		let service_builder = ServiceBuilder::default()
			.layer(ProxyGetRequestLayer::new("/status", "status")?);
		let server = ServerBuilder::default()
			.set_middleware(service_builder)
			.build(self.listen_addr).await?;
		let mut module = RpcModule::new(());
		module.register_async_method("status", |_, _| self.status())?;
		let handle = server.start(module)?;
		tracing::info!("server started");
		ctrl_c().await?;
		tracing::info!("received SIGINT, shutting down...");
		handle.stop().map_err(Error::from)
	}

	pub async fn status(&'static self) -> Result<Value, jsonrpsee::core::Error> {
		let res = self.http.request("status", rpc_params![]).await?;
		let status: Tendermint34Status = serde_json::from_value(res)?;
		serde_json::to_value(status.strip_sensitive_info()).map_err(jsonrpsee::core::Error::from)
	}
}
