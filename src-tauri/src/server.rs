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

#[derive(Deserialize)]
struct PrintQuery {
    printer: Option<String>,
    encoding: Option<String>,
}

#[derive(Serialize)]
struct PrintResponse {
    job_id: String,
}

#[derive(Serialize)]
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

    let printer = query
        .printer
        .or_else(|| {
            state
                .config
                .read()
                .ok()
                .and_then(|c| c.selected_printer.clone())
        })
        .ok_or((StatusCode::BAD_REQUEST, "No printer selected".to_string()))?;

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

async fn status_handler() -> Json<StatusResponse> {
    Json(StatusResponse {
        status: "running".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

pub struct ServerHandle {
    task: tokio::task::JoinHandle<()>,
    cancel: CancellationToken,
}

impl ServerHandle {
    pub fn is_finished(&self) -> bool {
        self.task.is_finished()
    }

    /// Gracefully shut down the server and wait for it to stop.
    pub async fn shutdown(self) {
        self.cancel.cancel();
        // Wait up to 2 seconds for graceful shutdown
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), self.task).await;
    }
}

/// Bind the port and start the Axum server. Returns a handle that supports
/// graceful shutdown. Fails immediately if the port can't be bound.
pub async fn start(state: Arc<crate::AppState>) -> Result<ServerHandle, String> {
    let port = state.config.read().map_err(|e| e.to_string())?.port;

    let router = Router::new()
        .route("/print", post(print_handler))
        .route("/printers", get(printers_handler))
        .route("/status", get(status_handler))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10 MB
        .layer(CorsLayer::permissive())
        .with_state(state.clone());

    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .map_err(|e| format!("Failed to bind port {port}: {e}"))?;

    log::info!("HTTP server listening on 127.0.0.1:{port}");
    state.app_handle.emit("server-status", true).ok();

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    let task = tokio::spawn(async move {
        let shutdown = async move { cancel_clone.cancelled().await };
        if let Err(e) = axum::serve(listener, router)
            .with_graceful_shutdown(shutdown)
            .await
        {
            log::error!("Server error: {e}");
        }
        state.app_handle.emit("server-status", false).ok();
    });

    Ok(ServerHandle { task, cancel })
}
