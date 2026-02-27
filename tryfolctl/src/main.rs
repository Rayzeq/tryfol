use std::{
    env,
    io::{self, ErrorKind, Stdout, Write, stdout},
    path::PathBuf,
    pin::pin,
    process::{ChildStdin, Command as StdCommand, Stdio},
};

use clap::{Parser, Subcommand};
use futures::StreamExt;
use terminal_size::terminal_size_of;
use tryfol_ipc::daemon_control::{
    DaemonControl, LogsError, ModuleStatus, StartError, StatusError, StopError,
};
use which::which;

#[derive(Debug, Subcommand)]
enum Command {
    /// Start a module
    ///
    /// Does nothing if the module is already running.
    Start {
        /// The name of the module to start
        module: String,
    },
    /// Stop a module
    ///
    /// Does nothing if the module is not running.
    Stop {
        /// The name of the module to stop
        module: String,
    },
    /// Get the status of a module
    Status {
        /// The name of the module to query
        module: String,
    },
    /// View a module's logs in real time
    Logs {
        /// The name of the module to view logs of
        module: String,
        /// Lines of recorded logs to show before showing live logs
        lines: Option<u64>,
        /// Don't show the logs in a pager
        #[arg(short = 'P', long)]
        no_pager: bool,
    },
}

#[derive(Parser)]
struct Arguments {
    #[clap(subcommand)]
    command: Command,
}

fn find_pager() -> Option<PathBuf> {
    let mut pager = None;

    if let Some(Ok(path)) = env::var_os("PAGER").map(which) {
        pager = Some(path);
    } else if let Ok(path) = which("less") {
        pager = Some(path);
    }

    pager
}

enum Writer {
    Stdout(Stdout),
    Pager(ChildStdin),
}

#[tokio::main]
async fn main() {
    let args = Arguments::parse();

    let client = match tryfol_ipc::daemon_control::Client::new() {
        Ok(x) => x,
        Err(e) => {
            println!("Could not connect to tryfol-daemon: {e}");
            return;
        }
    };

    match args.command {
        Command::Start { module } => match client.start(&module).await {
            Ok(Ok(())) => println!("Module started succesfully"),
            Ok(Err(StartError::NotFound)) => println!("No module named {module}"),
            Ok(Err(StartError::AlreadyRunning)) => println!("Module is already running"),
            Err(e) => println!("Could not communicate with daemon: {e}"),
        },
        Command::Stop { module } => match client.stop(&module).await {
            Ok(Ok(())) => println!("Module stopped succesfully"),
            Ok(Err(StopError::NotFound)) => println!("No module named {module}"),
            Ok(Err(StopError::NotRunning)) => println!("Module wasn't running"),
            Ok(Err(StopError::ForceStopped)) => println!("Module was force stopped"),
            Err(e) => println!("Could not communicate with daemon: {e}"),
        },
        Command::Status { module } => match client.status(&module).await {
            Ok(Ok(ModuleStatus::Stopped)) => println!("Module is stopped"),
            Ok(Ok(ModuleStatus::Running)) => println!("Module is running"),
            Ok(Ok(ModuleStatus::Crashed)) => println!("Module crashed"),
            Ok(Err(StatusError::NotFound)) => println!("No module named {module}",),
            Err(e) => println!("Could not communicate with daemon: {e}"),
        },
        Command::Logs {
            module,
            lines,
            no_pager,
        } => {
            let lines =
                lines.or_else(|| terminal_size_of(io::stdout()).map(|x| u64::from(x.1.0) - 1));
            match client.logs(&module, lines).await {
                Ok(Ok(lines)) => {
                    let mut fd = if !no_pager && let Some(pager) = find_pager() {
                        let mut command = StdCommand::new(&pager);
                        if pager.file_name().is_some_and(|x| x == "less") {
                            command.arg("+F");
                        }
                        let pager = match command.stdin(Stdio::piped()).spawn() {
                            Ok(x) => x,
                            Err(e) => {
                                println!("Could not start pager: {e}");
                                return;
                            }
                        };
                        Writer::Pager(pager.stdin.expect("missing pager stdin handle"))
                    } else {
                        Writer::Stdout(stdout())
                    };

                    let mut lines = pin!(lines);
                    while let Some(line) = lines.next().await {
                        let res = match line {
                            Ok(line) => writeln!(fd, "{line}"),
                            Err(e) => writeln!(fd, "Could not read log line: {e}"),
                        };
                        if let Err(e) = res {
                            if e.kind() == ErrorKind::BrokenPipe {
                                break;
                            }
                            println!("Couldn't write log line to output: {e}");
                        }
                    }
                }
                Ok(Err(LogsError::NotFound)) => println!("No module named {module}"),
                Err(e) => println!("Could not get module logs: {e}"),
            }
        }
    }
}

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Stdout(stdout) => stdout.write(buf),
            Self::Pager(child_stdin) => child_stdin.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Stdout(stdout) => stdout.flush(),
            Self::Pager(child_stdin) => child_stdin.flush(),
        }
    }
}
