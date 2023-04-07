mod backend;
mod config;
mod proxy;
mod comet34;

use std::env;
use anyhow::Result;
use tracing_subscriber::{util::SubscriberInitExt, EnvFilter, FmtSubscriber};
use crate::backend::COMET34;

#[tokio::main]
async fn main() -> Result<()> {
	if env::var("RUST_LOG").is_err() {
		env::set_var("RUST_LOG", "info");
	}
    let filter = EnvFilter::try_from_default_env()?;
	FmtSubscriber::builder().with_env_filter(filter).finish().try_init()?;
    if let Err(e) = COMET34.start().await {
		tracing::error!("fatal error: {}", e);
	}
	Ok(())
}
