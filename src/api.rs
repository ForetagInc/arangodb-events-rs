use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// This data comes from doing an HTTP request to ArangoDB:
///
/// **`GET /_api/replication/logger-state`**
///
/// This data is gonna be used only on the initialization of a Trigger to retrieve information to
/// be used on the Trigger workflow
#[derive(Serialize, Deserialize)]
pub(crate) struct LoggerStateData {
	pub(crate) state: LoggerState,
}

/// State property coming from [`LoggerStateData`]
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

/// All log types supported for ArangoDB replication API
#[derive(Serialize, Deserialize)]
pub(crate) enum LogType {
	/// Create a database
	CreateDatabase = 1100,
	/// Drop a database
	DropDatabase = 1101,
	/// Create a collection
	CreateCollection = 2000,
	/// Drop a collection
	DropCollection = 2001,
	/// Rename a collection
	RenameCollection = 2002,
	/// Change collection properties
	ChangeCollection = 2003,
	/// Truncate a collection
	TruncateCollection = 2004,
	/// Create an index
	CreateIndex = 2100,
	/// Drop an index
	DropIndex = 2101,
	/// Create a view
	CreateView = 2110,
	/// Drop a view
	DropView = 2111,
	/// Change view properties (including the name)
	ChangeView = 2112,
	/// Mark the beginning of a transaction
	StartTransaction = 2200,
	/// Mark the successful end of a transaction
	CommitTransaction = 2201,
	/// Mark the abortion of a transaction
	AbortTransaction = 2202,
	/// Insert or replace a document
	InsertOrReplaceDocument = 2300,
	/// Remove a document
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

/// JSON structure for [`LogType::InsertOrReplaceDocument`] and [`LogType::RemoveDocument`] log
/// types coming from doing an HTTP request to ArangoDB:
///
/// **`GET /_api/replication/logger-follow`**
///
/// This data is then gonna be dispatched to event handlers
#[derive(Serialize, Deserialize, Debug)]
pub struct DocumentOperation {
	#[serde(rename = "cname")]
	pub collection: String,
	pub data: JsonValue,
}
