//! Garmin Connect adapter: a [`WearableProvider`] over a local MCP sidecar.
//!
//! Garmin has no official MCP server, so this adapter spawns a community
//! Python server as a child process and speaks MCP to it over stdio
//! ([`provider`]). Tool names vary between servers and versions, so
//! operations are negotiated against whatever the sidecar offers
//! ([`tools`]), and payloads are mapped alias-tolerantly into
//! provider-core drafts ([`mapping`]).
//!
//! Read-only by contract: the community servers expose no workout writes,
//! so [`runalytics_provider_core::Capability::PLANS_WRITE`] is never
//! advertised and pushes fail with `Unsupported` — COROS remains the
//! write-back path.

#![forbid(unsafe_code)]

pub mod mapping;
pub mod provider;
pub mod tools;

pub use provider::{GarminConfig, GarminProvider, SidecarConfig, default_sidecar_json};
pub use tools::{Operation, ToolMap};
