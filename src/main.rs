mod proxy;

use std::{env, net::SocketAddr};
use anyhow::{Result, Error};
use jsonrpsee::{
	core::client::ClientT,
	http_client::HttpClientBuilder,
	server::{RpcModule, ServerBuilder},
	rpc_params,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing_subscriber::{util::SubscriberInitExt, EnvFilter, FmtSubscriber};
use tokio::signal::ctrl_c;
use tower::ServiceBuilder;
use crate::proxy::ProxyGetRequestLayer;

#[tokio::main]
async fn main() -> Result<()> {
	if env::var("RUST_LOG").is_err() {
		env::set_var("RUST_LOG", "info");
	}
    let filter = EnvFilter::try_from_default_env()?;
	FmtSubscriber::builder().with_env_filter(filter).finish().try_init()?;
    if let Err(e) = run_server().await {
		tracing::error!("fatal error: {}", e);
	}
	Ok(())
}
#[derive(Deserialize, Serialize, Clone)]
struct Tendermint34NodeInfo {
	protocol_version: Tendermint34ProtocolVersion,
	id: String,
	listen_addr: String,
	network: String,
	version: String,
	channels: String,
	moniker: String,
	other: Tendermint34Other,
}

#[derive(Deserialize, Serialize, Clone)]
struct Tendermint34ProtocolVersion {
	p2p: String,
	block: String,
	app: String,
}

#[derive(Deserialize, Serialize, Clone)]
struct Tendermint34Other {
	tx_index: String,
	rpc_address: String,
}

#[derive(Deserialize, Serialize, Clone)]
struct Tendermint34SyncInfo {
	latest_block_hash: String,
	latest_app_hash: String,
	latest_block_height: String,
	latest_block_time: String,
	earliest_block_hash: String,
	earliest_app_hash: String,
	earliest_block_height: String,
	earliest_block_time: String,
	catching_up: bool,
}

#[derive(Deserialize, Serialize, Clone)]
struct Tendermint34ValidatorInfo {
	address: String,
	pub_key: Tendermint34PubKey,
	voting_power: String,
}

#[derive(Deserialize, Serialize, Clone)]
struct Tendermint34PubKey {
	#[serde(rename = "type")]
	type_: String,
	value: String,
}

#[derive(Deserialize, Serialize, Clone)]
struct Tendermint34Status {
	node_info: Tendermint34NodeInfo,
	sync_info: Tendermint34SyncInfo,
	validator_info: Tendermint34ValidatorInfo,
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

async fn status() -> Result<Value, jsonrpsee::core::Error> {
	let client = HttpClientBuilder::default().build("https://rpc-kujira.mintthemoon.xyz:443")?;
	let res = client.request("status", rpc_params![]).await?;
	let status: Tendermint34Status = serde_json::from_value(res)?;
	serde_json::to_value(status.strip_sensitive_info()).map_err(jsonrpsee::core::Error::from)
}

async fn run_server() -> Result<()> {
	let service_builder = ServiceBuilder::default()
		.layer(ProxyGetRequestLayer::new("/status", "status")?);
	let server = ServerBuilder::default()
		.set_middleware(service_builder)
		.build("127.0.0.1:8080".parse::<SocketAddr>()?).await?;
	let mut module = RpcModule::new(());
	module.register_async_method("status", |_, _| status())?;
	let handle = server.start(module)?;
	tracing::info!("server started");
    ctrl_c().await?;
	tracing::info!("received SIGINT, shutting down...");
	handle.stop().map_err(Error::from)
}