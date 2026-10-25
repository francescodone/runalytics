use std::path::Path;
use std::time::Duration;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::calendar::{self, CalendarExport};
use crate::error::{AppError, Result};
use crate::state::AppState;

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Publishing {
    pub enabled: bool,
    pub upload_url: String,
    pub subscription_url: String,
    pub username: String,
    pub has_password: bool,
    pub last_published: Option<String>,
    pub last_error: Option<String>,
}

pub fn load(directory: &Path) -> Result<Publishing> {
    match std::fs::read(directory.join("calendar-publishing.json")) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Publishing::default()),
        Err(error) => Err(error.into()),
    }
}

fn save(directory: &Path, config: &Publishing) -> Result<()> {
    let temporary = directory.join("calendar-publishing.json.tmp");
    std::fs::write(&temporary, serde_json::to_vec_pretty(config)?)?;
    std::fs::rename(temporary, directory.join("calendar-publishing.json"))?;
    Ok(())
}

fn validate_url(value: &str) -> Result<()> {
    let url = reqwest::Url::parse(value)
        .map_err(|_| AppError::Other("Enter a valid HTTPS calendar URL".into()))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(AppError::Other(
            "Calendar URLs must use HTTPS, without embedded credentials or fragments".into(),
        ));
    }
    Ok(())
}

fn credential(config: &Publishing) -> Result<keyring_core::Entry> {
    crate::secure_store::entry("com.fradone.runalytics.calendar", &config.upload_url)
        .map_err(|_| AppError::Other("Calendar Keychain entry unavailable".into()))
}

pub async fn configure(
    state: &AppState,
    mut config: Publishing,
    password: Option<String>,
) -> Result<Publishing> {
    let _guard = state.calendar_lock.lock().await;
    config.upload_url = config.upload_url.trim().to_string();
    config.subscription_url = config.subscription_url.trim().to_string();
    config.username = config.username.trim().to_string();
    if config.enabled || !config.upload_url.is_empty() || !config.subscription_url.is_empty() {
        validate_url(&config.upload_url)?;
        validate_url(&config.subscription_url)?;
    }
    let previous = load(&state.data_dir)?;
    let same_target =
        previous.upload_url == config.upload_url && previous.username == config.username;
    config.has_password = same_target && previous.has_password;
    config.last_published = if same_target && previous.subscription_url == config.subscription_url {
        previous.last_published
    } else {
        None
    };
    config.last_error = None;
    if let Some(password) = password {
        let entry = credential(&config)?;
        if password.is_empty() {
            match entry.delete_credential() {
                Ok(()) | Err(keyring_core::Error::NoEntry) => {}
                Err(_) => {
                    return Err(AppError::Other(
                        "Could not clear calendar credential".into(),
                    ))
                }
            }
            config.has_password = false;
        } else {
            entry.set_password(&password).map_err(|_| {
                AppError::Other("Could not save calendar credential in Keychain".into())
            })?;
            config.has_password = true;
        }
    }
    save(&state.data_dir, &config)?;
    refresh_locked(state).await?;
    load(&state.data_dir)
}

async fn upload(
    client: &reqwest::Client,
    config: &Publishing,
    password: Option<String>,
    body: String,
) -> Result<()> {
    if let Some(file) = gist_file(config) {
        return upload_gist(client, config, password, &file, &body).await;
    }
    let mut request = client
        .put(&config.upload_url)
        .header("Content-Type", "text/calendar; charset=utf-8")
        .header("Cache-Control", "no-cache")
        .body(body.clone());
    if !config.username.is_empty() {
        request = request.basic_auth(&config.username, password);
    } else if let Some(token) = password {
        request = request.bearer_auth(token);
    }
    let response = request
        .send()
        .await
        .map_err(|_| AppError::Other("Calendar upload failed: network or TLS error".into()))?;
    if !response.status().is_success() {
        return Err(AppError::Other(format!(
            "Calendar upload failed (HTTP {})",
            response.status().as_u16()
        )));
    }
    let response = client
        .get(&config.subscription_url)
        .header("Cache-Control", "no-cache")
        .send()
        .await
        .map_err(|_| {
            AppError::Other(
                "Upload succeeded, but the subscription URL could not be reached".into(),
            )
        })?;
    if !response.status().is_success() {
        return Err(AppError::Other(format!(
            "Upload succeeded, but the subscription URL returned HTTP {}",
            response.status().as_u16()
        )));
    }
    let length = response.content_length();
    if length.is_some_and(|length| length > body.len() as u64 + 1024) {
        return Err(AppError::Other(
            "Subscription URL does not serve the uploaded calendar".into(),
        ));
    }
    let mut response = response;
    let mut downloaded = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| AppError::Other("Could not read subscription feed".into()))?
    {
        downloaded.extend_from_slice(&chunk);
        if downloaded.len() > body.len() + 1024 {
            break;
        }
    }
    if downloaded != body.as_bytes() {
        return Err(AppError::Other(
            "Upload succeeded, but the subscription URL serves different or cached content".into(),
        ));
    }
    Ok(())
}

/// Gist file name when the upload URL is a GitHub gist API URL (`.../gists/<id>`, also GitHub Enterprise).
fn gist_file(config: &Publishing) -> Option<String> {
    let upload = reqwest::Url::parse(&config.upload_url).ok()?;
    let segments: Vec<_> = upload.path_segments()?.collect();
    if !matches!(segments.as_slice(), [.., "gists", id] if !id.is_empty()) {
        return None;
    }
    let file = reqwest::Url::parse(&config.subscription_url)
        .ok()
        .and_then(|url| url.path_segments()?.last().map(str::to_string))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "runalytics.ics".into());
    Some(file)
}

async fn upload_gist(
    client: &reqwest::Client,
    config: &Publishing,
    password: Option<String>,
    file: &str,
    body: &str,
) -> Result<()> {
    let token =
        password.ok_or_else(|| AppError::Other("Gist publishing needs a GitHub token".into()))?;
    let mut files = serde_json::Map::new();
    files.insert(file.into(), serde_json::json!({ "content": body }));
    let response = client
        .patch(&config.upload_url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "Runalytics")
        .bearer_auth(token)
        .json(&serde_json::json!({ "files": files }))
        .send()
        .await
        .map_err(|_| AppError::Other("Gist upload failed: network or TLS error".into()))?;
    if !response.status().is_success() {
        return Err(AppError::Other(format!(
            "Gist upload failed (HTTP {})",
            response.status().as_u16()
        )));
    }
    let gist: serde_json::Value = response
        .json()
        .await
        .map_err(|_| AppError::Other("Gist upload returned an unreadable response".into()))?;
    let stored = &gist["files"][file];
    if stored["truncated"] != true && stored["content"].as_str() != Some(body) {
        return Err(AppError::Other(
            "Gist upload succeeded, but GitHub stored different content".into(),
        ));
    }
    // The raw URL is CDN-cached for a few minutes, so only check that it is reachable.
    let status = client
        .get(&config.subscription_url)
        .send()
        .await
        .map_err(|_| {
            AppError::Other("Gist updated, but the subscription URL could not be reached".into())
        })?
        .status();
    if !status.is_success() {
        return Err(AppError::Other(format!(
            "Gist updated, but the subscription URL returned HTTP {}",
            status.as_u16()
        )));
    }
    Ok(())
}

/// Where the .ics lives, without regenerating or uploading it. The Settings
/// pane shows this path and the publish status on open; reading must never
/// trigger a publish — only a sync, a calendar edit, or the explicit
/// refresh button run `refresh`.
pub fn export_path(data_dir: &Path) -> String {
    CalendarExport::new(&data_dir.to_path_buf())
        .path
        .display()
        .to_string()
}

pub async fn refresh(state: &AppState) -> Result<String> {
    let _guard = state.calendar_lock.lock().await;
    refresh_locked(state).await
}

async fn refresh_locked(state: &AppState) -> Result<String> {
    calendar::reconcile(&state.store)?;
    // Freeze each new run's load and injury risk before rendering, so every
    // banked session's description carries the numbers the body was carrying
    // when it landed.
    let thresholds = state.settings.lock().unwrap().thresholds();
    calendar::snapshot_scores(&state.store, &thresholds)?;
    let path = CalendarExport::new(&state.data_dir).regenerate(&state.store)?;
    let mut config = load(&state.data_dir)?;
    if config.enabled {
        let result = async {
            validate_url(&config.upload_url)?;
            validate_url(&config.subscription_url)?;
            let password = if config.has_password {
                Some(credential(&config)?.get_password().map_err(|_| {
                    AppError::Other("Calendar credential unavailable; enter it again".into())
                })?)
            } else {
                None
            };
            let client = reqwest::Client::builder()
                .https_only(true)
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|_| AppError::Other("Could not initialize calendar publisher".into()))?;
            upload(&client, &config, password, std::fs::read_to_string(&path)?).await
        }
        .await;
        match result {
            Ok(()) => {
                config.last_published = Some(Utc::now().to_rfc3339());
                config.last_error = None;
            }
            Err(error) => config.last_error = Some(error.to_string()),
        }
        save(&state.data_dir, &config)?;
    }
    Ok(path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publishing_requires_https_and_separate_credentials() {
        assert!(validate_url("https://example.com/private/calendar.ics").is_ok());
        for url in [
            "http://example.com/calendar.ics",
            "file:///calendar.ics",
            "https://user:password@example.com/calendar.ics",
            "https://example.com/calendar.ics#fragment",
            "invalid",
        ] {
            assert!(validate_url(url).is_err());
        }
    }

    #[tokio::test]
    async fn publication_failure_preserves_local_calendar_and_records_retry_status() {
        let directory =
            std::env::temp_dir().join(format!("runalytics-calendar-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let state = AppState {
            store: crate::store::Store::open(Path::new(":memory:")).unwrap(),
            settings: std::sync::Mutex::new(Default::default()),
            providers: Vec::new(),
            data_dir: directory.clone(),
            syncing: std::sync::atomic::AtomicBool::new(false),
            sync_lock: tokio::sync::Mutex::new(()),
            calendar_lock: tokio::sync::Mutex::new(()),
        };
        save(
            &directory,
            &Publishing {
                enabled: true,
                upload_url: "http://invalid.example/feed.ics".into(),
                subscription_url: "https://invalid.example/feed.ics".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let exported = refresh(&state).await.unwrap();
        assert!(std::fs::read_to_string(exported)
            .unwrap()
            .contains("END:VCALENDAR"));
        let config = load(&directory).unwrap();
        assert!(config.last_error.unwrap().contains("HTTPS"));
        assert!(config.last_published.is_none());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn upload_overwrites_and_verifies_public_feed_without_upload_credentials() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let body = "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n".to_string();
        let expected = body.clone();
        let server = tokio::spawn(async move {
            for method in ["PUT", "GET"] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                loop {
                    let mut buffer = [0; 2048];
                    let count = socket.read(&mut buffer).await.unwrap();
                    assert!(count > 0);
                    bytes.extend_from_slice(&buffer[..count]);
                    if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                        let length: usize = headers
                            .lines()
                            .find_map(|line| line.strip_prefix("content-length: "))
                            .unwrap_or("0")
                            .parse()
                            .unwrap();
                        if bytes.len() >= end + 4 + length {
                            break;
                        }
                    }
                }
                let request = String::from_utf8(bytes).unwrap();
                assert!(request.starts_with(method));
                if method == "PUT" {
                    assert!(request.to_lowercase().contains("authorization: basic "));
                    assert!(request.ends_with(&expected));
                } else {
                    assert!(!request.to_lowercase().contains("authorization:"));
                }
                let response_body = if method == "GET" {
                    expected.as_str()
                } else {
                    ""
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });
        let config = Publishing {
            upload_url: format!("http://{address}/upload.ics"),
            subscription_url: format!("http://{address}/feed.ics"),
            username: "runner".into(),
            ..Default::default()
        };
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        upload(&client, &config, Some("test-password".into()), body)
            .await
            .unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn gist_upload_patches_file_with_token_and_checks_raw_url() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let body = "BEGIN:VCALENDAR\r\nX-NOTE:a, b\r\nEND:VCALENDAR\r\n".to_string();
        let expected = body.clone();
        let server = tokio::spawn(async move {
            for method in ["PATCH", "GET"] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                loop {
                    let mut buffer = [0; 2048];
                    let count = socket.read(&mut buffer).await.unwrap();
                    assert!(count > 0);
                    bytes.extend_from_slice(&buffer[..count]);
                    if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                        let length: usize = headers
                            .lines()
                            .find_map(|line| line.strip_prefix("content-length: "))
                            .unwrap_or("0")
                            .parse()
                            .unwrap();
                        if bytes.len() >= end + 4 + length {
                            break;
                        }
                    }
                }
                let request = String::from_utf8(bytes).unwrap();
                assert!(request.starts_with(&format!("{method} ")));
                let response_body = if method == "PATCH" {
                    assert!(request.starts_with("PATCH /gists/abc123 "));
                    assert!(request
                        .to_lowercase()
                        .contains("authorization: bearer gh-token"));
                    let json: serde_json::Value =
                        serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
                    assert_eq!(json["files"]["plan.ics"]["content"], expected.as_str());
                    serde_json::json!({ "files": { "plan.ics": { "content": expected, "truncated": false } } })
                        .to_string()
                } else {
                    assert!(request.starts_with("GET /runner/abc123/raw/plan.ics "));
                    assert!(!request.to_lowercase().contains("authorization:"));
                    "stale cached copy".into()
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });
        let config = Publishing {
            upload_url: format!("http://{address}/gists/abc123"),
            subscription_url: format!("http://{address}/runner/abc123/raw/plan.ics"),
            ..Default::default()
        };
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        assert!(upload(&client, &config, None, body.clone()).await.is_err());
        upload(&client, &config, Some("gh-token".into()), body)
            .await
            .unwrap();
        server.await.unwrap();
    }
}
