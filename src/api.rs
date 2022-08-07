use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct LoggerStateData {
	pub(crate) state: LoggerState,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct LoggerState {
	pub(crate) running: bool,
	#[serde(rename = "lastLogTick")]
	pub(crate) last_log_tick: String,
	#[serde(rename = "lastUncommittedLogTick")]
	pub(crate) last_uncommitted_log_tick: String,
	#[serde(rename = "totalEvents")]
	pub(crate) total_events: u128,
	pub(crate) time: String,
}
