use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use serde_json::{Value, json};
use workflow_core::{
    ArbiterVerdict, ArchitecturePlan, CandidateId, ContentDigest, GoalId, ProtocolEnvelope,
    ProtocolPayload, ReceiptId, RequestRecord, ReviewVerdict, UserRoutingPreference,
    VerificationPlanId, WorkflowId,
};
use workflow_ipc::{
    ClientMessage, ControlOperation, ExecutionOutcome, GoalControlAction, GoalOperation,
    IpcRequest, IpcResponse, ServerMessage,
    audit::{AuditData, AuditModel, AuditObservation},
    protocol::{HistoryOperation, ManagedBrowserAttestation, MemoryOperation},
};
use workflow_roles::{
    CompletionUsage, ReviewOutput, RoleOperation, RolesClient, UsageLedger, config as roles,
    config::RolesFile,
};

use crate::daemon::Daemon;
use crate::jobs::Jobs;

const FREEZE_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const PROMOTE_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const VERIFY_TIMEOUT: Duration = Duration::from_secs(24 * 60 * 60);
const ARBITRATE_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const INDEX_TIMEOUT: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Clone)]
pub struct ToolContext {
    pub daemon: Daemon,
    pub jobs: Arc<Jobs>,
    pub usage: Arc<UsageLedger>,
    pub roles: RolesClient,
    pub data_dir: PathBuf,
}

pub fn descriptors() -> Vec<Value> {
    let tool = |name: &str, description: &str, properties: Value, required: &[&str]| {
        let required = required.iter().map(|item| json!(item)).collect::<Vec<_>>();
        json!({
            "description": description,
            "inputSchema": {
                "additionalProperties": false,
                "properties": properties,
                "required": required,
                "type": "object",
            },
            "name": name,
        })
    };
    let project_key = || json!({"type": "string", "minLength": 1});
    let workflow_id = || json!({"type": "string"});
    vec![
        tool(
            "cycle_setup",
            "Validate installation, role models, Git and the local control plane.",
            json!({}),
            &[],
        ),
        tool(
            "cycle_doctor",
            "Read-only diagnostics for the control plane, database, ledger and role configuration.",
            json!({}),
            &[],
        ),
        tool(
            "cycle_start",
            "Arm or launch a governed workflow from the exact user request.",
            json!({
                "affected_paths": {"type": "array", "items": {"type": "string"}},
                "critical_downgrade_approval": {"type": "string"},
                "mode": {"type": "string", "enum": ["auto", "quick", "full"]},
                "original_request": {"type": "string", "minLength": 1},
                "project_key": project_key(),
            }),
            &["project_key", "original_request", "mode"],
        ),
        tool(
            "cycle_status",
            "Show workflow state, repair budget and background job results.",
            json!({
                "project_key": project_key(),
                "workflow_id": workflow_id(),
            }),
            &["project_key"],
        ),
        tool(
            "cycle_tasks",
            "Show durable task identifiers, states and dependencies.",
            json!({
                "project_key": project_key(),
                "workflow_id": workflow_id(),
            }),
            &["project_key"],
        ),
        tool(
            "cycle_evidence",
            "Register executor evidence such as command outputs or browser checks in the audit ledger.",
            json!({
                "files": {"type": "array", "items": {"type": "string"}},
                "metadata": {"type": "object", "additionalProperties": {"type": "string"}},
                "project_key": project_key(),
                "session_id": {"type": "string"},
                "workflow_id": workflow_id(),
            }),
            &["project_key"],
        ),
        tool(
            "cycle_worktree",
            "Create the isolated Git worktree for governed execution. Returns the worktree path and base revision.",
            json!({
                "project_directory": {"type": "string", "minLength": 1},
                "project_key": project_key(),
                "workflow_id": workflow_id(),
            }),
            &["project_key", "workflow_id", "project_directory"],
        ),
        tool(
            "cycle_index",
            "Index the project repository for code intelligence and delivery identity. Long running; returns a job.",
            json!({
                "project_directory": {"type": "string", "minLength": 1},
                "project_key": project_key(),
                "workflow_id": workflow_id(),
            }),
            &["project_key", "workflow_id", "project_directory"],
        ),
        tool(
            "cycle_submit_architecture",
            "Submit the architect plan for validation and acceptance.",
            json!({
                "plan": {"type": "object"},
                "project_key": project_key(),
                "workflow_id": workflow_id(),
            }),
            &["project_key", "workflow_id", "plan"],
        ),
        tool(
            "cycle_execution_report",
            "Report task completion state: blocked or plan_defect.",
            json!({
                "outcome": {"type": "string", "enum": ["blocked", "plan_defect"]},
                "project_key": project_key(),
                "workflow_id": workflow_id(),
            }),
            &["project_key", "workflow_id", "outcome"],
        ),
        tool(
            "cycle_freeze",
            "Plan verification and freeze the exact candidate. Long running; returns a job.",
            json!({
                "base_revision": {"type": "string", "minLength": 40},
                "project_key": project_key(),
                "workflow_id": workflow_id(),
            }),
            &["project_key", "workflow_id", "base_revision"],
        ),
        tool(
            "cycle_verify",
            "Run the verification plan over the frozen candidate. Long running; returns a job.",
            json!({
                "attestations": {"type": "array", "items": {"type": "object"}},
                "candidate_id": {"type": "string"},
                "plan_id": {"type": "string"},
                "project_key": project_key(),
                "workflow_id": workflow_id(),
            }),
            &["project_key", "workflow_id", "candidate_id", "plan_id"],
        ),
        tool(
            "cycle_review",
            "Submit one independent review verdict bound to the frozen candidate.",
            json!({
                "candidate_id": {"type": "string"},
                "project_key": project_key(),
                "verdict": {"type": "object"},
                "workflow_id": workflow_id(),
            }),
            &["project_key", "workflow_id", "candidate_id", "verdict"],
        ),
        tool(
            "cycle_arbitrate",
            "Submit the final arbiter verdict. Long running; returns a job.",
            json!({
                "candidate_id": {"type": "string"},
                "project_key": project_key(),
                "verdict": {"type": "object"},
                "workflow_id": workflow_id(),
            }),
            &["project_key", "workflow_id", "candidate_id", "verdict"],
        ),
        tool(
            "cycle_promote",
            "Deliver the approved exact bytes into the project directory. Long running; returns a job.",
            json!({
                "candidate_id": {"type": "string"},
                "project_directory": {"type": "string", "minLength": 1},
                "project_key": project_key(),
                "workflow_id": workflow_id(),
            }),
            &[
                "project_key",
                "workflow_id",
                "candidate_id",
                "project_directory",
            ],
        ),
        tool(
            "cycle_pause",
            "Pause the workflow at the next safe boundary.",
            json!({
                "project_key": project_key(),
                "workflow_id": workflow_id(),
            }),
            &["project_key"],
        ),
        tool(
            "cycle_resume",
            "Resume a paused workflow from its saved phase.",
            json!({
                "project_key": project_key(),
                "workflow_id": workflow_id(),
            }),
            &["project_key"],
        ),
        tool(
            "cycle_retry",
            "Retry a classified transient failure without consuming a repair cycle.",
            json!({
                "project_key": project_key(),
                "workflow_id": workflow_id(),
            }),
            &["project_key"],
        ),
        tool(
            "cycle_cancel",
            "Cancel the workflow. Requires explicit confirmation.",
            json!({
                "confirm": {"type": "boolean"},
                "project_key": project_key(),
                "workflow_id": workflow_id(),
            }),
            &["project_key", "confirm"],
        ),
        tool(
            "cycle_role",
            "Consult a single role without starting a delivery cycle. Remote consultations run as jobs.",
            json!({
                "operation": {"type": "string", "enum": ["architect_consult", "executor_feasibility", "functional_review", "security_review", "arbiter_readiness", "arbiter_verdict"]},
                "project_key": project_key(),
                "request": {"type": "string", "minLength": 1},
                "role": {"type": "string", "enum": ["architect", "executor", "functional_reviewer", "security_reviewer", "arbiter"]},
                "session_id": {"type": "string"},
            }),
            &["operation", "role", "request"],
        ),
        tool(
            "cycle_goal_create",
            "Create a durable multi-milestone goal.",
            json!({
                "constraints": {"type": "array", "items": {"type": "string"}},
                "max_continuations": {"type": "integer", "minimum": 1, "maximum": 255},
                "non_goals": {"type": "array", "items": {"type": "string"}},
                "objective": {"type": "string", "minLength": 1},
                "project_key": project_key(),
                "session_id": {"type": "string", "minLength": 1},
                "success_criteria": {"type": "array", "items": {"type": "string"}, "minItems": 1},
            }),
            &["project_key", "session_id", "objective", "success_criteria"],
        ),
        tool(
            "cycle_goal_amend",
            "Append an immutable amendment to a goal.",
            json!({
                "goal_id": {"type": "string"},
                "project_key": project_key(),
                "text": {"type": "string", "minLength": 1},
            }),
            &["project_key", "goal_id", "text"],
        ),
        tool(
            "cycle_goal_status",
            "Show a goal, its latest plan and linked workflows.",
            json!({
                "goal_id": {"type": "string"},
                "project_key": project_key(),
                "session_id": {"type": "string"},
            }),
            &["project_key", "session_id"],
        ),
        tool(
            "cycle_goal_list",
            "List goals owned by the project.",
            json!({
                "project_key": project_key(),
            }),
            &["project_key"],
        ),
        tool(
            "cycle_goal_focus",
            "Focus one project goal in the session.",
            json!({
                "goal_id": {"type": "string"},
                "project_key": project_key(),
                "session_id": {"type": "string", "minLength": 1},
            }),
            &["project_key", "session_id", "goal_id"],
        ),
        tool(
            "cycle_goal_save_plan",
            "Save a new immutable goal plan revision.",
            json!({
                "content": {"type": "string", "minLength": 1},
                "goal_id": {"type": "string"},
                "project_key": project_key(),
                "source_session_id": {"type": "string", "minLength": 1},
            }),
            &["project_key", "goal_id", "source_session_id", "content"],
        ),
        tool(
            "cycle_goal_link",
            "Link a completed-eligible workflow to a goal milestone.",
            json!({
                "goal_id": {"type": "string"},
                "milestone": {"type": "string", "minLength": 1},
                "project_key": project_key(),
                "workflow_id": workflow_id(),
            }),
            &["project_key", "goal_id", "milestone", "workflow_id"],
        ),
        tool(
            "cycle_goal_control",
            "Apply one idempotent goal lifecycle transition.",
            json!({
                "action": {"type": "string", "enum": ["start_planning", "mark_ready", "activate", "pause", "resume", "block", "resume_blocked", "continue", "request_completion", "approve_completion", "reject_completion", "abort"]},
                "completion_evidence": {"type": "string"},
                "goal_id": {"type": "string"},
                "project_key": project_key(),
                "reason": {"type": "string"},
            }),
            &["project_key", "goal_id", "action"],
        ),
        tool(
            "cycle_memory_search",
            "Search reusable project knowledge.",
            json!({
                "confidence": {"type": "string", "enum": ["verified", "user_asserted", "inferred"]},
                "limit": {"type": "integer", "minimum": 1, "maximum": 1000},
                "project_key": project_key(),
                "scope": {"type": "string"},
                "text": {"type": "string", "minLength": 1},
            }),
            &["project_key", "text"],
        ),
        tool(
            "cycle_memory_explain",
            "Load one memory entry with its provenance.",
            json!({
                "memory_id": {"type": "string"},
                "project_key": project_key(),
            }),
            &["project_key", "memory_id"],
        ),
        tool(
            "cycle_memory_remove",
            "Revoke an eligible memory entry. Requires confirmation.",
            json!({
                "confirm": {"type": "boolean"},
                "memory_id": {"type": "string"},
                "project_key": project_key(),
            }),
            &["project_key", "memory_id", "confirm"],
        ),
        tool(
            "cycle_history",
            "Query the redacted project audit history.",
            json!({
                "after_sequence": {"type": "integer", "minimum": 0},
                "limit": {"type": "integer", "minimum": 1, "maximum": 500},
                "project_key": project_key(),
            }),
            &["project_key"],
        ),
        tool(
            "cycle_history_verify",
            "Verify the audit hash chain and signed checkpoints.",
            json!({
                "project_key": project_key(),
            }),
            &["project_key"],
        ),
        tool(
            "cycle_models",
            "Show effective per-role model assignments without secrets.",
            json!({}),
            &[],
        ),
        tool(
            "cycle_limits",
            "Show the live admission policy and resource reserves.",
            json!({}),
            &[],
        ),
        tool(
            "cycle_export",
            "Export redacted history. Requires explicit confirmation.",
            json!({
                "confirm": {"type": "boolean"},
                "project_key": project_key(),
            }),
            &["project_key", "confirm"],
        ),
    ]
}

pub async fn call(ctx: &ToolContext, name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "cycle_setup" => setup(ctx).await,
        "cycle_doctor" => doctor(ctx).await,
        "cycle_start" => start(ctx, args).await,
        "cycle_status" => status(ctx, args).await,
        "cycle_tasks" => control_result(ctx, args, ControlOperation::Tasks).await,
        "cycle_evidence" => evidence(ctx, args).await,
        "cycle_worktree" => worktree(ctx, args).await,
        "cycle_index" => index(ctx, args).await,
        "cycle_submit_architecture" => submit_architecture(ctx, args).await,
        "cycle_execution_report" => execution_report(ctx, args).await,
        "cycle_freeze" => freeze(ctx, args).await,
        "cycle_verify" => verify(ctx, args).await,
        "cycle_review" => review(ctx, args).await,
        "cycle_arbitrate" => arbitrate(ctx, args).await,
        "cycle_promote" => promote(ctx, args).await,
        "cycle_pause" => control_result(ctx, args, ControlOperation::Pause).await,
        "cycle_resume" => control_result(ctx, args, ControlOperation::Resume).await,
        "cycle_retry" => control_result(ctx, args, ControlOperation::Retry).await,
        "cycle_cancel" => cancel(ctx, args).await,
        "cycle_role" => role(ctx, args).await,
        "cycle_goal_create" => goal_create(ctx, args).await,
        "cycle_goal_amend" => {
            goal_op(
                ctx,
                args,
                GoalOperation::Amend {
                    goal_id: id_arg(args, "goal_id")?,
                    operation_id: ReceiptId::new(),
                    text: str_arg(args, "text")?,
                },
            )
            .await
        }
        "cycle_goal_status" => {
            goal_op(
                ctx,
                args,
                GoalOperation::Status {
                    goal_id: opt_id_arg(args, "goal_id")?,
                    session_id: str_arg(args, "session_id")?,
                },
            )
            .await
        }
        "cycle_goal_list" => goal_op(ctx, args, GoalOperation::List {}).await,
        "cycle_goal_focus" => {
            goal_op(
                ctx,
                args,
                GoalOperation::Focus {
                    goal_id: id_arg(args, "goal_id")?,
                    session_id: str_arg(args, "session_id")?,
                },
            )
            .await
        }
        "cycle_goal_save_plan" => {
            goal_op(
                ctx,
                args,
                GoalOperation::SavePlan {
                    content: str_arg(args, "content")?,
                    goal_id: id_arg(args, "goal_id")?,
                    source_session_id: str_arg(args, "source_session_id")?,
                },
            )
            .await
        }
        "cycle_goal_link" => {
            goal_op(
                ctx,
                args,
                GoalOperation::LinkWorkflow {
                    goal_id: id_arg(args, "goal_id")?,
                    milestone: str_arg(args, "milestone")?,
                    workflow_id: id_arg(args, "workflow_id")?,
                },
            )
            .await
        }
        "cycle_goal_control" => goal_control(ctx, args).await,
        "cycle_memory_search" => {
            memory_op(
                ctx,
                args,
                MemoryOperation::Search {
                    confidence: opt_str_arg(args, "confidence")?,
                    limit: args
                        .get("limit")
                        .and_then(Value::as_u64)
                        .unwrap_or(20)
                        .clamp(1, 1000) as usize,
                    scope: opt_str_arg(args, "scope")?,
                    text: str_arg(args, "text")?,
                },
            )
            .await
        }
        "cycle_memory_explain" => {
            memory_op(
                ctx,
                args,
                MemoryOperation::Explain {
                    memory_id: id_arg(args, "memory_id")?,
                },
            )
            .await
        }
        "cycle_memory_remove" => {
            require_confirm(args)?;
            memory_op(
                ctx,
                args,
                MemoryOperation::Remove {
                    memory_id: id_arg(args, "memory_id")?,
                },
            )
            .await
        }
        "cycle_history" => history(ctx, args).await,
        "cycle_history_verify" => history_verify(ctx, args).await,
        "cycle_models" => models(ctx).await,
        "cycle_limits" => limits(ctx).await,
        "cycle_export" => export(ctx, args).await,
        _ => Err(format!("unknown tool {name}")),
    }
}

async fn setup(ctx: &ToolContext) -> Result<Value, String> {
    let roles_report = match roles::load(&ctx.data_dir) {
        Ok(file) => json!({"configured": true, "assignments": file.report()}),
        Err(error) => json!({"configured": false, "error": error.to_string()}),
    };
    let git = std::process::Command::new("git")
        .arg("--version")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_default();
    let probe = ctx.data_dir.join(".setup-probe");
    std::fs::write(&probe, b"probe")
        .map_err(|error| format!("data directory is not writable: {error}"))?;
    let _ = std::fs::remove_file(&probe);
    let health = ctx.daemon.ensure().await?;
    Ok(json!({
        "dataDirectory": ctx.data_dir,
        "git": if git.is_empty() { Value::Null } else { json!(git) },
        "roles": roles_report,
        "writable": true,
        "controlPlane": health,
    }))
}

async fn doctor(ctx: &ToolContext) -> Result<Value, String> {
    let health = ctx.daemon.ensure().await?;
    let roles_report = match roles::load(&ctx.data_dir) {
        Ok(file) => file.report(),
        Err(_) => roles::missing_report(),
    };
    let project_key = "diagnostics".to_owned();
    let diagnostics = control(ctx, &project_key, ControlOperation::Doctor, None).await?;
    Ok(json!({
        "controlPlane": health,
        "diagnostics": diagnostics,
        "roles": roles_report,
    }))
}

async fn start(ctx: &ToolContext, args: &Value) -> Result<Value, String> {
    roles::load(&ctx.data_dir).map_err(|error| error.to_string())?;
    let project_key = str_arg(args, "project_key")?;
    let original_request = str_arg(args, "original_request")?;
    let preference = match str_arg(args, "mode")?.as_str() {
        "auto" => UserRoutingPreference::Auto,
        "quick" => UserRoutingPreference::Quick,
        "full" => UserRoutingPreference::Full,
        other => return Err(format!("invalid mode {other}")),
    };
    let affected_paths = args
        .get("affected_paths")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let downgrade = match opt_str_arg(args, "critical_downgrade_approval")? {
        Some(value) => Some(
            ReceiptId::from_str(&value)
                .map_err(|error| format!("critical_downgrade_approval is invalid: {error}"))?,
        ),
        None => None,
    };
    let envelope = ProtocolEnvelope::new(ProtocolPayload::Request(RequestRecord::new(
        original_request,
        Vec::new(),
    )));
    let message = ClientMessage::Request(IpcRequest {
        affected_paths,
        critical_downgrade_approval: downgrade,
        project_key,
        request_id: ctx.daemon.next_request_id(),
        routing_preference: preference,
        workflow_id: None,
        envelope,
    });
    match ctx.daemon.exchange(message).await? {
        ServerMessage::Response(IpcResponse::Accepted {
            mode,
            request_digest,
            workflow_id,
            ..
        }) => Ok(json!({
            "mode": mode,
            "requestDigest": request_digest,
            "workflowId": workflow_id,
        })),
        ServerMessage::Response(IpcResponse::Rejected { message, .. }) => Err(message),
        _ => Err("control plane returned an unexpected start response".to_owned()),
    }
}

async fn status(ctx: &ToolContext, args: &Value) -> Result<Value, String> {
    let project_key = str_arg(args, "project_key")?;
    let workflow_id = opt_id_arg(args, "workflow_id")?;
    let workflow = control(ctx, &project_key, ControlOperation::Status, workflow_id).await;
    let jobs = ctx.jobs.snapshot().await;
    let workflow = match workflow {
        Ok(value) => value,
        Err(error) if error.contains("project has no workflow") => Value::Null,
        Err(error) => return Err(error),
    };
    Ok(json!({
        "jobs": jobs["jobs"],
        "workflow": workflow,
    }))
}

async fn control(
    ctx: &ToolContext,
    project_key: &str,
    operation: ControlOperation,
    workflow_id: Option<WorkflowId>,
) -> Result<Value, String> {
    let message = ClientMessage::Control {
        operation,
        operation_id: ReceiptId::new(),
        project_key: project_key.to_owned(),
        request_id: ctx.daemon.next_request_id(),
        workflow_id,
    };
    match ctx.daemon.exchange(message).await? {
        ServerMessage::Control { result, .. } => Ok(result),
        _ => Err("control plane returned an unexpected control response".to_owned()),
    }
}

async fn control_result(
    ctx: &ToolContext,
    args: &Value,
    operation: ControlOperation,
) -> Result<Value, String> {
    let project_key = str_arg(args, "project_key")?;
    let workflow_id = opt_id_arg(args, "workflow_id")?;
    ctx.daemon.ensure().await?;
    control(ctx, &project_key, operation, workflow_id).await
}

async fn evidence(ctx: &ToolContext, args: &Value) -> Result<Value, String> {
    let project_key = str_arg(args, "project_key")?;
    let mut metadata = BTreeMap::new();
    if let Some(entries) = args.get("metadata").and_then(Value::as_object) {
        for (key, value) in entries {
            let text = match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            if text.len() <= 512 {
                metadata.insert(key.clone(), text);
            }
        }
    }
    let files = args
        .get("files")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .take(1000)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let observation = AuditObservation {
        actor_id: "trae-work-session".to_owned(),
        candidate_id: None,
        data: AuditData::Workflow {
            action: "executor_evidence".to_owned(),
        },
        evidence_ids: Default::default(),
        files,
        metadata,
        model: None,
        project_key,
        role: None,
        session_id: opt_str_arg(args, "session_id")?,
        task_id: None,
        timestamp_unix_millis: now_unix_millis(),
        workflow_id: opt_id_arg(args, "workflow_id")?,
    };
    let message = ClientMessage::Audit {
        request_id: ctx.daemon.next_request_id(),
        observation,
    };
    match ctx.daemon.exchange(message).await? {
        ServerMessage::AuditRecorded {
            entry_hash,
            sequence,
            ..
        } => Ok(json!({
            "entryHash": entry_hash,
            "sequence": sequence,
        })),
        _ => Err("control plane returned an unexpected audit response".to_owned()),
    }
}

async fn worktree(ctx: &ToolContext, args: &Value) -> Result<Value, String> {
    let project_key = str_arg(args, "project_key")?;
    let workflow_id = id_arg::<WorkflowId>(args, "workflow_id")?;
    let project_directory = str_arg(args, "project_directory")?;
    ctx.daemon.ensure().await?;
    let message = ClientMessage::Worktree {
        project_directory,
        project_key,
        request_id: ctx.daemon.next_request_id(),
        workflow_id,
    };
    match ctx.daemon.exchange(message).await? {
        ServerMessage::Worktree {
            base_revision,
            path,
            ..
        } => Ok(json!({"baseRevision": base_revision, "path": path})),
        _ => Err("control plane returned an unexpected worktree response".to_owned()),
    }
}

async fn index(ctx: &ToolContext, args: &Value) -> Result<Value, String> {
    let project_key = str_arg(args, "project_key")?;
    let workflow_id = id_arg::<WorkflowId>(args, "workflow_id")?;
    let project_directory = str_arg(args, "project_directory")?;
    let daemon = ctx.daemon.clone();
    Ok(spawn_job(&ctx.jobs, "cycle_index", async move {
        let message = ClientMessage::CodeIndex {
            project_directory,
            project_key,
            request_id: daemon.next_request_id(),
            workflow_id,
        };
        match daemon.exchange_long(message, INDEX_TIMEOUT).await? {
            ServerMessage::CodeIndex { result, .. } => Ok(result),
            _ => Err("control plane returned an unexpected index response".to_owned()),
        }
    })
    .await)
}

async fn submit_architecture(ctx: &ToolContext, args: &Value) -> Result<Value, String> {
    let project_key = str_arg(args, "project_key")?;
    let workflow_id = id_arg::<WorkflowId>(args, "workflow_id")?;
    let plan: ArchitecturePlan = serde_json::from_value(
        args.get("plan")
            .cloned()
            .ok_or_else(|| "plan is required".to_owned())?,
    )
    .map_err(|error| format!("plan is invalid: {error}"))?;
    let envelope = ProtocolEnvelope::new(ProtocolPayload::Architecture(plan));
    let message = ClientMessage::Request(IpcRequest {
        affected_paths: Vec::new(),
        critical_downgrade_approval: None,
        project_key,
        request_id: ctx.daemon.next_request_id(),
        routing_preference: UserRoutingPreference::Auto,
        workflow_id: Some(workflow_id),
        envelope,
    });
    match ctx.daemon.exchange(message).await? {
        ServerMessage::Response(IpcResponse::Accepted { .. }) => Ok(json!({"accepted": true})),
        ServerMessage::Response(IpcResponse::Rejected { message, .. }) => Err(message),
        _ => Err("control plane returned an unexpected architecture response".to_owned()),
    }
}

async fn execution_report(ctx: &ToolContext, args: &Value) -> Result<Value, String> {
    let project_key = str_arg(args, "project_key")?;
    let workflow_id = id_arg::<WorkflowId>(args, "workflow_id")?;
    let outcome = match str_arg(args, "outcome")?.as_str() {
        "blocked" => ExecutionOutcome::Blocked,
        "plan_defect" => ExecutionOutcome::PlanDefect,
        other => return Err(format!("invalid execution outcome {other}")),
    };
    let message = ClientMessage::ReportExecution {
        outcome,
        project_key,
        report_id: ReceiptId::new(),
        request_id: ctx.daemon.next_request_id(),
        workflow_id,
    };
    match ctx.daemon.exchange(message).await? {
        ServerMessage::ExecutionReported { workflow_state, .. } => {
            Ok(json!({"state": workflow_state}))
        }
        _ => Err("control plane returned an unexpected execution response".to_owned()),
    }
}

async fn freeze(ctx: &ToolContext, args: &Value) -> Result<Value, String> {
    let project_key = str_arg(args, "project_key")?;
    let workflow_id = id_arg::<WorkflowId>(args, "workflow_id")?;
    let base_revision = str_arg(args, "base_revision")?;
    let daemon = ctx.daemon.clone();
    Ok(spawn_job(&ctx.jobs, "cycle_freeze", async move {
        let plan_id = VerificationPlanId::new();
        let planned = daemon
            .exchange(ClientMessage::PlanVerification {
                plan_id,
                project_key: project_key.clone(),
                request_id: daemon.next_request_id(),
                workflow_id,
            })
            .await?;
        let evidence_ids = match planned {
            ServerMessage::VerificationPlanned { evidence_ids, .. } => evidence_ids,
            _ => return Err("control plane returned an unexpected plan response".to_owned()),
        };
        let candidate_id = CandidateId::new();
        let frozen = daemon
            .exchange_long(
                ClientMessage::FreezeCandidate {
                    base_revision,
                    candidate_id,
                    evidence_ids,
                    plan_id,
                    project_key,
                    request_id: daemon.next_request_id(),
                    workflow_id,
                },
                FREEZE_TIMEOUT,
            )
            .await?;
        match frozen {
            ServerMessage::CandidateFrozen {
                candidate_digest,
                manifest,
                ..
            } => Ok(json!({
                "candidateDigest": candidate_digest,
                "candidateId": candidate_id,
                "manifest": manifest,
                "planId": plan_id,
            })),
            _ => Err("control plane returned an unexpected freeze response".to_owned()),
        }
    })
    .await)
}

async fn verify(ctx: &ToolContext, args: &Value) -> Result<Value, String> {
    let project_key = str_arg(args, "project_key")?;
    let workflow_id = id_arg::<WorkflowId>(args, "workflow_id")?;
    let candidate_id = id_arg::<CandidateId>(args, "candidate_id")?;
    let plan_id = id_arg::<VerificationPlanId>(args, "plan_id")?;
    let attestations: Vec<ManagedBrowserAttestation> = args
        .get("attestations")
        .cloned()
        .map(|value| serde_json::from_value(value).map_err(|error| error.to_string()))
        .transpose()?
        .unwrap_or_default();
    let daemon = ctx.daemon.clone();
    Ok(spawn_job(&ctx.jobs, "cycle_verify", async move {
        let message = ClientMessage::VerifyCandidate {
            attestations,
            candidate_id,
            plan_id,
            project_key,
            request_id: daemon.next_request_id(),
            workflow_id,
        };
        match daemon.exchange_long(message, VERIFY_TIMEOUT).await? {
            ServerMessage::VerificationCompleted {
                evidence,
                mandatory_passed,
                workflow_state,
                ..
            } => Ok(json!({
                "evidence": evidence,
                "mandatoryPassed": mandatory_passed,
                "workflowState": workflow_state,
            })),
            _ => Err("control plane returned an unexpected verification response".to_owned()),
        }
    })
    .await)
}

async fn review(ctx: &ToolContext, args: &Value) -> Result<Value, String> {
    let project_key = str_arg(args, "project_key")?;
    let workflow_id = id_arg::<WorkflowId>(args, "workflow_id")?;
    let candidate_id = id_arg::<CandidateId>(args, "candidate_id")?;
    let verdict: ReviewVerdict = serde_json::from_value(
        args.get("verdict")
            .cloned()
            .ok_or_else(|| "verdict is required".to_owned())?,
    )
    .map_err(|error| format!("verdict is invalid: {error}"))?;
    let message = ClientMessage::SubmitReview {
        candidate_id,
        project_key,
        request_id: ctx.daemon.next_request_id(),
        verdict,
        workflow_id,
    };
    match ctx.daemon.exchange(message).await? {
        ServerMessage::ReviewRecorded { reviews_ready, .. } => {
            Ok(json!({"reviewsReady": reviews_ready}))
        }
        _ => Err("control plane returned an unexpected review response".to_owned()),
    }
}

async fn arbitrate(ctx: &ToolContext, args: &Value) -> Result<Value, String> {
    let project_key = str_arg(args, "project_key")?;
    let workflow_id = id_arg::<WorkflowId>(args, "workflow_id")?;
    let candidate_id = id_arg::<CandidateId>(args, "candidate_id")?;
    let verdict: ArbiterVerdict = serde_json::from_value(
        args.get("verdict")
            .cloned()
            .ok_or_else(|| "verdict is required".to_owned())?,
    )
    .map_err(|error| format!("verdict is invalid: {error}"))?;
    let daemon = ctx.daemon.clone();
    Ok(spawn_job(&ctx.jobs, "cycle_arbitrate", async move {
        let message = ClientMessage::SubmitArbitration {
            candidate_id,
            project_key,
            request_id: daemon.next_request_id(),
            verdict,
            workflow_id,
        };
        match daemon.exchange_long(message, ARBITRATE_TIMEOUT).await? {
            ServerMessage::ArbitrationRecorded {
                decision,
                receipt_digest,
                workflow_state,
                ..
            } => Ok(json!({
                "decision": decision,
                "receiptDigest": receipt_digest,
                "workflowState": workflow_state,
            })),
            _ => Err("control plane returned an unexpected arbitration response".to_owned()),
        }
    })
    .await)
}

async fn promote(ctx: &ToolContext, args: &Value) -> Result<Value, String> {
    let project_key = str_arg(args, "project_key")?;
    let workflow_id = id_arg::<WorkflowId>(args, "workflow_id")?;
    let candidate_id = id_arg::<CandidateId>(args, "candidate_id")?;
    let project_directory = str_arg(args, "project_directory")?;
    let daemon = ctx.daemon.clone();
    Ok(spawn_job(&ctx.jobs, "cycle_promote", async move {
        let message = ClientMessage::PromoteCandidate {
            candidate_id,
            project_directory,
            project_key,
            request_id: daemon.next_request_id(),
            workflow_id,
        };
        match daemon.exchange_long(message, PROMOTE_TIMEOUT).await? {
            ServerMessage::CandidatePromoted {
                changed_paths,
                workflow_state,
                ..
            } => Ok(json!({
                "changedPaths": changed_paths,
                "workflowState": workflow_state,
            })),
            _ => Err("control plane returned an unexpected delivery response".to_owned()),
        }
    })
    .await)
}

async fn cancel(ctx: &ToolContext, args: &Value) -> Result<Value, String> {
    require_confirm(args)?;
    control_result(ctx, args, ControlOperation::Cancel).await
}

async fn role(ctx: &ToolContext, args: &Value) -> Result<Value, String> {
    let operation =
        RoleOperation::from_str(&str_arg(args, "operation")?).map_err(|error| error.to_string())?;
    let role_name = str_arg(args, "role")?;
    if operation.role() != role_name {
        return Err(format!(
            "operation {} requires role {}",
            operation.as_str(),
            operation.role()
        ));
    }
    if operation.is_in_session() {
        return Err(
            "executor consultations run inside the Trae Work session; the executor model is the one selected in Trae Work"
                .to_owned(),
        );
    }
    let config = roles::load(&ctx.data_dir).map_err(|error| error.to_string())?;
    let request = str_arg(args, "request")?;
    let session_id = opt_str_arg(args, "session_id")?;
    let project_key =
        opt_str_arg(args, "project_key")?.unwrap_or_else(|| "consultations".to_owned());
    let context = ctx.clone();
    Ok(spawn_job(&ctx.jobs, "cycle_role", async move {
        run_role_call(context, config, operation, request, session_id, project_key).await
    })
    .await)
}

async fn run_role_call(
    ctx: ToolContext,
    config: RolesFile,
    operation: RoleOperation,
    request: String,
    session_id: Option<String>,
    project_key: String,
) -> Result<Value, String> {
    let role_name = operation.role();
    let endpoint = config
        .endpoint(role_name)
        .ok_or_else(|| format!("role '{role_name}' has no configured endpoint"))?;
    let provider = endpoint
        .base_url_host()
        .unwrap_or_else(|| "unknown".to_owned());
    let model = endpoint.model_id.clone();
    let (output, usage) = match operation {
        RoleOperation::ArchitectConsult | RoleOperation::ArbiterReadiness => {
            let call = ctx
                .roles
                .consult(&config, &ctx.data_dir, operation, &request, &ctx.usage)
                .await?;
            (
                json!({"advisory": call.result, "binding": false}),
                call.usage,
            )
        }
        RoleOperation::FunctionalReview | RoleOperation::SecurityReview => {
            let call = ctx
                .roles
                .review(&config, &ctx.data_dir, operation, &request, &ctx.usage)
                .await?;
            match call.result {
                ReviewOutput::Binding(verdict) => {
                    (json!({"binding": true, "verdict": verdict}), call.usage)
                }
                ReviewOutput::Advisory(value) => {
                    (json!({"advisory": value, "binding": false}), call.usage)
                }
            }
        }
        RoleOperation::ArbiterVerdict => {
            let call = ctx
                .roles
                .arbitration(&config, &ctx.data_dir, &request, &ctx.usage)
                .await?;
            (json!({"binding": true, "verdict": call.result}), call.usage)
        }
        RoleOperation::ExecutorFeasibility => {
            return Err("executor consultations run inside the Trae Work session".to_owned());
        }
    };
    record_role_audit(
        &ctx,
        RoleAuditRecord {
            operation,
            provider: &provider,
            model: &model,
            output: &output,
            usage,
            session_id,
            project_key: &project_key,
        },
    )
    .await?;
    Ok(json!({
        "operation": operation.as_str(),
        "output": output,
        "role": role_name,
    }))
}

struct RoleAuditRecord<'a> {
    operation: RoleOperation,
    provider: &'a str,
    model: &'a str,
    output: &'a Value,
    usage: Option<CompletionUsage>,
    session_id: Option<String>,
    project_key: &'a str,
}

async fn record_role_audit(ctx: &ToolContext, record: RoleAuditRecord<'_>) -> Result<(), String> {
    ctx.daemon.ensure().await?;
    let mut metadata = BTreeMap::new();
    metadata.insert("operation".to_owned(), record.operation.as_str().to_owned());
    if let Some(usage) = record.usage {
        metadata.insert("promptTokens".to_owned(), usage.prompt_tokens.to_string());
        metadata.insert(
            "completionTokens".to_owned(),
            usage.completion_tokens.to_string(),
        );
        metadata.insert("totalTokens".to_owned(), usage.total_tokens.to_string());
    }
    let observation = AuditObservation {
        actor_id: "trae-work-session".to_owned(),
        candidate_id: None,
        data: AuditData::Tool {
            invocation_digest: ContentDigest::of(record.output.to_string().as_bytes()),
            tool: "cycle_role".to_owned(),
        },
        evidence_ids: BTreeSet::new(),
        files: BTreeSet::new(),
        metadata,
        model: Some(AuditModel {
            model: record.model.to_owned(),
            provider: record.provider.to_owned(),
        }),
        project_key: record.project_key.to_owned(),
        role: Some(record.operation.workflow_role()),
        session_id: record.session_id,
        task_id: None,
        timestamp_unix_millis: now_unix_millis(),
        workflow_id: None,
    };
    let message = ClientMessage::Audit {
        request_id: ctx.daemon.next_request_id(),
        observation,
    };
    match ctx.daemon.exchange(message).await? {
        ServerMessage::AuditRecorded { .. } => Ok(()),
        _ => Err("control plane returned an unexpected audit response".to_owned()),
    }
}

async fn goal_create(ctx: &ToolContext, args: &Value) -> Result<Value, String> {
    let operation = GoalOperation::Create {
        constraints: string_list(args, "constraints"),
        goal_id: GoalId::new(),
        max_continuations: args
            .get("max_continuations")
            .and_then(Value::as_u64)
            .unwrap_or(5)
            .clamp(1, 255) as u8,
        non_goals: string_list(args, "non_goals"),
        objective: str_arg(args, "objective")?,
        session_id: str_arg(args, "session_id")?,
        success_criteria: {
            let criteria = string_list(args, "success_criteria");
            if criteria.is_empty() {
                return Err("success_criteria must not be empty".to_owned());
            }
            criteria
        },
    };
    goal_op(ctx, args, operation).await
}

async fn goal_control(ctx: &ToolContext, args: &Value) -> Result<Value, String> {
    let action: GoalControlAction = serde_json::from_value(json!(str_arg(args, "action")?))
        .map_err(|error| format!("action is invalid: {error}"))?;
    let completion_evidence = match opt_str_arg(args, "completion_evidence")? {
        Some(value) => Some(
            ContentDigest::from_str(&value)
                .map_err(|error| format!("completion_evidence is invalid: {error}"))?,
        ),
        None => None,
    };
    let operation = GoalOperation::Control {
        action,
        completion_evidence,
        goal_id: id_arg(args, "goal_id")?,
        operation_id: ReceiptId::new(),
        reason: opt_str_arg(args, "reason")?,
    };
    goal_op(ctx, args, operation).await
}

async fn goal_op(
    ctx: &ToolContext,
    args: &Value,
    operation: GoalOperation,
) -> Result<Value, String> {
    let project_key = str_arg(args, "project_key")?;
    let message = ClientMessage::Goal {
        operation,
        project_key,
        request_id: ctx.daemon.next_request_id(),
    };
    ctx.daemon.ensure().await?;
    match ctx.daemon.exchange(message).await? {
        ServerMessage::Goal { result, .. } => Ok(result),
        _ => Err("control plane returned an unexpected goal response".to_owned()),
    }
}

async fn memory_op(
    ctx: &ToolContext,
    args: &Value,
    operation: MemoryOperation,
) -> Result<Value, String> {
    let project_key = str_arg(args, "project_key")?;
    let message = ClientMessage::Memory {
        operation,
        project_key,
        request_id: ctx.daemon.next_request_id(),
    };
    match ctx.daemon.exchange(message).await? {
        ServerMessage::Memory { result, .. } => Ok(result),
        _ => Err("control plane returned an unexpected memory response".to_owned()),
    }
}

async fn history(ctx: &ToolContext, args: &Value) -> Result<Value, String> {
    let project_key = str_arg(args, "project_key")?;
    let operation = HistoryOperation::Query {
        after_sequence: args.get("after_sequence").and_then(Value::as_u64),
        limit: args
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(50)
            .clamp(1, 500) as usize,
    };
    history_op(ctx, &project_key, operation).await
}

async fn history_verify(ctx: &ToolContext, args: &Value) -> Result<Value, String> {
    let project_key = str_arg(args, "project_key")?;
    history_op(ctx, &project_key, HistoryOperation::Verify).await
}

async fn export(ctx: &ToolContext, args: &Value) -> Result<Value, String> {
    require_confirm(args)?;
    let project_key = str_arg(args, "project_key")?;
    history_op(ctx, &project_key, HistoryOperation::Export).await
}

async fn history_op(
    ctx: &ToolContext,
    project_key: &str,
    operation: HistoryOperation,
) -> Result<Value, String> {
    let message = ClientMessage::History {
        operation,
        project_key: project_key.to_owned(),
        request_id: ctx.daemon.next_request_id(),
    };
    match ctx.daemon.exchange(message).await? {
        ServerMessage::History { result, .. } => Ok(result),
        _ => Err("control plane returned an unexpected history response".to_owned()),
    }
}

async fn models(ctx: &ToolContext) -> Result<Value, String> {
    let mut report = match roles::load(&ctx.data_dir) {
        Ok(file) => file.report(),
        Err(_) => roles::missing_report(),
    };
    let usage = ctx.usage.snapshot().await;
    if let Some(object) = report.as_object_mut() {
        object.insert("usage".to_owned(), usage);
    }
    Ok(report)
}

async fn limits(ctx: &ToolContext) -> Result<Value, String> {
    ctx.daemon.ensure().await?;
    control(ctx, "limits", ControlOperation::Limits, None).await
}

async fn spawn_job<F>(jobs: &Arc<Jobs>, tool: &str, work: F) -> Value
where
    F: std::future::Future<Output = Result<Value, String>> + Send + 'static,
{
    let job_id = uuid::Uuid::now_v7().to_string();
    jobs.register(job_id.clone(), tool).await;
    let tracker = Arc::clone(jobs);
    let tracked = job_id.clone();
    tokio::spawn(async move {
        let outcome = work.await;
        tracker.finish(&tracked, outcome).await;
    });
    json!({
        "jobId": job_id,
        "note": "observe completion with cycle_status",
        "state": "running",
    })
}

fn str_arg(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("{key} is required"))
}

fn opt_str_arg(args: &Value, key: &str) -> Result<Option<String>, String> {
    match args.get(key).and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => Ok(Some(value.to_owned())),
        _ => Ok(None),
    }
}

fn id_arg<T>(args: &Value, key: &str) -> Result<T, String>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    str_arg(args, key)?
        .parse()
        .map_err(|error| format!("{key} is invalid: {error}"))
}

fn opt_id_arg<T>(args: &Value, key: &str) -> Result<Option<T>, String>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    match opt_str_arg(args, key)? {
        Some(value) => value
            .parse()
            .map(Some)
            .map_err(|error| format!("{key} is invalid: {error}")),
        None => Ok(None),
    }
}

fn string_list(args: &Value, key: &str) -> Vec<String> {
    args.get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn require_confirm(args: &Value) -> Result<(), String> {
    match args.get("confirm").and_then(Value::as_bool) {
        Some(true) => Ok(()),
        _ => Err("this command requires confirm: true after explicit user approval".to_owned()),
    }
}

fn now_unix_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{ToolContext, call, descriptors};
    use crate::{daemon::Daemon, jobs::Jobs};
    use serde_json::json;
    use workflow_roles::{CONSULT_TIMEOUT, RolesClient, UsageLedger};

    fn context() -> ToolContext {
        ToolContext {
            daemon: Daemon::new(std::env::temp_dir().join("trae-cycle-test")),
            data_dir: std::env::temp_dir().join("trae-cycle-test"),
            jobs: std::sync::Arc::new(Jobs::new()),
            roles: RolesClient::new(CONSULT_TIMEOUT),
            usage: std::sync::Arc::new(UsageLedger::new()),
        }
    }

    #[tokio::test]
    async fn executor_consultations_stay_in_session() {
        let context = context();
        let error = call(
            &context,
            "cycle_role",
            &json!({"operation": "executor_feasibility", "request": "can we ship this", "role": "executor"}),
        )
        .await
        .unwrap_err();
        assert!(error.contains("Trae Work session"));
    }

    #[tokio::test]
    async fn role_operations_require_the_paired_role() {
        let context = context();
        let error = call(
            &context,
            "cycle_role",
            &json!({"operation": "security_review", "request": "review", "role": "architect"}),
        )
        .await
        .unwrap_err();
        assert!(error.contains("requires role security_reviewer"));
    }

    #[tokio::test]
    async fn remote_consultations_fail_closed_without_roles_file() {
        let context = context();
        let error = call(
            &context,
            "cycle_role",
            &json!({"operation": "architect_consult", "request": "design", "role": "architect"}),
        )
        .await
        .unwrap_err();
        assert!(error.contains("roles.json is missing"));
    }

    #[test]
    fn descriptor_names_are_unique_and_dispatch_is_exhaustive() {
        let names = descriptors()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(names.len(), sorted.len());
        assert_eq!(names.len(), 36);
    }

    #[tokio::test]
    async fn cancel_and_export_refuse_without_confirmation() {
        let context = context();
        for tool in ["cycle_cancel", "cycle_export"] {
            let error = call(&context, tool, &json!({"project_key": "p"}))
                .await
                .unwrap_err();
            assert!(error.contains("confirm"));
        }
    }

    #[tokio::test]
    async fn unknown_tool_is_rejected() {
        let context = context();
        assert!(call(&context, "cycle_nope", &json!({})).await.is_err());
    }
}
