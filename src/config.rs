use std::{collections::HashSet, env};
use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use strum_macros::EnumString;

#[derive(Clone, EnumString, Deserialize, Serialize)]
pub enum Backend {
    Tendermint34
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Config {
    pub backend: Backend,
    pub blocked_routes: HashSet<String>,
    pub listen_addr: String,
    pub rpc_addr: String,
    pub max_connections: u32,
    pub max_subscriptions_per_connection: u32,
    pub max_request_body_size_bytes: u32,
    pub max_response_body_size_bytes: u32,
    pub ws_ping_interval_seconds: u32,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            backend: env::var(ENV_BACKEND)
                .ok()
                .map(|s| s.parse())
                .transpose()?
                .unwrap_or(DEFAULT_BACKEND),
            blocked_routes: HashSet::from_iter(env::var(ENV_BLOCKED_ROUTES)
                .unwrap_or(DEFAULT_BLOCKED_ROUTES.to_string())
                .split(',')
                .map(|s| s.to_string()).collect::<Vec<String>>()),
            listen_addr: env::var(ENV_LISTEN_ADDR)
                .unwrap_or(DEFAULT_LISTEN_ADDR.to_string()),
            rpc_addr: env::var(ENV_RPC_ADDR)
                .map_err(|_| anyhow!("missing required environment variable: {}", ENV_RPC_ADDR))?,
            max_connections: env::var(ENV_MAX_CONNECTIONS)
                .ok()
                .map(|s| s.parse())
                .transpose()?
                .unwrap_or(DEFAULT_MAX_CONNECTIONS),
            max_subscriptions_per_connection: env::var(ENV_MAX_SUBSCRIPTIONS_PER_CONNECTION)
                .ok()
                .map(|s| s.parse())
                .transpose()?
                .unwrap_or(DEFAULT_MAX_SUBSCRIPTIONS_PER_CONNECTION),
            max_request_body_size_bytes: env::var(ENV_MAX_REQUEST_BODY_SIZE_BYTES)
                .ok()
                .map(|s| s.parse())
                .transpose()?
                .unwrap_or(DEFAULT_MAX_REQUEST_BODY_SIZE_BYTES),
            max_response_body_size_bytes: env::var(ENV_MAX_RESPONSE_BODY_SIZE_BYTES)
                .ok()
                .map(|s| s.parse())
                .transpose()?
                .unwrap_or(DEFAULT_MAX_RESPONSE_BODY_SIZE_BYTES),
            ws_ping_interval_seconds: env::var(ENV_WS_PING_INTERVAL_SECONDS)
                .ok()
                .map(|s| s.parse())
                .transpose()?
                .unwrap_or(DEFAULT_WS_PING_INTERVAL_SECONDS),
        })
    }
}

pub const ENV_BACKEND: &str = "IGLTC_BACKEND";
pub const ENV_BLOCKED_ROUTES: &str = "IGLTC_BLOCKED_ROUTES";
pub const ENV_LISTEN_ADDR: &str = "IGLTC_LISTEN_ADDR";
pub const ENV_RPC_ADDR: &str = "IGLTC_RPC_ADDR";
pub const ENV_MAX_CONNECTIONS: &str = "IGLTC_MAX_CONNECTIONS";
pub const ENV_MAX_SUBSCRIPTIONS_PER_CONNECTION: &str = "IGLTC_MAX_SUBSCRIPTIONS_PER_CONNECTION";
pub const ENV_MAX_REQUEST_BODY_SIZE_BYTES: &str = "IGLTC_MAX_REQUEST_BODY_SIZE_BYTES";
pub const ENV_MAX_RESPONSE_BODY_SIZE_BYTES: &str = "IGLTC_MAX_RESPONSE_BODY_SIZE_BYTES";
pub const ENV_WS_PING_INTERVAL_SECONDS: &str = "IGLTC_WS_PING_INTERVAL_SECONDS";

pub const DEFAULT_BACKEND: Backend = Backend::Tendermint34;
pub const DEFAULT_BLOCKED_ROUTES: &str = "";
pub const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:8080";
pub const DEFAULT_MAX_CONNECTIONS: u32 = 1000;
pub const DEFAULT_MAX_SUBSCRIPTIONS_PER_CONNECTION: u32 = 5;
pub const DEFAULT_MAX_REQUEST_BODY_SIZE_BYTES: u32 = 1024 * 1024;
pub const DEFAULT_MAX_RESPONSE_BODY_SIZE_BYTES: u32 = 1024 * 1024;
pub const DEFAULT_WS_PING_INTERVAL_SECONDS: u32 = 30;

lazy_static! {
    pub static ref CONFIG: Config = Config::from_env().expect("failed reading environment config");
}