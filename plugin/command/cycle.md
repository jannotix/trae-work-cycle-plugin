# Command: cycle

Create this command once in TRAE Work: Settings → Commands → Create, environment **Local** (desktop). Use exactly these values.

- **Command Name**: `cycle`
- **Description**: Route Cycle operations: run, status, tasks, evidence, pause, resume, retry, cancel, goal, memory, history, models, limits, setup, doctor, export, help.

## Instructions

Load the `cycle-delivery` skill before executing any governed operation. The control plane is the authority; relay its results verbatim.

Parse the command text after `/cycle`. The subcommand is the first token, with a leading colon accepted (`/cycle:resume` equals `/cycle resume`). Everything after the first token is the subcommand's argument text.

- **run [auto|quick|full]** — Arm a mode (default `auto`). Confirm the armed mode, then tell the user the next message becomes the immutable original request. On the next non-command message, start the cycle with `cycle_start` and follow the skill's phase protocol.
- **status** — `cycle_status` for the current project; relay state, repair budget and job results.
- **tasks** — `cycle_tasks`; relay the task list with states and dependencies.
- **evidence** — `cycle_evidence` with the user's described evidence, or summarize registered evidence through `cycle_status`.
- **pause** / **resume** — `cycle_pause` / `cycle_resume`.
- **retry** — `cycle_retry` for a classified transient failure.
- **cancel --confirm** — Requires the literal `--confirm` flag. Without it, ask the user to confirm explicitly first. Then `cycle_cancel` with `confirm: true`.
- **goal ...** — Goal operations: map the remaining text to `cycle_goal_create`, `cycle_goal_amend`, `cycle_goal_status`, `cycle_goal_list`, `cycle_goal_focus`, `cycle_goal_save_plan`, `cycle_goal_link`, `cycle_goal_control`. When the intent is ambiguous, show the goal with `cycle_goal_status` and ask.
- **memory ...** — Memory operations: `cycle_memory_search`, `cycle_memory_explain`. Removal requires explicit user confirmation, then `cycle_memory_remove` with `confirm: true`.
- **history [verify]** — `cycle_history`; with `verify`, `cycle_history_verify`. On verification failure stop and preserve data.
- **models** — `cycle_models`: effective role assignments and usage totals, no secrets.
- **limits** — `cycle_limits`: admission policy and resource reserves.
- **permissions** — `cycle_doctor`: effective configuration and control plane state; permission-relevant facts live there and in `cycle_limits`.
- **setup** — `cycle_setup`; relay any configuration fix needed before cycles can start.
- **doctor** — `cycle_doctor`; explain each issue in plain language.
- **export --confirm** — Requires the literal `--confirm` flag and explicit user approval; then `cycle_export` with `confirm: true`.
- **help** or no subcommand — Reply with this list, one line per command, and note that all operations are also invoked automatically by the skill when needed.

Unknown subcommand: say so and show the help list. Destructive operations (`cancel`, `export`, memory removal) never proceed without explicit confirmation in the same conversation.
