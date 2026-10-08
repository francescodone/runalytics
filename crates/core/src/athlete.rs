//! The athlete: everything the plan engine is allowed to know about a person.

use serde::{Deserialize, Serialize};

use crate::units::{HeartRate, VolumeKm};
use crate::{Date, Tz};

/// Biological sex, used only where the training literature differentiates
/// (heat acclimatisation and haemoglobin-driven intensity ceilings).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sex {
    #[default]
    Unspecified,
    Female,
    Male,
}

/// Self-reported training history, the strongest predictor of how much load a
/// plan may safely add per week.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperienceLevel {
    /// Under 3 months of consistent running, or returning after a long break.
    #[default]
    Beginner,
    /// 3-12 months consistent, sub-60 km weeks typical.
    Developing,
    /// 1-3 years, comfortable at 40-70 km weeks.
    Established,
    /// Multiple race seasons, 70+ km weeks.
    Advanced,
}

impl ExperienceLevel {
    /// Weekly load step ceiling for this level, as a fraction.
    ///
    /// The classic "10 % rule" is unsafe for a beginner whose base is small and
    /// noisy, so the ceiling tightens at the bottom of the range.
    #[must_use]
    pub const fn max_weekly_step(self) -> f64 {
        match self {
            Self::Beginner => 0.08,
            Self::Developing => 0.08,
            Self::Established => 0.10,
            Self::Advanced => 0.10,
        }
    }

    /// Largest share of a week allowed on the long run.
    #[must_use]
    pub const fn max_long_run_share(self) -> f64 {
        match self {
            Self::Beginner => 0.30,
            Self::Developing => 0.32,
            Self::Established | Self::Advanced => 0.35,
        }
    }

    /// Number of quality (non-easy) sessions per week this level can absorb.
    #[must_use]
    pub const fn quality_sessions(self) -> u8 {
        match self {
            Self::Beginner => 0,
            Self::Developing => 1,
            Self::Established => 2,
            Self::Advanced => 3,
        }
    }
}

/// A point-in-time view of the athlete, assembled from profile fields plus the
/// latest provider-derived metrics.
///
/// Deliberately a *snapshot* rather than a live record: a plan generated from a
/// snapshot stays reproducible after the athlete's data moves on, which is what
/// makes plan diffs and regression fixtures meaningful.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AthleteSnapshot {
    /// Age in years at the time of the snapshot.
    pub age: u8,
    pub sex: Sex,
    /// Measured or estimated max heart rate.
    pub max_hr: HeartRate,
    /// Rolling average resting heart rate.
    pub resting_hr: HeartRate,
    /// Body mass in kilograms.
    pub weight_kg: f64,
    /// VO2max as reported by the provider, when available.
    pub vo2max: Option<f64>,
    /// Self-reported training history.
    pub experience: ExperienceLevel,
    /// Mean weekly volume over the trailing 4 weeks — the honest base.
    pub current_weekly_volume: VolumeKm,
    /// Peak weekly volume in the trailing 12 weeks.
    pub peak_weekly_volume: VolumeKm,
    /// Average weekly volume over the trailing 4 weeks, as a fraction of
    /// `peak_weekly_volume`. Low values mean the athlete is detrained and the
    /// engine must build back before it can build up.
    pub consistency: f64,
    /// Recent chronic training load, if any activities have been synced.
    pub chronic_load: Option<f64>,
    /// Date of the most recent race result, used to infer current fitness.
    pub last_race_date: Option<Date>,
    /// Recent injury history that should constrain intensity.
    pub injury_flags: Vec<String>,
    /// The athlete's IANA time zone, used to anchor session start times.
    pub timezone: Tz,
}

impl AthleteSnapshot {
    /// A conservative profile used before any provider data exists, and as the
    /// default in tests and previews.
    #[must_use]
    pub fn placeholder(timezone: Tz) -> Self {
        Self {
            age: 35,
            sex: Sex::Unspecified,
            max_hr: HeartRate::new(185),
            resting_hr: HeartRate::new(55),
            weight_kg: 72.0,
            vo2max: None,
            experience: ExperienceLevel::Developing,
            current_weekly_volume: VolumeKm(30.0),
            peak_weekly_volume: VolumeKm(35.0),
            consistency: 0.85,
            chronic_load: None,
            last_race_date: None,
            injury_flags: Vec::new(),
            timezone,
        }
    }

    /// Heart-rate ceilings per zone, derived from `max_hr`.
    ///
    /// Uses the five-zone model anchored to `%HRmax`, which is what COROS and
    /// Garmin both report against, keeping zone labels comparable across
    /// providers.
    #[must_use]
    pub fn hr_zone_ceiling(&self, zone: crate::workout::HrZone) -> HeartRate {
        HeartRate::new((f64::from(self.max_hr.as_u16()) * zone.ceiling_pct()).round() as u16)
    }

    /// Whether recent injury history should suppress high-intensity work.
    #[must_use]
    pub fn injury_constrained(&self) -> bool {
        !self.injury_flags.is_empty()
    }

    /// Effective weekly step ceiling, tightened when the athlete is detrained
    /// or carrying injury history.
    #[must_use]
    pub fn effective_weekly_step(&self) -> f64 {
        let base = self.experience.max_weekly_step();
        if self.injury_constrained() {
            base.min(0.05)
        } else if self.consistency < 0.6 {
            base.min(0.06)
        } else {
            base
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tz() -> Tz {
        "Europe/Madrid".parse().expect("valid tz")
    }

    #[test]
    fn detrained_athlete_gets_a_smaller_step() {
        let mut a = AthleteSnapshot::placeholder(tz());
        assert!((a.effective_weekly_step() - 0.08).abs() < f64::EPSILON);
        a.consistency = 0.4;
        assert!((a.effective_weekly_step() - 0.06).abs() < f64::EPSILON);
        a.consistency = 0.9;
        a.injury_flags.push("achilles".into());
        assert!((a.effective_weekly_step() - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn zone_ceilings_scale_with_max_hr() {
        let a = AthleteSnapshot::placeholder(tz());
        let z4 = a.hr_zone_ceiling(crate::workout::HrZone::Z4);
        assert_eq!(z4.as_u16(), 167); // 185 * 0.90
    }

    #[test]
    fn experience_orders_by_tolerance() {
        assert!(ExperienceLevel::Beginner < ExperienceLevel::Advanced);
        assert!(
            ExperienceLevel::Beginner.max_weekly_step()
                <= ExperienceLevel::Advanced.max_weekly_step()
        );
    }
}
