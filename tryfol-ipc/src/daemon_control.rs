use ipc::{Read, Write};

#[derive(Debug, Read, Write)]
pub enum StartError {
	/// Module was not found
	NotFound,
	/// Module was already running
	AlreadyRunning,
}

#[derive(Debug, Read, Write)]
pub enum StopError {
	/// Module was not found
	NotFound,
	/// Module wasn't running
	NotRunning,
	/// Module had to be force stopped because it was stopping too slowly
	ForceStopped,
}

#[derive(Debug, Read, Write)]
pub enum StatusError {
	/// Module was not found
	NotFound,
}

#[derive(Debug, Read, Write)]
pub enum LogsError {
	/// Module was not found
	NotFound,
}

#[derive(Debug, Clone, Read, Write)]
pub enum ModuleStatus {
	Stopped,
	Running,
	Crashed,
}

#[ipc::protocol(
    abstract_socket = "tryfol-daemonctl",
    client_name = Client,
    server_name = Server
)]
pub trait DaemonControl {
	async fn start(&self, module: String) -> Result<(), StartError>;
	async fn stop(&self, module: String) -> Result<(), StopError>;
	async fn status(&self, module: String) -> Result<ModuleStatus, StatusError>;

	/// Get logs from storage then send live logs
	///
	/// If `lines` is [`None`], send everything in storage, otherwise send `lines` lines of logs.
	#[stream(early_error = LogsError)]
	async fn logs(&self, module: String, lines: Option<u64>) -> String;
}
