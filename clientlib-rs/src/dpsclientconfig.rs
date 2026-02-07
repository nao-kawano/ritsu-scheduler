//!
//! Client config for DPS.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

/* -------------------------------------------------------------------------- */

pub struct DPSClientConfig {
    pub retry_time_msec_join: u32,
    pub retry_count_join: u32,

    pub retry_time_msec_ready_startup: u32,
    pub retry_count_ready_startup: u32,

    pub retry_time_msec_ready: u32,
    pub retry_count_ready: u32,

    pub retry_time_msec_done: u32,
    pub retry_count_done: u32,

    pub retry_time_msec_exit: u32,
    pub retry_count_exit: u32,
}

impl DPSClientConfig {
    pub const TIMEOUT_MS_READY_STARTUP_DEFAULT: u32 = 1000;

    pub fn new(run_cycle_time_ms: u32, startup_wait_ms: u32) -> Self {
        DPSClientConfig {
            retry_time_msec_join: 20,
            retry_count_join: 3,
            retry_time_msec_ready_startup: DPSClientConfig::TIMEOUT_MS_READY_STARTUP_DEFAULT,
            retry_count_ready_startup: startup_wait_ms
                / DPSClientConfig::TIMEOUT_MS_READY_STARTUP_DEFAULT,
            retry_time_msec_ready: run_cycle_time_ms,
            retry_count_ready: 3,
            retry_time_msec_done: 50,
            retry_count_done: 3,
            retry_time_msec_exit: 20,
            retry_count_exit: 3,
        }
    }
}
