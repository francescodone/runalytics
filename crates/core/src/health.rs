//! Daily health and recovery data.

use serde::{Deserialize, Serialize};

use crate::Date;
use crate::units::{DurationSecs, HeartRate};

/// One night's sleep, normalised across providers.
///
/// Providers disagree about what "sleep" includes: COROS reports a single
/// window, Garmin splits main sleep from naps. Runalytics keeps the main window
/// plus a nap total, because the training model cares about *consolidated*
/// overnight sleep and treats naps as a separate recovery credit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SleepDay {
    pub date: Date,
    /// Main sleep window duration.
    pub total: DurationSecs,
    pub deep: DurationSecs,
    pub light: DurationSecs,
    pub rem: DurationSecs,
    pub awake: DurationSecs,
    /// Additional daytime sleep.
    pub nap: DurationSecs,
    /// Provider's own 0-100 sleep score, retained for cross-validation.
    pub score: Option<u8>,
    /// Lowest overnight heart rate, a leading indicator of illness and fatigue.
    pub lowest_hr: Option<HeartRate>,
    /// Heart-rate variability during sleep, ms. The single most useful recovery
    /// signal available from a consumer wearable.
    pub hrv: Option<f64>,
    /// Respiratory rate, breaths per minute.
    pub respiratory_rate: Option<f64>,
}

impl SleepDay {
    /// Share of the main window spent in deep sleep, `0.0..=1.0`.
    #[must_use]
    pub fn deep_share(&self) -> f64 {
        if self.total.as_u32() == 0 {
            return 0.0;
        }
        f64::from(self.deep.as_u32()) / f64::from(self.total.as_u32())
    }

    /// Share of the main window spent in REM sleep.
    #[must_use]
    pub fn rem_share(&self) -> f64 {
        if self.total.as_u32() == 0 {
            return 0.0;
        }
        f64::from(self.rem.as_u32()) / f64::from(self.total.as_u32())
    }

    /// Sleep opportunity: time in bed, including awake periods.
    #[must_use]
    pub fn opportunity(&self) -> DurationSecs {
        self.total + self.awake
    }

    /// Sleep efficiency: the share of time in bed actually spent asleep.
    /// Below ~85 % is clinically relevant insomnia and degrades recovery.
    #[must_use]
    pub fn efficiency(&self) -> f64 {
        let opp = self.opportunity().as_u32();
        if opp == 0 {
            return 0.0;
        }
        f64::from(self.total.as_u32()) / f64::from(opp)
    }
}

/// Aggregated health metrics for one local day.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthDay {
    pub id: crate::ids::HealthDayId,
    pub account: crate::ids::ProviderAccountId,
    pub date: Date,
    pub resting_hr: Option<HeartRate>,
    /// Mean daytime stress, provider scale 1-99.
    pub avg_stress: Option<f64>,
    /// Minutes spent in the high-stress band.
    pub high_stress_minutes: Option<u32>,
    pub sleep: Option<SleepDay>,
    /// Steps recorded.
    pub steps: Option<u32>,
    /// Provider's own readiness-style score, when it publishes one.
    pub provider_readiness: Option<u8>,
    /// Resting metabolic estimate, kcal.
    pub basal_energy: Option<f64>,
}

impl HealthDay {
    /// Whether any recovery-relevant signal was recorded.
    ///
    /// A day with only step data must not be treated as a full recovery record,
    /// or the readiness model silently over-trusts a partial sync.
    #[must_use]
    pub fn has_recovery_signal(&self) -> bool {
        self.resting_hr.is_some() || self.sleep.is_some() || self.avg_stress.is_some()
    }
}

/// Provider-reported fitness estimates, tracked over time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FitnessAssessment {
    pub date: Date,
    pub vo2max: Option<f64>,
    /// Running level / performance index, provider scale.
    pub running_level: Option<f64>,
    /// Threshold pace as reported by the provider.
    pub threshold_pace: Option<crate::units::Pace>,
    /// Race predictions the provider publishes, minutes per kilometre by race.
    pub predicted_paces: Vec<(String, f64)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sleep(total_min: u32, awake_min: u32) -> SleepDay {
        SleepDay {
            date: Date::from_ymd_opt(2026, 1, 1).expect("test date"),
            total: DurationSecs::from_minutes(total_min),
            deep: DurationSecs::from_minutes(total_min / 5),
            light: DurationSecs::from_minutes(total_min / 2),
            rem: DurationSecs::from_minutes(total_min / 4),
            awake: DurationSecs::from_minutes(awake_min),
            nap: DurationSecs::ZERO,
            score: None,
            lowest_hr: None,
            hrv: None,
            respiratory_rate: None,
        }
    }

    #[test]
    fn efficiency_uses_time_in_bed() {
        let s = sleep(420, 60);
        assert!((s.efficiency() - 0.875).abs() < 0.001);
        assert_eq!(sleep(0, 0).efficiency(), 0.0);
    }

    #[test]
    fn stage_shares_are_bounded() {
        let s = sleep(480, 0);
        assert!((s.deep_share() - 0.2).abs() < 0.01);
        assert!((s.rem_share() - 0.25).abs() < 0.01);
    }

    #[test]
    fn steps_alone_are_not_a_recovery_signal() {
        let day = HealthDay {
            id: crate::ids::HealthDayId::new(),
            account: crate::ids::ProviderAccountId::new(),
            date: Date::from_ymd_opt(2026, 1, 1).expect("test date"),
            resting_hr: None,
            avg_stress: None,
            high_stress_minutes: None,
            sleep: None,
            steps: Some(9_000),
            provider_readiness: None,
            basal_energy: None,
        };
        assert!(!day.has_recovery_signal());
    }
}
