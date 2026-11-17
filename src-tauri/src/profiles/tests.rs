use super::*;

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        secure_store::reset_test_store();
        let path =
            std::env::temp_dir().join(format!("runalytics-profiles-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
        secure_store::reset_test_store();
    }
}

fn seed(state: &AppState, name: &str) -> serde_json::Value {
    let request = serde_json::from_value(serde_json::json!({
        "distance": "21K", "targetDate": (chrono::Utc::now().date_naive() + chrono::Duration::days(70)).to_string(),
        "thresholdSecKm": 280, "weeklyFrequency": 4, "longrunDay": 7, "increase": "moderate", "name": name,
    })).unwrap();
    let (plan, workouts) = crate::plan::engine::generate(&request, 30.0).unwrap();
    state.store.insert_plan(&plan).unwrap();
    state.store.insert_workouts(&workouts).unwrap();
    state.store.conn.lock().unwrap().execute("INSERT INTO activities(provider,provider_record_id,sport,start_ts,duration_s,distance_m) VALUES('coros',?1,'run',1790000000,1800,5000)", [name]).unwrap();
    let mut settings = default_settings();
    settings.theme = Some(name.to_string());
    save_json(&state.data_dir.join("settings.json"), &settings).unwrap();
    *state.settings.lock().unwrap() = settings;
    serde_json::to_value(state.store.current_plan().unwrap()).unwrap()
}

#[test]
fn different_coros_ids_restore_separate_plans_settings_and_history_after_restart() {
    let directory = Directory::new();
    let manager = ProfileManager::open(directory.0.clone()).unwrap();
    assert!(manager.session().unwrap().profile_key.is_none());
    manager.prepare_profile("account-a", false).unwrap();
    let a = profile_key("account-a");
    manager.select(Some(&a)).unwrap();
    let saved = {
        let state = manager.capture(Some(&a)).unwrap();
        seed(&state, "runner-a")
    };
    manager.prepare_profile("account-b", false).unwrap();
    let b = profile_key("account-b");
    manager.select(Some(&b)).unwrap();
    {
        let state = manager.capture(Some(&b)).unwrap();
        assert!(state.store.current_plan().unwrap().is_none());
        assert!(state.store.list_activities(-1).unwrap().is_empty());
        assert_eq!(state.settings.lock().unwrap().theme, None);
        seed(&state, "runner-b");
    }
    manager.select(None).unwrap();
    assert!(manager.capture(Some(&a)).is_err());
    assert!(manager
        .capture(Some(SIGNED_OUT))
        .unwrap()
        .store
        .current_plan()
        .unwrap()
        .is_none());
    drop(manager);
    let manager = ProfileManager::open(directory.0.clone()).unwrap();
    assert!(manager.session().unwrap().profile_key.is_none());
    manager.select(Some(&a)).unwrap();
    drop(manager);
    let manager = ProfileManager::open(directory.0.clone()).unwrap();
    let state = manager.capture(Some(&a)).unwrap();
    assert_eq!(
        serde_json::to_value(state.store.current_plan().unwrap()).unwrap(),
        saved
    );
    assert_eq!(state.store.list_activities(-1).unwrap().len(), 1);
    assert_eq!(
        state.settings.lock().unwrap().theme.as_deref(),
        Some("runner-a")
    );
    assert!(manager.capture(Some(&b)).is_err());
}

#[tokio::test]
async fn third_party_secrets_and_plan_channels_are_scoped_to_the_coros_profile() {
    let directory = Directory::new();
    let manager = ProfileManager::open(directory.0.clone()).unwrap();
    let a = profile_key("account-a");
    let b = profile_key("account-b");
    manager.prepare_profile("account-a", false).unwrap();
    manager.prepare_profile("account-b", false).unwrap();
    secure_store::entry(
        secure_store::GITHUB_SERVICE,
        secure_store::GITHUB_TOKEN_USER,
    )
    .unwrap()
    .set_password("unowned-legacy-token")
    .unwrap();
    secure_store::entry_scoped(
        Some(&a),
        secure_store::GITHUB_SERVICE,
        secure_store::GITHUB_TOKEN_USER,
    )
    .unwrap()
    .set_password("synthetic-a-token")
    .unwrap();
    secure_store::entry_scoped(Some(&a), "com.fradone.runalytics.strava", "credentials")
        .unwrap()
        .set_password(r#"{"client_id":"123","client_secret":"synthetic-a-secret","tokens":null}"#)
        .unwrap();
    manager.select(Some(&b)).unwrap();
    {
        let state = manager.capture(Some(&b)).unwrap();
        assert!(crate::plan_sync::stored_token(&state).is_none());
        assert!(crate::strava::transfer_credentials(&state.store)
            .unwrap()
            .is_none());
    }
    manager.select(Some(&a)).unwrap();
    let state = manager.capture(Some(&a)).unwrap();
    let calendar = crate::calendar_publish::Publishing {
        upload_url: "https://calendar.example.test/a.ics".into(),
        subscription_url: "https://calendar.example.test/a.ics".into(),
        ..Default::default()
    };
    crate::calendar_publish::configure(&state, calendar, Some("synthetic-a-password".into()))
        .await
        .unwrap();
    let exported = crate::calendar_publish::export_connection_settings(state.clone())
        .await
        .unwrap();
    assert_eq!(exported["calendar"]["password"], "synthetic-a-password");
    assert_eq!(exported["githubToken"], "synthetic-a-token");
    drop(state);
    manager.select(Some(&b)).unwrap();
    let exported =
        crate::calendar_publish::export_connection_settings(manager.capture(Some(&b)).unwrap())
            .await
            .unwrap();
    assert!(exported["calendar"]["password"].is_null());
    assert!(exported["githubToken"].is_null());
    assert!(exported["strava"].is_null());
    manager.select(Some(&a)).unwrap();
    let state = manager.capture(Some(&a)).unwrap();
    assert_eq!(
        crate::calendar_publish::load(&state.data_dir)
            .unwrap()
            .upload_url,
        "https://calendar.example.test/a.ics"
    );
    assert_eq!(
        crate::plan_sync::stored_token(&state).as_deref(),
        Some("synthetic-a-token")
    );
    assert_eq!(
        crate::strava::transfer_credentials(&state.store)
            .unwrap()
            .unwrap()["clientId"],
        "123"
    );
}

#[tokio::test]
async fn switching_waits_for_pinned_work_and_rejects_stale_account_requests() {
    let directory = Directory::new();
    let manager = Arc::new(ProfileManager::open(directory.0.clone()).unwrap());
    manager.prepare_profile("account-a", false).unwrap();
    manager.prepare_profile("account-b", false).unwrap();
    let a = profile_key("account-a");
    let b = profile_key("account-b");
    manager.select(Some(&a)).unwrap();
    let foreground = manager.capture(Some(&a)).unwrap();
    let background = foreground.owned();
    let switching = manager.clone();
    let next = b.clone();
    let task = tokio::spawn(async move {
        let _gate = switching.gate.write().await;
        switching.select(Some(&next)).unwrap();
    });
    tokio::task::yield_now().await;
    assert!(!task.is_finished());
    assert!(manager.capture(Some(&a)).is_err());
    drop(foreground);
    tokio::task::yield_now().await;
    assert!(!task.is_finished());
    assert_eq!(
        background.store.credential_scope.as_deref(),
        Some(a.as_str())
    );
    drop(background);
    task.await.unwrap();
    assert!(manager.capture(Some(&a)).is_err());
    assert_eq!(
        manager
            .capture(Some(&b))
            .unwrap()
            .store
            .credential_scope
            .as_deref(),
        Some(b.as_str())
    );
}

#[test]
fn an_existing_authenticated_installation_migrates_only_to_its_known_coros_id() {
    let directory = Directory::new();
    let legacy = Store::open(&directory.0.join("runalytics.sqlite")).unwrap();
    legacy.conn.lock().unwrap().execute("INSERT INTO activities(provider,provider_record_id,sport,start_ts,duration_s,distance_m) VALUES('coros','legacy-run','run',1790000000,1800,5000)", []).unwrap();
    secure_store::entry(COROS_SERVICE, "oauth")
        .unwrap()
        .set_password(
            r#"{"token_response":{"access_token":"e30.eyJzdWIiOiJsZWdhY3ktdXNlciJ9.sig"}}"#,
        )
        .unwrap();
    secure_store::entry(
        secure_store::GITHUB_SERVICE,
        secure_store::GITHUB_TOKEN_USER,
    )
    .unwrap()
    .set_password("synthetic-legacy-token")
    .unwrap();
    let manager = ProfileManager::open(directory.0.clone()).unwrap();
    let key = profile_key("legacy-user");
    assert_eq!(
        manager.session().unwrap().profile_key.as_deref(),
        Some(key.as_str())
    );
    assert!(manager.session().unwrap().legacy_profile);
    {
        let state = manager.capture(Some(&key)).unwrap();
        assert_eq!(state.store.list_activities(-1).unwrap().len(), 1);
        assert_eq!(
            crate::plan_sync::stored_token(&state).as_deref(),
            Some("synthetic-legacy-token")
        );
    }
    assert_eq!(legacy.list_activities(-1).unwrap().len(), 1);
    manager.prepare_profile("different-user", false).unwrap();
    let other = profile_key("different-user");
    manager.select(Some(&other)).unwrap();
    let state = manager.capture(Some(&other)).unwrap();
    assert!(state.store.list_activities(-1).unwrap().is_empty());
    assert!(crate::plan_sync::stored_token(&state).is_none());
}

#[test]
fn unowned_legacy_data_requires_confirmation_and_never_overwrites_an_existing_profile() {
    let directory = Directory::new();
    let legacy = Store::open(&directory.0.join("runalytics.sqlite")).unwrap();
    legacy.conn.lock().unwrap().execute("INSERT INTO activities(provider,provider_record_id,sport,start_ts,duration_s,distance_m) VALUES('coros','unowned','run',1790000000,1800,5000)", []).unwrap();
    let manager = ProfileManager::open(directory.0.clone()).unwrap();
    manager.prepare_profile("account-a", false).unwrap();
    let a = profile_key("account-a");
    manager.select(Some(&a)).unwrap();
    assert!(manager.session().unwrap().legacy_available);
    assert!(manager
        .capture(Some(&a))
        .unwrap()
        .store
        .list_activities(-1)
        .unwrap()
        .is_empty());
    assert!(manager.import_legacy(&profile_key("account-b")).is_err());
    let saved_secret =
        secure_store::entry_scoped(Some(&a), "com.fradone.runalytics.strava", "credentials")
            .unwrap();
    saved_secret
        .set_password(
            r#"{"client_id":"123","client_secret":"synthetic-existing-secret","tokens":null}"#,
        )
        .unwrap();
    assert!(!manager.session().unwrap().legacy_available);
    assert!(manager.import_legacy(&a).is_err());
    assert!(saved_secret
        .get_password()
        .unwrap()
        .contains("synthetic-existing-secret"));
    saved_secret.delete_credential().unwrap();
    manager.prepare_profile("populated-account", false).unwrap();
    let populated = profile_key("populated-account");
    manager.select(Some(&populated)).unwrap();
    for (service, user, value) in [
        (
            "com.fradone.runalytics.strava",
            "credentials",
            r#"{"client_id":"456","client_secret":"synthetic-new-secret","tokens":null}"#,
        ),
        (
            secure_store::GITHUB_SERVICE,
            secure_store::GITHUB_TOKEN_USER,
            "synthetic-new-token",
        ),
    ] {
        let entry = secure_store::entry_scoped(Some(&populated), service, user).unwrap();
        entry.set_password(value).unwrap();
        assert!(!manager.session().unwrap().legacy_available);
        assert!(manager.import_legacy(&populated).is_err());
        assert_eq!(entry.get_password().unwrap(), value);
        entry.delete_credential().unwrap();
    }
    let saved = seed(
        &manager.capture(Some(&populated)).unwrap(),
        "existing-runner",
    );
    assert!(manager.import_legacy(&populated).is_err());
    assert_eq!(
        serde_json::to_value(
            manager
                .capture(Some(&populated))
                .unwrap()
                .store
                .current_plan()
                .unwrap()
        )
        .unwrap(),
        saved
    );
    manager.select(Some(&a)).unwrap();
    manager.import_legacy(&a).unwrap();
    assert!(manager.session().unwrap().legacy_profile);
    assert_eq!(
        manager
            .capture(Some(&a))
            .unwrap()
            .store
            .list_activities(-1)
            .unwrap()
            .len(),
        1
    );
    std::fs::remove_file(directory.0.join("legacy-profile-owner.json")).unwrap();
    assert!(
        !manager.legacy_unclaimed(),
        "Installed profile metadata must preserve ownership after an interrupted marker write"
    );
    manager.prepare_profile("account-b", false).unwrap();
    manager.select(Some(&profile_key("account-b"))).unwrap();
    assert!(!manager.session().unwrap().legacy_available);
    assert!(manager.import_legacy(&profile_key("account-b")).is_err());
}
