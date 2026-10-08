//! Paces prescribed by generated sessions.
//!
//! Everything derives from a single threshold pace. When the provider has
//! reported a VO2max we take the standard physiological route (vVO2max via the
//! ~0.2 ml/kg/m running cost of running, threshold at ~90 % of it); otherwise
//! experience level supplies a conservative estimate. Deriving one table once
//! per plan keeps every session's paces mutually consistent — a 5K-pace
//! interval and a marathon-pace long run segment cannot disagree.

use runalytics_core::{AthleteSnapshot, ExperienceLevel, GoalKind, Pace, SessionKind};

/// The pace table a generated plan prescribes from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaceModel {
    /// Threshold (roughly 1-hour effort) pace — the anchor of the table.
    pub threshold: Pace,
    /// Conversational aerobic pace.
    pub easy: Pace,
    /// Deliberately slow recovery pace.
    pub recovery: Pace,
    pub marathon: Pace,
    pub half: Pace,
    pub ten_k: Pace,
    pub five_k: Pace,
    /// ~3K effort, the anchor for short intervals.
    pub interval: Pace,
    /// Comfortably-hard sustained pace, slightly slower than threshold.
    pub tempo: Pace,
}

impl PaceModel {
    /// Build the table from the athlete snapshot the plan is generated from.
    #[must_use]
    pub fn for_athlete(athlete: &AthleteSnapshot) -> Self {
        let threshold_secs = match athlete.vo2max {
            // vVO2max = VO2max / 0.2 ml/kg/m; threshold velocity ~90 % of it.
            // pace = 60000 / (0.9 * vVO2max) = 13333 / VO2max, seconds per km.
            Some(vo2max) if vo2max > 0.0 => (13_333.0 / vo2max).clamp(200.0, 420.0),
            None => match athlete.experience {
                ExperienceLevel::Beginner => 330.0,
                ExperienceLevel::Developing => 285.0,
                ExperienceLevel::Established => 255.0,
                ExperienceLevel::Advanced => 225.0,
            },
        };
        let t = Pace::new(threshold_secs);
        Self {
            threshold: t,
            easy: t.scaled(1.35),
            recovery: t.scaled(1.55),
            marathon: t.scaled(1.08),
            half: t.scaled(1.03),
            ten_k: t.scaled(0.97),
            five_k: t.scaled(0.93),
            interval: t.scaled(0.90),
            tempo: t.scaled(1.05),
        }
    }

    /// The pace the race itself would be run at, for race goals.
    #[must_use]
    pub fn for_goal(&self, goal: GoalKind) -> Pace {
        match goal {
            GoalKind::Marathon => self.marathon,
            GoalKind::HalfMarathon => self.half,
            GoalKind::TenK => self.ten_k,
            GoalKind::FiveK => self.five_k,
            GoalKind::Recovery | GoalKind::Maintain => self.threshold,
        }
    }

    /// The pace the hard portion of a session is prescribed at.
    #[must_use]
    pub fn hard_pace(&self, kind: SessionKind, goal: GoalKind) -> Pace {
        match kind {
            SessionKind::Tempo => self.tempo,
            SessionKind::CruiseIntervals => self.threshold,
            SessionKind::Intervals => self.interval,
            SessionKind::ExtensiveIntervals => match goal {
                GoalKind::Marathon => self.marathon,
                GoalKind::HalfMarathon => self.half,
                _ => self.ten_k,
            },
            SessionKind::Fartlek => self.half,
            SessionKind::Race => self.for_goal(goal),
            SessionKind::Recovery => self.recovery,
            _ => self.easy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runalytics_core::Tz;

    fn athlete() -> AthleteSnapshot {
        AthleteSnapshot::placeholder("Europe/Madrid".parse::<Tz>().expect("valid tz"))
    }

    #[test]
    fn vo2max_drives_the_threshold_pace() {
        let mut a = athlete();
        a.vo2max = Some(50.0);
        let p = PaceModel::for_athlete(&a);
        // 13333 / 50 = 266 s/km, about 4:26.
        assert!((p.threshold.as_secs_per_km() - 266.6).abs() < 1.0);
    }

    #[test]
    fn the_table_orders_from_fastest_to_slowest() {
        let p = PaceModel::for_athlete(&athlete());
        assert!(p.interval < p.five_k);
        assert!(p.five_k < p.ten_k);
        assert!(p.ten_k < p.threshold);
        assert!(p.threshold < p.half);
        assert!(p.half < p.tempo);
        assert!(p.tempo < p.marathon);
        assert!(p.marathon < p.easy);
        assert!(p.easy < p.recovery);
    }

    #[test]
    fn experience_estimates_are_conservative_without_vo2max() {
        let mut a = athlete();
        a.vo2max = None;
        a.experience = ExperienceLevel::Beginner;
        let beginner = PaceModel::for_athlete(&a).threshold;
        a.experience = ExperienceLevel::Advanced;
        let advanced = PaceModel::for_athlete(&a).threshold;
        assert!(advanced < beginner, "more experience, faster estimate");
    }

    #[test]
    fn extensive_intervals_follow_the_goal_distance() {
        let p = PaceModel::for_athlete(&athlete());
        let mp = p.hard_pace(SessionKind::ExtensiveIntervals, GoalKind::Marathon);
        let hmp = p.hard_pace(SessionKind::ExtensiveIntervals, GoalKind::HalfMarathon);
        assert!(hmp < mp, "HMP is faster than MP");
    }
}
