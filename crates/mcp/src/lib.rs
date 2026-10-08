//! The Runalytics MCP server: plan, scoring and calendar tools for external
//! agents (the coach agent in particular).
//!
//! The server is deliberately *thin*. Every tool is a wrapper around a pure
//! operation in [`ops`] that takes a [`Db`](runalytics_store::Db) and returns
//! JSON — the rmcp layer adds nothing but schema and transport. That split is
//! what lets the whole tool surface be tested without an MCP client, and it is
//! why the desktop app and this server can share one SQLite file: neither owns
//! the data, the store does.
//!
//! Two transports, one service:
//!
//! - **stdio** (default) — the desktop shell spawns this binary as a child
//!   process and registers it with the user's agent.
//! - **streamable HTTP** (`--http PORT`) — loopback only by default, for
//!   agents that speak HTTP.
//!
//! See `SKILL.md` at the repo root for the agent-facing contract.

#![forbid(unsafe_code)]

pub mod ops;
pub mod server;

pub use ops::ServerConfig;
pub use server::RunalyticsServer;
