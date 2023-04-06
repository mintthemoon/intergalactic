use std::{collections::HashSet, env};
use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use strum_macros::EnumString;

#[derive(EnumString, Deserialize, Serialize)]
pub enum Backend {
    Tendermint34
}

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub backend: Backend,
    pub blocked_routes: HashSet<String>,
    pub listen_addr: String,
    pub rpc_addr: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            backend: env::var(ENV_BACKEND).ok().map(|s| s.parse()).transpose()?.unwrap_or(Backend::Tendermint34),
            blocked_routes: env::var(ENV_BLOCKED_ROUTES)
                .map(|s| HashSet::from_iter(s.split(',').map(|s| s.to_string()).collect::<Vec<String>>()))
                .unwrap_or(HashSet::new()),
            listen_addr: env::var(ENV_LISTEN_ADDR).unwrap_or("127.0.0.1:8080".to_string()),
            rpc_addr: env::var(ENV_RPC_ADDR).map_err(|_| anyhow!("missing required environment variable: {}", ENV_RPC_ADDR))?,
        })
    }
}

pub const ENV_BACKEND: &str = "IGLTC_BACKEND";
pub const ENV_BLOCKED_ROUTES: &str = "IGLTC_BLOCKED_ROUTES";
pub const ENV_LISTEN_ADDR: &str = "IGLTC_LISTEN_ADDR";
pub const ENV_RPC_ADDR: &str = "IGLTC_RPC_ADDR";

lazy_static! {
    pub static ref CONFIG: Config = Config::from_env().expect("failed reading environment config");
}