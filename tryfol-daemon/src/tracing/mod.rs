mod layer;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};

pub use layer::ModuleLogLayer;
use tokio::sync::broadcast;

const MAX_STORED_LINES: usize = 1_000;

#[derive(Debug, Clone, Default)]
pub struct LogStore(Arc<Mutex<HashMap<String, ModuleLogs>>>);

#[derive(Debug)]
struct ModuleLogs {
    lines: VecDeque<String>,
    tx: broadcast::Sender<String>,
}

impl ModuleLogs {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            lines: VecDeque::new(),
            tx,
        }
    }

    fn push(&mut self, line: String) {
        if self.lines.len() >= MAX_STORED_LINES {
            self.lines.pop_front();
        }
        self.lines.push_back(line.clone());
        // error if there's no receiver, ignore it
        let _ = self.tx.send(line);
    }
}

impl LogStore {
    #[must_use]
    pub fn layer(&self) -> ModuleLogLayer {
        ModuleLogLayer(self.clone())
    }

    pub(crate) fn push(&self, module: String, line: String) {
        #[expect(clippy::unwrap_used, reason = "propagate panics")]
        self.0
            .lock()
            .unwrap()
            .entry(module)
            .or_insert_with(ModuleLogs::new)
            .push(line);
    }

    #[expect(
        clippy::unwrap_used,
        clippy::missing_panics_doc,
        reason = "propagate panics"
    )]
    pub fn tail(
        &self,
        module: String,
        n: Option<u64>,
    ) -> (Vec<String>, broadcast::Receiver<String>) {
        let mut modules = self.0.lock().unwrap();
        let module = modules.entry(module).or_insert_with(ModuleLogs::new);
        let lines = if let Some(n) = n {
            module
                .lines
                .iter()
                .rev()
                .take(n as usize)
                .rev()
                .cloned()
                .collect()
        } else {
            module.lines.iter().cloned().collect()
        };
        let rx = module.tx.subscribe();
        drop(modules);

        (lines, rx)
    }
}
