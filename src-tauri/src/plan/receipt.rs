//! The plan receipt: what the plan was built from, said back before it is shown.
//!
//! A generated plan arrives as a wall of sessions, and the runner has no way to
//! tell whether the machine listened. The fix is not a settings screen — it is
//! echoing the inputs back in second person, in the order they actually shaped
//! the answer, so a wrong number is visible at a glance and the plan reads as
//! something that was asked for rather than something that appeared.
//!
//! This module is rendering only. Every figure comes from the stored plan and
//! its generated workouts, or from the engine's own helpers, so the receipt
//! cannot claim a shape the generator did not produce. Where a number is a
//! consequence of the plan rather than an input to it (the peak week, the
//! recovery weeks, the taper), it is measured from the workouts the engine
//! wrote — the receipt describes the plan, it does not re-plan it.

use chrono::{Datelike, NaiveDate};

use crate::plan::engine;
use crate::types::*;

/// One line of the receipt: the input the runner gave, and what it caused.
///
/// Kept as a pair rather than one sentence so the UI can render the input as a
/// label and the consequence as prose, and so a screen reader gets the same
/// cause-then-effect order the page shows.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptLine {
    /// The decision, named the way the form names it.
    pub input: String,
    /// Second person, one sentence: what this input did to the plan.
    pub sentence: String,
}

/// The receipt for a plan, ordered by how much each input shaped it.
///
/// The goal and its date come first because they decide the length of
/// everything; the starting volume next because it sizes the ramp; threshold
/// pace after that because it sets every pace band but no session's distance.
/// The layout choices (which days, which weekday for the long run) are last —
/// real, but the least consequential part of the plan.
pub fn receipt(plan: &TrainingPlan, workouts: &[PlanWorkout]) -> Vec<ReceiptLine> {
    let weeks = engine::weeks_of(plan);
    let down = engine::down_weeks(plan);
    let taper = engine::taper_weeks_for(plan.kind, plan.distance);
    let first_week = workouts
        .iter()
        .map(|w| w.planned_date)
        .min()
        .map(week_monday)
        .unwrap_or(plan.created_at.date_naive());
    let peak = peak_week(workouts);
    let long_run = workouts
        .iter()
        .filter(|w| w.kind == WorkoutKind::LongRun)
        .filter_map(|w| w.distance_m)
        .fold(0.0_f64, f64::max);

    let mut lines = Vec::new();

    // 1. The goal and the date — they decide the length of the plan.
    lines.push(ReceiptLine {
        input: goal_input(plan),
        sentence: match plan.kind {
            PlanKind::Race => format!(
                "We have {} weeks to your {} on {}, starting the week of {}.",
                weeks,
                plan.distance.label(),
                fmt_date(plan.target_date),
                fmt_date(first_week)
            ),
            _ => format!(
                "We have laid out {} weeks from {}, ending {}.",
                weeks,
                fmt_date(first_week),
                fmt_date(plan.target_date)
            ),
        },
    });

    // 2. Starting volume — it sizes the ramp more than any other number.
    lines.push(match plan.baseline_weekly_km {
        Some(baseline) => ReceiptLine {
            input: format!("{:.0} km a week", baseline),
            sentence: match plan.kind {
                PlanKind::Maintenance => format!(
                    "Week 1 opens at {:.0} km and the block holds that volume — the point here is to keep, not to build.",
                    baseline
                ),
                _ => match peak {
                    Some(peak_km) if peak_km > baseline + 0.5 => format!(
                        "We start at {:.0} km a week and build to {:.0} km, so the first week is the easiest week of the plan.",
                        baseline, peak_km
                    ),
                    _ => format!(
                        "We start at {:.0} km a week, which is already where this plan runs — there is nothing to build from, and we won't pretend otherwise.",
                        baseline
                    ),
                },
            },
        },
        // Plans stored before the baseline was recorded: the receipt says so
        // rather than inventing a number the database does not have.
        None => ReceiptLine {
            input: "Weekly volume".into(),
            sentence: "This plan was created before we started recording the volume it opened with, so the first week's distance is the number to judge it against.".into(),
        },
    });

    // 3. Threshold pace — it sets every pace band in the plan.
    lines.push(ReceiptLine {
        input: format!("{} per km", fmt_pace(plan.threshold_sec_km as i64)),
        sentence: format!(
            "Every pace in the plan is cut from this one number: {} is your threshold, so tempo sits around it and easy days sit well back from it.",
            fmt_pace(plan.threshold_sec_km as i64)
        ),
    });

    // 4. How many days a week.
    lines.push(ReceiptLine {
        input: format!("{} runs a week", plan.weekly_frequency),
        sentence: format!(
            "That is {} sessions across the block, with the easy days spread so no two hard days sit back to back.",
            weeks * plan.weekly_frequency as usize,
        ),
    });

    // 5. How fast it grows.
    lines.push(ReceiptLine {
        input: increase_label(plan.increase).into(),
        sentence: match plan.kind {
            PlanKind::Maintenance => "A maintenance block does not ramp, so this choice changes nothing here.".into(),
            other => format!(
                "Volume grows by at most {:.0}% a week{}.",
                (plan.increase.weekly_factor() - 1.0) * 100.0,
                if other == PlanKind::Race {
                    " — the rest of the build comes from the sessions getting harder, not the weeks getting much longer"
                } else {
                    ""
                }
            ),
        },
    });

    // 6. Where the long run lands.
    lines.push(ReceiptLine {
        input: weekday(plan.longrun_day).into(),
        sentence: match long_run {
            v if v > 0.0 => format!(
                "The long run is a {} — {} at its furthest, and it stays on that day all the way through.",
                weekday(plan.longrun_day),
                fmt_km(v)
            ),
            // A plan whose long run fell on the race day itself never got one.
            _ => "The long run sits on this day whenever the schedule allows it.".into(),
        },
    });

    // 7. The structure the runner did not choose but should know about.
    let mut structure = Vec::new();
    if !down.is_empty() {
        let verb = if down.len() == 1 { "cuts" } else { "cut" };
        structure.push(format!(
            "{} recovery week{} {} the volume back every fourth week",
            down.len(),
            if down.len() == 1 { "" } else { "s" },
            verb
        ));
    }
    if taper > 0 {
        structure.push(if taper == 1 {
            "the last week before the race tapers down".to_string()
        } else {
            format!("the last {} weeks before the race taper down", taper)
        });
    }
    if plan.kind.has_race() {
        structure.push("race day itself is a shakeout, not a session".into());
    }
    if !structure.is_empty() {
        lines.push(ReceiptLine {
            input: "The shape of the block".into(),
            sentence: format!("{}.", join_and(&structure)),
        });
    }

    // 8. A pacing adjustment the runner has already accepted is part of the
    //    plan's basis now, and it is the one thing that changed after creation.
    if plan.quality_offset_sec_km != 0.0 {
        let sharpen = plan.quality_offset_sec_km < 0.0;
        lines.push(ReceiptLine {
            input: "Pacing adjustment".into(),
            sentence: format!(
                "Quality paces run {} by {} s/km from {} — you accepted that after your recent tempo and interval reps.",
                if sharpen { "sharper" } else { "easier" },
                plan.quality_offset_sec_km.abs().round(),
                plan
                    .quality_offset_from
                    .map(fmt_date)
                    .unwrap_or_else(|| "the day you accepted it".into())
            ),
        });
    }

    lines
}

/// Join clauses the way prose does: "a", "a and b", "a, b and c".
fn join_and(parts: &[String]) -> String {
    match parts.len() {
        0 => String::new(),
        1 => parts[0].clone(),
        _ => {
            let (last, rest) = parts.split_last().unwrap();
            format!("{} and {}", rest.join(", "), last)
        }
    }
}

fn goal_input(plan: &TrainingPlan) -> String {
    match plan.kind {
        PlanKind::Race => format!("{} on {}", plan.distance.label(), fmt_date(plan.target_date)),
        PlanKind::Improvement => format!("Improvement over {}", plan.distance.label()),
        PlanKind::Maintenance => "Maintenance".into(),
    }
}

/// The heaviest generated week, in km — measured from the plan rather than
/// forecast, so the receipt cannot promise a peak the generator did not write.
/// Returns `None` for a plan with no distance in it (a block of rest days).
fn peak_week(workouts: &[PlanWorkout]) -> Option<f64> {
    let mut by_week: std::collections::BTreeMap<NaiveDate, f64> = Default::default();
    for w in workouts {
        if w.kind == WorkoutKind::Rest {
            continue;
        }
        *by_week.entry(week_monday(w.planned_date)).or_default() +=
            w.distance_m.unwrap_or(0.0) / 1000.0;
    }
    by_week
        .values()
        .cloned()
        .fold(None, |acc: Option<f64>, v| Some(acc.map_or(v, |a| a.max(v))))
}

fn week_monday(d: NaiveDate) -> NaiveDate {
    d - chrono::Duration::days(i64::from(d.weekday().num_days_from_monday()))
}

fn increase_label(m: IncreaseMethod) -> &'static str {
    match m {
        IncreaseMethod::Conservative => "Conservative",
        IncreaseMethod::Moderate => "Moderate",
        IncreaseMethod::Ambitious => "Ambitious",
    }
}

fn weekday(iso: u32) -> &'static str {
    match iso {
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        6 => "Saturday",
        _ => "Sunday",
    }
}

/// "Dec 15" — short enough to sit inside a sentence. `%e` is space-padded, so
/// single-digit days are trimmed rather than reading "Dec  5".
fn fmt_date(d: NaiveDate) -> String {
    format!("{}", d.format("%e %b")).trim().to_string()
}

fn fmt_pace(seconds: i64) -> String {
    let s = seconds.abs();
    format!("{}:{:02}", s / 60, s % 60)
}

fn fmt_km(m: f64) -> String {
    format!("{:.0} km", m / 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::engine;
    use chrono::Utc;

    fn build(req: &PlanRequest) -> (TrainingPlan, Vec<PlanWorkout>) {
        let (plan, workouts) = engine::generate(req, 40.0).unwrap();
        (plan, workouts)
    }

    fn request(days_out: i64) -> PlanRequest {
        let today = Utc::now().date_naive();
        PlanRequest {
            distance: RaceDistance::K10,
            kind: PlanKind::Race,
            target_date: today + chrono::Duration::days(days_out),
            threshold_sec_km: 285.0,
            weekly_frequency: 4,
            longrun_day: 6,
            increase: IncreaseMethod::Moderate,
            current_weekly_km: Some(40.0),
        }
    }

    /// The receipt is the runner's chance to spot a wrong number, so every
    /// input they gave has to appear in it.
    #[test]
    fn every_input_the_runner_gave_is_echoed_back() {
        let (plan, workouts) = build(&request(70));
        let lines = receipt(&plan, &workouts);
        let all: String = lines
            .iter()
            .map(|l| format!("{} {}", l.input, l.sentence))
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(all.contains("10K"), "the goal: {all}");
        assert!(all.contains("4:45"), "the threshold: {all}");
        assert!(all.contains("4 runs a week"), "the frequency: {all}");
        assert!(all.contains("Moderate"), "the increase method: {all}");
        assert!(all.contains("Saturday"), "the long-run day: {all}");
        assert!(all.contains("40 km"), "the starting volume: {all}");
    }

    /// The order is the point: the goal decides the length of everything, so a
    /// reader scanning the first line learns the most from it.
    #[test]
    fn the_biggest_influence_leads() {
        let (plan, workouts) = build(&request(70));
        let lines = receipt(&plan, &workouts);
        assert!(lines[0].input.contains("10K"), "goal first: {lines:?}");
        assert!(lines[1].input.contains("km a week"), "volume second");
        assert!(lines[2].input.contains("4:45"), "threshold third");
    }

    /// The peak is measured from the generated workouts, so the receipt cannot
    /// promise a build the plan does not contain.
    #[test]
    fn the_peak_it_quotes_is_the_peak_the_plan_contains() {
        let (plan, workouts) = build(&request(70));
        let measured = peak_week(&workouts).expect("a plan has weeks");
        let lines = receipt(&plan, &workouts);
        let quoted = lines
            .iter()
            .find(|l| l.sentence.contains("build to"))
            .expect("a building plan says where it builds to");
        let number: String = quoted
            .sentence
            .split("build to ")
            .nth(1)
            .unwrap()
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        assert_eq!(number.parse::<f64>().unwrap(), measured.round());
    }

    /// A maintenance block holds volume; a receipt that promised a build there
    /// would describe a plan the generator refused to make.
    #[test]
    fn a_maintenance_block_does_not_claim_a_build() {
        let req = PlanRequest {
            kind: PlanKind::Maintenance,
            ..request(60)
        };
        let (plan, workouts) = build(&req);
        let all: String = receipt(&plan, &workouts)
            .iter()
            .map(|l| l.sentence.clone())
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(!all.contains("build to"), "no ramp promised: {all}");
        assert!(all.contains("hold"), "it says what it does instead: {all}");
    }

    /// Recovery weeks are the plan's own structure and the thing runners
    /// misread as a bug, so the receipt has to name them.
    #[test]
    fn the_structure_the_runner_did_not_choose_is_named() {
        let (plan, workouts) = build(&request(70));
        let down = engine::down_weeks(&plan);
        let all: String = receipt(&plan, &workouts)
            .iter()
            .map(|l| l.sentence.clone())
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(!down.is_empty(), "a 10-week build has a down week");
        assert!(
            all.contains("recovery week"),
            "and the receipt says so: {all}"
        );
        assert!(all.contains("taper"), "a race build tapers: {all}");
    }

    /// A plan whose baseline was never recorded must not invent one.
    #[test]
    fn a_missing_baseline_is_admitted_not_invented() {
        let (plan, workouts) = build(&request(70));
        let blind = TrainingPlan {
            baseline_weekly_km: None,
            ..plan.clone()
        };
        let lines = receipt(&blind, &workouts);
        assert!(lines
            .iter()
            .any(|l| l.sentence.contains("before we started recording")));
        assert!(!lines
            .iter()
            .any(|l| l.input.ends_with("km a week") && l.input.starts_with('0')));
    }

    /// An accepted pacing adjustment changed the plan after it was built, and
    /// the receipt is where that belongs.
    #[test]
    fn an_accepted_pacing_adjustment_is_part_of_the_basis() {
        let (plan, workouts) = build(&request(70));
        let adjusted = TrainingPlan {
            quality_offset_sec_km: -5.0,
            quality_offset_from: Some(plan.target_date - chrono::Duration::days(20)),
            ..plan.clone()
        };
        let before = receipt(&plan, &workouts);
        let after = receipt(&adjusted, &workouts);
        assert_eq!(before.len() + 1, after.len(), "one extra line, not a rewrite");
        assert!(after
            .last()
            .unwrap()
            .sentence
            .contains("sharper by 5 s/km"));
    }
}
