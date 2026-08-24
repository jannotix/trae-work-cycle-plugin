use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use tokio::time::{sleep, timeout};
use workflow_core::PROTOCOL_VERSION;
use workflow_ipc::{
    ClientMessage, HealthReport, ServerMessage,
    auth::Authenticator,
    channel::JsonChannel,
    secret::{IpcSecret, load_or_create},
    transport::LocalStream,
};

const HEALTH_WAIT: Duration = Duration::from_secs(15);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Debug)]
pub enum DaemonError {
    Channel(String),
    ProtocolMismatch { daemon: u16, expected: u16 },
    Rejected { code: String, message: String },
    Spawn(String),
}

impl std::fmt::Display for DaemonError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Channel(message) => write!(formatter, "local control plane error: {message}"),
            Self::ProtocolMismatch { daemon, expected } => write!(
                formatter,
                "control plane protocol {daemon} is incompatible with {expected}"
            ),
            Self::Rejected { code, message } => {
                write!(
                    formatter,
                    "control plane rejected the request ({code}): {message}"
                )
            }
            Self::Spawn(message) => {
                write!(formatter, "control plane did not become healthy: {message}")
            }
        }
    }
}

impl std::error::Error for DaemonError {}

impl From<DaemonError> for String {
    fn from(value: DaemonError) -> Self {
        value.to_string()
    }
}

#[derive(Clone)]
pub struct Daemon {
    data_dir: PathBuf,
    request_id: Arc<AtomicU64>,
}

impl Daemon {
    #[must_use]
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            request_id: Arc::new(AtomicU64::new(100)),
        }
    }

    #[must_use]
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub async fn ensure(&self) -> Result<HealthReport, DaemonError> {
        let secret_exists = self.secret_path().is_file();
        if secret_exists && let Ok(report) = self.health().await {
            return self.check_protocol(report);
        }
        if !secret_exists {
            std::fs::create_dir_all(self.runtime_dir())
                .map_err(|error| DaemonError::Spawn(error.to_string()))?;
        }
        self.spawn_serve()?;
        let deadline = std::time::Instant::now() + HEALTH_WAIT;
        let mut last = String::new();
        while std::time::Instant::now() < deadline {
            match self.health().await {
                Ok(report) => return self.check_protocol(report),
                Err(error) => last = error.to_string(),
            }
            sleep(POLL_INTERVAL).await;
        }
        Err(DaemonError::Spawn(last))
    }

    fn check_protocol(&self, report: HealthReport) -> Result<HealthReport, DaemonError> {
        if report.protocol_version == PROTOCOL_VERSION {
            Ok(report)
        } else {
            Err(DaemonError::ProtocolMismatch {
                daemon: report.protocol_version,
                expected: PROTOCOL_VERSION,
            })
        }
    }

    pub async fn health(&self) -> Result<HealthReport, DaemonError> {
        let id = self.next_request_id();
        let message = self
            .exchange(ClientMessage::Health { request_id: id })
            .await?;
        match message {
            ServerMessage::Health { request_id, report } if request_id == id => Ok(report),
            ServerMessage::Error { message, .. } => Err(DaemonError::Rejected {
                code: "health".to_owned(),
                message,
            }),
            _ => Err(DaemonError::Channel(
                "unexpected health response from the control plane".to_owned(),
            )),
        }
    }

    pub async fn exchange(&self, message: ClientMessage) -> Result<ServerMessage, DaemonError> {
        timeout(REQUEST_TIMEOUT, self.exchange_unbounded(message))
            .await
            .map_err(|_| DaemonError::Channel("control plane response timed out".to_owned()))?
    }

    pub async fn exchange_long(
        &self,
        message: ClientMessage,
        limit: Duration,
    ) -> Result<ServerMessage, DaemonError> {
        timeout(limit, self.exchange_unbounded(message))
            .await
            .map_err(|_| DaemonError::Channel("control plane response timed out".to_owned()))?
    }

    async fn exchange_unbounded(
        &self,
        message: ClientMessage,
    ) -> Result<ServerMessage, DaemonError> {
        let mut channel = self.connect().await?;
        channel
            .send(&message)
            .await
            .map_err(|error| DaemonError::Channel(error.to_string()))?;
        let response = channel
            .receive::<ServerMessage>()
            .await
            .map_err(|error| DaemonError::Channel(error.to_string()))?;
        match response {
            ServerMessage::Error {
                request_id: _,
                code,
                message,
            } => Err(DaemonError::Rejected { code, message }),
            other => Ok(other),
        }
    }

    async fn connect(&self) -> Result<JsonChannel<LocalStream>, DaemonError> {
        let secret = load_or_create(self.secret_path())
            .map_err(|error| DaemonError::Channel(error.to_string()))?;
        let stream = self
            .open_stream(&secret)
            .await
            .map_err(|error| DaemonError::Channel(error.to_string()))?;
        let mut channel = JsonChannel::new(stream);
        let challenge = match channel
            .receive::<ServerMessage>()
            .await
            .map_err(|error| DaemonError::Channel(error.to_string()))?
        {
            ServerMessage::Challenge(challenge) => challenge,
            _ => {
                return Err(DaemonError::Channel(
                    "control plane did not send an authentication challenge".to_owned(),
                ));
            }
        };
        channel
            .send(&ClientMessage::Authenticate(Authenticator::respond(
                secret.as_bytes(),
                &challenge,
            )))
            .await
            .map_err(|error| DaemonError::Channel(error.to_string()))?;
        Ok(channel)
    }

    #[cfg(windows)]
    async fn open_stream(&self, secret: &IpcSecret) -> std::io::Result<LocalStream> {
        workflow_ipc::transport::connect(&secret.endpoint_id()).await
    }

    #[cfg(unix)]
    async fn open_stream(&self, _secret: &IpcSecret) -> std::io::Result<LocalStream> {
        workflow_ipc::transport::connect(self.runtime_dir().join("workflow.sock")).await
    }

    fn spawn_serve(&self) -> Result<(), DaemonError> {
        let executable =
            std::env::current_exe().map_err(|error| DaemonError::Spawn(error.to_string()))?;
        let mut command = std::process::Command::new(executable);
        command
            .arg("serve")
            .arg("--data-dir")
            .arg(&self.data_dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        command
            .spawn()
            .map(|_| ())
            .map_err(|error| DaemonError::Spawn(error.to_string()))
    }

    pub fn next_request_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::Relaxed)
    }

    fn runtime_dir(&self) -> PathBuf {
        self.data_dir.join("runtime")
    }

    fn secret_path(&self) -> PathBuf {
        self.runtime_dir().join("ipc.secret")
    }
}
