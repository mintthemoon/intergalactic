use lazy_static::lazy_static;
use crate::config::CONFIG;
use crate::comet34::Comet34Backend;

lazy_static! {
    pub static ref COMET34: Comet34Backend = CONFIG.clone().try_into().expect("failed to init comet34 backend");
}
