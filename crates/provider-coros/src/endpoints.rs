//! COROS MCP endpoints and regional discovery.
//!
//! COROS hosts its MCP server per data-residency region. An account's region
//! is decided by where it was registered, not by where the athlete currently
//! is, and the token is only valid against its own region's server. The
//! desktop app cannot ask COROS "which region is this email" before consent,
//! so the flow is: try the last-known region, then the default, and record
//! whichever answered into `provider_account.region`.

use std::fmt;

use serde::{Deserialize, Serialize};

/// COROS data-residency regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Region {
    /// The global endpoint, tried first: it fronts the majority of accounts.
    Global,
    /// Mainland China.
    Cn,
    /// Europe.
    Eu,
    /// United States.
    Us,
}

impl Region {
    /// The MCP endpoint for this region.
    #[must_use]
    pub const fn mcp_url(self) -> &'static str {
        match self {
            Self::Global => "https://mcp.coros.com/mcp",
            Self::Cn => "https://mcpcn.coros.com/mcp",
            Self::Eu => "https://mcpeu.coros.com/mcp",
            Self::Us => "https://mcpus.coros.com/mcp",
        }
    }

    /// Parse a stored region label; unknown labels are `None` so the caller
    /// falls back to discovery rather than trusting a corrupt row.
    #[must_use]
    pub fn parse(label: &str) -> Option<Self> {
        match label.trim().to_ascii_lowercase().as_str() {
            "global" => Some(Self::Global),
            "cn" => Some(Self::Cn),
            "eu" => Some(Self::Eu),
            "us" => Some(Self::Us),
            _ => None,
        }
    }

    /// All regions in probe order: the global endpoint first so a fresh
    /// connect lands somewhere sensible.
    pub const ALL: [Self; 4] = [Self::Global, Self::Eu, Self::Us, Self::Cn];
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Global => "global",
            Self::Cn => "cn",
            Self::Eu => "eu",
            Self::Us => "us",
        })
    }
}

/// The endpoint sequence to try for an account whose region may be known.
///
/// Known region first (a reconnect should not re-probe), then the rest.
#[must_use]
pub fn endpoint_order(known: Option<Region>) -> Vec<Region> {
    let mut order = Vec::with_capacity(Region::ALL.len());
    if let Some(region) = known {
        order.push(region);
    }
    order.extend(Region::ALL.into_iter().filter(|r| Some(*r) != known));
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_https_and_regional() {
        for region in Region::ALL {
            let url = region.mcp_url();
            assert!(url.starts_with("https://"), "{url} must be TLS");
            assert!(url.ends_with("/mcp"), "{url} must target the MCP path");
        }
    }

    #[test]
    fn known_region_is_probed_first_without_duplicates() {
        let order = endpoint_order(Some(Region::Cn));
        assert_eq!(order[0], Region::Cn);
        assert_eq!(order.len(), Region::ALL.len());
        let unique: std::collections::HashSet<_> = order.iter().collect();
        assert_eq!(unique.len(), Region::ALL.len());
    }

    #[test]
    fn unknown_region_falls_back_to_default_order() {
        assert_eq!(endpoint_order(None), Region::ALL.to_vec());
        assert_eq!(Region::parse("EU"), Some(Region::Eu));
        assert_eq!(Region::parse("mars"), None);
    }
}
