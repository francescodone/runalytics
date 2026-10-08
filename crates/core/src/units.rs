//! Physical quantities used across the training model.
//!
//! Each quantity is a newtype so that arithmetic between incompatible
//! dimensions is rejected at compile time. All quantities are `Copy`, cheap to
//! pass around, and serialise as plain numbers so the JSON contract with the
//! frontend stays simple.

use std::fmt;
use std::ops::{Add, Sub};

use serde::{Deserialize, Serialize};

/// Distance in kilometres.
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VolumeKm(pub f64);

impl VolumeKm {
    pub const ZERO: Self = Self(0.0);

    #[must_use]
    pub fn new(km: f64) -> Self {
        Self(km.max(0.0))
    }

    /// Convert to miles (1 km = 0.621371 mi).
    #[must_use]
    pub fn as_miles(self) -> f64 {
        self.0 * 0.621_371
    }

    #[must_use]
    pub fn as_f64(self) -> f64 {
        self.0
    }

    /// Round to the nearest increment, used when writing human-friendly
    /// targets into a plan (e.g. 9.7 km -> 10.0 km).
    #[must_use]
    pub fn rounded_to(self, increment: f64) -> Self {
        if increment <= 0.0 {
            return self;
        }
        Self((self.0 / increment).round() * increment)
    }
}

impl Add for VolumeKm {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for VolumeKm {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self((self.0 - rhs.0).max(0.0))
    }
}

impl fmt::Display for VolumeKm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1} km", self.0)
    }
}

/// Duration in whole seconds.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash,
)]
#[serde(transparent)]
pub struct DurationSecs(pub u32);

impl DurationSecs {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub const fn new(seconds: u32) -> Self {
        Self(seconds)
    }

    #[must_use]
    pub const fn from_minutes(minutes: u32) -> Self {
        Self(minutes * 60)
    }

    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    #[must_use]
    pub const fn as_minutes(self) -> u32 {
        self.0 / 60
    }

    #[must_use]
    pub const fn as_f64(self) -> f64 {
        self.0 as f64
    }

    /// `h:mm:ss`, or `mm:ss` under an hour — the format runners expect.
    #[must_use]
    pub fn format_clock(self) -> String {
        let (h, m, s) = (self.0 / 3600, (self.0 % 3600) / 60, self.0 % 60);
        if h > 0 {
            format!("{h}:{m:02}:{s:02}")
        } else {
            format!("{m}:{s:02}")
        }
    }
}

impl Add for DurationSecs {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for DurationSecs {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl fmt::Display for DurationSecs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format_clock())
    }
}

/// Pace in seconds per kilometre. Lower is faster.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Pace(pub f64);

impl Default for Pace {
    /// A deliberately slow default so an unset pace never masquerades as a
    /// fast one.
    fn default() -> Self {
        Self(420.0)
    }
}

impl Pace {
    /// Slowest pace modelled — used to clamp recovery efforts.
    pub const WALK: Self = Self(720.0);

    #[must_use]
    pub fn new(secs_per_km: f64) -> Self {
        Self(secs_per_km.clamp(120.0, f64::from(Self::WALK.0)))
    }

    /// Derive pace from a distance and elapsed time.
    #[must_use]
    pub fn from_parts(distance_km: VolumeKm, duration: DurationSecs) -> Option<Self> {
        if distance_km.0 < 0.01 || duration.0 == 0 {
            return None;
        }
        Some(Self::new(f64::from(duration.0) / distance_km.0))
    }

    /// Inverse relationship: a `factor` above 1.0 means *slower*.
    #[must_use]
    pub fn scaled(self, factor: f64) -> Self {
        Self::new(self.0 * factor)
    }

    #[must_use]
    pub fn as_secs_per_km(self) -> f64 {
        self.0
    }

    /// `m:ss /km`, e.g. `4:32`.
    #[must_use]
    pub fn format(self) -> String {
        let total = self.0.round();
        let m = (total / 60.0).floor() as u32;
        let s = (total - f64::from(m * 60)) as u32;
        format!("{m}:{s:02}")
    }
}

impl fmt::Display for Pace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/km", self.format())
    }
}

/// Heart rate in beats per minute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct HeartRate(pub u16);

impl HeartRate {
    /// Sentinel for "no heart-rate data recorded".
    pub const NONE: Self = Self(0);

    #[must_use]
    pub const fn new(bpm: u16) -> Self {
        Self(bpm)
    }

    #[must_use]
    pub const fn as_u16(self) -> u16 {
        self.0
    }

    #[must_use]
    pub const fn is_recorded(self) -> bool {
        self.0 > 0
    }

    /// Percentage of max heart rate, `0.0..=1.0`. Returns `None` when either
    /// input is a sentinel so callers never divide by zero.
    #[must_use]
    pub fn pct_of_max(self, max_hr: Self) -> Option<f64> {
        if self.0 == 0 || max_hr.0 == 0 {
            return None;
        }
        Some(f64::from(self.0) / f64::from(max_hr.0))
    }
}

impl fmt::Display for HeartRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == 0 {
            f.write_str("-- bpm")
        } else {
            write!(f, "{} bpm", self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pace_formats_like_a_runner_expects() {
        assert_eq!(Pace::new(272.0).format(), "4:32");
        assert_eq!(Pace::new(300.0).format(), "5:00");
        assert_eq!(Pace::new(270.5).format(), "4:31");
    }

    #[test]
    fn clock_duration_drops_the_hour_when_zero() {
        assert_eq!(DurationSecs::from_minutes(42).format_clock(), "42:00");
        assert_eq!(DurationSecs(5_400).format_clock(), "1:30:00");
    }

    #[test]
    fn pace_from_parts_rejects_degenerate_input() {
        assert!(Pace::from_parts(VolumeKm(0.0), DurationSecs::from_minutes(30)).is_none());
        assert!(Pace::from_parts(VolumeKm(10.0), DurationSecs(0)).is_none());
        let p = Pace::from_parts(VolumeKm(10.0), DurationSecs::from_minutes(50)).expect("valid");
        assert!((p.as_secs_per_km() - 300.0).abs() < 0.01);
    }

    #[test]
    fn volume_rounds_to_training_increments() {
        assert_eq!(VolumeKm(9.7).rounded_to(0.5).as_f64(), 9.5);
        assert_eq!(VolumeKm(9.8).rounded_to(1.0).as_f64(), 10.0);
    }

    #[test]
    fn pct_of_max_guards_sentinels() {
        assert_eq!(HeartRate::NONE.pct_of_max(HeartRate::new(190)), None);
        assert_eq!(HeartRate::new(190).pct_of_max(HeartRate::NONE), None);
        let pct = HeartRate::new(95)
            .pct_of_max(HeartRate::new(190))
            .expect("valid");
        assert!((pct - 0.5).abs() < f64::EPSILON);
    }
}
