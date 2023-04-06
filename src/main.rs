mod proxy;
mod tendermint34;

use std::env;
use anyhow::Result;
use lazy_static::lazy_static;
use tracing_subscriber::{util::SubscriberInitExt, EnvFilter, FmtSubscriber};
use crate::tendermint34::Tendermint34Backend;

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
	lazy_static! {
		static ref BACKEND: Tendermint34Backend = Tendermint34Backend::new("https://rpc-kujira.mintthemoon.xyz:443", "127.0.0.1:8080", &vec!["block_search", "tx_search"]).unwrap();
	}
	BACKEND.start().await
}