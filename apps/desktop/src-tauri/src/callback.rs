//! The localhost redirect receiver for the OAuth code flow.
//!
//! COROS redirects the browser to `http://localhost:{port}/callback?code=…&state=…`
//! after consent. This module owns the socket: bind an ephemeral port, wait
//! for exactly one request, answer the browser with a tiny HTML page, and
//! hand the `(code, state)` pair back to the caller. A full HTTP stack would
//! be overkill — the request line is parsed by hand and everything else is a
//! fixed response.
//!
//! The listener is single-shot on purpose: a second request after the flow
//! completed (a refresh of the result page, a scanner) must not be able to
//! feed a stale code into a second exchange.

use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpListener;

/// What the browser delivered on the callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Callback {
    pub code: Option<String>,
    pub state: Option<String>,
    /// Provider-reported error, if the user denied or COROS failed early.
    pub error: Option<String>,
    pub error_description: Option<String>,
}

impl Callback {
    /// The authorization code, or a descriptive error for the UI.
    ///
    /// # Errors
    /// A human-readable message when the provider reported an error or no
    /// code arrived.
    pub fn into_code(self) -> std::result::Result<String, String> {
        if let Some(error) = self.error {
            let detail = self.error_description.unwrap_or_default();
            return Err(format!("provider reported {error}: {detail}"));
        }
        self.code
            .filter(|c| !c.is_empty())
            .ok_or_else(|| "callback carried no authorization code".to_string())
    }
}

/// A bound callback listener awaiting the browser.
pub struct CallbackListener {
    listener: TcpListener,
    port: u16,
}

impl CallbackListener {
    /// Bind `127.0.0.1:0` and report the chosen port, which the caller must
    /// register with the provider *before* opening the consent URL.
    ///
    /// # Errors
    /// I/O failure binding the loopback port (should not happen).
    pub async fn bind() -> std::io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let port = listener.local_addr()?.port();
        Ok(Self { listener, port })
    }

    /// The exact redirect URI to register and announce, byte-stable: COROS
    /// compares it verbatim across register → authorize → exchange.
    #[must_use]
    pub fn redirect_uri(&self) -> String {
        format!("http://localhost:{}/callback", self.port)
    }

    /// The loopback port the listener is bound to.
    #[cfg(test)]
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Wait (up to `timeout`) for the browser's callback request.
    ///
    /// # Errors
    /// A message when the deadline passes or the connection dies before a
    /// request line arrives.
    pub async fn wait(self, timeout: std::time::Duration) -> std::result::Result<Callback, String> {
        let outcome = tokio::time::timeout(timeout, async {
            let (mut socket, _) = self
                .listener
                .accept()
                .await
                .map_err(|e| format!("callback socket failed: {e}"))?;
            let mut buf = vec![0_u8; 8 * 1024];
            let mut read = 0_usize;
            // Read until the request line is complete (headers may still be
            // in flight; the query we need is entirely in the first line).
            let request_line = loop {
                let n = socket
                    .read(&mut buf[read..])
                    .await
                    .map_err(|e| format!("callback read failed: {e}"))?;
                if n == 0 {
                    return Err("browser closed the connection".to_string());
                }
                read += n;
                if let Some(pos) = buf[..read].windows(2).position(|w| w == b"\r\n") {
                    break String::from_utf8_lossy(&buf[..pos]).into_owned();
                }
                if read >= buf.len() {
                    return Err("callback request too large".to_string());
                }
            };
            let target = request_line
                .split_whitespace()
                .nth(1)
                .ok_or_else(|| "malformed request line".to_string())?;
            let callback = parse_query(target);
            let page = if callback.error.is_none() && callback.code.is_some() {
                "Authorization complete. You can close this window and return to Runalytics."
            } else {
                "Authorization failed. Return to Runalytics for details."
            };
            let body = format!(
                "<html><body style=\"font-family:sans-serif\"><h3>{page}</h3></body></html>"
            );
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            // Best-effort: even if the write fails we already have the code.
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.flush().await;
            Ok(callback)
        })
        .await;
        outcome.map_err(|_| "timed out waiting for the browser to return".to_string())?
    }
}

/// Parse `/callback?code=…&state=…` (or any path?query) into a [`Callback`].
fn parse_query(target: &str) -> Callback {
    let query = target.split_once('?').map_or("", |(_, q)| q);
    let mut callback = Callback {
        code: None,
        state: None,
        error: None,
        error_description: None,
    };
    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let value = percent_decode(value);
            match key {
                "code" => callback.code = Some(value),
                "state" => callback.state = Some(value),
                "error" => callback.error = Some(value),
                "error_description" => callback.error_description = Some(value),
                _ => {}
            }
        }
    }
    callback
}

/// Percent-decode a query value (`+` counts as space, per form encoding).
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    // The query is ASCII after decoding in practice; lossy keeps this total.
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_code_and_state() {
        let cb = parse_query("/callback?code=abc123&state=xyz");
        assert_eq!(cb.code.as_deref(), Some("abc123"));
        assert_eq!(cb.state.as_deref(), Some("xyz"));
        assert_eq!(cb.into_code().expect("code").as_str(), "abc123");
    }

    #[test]
    fn percent_decodes_values() {
        let cb = parse_query("/callback?code=a%2Fb%3Dc&state=s");
        assert_eq!(cb.code.as_deref(), Some("a/b=c"));
    }

    #[test]
    fn provider_error_becomes_message() {
        let cb = parse_query("/callback?error=access_denied&error_description=user+said+no");
        let err = cb.into_code().expect_err("must fail");
        assert!(err.contains("access_denied"), "{err}");
        assert!(err.contains("user said no"), "{err}");
    }

    #[test]
    fn missing_code_is_error() {
        let cb = parse_query("/callback?state=xyz");
        assert!(cb.into_code().is_err());
    }

    #[tokio::test]
    async fn listener_answers_one_request() {
        let listener = CallbackListener::bind().await.expect("bind");
        let uri = listener.redirect_uri();
        assert!(uri.starts_with("http://localhost:"));
        let port = listener.port();
        let handle =
            tokio::spawn(async move { listener.wait(std::time::Duration::from_secs(5)).await });
        // Drive it as the browser would.
        let mut client = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect");
        client
            .write_all(b"GET /callback?code=zzz&state=qq HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .expect("write");
        let cb = handle.await.expect("join").expect("callback");
        assert_eq!(cb.code.as_deref(), Some("zzz"));
        assert_eq!(cb.state.as_deref(), Some("qq"));
    }
}
