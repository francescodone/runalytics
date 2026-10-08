//! Tool-name resolution against the live COROS server.
//!
//! COROS has renamed MCP tools between releases, and the names differ
//! slightly across regional deployments. Rather than hard-coding one name per
//! operation — which turns every COROS release into a Runalytics outage — the
//! adapter resolves each operation against the server's actual tool list at
//! connect time, from an ordered candidate list plus a substring fallback.
//! The resolution is logged once per connect so a rename is visible in the
//! logs before it is visible in user reports.

/// The operations the adapter needs from the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operation {
    /// List activities in a date range.
    ListActivities,
    /// Full detail (laps) for one activity.
    ActivityDetail,
    /// Daily health records in a date range.
    DailyHealth,
    /// Fitness estimates (VO2max, running level).
    Fitness,
    /// Create or update a workout on the account.
    UpsertWorkout,
    /// Delete a workout.
    DeleteWorkout,
    /// Who am I — identity for account dedup.
    Whoami,
}

impl Operation {
    /// Candidate tool names, best guess first. Matched case-insensitively.
    #[must_use]
    pub const fn candidates(self) -> &'static [&'static str] {
        match self {
            Self::ListActivities => &["query_activity_list", "get_activities", "activities"],
            Self::ActivityDetail => &["query_activity_detail", "get_activity", "activity_detail"],
            Self::DailyHealth => &["query_daily_health", "get_health_data", "daily_health"],
            Self::Fitness => &["query_fitness_data", "get_fitness", "fitness_data"],
            Self::UpsertWorkout => &["create_training_workout", "upsert_workout", "add_workout"],
            Self::DeleteWorkout => &[
                "delete_training_workout",
                "delete_workout",
                "remove_workout",
            ],
            Self::Whoami => &["get_user_info", "whoami", "user_info"],
        }
    }

    /// Substrings an otherwise-unmatched tool name may still qualify through.
    #[must_use]
    pub const fn hints(self) -> &'static [&'static str] {
        match self {
            Self::ListActivities => &["activit", "run"],
            Self::ActivityDetail => &["detail"],
            Self::DailyHealth => &["health", "sleep"],
            Self::Fitness => &["fit", "vo2", "level"],
            Self::UpsertWorkout => &["workout", "training"],
            Self::DeleteWorkout => &["delete", "remove"],
            Self::Whoami => &["user", "profile"],
        }
    }
}

/// A resolved operation -> tool-name map for one connected server.
#[derive(Debug, Clone, Default)]
pub struct ToolMap {
    resolved: Vec<(Operation, String)>,
}

impl ToolMap {
    /// Resolve every operation against `tools` (the server's tool names).
    ///
    /// Exact candidate matches win over hint matches; among hint matches the
    /// shortest name wins, because `query_activity_list` beats
    /// `query_activity_detail_of_lap_v2` for the list operation on brevity
    /// alone often enough to be worth the rule, and the log line lets a human
    /// check it.
    #[must_use]
    pub fn resolve(tools: &[String]) -> Self {
        let operations = [
            Operation::ListActivities,
            Operation::ActivityDetail,
            Operation::DailyHealth,
            Operation::Fitness,
            Operation::UpsertWorkout,
            Operation::DeleteWorkout,
            Operation::Whoami,
        ];
        let lower: Vec<(String, &String)> = tools.iter().map(|t| (t.to_lowercase(), t)).collect();
        let mut resolved = Vec::new();
        for op in operations {
            let exact = op.candidates().iter().find_map(|c| {
                lower
                    .iter()
                    .find(|(l, _)| l == c)
                    .map(|(_, original)| (*original).clone())
            });
            let chosen = exact.or_else(|| {
                let mut best: Option<(usize, String)> = None;
                for (l, original) in &lower {
                    if op.hints().iter().any(|h| l.contains(h)) {
                        let len = l.len();
                        if best.as_ref().is_none_or(|(bl, _)| len < *bl) {
                            best = Some((len, (*original).clone()));
                        }
                    }
                }
                best.map(|(_, name)| name)
            });
            if let Some(name) = chosen {
                resolved.push((op, name));
            }
        }
        Self { resolved }
    }

    #[must_use]
    pub fn tool(&self, op: Operation) -> Option<&str> {
        self.resolved
            .iter()
            .find(|(o, _)| *o == op)
            .map(|(_, n)| n.as_str())
    }

    /// Operations the server could serve.
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
    fn exact_candidates_win() {
        let map = ToolMap::resolve(&names(&[
            "query_activity_list",
            "query_activity_detail",
            "query_daily_health",
            "query_fitness_data",
            "create_training_workout",
            "delete_training_workout",
            "get_user_info",
        ]));
        assert_eq!(map.resolved_count(), 7);
        assert_eq!(
            map.tool(Operation::ListActivities),
            Some("query_activity_list")
        );
    }

    #[test]
    fn renamed_tools_fall_back_to_hints() {
        let map = ToolMap::resolve(&names(&[
            "coros_getRuns",
            "coros_runDetail",
            "coros_healthDaily",
            "coros_vo2max",
            "coros_userProfile",
        ]));
        assert_eq!(map.tool(Operation::ListActivities), Some("coros_getRuns"));
        assert_eq!(map.tool(Operation::DailyHealth), Some("coros_healthDaily"));
        assert_eq!(map.tool(Operation::Fitness), Some("coros_vo2max"));
        assert_eq!(map.tool(Operation::Whoami), Some("coros_userProfile"));
        // A server genuinely without write tools resolves to None, which the
        // adapter turns into a capability downgrade, not an error.
        assert_eq!(map.tool(Operation::UpsertWorkout), None);
    }

    #[test]
    fn detail_beats_list_for_the_detail_operation() {
        // Both contain "activit"; only the detail one passes its own hint.
        let map = ToolMap::resolve(&names(&["query_activity_list", "activity_detail_v2"]));
        assert_eq!(
            map.tool(Operation::ListActivities),
            Some("query_activity_list")
        );
        assert_eq!(
            map.tool(Operation::ActivityDetail),
            Some("activity_detail_v2")
        );
    }
}
