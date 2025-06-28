use axum::response::IntoResponse;
use cja::{
    color_eyre::{self, eyre::Context as _},
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
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { "Units MCP Server - Example Implementation" }
                script src="https://cdn.tailwindcss.com" {}
                style { "
                    @keyframes float {
                        0%, 100% { transform: translateY(0px); }
                        50% { transform: translateY(-10px); }
                    }
                    .float-animation {
                        animation: float 3s ease-in-out infinite;
                    }
                " }
            }
            body class="bg-gradient-to-br from-purple-50 to-pink-50 text-gray-900 min-h-screen" {
                div class="container mx-auto px-4 py-12 max-w-4xl" {
                    header class="text-center mb-12" {
                        h1 class="text-5xl font-bold bg-gradient-to-r from-purple-600 to-pink-600 bg-clip-text text-transparent mb-4" { 
                            "Units MCP Server" 
                        }
                        p class="text-xl text-gray-700 font-medium" { 
                            "A simple example MCP server that works with Claude and other MCP-compatible clients" 
                        }
                    }

                    section class="bg-white/80 backdrop-blur rounded-2xl shadow-xl p-8 mb-8 border border-purple-100" {
                        h2 class="text-3xl font-bold mb-4 text-purple-700" { 
                            "Getting Started" 
                        }
                        p class="text-gray-700 mb-6 text-lg leading-relaxed" {
                            "This example MCP server demonstrates how to build a simple tool that works with Claude and any other MCP-supporting client. "
                            "It handles basic unit conversions between common measurement units."
                        }

                        div class="bg-gradient-to-r from-purple-100 to-pink-100 rounded-xl p-6 mb-6 border-2 border-purple-200" {
                            h3 class="font-bold mb-2 text-purple-800 text-lg" { 
                                "MCP Server URL" 
                            }
                            code class="bg-white px-4 py-2 rounded-lg text-sm font-mono text-purple-600 inline-block shadow-sm" { 
                                "units.coreyja.com/mcp/sse" 
                            }
                        }

                        h3 class="font-bold mb-4 text-purple-700 text-lg" { 
                            "Quick Setup" 
                        }
                        ol class="list-decimal list-inside space-y-3 text-gray-700 ml-8" {
                            li class="leading-relaxed" { 
                                "Add the URL to your MCP client configuration (Claude, etc.)" 
                            }
                            li class="leading-relaxed" { 
                                "Connect via SSE (Server-Sent Events) transport" 
                            }
                            li class="leading-relaxed" { 
                                "Use the convert_units tool in your conversations" 
                            }
                        }
                    }

                    section class="bg-white/80 backdrop-blur rounded-2xl shadow-xl p-8 mb-8 border border-purple-100" {
                        h2 class="text-3xl font-bold mb-6 text-purple-700" { 
                            "Available Tool" 
                        }

                        div class="bg-gradient-to-r from-blue-500 to-purple-600 p-1 rounded-xl" {
                            div class="bg-white rounded-lg p-6" {
                                h3 class="font-bold text-2xl mb-3 text-purple-800" { 
                                    "convert_units" 
                                }
                                p class="text-gray-700 mb-4 text-lg" { 
                                    "A simple tool that converts between units. Provide an input value and desired output unit." 
                                }

                                div class="space-y-4 bg-purple-50 rounded-lg p-4" {
                                    div {
                                        span class="font-bold text-purple-700" { "Parameters:" }
                                    }
                                    ul class="space-y-3 ml-4" {
                                        li class="flex items-start gap-2" {
                                            span class="text-purple-500 text-xl" { "•" }
                                            div {
                                                code class="bg-purple-200 px-2 py-1 rounded text-sm font-bold text-purple-800" { "input_value" }
                                                span class="text-gray-700" { " → Your starting point (like \"10 meters\" or \"32 fahrenheit\")" }
                                            }
                                        }
                                        li class="flex items-start gap-2" {
                                            span class="text-purple-500 text-xl" { "•" }
                                            div {
                                                code class="bg-purple-200 px-2 py-1 rounded text-sm font-bold text-purple-800" { "output_unit" }
                                                span class="text-gray-700" { " → Your destination (like \"feet\" or \"celsius\")" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    section class="bg-white/80 backdrop-blur rounded-2xl shadow-xl p-8 border border-purple-100" {
                        h2 class="text-3xl font-bold mb-6 text-purple-700" { 
                            "Supported Unit Types" 
                        }
                        p class="text-gray-600 mb-6" {
                            "This example server supports a handful of common unit conversions. "
                            "It's not comprehensive, but it demonstrates the MCP pattern nicely."
                        }

                        div class="grid grid-cols-1 md:grid-cols-2 gap-6" {
                            div class="space-y-4" {
                                div class="bg-purple-50 rounded-lg p-4 hover:shadow-md transition-shadow" {
                                    h4 class="font-bold text-purple-700" { "Length" }
                                    p class="text-sm text-gray-600" { "meters, feet, kilometers, miles" }
                                }
                                div class="bg-pink-50 rounded-lg p-4 hover:shadow-md transition-shadow" {
                                    h4 class="font-bold text-pink-700" { "Mass" }
                                    p class="text-sm text-gray-600" { "kilograms, pounds, grams" }
                                }
                                div class="bg-purple-50 rounded-lg p-4 hover:shadow-md transition-shadow" {
                                    h4 class="font-bold text-purple-700" { "Temperature" }
                                    p class="text-sm text-gray-600" { "celsius, fahrenheit" }
                                }
                                div class="bg-pink-50 rounded-lg p-4 hover:shadow-md transition-shadow" {
                                    h4 class="font-bold text-pink-700" { "Volume" }
                                    p class="text-sm text-gray-600" { "liters, gallons, milliliters, cubic meters/feet/inches" }
                                }
                                div class="bg-purple-50 rounded-lg p-4 hover:shadow-md transition-shadow" {
                                    h4 class="font-bold text-purple-700" { "Velocity" }
                                    p class="text-sm text-gray-600" { "mph, km/h, m/s, ft/s" }
                                }
                                div class="bg-pink-50 rounded-lg p-4 hover:shadow-md transition-shadow" {
                                    h4 class="font-bold text-pink-700" { "Area" }
                                    p class="text-sm text-gray-600" { "square meters/feet/kilometers/miles, acres" }
                                }
                            }
                            div class="space-y-4" {
                                div class="bg-pink-50 rounded-lg p-4 hover:shadow-md transition-shadow" {
                                    h4 class="font-bold text-pink-700" { "Density" }
                                    p class="text-sm text-gray-600" { "kg/m³, lb/ft³, g/cm³, g/mL" }
                                }
                                div class="bg-purple-50 rounded-lg p-4 hover:shadow-md transition-shadow" {
                                    h4 class="font-bold text-purple-700" { "Acceleration" }
                                    p class="text-sm text-gray-600" { "m/s², ft/s²" }
                                }
                                div class="bg-pink-50 rounded-lg p-4 hover:shadow-md transition-shadow" {
                                    h4 class="font-bold text-pink-700" { "Force" }
                                    p class="text-sm text-gray-600" { "newtons, pounds force" }
                                }
                                div class="bg-purple-50 rounded-lg p-4 hover:shadow-md transition-shadow" {
                                    h4 class="font-bold text-purple-700" { "Energy" }
                                    p class="text-sm text-gray-600" { "joules, foot pounds" }
                                }
                                div class="bg-pink-50 rounded-lg p-4 hover:shadow-md transition-shadow" {
                                    h4 class="font-bold text-pink-700" { "Power" }
                                    p class="text-sm text-gray-600" { "watts, horsepower" }
                                }
                                div class="bg-purple-50 rounded-lg p-4 hover:shadow-md transition-shadow" {
                                    h4 class="font-bold text-purple-700" { "Fuel Economy" }
                                    p class="text-sm text-gray-600" { "miles/gallon, km/L, L/100km" }
                                }
                            }
                        }
                    }

                    footer class="mt-12 py-8 border-t border-purple-200" {
                        div class="flex flex-col sm:flex-row justify-center items-center gap-4 text-gray-700" {
                            a href="https://coreyja.com" target="_blank" rel="noopener noreferrer" class="hover:text-purple-600 transition-colors font-medium" {
                                "Made with 💜 by coreyja"
                            }
                            span class="hidden sm:inline text-purple-300" { "•" }
                            a href="https://github.com/coreyja/units" target="_blank" rel="noopener noreferrer" class="flex items-center gap-2 hover:text-purple-600 transition-colors font-medium" {
                                svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24" {
                                    path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" {}
                                }
                                "View on GitHub"
                            }
                            span class="hidden sm:inline text-purple-300" { "•" }
                            a href="https://github.com/sponsors/coreyja" target="_blank" rel="noopener noreferrer" class="flex items-center gap-2 hover:text-pink-500 transition-colors font-medium" {
                                svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24" {
                                    path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z" {}
                                }
                                "Sponsor on GitHub"
                            }
                        }
                    }
                }
            }
        }
    }
}
