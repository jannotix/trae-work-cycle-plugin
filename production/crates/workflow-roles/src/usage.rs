use std::collections::BTreeMap;

use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::client::CompletionUsage;

#[derive(Default)]
struct RoleUsage {
    calls: u64,
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

#[derive(Default)]
pub struct UsageLedger {
    inner: Mutex<BTreeMap<(String, String), RoleUsage>>,
}

impl UsageLedger {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn record(&self, role: &str, model: &str, usage: Option<CompletionUsage>) {
        let mut guard = self.inner.lock().await;
        let entry = guard
            .entry((role.to_owned(), model.to_owned()))
            .or_default();
        entry.calls += 1;
        if let Some(usage) = usage {
            entry.prompt_tokens += usage.prompt_tokens;
            entry.completion_tokens += usage.completion_tokens;
            entry.total_tokens += usage.total_tokens;
        }
    }

    pub async fn snapshot(&self) -> Value {
        let guard = self.inner.lock().await;
        let mut calls = 0_u64;
        let entries = guard
            .iter()
            .map(|((role, model), usage)| {
                calls += usage.calls;
                json!({
                    "calls": usage.calls,
                    "completionTokens": usage.completion_tokens,
                    "model": model,
                    "promptTokens": usage.prompt_tokens,
                    "role": role,
                    "totalTokens": usage.total_tokens,
                })
            })
            .collect::<Vec<_>>();
        json!({"calls": calls, "entries": entries})
    }
}

#[cfg(test)]
mod tests {
    use super::UsageLedger;
    use crate::client::CompletionUsage;

    fn usage(prompt: u64, completion: u64) -> Option<CompletionUsage> {
        Some(CompletionUsage {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
        })
    }

    #[tokio::test]
    async fn ledger_aggregates_per_role_and_model() {
        let ledger = UsageLedger::new();
        ledger.record("architect", "provider/a", usage(10, 5)).await;
        ledger.record("architect", "provider/a", usage(1, 1)).await;
        ledger.record("arbiter", "provider/b", None).await;
        let snapshot = ledger.snapshot().await;
        assert_eq!(snapshot["calls"], 3);
        let entries = snapshot["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["role"], "arbiter");
        assert_eq!(entries[0]["calls"], 1);
        assert_eq!(entries[0]["totalTokens"], 0);
        assert_eq!(entries[1]["role"], "architect");
        assert_eq!(entries[1]["calls"], 2);
        assert_eq!(entries[1]["promptTokens"], 11);
    }
}
