//! What a provider can actually do, declared once at connect time.

use bitflags::bitflags;

bitflags! {
    /// Data classes and write abilities a provider exposes.
    ///
    /// The sync scheduler iterates the flags a connected account actually has
    /// instead of trying every fetch and handling the failure, and the UI
    /// renders "push to watch" only where [`Self::PLANS_WRITE`] is set.
    ///
    /// Deliberately not `Serialize`: a bitflags value serialising as a bare
    /// integer is a contract the frontend can never pin. The app layer
    /// serialises [`Self::data_classes`] instead.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Capability: u32 {
        /// Historical and recent training activities with laps.
        const ACTIVITIES = 1 << 0;
        /// Daily health: sleep, resting HR, HRV, stress, steps.
        const HEALTH = 1 << 1;
        /// Provider fitness estimates: VO2max, running level, threshold pace.
        const FITNESS = 1 << 2;
        /// Runalytics can push planned sessions into the provider.
        const PLANS_WRITE = 1 << 3;
        /// The provider can be queried for the athlete's identity, which is
        /// what dedups accounts across reconnects. A provider without this
        /// cannot support more than one connected account.
        const IDENTITY = 1 << 4;
    }
}

impl Capability {
    /// Everything a full read-only consumer platform provides.
    #[must_use]
    pub const fn read_only() -> Self {
        Self::ACTIVITIES
            .union(Self::HEALTH)
            .union(Self::FITNESS)
            .union(Self::IDENTITY)
    }

    /// Whether `self` can serve the given data class, by its store name
    /// (`"activities"`, `"health"`, `"fitness"`). Unknown classes are false,
    /// which makes a typo'd class a no-op rather than a silent full fetch.
    #[must_use]
    pub fn serves(self, data_class: &str) -> bool {
        match data_class {
            "activities" => self.contains(Self::ACTIVITIES),
            "health" => self.contains(Self::HEALTH),
            "fitness" => self.contains(Self::FITNESS),
            _ => false,
        }
    }

    /// The data-class names this capability set can serve, in sync order.
    ///
    /// Health before activities because the dashboard's readiness number is
    /// useless without sleep, and a slow activity history must not delay it.
    #[must_use]
    pub fn data_classes(self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.contains(Self::HEALTH) {
            out.push("health");
        }
        if self.contains(Self::ACTIVITIES) {
            out.push("activities");
        }
        if self.contains(Self::FITNESS) {
            out.push("fitness");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_class_is_not_served() {
        assert!(!Capability::read_only().serves("nutrition"));
        assert!(!Capability::empty().serves("activities"));
    }

    #[test]
    fn data_classes_are_ordered_health_first() {
        let classes = Capability::read_only().data_classes();
        assert_eq!(classes, vec!["health", "activities", "fitness"]);
        assert_eq!(Capability::ACTIVITIES.data_classes(), vec!["activities"]);
    }

    #[test]
    fn write_capability_is_additive() {
        let coros = Capability::read_only().union(Capability::PLANS_WRITE);
        assert!(coros.contains(Capability::PLANS_WRITE));
        assert!(!Capability::read_only().contains(Capability::PLANS_WRITE));
    }
}
