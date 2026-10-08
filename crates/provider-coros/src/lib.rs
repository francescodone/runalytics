//! COROS adapter for Runalytics.
//!
//! COROS publishes its data through a hosted MCP server rather than a REST
//! API, so this crate is an MCP client with an OAuth 2.1 + PKCE front door.
//! The pieces, smallest first:
//!
//! * [`endpoints`] — the regional MCP hosts and the probe order.
//! * [`oauth`] — PKCE, the consent URL, code exchange, and refresh. The
//!   browser and the localhost redirect listener belong to the desktop shell;
//!   everything here is pure or a plain form POST.
//! * [`tools`] — operation-to-tool-name resolution against the live server,
//!   so a COROS rename degrades gracefully instead of breaking the app.
//! * [`mapping`] — tolerant COROS-JSON to provider-core draft translation.
//! * [`provider`] — [`CorosProvider`], the [`WearableProvider`] implementation
//!   that ties the above together over a self-healing MCP session.
//!
//! Reading data requires only a stored token; writing workouts additionally
//! requires the server to offer both write tools, which [`capabilities_for`]
//! negotiates at connect time rather than assuming.

#![forbid(unsafe_code)]

pub mod endpoints;
pub mod mapping;
pub mod oauth;
pub mod provider;
pub mod tools;

pub use endpoints::Region;
pub use provider::{CorosConfig, CorosProvider, OAuthClient, capabilities_for, date_args};
pub use tools::{Operation, ToolMap};
