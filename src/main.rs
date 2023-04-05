mod proxy;
mod tendermint34;

use std::{env, net::SocketAddr};
use anyhow::{Result, Error};
use jsonrpsee::server::{RpcModule, ServerBuilder};
use tracing_subscriber::{util::SubscriberInitExt, EnvFilter, FmtSubscriber};
use tokio::signal::ctrl_c;
use tower::ServiceBuilder;
use crate::proxy::ProxyGetRequestLayer;
use crate::tendermint34::status;

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