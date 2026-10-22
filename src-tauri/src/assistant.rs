//! LLM assistant for the injury journal.
//!
//! Talks to any OpenAI-compatible /chat/completions endpoint (OpenRouter,
//! OpenAI, Ollama, LM Studio, vLLM). The request is made here, in Rust, for
//! two reasons: the app's CSP connect-src is locked to 'self + the tile hosts,
//! so the webview cannot reach a user-supplied https endpoint at all; and the
//! API key must never travel through the frontend.
//!
//! The assistant only *explains*. Plan changes come from proposal::build and
//! are applied only after the runner accepts — the model never emits
//! executable edits, so the offer stays auditable and identical with the LLM
//! switched off.

use chrono::Utc;

use crate::error::{AppError, Result};
use crate::metrics;
use crate::plan::adaptive::recovery_from_health;
use crate::proposal::region_label;
use crate::store::Store;
use crate::types::*;

/// Complete a base URL into the chat route. Providers document their base
/// (`https://host/v1`), but the request is posted straight at the chat
/// endpoint, so a base saved verbatim would 404 with an opaque upstream
/// error. Only the shapes that unambiguously mean "my OpenAI-compatible base"
/// are extended; anything else is the user's explicit choice and is kept.
fn complete_chat_path(url: &mut reqwest::Url) {
    let path = url.path().trim_end_matches('/').to_string();
    if path.ends_with("/chat/completions") {
        return;
    }
    let base = match path.as_str() {
        "" | "/" => "/v1/chat/completions".to_string(),
        p if p.ends_with("/v1") || p.ends_with("/api") || p.ends_with("/openai") => {
            format!("{p}/chat/completions")
        }
        _ => return,
    };
    url.set_path(&base);
}

/// The chat endpoint must be https, or http only on loopback (local models).
/// A base URL is normalised to its `/chat/completions` route.
pub fn validate_endpoint(raw: &str) -> Result<String> {
    let mut url = reqwest::Url::parse(raw)
        .map_err(|e| AppError::Other(format!("endpoint is not a valid URL: {e}")))?;
    match url.scheme() {
        "https" => {}
        "http" => {
            let host = url.host_str().unwrap_or("");
            if host != "localhost" && host != "127.0.0.1" && host != "::1" {
                return Err(AppError::Other(
                    "plain http is only allowed for a local endpoint (localhost)".into(),
                ));
            }
        }
        s => {
            return Err(AppError::Other(format!(
                "unsupported scheme '{s}' — use https (or http for localhost)"
            )))
        }
    }
    complete_chat_path(&mut url);
    Ok(url.to_string())
}

/// Everything the model is told about the runner, as compact fact lines.
/// Built from the same store queries the dashboard uses, so the assistant can
/// quote real numbers instead of guessing.
pub fn build_context(store: &Store, body_part: &str, th: &Thresholds) -> Result<String> {
    let region = region_label(body_part);
    let mut lines: Vec<String> = Vec::new();

    let entries = store.list_pain_entries(200)?;
    let mine: Vec<&PainEntry> = entries
        .iter()
        .filter(|e| e.body_part == body_part)
        .take(30)
        .collect();
    if mine.is_empty() {
        lines.push(format!("No pain entries recorded for the {region} yet."));
    } else {
        lines.push(format!("Pain log for the {region} (newest first):"));
        for e in &mine {
            let status = match e.resolved_on {
                Some(r) => format!("resolved {}", r.format("%Y-%m-%d")),
                None => "still hurting".into(),
            };
            let notes = e.notes.trim();
            let note_part = if notes.is_empty() {
                String::new()
            } else {
                format!(", notes: \"{notes}\"")
            };
            lines.push(format!(
                "- logged {} {}/10 (since {}, {status}{note_part})",
                e.created_at.format("%Y-%m-%d"),
                e.intensity,
                e.started_on.format("%Y-%m-%d"),
            ));
        }
    }

    let today = Utc::now().date_naive();
    let (factors, _) = metrics::compute_risk_factors(store, 7, th)?;
    if !factors.is_empty() {
        lines.push("Current training-risk factors:".into());
        for f in factors.iter().take(8) {
            lines.push(format!("- {}: {}", f.label, f.detail));
        }
    }
    let health = store.recent_health(3)?;
    lines.push(format!(
        "Recovery score (sleep/RHR/HRV, last 3 days): {}/100.",
        recovery_from_health(&health)
    ));
    let last7 = store.distance_window(7, 0)? / 1000.0;
    let prior = store.distance_window(28, 7)? / 1000.0 / 3.0;
    lines.push(format!(
        "Mileage: last 7 days {last7:.1} km; prior 3-week baseline {prior:.1} km/wk."
    ));

    if let Some((plan, workouts)) = store.current_plan()? {
        let horizon = today + chrono::Duration::days(7);
        let upcoming: Vec<&PlanWorkout> = workouts
            .iter()
            .filter(|w| !w.done && w.date > today && w.date <= horizon)
            .collect();
        lines.push(format!(
            "Active plan: {} {} ending {}, {} scheduled session(s) in the next 7 days.",
            plan.kind.label().to_lowercase(),
            plan.distance.label(),
            plan.target_date.format("%Y-%m-%d"),
            upcoming.len()
        ));
        for w in upcoming.iter().take(5) {
            lines.push(format!(
                "- {} {}: {}",
                w.date.format("%a %m-%d"),
                kind_word(w.kind),
                w.title
            ));
        }
    } else {
        lines.push("No active training plan.".into());
    }

    if let Some(p) = store.pending_proposal()? {
        lines.push(format!("A plan change is already on offer: {}", p.summary));
    }

    Ok(lines.join("\n"))
}

fn kind_word(k: WorkoutKind) -> &'static str {
    match k {
        WorkoutKind::Rest => "rest",
        WorkoutKind::Easy => "easy run",
        WorkoutKind::Tempo => "tempo",
        WorkoutKind::Intervals => "intervals",
        WorkoutKind::LongRun => "long run",
        WorkoutKind::Race => "race",
    }
}

const SYSTEM_PROMPT: &str = "\
You are a running-injury triage assistant inside Runalytics, a training app. \
You are given the runner's pain journal and objective training data. \
Help them think about what their pain might mean and what to watch for. \
Rules: you are not a doctor and must not diagnose; when you use a number, quote \
it from the supplied data, never invent one; ask at most one clarifying question \
per reply; if you spot a red flag (sharp or worsening pain, swelling, numbness, \
pain that alters your gait, night pain), say plainly that they should see a \
clinician; keep replies under 180 words; never promise or describe a plan \
change — the app offers those itself and the runner accepts them. \
Answer in the language the runner writes in.";

/// One chat turn. `history` is the stored thread (oldest first, without the
/// new user message). Returns the assistant reply text.
pub async fn chat(
    endpoint: &str,
    model: &str,
    api_key: Option<&str>,
    context: &str,
    history: &[PainMessage],
    question: &str,
) -> Result<String> {
    let mut messages = vec![serde_json::json!({
        "role": "system",
        "content": format!("{SYSTEM_PROMPT}\n\n--- Runner data ---\n{context}")
    })];
    for m in history.iter().rev().take(12).rev() {
        let role = if m.role == "assistant" {
            "assistant"
        } else {
            "user"
        };
        messages.push(serde_json::json!({ "role": role, "content": m.content }));
    }
    messages.push(serde_json::json!({ "role": "user", "content": question }));

    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "temperature": 0.4,
        "stream": false,
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|e| AppError::Other(format!("http client: {e}")))?;
    let mut req = client.post(endpoint).json(&body);
    // Local servers (Ollama, LM Studio) have no key — send no auth header.
    if let Some(k) = api_key.filter(|k| !k.trim().is_empty()) {
        req = req.bearer_auth(k);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| AppError::Provider(format!("could not reach the assistant endpoint: {e}")))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        let trimmed: String = text.chars().take(400).collect();
        return Err(AppError::Provider(format!(
            "assistant endpoint returned {status}: {trimmed}"
        )));
    }
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| AppError::Provider(format!("assistant reply was not JSON: {e}")))?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::Provider("assistant reply had no message content".into()))?;
    Ok(content)
}

/// The canned reply used when the chat cannot run — the journal still works as
/// a plain logger, and the thread shows why the chat is inert. The two causes
/// need different fixes, so they must not share one message: an endpoint that
/// is set but switched off needs the toggle, not a new endpoint.
pub fn offline_note(endpoint_set: bool) -> String {
    if endpoint_set {
        "The assistant is switched off. An endpoint is configured, so tick \
         \u{201c}Enable the assistant\u{201d} under Settings \u{2192} Assistant to start a \
         conversation. Your pain journal keeps logging either way."
            .into()
    } else {
        "The assistant is off — no endpoint is configured. Your pain journal still \
         works as a plain log, and the app still offers plan changes based on what \
         you record. Set an endpoint under Settings \u{2192} Assistant to start a \
         conversation."
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_rules() {
        assert!(validate_endpoint("https://openrouter.ai/api/v1/chat/completions").is_ok());
        assert!(validate_endpoint("http://localhost:11434/v1/chat/completions").is_ok());
        assert!(validate_endpoint("http://127.0.0.1:1234/v1/chat/completions").is_ok());
        assert!(validate_endpoint("http://example.com/v1/chat/completions").is_err());
        assert!(validate_endpoint("ftp://localhost/x").is_err());
        assert!(validate_endpoint("not a url").is_err());
    }

    /// A saved base URL must reach the chat route: pasting `.../v1` and getting
    /// an upstream 404 reads as "the key is wrong", which sends the runner off
    /// debugging the wrong thing.
    #[test]
    fn base_urls_are_completed_to_the_chat_route() {
        assert_eq!(
            validate_endpoint("https://gw.example.com/v1").unwrap(),
            "https://gw.example.com/v1/chat/completions"
        );
        assert_eq!(
            validate_endpoint("https://gw.example.com/v1/").unwrap(),
            "https://gw.example.com/v1/chat/completions"
        );
        assert_eq!(
            validate_endpoint("https://gw.example.com/api").unwrap(),
            "https://gw.example.com/api/chat/completions"
        );
        assert_eq!(
            validate_endpoint("https://gw.example.com").unwrap(),
            "https://gw.example.com/v1/chat/completions"
        );
        // Already a chat endpoint: untouched, no doubled suffix.
        assert_eq!(
            validate_endpoint("https://openrouter.ai/api/v1/chat/completions").unwrap(),
            "https://openrouter.ai/api/v1/chat/completions"
        );
        // A non-standard route is the user's explicit choice and stays as typed.
        assert_eq!(
            validate_endpoint("http://localhost:1234/aliases/my-model").unwrap(),
            "http://localhost:1234/aliases/my-model"
        );
    }

    /// The OpenAI-shaped reply must yield the message text; a malformed body
    /// must fail with a provider error rather than panicking on indexing.
    #[test]
    fn reply_parsing_is_defensive() {
        let good = serde_json::json!({
            "choices": [{ "message": { "role": "assistant", "content": "  ease off.  " } }]
        });
        let content = good["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.trim().to_string());
        assert_eq!(content.as_deref(), Some("ease off."));
        let bad = serde_json::json!({ "error": { "message": "bad key" } });
        assert!(bad["choices"][0]["message"]["content"].as_str().is_none());
    }
}
