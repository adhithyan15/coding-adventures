//! # os-job-core
//!
//! `os-job-core` is the repository-owned contract for scheduled work.
//!
//! The key design idea is that job authors describe *intent* once in a portable
//! [`JobSpec`], and backend crates decide how that intent maps onto native
//! operating-system schedulers such as `launchd`, `systemd --user`, and Windows
//! Task Scheduler.
//!
//! ## Why a repository-owned schema?
//!
//! Raw cron strings are compact, but they leak backend details into every call
//! site. That makes cross-platform behavior hard to reason about:
//!
//! ```text
//! cron:          "*/5 * * * *"
//! launchd:       StartInterval = 300
//! systemd:       OnUnitActiveSec = 300
//! task scheduler: Repetition.Interval = PT5M
//! ```
//!
//! By normalizing everything into a single Rust type, the rest of the Chief of
//! Staff stack can ask a simpler question:
//!
//! ```text
//! "What job should run, and when should it run?"
//! ```
//!
//! rather than:
//!
//! ```text
//! "Which scheduler syntax does this machine need?"
//! ```
//!
//! ## Layers in miniature
//!
//! ```text
//! JobSpec  ──► backend validation ──► InstallPlan ──► OS-specific installer
//! ```
//!
//! `os-job-core` owns the first three nouns in that sentence.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Milliseconds since the Unix epoch.
pub type TimestampMs = u64;

// ============================================================================
// BackendKind
// ============================================================================

/// The native scheduler family that a backend targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    /// macOS per-user jobs managed by `launchd`.
    Launchd,
    /// Linux per-user jobs managed by `systemd --user`.
    SystemdUser,
    /// Windows jobs managed by Task Scheduler.
    WindowsTaskScheduler,
    /// Pure in-process fallback runtime.
    InProcess,
}

impl BackendKind {
    /// Return the repository-owned wire name for the backend.
    pub fn as_str(self) -> &'static str {
        match self {
            BackendKind::Launchd => "launchd",
            BackendKind::SystemdUser => "systemd-user",
            BackendKind::WindowsTaskScheduler => "windows-task",
            BackendKind::InProcess => "in-process",
        }
    }
}

impl Display for BackendKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// JobSpec
// ============================================================================

/// A portable description of one scheduled unit of work.
///
/// The struct intentionally stays close to the D18C spec:
///
/// ```text
/// JobSpec
/// |-- job_id
/// |-- name
/// |-- description
/// |-- action
/// |-- trigger
/// |-- concurrency_policy
/// |-- retry_policy
/// |-- timeout_seconds
/// |-- env
/// |-- working_directory?
/// |-- output_policy
/// |-- enabled
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSpec {
    pub job_id: String,
    pub name: String,
    pub description: String,
    pub action: JobAction,
    pub trigger: JobTrigger,
    pub concurrency_policy: ConcurrencyPolicy,
    pub retry_policy: RetryPolicy,
    pub timeout_seconds: Option<u32>,
    pub env: Vec<EnvironmentEntry>,
    pub working_directory: Option<String>,
    pub output_policy: OutputPolicy,
    pub enabled: bool,
}

impl JobSpec {
    /// Validate the portable parts of the job spec.
    pub fn validate(&self) -> ValidationResult {
        validate_job_spec(self)
    }
}

// ============================================================================
// JobAction
// ============================================================================

/// What the scheduler should invoke when the trigger fires.
///
/// Native schedulers ultimately need a concrete executable. Command actions
/// already have one. `agent_run` and `function` actions resolve through
/// repository-owned shims so the higher layers can stay modelled in terms of
/// Chief of Staff concepts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobAction {
    /// Execute a program directly.
    Command {
        program: String,
        args: Vec<String>,
        input: Option<String>,
    },
    /// Invoke the Chief of Staff agent-run shim.
    AgentRun {
        agent_id: String,
        args: Vec<String>,
        input: Option<String>,
    },
    /// Invoke the Chief of Staff function-run shim.
    Function {
        function_id: String,
        args: Vec<String>,
        input: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobActionKind {
    Command,
    AgentRun,
    Function,
}

impl JobActionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::AgentRun => "agent_run",
            Self::Function => "function",
        }
    }
}

impl Display for JobActionKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl JobAction {
    /// Return the typed action kind.
    pub fn kind(&self) -> JobActionKind {
        match self {
            JobAction::Command { .. } => JobActionKind::Command,
            JobAction::AgentRun { .. } => JobActionKind::AgentRun,
            JobAction::Function { .. } => JobActionKind::Function,
        }
    }

    /// Return the repository-owned action kind string.
    pub fn kind_name(&self) -> &'static str {
        self.kind().as_str()
    }

    /// Return whether the action requests stdin input.
    pub fn has_input(&self) -> bool {
        match self {
            JobAction::Command { input, .. }
            | JobAction::AgentRun { input, .. }
            | JobAction::Function { input, .. } => input.is_some(),
        }
    }

    /// Convert the portable action into an executable command line.
    ///
    /// The backend layer can render this into plist XML, systemd unit files, or
    /// Task Scheduler XML without needing to know about higher-level agent
    /// concepts.
    pub fn command_line(&self) -> CommandLine {
        match self {
            JobAction::Command { program, args, .. } => CommandLine {
                program: program.clone(),
                args: args.clone(),
            },
            JobAction::AgentRun { agent_id, args, .. } => {
                let mut command_args = vec!["--agent-id".to_string(), agent_id.clone()];
                command_args.extend(args.clone());
                CommandLine {
                    program: "chief-of-staff-agent-runner".to_string(),
                    args: command_args,
                }
            }
            JobAction::Function {
                function_id, args, ..
            } => {
                let mut command_args = vec!["--function-id".to_string(), function_id.clone()];
                command_args.extend(args.clone());
                CommandLine {
                    program: "chief-of-staff-function-runner".to_string(),
                    args: command_args,
                }
            }
        }
    }
}

/// A concrete executable plus arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLine {
    pub program: String,
    pub args: Vec<String>,
}

// ============================================================================
// JobTrigger
// ============================================================================

/// When the scheduler should run the job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobTrigger {
    Once {
        at: DateTimeParts,
    },
    Interval {
        every_seconds: u32,
        anchor: Option<DateTimeParts>,
    },
    Daily {
        hour: u8,
        minute: u8,
    },
    Weekly {
        days: Vec<Weekday>,
        hour: u8,
        minute: u8,
    },
    Monthly {
        day: u8,
        hour: u8,
        minute: u8,
    },
    AtLogin,
    AtBoot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobTriggerKind {
    Once,
    Interval,
    Daily,
    Weekly,
    Monthly,
    AtLogin,
    AtBoot,
}

impl JobTriggerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Once => "once",
            Self::Interval => "interval",
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
            Self::AtLogin => "at_login",
            Self::AtBoot => "at_boot",
        }
    }
}

impl Display for JobTriggerKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl JobTrigger {
    /// Return the typed trigger kind.
    pub fn kind(&self) -> JobTriggerKind {
        match self {
            JobTrigger::Once { .. } => JobTriggerKind::Once,
            JobTrigger::Interval { .. } => JobTriggerKind::Interval,
            JobTrigger::Daily { .. } => JobTriggerKind::Daily,
            JobTrigger::Weekly { .. } => JobTriggerKind::Weekly,
            JobTrigger::Monthly { .. } => JobTriggerKind::Monthly,
            JobTrigger::AtLogin => JobTriggerKind::AtLogin,
            JobTrigger::AtBoot => JobTriggerKind::AtBoot,
        }
    }

    /// Return the repository-owned trigger kind string.
    pub fn kind_name(&self) -> &'static str {
        self.kind().as_str()
    }
}

/// Days of the week in a portable order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    /// Three-letter form used by systemd calendar expressions.
    pub fn systemd_name(self) -> &'static str {
        match self {
            Weekday::Monday => "Mon",
            Weekday::Tuesday => "Tue",
            Weekday::Wednesday => "Wed",
            Weekday::Thursday => "Thu",
            Weekday::Friday => "Fri",
            Weekday::Saturday => "Sat",
            Weekday::Sunday => "Sun",
        }
    }

    /// Integer expected by `launchd` calendar intervals.
    ///
    /// `launchd` uses `0` and `7` for Sunday. We choose `0` to keep the mapping
    /// single-valued.
    pub fn launchd_weekday(self) -> u8 {
        match self {
            Weekday::Sunday => 0,
            Weekday::Monday => 1,
            Weekday::Tuesday => 2,
            Weekday::Wednesday => 3,
            Weekday::Thursday => 4,
            Weekday::Friday => 5,
            Weekday::Saturday => 6,
        }
    }

    /// XML tag name used by Windows Task Scheduler weekly triggers.
    pub fn windows_tag(self) -> &'static str {
        match self {
            Weekday::Monday => "Monday",
            Weekday::Tuesday => "Tuesday",
            Weekday::Wednesday => "Wednesday",
            Weekday::Thursday => "Thursday",
            Weekday::Friday => "Friday",
            Weekday::Saturday => "Saturday",
            Weekday::Sunday => "Sunday",
        }
    }
}

/// A timezone-free local timestamp.
///
/// Native schedulers generally interpret scheduled times in the local machine's
/// timezone, so the portable spec does the same.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTimeParts {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl DateTimeParts {
    /// Render as an ISO-8601 local timestamp without a timezone suffix.
    pub fn to_iso8601_local(self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }

    /// Render as the `systemd` calendar format used by `OnCalendar=`.
    pub fn to_systemd_calendar(self) -> String {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

// ============================================================================
// Supporting policies
// ============================================================================

/// What to do if a new run is due while an earlier run is still executing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcurrencyPolicy {
    /// Allow multiple runs at the same time.
    Allow,
    /// Skip the new run.
    Skip,
    /// Replace the old run with the new one.
    Replace,
}

/// Retry behavior after a failed run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_backoff_seconds: u32,
    pub max_backoff_seconds: Option<u32>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 0,
            initial_backoff_seconds: 60,
            max_backoff_seconds: None,
        }
    }
}

/// Environment variables supplied to the job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentEntry {
    pub key: String,
    pub value: String,
}

/// Where stdout and stderr should go.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OutputPolicy {
    pub stdout_path: Option<String>,
    pub stderr_path: Option<String>,
    pub append: bool,
}

// ============================================================================
// Installed jobs and run observability
// ============================================================================

/// A job after it has been installed into one scheduler backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledJob {
    pub job_id: String,
    pub backend: BackendKind,
    pub spec: JobSpec,
    pub installed_at: TimestampMs,
    pub native_identifier: Option<String>,
    pub enabled: bool,
}

impl InstalledJob {
    /// Build an installed-job record from the portable spec and backend.
    pub fn new(
        backend: BackendKind,
        spec: JobSpec,
        installed_at: TimestampMs,
        native_identifier: Option<String>,
    ) -> Self {
        Self {
            job_id: spec.job_id.clone(),
            enabled: spec.enabled,
            backend,
            spec,
            installed_at,
            native_identifier,
        }
    }

    /// Validate the portable installed-job metadata.
    pub fn validate(&self) -> ValidationResult {
        let mut result = self.spec.validate();
        if self.job_id != self.spec.job_id {
            result.push_error("job_id", "must match spec.job_id");
        }
        if let Some(native_identifier) = &self.native_identifier {
            validate_non_empty("native_identifier", native_identifier, &mut result);
            validate_single_line("native_identifier", native_identifier, &mut result);
        }
        result
    }
}

/// High-level status returned by a job runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatusKind {
    Missing,
    Installed,
    Running,
    Disabled,
    Failed,
}

impl JobStatusKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Installed => "installed",
            Self::Running => "running",
            Self::Disabled => "disabled",
            Self::Failed => "failed",
        }
    }
}

impl Display for JobStatusKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Runtime status for one installed job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobStatus {
    pub job_id: String,
    pub backend: BackendKind,
    pub status: JobStatusKind,
    pub enabled: bool,
    pub last_run: Option<JobRunReceipt>,
    pub next_run_hint: Option<DateTimeParts>,
}

impl JobStatus {
    pub fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::new();
        validate_identifier("job_id", &self.job_id, &mut result);
        if let Some(receipt) = &self.last_run {
            let receipt_result = receipt.validate();
            for message in receipt_result.errors {
                result.push_error(format!("last_run.{}", message.field), message.message);
            }
            for message in receipt_result.warnings {
                result.push_warning(format!("last_run.{}", message.field), message.message);
            }
            if receipt.job_id != self.job_id {
                result.push_error("last_run.job_id", "must match job_id");
            }
        }
        if let Some(next_run_hint) = self.next_run_hint {
            validate_datetime("next_run_hint", next_run_hint, &mut result);
        }
        result
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstalledJobSort {
    JobId,
    Name,
    BackendThenJobId,
    InstalledAtAsc,
    InstalledAtDesc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledJobQuery {
    pub job_ids: Vec<String>,
    pub backends: Vec<BackendKind>,
    pub action_kinds: Vec<JobActionKind>,
    pub trigger_kinds: Vec<JobTriggerKind>,
    pub enabled: Option<bool>,
    pub installed_at_or_after: Option<TimestampMs>,
    pub installed_at_or_before: Option<TimestampMs>,
    pub sort: InstalledJobSort,
    pub limit: Option<usize>,
}

impl Default for InstalledJobQuery {
    fn default() -> Self {
        Self {
            job_ids: Vec::new(),
            backends: Vec::new(),
            action_kinds: Vec::new(),
            trigger_kinds: Vec::new(),
            enabled: None,
            installed_at_or_after: None,
            installed_at_or_before: None,
            sort: InstalledJobSort::JobId,
            limit: None,
        }
    }
}

impl InstalledJobQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_job_id(mut self, job_id: impl Into<String>) -> Self {
        self.job_ids.push(job_id.into());
        self
    }

    pub fn with_backend(mut self, backend: BackendKind) -> Self {
        self.backends.push(backend);
        self
    }

    pub fn with_action_kind(mut self, action_kind: JobActionKind) -> Self {
        self.action_kinds.push(action_kind);
        self
    }

    pub fn with_trigger_kind(mut self, trigger_kind: JobTriggerKind) -> Self {
        self.trigger_kinds.push(trigger_kind);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }

    pub fn installed_at_or_after(mut self, timestamp_ms: TimestampMs) -> Self {
        self.installed_at_or_after = Some(timestamp_ms);
        self
    }

    pub fn installed_at_or_before(mut self, timestamp_ms: TimestampMs) -> Self {
        self.installed_at_or_before = Some(timestamp_ms);
        self
    }

    pub fn sorted_by(mut self, sort: InstalledJobSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn limited_to(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn matches_job(&self, job: &InstalledJob) -> bool {
        if !matches_any(&self.job_ids, &job.job_id) {
            return false;
        }
        if !matches_any(&self.backends, &job.backend) {
            return false;
        }
        if !matches_any(&self.action_kinds, &job.spec.action.kind()) {
            return false;
        }
        if !matches_any(&self.trigger_kinds, &job.spec.trigger.kind()) {
            return false;
        }
        if let Some(enabled) = self.enabled {
            if job.enabled != enabled {
                return false;
            }
        }
        if let Some(installed_at_or_after) = self.installed_at_or_after {
            if job.installed_at < installed_at_or_after {
                return false;
            }
        }
        if let Some(installed_at_or_before) = self.installed_at_or_before {
            if job.installed_at > installed_at_or_before {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatusSort {
    JobId,
    BackendThenJobId,
    StatusThenJobId,
    LastRunFinishedDesc,
    NextRunThenJobId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobStatusQuery {
    pub job_ids: Vec<String>,
    pub backends: Vec<BackendKind>,
    pub statuses: Vec<JobStatusKind>,
    pub enabled: Option<bool>,
    pub has_last_run: Option<bool>,
    pub last_run_outcomes: Vec<JobRunOutcome>,
    pub next_run_at_or_before: Option<DateTimeParts>,
    pub sort: JobStatusSort,
    pub limit: Option<usize>,
}

impl Default for JobStatusQuery {
    fn default() -> Self {
        Self {
            job_ids: Vec::new(),
            backends: Vec::new(),
            statuses: Vec::new(),
            enabled: None,
            has_last_run: None,
            last_run_outcomes: Vec::new(),
            next_run_at_or_before: None,
            sort: JobStatusSort::JobId,
            limit: None,
        }
    }
}

impl JobStatusQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_job_id(mut self, job_id: impl Into<String>) -> Self {
        self.job_ids.push(job_id.into());
        self
    }

    pub fn with_backend(mut self, backend: BackendKind) -> Self {
        self.backends.push(backend);
        self
    }

    pub fn with_status(mut self, status: JobStatusKind) -> Self {
        self.statuses.push(status);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }

    pub fn has_last_run(mut self, has_last_run: bool) -> Self {
        self.has_last_run = Some(has_last_run);
        self
    }

    pub fn with_last_run_outcome(mut self, outcome: JobRunOutcome) -> Self {
        self.last_run_outcomes.push(outcome);
        self
    }

    pub fn next_run_at_or_before(mut self, datetime: DateTimeParts) -> Self {
        self.next_run_at_or_before = Some(datetime);
        self
    }

    pub fn sorted_by(mut self, sort: JobStatusSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn limited_to(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn matches_status(&self, status: &JobStatus) -> bool {
        if !matches_any(&self.job_ids, &status.job_id) {
            return false;
        }
        if !matches_any(&self.backends, &status.backend) {
            return false;
        }
        if !matches_any(&self.statuses, &status.status) {
            return false;
        }
        if let Some(enabled) = self.enabled {
            if status.enabled != enabled {
                return false;
            }
        }
        if let Some(has_last_run) = self.has_last_run {
            if status.last_run.is_some() != has_last_run {
                return false;
            }
        }
        if !self.last_run_outcomes.is_empty()
            && !status.last_run.as_ref().is_some_and(|receipt| {
                self.last_run_outcomes
                    .contains(&receipt.exit_status.outcome)
            })
        {
            return false;
        }
        if let Some(cutoff) = self.next_run_at_or_before {
            if !status
                .next_run_hint
                .is_some_and(|next_run| datetime_le(next_run, cutoff))
            {
                return false;
            }
        }
        true
    }
}

pub fn query_installed_jobs<'a, I>(jobs: I, query: &InstalledJobQuery) -> Vec<&'a InstalledJob>
where
    I: IntoIterator<Item = &'a InstalledJob>,
{
    let mut results = jobs
        .into_iter()
        .filter(|job| query.matches_job(job))
        .collect::<Vec<_>>();

    sort_installed_job_results(&mut results, query.sort);
    if let Some(limit) = query.limit {
        results.truncate(limit);
    }

    results
}

pub fn query_job_statuses<'a, I>(statuses: I, query: &JobStatusQuery) -> Vec<&'a JobStatus>
where
    I: IntoIterator<Item = &'a JobStatus>,
{
    let mut results = statuses
        .into_iter()
        .filter(|status| query.matches_status(status))
        .collect::<Vec<_>>();

    sort_job_status_results(&mut results, query.sort);
    if let Some(limit) = query.limit {
        results.truncate(limit);
    }

    results
}

/// Coarse outcome of one job run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobRunOutcome {
    Succeeded,
    Failed,
    TimedOut,
    Cancelled,
}

impl JobRunOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::TimedOut => "timed_out",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn is_success(self) -> bool {
        self == Self::Succeeded
    }
}

impl Display for JobRunOutcome {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Portable exit status for one job run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobExitStatus {
    pub outcome: JobRunOutcome,
    pub code: Option<i32>,
}

impl JobExitStatus {
    pub fn succeeded(code: i32) -> Self {
        Self {
            outcome: JobRunOutcome::Succeeded,
            code: Some(code),
        }
    }

    pub fn failed(code: Option<i32>) -> Self {
        Self {
            outcome: JobRunOutcome::Failed,
            code,
        }
    }

    pub fn timed_out() -> Self {
        Self {
            outcome: JobRunOutcome::TimedOut,
            code: None,
        }
    }

    pub fn cancelled() -> Self {
        Self {
            outcome: JobRunOutcome::Cancelled,
            code: None,
        }
    }

    pub fn is_success(&self) -> bool {
        self.outcome.is_success()
    }
}

/// D18C receipt emitted after every attempted run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobRunReceipt {
    pub run_id: String,
    pub job_id: String,
    pub started_at: TimestampMs,
    pub finished_at: TimestampMs,
    pub exit_status: JobExitStatus,
    pub output_refs: Vec<String>,
    pub error: Option<String>,
}

impl JobRunReceipt {
    pub fn succeeded(
        run_id: impl Into<String>,
        job_id: impl Into<String>,
        started_at: TimestampMs,
        finished_at: TimestampMs,
        output_refs: Vec<String>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            job_id: job_id.into(),
            started_at,
            finished_at,
            exit_status: JobExitStatus::succeeded(0),
            output_refs,
            error: None,
        }
    }

    pub fn failed(
        run_id: impl Into<String>,
        job_id: impl Into<String>,
        started_at: TimestampMs,
        finished_at: TimestampMs,
        exit_status: JobExitStatus,
        error: impl Into<String>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            job_id: job_id.into(),
            started_at,
            finished_at,
            exit_status,
            output_refs: Vec::new(),
            error: Some(error.into()),
        }
    }

    pub fn duration_ms(&self) -> u64 {
        self.finished_at.saturating_sub(self.started_at)
    }

    pub fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::new();
        validate_identifier("run_id", &self.run_id, &mut result);
        validate_identifier("job_id", &self.job_id, &mut result);
        if self.finished_at < self.started_at {
            result.push_error("finished_at", "must be greater than or equal to started_at");
        }
        if self.exit_status.is_success() && self.error.is_some() {
            result.push_error("error", "must be absent when exit_status succeeded");
        }
        if !self.exit_status.is_success() {
            match self.error.as_deref() {
                Some(error) => {
                    validate_non_empty("error", error, &mut result);
                    validate_single_line("error", error, &mut result);
                }
                None => {
                    result.push_error("error", "must be present when exit_status did not succeed");
                }
            }
        }
        for output_ref in &self.output_refs {
            validate_run_ref("output_refs", output_ref, &mut result);
        }
        result
    }
}

fn sort_installed_job_results(jobs: &mut Vec<&InstalledJob>, sort: InstalledJobSort) {
    match sort {
        InstalledJobSort::JobId => jobs.sort_by(|left, right| left.job_id.cmp(&right.job_id)),
        InstalledJobSort::Name => jobs.sort_by(|left, right| {
            left.spec
                .name
                .cmp(&right.spec.name)
                .then_with(|| left.job_id.cmp(&right.job_id))
        }),
        InstalledJobSort::BackendThenJobId => jobs.sort_by(|left, right| {
            backend_rank(left.backend)
                .cmp(&backend_rank(right.backend))
                .then_with(|| left.job_id.cmp(&right.job_id))
        }),
        InstalledJobSort::InstalledAtAsc => jobs.sort_by(|left, right| {
            left.installed_at
                .cmp(&right.installed_at)
                .then_with(|| left.job_id.cmp(&right.job_id))
        }),
        InstalledJobSort::InstalledAtDesc => jobs.sort_by(|left, right| {
            right
                .installed_at
                .cmp(&left.installed_at)
                .then_with(|| left.job_id.cmp(&right.job_id))
        }),
    }
}

fn sort_job_status_results(statuses: &mut Vec<&JobStatus>, sort: JobStatusSort) {
    match sort {
        JobStatusSort::JobId => statuses.sort_by(|left, right| left.job_id.cmp(&right.job_id)),
        JobStatusSort::BackendThenJobId => statuses.sort_by(|left, right| {
            backend_rank(left.backend)
                .cmp(&backend_rank(right.backend))
                .then_with(|| left.job_id.cmp(&right.job_id))
        }),
        JobStatusSort::StatusThenJobId => statuses.sort_by(|left, right| {
            status_rank(left.status)
                .cmp(&status_rank(right.status))
                .then_with(|| left.job_id.cmp(&right.job_id))
        }),
        JobStatusSort::LastRunFinishedDesc => statuses.sort_by(|left, right| {
            right
                .last_run
                .as_ref()
                .map(|receipt| receipt.finished_at)
                .unwrap_or(0)
                .cmp(
                    &left
                        .last_run
                        .as_ref()
                        .map(|receipt| receipt.finished_at)
                        .unwrap_or(0),
                )
                .then_with(|| left.job_id.cmp(&right.job_id))
        }),
        JobStatusSort::NextRunThenJobId => statuses.sort_by(|left, right| {
            compare_optional_datetime(left.next_run_hint, right.next_run_hint)
                .then_with(|| left.job_id.cmp(&right.job_id))
        }),
    }
}

fn matches_any<T: PartialEq>(needles: &[T], value: &T) -> bool {
    needles.is_empty() || needles.iter().any(|needle| needle == value)
}

fn backend_rank(backend: BackendKind) -> u8 {
    match backend {
        BackendKind::Launchd => 0,
        BackendKind::SystemdUser => 1,
        BackendKind::WindowsTaskScheduler => 2,
        BackendKind::InProcess => 3,
    }
}

fn status_rank(status: JobStatusKind) -> u8 {
    match status {
        JobStatusKind::Missing => 0,
        JobStatusKind::Installed => 1,
        JobStatusKind::Running => 2,
        JobStatusKind::Disabled => 3,
        JobStatusKind::Failed => 4,
    }
}

fn datetime_le(left: DateTimeParts, right: DateTimeParts) -> bool {
    compare_datetime(left, right) != Ordering::Greater
}

fn compare_optional_datetime(
    left: Option<DateTimeParts>,
    right: Option<DateTimeParts>,
) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => compare_datetime(left, right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_datetime(left: DateTimeParts, right: DateTimeParts) -> Ordering {
    datetime_key(left).cmp(&datetime_key(right))
}

fn datetime_key(datetime: DateTimeParts) -> (u16, u8, u8, u8, u8, u8) {
    (
        datetime.year,
        datetime.month,
        datetime.day,
        datetime.hour,
        datetime.minute,
        datetime.second,
    )
}

// ============================================================================
// InstallPlan
// ============================================================================

/// A deterministic plan that a higher layer can inspect before mutating the OS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallPlan {
    pub backend: BackendKind,
    pub files_to_write: Vec<InstallFile>,
    pub commands_to_run: Vec<InstallCommand>,
    pub permissions_needed: Vec<PermissionRequirement>,
}

/// One file that should be written as part of installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallFile {
    pub path: String,
    pub contents: String,
    pub mode: Option<u32>,
    pub reason: String,
}

/// One command that should run as part of installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallCommand {
    pub program: String,
    pub args: Vec<String>,
    pub description: String,
}

/// A human-readable permission requirement surfaced before installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRequirement {
    pub scope: String,
    pub detail: String,
}

// ============================================================================
// Validation
// ============================================================================

/// A validation report with both errors and non-fatal warnings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValidationResult {
    pub errors: Vec<ValidationMessage>,
    pub warnings: Vec<ValidationMessage>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_error(
        &mut self,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> &mut Self {
        self.errors.push(ValidationMessage {
            field: field.into(),
            message: message.into(),
        });
        self
    }

    pub fn push_warning(
        &mut self,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> &mut Self {
        self.warnings.push(ValidationMessage {
            field: field.into(),
            message: message.into(),
        });
        self
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn into_result(self) -> Result<(), JobError> {
        if self.is_valid() {
            Ok(())
        } else {
            Err(JobError::ValidationFailed(self))
        }
    }
}

/// One error or warning generated during validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationMessage {
    pub field: String,
    pub message: String,
}

impl Display for ValidationMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// A portability report describing whether a job can run across the current
/// repository-wide portability target.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PortabilityReport {
    pub issues: Vec<PortabilityIssue>,
}

impl PortabilityReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_issue(
        &mut self,
        field: impl Into<String>,
        message: impl Into<String>,
        unsupported_backends: Vec<BackendKind>,
    ) -> &mut Self {
        self.issues.push(PortabilityIssue {
            field: field.into(),
            message: message.into(),
            unsupported_backends,
        });
        self
    }

    pub fn is_portable(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn into_result(self) -> Result<(), JobError> {
        if self.is_portable() {
            Ok(())
        } else {
            Err(JobError::PortabilityValidationFailed(self))
        }
    }
}

/// One portability constraint violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortabilityIssue {
    pub field: String,
    pub message: String,
    pub unsupported_backends: Vec<BackendKind>,
}

impl Display for PortabilityIssue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)?;
        if !self.unsupported_backends.is_empty() {
            write!(f, " [unsupported on: ")?;
            for (index, backend) in self.unsupported_backends.iter().enumerate() {
                if index > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{backend}")?;
            }
            write!(f, "]")?;
        }
        Ok(())
    }
}

/// Validate the portable parts of a job spec.
pub fn validate_job_spec(spec: &JobSpec) -> ValidationResult {
    let mut result = ValidationResult::new();

    validate_identifier("job_id", &spec.job_id, &mut result);
    validate_non_empty("name", &spec.name, &mut result);
    validate_non_empty("description", &spec.description, &mut result);
    validate_single_line("name", &spec.name, &mut result);
    validate_single_line("description", &spec.description, &mut result);

    if let Some(timeout_seconds) = spec.timeout_seconds {
        if timeout_seconds == 0 {
            result.push_error("timeout_seconds", "must be greater than zero when set");
        }
    }

    if let Some(working_directory) = &spec.working_directory {
        validate_non_empty("working_directory", working_directory, &mut result);
        validate_single_line("working_directory", working_directory, &mut result);
    }

    validate_action(&spec.action, &mut result);
    validate_trigger(&spec.trigger, &mut result);
    validate_retry_policy(&spec.retry_policy, &mut result);
    validate_output_policy(&spec.output_policy, &mut result);
    validate_environment(&spec.env, &mut result);

    result
}

fn validate_action(action: &JobAction, result: &mut ValidationResult) {
    match action {
        JobAction::Command {
            program,
            args,
            input,
        } => {
            validate_non_empty("action.program", program, result);
            validate_single_line("action.program", program, result);
            for arg in args {
                validate_single_line("action.args", arg, result);
            }
            if let Some(input) = input {
                validate_non_empty("action.input", input, result);
            }
        }
        JobAction::AgentRun {
            agent_id,
            args,
            input,
        } => {
            validate_identifier("action.agent_id", agent_id, result);
            for arg in args {
                validate_single_line("action.args", arg, result);
            }
            if let Some(input) = input {
                validate_non_empty("action.input", input, result);
            }
        }
        JobAction::Function {
            function_id,
            args,
            input,
        } => {
            validate_identifier("action.function_id", function_id, result);
            for arg in args {
                validate_single_line("action.args", arg, result);
            }
            if let Some(input) = input {
                validate_non_empty("action.input", input, result);
            }
        }
    }
}

fn validate_trigger(trigger: &JobTrigger, result: &mut ValidationResult) {
    match trigger {
        JobTrigger::Once { at } => validate_datetime("trigger.once.at", *at, result),
        JobTrigger::Interval {
            every_seconds,
            anchor,
        } => {
            if *every_seconds == 0 {
                result.push_error("trigger.interval.every_seconds", "must be at least 1");
            }
            if let Some(anchor) = anchor {
                validate_datetime("trigger.interval.anchor", *anchor, result);
            }
        }
        JobTrigger::Daily { hour, minute } => {
            validate_clock("trigger.daily", *hour, *minute, 0, result);
        }
        JobTrigger::Weekly { days, hour, minute } => {
            if days.is_empty() {
                result.push_error("trigger.weekly.days", "must contain at least one weekday");
            }
            for window in days.windows(2) {
                if window[0] == window[1] {
                    result.push_error(
                        "trigger.weekly.days",
                        "must not contain duplicate weekdays next to each other",
                    );
                }
            }
            validate_clock("trigger.weekly", *hour, *minute, 0, result);
        }
        JobTrigger::Monthly { day, hour, minute } => {
            if !(1..=31).contains(day) {
                result.push_error("trigger.monthly.day", "must be between 1 and 31");
            }
            validate_clock("trigger.monthly", *hour, *minute, 0, result);
        }
        JobTrigger::AtLogin | JobTrigger::AtBoot => {}
    }
}

fn validate_retry_policy(policy: &RetryPolicy, result: &mut ValidationResult) {
    if policy.max_attempts > 0 && policy.initial_backoff_seconds == 0 {
        result.push_error(
            "retry_policy.initial_backoff_seconds",
            "must be greater than zero when retries are enabled",
        );
    }

    if let Some(max_backoff_seconds) = policy.max_backoff_seconds {
        if max_backoff_seconds == 0 {
            result.push_error(
                "retry_policy.max_backoff_seconds",
                "must be greater than zero when set",
            );
        }
        if max_backoff_seconds < policy.initial_backoff_seconds {
            result.push_error(
                "retry_policy.max_backoff_seconds",
                "must be greater than or equal to initial_backoff_seconds",
            );
        }
    }
}

fn validate_output_policy(policy: &OutputPolicy, result: &mut ValidationResult) {
    if let Some(stdout_path) = &policy.stdout_path {
        validate_non_empty("output_policy.stdout_path", stdout_path, result);
        validate_single_line("output_policy.stdout_path", stdout_path, result);
    }
    if let Some(stderr_path) = &policy.stderr_path {
        validate_non_empty("output_policy.stderr_path", stderr_path, result);
        validate_single_line("output_policy.stderr_path", stderr_path, result);
    }
}

fn validate_environment(entries: &[EnvironmentEntry], result: &mut ValidationResult) {
    let mut seen = std::collections::BTreeSet::new();

    for entry in entries {
        validate_env_key(&entry.key, result);
        if entry.value.contains('\0') {
            result.push_error("env.value", "must not contain NUL bytes");
        }
        if entry.value.contains('\n') || entry.value.contains('\r') {
            result.push_error("env.value", "must not contain carriage returns or newlines");
        }
        if !seen.insert(entry.key.clone()) {
            result.push_error(
                "env.key",
                format!("duplicate environment key `{}`", entry.key),
            );
        }
    }
}

fn validate_identifier(field: &str, value: &str, result: &mut ValidationResult) {
    validate_non_empty(field, value, result);
    if value
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-')))
    {
        result.push_error(
            field,
            "must contain only ASCII letters, digits, dots, underscores, or hyphens",
        );
    }
}

fn validate_run_ref(field: &str, value: &str, result: &mut ValidationResult) {
    validate_non_empty(field, value, result);
    validate_single_line(field, value, result);
}

fn validate_non_empty(field: &str, value: &str, result: &mut ValidationResult) {
    if value.trim().is_empty() {
        result.push_error(field, "must not be empty");
    }
}

fn validate_single_line(field: &str, value: &str, result: &mut ValidationResult) {
    if value.contains('\n') || value.contains('\r') {
        result.push_error(field, "must not contain carriage returns or newlines");
    }
}

fn validate_env_key(key: &str, result: &mut ValidationResult) {
    validate_non_empty("env.key", key, result);
    if key
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || ch == '_'))
    {
        result.push_error(
            "env.key",
            format!(
                "environment key `{}` must use only ASCII letters, digits, or underscores",
                key
            ),
        );
    }
}

fn validate_datetime(field_prefix: &str, datetime: DateTimeParts, result: &mut ValidationResult) {
    if datetime.year < 1970 {
        result.push_error(field_prefix, "year must be 1970 or later");
    }

    if !(1..=12).contains(&datetime.month) {
        result.push_error(field_prefix, "month must be between 1 and 12");
    }

    if !(1..=31).contains(&datetime.day) {
        result.push_error(field_prefix, "day must be between 1 and 31");
    }

    validate_clock(
        field_prefix,
        datetime.hour,
        datetime.minute,
        datetime.second,
        result,
    );
}

fn validate_clock(
    field_prefix: &str,
    hour: u8,
    minute: u8,
    second: u8,
    result: &mut ValidationResult,
) {
    if hour > 23 {
        result.push_error(field_prefix, "hour must be between 0 and 23");
    }
    if minute > 59 {
        result.push_error(field_prefix, "minute must be between 0 and 59");
    }
    if second > 59 {
        result.push_error(field_prefix, "second must be between 0 and 59");
    }
}

// ============================================================================
// Errors
// ============================================================================

/// Job-framework errors surfaced to higher layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobError {
    ValidationFailed(ValidationResult),
    PortabilityValidationFailed(PortabilityReport),
    UnsupportedTrigger {
        backend: BackendKind,
        trigger: String,
        reason: String,
    },
    UnsupportedAction {
        backend: BackendKind,
        action: String,
        reason: String,
    },
    UnsupportedFeature {
        backend: BackendKind,
        feature: String,
        reason: String,
    },
    UnsupportedPlatform(String),
}

impl Display for JobError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            JobError::ValidationFailed(validation) => {
                write!(f, "job spec validation failed")?;
                if !validation.errors.is_empty() {
                    write!(f, ": ")?;
                    for (index, message) in validation.errors.iter().enumerate() {
                        if index > 0 {
                            write!(f, "; ")?;
                        }
                        write!(f, "{message}")?;
                    }
                }
                Ok(())
            }
            JobError::PortabilityValidationFailed(report) => {
                write!(f, "job portability validation failed")?;
                if !report.issues.is_empty() {
                    write!(f, ": ")?;
                    for (index, issue) in report.issues.iter().enumerate() {
                        if index > 0 {
                            write!(f, "; ")?;
                        }
                        write!(f, "{issue}")?;
                    }
                }
                Ok(())
            }
            JobError::UnsupportedTrigger {
                backend,
                trigger,
                reason,
            } => write!(
                f,
                "backend `{backend}` does not support trigger `{trigger}`: {reason}"
            ),
            JobError::UnsupportedAction {
                backend,
                action,
                reason,
            } => write!(
                f,
                "backend `{backend}` does not support action `{action}`: {reason}"
            ),
            JobError::UnsupportedFeature {
                backend,
                feature,
                reason,
            } => write!(
                f,
                "backend `{backend}` does not support feature `{feature}`: {reason}"
            ),
            JobError::UnsupportedPlatform(message) => write!(f, "{message}"),
        }
    }
}

impl Error for JobError {}

// ============================================================================
// JobBackend
// ============================================================================

/// The contract each backend crate implements.
pub trait JobBackend {
    fn kind(&self) -> BackendKind;

    fn validate(&self, spec: &JobSpec) -> ValidationResult {
        spec.validate()
    }

    fn install_plan(&self, spec: &JobSpec) -> Result<InstallPlan, JobError>;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn example_job(trigger: JobTrigger) -> JobSpec {
        JobSpec {
            job_id: "memory-extract".to_string(),
            name: "Memory Extract".to_string(),
            description: "Extract durable memories from recent sessions".to_string(),
            action: JobAction::AgentRun {
                agent_id: "memory-extractor".to_string(),
                args: vec!["--scope".to_string(), "daily".to_string()],
                input: None,
            },
            trigger,
            concurrency_policy: ConcurrencyPolicy::Skip,
            retry_policy: RetryPolicy::default(),
            timeout_seconds: Some(600),
            env: vec![EnvironmentEntry {
                key: "COS_ENV".to_string(),
                value: "production".to_string(),
            }],
            working_directory: Some("/srv/chief-of-staff".to_string()),
            output_policy: OutputPolicy {
                stdout_path: Some("/tmp/memory-extract.out".to_string()),
                stderr_path: Some("/tmp/memory-extract.err".to_string()),
                append: true,
            },
            enabled: true,
        }
    }

    #[test]
    fn valid_job_spec_passes_validation() {
        let result = example_job(JobTrigger::Daily {
            hour: 3,
            minute: 15,
        })
        .validate();
        assert!(result.is_valid(), "expected valid job spec, got {result:?}");
    }

    #[test]
    fn invalid_job_id_is_rejected() {
        let mut job = example_job(JobTrigger::AtLogin);
        job.job_id = "bad job id".to_string();

        let result = job.validate();

        assert!(!result.is_valid());
        assert!(
            result
                .errors
                .iter()
                .any(|message| message.field == "job_id"),
            "expected job_id validation error, got {result:?}"
        );
    }

    #[test]
    fn env_values_reject_newlines() {
        let mut job = example_job(JobTrigger::AtLogin);
        job.env[0].value = "line-one\nline-two".to_string();

        let result = job.validate();

        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|message| message.field == "env.value"));
    }

    #[test]
    fn action_args_reject_newlines() {
        let mut job = example_job(JobTrigger::AtLogin);
        job.action = JobAction::Command {
            program: "/usr/local/bin/chief-of-staff".to_string(),
            args: vec!["digest\nrm -rf /".to_string()],
            input: None,
        };

        let result = job.validate();

        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|message| message.field == "action.args"));
    }

    #[test]
    fn weekly_trigger_rejects_empty_days() {
        let result = example_job(JobTrigger::Weekly {
            days: Vec::new(),
            hour: 9,
            minute: 30,
        })
        .validate();

        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|message| message.field == "trigger.weekly.days"));
    }

    #[test]
    fn command_line_wraps_agent_runs() {
        let command_line = JobAction::AgentRun {
            agent_id: "daily-digest".to_string(),
            args: vec!["--audience".to_string(), "exec".to_string()],
            input: None,
        }
        .command_line();

        assert_eq!(command_line.program, "chief-of-staff-agent-runner");
        assert_eq!(
            command_line.args,
            vec![
                "--agent-id".to_string(),
                "daily-digest".to_string(),
                "--audience".to_string(),
                "exec".to_string()
            ]
        );
    }

    #[test]
    fn datetime_formats_are_stable() {
        let datetime = DateTimeParts {
            year: 2026,
            month: 4,
            day: 17,
            hour: 9,
            minute: 5,
            second: 0,
        };

        assert_eq!(datetime.to_iso8601_local(), "2026-04-17T09:05:00");
        assert_eq!(datetime.to_systemd_calendar(), "2026-04-17 09:05:00");
    }

    #[test]
    fn installed_job_preserves_spec_identity_and_backend_metadata() {
        let spec = example_job(JobTrigger::Daily {
            hour: 3,
            minute: 15,
        });
        let installed = InstalledJob::new(
            BackendKind::SystemdUser,
            spec.clone(),
            1_776_000_000_000,
            Some("chief-of-staff-memory-extract.service".to_string()),
        );

        assert_eq!(installed.job_id, spec.job_id);
        assert_eq!(installed.backend, BackendKind::SystemdUser);
        assert_eq!(installed.enabled, spec.enabled);
        assert!(installed.validate().is_valid());
    }

    #[test]
    fn installed_job_queries_compose_backend_action_trigger_enabled_and_limit_filters() {
        let mut memory = example_job(JobTrigger::Daily {
            hour: 3,
            minute: 15,
        });
        memory.job_id = "memory-extract".to_string();
        memory.name = "Memory Extract".to_string();
        let mut digest = example_job(JobTrigger::Interval {
            every_seconds: 3_600,
            anchor: None,
        });
        digest.job_id = "digest-email".to_string();
        digest.name = "Digest Email".to_string();
        digest.action = JobAction::Command {
            program: "chief-of-staff".to_string(),
            args: vec!["digest-email".to_string()],
            input: None,
        };
        let mut disabled = example_job(JobTrigger::Daily { hour: 5, minute: 0 });
        disabled.job_id = "artifact-gc".to_string();
        disabled.name = "Artifact GC".to_string();
        disabled.enabled = false;

        let jobs = vec![
            InstalledJob::new(BackendKind::SystemdUser, memory, 200, None),
            InstalledJob::new(BackendKind::Launchd, digest, 300, None),
            InstalledJob::new(BackendKind::SystemdUser, disabled, 100, None),
        ];
        let query = InstalledJobQuery::new()
            .with_backend(BackendKind::SystemdUser)
            .with_action_kind(JobActionKind::AgentRun)
            .with_trigger_kind(JobTriggerKind::Daily)
            .enabled(true)
            .installed_at_or_after(150)
            .sorted_by(InstalledJobSort::InstalledAtDesc)
            .limited_to(1);

        let results = query_installed_jobs(&jobs, &query);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].job_id, "memory-extract");
        assert!(query.matches_job(results[0]));
        assert_eq!(JobActionKind::AgentRun.to_string(), "agent_run");
        assert_eq!(JobTriggerKind::Daily.to_string(), "daily");
    }

    #[test]
    fn successful_run_receipts_validate_and_report_duration() {
        let receipt = JobRunReceipt::succeeded(
            "run-1",
            "memory-extract",
            1_000,
            1_250,
            vec!["artifact:logs/memory-extract/run-1".to_string()],
        );

        assert!(receipt.exit_status.is_success());
        assert_eq!(receipt.duration_ms(), 250);
        assert!(receipt.validate().is_valid());
    }

    #[test]
    fn failed_run_receipts_require_error_text() {
        let mut receipt = JobRunReceipt::failed(
            "run-2",
            "memory-extract",
            1_000,
            2_000,
            JobExitStatus::failed(Some(1)),
            "agent runner exited with status 1",
        );

        assert!(receipt.validate().is_valid());

        receipt.error = None;
        let result = receipt.validate();
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|message| message.field == "error"));
    }

    #[test]
    fn run_receipts_reject_time_travel_and_multiline_refs() {
        let receipt = JobRunReceipt::succeeded(
            "run-3",
            "memory-extract",
            2_000,
            1_000,
            vec!["artifact:logs\nbad".to_string()],
        );

        let result = receipt.validate();
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|message| message.field == "finished_at"));
        assert!(result
            .errors
            .iter()
            .any(|message| message.field == "output_refs"));
    }

    #[test]
    fn job_status_validates_last_run_and_next_run_hint() {
        let receipt = JobRunReceipt::succeeded("run-4", "memory-extract", 1_000, 1_100, Vec::new());
        let status = JobStatus {
            job_id: "memory-extract".to_string(),
            backend: BackendKind::Launchd,
            status: JobStatusKind::Installed,
            enabled: true,
            last_run: Some(receipt),
            next_run_hint: Some(DateTimeParts {
                year: 2026,
                month: 5,
                day: 8,
                hour: 3,
                minute: 15,
                second: 0,
            }),
        };

        assert!(status.validate().is_valid());
        assert_eq!(status.status.to_string(), "installed");
    }

    #[test]
    fn job_status_queries_filter_outcomes_next_runs_and_sort_by_recent_activity() {
        let succeeded =
            JobRunReceipt::succeeded("run-1", "memory-extract", 1_000, 1_100, Vec::new());
        let failed = JobRunReceipt::failed(
            "run-2",
            "digest-email",
            2_000,
            2_200,
            JobExitStatus::failed(Some(1)),
            "agent runner exited with status 1",
        );
        let statuses = vec![
            JobStatus {
                job_id: "memory-extract".to_string(),
                backend: BackendKind::Launchd,
                status: JobStatusKind::Installed,
                enabled: true,
                last_run: Some(succeeded),
                next_run_hint: Some(DateTimeParts {
                    year: 2026,
                    month: 5,
                    day: 8,
                    hour: 3,
                    minute: 15,
                    second: 0,
                }),
            },
            JobStatus {
                job_id: "digest-email".to_string(),
                backend: BackendKind::SystemdUser,
                status: JobStatusKind::Failed,
                enabled: true,
                last_run: Some(failed),
                next_run_hint: Some(DateTimeParts {
                    year: 2026,
                    month: 5,
                    day: 8,
                    hour: 1,
                    minute: 0,
                    second: 0,
                }),
            },
            JobStatus {
                job_id: "artifact-gc".to_string(),
                backend: BackendKind::WindowsTaskScheduler,
                status: JobStatusKind::Disabled,
                enabled: false,
                last_run: None,
                next_run_hint: None,
            },
        ];
        let failed_query = JobStatusQuery::new()
            .enabled(true)
            .has_last_run(true)
            .with_last_run_outcome(JobRunOutcome::Failed)
            .sorted_by(JobStatusSort::LastRunFinishedDesc);

        let failed_results = query_job_statuses(&statuses, &failed_query);

        assert_eq!(failed_results.len(), 1);
        assert_eq!(failed_results[0].job_id, "digest-email");

        let due_query = JobStatusQuery::new()
            .next_run_at_or_before(DateTimeParts {
                year: 2026,
                month: 5,
                day: 8,
                hour: 2,
                minute: 0,
                second: 0,
            })
            .sorted_by(JobStatusSort::NextRunThenJobId)
            .limited_to(1);
        let due_results = query_job_statuses(&statuses, &due_query);

        assert_eq!(due_results.len(), 1);
        assert_eq!(due_results[0].job_id, "digest-email");
        assert!(due_query.matches_status(due_results[0]));
    }
}
