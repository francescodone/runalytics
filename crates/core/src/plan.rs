//! Training plans: the goal, the anchor, the phases and the emitted sessions.

use chrono::Datelike;
use serde::{Deserialize, Serialize};

use crate::athlete::AthleteSnapshot;
use crate::units::VolumeKm;
use crate::workout::PlannedSession;
use crate::{Date, DomainError};

/// What the block is building toward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalKind {
    /// Full marathon.
    Marathon,
    /// Half marathon.
    HalfMarathon,
    /// 10 km road race.
    TenK,
    /// 5 km road race.
    FiveK,
    /// No race: rebuild after a block, illness or injury.
    Recovery,
    /// No race: hold current fitness through a busy patch.
    Maintain,
}

impl GoalKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Marathon => "marathon",
            Self::HalfMarathon => "half_marathon",
            Self::TenK => "ten_k",
            Self::FiveK => "five_k",
            Self::Recovery => "recovery",
            Self::Maintain => "maintain",
        }
    }

    /// Short label for UI.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Marathon => "Marathon",
            Self::HalfMarathon => "Half marathon",
            Self::TenK => "10K",
            Self::FiveK => "5K",
            Self::Recovery => "Recovery",
            Self::Maintain => "Maintain",
        }
    }

    /// Whether the goal terminates in a race.
    #[must_use]
    pub const fn is_race(self) -> bool {
        matches!(
            self,
            Self::Marathon | Self::HalfMarathon | Self::TenK | Self::FiveK
        )
    }

    /// Shortest block that can produce a meaningful adaptation, in weeks.
    ///
    /// Below this the engine will still emit a plan, but it is a sharpening
    /// block rather than a training block, and the UI should say so.
    #[must_use]
    pub const fn minimum_weeks(self) -> u8 {
        match self {
            Self::Marathon => 8,
            Self::HalfMarathon => 5,
            Self::TenK => 4,
            Self::FiveK => 3,
            Self::Recovery | Self::Maintain => 1,
        }
    }

    /// Taper length in days before the start line.
    ///
    /// Marathon needs the longest clearance because the long run that produces
    /// the adaptation also produces the fatigue that must dissipate.
    #[must_use]
    pub const fn taper_days(self) -> u32 {
        match self {
            Self::Marathon => 21,
            Self::HalfMarathon => 14,
            Self::TenK => 10,
            Self::FiveK => 7,
            Self::Recovery | Self::Maintain => 0,
        }
    }

    /// Peak weekly volume as a multiple of the athlete's current base.
    #[must_use]
    pub const fn peak_volume_multiplier(self) -> f64 {
        match self {
            Self::Marathon => 1.35,
            Self::HalfMarathon => 1.22,
            Self::TenK => 1.12,
            Self::FiveK => 1.06,
            Self::Recovery => 0.70,
            Self::Maintain => 1.0,
        }
    }

    /// How the COROS API labels this goal, when it accepts one.
    #[must_use]
    pub const fn coros_goal(self) -> Option<&'static str> {
        match self {
            Self::Marathon => Some("MARATHON"),
            Self::HalfMarathon => Some("HALF_MARATHON"),
            Self::TenK => Some("TEN_K"),
            Self::FiveK => Some("FIVE_K"),
            Self::Recovery | Self::Maintain => None,
        }
    }

    /// The race distance in kilometres, for race goals.
    ///
    /// Road standard distances; the engine uses this to size the race-week
    /// session and to label the plan. `None` for the non-race goals.
    #[must_use]
    pub const fn race_distance_km(self) -> Option<f64> {
        match self {
            Self::Marathon => Some(42.2),
            Self::HalfMarathon => Some(21.1),
            Self::TenK => Some(10.0),
            Self::FiveK => Some(5.0),
            Self::Recovery | Self::Maintain => None,
        }
    }
}

impl std::fmt::Display for GoalKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

impl std::str::FromStr for GoalKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s
            .trim()
            .to_ascii_lowercase()
            .replace(['-', ' '], "_")
            .as_str()
        {
            "marathon" | "full_marathon" => Ok(Self::Marathon),
            "half_marathon" | "half" | "hm" => Ok(Self::HalfMarathon),
            "ten_k" | "10k" | "tenk" => Ok(Self::TenK),
            "five_k" | "5k" | "fivek" => Ok(Self::FiveK),
            "recovery" => Ok(Self::Recovery),
            "maintain" | "maintenance" => Ok(Self::Maintain),
            other => Err(format!("unknown goal '{other}'")),
        }
    }
}

/// How the end of a plan is fixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
pub enum Anchor {
    /// Train for `weeks` from the start date. Capped at 10 by the product brief.
    Horizon { weeks: u8 },
    /// Build backward from a race or target date.
    RaceDate { date: Date },
}

/// The concrete dates a request resolved to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorResolution {
    pub start: Date,
    pub end: Date,
    pub weeks: u8,
    /// The race date, for race goals.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub race_date: Option<Date>,
    /// True when the requested horizon had to be shortened or lengthened.
    pub adjusted: bool,
    /// Human-readable note explaining any adjustment, shown in the UI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A named training phase with its week range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseSpan {
    pub phase: Phase,
    /// Inclusive week index, 0-based.
    pub from_week: u8,
    /// Inclusive week index.
    pub to_week: u8,
    /// Short rationale shown in the plan timeline.
    pub note: String,
}

/// The macro-cycle phase a week belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Movement, leg strength, short strides. Prepares tissue before load.
    Base,
    /// Volume and quality both rising.
    Build,
    /// Highest load of the block.
    Peak,
    /// Deliberate reduction to dissipate fatigue.
    Taper,
    /// The race itself.
    Race,
    /// Easy volume after a race or hard block.
    Rebuild,
    /// Flat load, no progression.
    Hold,
}

impl Phase {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Build => "build",
            Self::Peak => "peak",
            Self::Taper => "taper",
            Self::Race => "race",
            Self::Rebuild => "rebuild",
            Self::Hold => "hold",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Base => "Base",
            Self::Build => "Build",
            Self::Peak => "Peak",
            Self::Taper => "Taper",
            Self::Race => "Race",
            Self::Rebuild => "Rebuild",
            Self::Hold => "Hold",
        }
    }

    /// Multiplier applied to the week's target volume.
    #[must_use]
    pub const fn volume_factor(self) -> f64 {
        match self {
            Self::Base => 0.85,
            Self::Build => 1.0,
            Self::Peak => 1.1,
            Self::Taper => 0.65,
            Self::Race => 0.35,
            Self::Rebuild => 0.6,
            Self::Hold => 1.0,
        }
    }

    /// Multiplier applied to the number of quality sessions.
    #[must_use]
    pub const fn quality_factor(self) -> f64 {
        match self {
            Self::Base => 0.34,
            Self::Build => 1.0,
            Self::Peak => 1.0,
            Self::Taper => 0.5,
            Self::Race => 0.0,
            Self::Rebuild => 0.0,
            Self::Hold => 0.5,
        }
    }
}

impl std::str::FromStr for Phase {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "base" => Ok(Self::Base),
            "build" => Ok(Self::Build),
            "peak" => Ok(Self::Peak),
            "taper" => Ok(Self::Taper),
            "race" => Ok(Self::Race),
            "rebuild" => Ok(Self::Rebuild),
            "hold" => Ok(Self::Hold),
            other => Err(format!("unknown phase '{other}'")),
        }
    }
}

/// Where a plan sits in its lifecycle.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    /// Generated but not yet accepted by the athlete.
    #[default]
    Draft,
    /// Accepted: sessions are pushed to the provider and the calendar.
    Active,
    /// The final session has been executed.
    Completed,
    /// Stopped by the athlete; retained for history and adherence reports.
    Paused,
    /// Superseded by a re-plan. Never deleted, so past scores stay explainable.
    Archived,
}

impl PlanStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Paused => "paused",
            Self::Archived => "archived",
        }
    }

    /// Whether sessions in this plan may still be mutated or pushed.
    #[must_use]
    pub const fn is_mutable(self) -> bool {
        matches!(self, Self::Draft | Self::Active | Self::Paused)
    }
}

impl std::str::FromStr for PlanStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(Self::Draft),
            "active" => Ok(Self::Active),
            "completed" => Ok(Self::Completed),
            "paused" => Ok(Self::Paused),
            "archived" => Ok(Self::Archived),
            other => Err(format!("unknown plan status '{other}'")),
        }
    }
}

/// One week of the plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanWeek {
    /// 0-based index within the plan.
    pub index: u8,
    pub phase: Phase,
    /// Monday of this week in the athlete's time zone.
    pub start: Date,
    pub end: Date,
    pub target_volume: VolumeKm,
    /// Previous week's volume, so the UI can show the step without recomputing.
    pub previous_volume: VolumeKm,
    /// Fractional change against `previous_volume`.
    pub step_pct: f64,
    /// Planned acute:chronic ratio for the end of this week.
    pub projected_acwr: f64,
    pub sessions: Vec<PlannedSession>,
    /// Deload weeks are intentionally easy and must not be flagged as
    /// under-delivery by the adherence report.
    pub is_deload: bool,
}

impl PlanWeek {
    /// Volume actually attributed to running.
    #[must_use]
    pub fn actual_volume(&self) -> VolumeKm {
        self.sessions
            .iter()
            .fold(VolumeKm::ZERO, |acc, s| acc + s.volume())
    }

    /// Number of quality sessions planned.
    #[must_use]
    pub fn quality_count(&self) -> usize {
        self.sessions.iter().filter(|s| s.quality).count()
    }

    /// Largest single session as a share of the week.
    #[must_use]
    pub fn long_run_share(&self) -> f64 {
        let total = self.actual_volume().as_f64();
        if total <= 0.0 {
            return 0.0;
        }
        let longest = self
            .sessions
            .iter()
            .map(|s| s.volume().as_f64())
            .fold(0.0_f64, f64::max);
        longest / total
    }
}

/// A generated training plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub id: crate::ids::PlanId,
    pub name: String,
    pub goal: GoalKind,
    pub status: PlanStatus,
    pub anchor: Anchor,
    pub resolution: AnchorResolution,
    pub phases: Vec<PhaseSpan>,
    pub weeks: Vec<PlanWeek>,
    /// The athlete profile the plan was generated from, retained so the plan is
    /// reproducible and auditable.
    pub athlete: AthleteSnapshot,
    /// Weekly volume ceiling the engine was allowed to exceed, for reporting.
    pub ceiling_volume: VolumeKm,
    /// Whether this plan can be pushed to COROS as a single plan object.
    ///
    /// COROS accepts plans of 4-16 natural weeks starting within 14 days of
    /// today. Anything shorter must be pushed as individual scheduled
    /// workouts, which is why this flag exists on the model rather than in the
    /// exporter.
    pub emittable_as_coros_plan: bool,
    /// Provider-side plan ids after a successful push, keyed by provider.
    pub external_ids: Vec<(String, String)>,
    pub created_at: crate::Timestamp,
}

impl Plan {
    /// Total planned volume across the block.
    #[must_use]
    pub fn total_volume(&self) -> VolumeKm {
        self.weeks
            .iter()
            .fold(VolumeKm::ZERO, |acc, w| acc + w.actual_volume())
    }

    /// Every session in date order.
    pub fn all_sessions(&self) -> impl Iterator<Item = &PlannedSession> {
        self.weeks.iter().flat_map(|w| &w.sessions)
    }

    /// The session scheduled for a given day, if any.
    pub fn session_on(&self, date: Date) -> Option<&PlannedSession> {
        self.all_sessions().find(|s| s.date == date)
    }

    /// Look up a session by id, for MCP mutations.
    pub fn session_by_id(&self, id: crate::ids::PlannedSessionId) -> Option<&PlannedSession> {
        self.all_sessions().find(|s| s.id == id)
    }

    /// Mutable lookup used by the re-plan path.
    pub fn session_by_id_mut(
        &mut self,
        id: crate::ids::PlannedSessionId,
    ) -> Option<&mut PlannedSession> {
        self.weeks
            .iter_mut()
            .flat_map(|w| &mut w.sessions)
            .find(|s| s.id == id)
    }

    /// The peak week by volume, which is what the UI headlines.
    pub fn peak_week(&self) -> Option<&PlanWeek> {
        self.weeks.iter().max_by(|a, b| {
            a.actual_volume()
                .partial_cmp(&b.actual_volume())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Weeks that carry a race goal but are shorter than the goal's stated
    /// minimum, meaning the block is a sharpening block, not a build.
    #[must_use]
    pub fn is_short_for_goal(&self) -> bool {
        self.goal.is_race()
            && u32::from(self.resolution.weeks) < u32::from(self.goal.minimum_weeks())
    }

    /// Validate the internal consistency of a plan loaded from storage.
    ///
    /// Guards against a partially-written row or a hand-edited database
    /// producing a plan the UI would render as nonsense.
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.weeks.is_empty() {
            return Err(DomainError::UnbuildablePlan("plan has no weeks".into()));
        }
        if self.resolution.weeks == 0 || u32::from(self.resolution.weeks) > 10 {
            return Err(DomainError::HorizonOutOfRange {
                weeks: u32::from(self.resolution.weeks),
                min: 1,
                max: 10,
            });
        }
        for week in &self.weeks {
            if week.end < week.start {
                return Err(DomainError::UnbuildablePlan(format!(
                    "week {} ends before it starts",
                    week.index
                )));
            }
            for session in &week.sessions {
                if session.date < week.start || session.date > week.end {
                    return Err(DomainError::UnbuildablePlan(format!(
                        "session {} falls outside week {}",
                        session.date, week.index
                    )));
                }
            }
        }
        Ok(())
    }
}

/// Constraints the athlete imposes on a plan, layered over the engine's defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanConstraints {
    /// Days of the week (0 = Monday) the athlete cannot train.
    pub blackout_weekdays: Vec<u32>,
    /// Days the athlete will not do a quality session, e.g. workdays.
    pub no_quality_weekdays: Vec<u32>,
    /// Preferred start time for sessions, local.
    pub preferred_start: crate::TimeOfDay,
    /// Hard ceiling on weekly volume the athlete is willing to commit to.
    pub max_weekly_volume: Option<VolumeKm>,
    /// Explicitly requested number of quality sessions, overriding the level
    /// default.
    pub quality_sessions: Option<u8>,
    /// Dates that must be kept free (travel, family).
    pub blackout_dates: Vec<Date>,
    /// Long-run day preference, 0 = Monday.
    pub long_run_weekday: Option<u32>,
}

impl Default for PlanConstraints {
    fn default() -> Self {
        Self {
            blackout_weekdays: Vec::new(),
            no_quality_weekdays: Vec::new(),
            preferred_start: crate::TimeOfDay::from_hms_opt(7, 0, 0).unwrap(),
            max_weekly_volume: None,
            quality_sessions: None,
            blackout_dates: Vec::new(),
            long_run_weekday: None,
        }
    }
}

impl PlanConstraints {
    /// Whether training is allowed on a date.
    #[must_use]
    pub fn allows_training(&self, date: Date) -> bool {
        if self.blackout_dates.contains(&date) {
            return false;
        }
        let weekday = date.weekday().num_days_from_monday();
        !self.blackout_weekdays.contains(&weekday)
    }

    /// Whether a quality session is allowed on a date.
    #[must_use]
    pub fn allows_quality(&self, date: Date) -> bool {
        if !self.allows_training(date) {
            return false;
        }
        let weekday = date.weekday().num_days_from_monday();
        !self.no_quality_weekdays.contains(&weekday)
    }
}

/// Everything needed to generate a plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanRequest {
    pub goal: GoalKind,
    pub anchor: Anchor,
    pub athlete: AthleteSnapshot,
    pub constraints: PlanConstraints,
    /// First day of the plan. Defaults to the next Monday when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<Date>,
}

impl PlanRequest {
    /// Resolve the anchor into concrete dates, clamping to the supported window.
    ///
    /// A race date is authoritative — the engine must fit inside it even when
    /// that means fewer weeks than the goal would ideally want. A horizon is a
    /// request and gets clamped to 1-10 weeks with a note explaining why.
    pub fn resolve(&self, today: Date) -> Result<AnchorResolution, DomainError> {
        const MIN_WEEKS: u32 = 1;
        const MAX_WEEKS: u32 = 10;

        let start = self
            .start_date
            .unwrap_or_else(|| next_monday(today))
            .max(today);

        match self.anchor {
            Anchor::Horizon { weeks } => {
                let requested = u32::from(weeks);
                let clamped = requested.clamp(MIN_WEEKS, MAX_WEEKS);
                let adjusted = clamped != requested;
                let note = adjusted.then(|| {
                    format!(
                        "Adjusted from {requested} to {clamped} weeks — Runalytics plans between {MIN_WEEKS} and {MAX_WEEKS} weeks."
                    )
                });
                let start = next_monday(start);
                let days = i64::from(clamped) * 7 - 1;
                Ok(AnchorResolution {
                    start,
                    end: start + chrono::Duration::days(days),
                    weeks: clamped as u8,
                    race_date: None,
                    adjusted,
                    note,
                })
            }
            Anchor::RaceDate { date } => {
                if date < today {
                    return Err(DomainError::RaceDateUnusable(format!(
                        "{date} is in the past"
                    )));
                }
                let days = (date - start).num_days();
                if days < 3 {
                    return Err(DomainError::RaceDateUnusable(format!(
                        "{date} is too close to train safely — at least a full week is needed"
                    )));
                }
                // Round the span up to whole weeks, then clamp to the window.
                let natural_weeks =
                    ((days + 6) / 7).clamp(i64::from(MIN_WEEKS), i64::from(MAX_WEEKS));
                let weeks = natural_weeks as u8;
                let adjusted = natural_weeks != (days + 6) / 7;
                let note = adjusted.then(|| {
                    format!(
                        "Only {MAX_WEEKS} weeks of build are available before {date}; the plan starts later to keep the block coherent."
                    )
                });
                // Anchor the start so the plan ends exactly on race day.
                let start = if adjusted {
                    let back = date - chrono::Duration::days(i64::from(weeks) * 7 - 1);
                    next_monday(back.max(today))
                } else {
                    next_monday(start)
                };
                Ok(AnchorResolution {
                    start,
                    end: date,
                    weeks,
                    race_date: Some(date),
                    adjusted,
                    note,
                })
            }
        }
    }

    /// Whether the resolved plan can be pushed to COROS as one plan object.
    #[must_use]
    pub fn coros_plan_eligible(resolution: &AnchorResolution, today: Date) -> bool {
        let weeks = u32::from(resolution.weeks);
        (4..=16).contains(&weeks) && (resolution.start - today).num_days() <= 14
    }
}

/// The Monday of the current week, or next Monday if today is already Monday.
///
/// Plans always start on a Monday so week boundaries line up with the athlete's
/// mental model and with the calendar grid.
#[must_use]
pub fn next_monday(date: Date) -> Date {
    let offset = date.weekday().num_days_from_monday();
    if offset == 0 {
        date
    } else {
        date + chrono::Duration::days(i64::from(7 - offset))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SessionKind;

    fn date(y: i32, m: u32, d: u32) -> Date {
        Date::from_ymd_opt(y, m, d).expect("test date")
    }

    fn tz() -> crate::Tz {
        "Europe/Madrid".parse().unwrap()
    }

    fn request(goal: GoalKind, anchor: Anchor) -> PlanRequest {
        PlanRequest {
            goal,
            anchor,
            athlete: AthleteSnapshot::placeholder(tz()),
            constraints: PlanConstraints::default(),
            start_date: None,
        }
    }

    #[test]
    fn monday_is_stable_and_other_days_roll_forward() {
        assert_eq!(next_monday(date(2026, 10, 5)), date(2026, 10, 5)); // Monday
        assert_eq!(next_monday(date(2026, 10, 6)), date(2026, 10, 12)); // Tuesday
        assert_eq!(next_monday(date(2026, 10, 11)), date(2026, 10, 12)); // Sunday
    }

    #[test]
    fn horizon_clamps_above_ten_weeks() {
        let req = request(GoalKind::Marathon, Anchor::Horizon { weeks: 18 });
        let r = req.resolve(date(2026, 10, 6)).expect("resolves");
        assert_eq!(r.weeks, 10);
        assert!(r.adjusted);
        assert!(r.note.is_some());
    }

    #[test]
    fn horizon_below_one_week_is_rejected_by_the_type_and_clamped_in_practice() {
        let req = request(GoalKind::Recovery, Anchor::Horizon { weeks: 0 });
        let r = req.resolve(date(2026, 10, 6)).expect("resolves");
        assert_eq!(r.weeks, 1);
    }

    #[test]
    fn race_date_is_authoritative_and_ends_on_the_race() {
        let race = date(2027, 3, 14);
        let req = request(GoalKind::Marathon, Anchor::RaceDate { date: race });
        let r = req.resolve(date(2026, 10, 6)).expect("resolves");
        assert_eq!(r.end, race);
        assert_eq!(r.race_date, Some(race));
        assert_eq!(r.weeks, 10, "clamped to the window");
        assert!(r.adjusted);
    }

    #[test]
    fn past_race_dates_are_rejected() {
        let req = request(
            GoalKind::TenK,
            Anchor::RaceDate {
                date: date(2025, 1, 1),
            },
        );
        assert!(matches!(
            req.resolve(date(2026, 10, 6)),
            Err(DomainError::RaceDateUnusable(_))
        ));
    }

    #[test]
    fn short_notice_race_reports_fewer_weeks() {
        let race = date(2026, 10, 31);
        let req = request(GoalKind::FiveK, Anchor::RaceDate { date: race });
        let r = req.resolve(date(2026, 10, 6)).expect("resolves");
        assert_eq!(r.end, race);
        assert!(r.weeks <= 4);
    }

    #[test]
    fn coros_eligibility_needs_four_weeks_and_a_near_start() {
        let today = date(2026, 10, 6);
        let base = |weeks: u8, start: Date| AnchorResolution {
            start,
            end: start + chrono::Duration::days(i64::from(weeks) * 7 - 1),
            weeks,
            race_date: None,
            adjusted: false,
            note: None,
        };
        assert!(PlanRequest::coros_plan_eligible(
            &base(4, date(2026, 10, 12)),
            today
        ));
        assert!(
            !PlanRequest::coros_plan_eligible(&base(3, date(2026, 10, 12)), today),
            "COROS rejects plans under 4 weeks"
        );
        assert!(
            !PlanRequest::coros_plan_eligible(&base(4, date(2026, 12, 1)), today),
            "COROS rejects a start more than 14 days out"
        );
    }

    #[test]
    fn constraints_block_days_and_quality_days() {
        let c = PlanConstraints {
            blackout_weekdays: vec![0],
            no_quality_weekdays: vec![2],
            ..Default::default()
        };
        assert!(!c.allows_training(date(2026, 10, 5)), "Monday is blocked");
        assert!(c.allows_training(date(2026, 10, 6)));
        assert!(
            !c.allows_quality(date(2026, 10, 7)),
            "Wednesday is easy-only"
        );
        assert!(c.allows_quality(date(2026, 10, 6)));
    }

    #[test]
    fn goal_parses_the_way_people_type_it() {
        assert_eq!("10K".parse::<GoalKind>().unwrap(), GoalKind::TenK);
        assert_eq!("Half".parse::<GoalKind>().unwrap(), GoalKind::HalfMarathon);
        assert_eq!(
            "full-marathon".parse::<GoalKind>().unwrap(),
            GoalKind::Marathon
        );
        assert!("ultra".parse::<GoalKind>().is_err());
    }

    #[test]
    fn taper_shortens_with_race_distance() {
        assert!(GoalKind::Marathon.taper_days() > GoalKind::HalfMarathon.taper_days());
        assert!(GoalKind::HalfMarathon.taper_days() > GoalKind::TenK.taper_days());
        assert!(GoalKind::TenK.taper_days() > GoalKind::FiveK.taper_days());
    }

    #[test]
    fn week_helpers_aggregate_sessions() {
        let mut week = PlanWeek {
            index: 0,
            phase: Phase::Base,
            start: date(2026, 10, 12),
            end: date(2026, 10, 18),
            target_volume: VolumeKm(30.0),
            previous_volume: VolumeKm(28.0),
            step_pct: 0.071,
            projected_acwr: 0.95,
            sessions: Vec::new(),
            is_deload: false,
        };
        let mk = |date: Date, kind: SessionKind, km: f64| PlannedSession {
            id: crate::ids::PlannedSessionId::new(),
            date,
            start: crate::TimeOfDay::from_hms_opt(7, 0, 0).unwrap(),
            kind,
            title: String::new(),
            intent: String::new(),
            workout: crate::workout::StructuredWorkout::default(),
            target_volume: VolumeKm(km),
            target_duration: crate::units::DurationSecs::ZERO,
            target_pace: None,
            rpe_target: None,
            quality: kind.is_quality(),
            external_id: None,
        };
        week.sessions = vec![
            mk(date(2026, 10, 12), SessionKind::Easy, 8.0),
            mk(date(2026, 10, 18), SessionKind::LongRun, 14.0),
            mk(date(2026, 10, 14), SessionKind::Rest, 0.0),
        ];
        assert_eq!(week.actual_volume(), VolumeKm(22.0));
        assert!((week.long_run_share() - 14.0 / 22.0).abs() < 0.001);
        assert_eq!(week.quality_count(), 0);
    }
}
