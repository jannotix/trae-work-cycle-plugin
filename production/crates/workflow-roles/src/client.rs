use std::{path::Path, time::Duration};

use serde::Deserialize;
use serde_json::{Value, json};
use workflow_core::{ArbiterVerdict, ReviewVerdict};

use crate::config::{RoleEndpoint, RolesFile};
use crate::operation::RoleOperation;
use crate::prompts;
use crate::usage::UsageLedger;

pub const CONSULT_TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Debug)]
pub enum RoleError {
    Config(String),
    EmptyContent,
    Malformed(String),
    Status { code: u16, excerpt: String },
    Transport(String),
}

impl RoleError {
    #[must_use]
    pub fn is_transient(&self) -> bool {
        match self {
            Self::Transport(_) => true,
            Self::Status { code, .. } => *code == 429 || *code >= 500,
            _ => false,
        }
    }
}

impl std::fmt::Display for RoleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(message) => write!(formatter, "role configuration error: {message}"),
            Self::EmptyContent => {
                write!(formatter, "role endpoint returned an empty completion")
            }
            Self::Malformed(message) => {
                write!(formatter, "role output is not valid: {message}")
            }
            Self::Status { code, excerpt } => {
                write!(formatter, "role endpoint returned HTTP {code}: {excerpt}")
            }
            Self::Transport(message) => {
                write!(formatter, "role endpoint is unreachable: {message}")
            }
        }
    }
}

impl std::error::Error for RoleError {}

impl From<RoleError> for String {
    fn from(value: RoleError) -> Self {
        value.to_string()
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct CompletionUsage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
}

#[derive(Debug)]
pub struct RoleCall<T> {
    pub result: T,
    pub usage: Option<CompletionUsage>,
}

#[derive(Debug)]
pub enum ReviewOutput {
    Advisory(Value),
    Binding(ReviewVerdict),
}

#[derive(Clone)]
pub struct RolesClient {
    http: reqwest::Client,
}

impl RolesClient {
    #[must_use]
    pub fn new(timeout: Duration) -> Self {
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("http client initializes");
        Self { http }
    }

    pub async fn consult(
        &self,
        config: &RolesFile,
        data_dir: &Path,
        operation: RoleOperation,
        request: &str,
        usage: &UsageLedger,
    ) -> Result<RoleCall<Value>, RoleError> {
        let role = operation.role();
        let endpoint = endpoint_for(config, role)?;
        let api_key = resolve_key(endpoint, data_dir)?;
        let (content, completion) = self
            .chat(
                endpoint,
                api_key.as_deref(),
                prompts::advisory_prompt(operation),
                request,
            )
            .await?;
        usage.record(role, &endpoint.model_id, completion).await;
        let value: Value = serde_json::from_str(&content).map_err(|error| {
            RoleError::Malformed(format!("advisory output is not JSON: {error}"))
        })?;
        if !value.is_object() {
            return Err(RoleError::Malformed(
                "advisory output must be a JSON object".to_owned(),
            ));
        }
        Ok(RoleCall {
            result: value,
            usage: completion,
        })
    }

    pub async fn review(
        &self,
        config: &RolesFile,
        data_dir: &Path,
        operation: RoleOperation,
        request: &str,
        usage: &UsageLedger,
    ) -> Result<RoleCall<ReviewOutput>, RoleError> {
        let role = operation.role();
        let endpoint = endpoint_for(config, role)?;
        let api_key = resolve_key(endpoint, data_dir)?;
        let (content, completion) = self
            .chat(
                endpoint,
                api_key.as_deref(),
                prompts::review_prompt(operation),
                request,
            )
            .await?;
        usage.record(role, &endpoint.model_id, completion).await;
        let output = match serde_json::from_str::<ReviewVerdict>(&content) {
            Ok(verdict) => {
                if verdict.role != operation.workflow_role() {
                    return Err(RoleError::Malformed(
                        "review verdict role does not match the consulted role".to_owned(),
                    ));
                }
                ReviewOutput::Binding(verdict)
            }
            Err(_) => {
                let value: Value = serde_json::from_str(&content).map_err(|error| {
                    RoleError::Malformed(format!(
                        "review output is neither a verdict nor a JSON object: {error}"
                    ))
                })?;
                if !value.is_object() {
                    return Err(RoleError::Malformed(
                        "review output must be a JSON object".to_owned(),
                    ));
                }
                ReviewOutput::Advisory(value)
            }
        };
        Ok(RoleCall {
            result: output,
            usage: completion,
        })
    }

    pub async fn arbitration(
        &self,
        config: &RolesFile,
        data_dir: &Path,
        request: &str,
        usage: &UsageLedger,
    ) -> Result<RoleCall<ArbiterVerdict>, RoleError> {
        let endpoint = endpoint_for(config, RoleOperation::ArbiterVerdict.role())?;
        let api_key = resolve_key(endpoint, data_dir)?;
        let (content, completion) = self
            .chat(
                endpoint,
                api_key.as_deref(),
                prompts::arbiter_verdict_prompt(),
                request,
            )
            .await?;
        usage
            .record(
                RoleOperation::ArbiterVerdict.role(),
                &endpoint.model_id,
                completion,
            )
            .await;
        let verdict: ArbiterVerdict = serde_json::from_str(&content).map_err(|error| {
            RoleError::Malformed(format!("arbiter verdict is not valid: {error}"))
        })?;
        Ok(RoleCall {
            result: verdict,
            usage: completion,
        })
    }

    async fn chat(
        &self,
        endpoint: &RoleEndpoint,
        api_key: Option<&str>,
        system: &str,
        request: &str,
    ) -> Result<(String, Option<CompletionUsage>), RoleError> {
        let url = format!(
            "{}/chat/completions",
            endpoint.base_url.trim_end_matches('/')
        );
        let body = json!({
            "messages": [
                {"content": system, "role": "system"},
                {"content": request, "role": "user"},
            ],
            "model": endpoint.model_id,
            "response_format": {"type": "json_object"},
        });
        let request = self.http.post(url);
        let request = if let Some(api_key) = api_key {
            request.bearer_auth(api_key)
        } else {
            request
        };
        let response = request
            .json(&body)
            .send()
            .await
            .map_err(|error| RoleError::Transport(error.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(RoleError::Status {
                code: status.as_u16(),
                excerpt: truncate(&text, 200),
            });
        }
        let payload: Value = response.json().await.map_err(|error| {
            RoleError::Malformed(format!("endpoint response is not JSON: {error}"))
        })?;
        let content = payload["choices"][0]["message"]["content"]
            .as_str()
            .ok_or(RoleError::EmptyContent)?
            .to_owned();
        let completion = payload
            .get("usage")
            .cloned()
            .and_then(|value| serde_json::from_value::<CompletionUsage>(value).ok());
        Ok((content, completion))
    }
}

fn endpoint_for<'a>(config: &'a RolesFile, role: &str) -> Result<&'a RoleEndpoint, RoleError> {
    config
        .endpoint(role)
        .ok_or_else(|| RoleError::Config(format!("role '{role}' has no configured endpoint")))
}

fn resolve_key(endpoint: &RoleEndpoint, data_dir: &Path) -> Result<Option<String>, RoleError> {
    let raw = if let Some(name) = &endpoint.api_key_env {
        std::env::var(name)
            .map_err(|_| RoleError::Config(format!("environment variable {name} is not set")))?
    } else if let Some(file) = &endpoint.api_key_file {
        std::fs::read_to_string(data_dir.join(file)).map_err(|error| {
            RoleError::Config(format!("api key file {file} cannot be read: {error}"))
        })?
    } else if endpoint.is_loopback() {
        return Ok(None);
    } else {
        return Err(RoleError::Config(format!(
            "role endpoint for {} has no api key source",
            endpoint.model_id
        )));
    };
    let key = raw.trim().to_owned();
    if key.is_empty() {
        return Err(RoleError::Config("api key is empty".to_owned()));
    }
    Ok(Some(key))
}

fn truncate(value: &str, limit: usize) -> String {
    if value.len() <= limit {
        value.to_owned()
    } else {
        let mut cut = limit;
        while !value.is_char_boundary(cut) {
            cut -= 1;
        }
        format!("{}...", &value[..cut])
    }
}

#[cfg(test)]
mod tests {
    use super::{RoleError, truncate};

    #[test]
    fn error_classification_separates_transient_from_permanent() {
        assert!(
            RoleError::Status {
                code: 503,
                excerpt: String::new()
            }
            .is_transient()
        );
        assert!(
            RoleError::Status {
                code: 429,
                excerpt: String::new()
            }
            .is_transient()
        );
        assert!(
            !RoleError::Status {
                code: 401,
                excerpt: String::new()
            }
            .is_transient()
        );
        assert!(RoleError::Transport("timed out".to_owned()).is_transient());
        assert!(!RoleError::Malformed("bad json".to_owned()).is_transient());
        assert!(!RoleError::Config("missing".to_owned()).is_transient());
    }

    #[test]
    fn truncation_respects_character_boundaries() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("abcdef", 3), "abc...");
    }
}
