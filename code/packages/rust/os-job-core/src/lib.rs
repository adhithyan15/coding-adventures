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

use std::error::Error;
use std::fmt::{self, Display, Formatter};

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

impl JobAction {
    /// Return the repository-owned action kind string.
    pub fn kind_name(&self) -> &'static str {
        match self {
            JobAction::Command { .. } => "command",
            JobAction::AgentRun { .. } => "agent_run",
            JobAction::Function { .. } => "function",
        }
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

impl JobTrigger {
    /// Return the repository-owned trigger kind string.
    pub fn kind_name(&self) -> &'static str {
        match self {
            JobTrigger::Once { .. } => "once",
            JobTrigger::Interval { .. } => "interval",
            JobTrigger::Daily { .. } => "daily",
            JobTrigger::Weekly { .. } => "weekly",
            JobTrigger::Monthly { .. } => "monthly",
            JobTrigger::AtLogin => "at_login",
            JobTrigger::AtBoot => "at_boot",
        }
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
        JobAction::Command { program, args, input } => {
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
            result.push_error(
                "env.value",
                "must not contain carriage returns or newlines",
            );
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
}
