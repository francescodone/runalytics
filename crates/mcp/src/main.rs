//! `runalytics-mcp` — the MCP server binary.
//!
//! Default transport is stdio (the desktop shell spawns it as a child).
//! `--http PORT` serves the same tools over streamable HTTP on loopback.

use std::path::PathBuf;

use anyhow::{Context as _, Result};
use clap::Parser;
use rmcp::ServiceExt;
use runalytics_mcp::{RunalyticsServer, ServerConfig, ops::Context};

#[derive(Parser, Debug)]
#[command(
    name = "runalytics-mcp",
    about = "Runalytics MCP server: plan, scoring and calendar tools for agents."
)]
struct Args {
    /// SQLite database file. Created on first run.
    #[arg(long, env = "RUNALYTICS_DB")]
    db: Option<PathBuf>,

    /// IANA time zone for the athlete (dates, calendar times, "today").
    #[arg(long, default_value = "Europe/Berlin")]
    tz: String,

    /// Calendar.app calendar to publish into.
    #[arg(long, default_value = "Runalytics")]
    calendar_name: String,

    /// Also write the plan ICS feed to this path (for webcal clients).
    #[arg(long)]
    ics_path: Option<PathBuf>,

    /// Serve streamable HTTP on this loopback port instead of stdio.
    #[arg(long)]
    http: Option<u16>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "runalytics_mcp=info".into()),
        )
        // stdio mode owns stdout — logs must go to stderr there.
        .with_writer(std::io::stderr)
        .init();

    let timezone = args
        .tz
        .parse()
        .with_context(|| format!("unknown time zone '{}'", args.tz))?;
    let config = ServerConfig {
        db_path: args.db.clone(),
        timezone,
        calendar_name: args.calendar_name.clone(),
        ics_path: args.ics_path.clone(),
    };
    let ctx = Context::open(config).context("opening the runalytics database")?;

    let runtime = tokio::runtime::Runtime::new()?;
    match args.http {
        Some(port) => runtime.block_on(serve_http(ctx, port)),
        None => runtime.block_on(serve_stdio(ctx)),
    }
}

async fn serve_stdio(ctx: Context) -> Result<()> {
    let server = RunalyticsServer::new(ctx);
    tracing::info!("runalytics-mcp listening on stdio");
    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}

async fn serve_http(ctx: Context, port: u16) -> Result<()> {
    use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
    use rmcp::transport::{StreamableHttpServerConfig, StreamableHttpService};
    use std::sync::Arc;

    let service = StreamableHttpService::new(
        move || Ok(RunalyticsServer::new(ctx.clone())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );
    let app = axum::Router::new().nest_service("/mcp", service);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .with_context(|| format!("binding 127.0.0.1:{port}"))?;
    tracing::info!(
        port,
        "runalytics-mcp serving streamable HTTP on loopback /mcp"
    );
    axum::serve(listener, app)
        .await
        .context("http server failed")
}
