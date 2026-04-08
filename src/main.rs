use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use toko_rs::{app_router, build_app_state, config, seed};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::AppConfig::load().unwrap_or_else(|e| {
        eprintln!("Failed to load config: {}", e);
        std::process::exit(1);
    });

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}={}", env!("CARGO_PKG_NAME"), config.rust_log).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (state, app_db) =
        build_app_state(&config.database_url, &config.default_currency_code).await?;
    tracing::info!("Connected to database and migrations executed");

    if std::env::args().any(|arg| arg == "--seed") {
        seed::run_seed(&app_db).await?;
        tracing::info!("Seeding complete. Exiting.");
        return Ok(());
    }

    let app = app_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { tracing::info!("Received Ctrl+C, shutting down gracefully..."); },
        _ = terminate => { tracing::info!("Received SIGTERM, shutting down gracefully..."); },
    }
}
