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

#[derive(Clone)]
struct AppState {
    retry_timeout: Duration,
    device_path: String,
}

pub async fn run(port: u16, retry_timeout: Duration, device_path: String) -> Result<(), Box<dyn std::error::Error>> {
    let state = AppState {
        retry_timeout,
        device_path,
    };
    
    let app = Router::new()
        .route("/play", put(play_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;

    println!("Server listening on {}", addr);
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
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("Failed to read request body".to_string())
                .unwrap();
        }
    };

    let melody = match String::from_utf8(body_bytes.to_vec()) {
        Ok(s) => s,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("Invalid UTF-8 in melody data".to_string())
                .unwrap();
        }
    };

    match speaker::play_melody(&melody, client_addr, state.retry_timeout, &state.device_path).await {
        Ok(()) => Response::builder()
            .status(StatusCode::OK)
            .body("".to_string())
            .unwrap(),
        Err(SpeakerError::InvalidMelody(msg)) => Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(msg)
            .unwrap(),
        Err(SpeakerError::Timeout) => Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .body("Device busy - request timed out".to_string())
            .unwrap(),
        Err(SpeakerError::DeviceError(e)) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(format!("Device error: {}", e))
            .unwrap(),
        Err(SpeakerError::DeviceBusy) => Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .body("Device busy".to_string())
            .unwrap(),
    }
}