//! Typed identifiers.
//!
//! Every entity carries a UUIDv7 so identifiers sort by creation time, which
//! keeps SQLite index pages dense. Where a UUID is unavailable (rows imported
//! from a provider) the id is derived deterministically from the provider key.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Generate a fresh identifier.
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            /// Deterministic identifier derived from a stable external key.
            ///
            /// Uses a name-based UUID so re-importing the same provider record
            /// always lands on the same primary key, which is what makes
            /// ingestion idempotent.
            #[must_use]
            pub fn from_external(namespace: &Uuid, key: &str) -> Self {
                Self(Uuid::new_v5(namespace, key.as_bytes()))
            }

            #[must_use]
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(Uuid::parse_str(s)?))
            }
        }
    };
}

id_newtype! {
    /// Identifies a training plan.
    PlanId
}

id_newtype! {
    /// Identifies one planned training session inside a plan.
    PlannedSessionId
}

id_newtype! {
    /// Identifies an imported activity from a wearable provider.
    ActivityId
}

id_newtype! {
    /// Identifies a completed session's outcome record.
    SessionResultId
}

id_newtype! {
    /// Identifies a day of aggregated health data.
    HealthDayId
}

id_newtype! {
    /// Identifies the local athlete profile.
    UserId
}

id_newtype! {
    /// Identifies one connected provider account. A distinct id per
    /// `(athlete, provider)` so credentials and sync cursors never leak across
    /// providers.
    ProviderAccountId
}

/// Namespace for deriving [`ActivityId`] from `(provider, provider_activity_id)`.
///
/// The leading bytes spell `Runalytics`; the tail distinguishes the two
/// derivation contexts.
pub const ACTIVITY_NAMESPACE: Uuid = Uuid::from_u128(0x5275_6e61_6c79_7469_6373_0000_0000_0001);

/// Namespace for deriving [`PlannedSessionId`] from `(plan, date, slot)`.
pub const SESSION_NAMESPACE: Uuid = Uuid::from_u128(0x5275_6e61_6c79_7469_6373_0000_0000_0002);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_ids_are_stable() {
        let a = ActivityId::from_external(&ACTIVITY_NAMESPACE, "coros:12345");
        let b = ActivityId::from_external(&ACTIVITY_NAMESPACE, "coros:12345");
        let c = ActivityId::from_external(&ACTIVITY_NAMESPACE, "garmin:12345");
        assert_eq!(a, b, "same external key must produce the same id");
        assert_ne!(a, c, "provider must participate in the key");
    }

    #[test]
    fn round_trips_through_string() {
        let id = PlanId::new();
        let parsed: PlanId = id.to_string().parse().expect("round trip");
        assert_eq!(id, parsed);
    }
}
