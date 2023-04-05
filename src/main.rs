use std::env;
use std::net::SocketAddr;
use anyhow::{Result, Error};
use jsonrpsee::server::{RpcModule, ServerBuilder};
use jsonrpsee::server::middleware::proxy_get_request::ProxyGetRequestLayer;
use tracing_subscriber::util::SubscriberInitExt;
use tokio::signal::ctrl_c;
use tower::ServiceBuilder;

#[tokio::main]
async fn main() -> Result<()> {
	if env::var("RUST_LOG").is_err() {
		env::set_var("RUST_LOG", "info");
	}
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()?;
	tracing_subscriber::FmtSubscriber::builder().with_env_filter(filter).finish().try_init()?;
    if let Err(e) = run_server().await {
		tracing::error!("fatal error: {}", e);
	}
	Ok(())
}

async fn run_server() -> Result<()> {
	let service_builder = ServiceBuilder::default()
		.layer(ProxyGetRequestLayer::new("/say_hello", "say_hello")?);
	let server = ServerBuilder::default()
		.set_middleware(service_builder)
		.build("127.0.0.1:8080".parse::<SocketAddr>()?).await?;
	let mut module = RpcModule::new(());
	module.register_method("say_hello", |_, _| Ok("hello world!"))?;
	let handle = server.start(module)?;
    ctrl_c().await?;
	tracing::info!("received SIGINT, shutting down...");
	handle.stop().map_err(Error::from)
}