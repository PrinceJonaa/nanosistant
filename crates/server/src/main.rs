//! Nanosistant server binary.
//!
//! Starts the HTTP/SSE server.

use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Nanosistant server starting on {addr}");

    // Build the router.
    let app = nstn_server::build_router();

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");

    tracing::info!("Nanosistant server listening on {addr}");
    axum::serve(listener, app)
        .await
        .expect("server error");
}
