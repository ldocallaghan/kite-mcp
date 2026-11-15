//! The long-lived worker process: one JSON request per line on its stdin,
//! one JSON reply per line on its stdout.

use crate::protocol::{Hello, PROTOCOL_VERSION, Reply, Request, WorkerError};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct WorkerConfig {
    /// Program that runs the worker, `Rscript` for the R worker
    pub program: PathBuf,
    /// Arguments, for the R worker the path of `kite_worker.R`
    pub args: Vec<String>,
    /// Longest time one request may take before the worker is restarted
    pub request_timeout: Duration,
    /// Longest time the worker may take to start and answer `hello`
    pub startup_timeout: Duration,
}

impl WorkerConfig {
    /// The R worker, located from `KITE_MCP_RSCRIPT` and `KITE_MCP_WORKER`
    /// or their defaults.
    pub fn r_from_env(request_timeout: Duration) -> Self {
        let program = std::env::var_os("KITE_MCP_RSCRIPT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("Rscript"));
        let script = default_worker_script();
        WorkerConfig {
            program,
            args: vec!["--vanilla".into(), script.to_string_lossy().into_owned()],
            request_timeout,
            startup_timeout: Duration::from_secs(60),
        }
    }
}

/// `KITE_MCP_WORKER` if set; otherwise `share/kite-mcp/worker/kite_worker.R`
/// next to the binary's parent directory (the Docker layout); otherwise the
/// copy in the source tree.
pub fn default_worker_script() -> PathBuf {
    if let Some(p) = std::env::var_os("KITE_MCP_WORKER") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(prefix) = exe.parent().and_then(|p| p.parent())
    {
        let installed = prefix.join("share/kite-mcp/worker/kite_worker.R");
        if installed.exists() {
            return installed;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("worker/kite_worker.R")
}

#[derive(Debug, Error, Clone)]
pub enum WorkerFailure {
    #[error("the worker could not be started: {0}")]
    Spawn(String),
    #[error("the worker did not answer within {0:?} and was restarted")]
    Timeout(Duration),
    #[error("the worker exited unexpectedly and will be restarted: {0}")]
    Crashed(String),
    #[error("the worker sent a reply that does not follow the protocol: {0}")]
    Protocol(String),
    #[error("{}", .0.message)]
    Model(WorkerError),
}

struct Proc {
    child: Child,
    stdin: ChildStdin,
    stdout: Lines<BufReader<ChildStdout>>,
}

pub struct Worker {
    config: WorkerConfig,
    proc: Mutex<Option<Proc>>,
    next_id: AtomicU64,
    restarts: AtomicU64,
    hello: Mutex<Option<Hello>>,
}

impl Worker {
    pub fn new(config: WorkerConfig) -> Self {
        Worker {
            config,
            proc: Mutex::new(None),
            next_id: AtomicU64::new(1),
            restarts: AtomicU64::new(0),
            hello: Mutex::new(None),
        }
    }

    pub fn config(&self) -> &WorkerConfig {
        &self.config
    }

    /// Number of times the worker process has been started after the first.
    pub fn restarts(&self) -> u64 {
        self.restarts.load(Ordering::Relaxed)
    }

    fn spawn(&self) -> Result<Proc, WorkerFailure> {
        let mut cmd = Command::new(&self.config.program);
        cmd.args(&self.config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .env_remove("KITE_MCP_WORKER_NO_MAIN")
            .kill_on_drop(true);
        let mut child = cmd
            .spawn()
            .map_err(|e| WorkerFailure::Spawn(format!("{}: {e}", self.config.program.display())))?;
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = BufReader::new(child.stdout.take().expect("piped stdout")).lines();
        Ok(Proc {
            child,
            stdin,
            stdout,
        })
    }

    /// Send one request and wait for its reply. A timeout, a crash or a
    /// malformed reply stops the process; the next call starts a new one.
    pub async fn call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, WorkerFailure> {
        let timeout = if method == "hello" {
            self.config.startup_timeout
        } else {
            self.config.request_timeout
        };
        let mut guard = self.proc.lock().await;
        if guard.is_none() {
            let started_before = self.next_id.load(Ordering::Relaxed) > 1;
            *guard = Some(self.spawn()?);
            if started_before {
                self.restarts.fetch_add(1, Ordering::Relaxed);
            }
        }
        let id = format!("r{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let line = serde_json::to_string(&Request {
            id: id.clone(),
            method,
            params,
        })
        .map_err(|e| WorkerFailure::Protocol(e.to_string()))?;

        let proc = guard.as_mut().expect("worker process");
        let exchange = async {
            proc.stdin.write_all(line.as_bytes()).await?;
            proc.stdin.write_all(b"\n").await?;
            proc.stdin.flush().await?;
            proc.stdout.next_line().await
        };
        let outcome = tokio::time::timeout(timeout, exchange).await;
        let reply_line = match outcome {
            Err(_) => {
                self.stop(&mut guard).await;
                return Err(WorkerFailure::Timeout(timeout));
            }
            Ok(Err(e)) => {
                self.stop(&mut guard).await;
                return Err(WorkerFailure::Crashed(e.to_string()));
            }
            Ok(Ok(None)) => {
                let status = guard
                    .as_mut()
                    .and_then(|p| p.child.try_wait().ok().flatten())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "closed its output".into());
                self.stop(&mut guard).await;
                return Err(WorkerFailure::Crashed(status));
            }
            Ok(Ok(Some(l))) => l,
        };
        let reply: Reply = match serde_json::from_str(&reply_line) {
            Ok(r) => r,
            Err(e) => {
                self.stop(&mut guard).await;
                return Err(WorkerFailure::Protocol(format!(
                    "{e}: {}",
                    truncate(&reply_line, 200)
                )));
            }
        };
        if reply.id.as_deref() != Some(id.as_str()) {
            self.stop(&mut guard).await;
            return Err(WorkerFailure::Protocol(format!(
                "reply id {:?} does not match request id {id}",
                reply.id
            )));
        }
        if reply.ok {
            reply
                .result
                .ok_or_else(|| WorkerFailure::Protocol("reply has no result".into()))
        } else {
            Err(WorkerFailure::Model(reply.error.unwrap_or(WorkerError {
                class: vec![],
                message: "the worker reported an error without a message".into(),
            })))
        }
    }

    async fn stop(&self, guard: &mut Option<Proc>) {
        if let Some(mut p) = guard.take() {
            let _ = p.child.start_kill();
            let _ = p.child.wait().await;
        }
    }

    /// Start the worker if needed and return its `hello` answer, cached.
    pub async fn hello(&self) -> Result<Hello, WorkerFailure> {
        if let Some(h) = self.hello.lock().await.clone() {
            return Ok(h);
        }
        let started = Instant::now();
        let v = self.call("hello", serde_json::Value::Null).await?;
        let h: Hello =
            serde_json::from_value(v).map_err(|e| WorkerFailure::Protocol(e.to_string()))?;
        if h.protocol != PROTOCOL_VERSION {
            return Err(WorkerFailure::Protocol(format!(
                "worker speaks protocol {}, the server speaks {PROTOCOL_VERSION}",
                h.protocol
            )));
        }
        tracing::info!(
            elapsed_ms = started.elapsed().as_millis() as u64,
            "worker started"
        );
        *self.hello.lock().await = Some(h.clone());
        Ok(h)
    }

    /// Typed call: serialise `params`, deserialise the result.
    pub async fn call_typed<P: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: &P,
    ) -> Result<R, WorkerFailure> {
        let p = serde_json::to_value(params).map_err(|e| WorkerFailure::Protocol(e.to_string()))?;
        let v = self.call(method, p).await?;
        serde_json::from_value(v).map_err(|e| WorkerFailure::Protocol(e.to_string()))
    }
}

fn truncate(s: &str, n: usize) -> &str {
    match s.char_indices().nth(n) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}
