// HTTP server setup and routing. Holds the chosen output backend (either the
// FreeBSD /dev/speaker writer or, when compiled with the `cpal` feature, the
// CPAL audio renderer) and dispatches /play requests accordingly. The melody
// length limit is configured at startup and threaded through to whichever
// backend validates the incoming body. Error mapping to HTTP status codes is
// shared between the available backends.

#[cfg(feature = "cpal")]
use crate::cpal_backend::CpalBackend;
use crate::error::SpeakerError;
use crate::freebsd_speaker;
use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
    response::Response,
    routing::put,
    Router,
};
use log::{debug, error, info};
use std::net::SocketAddr;
#[cfg(feature = "cpal")]
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;

#[derive(Clone)]
pub enum Backend {
    FreebsdSpeaker { device_path: String },
    #[cfg(feature = "cpal")]
    Cpal(Arc<CpalBackend>),
}

#[derive(Clone)]
struct AppState {
    retry_timeout: Duration,
    backend: Backend,
    max_melody_length: usize,
    debug: bool,
}

pub async fn run(
    port: u16,
    retry_timeout: Duration,
    backend: Backend,
    max_melody_length: usize,
    debug: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = AppState {
        retry_timeout,
        backend,
        max_melody_length,
        debug,
    };

    let app = Router::new()
        .route("/play", put(play_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;

    info!("Server listening on {}", addr);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
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

    let result = match &state.backend {
        Backend::FreebsdSpeaker { device_path } => {
            freebsd_speaker::play_melody(
                &melody,
                client_addr,
                state.retry_timeout,
                device_path,
                state.max_melody_length,
                state.debug,
            )
            .await
        }
        #[cfg(feature = "cpal")]
        Backend::Cpal(b) => {
            b.play_melody(
                &melody,
                client_addr,
                state.retry_timeout,
                state.max_melody_length,
                state.debug,
            )
            .await
        }
    };

    match result {
        Ok(retries) => {
            if state.debug {
                debug!(
                    "Request from {} completed successfully after {} retries",
                    client_addr.ip(),
                    retries
                );
            }
            Response::builder()
                .status(StatusCode::OK)
                .body("".to_string())
                .unwrap()
        }
        Err(SpeakerError::InvalidMelody(msg)) => {
            error!("Invalid melody from {}: {}", client_addr.ip(), msg);
            Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(msg)
                .unwrap()
        }
        Err(SpeakerError::Timeout) => {
            error!("Request from {} timed out (device busy)", client_addr.ip());
            Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body("Device busy - request timed out".to_string())
                .unwrap()
        }
        Err(SpeakerError::DeviceError(e)) => {
            error!("Device error for request from {}: {}", client_addr.ip(), e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("Device error: {}", e))
                .unwrap()
        }
        Err(SpeakerError::DeviceBusy) => {
            error!("Device busy for request from {}", client_addr.ip());
            Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body("Device busy".to_string())
                .unwrap()
        }
        #[cfg(feature = "cpal")]
        Err(SpeakerError::CpalError(msg)) => {
            error!("CPAL error for request from {}: {}", client_addr.ip(), msg);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("CPAL error: {}", msg))
                .unwrap()
        }
    }
}
