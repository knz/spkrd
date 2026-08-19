// HTTP server setup and routing. Holds the chosen output backend (either the
// FreeBSD /dev/speaker writer or, when compiled with the `cpal` feature, the
// CPAL audio renderer) and dispatches /play requests accordingly. The melody
// length limit is configured at startup and threaded through to whichever
// backend validates the incoming body. Error mapping to HTTP status codes is
// shared between the available backends. run() binds one listener per
// address in the caller-supplied list (see the bind module for how that
// list is parsed from --bind) and serves the same app on all of them
// concurrently.
//
// IPv6 listeners are bound v6-only (bind_listener sets IPV6_V6ONLY). The
// default --bind spec is "0.0.0.0,[::]", which only works if the two
// wildcard sockets are independent. That is the native behaviour on
// FreeBSD, where net.inet6.ip6.v6only defaults to 1, but not on Linux,
// where net.ipv6.bindv6only defaults to 0 and makes a [::] socket
// dual-stack: it accepts IPv4 too, so it overlaps an already-bound
// 0.0.0.0 socket on the same port and the second bind fails with
// EADDRINUSE. Setting the option explicitly makes the documented default
// mean the same thing everywhere — two separate listeners, one per
// family — instead of depending on a host sysctl. The consequence is
// that binding only "[::]" is genuinely IPv6-only and will not serve
// IPv4 clients; list "0.0.0.0" as well to serve both.

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
use socket2::{Domain, Protocol, Socket, Type};
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
    addrs: Vec<SocketAddr>,
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

    let mut listeners = Vec::with_capacity(addrs.len());
    for addr in &addrs {
        let listener = bind_listener(*addr).map_err(|e| format!("failed to bind {}: {}", addr, e))?;
        info!("Server listening on {}", addr);
        listeners.push(listener);
    }

    let mut tasks = tokio::task::JoinSet::new();
    for listener in listeners {
        let make_service = app.clone().into_make_service_with_connect_info::<SocketAddr>();
        tasks.spawn(async move { axum::serve(listener, make_service).await });
    }

    while let Some(result) = tasks.join_next().await {
        result??;
    }

    Ok(())
}

// Bind a single listener. IPv6 addresses get IPV6_V6ONLY so that the
// wildcard IPv4 and IPv6 entries of the default --bind spec are separate
// sockets rather than overlapping ones; see the module comment.
//
// Built through socket2 because tokio's TcpListener::bind offers no way to
// set socket options before the bind() call, and IPV6_V6ONLY must be set
// while the socket is still unbound.
fn bind_listener(addr: SocketAddr) -> std::io::Result<TcpListener> {
    let domain = match addr {
        SocketAddr::V4(_) => Domain::IPV4,
        SocketAddr::V6(_) => Domain::IPV6,
    };
    let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;

    // std::net::TcpListener::bind sets SO_REUSEADDR on Unix and socket2
    // does not; without it a restart fails for as long as the previous
    // listener lingers in TIME_WAIT.
    #[cfg(unix)]
    socket.set_reuse_address(true)?;

    if matches!(addr, SocketAddr::V6(_)) {
        socket.set_only_v6(true)?;
    }

    socket.bind(&addr.into())?;
    // Same backlog std::net::TcpListener::bind uses.
    socket.listen(128)?;
    socket.set_nonblocking(true)?;

    TcpListener::from_std(std::net::TcpListener::from(socket))
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
        // Reaching this case means acquire_and_play exhausted
        // --retry-timeout while trying to rebuild the device. Surface as
        // a 500 — the host/device is genuinely unreachable for now.
        #[cfg(feature = "cpal")]
        Err(SpeakerError::CpalDisconnect(msg)) => {
            error!(
                "CPAL disconnect for request from {} after retries: {}",
                client_addr.ip(),
                msg
            );
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("CPAL disconnect: {}", msg))
                .unwrap()
        }
    }
}
