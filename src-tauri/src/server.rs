use crate::printing;
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Emitter;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintJob {
    pub id: String,
    pub printer: String,
    pub timestamp: u64,
    pub status: JobStatus,
    pub zpl_preview: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Printing,
    Completed,
    Failed,
}

/// Who answers print jobs on this PC's port. Loopback ports are shared by every
/// signed-in user, so only one Dazzle on the PC can serve.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum ServerStatus {
    Starting,
    /// This Dazzle owns the port.
    Serving,
    /// Another signed-in user's Dazzle owns the port. This one keeps trying and takes over when that one exits.
    Standby,
    /// No Dazzle serves print jobs on this port.
    Failed {
        error: String,
    },
}

#[derive(Deserialize)]
struct PrintQuery {
    printer: Option<String>,
    encoding: Option<String>,
}

#[derive(Serialize)]
struct PrintResponse {
    job_id: String,
}

#[derive(Serialize, Deserialize)]
struct StatusResponse {
    status: String,
    version: String,
}

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{nanos:x}")
}

fn now_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

async fn print_handler(
    State(state): State<Arc<crate::AppState>>,
    Query(query): Query<PrintQuery>,
    body: Bytes,
) -> Result<Json<PrintResponse>, (StatusCode, String)> {
    if body.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Empty ZPL body".to_string()));
    }

    let data = if query.encoding.as_deref() == Some("base64") {
        base64::engine::general_purpose::STANDARD
            .decode(&body)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid base64: {e}")))?
    } else {
        body.to_vec()
    };

    let printer = match query.printer {
        Some(name) => name,
        None => {
            let selected = state
                .config
                .read()
                .ok()
                .and_then(|config| config.selected_printer.clone());
            tokio::task::spawn_blocking(move || printing::choice(selected.as_deref()))
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Task panicked: {e}"),
                    )
                })?
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
                .ok_or((StatusCode::BAD_REQUEST, "No printer found".to_string()))?
                .name
        }
    };

    let job_id = generate_id();
    let zpl_preview = String::from_utf8_lossy(&data[..data.len().min(200)]).to_string();

    let mut job = PrintJob {
        id: job_id.clone(),
        printer: printer.clone(),
        timestamp: now_secs(),
        status: JobStatus::Printing,
        zpl_preview: Some(zpl_preview),
        error: None,
    };

    state.app_handle.emit("print-job", &job).ok();

    // Run blocking print operation off the async runtime
    let print_result = tokio::task::spawn_blocking(move || printing::send_raw(&printer, &data))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Print task panicked: {e}"),
            )
        })?;

    match print_result {
        Ok(()) => {
            job.status = JobStatus::Completed;
            log::info!("Printed to {}", job.printer);
        }
        Err(e) => {
            job.status = JobStatus::Failed;
            job.error = Some(e.clone());
            log::error!("Print failed for {}: {e}", job.printer);
        }
    }

    // Store job
    if let Ok(mut jobs) = state.print_jobs.write() {
        jobs.insert(0, job.clone());
        jobs.truncate(100);
    }

    state.app_handle.emit("print-job", &job).ok();

    if matches!(job.status, JobStatus::Failed) {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            job.error.unwrap_or_default(),
        ))
    } else {
        Ok(Json(PrintResponse { job_id }))
    }
}

async fn printers_handler() -> Result<Json<Vec<printing::Printer>>, (StatusCode, String)> {
    tokio::task::spawn_blocking(printing::discover)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Task panicked: {e}"),
            )
        })?
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))
}

async fn status_handler(State(state): State<Arc<crate::AppState>>) -> Json<StatusResponse> {
    Json(StatusResponse {
        status: "running".to_string(),
        // The release version lives in tauri.conf.json; Cargo.toml's is never bumped.
        version: state.app_handle.package_info().version.to_string(),
    })
}

pub struct ServerHandle {
    task: tokio::task::JoinHandle<()>,
    cancel: CancellationToken,
}

impl ServerHandle {
    /// Gracefully shut down the server and wait for it to stop.
    pub async fn shutdown(self) {
        self.cancel.cancel();
        // Wait up to 2 seconds for graceful shutdown
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), self.task).await;
    }
}

const RETRY_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

/// Start the print server. It serves when it can bind the port, and otherwise
/// retries every RETRY_INTERVAL, so a standby Dazzle takes the port over when
/// the serving one exits.
pub fn start(state: Arc<crate::AppState>) -> ServerHandle {
    crate::set_server_status(&state, ServerStatus::Starting);

    let router = Router::new()
        .route("/print", post(print_handler))
        .route("/printers", get(printers_handler))
        .route("/status", get(status_handler))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10 MB
        .layer(CorsLayer::permissive())
        .with_state(state.clone());

    let cancel = CancellationToken::new();
    let cancel_task = cancel.clone();

    let task = tokio::spawn(async move {
        loop {
            let port = state
                .config
                .read()
                .unwrap_or_else(|err| err.into_inner())
                .port;

            match TcpListener::bind(("127.0.0.1", port)).await {
                Ok(listener) => {
                    log::info!("HTTP server listening on 127.0.0.1:{port}");
                    crate::set_server_status(&state, ServerStatus::Serving);
                    let shutdown = cancel_task.clone();
                    if let Err(e) = axum::serve(listener, router.clone())
                        .with_graceful_shutdown(async move { shutdown.cancelled().await })
                        .await
                    {
                        log::error!("Server error: {e}");
                    }
                }
                Err(e) => {
                    tokio::select! {
                        _ = cancel_task.cancelled() => return,
                        status = bind_failure_status(port, &e) => {
                            crate::set_server_status(&state, status);
                        }
                    }
                }
            }

            tokio::select! {
                _ = cancel_task.cancelled() => return,
                _ = tokio::time::sleep(RETRY_INTERVAL) => {}
            }
        }
    });

    ServerHandle { task, cancel }
}

/// What a failed bind means: another Dazzle on this PC is serving, or nothing is.
async fn bind_failure_status(port: u16, error: &std::io::Error) -> ServerStatus {
    if another_dazzle_answers(port).await {
        ServerStatus::Standby
    } else if error.kind() == std::io::ErrorKind::AddrInUse {
        ServerStatus::Failed {
            error: format!("Port {port} is in use by another program"),
        }
    } else {
        ServerStatus::Failed {
            error: format!("Failed to bind port {port}: {error}"),
        }
    }
}

/// Whether the program on this port is a Dazzle (it answers GET /status the way status_handler does).
async fn another_dazzle_answers(port: u16) -> bool {
    let exchange = async {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .ok()?;
        stream
            .write_all(b"GET /status HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
            .await
            .ok()?;
        let mut response = String::new();
        stream.read_to_string(&mut response).await.ok()?;

        let (_, body) = response.split_once("\r\n\r\n")?;
        let status: StatusResponse = serde_json::from_str(body).ok()?;
        Some(status.status == "running")
    };

    matches!(
        tokio::time::timeout(std::time::Duration::from_secs(2), exchange).await,
        Ok(Some(true))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn dazzle_status() -> Json<StatusResponse> {
        Json(StatusResponse {
            status: "running".to_string(),
            version: "0.0.0".to_string(),
        })
    }

    async fn free_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        port
    }

    #[tokio::test]
    async fn another_dazzle_answers_when_status_is_served() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let app = Router::new().route("/status", get(dazzle_status));
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        assert!(another_dazzle_answers(port).await);

        server.abort();
    }

    #[tokio::test]
    async fn another_dazzle_answers_false_when_listener_never_replies() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        assert!(!another_dazzle_answers(port).await);

        drop(listener);
    }

    #[tokio::test]
    async fn another_dazzle_answers_false_when_nothing_listens() {
        let port = free_port().await;
        assert!(!another_dazzle_answers(port).await);
    }

    #[tokio::test]
    async fn bind_failure_is_standby_when_a_dazzle_answers() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let app = Router::new().route("/status", get(dazzle_status));
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let error = std::io::Error::new(std::io::ErrorKind::AddrInUse, "in use");
        assert_eq!(
            bind_failure_status(port, &error).await,
            ServerStatus::Standby
        );

        server.abort();
    }

    #[tokio::test]
    async fn bind_failure_is_in_use_when_a_non_dazzle_holds_the_port() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let error = std::io::Error::new(std::io::ErrorKind::AddrInUse, "in use");

        assert_eq!(
            bind_failure_status(port, &error).await,
            ServerStatus::Failed {
                error: format!("Port {port} is in use by another program"),
            }
        );

        drop(listener);
    }

    #[tokio::test]
    async fn bind_failure_reports_other_errors() {
        let port = free_port().await;
        let error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");

        assert_eq!(
            bind_failure_status(port, &error).await,
            ServerStatus::Failed {
                error: format!("Failed to bind port {port}: denied"),
            }
        );
    }

    #[tokio::test]
    async fn server_status_serializes_to_the_four_shapes() {
        assert_eq!(
            serde_json::to_value(ServerStatus::Starting).unwrap(),
            serde_json::json!({"state": "starting"})
        );
        assert_eq!(
            serde_json::to_value(ServerStatus::Serving).unwrap(),
            serde_json::json!({"state": "serving"})
        );
        assert_eq!(
            serde_json::to_value(ServerStatus::Standby).unwrap(),
            serde_json::json!({"state": "standby"})
        );
        assert_eq!(
            serde_json::to_value(ServerStatus::Failed {
                error: "Port 29100 is in use by another program".to_string(),
            })
            .unwrap(),
            serde_json::json!({"state": "failed", "error": "Port 29100 is in use by another program"})
        );
    }
}
