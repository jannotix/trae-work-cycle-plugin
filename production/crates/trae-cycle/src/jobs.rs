use std::collections::{HashMap, VecDeque};

use serde_json::{Value, json};
use tokio::sync::Mutex;

const MAX_TRACKED_JOBS: usize = 32;

#[derive(Clone, Debug)]
pub struct Job {
    pub tool: String,
    pub state: JobState,
}

#[derive(Clone, Debug)]
pub enum JobState {
    Running,
    Done(Value),
    Failed(String),
}

#[derive(Default)]
struct JobsInner {
    entries: HashMap<String, Job>,
    order: VecDeque<String>,
}

#[derive(Default)]
pub struct Jobs {
    inner: Mutex<JobsInner>,
}

impl Jobs {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register(&self, job_id: String, tool: &str) {
        let mut guard = self.inner.lock().await;
        guard.entries.insert(
            job_id.clone(),
            Job {
                tool: tool.to_owned(),
                state: JobState::Running,
            },
        );
        guard.order.push_back(job_id);
        while guard.order.len() > MAX_TRACKED_JOBS {
            if let Some(oldest) = guard.order.pop_front() {
                guard.entries.remove(&oldest);
            }
        }
    }

    pub async fn finish(&self, job_id: &str, outcome: Result<Value, String>) {
        let mut guard = self.inner.lock().await;
        if let Some(job) = guard.entries.get_mut(job_id) {
            job.state = match outcome {
                Ok(result) => JobState::Done(result),
                Err(error) => JobState::Failed(error),
            };
        }
    }

    pub async fn snapshot(&self) -> Value {
        let guard = self.inner.lock().await;
        let jobs = guard
            .order
            .iter()
            .filter_map(|id| guard.entries.get(id).map(|job| (id, job)))
            .map(|(id, job)| {
                let mut entry = serde_json::Map::new();
                entry.insert("jobId".to_owned(), json!(id));
                entry.insert("tool".to_owned(), json!(job.tool));
                match &job.state {
                    JobState::Running => {
                        entry.insert("state".to_owned(), json!("running"));
                    }
                    JobState::Done(result) => {
                        entry.insert("state".to_owned(), json!("done"));
                        entry.insert("result".to_owned(), result.clone());
                    }
                    JobState::Failed(error) => {
                        entry.insert("state".to_owned(), json!("failed"));
                        entry.insert("error".to_owned(), json!(error));
                    }
                }
                Value::Object(entry)
            })
            .collect::<Vec<_>>();
        json!({ "jobs": jobs })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Jobs;

    #[tokio::test]
    async fn jobs_track_state_and_cap_history() {
        let jobs = Jobs::new();
        for index in 0..40 {
            let id = format!("job-{index}");
            jobs.register(id.clone(), "cycle_verify").await;
            jobs.finish(&id, Ok(json!({"index": index}))).await;
        }
        let snapshot = jobs.snapshot().await;
        let list = snapshot["jobs"].as_array().unwrap();
        assert_eq!(list.len(), 32);
        assert_eq!(list[0]["tool"], "cycle_verify");
        assert_eq!(list[0]["state"], "done");
    }

    #[tokio::test]
    async fn failed_jobs_keep_their_error() {
        let jobs = Jobs::new();
        jobs.register("job-1".to_owned(), "cycle_promote").await;
        jobs.finish("job-1", Err("delivery conflict".to_owned()))
            .await;
        let snapshot = jobs.snapshot().await;
        assert_eq!(snapshot["jobs"][0]["state"], "failed");
        assert_eq!(snapshot["jobs"][0]["error"], "delivery conflict");
    }
}
