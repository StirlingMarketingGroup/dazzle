use crate::printing;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Emitter;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintJob {
    pub id: String,
    pub printer: String,
    pub timestamp: u64,
    pub status: String,
    pub zpl_preview: Option<String>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
struct PrintQuery {
    printer: Option<String>,
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
    body: String,
) -> Result<Json<PrintResponse>, (StatusCode, String)> {
    if body.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Empty ZPL body".to_string()));
    }

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
    let zpl_preview = if body.len() > 200 {
        Some(format!("{}…", &body[..200]))
    } else {
        Some(body.clone())
    };

    let mut job = PrintJob {
        id: job_id.clone(),
        printer: printer.clone(),
        timestamp: now_secs(),
        status: "printing".to_string(),
        zpl_preview,
        error: None,
    };

    state.app_handle.emit("print-job", &job).ok();

    match printing::send_raw(&printer, body.as_bytes()) {
        Ok(()) => {
            job.status = "completed".to_string();
            log::info!("Printed {} bytes to {printer}", body.len());
        }
        Err(e) => {
            job.status = "failed".to_string();
            job.error = Some(e.clone());
            log::error!("Print failed for {printer}: {e}");
        }
    }

    // Store job
    if let Ok(mut jobs) = state.print_jobs.write() {
        jobs.insert(0, job.clone());
        jobs.truncate(100);
    }

    state.app_handle.emit("print-job", &job).ok();

    if job.status == "failed" {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            job.error.unwrap_or_default(),
        ))
    } else {
        Ok(Json(PrintResponse { job_id }))
    }
}

async fn printers_handler() -> Result<Json<Vec<printing::Printer>>, (StatusCode, String)> {
    printing::discover()
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))
}

async fn status_handler() -> Json<StatusResponse> {
    Json(StatusResponse {
        status: "running".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Bind the port and start the Axum server. Returns a JoinHandle for the
/// background task. Fails immediately if the port can't be bound.
pub async fn start(
    state: Arc<crate::AppState>,
) -> Result<tokio::task::JoinHandle<()>, String> {
    let port = state.config.read().map_err(|e| e.to_string())?.port;

    let router = Router::new()
        .route("/print", post(print_handler))
        .route("/printers", get(printers_handler))
        .route("/status", get(status_handler))
        .layer(CorsLayer::permissive())
        .with_state(state.clone());

    let listener = TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .map_err(|e| format!("Failed to bind port {port}: {e}"))?;

    log::info!("HTTP server listening on port {port}");
    state.app_handle.emit("server-status", true).ok();

    let handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            log::error!("Server error: {e}");
            state.app_handle.emit("server-status", false).ok();
        }
    });

    Ok(handle)
}
