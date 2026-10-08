//! Provider identity.
//!
//! Kept in `core` (rather than `provider-core`) because the provider appears in
//! the primary key of every imported row and in the derived [`ActivityId`], so
//! the store needs it without depending on the transport layer.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A wearable or training platform Runalytics can sync with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// COROS, via the official hosted MCP server.
    Coros,
    /// Garmin Connect, via a local community MCP sidecar.
    Garmin,
}

impl Provider {
    /// Stable lowercase key used in database columns and MCP tool arguments.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Coros => "coros",
            Self::Garmin => "garmin",
        }
    }

    /// Display name for the UI.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Coros => "COROS",
            Self::Garmin => "Garmin Connect",
        }
    }

    /// Whether Runalytics can push training *into* this provider.
    ///
    /// COROS exposes write tools on its MCP server; the Garmin path is
    /// read-only, so plans stay local and reach the watch through the calendar.
    #[must_use]
    pub const fn supports_write(self) -> bool {
        matches!(self, Self::Coros)
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Provider {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "coros" => Ok(Self::Coros),
            "garmin" => Ok(Self::Garmin),
            other => Err(format!(
                "unknown provider '{other}', expected 'coros' or 'garmin'"
            )),
        }
    }
}

pub use crate::ids::ProviderAccountId;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_case_insensitively() {
        assert_eq!("COROS".parse::<Provider>().unwrap(), Provider::Coros);
        assert_eq!(" garmin ".parse::<Provider>().unwrap(), Provider::Garmin);
        assert!("whoop".parse::<Provider>().is_err());
    }

    #[test]
    fn only_coros_is_writable() {
        assert!(Provider::Coros.supports_write());
        assert!(!Provider::Garmin.supports_write());
    }

    #[test]
    fn serialises_as_lowercase_for_the_frontend() {
        assert_eq!(serde_json_provider(Provider::Coros), "\"coros\"");
    }

    fn serde_json_provider(p: Provider) -> String {
        serde_json::to_string(&p).expect("serialisable")
    }
}
