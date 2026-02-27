#![feature(never_type)]

use std::{collections::HashMap, io, time::Duration};

use async_stream::stream;
use futures::{Stream, StreamExt, stream};
use tokio::{
	spawn,
	sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, broadcast::error::RecvError},
	task::JoinHandle,
	time::timeout,
};
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, error, info_span};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tryfol_daemon::{
	modules::{Module, test::TestMod},
	tracing::LogStore,
};
use tryfol_ipc::daemon_control::{
	self, LogsError, ModuleStatus, Server, StartError, StatusError, StopError,
};

type ModulesMap = HashMap<String, Box<dyn Module + Send + Sync>>;
type RunningModulesMap = HashMap<String, ModuleState>;

struct App {
	modules: RwLock<ModulesMap>,
	running_modules: RwLock<RunningModulesMap>,
	log_store: LogStore,
}

enum ModuleState {
	Running(CancellationToken, JoinHandle<anyhow::Result<()>>),
	Crashed,
}

#[tokio::main]
async fn main() -> () {
	let app = App::new();
	app.register(TestMod).await;

	let Err(e) = app.run().await;
	error!("{e}");
}

impl App {
	pub fn new() -> Self {
		let log_store = LogStore::default();
		tracing_subscriber::registry()
			.with(tracing_subscriber::fmt::layer())
			.with(log_store.layer())
			.init();

		Self {
			modules: RwLock::default(),
			running_modules: RwLock::default(),
			log_store,
		}
	}

	pub async fn register<T: Module + Send + Sync + 'static>(&self, module: T) {
		self.modules
			.write()
			.await
			.insert(T::name().to_string(), Box::new(module));
	}

	pub async fn run(self) -> io::Result<!> {
		// drop RwLock guard at the end of the scope
		{
			let modules = self.modules.read().await;
			let mut running_modules = self.running_modules.write().await;
			for module in modules.keys().cloned() {
				// the only error that can be returned from start_module is StartError::NotFound,
				// which isn't possible here
				let _ = Self::start_module(module, &modules, &mut running_modules);
			}
		}
		self.serve().await
	}

	fn start_module(
		name: String,
		modules: &RwLockReadGuard<'_, ModulesMap>,
		running_modules: &mut RwLockWriteGuard<'_, RunningModulesMap>,
	) -> Result<(), StartError> {
		modules
			.get(&name)
			.map_or(Err(StartError::NotFound), |module| {
				let token = CancellationToken::new();

				let future = module.run(token.clone());
				let handle = spawn(
					async {
						if let Err(e) = future.await {
							error!("{e}");
							return Err(e);
						}
						Ok(())
					}
					.instrument(info_span!("module", module = name)),
				);
				running_modules.insert(name, ModuleState::Running(token, handle));
				Ok(())
			})
	}
}

impl daemon_control::Server for App {
	#[expect(clippy::significant_drop_tightening, reason = "false positive")]
	async fn start(&self, module: String) -> Result<(), StartError> {
		let mut running_modules = self.running_modules.write().await;
		let is_running = running_modules.get(&module).is_some_and(|x| {
			if let ModuleState::Running(_, handle) = x
				&& !handle.is_finished()
			{
				true
			} else {
				false
			}
		});
		if is_running {
			return Err(StartError::AlreadyRunning);
		}
		Self::start_module(module, &self.modules.read().await, &mut running_modules)
	}

	async fn stop(&self, module: String) -> Result<(), StopError> {
		let mut running_modules = self.running_modules.write().await;
		if let Some(state) = running_modules.remove(&module) {
			let ModuleState::Running(token, handle) = state else {
				return Err(StopError::NotRunning);
			};
			if handle.is_finished() {
				return Err(StopError::NotRunning);
			}

			token.cancel();
			let abort_handle = handle.abort_handle();
			timeout(Duration::from_secs(10), handle).await.map_or_else(
				|_| {
					abort_handle.abort();
					Err(StopError::ForceStopped)
				},
				|x| {
					#[expect(
						clippy::unwrap_used,
						reason = "error if panicked (propagate) or already aborted (not possible)"
					)]
					let result = x.unwrap();
					if result.is_err() {
						running_modules.insert(module, ModuleState::Crashed);
					}
					Ok(())
				},
			)
		} else if self.modules.read().await.contains_key(&module) {
			Err(StopError::NotRunning)
		} else {
			Err(StopError::NotFound)
		}
	}

	async fn status(&self, module: String) -> Result<ModuleStatus, StatusError> {
		let mut running_modules = self.running_modules.write().await;
		if let Some(state) = running_modules.get_mut(&module) {
			let handle = match state {
				ModuleState::Running(_, handle) => handle,
				ModuleState::Crashed => return Ok(ModuleStatus::Crashed),
			};
			if !handle.is_finished() {
				return Ok(ModuleStatus::Running);
			}

			if handle.await.is_err() {
				running_modules.insert(module, ModuleState::Crashed);
				drop(running_modules);
				Ok(ModuleStatus::Crashed)
			} else {
				Ok(ModuleStatus::Running)
			}
		} else if self.modules.read().await.contains_key(&module) {
			Ok(ModuleStatus::Stopped)
		} else {
			Err(StatusError::NotFound)
		}
	}

	async fn logs(
		&self,
		module: String,
		lines: Option<u64>,
	) -> Result<impl Stream<Item = String>, LogsError> {
		if !self.modules.read().await.contains_key(&module) {
			return Err(LogsError::NotFound);
		}

		let (stored, mut rx) = self.log_store.tail(module, lines);
		Ok(stream::iter(stored).chain(stream! {
			loop {
				yield match rx.recv().await {
					Ok(x) => x,
					Err(RecvError::Lagged(count)) => format!("Receiver lagged ! Missing {count} log lines"),
					Err(RecvError::Closed) => break,
				}
			}
		}))
	}
}
