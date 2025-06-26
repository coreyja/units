use axum::response::IntoResponse;
use cja::{
    color_eyre::{
        self,
        eyre::{Context as _, eyre},
    },
    server::{cookies::CookieKey, run_server},
    setup::{setup_sentry, setup_tracing},
};
use maud::html;
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::net::SocketAddr;
use tokio_util::sync::CancellationToken;
use tracing::info;

#[derive(Clone)]
struct AppState {
    db: sqlx::PgPool,
    cookie_key: CookieKey,
    cancellation_token: CancellationToken,
}

impl AppState {
    async fn from_env() -> color_eyre::Result<Self> {
        let db = setup_db_pool().await?;
        let cookie_key = CookieKey::from_env_or_generate()?;
        let cancellation_token = CancellationToken::new();

        Ok(Self {
            db,
            cookie_key,
            cancellation_token,
        })
    }
}

impl cja::app_state::AppState for AppState {
    fn version(&self) -> &'static str {
        concat!(env!("CARGO_PKG_VERSION"), " (", env!("VERGEN_GIT_SHA"), ")")
    }

    fn db(&self) -> &sqlx::PgPool {
        &self.db
    }

    fn cookie_key(&self) -> &CookieKey {
        &self.cookie_key
    }
}

fn main() -> color_eyre::Result<()> {
    // Initialize Sentry for error tracking
    let _sentry_guard = setup_sentry();

    // Create and run the tokio runtime
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()?
        .block_on(async { run_application().await })
}

#[tracing::instrument(err)]
pub async fn setup_db_pool() -> cja::Result<PgPool> {
    const MIGRATION_LOCK_ID: i64 = 0xDB_DB_DB_DB_DB_DB_DB;

    let database_url = std::env::var("DATABASE_URL").wrap_err("DATABASE_URL must be set")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    sqlx::query!("SELECT pg_advisory_lock($1)", MIGRATION_LOCK_ID)
        .execute(&pool)
        .await?;

    sqlx::migrate!().run(&pool).await?;

    let unlock_result = sqlx::query!("SELECT pg_advisory_unlock($1)", MIGRATION_LOCK_ID)
        .fetch_one(&pool)
        .await?
        .pg_advisory_unlock;

    match unlock_result {
        Some(b) => {
            if b {
                tracing::info!("Migration lock unlocked");
            } else {
                tracing::info!("Failed to unlock migration lock");
            }
        }
        None => return Err(eyre!("Failed to unlock migration lock")),
    }

    Ok(pool)
}

async fn run_application() -> cja::Result<()> {
    // Initialize tracing
    setup_tracing("cja-site")?;

    let app_state = AppState::from_env().await?;

    // Spawn application tasks
    info!("Spawning application tasks");
    let futures = spawn_application_tasks(&app_state);

    // Set up Ctrl+C handler
    let cancellation_token = app_state.cancellation_token.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        info!("Received Ctrl+C, initiating shutdown...");
        cancellation_token.cancel();
    });

    // Wait for all tasks to complete
    futures::future::try_join_all(futures).await?;

    Ok(())
}

fn routes(app_state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/", axum::routing::get(root))
        .with_state(app_state)
}

async fn spawn_mcp_server(cancellation_token: CancellationToken) -> color_eyre::Result<()> {
    let bind_addr: SocketAddr = "0.0.0.0:3001".parse()?;
    let server_config = SseServerConfig {
        bind: bind_addr,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: cancellation_token.clone(),
        sse_keep_alive: None,
    };

    let (sse_server, router) = SseServer::new(server_config);

    // Create the UnitConversion service
    let unit_conversion = units::UnitConversion::new();

    info!("MCP SSE server listening on http://0.0.0.0:3001");
    info!("SSE endpoint: http://0.0.0.0:3001/sse");

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;

    // Spawn the SSE server with the service
    let sse_handle = sse_server.with_service(move || unit_conversion.clone());

    // Run the HTTP server directly
    axum::serve(listener, router.into_make_service())
        .with_graceful_shutdown(async move {
            cancellation_token.cancelled().await;
        })
        .await
        .map_err(|e| eyre!("HTTP server error: {}", e))?;

    sse_handle.cancel();

    Ok(())
}

async fn root() -> impl IntoResponse {
    html! {
        html {
            head {
                title { "CJA Site" }
            }
            body {
                h1 { "Hello, World!" }
            }
        }
    }
}

/// Spawn all application background tasks
fn spawn_application_tasks(
    app_state: &AppState,
) -> std::vec::Vec<tokio::task::JoinHandle<std::result::Result<(), cja::color_eyre::Report>>> {
    let mut futures = vec![];

    futures.push(tokio::spawn(run_server(routes(app_state.clone()))));

    let cancellation_token = app_state.cancellation_token.clone();
    futures.push(tokio::spawn(spawn_mcp_server(cancellation_token)));

    info!("All application tasks spawned successfully");
    futures
}
