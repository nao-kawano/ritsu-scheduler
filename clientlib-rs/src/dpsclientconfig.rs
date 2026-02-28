//!
//! Client config for DPS.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

/* -------------------------------------------------------------------------- */

pub struct DPSClientConfig {
    pub retry_sec_join: f64,
    pub retry_count_join: u32,

    pub retry_sec_ready_startup: f64,
    pub retry_count_ready_startup: u32,

    pub retry_sec_ready: f64,
    pub retry_count_ready: u32,

    pub retry_sec_done: f64,
    pub retry_count_done: u32,

    pub retry_sec_exit: f64,
    pub retry_count_exit: u32,
}

impl DPSClientConfig {
    pub const TIMEOUT_SEC_READY_STARTUP_DEFAULT: f64 = 1.0;

    pub fn new(run_cycle_sec: f64, startup_wait_sec: f64) -> Self {
        DPSClientConfig {
            retry_sec_join: 0.003,
            retry_count_join: 5,
            retry_sec_ready_startup: DPSClientConfig::TIMEOUT_SEC_READY_STARTUP_DEFAULT,
            retry_count_ready_startup: (startup_wait_sec
                / DPSClientConfig::TIMEOUT_SEC_READY_STARTUP_DEFAULT)
                as u32,
            retry_sec_ready: run_cycle_sec * 1.1,
            retry_count_ready: 3,
            retry_sec_done: 0.003,
            retry_count_done: 5,
            retry_sec_exit: 0.003,
            retry_count_exit: 5,
        }
    }
}
