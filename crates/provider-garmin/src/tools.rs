//! Tool negotiation for the Garmin MCP sidecar.
//!
//! Community Garmin Connect MCP servers do not agree on tool names, and
//! versions rename them. Like the COROS adapter, we resolve *operations*
//! against whatever the sidecar actually offers instead of hard-coding one
//! server's spelling. Candidates are ordered most-likely-first; hints are
//! substring fallbacks for servers we have not seen.

/// A capability we need the sidecar to serve, by our name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operation {
    /// Activity list for a date range.
    ListActivities,
    /// Full summary (and ideally laps) for one activity id.
    ActivityDetail,
    /// Sleep for a date.
    DailySleep,
    /// Heart-rate data (resting HR, avg) for a date.
    DailyHeart,
    /// Stress for a date.
    DailyStress,
    /// Step count for a date.
    Steps,
    /// VO2max / fitness estimates for a date.
    Fitness,
    /// Identity, when the sidecar exposes one.
    Whoami,
}

impl Operation {
    /// Exact tool names to try, in order.
    #[must_use]
    pub const fn candidates(self) -> &'static [&'static str] {
        match self {
            Self::ListActivities => &["get_activities", "activities", "list_activities"],
            Self::ActivityDetail => &[
                "get_activity_details",
                "activity_details",
                "get_activity_detail",
            ],
            Self::DailySleep => &["get_sleep_data", "sleep_data", "get_sleep"],
            Self::DailyHeart => &["get_rhr_day", "get_heart_rate", "daily_heart"],
            Self::DailyStress => &["get_daily_stress", "daily_stress", "get_stress"],
            Self::Steps => &["get_steps", "steps", "get_daily_steps"],
            Self::Fitness => &["get_vo2_max", "vo2max", "get_fitness_endurance_age"],
            Self::Whoami => &["get_user_settings", "whoami", "get_profile"],
        }
    }

    /// Substring hints for servers whose names match none of the candidates.
    #[must_use]
    pub const fn hints(self) -> &'static [&'static str] {
        match self {
            Self::ListActivities => &["activit"],
            Self::ActivityDetail => &["detail"],
            Self::DailySleep => &["sleep"],
            Self::DailyHeart => &["rhr", "heart"],
            Self::DailyStress => &["stress"],
            Self::Steps => &["step"],
            Self::Fitness => &["vo2", "fitness"],
            Self::Whoami => &["user", "profile", "whoami"],
        }
    }
}

/// The resolved operation -> tool-name table for one sidecar session.
#[derive(Debug, Clone, Default)]
pub struct ToolMap {
    resolved: Vec<(Operation, String)>,
}

impl ToolMap {
    /// Match offered tool names against every operation.
    ///
    /// An exact candidate always wins; otherwise the shortest offered name
    /// containing a hint wins (shortest = most specific: `get_sleep_data`
    /// over `get_sleep_segments_and_stages`).
    #[must_use]
    pub fn resolve(offered: &[String]) -> Self {
        let mut resolved = Vec::new();
        let operations = [
            Operation::ListActivities,
            Operation::ActivityDetail,
            Operation::DailySleep,
            Operation::DailyHeart,
            Operation::DailyStress,
            Operation::Steps,
            Operation::Fitness,
            Operation::Whoami,
        ];
        for op in operations {
            let exact = op
                .candidates()
                .iter()
                .find_map(|candidate| offered.iter().find(|name| name == candidate).cloned());
            if let Some(name) = exact {
                resolved.push((op, name));
                continue;
            }
            let mut best: Option<&String> = None;
            for name in offered {
                let lower = name.to_lowercase();
                if !op.hints().iter().any(|h| lower.contains(h)) {
                    continue;
                }
                if best.is_none_or(|b| name.len() < b.len()) {
                    best = Some(name);
                }
            }
            if let Some(name) = best {
                resolved.push((op, name.clone()));
            }
        }
        Self { resolved }
    }

    /// The sidecar tool name serving an operation, if negotiated.
    #[must_use]
    pub fn tool(&self, op: Operation) -> Option<&str> {
        self.resolved
            .iter()
            .find(|(o, _)| *o == op)
            .map(|(_, name)| name.as_str())
    }

    /// How many operations found a tool — logged at connect.
    #[must_use]
    pub fn resolved_count(&self) -> usize {
        self.resolved.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn exact_names_win_over_hints() {
        let map = ToolMap::resolve(&names(&[
            "get_activities",
            "get_activity_details",
            "get_sleep_data",
            "get_rhr_day",
            "get_daily_stress",
            "get_steps",
            "get_vo2_max",
        ]));
        assert_eq!(map.tool(Operation::ListActivities), Some("get_activities"));
        assert_eq!(map.tool(Operation::DailySleep), Some("get_sleep_data"));
        assert_eq!(map.tool(Operation::Fitness), Some("get_vo2_max"));
        assert_eq!(map.resolved_count(), 7);
    }

    #[test]
    fn hints_rescue_unfamiliar_spellings() {
        let map = ToolMap::resolve(&names(&["fetch_activity_log", "sleep_summary"]));
        assert_eq!(
            map.tool(Operation::ListActivities),
            Some("fetch_activity_log")
        );
        assert_eq!(map.tool(Operation::DailySleep), Some("sleep_summary"));
        assert_eq!(map.tool(Operation::Steps), None);
    }

    #[test]
    fn shortest_hint_match_is_most_specific() {
        let map = ToolMap::resolve(&names(&[
            "get_sleep_data",
            "get_sleep_segments_and_stages_v2_extended",
        ]));
        assert_eq!(map.tool(Operation::DailySleep), Some("get_sleep_data"));
    }
}
