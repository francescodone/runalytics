//! Imported training activities.

use serde::{Deserialize, Serialize};

use crate::units::{DurationSecs, HeartRate, Pace, VolumeKm};
use crate::{Date, Timestamp};

/// How hard an activity felt, derived from heart-rate and pace data rather than
/// the athlete's label, so the two sources can be compared.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intensity {
    /// Not enough data to classify.
    #[default]
    Unknown,
    Rest,
    Recovery,
    Aerobic,
    Tempo,
    Threshold,
    Vo2max,
    Neuromuscular,
}

impl Intensity {
    /// Weight applied to duration when computing training load.
    ///
    /// Approximates the Coggan-style intensity factor on a heart-rate basis;
    /// `runalytics-scoring` refines this with gradient-adjusted pace.
    #[must_use]
    pub const fn load_factor(self) -> f64 {
        match self {
            Self::Unknown => 0.5,
            Self::Rest => 0.0,
            Self::Recovery => 0.45,
            Self::Aerobic => 0.65,
            Self::Tempo => 0.85,
            Self::Threshold => 1.0,
            Self::Vo2max => 1.15,
            Self::Neuromuscular => 0.9,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Rest => "Rest",
            Self::Recovery => "Recovery",
            Self::Aerobic => "Aerobic",
            Self::Tempo => "Tempo",
            Self::Threshold => "Threshold",
            Self::Vo2max => "VO2max",
            Self::Neuromuscular => "Neuromuscular",
        }
    }

    /// Stable key for the database column.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Rest => "rest",
            Self::Recovery => "recovery",
            Self::Aerobic => "aerobic",
            Self::Tempo => "tempo",
            Self::Threshold => "threshold",
            Self::Vo2max => "vo2max",
            Self::Neuromuscular => "neuromuscular",
        }
    }
}

impl std::str::FromStr for Intensity {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unknown" | "" => Ok(Self::Unknown),
            "rest" => Ok(Self::Rest),
            "recovery" => Ok(Self::Recovery),
            "aerobic" => Ok(Self::Aerobic),
            "tempo" => Ok(Self::Tempo),
            "threshold" => Ok(Self::Threshold),
            "vo2max" => Ok(Self::Vo2max),
            "neuromuscular" => Ok(Self::Neuromuscular),
            other => Err(format!("unknown intensity '{other}'")),
        }
    }
}

/// A lap or split within an activity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityLap {
    pub index: u32,
    pub start: Timestamp,
    pub duration: DurationSecs,
    pub distance: VolumeKm,
    pub avg_pace: Option<Pace>,
    pub avg_hr: Option<HeartRate>,
    pub max_hr: Option<HeartRate>,
    /// Elevation gained within the lap, metres.
    pub elevation_gain: Option<f64>,
    /// Running cadence, steps per minute.
    pub cadence: Option<u16>,
}

/// The scalar summary of an activity, kept separately from the raw laps so the
/// dashboard can be served without touching lap rows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivitySummary {
    pub distance: VolumeKm,
    pub duration: DurationSecs,
    pub avg_pace: Option<Pace>,
    pub avg_hr: Option<HeartRate>,
    pub max_hr: Option<HeartRate>,
    pub elevation_gain: Option<f64>,
    pub avg_cadence: Option<u16>,
    /// Provider's own training-effect score, kept for cross-validation.
    pub training_load: Option<f64>,
}

/// One recorded training session, as reported by a provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: crate::ids::ActivityId,
    pub account: crate::ids::ProviderAccountId,
    /// The provider's own id, retained verbatim for re-fetch and dedup.
    pub provider_activity_id: String,
    pub name: String,
    pub started_at: Timestamp,
    /// Local calendar day the activity is attributed to.
    pub local_date: Date,
    pub summary: ActivitySummary,
    pub laps: Vec<ActivityLap>,
    /// How hard this actually was, derived by `runalytics-scoring`.
    pub intensity: Intensity,
    /// Planned session this activity is believed to satisfy, when matched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_session: Option<crate::ids::PlannedSessionId>,
    pub fetched_at: Timestamp,
}

impl Activity {
    /// Duration-weighted heart rate, `None` when no lap carried heart-rate data.
    #[must_use]
    pub fn time_in_zone(&self, max_hr: HeartRate) -> Option<f64> {
        if self.laps.is_empty() {
            return self.summary.avg_hr.and_then(|hr| hr.pct_of_max(max_hr));
        }
        let total: u32 = self.laps.iter().map(|l| l.duration.as_u32()).sum();
        if total == 0 {
            return None;
        }
        let weighted: f64 = self
            .laps
            .iter()
            .filter_map(|lap| lap.avg_hr.map(|hr| (lap, hr)))
            .map(|(lap, hr)| f64::from(hr.as_u16()) * f64::from(lap.duration.as_u32()))
            .sum();
        let recorded: u32 = self
            .laps
            .iter()
            .filter(|l| l.avg_hr.is_some())
            .map(|l| l.duration.as_u32())
            .sum();
        if recorded == 0 {
            return None;
        }
        Some(weighted / f64::from(total) / f64::from(max_hr.as_u16()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lap(dur_min: u32, hr: u16) -> ActivityLap {
        ActivityLap {
            index: 1,
            start: Timestamp::default(),
            duration: DurationSecs::from_minutes(dur_min),
            distance: VolumeKm(1.0),
            avg_pace: None,
            avg_hr: Some(HeartRate::new(hr)),
            max_hr: None,
            elevation_gain: None,
            cadence: None,
        }
    }

    #[test]
    fn intensity_factors_are_monotonic() {
        assert!(Intensity::Recovery.load_factor() < Intensity::Aerobic.load_factor());
        assert!(Intensity::Tempo.load_factor() < Intensity::Threshold.load_factor());
    }

    #[test]
    fn time_in_zone_weights_by_duration() {
        let mut activity = Activity {
            id: crate::ids::ActivityId::new(),
            account: crate::ids::ProviderAccountId::new(),
            provider_activity_id: "1".into(),
            name: "test".into(),
            started_at: Timestamp::default(),
            local_date: Date::from_ymd_opt(2026, 1, 1).expect("test date"),
            summary: ActivitySummary {
                distance: VolumeKm(10.0),
                duration: DurationSecs::from_minutes(60),
                avg_pace: None,
                avg_hr: None,
                max_hr: None,
                elevation_gain: None,
                avg_cadence: None,
                training_load: None,
            },
            laps: vec![lap(40, 120), lap(20, 180)],
            intensity: Intensity::Aerobic,
            matched_session: None,
            fetched_at: Timestamp::default(),
        };
        let pct = activity.time_in_zone(HeartRate::new(200)).expect("has hr");
        // (120*40 + 180*20) / 60 = 140 bpm -> 0.70 of max
        assert!((pct - 0.70).abs() < 0.001);

        activity.laps = vec![];
        assert_eq!(activity.time_in_zone(HeartRate::new(200)), None);
    }
}
