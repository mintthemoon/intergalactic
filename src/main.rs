use std::env;
use std::net::SocketAddr;
use std::time::Duration;
use anyhow::Result;
use jsonrpsee::server::{RpcModule, ServerBuilder};
use jsonrpsee::server::middleware::proxy_get_request::ProxyGetRequestLayer;
use tracing_subscriber::util::SubscriberInitExt;
use tokio::signal::ctrl_c;
use tokio::sync::oneshot;
use tower::ServiceBuilder;

#[tokio::main]
async fn main() -> Result<()> {
	if env::var("RUST_LOG").is_err() {
		env::set_var("RUST_LOG", "info");
	}
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()?;
	tracing_subscriber::FmtSubscriber::builder().with_env_filter(filter).finish().try_init()?;
    run_server().await
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
	let (shutdown_tx, shutdown_rx) = oneshot::channel();
    tokio::spawn(async move {
        ctrl_c().await.unwrap();
        tracing::info!("received SIGINT, shutting down...");
        if let Err(err) = handle.stop() {
            tracing::error!("failed to gracefully shutdown the server: {:?}", err);
        }
        shutdown_tx.send(()).unwrap();
    });
    shutdown_rx.await?;
	Ok(())
}