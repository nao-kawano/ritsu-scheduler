//!
//! Client config for Ritsu.
//!

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

/* -------------------------------------------------------------------------- */

/// Configuration for Ritsu client retry logic and timeouts.
pub struct RtClientConfig {
    /// Seconds to wait before retrying a Join request.
    pub retry_sec_join: f64,
    /// Number of times to retry a Join request.
    pub retry_count_join: u32,

    /// Seconds to wait before retrying a Ready request during startup.
    pub retry_sec_ready_startup: f64,
    /// Number of times to retry a Ready request during startup.
    pub retry_count_ready_startup: u32,

    /// Seconds to wait before retrying a Ready request during normal execution.
    /// Default is `run_cycle_sec * 2.2` to accommodate skipped cycles and dependency execution times.
    pub retry_sec_ready: f64,
    /// Number of times to retry a Ready request during normal execution.
    pub retry_count_ready: u32,

    /// Seconds to wait before retrying a Done request.
    pub retry_sec_done: f64,
    /// Number of times to retry a Done request.
    pub retry_count_done: u32,

    /// Seconds to wait before retrying an Exit request.
    pub retry_sec_exit: f64,
    /// Number of times to retry an Exit request.
    pub retry_count_exit: u32,
}

impl RtClientConfig {
    /// Default timeout in seconds for Ready requests during startup.
    pub const TIMEOUT_SEC_READY_STARTUP_DEFAULT: f64 = 5.0;

    /// Creates a new RtClientConfig with default values based on the run cycle and startup wait time.
    ///
    /// # Arguments
    ///
    /// * `run_cycle_sec` - The expected execution cycle of the client in seconds.
    ///   For example, if the server's Cycle Time is 50ms and the client's Cycle is 2, set this to 0.1 (100ms).
    /// * `startup_wait_sec` - The total time to wait during the startup phase in seconds.
    pub fn new(run_cycle_sec: f64, startup_wait_sec: f64) -> Self {
        RtClientConfig {
            retry_sec_join: 0.003,
            retry_count_join: 5,
            retry_sec_ready_startup: RtClientConfig::TIMEOUT_SEC_READY_STARTUP_DEFAULT,
            retry_count_ready_startup: (startup_wait_sec
                / RtClientConfig::TIMEOUT_SEC_READY_STARTUP_DEFAULT)
                as u32,
            retry_sec_ready: run_cycle_sec * 2.2,
            retry_count_ready: 3,
            retry_sec_done: 0.003,
            retry_count_done: 5,
            retry_sec_exit: 0.003,
            retry_count_exit: 5,
        }
    }
}
