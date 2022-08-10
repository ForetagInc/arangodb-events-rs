use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

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

#[derive(Serialize, Deserialize)]
pub(crate) enum LogType {
	CreateDatabase = 1100,
	DropDatabase = 1101,
	CreateCollection = 2000,
	DropCollection = 2001,
	RenameCollection = 2002,
	ChangeCollection = 2003,
	TruncateCollection = 2004,
	CreateIndex = 2100,
	DropIndex = 2101,
	CreateView = 2110,
	DropView = 2111,
	ChangeView = 2112,
	StartTransaction = 2200,
	CommitTransaction = 2201,
	AbortTransaction = 2202,
	InsertOrReplaceDocument = 2300,
	RemoveDocument = 2302,
}

impl TryFrom<u16> for LogType {
	type Error = ();

	fn try_from(v: u16) -> Result<Self, Self::Error> {
		match v {
			1100 => Ok(Self::CreateDatabase),
			1101 => Ok(Self::DropDatabase),
			2000 => Ok(Self::CreateCollection),
			2001 => Ok(Self::DropCollection),
			2002 => Ok(Self::RenameCollection),
			2003 => Ok(Self::ChangeCollection),
			2004 => Ok(Self::TruncateCollection),
			2100 => Ok(Self::CreateIndex),
			2101 => Ok(Self::DropIndex),
			2110 => Ok(Self::CreateView),
			2111 => Ok(Self::DropView),
			2112 => Ok(Self::ChangeView),
			2200 => Ok(Self::StartTransaction),
			2201 => Ok(Self::CommitTransaction),
			2202 => Ok(Self::AbortTransaction),
			2300 => Ok(Self::InsertOrReplaceDocument),
			2302 => Ok(Self::RemoveDocument),
			_ => Err(()),
		}
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DocumentOperation {
	#[serde(rename = "cname")]
	pub collection: String,
	pub data: JsonValue,
}
