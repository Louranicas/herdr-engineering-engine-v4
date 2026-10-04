//! Local-model door: a blocking client for Ollama's `/api/tags` and `/api/generate`.

use std::time::Duration;

use serde_json::{Value, json};

/// Why a model call failed.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ModelError {
    /// Nothing answered at the base URL.
    #[error("model unreachable: {0}")]
    ModelUnreachable(String),
    /// The daemon did not answer within the timeout.
    #[error("model timeout after {0:?}")]
    ModelTimeout(Duration),
    /// The daemon answered, but not with the expected JSON.
    #[error("model malformed: {0}")]
    ModelMalformed(String),
}

/// One finished generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generation {
    /// Generated text.
    pub text: String,
    /// Tokens evaluated, as reported by the daemon (0 if absent).
    pub eval_count: u64,
    /// Total duration in milliseconds, from the daemon's `total_duration` (ns).
    pub duration_ms: u64,
}

/// Client for one Ollama base URL.
#[derive(Debug, Clone)]
pub struct OllamaClient {
    base_url: String,
}

impl OllamaClient {
    /// A client for `base_url`, e.g. `http://127.0.0.1:11434`.
    #[must_use]
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
        }
    }

    /// Names of the models the daemon lists at `/api/tags`.
    ///
    /// # Errors
    /// [`ModelError`] when unreachable, slow, or the JSON lacks `models[].name`.
    pub fn tags(&self) -> Result<Vec<String>, ModelError> {
        let timeout = Duration::from_secs(10);
        let body = self.call("/api/tags", None, timeout)?;
        let models = body
            .get("models")
            .and_then(Value::as_array)
            .ok_or_else(|| ModelError::ModelMalformed("no `models` array".into()))?;
        models
            .iter()
            .map(|m| {
                m.get("name")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .ok_or_else(|| ModelError::ModelMalformed("model without `name`".into()))
            })
            .collect()
    }

    /// One non-streaming completion from `/api/generate`.
    ///
    /// # Errors
    /// [`ModelError`] when unreachable, past `timeout`, or the JSON lacks `response`.
    pub fn generate(
        &self,
        model: &str,
        prompt: &str,
        timeout: Duration,
    ) -> Result<Generation, ModelError> {
        let req = json!({"model": model, "prompt": prompt, "stream": false});
        let body = self.call("/api/generate", Some(&req), timeout)?;
        let text = body
            .get("response")
            .and_then(Value::as_str)
            .ok_or_else(|| ModelError::ModelMalformed("no `response` string".into()))?
            .to_owned();
        let eval_count = body.get("eval_count").and_then(Value::as_u64).unwrap_or(0);
        let duration_ms = body
            .get("total_duration")
            .and_then(Value::as_u64)
            .map_or(0, |ns| ns / 1_000_000);
        Ok(Generation {
            text,
            eval_count,
            duration_ms,
        })
    }

    fn call(
        &self,
        path: &str,
        req: Option<&Value>,
        timeout: Duration,
    ) -> Result<Value, ModelError> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .http_status_as_error(false)
            .build()
            .into();
        let url = format!("{}{path}", self.base_url);
        let sent = match req {
            Some(v) => agent.post(&url).send_json(v),
            None => agent.get(&url).call(),
        };
        let mut resp = sent.map_err(|e| classify(&e, timeout))?;
        let status = resp.status().as_u16();
        let text = resp
            .body_mut()
            .read_to_string()
            .map_err(|e| classify(&e, timeout))?;
        if status != 200 {
            return Err(ModelError::ModelMalformed(format!("http status {status}")));
        }
        serde_json::from_str(&text).map_err(|e| ModelError::ModelMalformed(e.to_string()))
    }
}

fn classify(e: &ureq::Error, timeout: Duration) -> ModelError {
    match e {
        ureq::Error::Timeout(_) => ModelError::ModelTimeout(timeout),
        ureq::Error::Io(io)
            if matches!(
                io.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            ) =>
        {
            ModelError::ModelTimeout(timeout)
        }
        other => ModelError::ModelUnreachable(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    type R = Result<(), Box<dyn std::error::Error>>;
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    /// One-shot mock on 127.0.0.1: reads the request, optionally sleeps, writes `reply`.
    fn mock(reply: &'static str, stall: Duration) -> std::io::Result<String> {
        let l = TcpListener::bind("127.0.0.1:0")?;
        let addr = l.local_addr()?;
        thread::spawn(move || {
            if let Ok((mut s, _)) = l.accept() {
                let mut buf = [0u8; 4096];
                let _ = s.read(&mut buf);
                thread::sleep(stall);
                let _ = s.write_all(reply.as_bytes());
            }
        });
        Ok(format!("http://{addr}"))
    }

    fn http(body: &str) -> &'static str {
        Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .into_boxed_str(),
        )
    }

    #[test]
    fn tags_lists_names() -> R {
        let url = mock(
            http(r#"{"models":[{"name":"a:1"},{"name":"b:2"}]}"#),
            Duration::ZERO,
        )?;
        assert_eq!(
            OllamaClient::new(&url).tags(),
            Ok(vec!["a:1".into(), "b:2".into()])
        );
        Ok(())
    }

    #[test]
    fn generate_parses_fields() -> R {
        let url = mock(
            http(r#"{"response":"hi","eval_count":7,"total_duration":3000000}"#),
            Duration::ZERO,
        )?;
        let g = OllamaClient::new(&url).generate("m", "p", Duration::from_secs(5))?;
        assert_eq!(
            g,
            Generation {
                text: "hi".into(),
                eval_count: 7,
                duration_ms: 3
            }
        );
        Ok(())
    }

    #[test]
    fn malformed_json_is_typed() -> R {
        let url = mock(http("not json"), Duration::ZERO)?;
        let e = OllamaClient::new(&url).generate("m", "p", Duration::from_secs(5));
        assert!(matches!(e, Err(ModelError::ModelMalformed(_))), "{e:?}");
        Ok(())
    }

    #[test]
    fn missing_field_is_malformed() -> R {
        let url = mock(http(r#"{"nope":1}"#), Duration::ZERO)?;
        let e = OllamaClient::new(&url).generate("m", "p", Duration::from_secs(5));
        assert!(matches!(e, Err(ModelError::ModelMalformed(_))), "{e:?}");
        Ok(())
    }

    #[test]
    fn slow_server_times_out() -> R {
        let url = mock(http("{}"), Duration::from_secs(3))?;
        let t = Duration::from_millis(200);
        let e = OllamaClient::new(&url).generate("m", "p", t);
        assert_eq!(e, Err(ModelError::ModelTimeout(t)));
        Ok(())
    }

    #[test]
    fn closed_port_is_unreachable() -> R {
        let port = {
            let l = TcpListener::bind("127.0.0.1:0")?;
            l.local_addr()?.port()
        };
        let e = OllamaClient::new(&format!("http://127.0.0.1:{port}")).tags();
        assert!(matches!(e, Err(ModelError::ModelUnreachable(_))), "{e:?}");
        Ok(())
    }
}
