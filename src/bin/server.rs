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
use rmcp::{
    service::RequestContext,
    transport::sse_server::{SseServer, SseServerConfig},
};
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

    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(MIGRATION_LOCK_ID)
        .execute(&pool)
        .await?;

    sqlx::migrate!().run(&pool).await?;

    use sqlx::Row;
    let unlock_result = sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(MIGRATION_LOCK_ID)
        .fetch_one(&pool)
        .await?
        .get::<bool, _>(0);

    if unlock_result {
        tracing::info!("Migration lock unlocked");
    } else {
        tracing::info!("Failed to unlock migration lock");
    }

    Ok(pool)
}

async fn run_application() -> cja::Result<()> {
    // Initialize tracing
    setup_tracing("cja-site")?;

    let app_state = AppState::from_env().await?;

    // Spawn application tasks
    info!("Spawning application tasks");
    let cancellation_token = app_state.cancellation_token.clone();

    let port = std::env::var("PORT").unwrap_or_else(|_| "3001".to_string());
    let bind_addr: SocketAddr = format!("0.0.0.0:{port}").parse()?;
    let server_config = SseServerConfig {
        bind: bind_addr,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: cancellation_token.clone(),
        sse_keep_alive: None,
    };

    let (sse_server, mcp_router) = SseServer::new(server_config);

    let routes = routes(app_state.clone());

    let routes = routes.nest("/mcp", mcp_router);

    let app_state: &AppState = &app_state;

    let server_handle = tokio::spawn(run_server(routes));

    info!("All application tasks spawned successfully");

    // Set up Ctrl+C handler
    let cancellation_token = app_state.cancellation_token.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        info!("Received Ctrl+C, initiating shutdown...");
        cancellation_token.cancel();
    });

    tokio::spawn(async move {
        if let Err(e) = server_handle.await {
            tracing::error!(error = %e, "sse server shutdown with error");
        }
    });

    let units = units::UnitConversion::new();

    println!(
        "units: {:?}",
        units::UnitConversion::tool_router().list_all()
    );

    let ct = sse_server.with_service(move || units.clone());

    tokio::signal::ctrl_c().await?;
    ct.cancel();
    Ok(())
}

fn routes(app_state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/", axum::routing::get(root))
        .with_state(app_state)
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
