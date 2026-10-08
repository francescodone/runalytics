//! Ingestion pipeline tests against a fake provider and a real in-memory
//! database.
//!
//! The fake records every window it was asked for, which is how these tests
//! check the two properties the pipeline exists for: a resumed sync starts at
//! `cursor - overlap`, and a failed sync leaves the cursor exactly where it
//! was.

use async_trait::async_trait;
use chrono::{Duration, NaiveDate};
use runalytics_core::{
    ActivitySummary, Date, DurationSecs, FitnessAssessment, HeartRate, Provider, SleepDay,
    Timestamp, VolumeKm,
};
use runalytics_provider_core::{
    AccountInfo, ActivityDraft, Capability, FitnessDraft, HealthDraft, ProviderError, Result,
    SyncPolicy, WearableProvider, WorkoutPush, ingest_activities, ingest_all, ingest_health,
    register_account,
};
use runalytics_store::Db;
use runalytics_store::repos::{ActivityRepo, HealthRepo, ProviderAccountRepo, SyncRunRepo};

fn date(y: i32, m: u32, d: u32) -> Date {
    NaiveDate::from_ymd_opt(y, m, d).expect("test date")
}

fn ts() -> Timestamp {
    Timestamp::default()
}

/// A provider whose responses and failures are scripted per data class.
struct FakeProvider {
    capabilities: Capability,
    /// Windows each fetch was asked for, in call order.
    activity_windows: std::sync::Mutex<Vec<(Date, Date)>>,
    health_windows: std::sync::Mutex<Vec<(Date, Date)>>,
    /// Fail any activity window whose `from` is >= this date.
    fail_activities_from: Option<Date>,
    /// Activities to return, keyed by the window's start date.
    activities: Vec<ActivityDraft>,
    health: Vec<HealthDraft>,
}

impl FakeProvider {
    fn new() -> Self {
        Self {
            capabilities: Capability::read_only(),
            activity_windows: std::sync::Mutex::new(Vec::new()),
            health_windows: std::sync::Mutex::new(Vec::new()),
            fail_activities_from: None,
            activities: Vec::new(),
            health: Vec::new(),
        }
    }

    fn windows(which: &std::sync::Mutex<Vec<(Date, Date)>>) -> Vec<(Date, Date)> {
        which
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

fn draft_activity(id: &str, day: Date) -> ActivityDraft {
    ActivityDraft {
        provider_activity_id: id.into(),
        name: "Morning run".into(),
        started_at: day.and_hms_opt(7, 0, 0).expect("valid").and_utc(),
        local_date: day,
        summary: ActivitySummary {
            distance: VolumeKm::new(10.0),
            duration: DurationSecs::from_minutes(60),
            avg_pace: None,
            avg_hr: Some(HeartRate::new(150)),
            max_hr: Some(HeartRate::new(165)),
            elevation_gain: None,
            avg_cadence: None,
            training_load: None,
        },
        laps: Vec::new(),
        provider_intensity_label: Some("aerobic".into()),
    }
}

fn draft_health(day: Date) -> HealthDraft {
    HealthDraft {
        date: day,
        resting_hr: Some(HeartRate::new(52)),
        avg_stress: None,
        high_stress_minutes: None,
        sleep: Some(SleepDay {
            date: day,
            total: DurationSecs::from_minutes(420),
            deep: DurationSecs::from_minutes(80),
            light: DurationSecs::from_minutes(210),
            rem: DurationSecs::from_minutes(110),
            awake: DurationSecs::from_minutes(20),
            nap: DurationSecs::ZERO,
            score: None,
            lowest_hr: None,
            hrv: Some(48.0),
            respiratory_rate: None,
        }),
        steps: Some(9_000),
        provider_readiness: None,
        basal_energy: None,
    }
}

#[async_trait]
impl WearableProvider for FakeProvider {
    fn provider(&self) -> Provider {
        Provider::Coros
    }

    fn capabilities(&self) -> Capability {
        self.capabilities
    }

    async fn connect(&mut self) -> Result<AccountInfo> {
        Ok(AccountInfo {
            external_user_id: Some("42".into()),
            account_label: "athlete@example.com".into(),
            region: Some("eu".into()),
            capabilities: self.capabilities,
        })
    }

    fn account(&self) -> Option<&AccountInfo> {
        None
    }

    async fn fetch_activities(&self, from: Date, to: Date) -> Result<Vec<ActivityDraft>> {
        self.activity_windows
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((from, to));
        if let Some(bad) = self.fail_activities_from
            && from >= bad
        {
            return Err(ProviderError::Transport {
                provider: "fake".into(),
                source: anyhow::anyhow!("simulated outage"),
            });
        }
        Ok(self
            .activities
            .iter()
            .filter(|d| d.local_date >= from && d.local_date <= to)
            .cloned()
            .collect())
    }

    async fn fetch_health(&self, from: Date, to: Date) -> Result<Vec<HealthDraft>> {
        self.health_windows
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((from, to));
        Ok(self
            .health
            .iter()
            .filter(|d| d.date >= from && d.date <= to)
            .cloned()
            .collect())
    }

    async fn fetch_fitness(&self, from: Date, to: Date) -> Result<Vec<FitnessDraft>> {
        let all = vec![FitnessDraft {
            assessment: FitnessAssessment {
                date: to,
                vo2max: Some(52.0),
                running_level: None,
                threshold_pace: None,
                predicted_paces: Vec::new(),
            },
        }];
        Ok(all
            .into_iter()
            .filter(|d| d.assessment.date >= from && d.assessment.date <= to)
            .collect())
    }

    async fn push_workout(&self, _push: &WorkoutPush<'_>) -> Result<String> {
        Err(ProviderError::Unsupported {
            provider: "fake".into(),
            capability: "plans_write".into(),
        })
    }

    async fn delete_workout(&self, _external_id: &str) -> Result<()> {
        Err(ProviderError::Unsupported {
            provider: "fake".into(),
            capability: "plans_write".into(),
        })
    }
}

fn connected(db: &Db) -> runalytics_core::ProviderAccountId {
    let info = AccountInfo {
        external_user_id: Some("42".into()),
        account_label: "athlete@example.com".into(),
        region: Some("eu".into()),
        capabilities: Capability::read_only(),
    };
    register_account(db, Provider::Coros, &info).expect("register")
}

fn policy() -> SyncPolicy {
    SyncPolicy {
        overlap_days: 3,
        initial_backfill_days: 30,
        max_window_days: 10,
    }
}

#[tokio::test]
async fn first_sync_backfills_and_parks_the_cursor_one_day_back() {
    let db = Db::in_memory().expect("db");
    let account = connected(&db);
    let today = date(2026, 10, 8);

    let provider = FakeProvider::new();
    let report = ingest_activities(&db, account, &provider, today, ts(), &policy())
        .await
        .expect("sync");

    assert_eq!(report.records, 0);
    // The cursor must not sit on today: tonight's run and last night's sleep
    // are still being written.
    assert_eq!(report.cursor_after, Some(today - Duration::days(1)));
    assert_eq!(
        ProviderAccountRepo::cursor(&db, account, "activities")
            .expect("cursor")
            .unwrap(),
        today - Duration::days(1)
    );

    // The backfill was windowed, not one giant call, and the windows tile
    // the range without gaps or overlaps beyond the window boundary.
    let windows = FakeProvider::windows(&provider.activity_windows);
    assert!(windows.len() >= 3, "30 days / 10-day windows");
    assert_eq!(windows[0].0, today - Duration::days(30));
    assert_eq!(windows.last().unwrap().1, today);
    for pair in windows.windows(2) {
        assert_eq!(pair[0].1 + Duration::days(1), pair[1].0, "windows tile");
    }
}

#[tokio::test]
async fn resumed_sync_starts_at_cursor_minus_overlap() {
    let db = Db::in_memory().expect("db");
    let account = connected(&db);
    let today = date(2026, 10, 8);
    ProviderAccountRepo::set_cursor(&db, account, "activities", date(2026, 10, 1)).expect("cursor");

    let provider = FakeProvider::new();
    ingest_activities(&db, account, &provider, today, ts(), &policy())
        .await
        .expect("sync");

    let windows = FakeProvider::windows(&provider.activity_windows);
    assert_eq!(windows[0].0, date(2026, 9, 28), "cursor 10-01 minus 3");
}

#[tokio::test]
async fn a_failed_window_leaves_the_cursor_untouched() {
    let db = Db::in_memory().expect("db");
    let account = connected(&db);
    let today = date(2026, 10, 8);
    ProviderAccountRepo::set_cursor(&db, account, "activities", date(2026, 10, 1)).expect("cursor");

    let provider = FakeProvider {
        fail_activities_from: Some(date(2026, 10, 5)),
        ..FakeProvider::new()
    };
    let err = ingest_activities(&db, account, &provider, today, ts(), &policy())
        .await
        .expect_err("must fail");
    assert!(err.is_retryable());

    // The whole range re-fetches next time. Advancing the cursor past a window
    // that failed would silently delete those days from history.
    assert_eq!(
        ProviderAccountRepo::cursor(&db, account, "activities")
            .expect("cursor")
            .unwrap(),
        date(2026, 10, 1)
    );

    // And the failure is in the sync log the user will be asked about.
    let runs = SyncRunRepo::recent(&db, 10).expect("runs");
    let failed = runs.iter().find(|r| r.status == "error").expect("logged");
    assert_eq!(failed.data_class, "activities");
    assert!(
        failed
            .error
            .as_deref()
            .is_some_and(|e| e.contains("outage"))
    );
}

#[tokio::test]
async fn activities_land_idempotently_and_health_keeps_sleep() {
    let db = Db::in_memory().expect("db");
    let account = connected(&db);
    let today = date(2026, 10, 8);

    let mut provider = FakeProvider::new();
    provider.activities = vec![draft_activity("act-1", date(2026, 10, 6))];
    provider.health = vec![draft_health(date(2026, 10, 7))];

    let a = ingest_activities(&db, account, &provider, today, ts(), &policy())
        .await
        .expect("activities");
    let h = ingest_health(&db, account, &provider, today, &policy())
        .await
        .expect("health");
    assert_eq!(a.records, 1);
    assert_eq!(h.records, 1);

    // Re-syncing the same window must not duplicate: same derived id, one row.
    let again = ingest_activities(&db, account, &provider, today, ts(), &policy())
        .await
        .expect("resync");
    assert_eq!(again.records, 1, "re-fetched and upserted");

    let stored = ActivityRepo::list_range(&db, date(2026, 1, 1), today)
        .expect("range")
        .into_iter()
        .filter(|a| a.provider_activity_id == "act-1")
        .collect::<Vec<_>>();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].intensity, runalytics_core::Intensity::Aerobic);

    let day = HealthRepo::get(&db, date(2026, 10, 7))
        .expect("health")
        .expect("present");
    assert!(day.has_recovery_signal());
    assert_eq!(day.sleep.expect("sleep").hrv, Some(48.0));
}

#[tokio::test]
async fn ingest_all_follows_capabilities_and_survives_one_class_failing() {
    let db = Db::in_memory().expect("db");
    let account = connected(&db);
    let today = date(2026, 10, 8);

    let provider = FakeProvider {
        fail_activities_from: Some(date(2026, 1, 1)),
        ..FakeProvider::new()
    };
    let reports = ingest_all(&db, account, &provider, today, ts(), &policy()).await;

    // Health and fitness succeeded, activities failed, nothing threw.
    assert_eq!(reports.len(), 3, "health, activities, fitness");
    assert!(reports[0].is_ok(), "health first");
    assert!(reports[1].is_err(), "activities failed");
    assert!(reports[2].is_ok(), "fitness continued past the failure");

    // A read-only provider is never asked to write.
    assert!(!provider.capabilities().contains(Capability::PLANS_WRITE));
}
