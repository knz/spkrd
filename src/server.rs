// HTTP server setup and routing for speaker device

use crate::error::SpeakerError;
use crate::speaker;
use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
    response::Response,
    routing::put,
    Router,
};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use log::{info, error, debug};

#[derive(Clone)]
struct AppState {
    retry_timeout: Duration,
    device_path: String,
    debug: bool,
}

pub async fn run(port: u16, retry_timeout: Duration, device_path: String, debug: bool) -> Result<(), Box<dyn std::error::Error>> {
    let state = AppState {
        retry_timeout,
        device_path,
        debug,
    };
    
    let app = Router::new()
        .route("/play", put(play_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;

    info!("Server listening on {}", addr);
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;

    Ok(())
}

async fn play_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    axum::extract::State(state): axum::extract::State<AppState>,
    request: Request<Body>,
) -> Response<String> {
    let body_bytes = match axum::body::to_bytes(request.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to read request body from {}: {}", client_addr.ip(), e);
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("Failed to read request body".to_string())
                .unwrap();
        }
    };

    let melody = match String::from_utf8(body_bytes.to_vec()) {
        Ok(s) => s,
        Err(e) => {
            error!("Invalid UTF-8 in melody data from {}: {}", client_addr.ip(), e);
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("Invalid UTF-8 in melody data".to_string())
                .unwrap();
        }
    };

    match speaker::play_melody(&melody, client_addr, state.retry_timeout, &state.device_path, state.debug).await {
        Ok(retries) => {
            if state.debug {
                debug!("Request from {} completed successfully after {} retries", client_addr.ip(), retries);
            }
            Response::builder()
                .status(StatusCode::OK)
                .body("".to_string())
                .unwrap()
        },
        Err(SpeakerError::InvalidMelody(msg)) => {
            error!("Invalid melody from {}: {}", client_addr.ip(), msg);
            Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(msg)
                .unwrap()
        },
        Err(SpeakerError::Timeout) => {
            error!("Request from {} timed out (device busy)", client_addr.ip());
            Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body("Device busy - request timed out".to_string())
                .unwrap()
        },
        Err(SpeakerError::DeviceError(e)) => {
            error!("Device error for request from {}: {}", client_addr.ip(), e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("Device error: {}", e))
                .unwrap()
        },
        Err(SpeakerError::DeviceBusy) => {
            error!("Device busy for request from {}", client_addr.ip());
            Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body("Device busy".to_string())
                .unwrap()
        },
    }
}