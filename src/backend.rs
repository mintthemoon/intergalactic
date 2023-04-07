use lazy_static::lazy_static;
use crate::config::CONFIG;
use crate::tendermint34::Tendermint34Backend;

lazy_static! {
    pub static ref TENDERMINT34: Tendermint34Backend = CONFIG.clone().try_into().expect("failed to init tendermint34 backend");
}
