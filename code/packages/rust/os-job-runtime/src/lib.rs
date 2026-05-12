//! # os-job-runtime
//!
//! `os-job-runtime` is the thin delegating layer that picks a backend and asks it
//! for a native install plan.
//!
//! The important architectural boundary is this:
//!
//! - backend crates know scheduler syntax
//! - `os-job-runtime` knows backend selection
//! - callers only need to hand over a [`os_job_core::JobSpec`]
//!
//! That keeps the rest of Chief of Staff insulated from plist XML, unit-file
//! syntax, and Task Scheduler schema details.

use linux_job_backend_systemd_files::SystemdUserFileBackend;
use macos_job_backend_launchd_files::LaunchdFileBackend;
use os_job_core::{
    BackendKind, InstallPlan, JobBackend, JobError, JobSpec, JobTrigger, OutputPolicy,
    PermissionRequirement, PortabilityIssue, PortabilityReport, RetryPolicy,
};
use windows_job_backend_task_xml::WindowsTaskSchedulerXmlBackend;

/// Which backend the runtime should use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendSelection {
    /// Detect the backend from the current compilation target.
    CurrentOs,
    /// Force the macOS backend.
    Launchd,
    /// Force the Linux `systemd --user` backend.
    SystemdUser,
    /// Force the Windows Task Scheduler backend.
    WindowsTaskScheduler,
    /// Force the pure in-process fallback backend.
    InProcess,
}

/// The portability contract the runtime enforces before compiling an install
/// plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortabilityTarget {
    /// Accept only jobs that work across macOS, Linux, and Windows using the
    /// repository's pure native backends.
    AllNativeOses,
    /// Let the selected backend decide its own support boundary.
    SelectedBackendOnly,
}

/// Portable entry point used by higher-level language bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeJobRuntime {
    selection: BackendSelection,
    portability_target: PortabilityTarget,
}

impl NativeJobRuntime {
    /// Construct a runtime that follows the current OS.
    pub fn for_current_os() -> Self {
        Self {
            selection: BackendSelection::CurrentOs,
            portability_target: PortabilityTarget::AllNativeOses,
        }
    }

    /// Construct a runtime pinned to an explicit backend.
    pub fn for_backend(selection: BackendSelection) -> Self {
        Self {
            selection,
            portability_target: PortabilityTarget::AllNativeOses,
        }
    }

    /// Construct a runtime pinned to the in-process fallback backend.
    pub fn for_in_process() -> Self {
        Self {
            selection: BackendSelection::InProcess,
            portability_target: PortabilityTarget::SelectedBackendOnly,
        }
    }

    /// Return a copy of this runtime with a different portability target.
    pub fn with_portability_target(mut self, portability_target: PortabilityTarget) -> Self {
        self.portability_target = portability_target;
        self
    }

    /// Return the backend the runtime will use.
    pub fn backend_kind(&self) -> Result<BackendKind, JobError> {
        resolve_backend_kind(self.selection)
    }

    /// Return the portability target enforced by this runtime.
    pub fn portability_target(&self) -> PortabilityTarget {
        self.portability_target
    }

    /// Summarize the selected backend and runtime-wide backend catalog shape
    /// without compiling an install plan.
    pub fn backend_summary(&self) -> Result<RuntimeBackendSummary, JobError> {
        Ok(RuntimeBackendSummary::new(
            resolve_backend_kind(self.selection)?,
            self.portability_target,
        ))
    }

    /// Validate whether a job fits the repository's current portability
    /// contract before backend-specific planning.
    pub fn validate_portability(&self, spec: &JobSpec) -> PortabilityReport {
        validate_portability(spec, self.portability_target)
    }

    /// Return one support row per backend covered by this runtime's portability
    /// target.
    pub fn portability_backend_statuses(
        &self,
        spec: &JobSpec,
    ) -> Result<Vec<PortabilityBackendStatus>, JobError> {
        let report = self.validate_portability(spec);
        let backends = match self.portability_target {
            PortabilityTarget::AllNativeOses => native_backends().to_vec(),
            PortabilityTarget::SelectedBackendOnly => vec![resolve_backend_kind(self.selection)?],
        };

        Ok(summarize_portability_backend_statuses(&report, &backends))
    }

    /// Compile a job spec into the install plan for the selected backend.
    pub fn install_plan(&self, spec: &JobSpec) -> Result<InstallPlan, JobError> {
        self.validate_portability(spec).into_result()?;
        match resolve_backend_kind(self.selection)? {
            BackendKind::Launchd => LaunchdFileBackend.install_plan(spec),
            BackendKind::SystemdUser => SystemdUserFileBackend.install_plan(spec),
            BackendKind::WindowsTaskScheduler => WindowsTaskSchedulerXmlBackend.install_plan(spec),
            BackendKind::InProcess => InProcessFallbackBackend.install_plan(spec),
        }
    }

    /// Convenience alias for call sites that think in terms of planning.
    pub fn compile_install_plan(&self, spec: &JobSpec) -> Result<InstallPlan, JobError> {
        self.install_plan(spec)
    }
}

impl Default for NativeJobRuntime {
    fn default() -> Self {
        Self::for_current_os()
    }
}

/// Return all backends exposed by this crate.
pub fn supported_backends() -> [BackendKind; 4] {
    [
        BackendKind::Launchd,
        BackendKind::SystemdUser,
        BackendKind::WindowsTaskScheduler,
        BackendKind::InProcess,
    ]
}

/// Return native scheduler backends, excluding the pure in-process fallback.
pub fn native_backends() -> [BackendKind; 3] {
    [
        BackendKind::Launchd,
        BackendKind::SystemdUser,
        BackendKind::WindowsTaskScheduler,
    ]
}

/// Validate a job spec against a portability target.
pub fn validate_portability(spec: &JobSpec, target: PortabilityTarget) -> PortabilityReport {
    match target {
        PortabilityTarget::AllNativeOses => validate_all_native_oses(spec),
        PortabilityTarget::SelectedBackendOnly => PortabilityReport::new(),
    }
}

/// Stable sort modes for portability issue read APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortabilityIssueSort {
    /// Preserve validator emission order.
    OriginalOrder,
    /// Sort by issue field and then message.
    Field,
    /// Sort by affected backend family and then issue field.
    BackendThenField,
}

impl Default for PortabilityIssueSort {
    fn default() -> Self {
        Self::OriginalOrder
    }
}

/// Bounded selector for portability issue read tools.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PortabilityIssueQuery {
    /// Restrict results to an exact `PortabilityIssue.field` value.
    pub field: Option<String>,
    /// Restrict results to issues that affect a backend.
    pub backend: Option<BackendKind>,
    /// Stable result ordering.
    pub sort: PortabilityIssueSort,
    /// Maximum number of issues to return.
    pub limit: Option<usize>,
}

impl PortabilityIssueQuery {
    /// Construct an unfiltered query in validator emission order.
    pub fn new() -> Self {
        Self::default()
    }

    /// Restrict results to an exact `PortabilityIssue.field` value.
    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    /// Restrict results to issues that affect a backend.
    pub fn with_backend(mut self, backend: BackendKind) -> Self {
        self.backend = Some(backend);
        self
    }

    /// Select a stable result ordering.
    pub fn with_sort(mut self, sort: PortabilityIssueSort) -> Self {
        self.sort = sort;
        self
    }

    /// Bound the result set.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Summary of all portability issues that block one backend family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortabilityBackendSummary {
    pub backend: BackendKind,
    pub issue_count: usize,
    pub blocked_fields: Vec<String>,
}

/// Summary of all portability issues attached to one spec field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortabilityFieldSummary {
    pub field: String,
    pub issue_count: usize,
    pub unsupported_backends: Vec<BackendKind>,
}

/// Backend-level support row derived from a portability report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortabilityBackendStatus {
    pub backend: BackendKind,
    pub is_supported: bool,
    pub issue_count: usize,
    pub blocked_fields: Vec<String>,
}

/// Read-side summary of runtime backend selection and catalog shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeBackendSummary {
    pub selected_backend: BackendKind,
    pub portability_target: PortabilityTarget,
    pub supported_backend_count: usize,
    pub native_backend_count: usize,
    pub fallback_backend_count: usize,
    pub selected_backend_is_native: bool,
    pub selected_backend_is_fallback: bool,
}

impl RuntimeBackendSummary {
    pub fn new(selected_backend: BackendKind, portability_target: PortabilityTarget) -> Self {
        let native_backend_count = native_backends().len();
        let supported_backend_count = supported_backends().len();
        let selected_backend_is_native = native_backends().contains(&selected_backend);
        Self {
            selected_backend,
            portability_target,
            supported_backend_count,
            native_backend_count,
            fallback_backend_count: supported_backend_count.saturating_sub(native_backend_count),
            selected_backend_is_native,
            selected_backend_is_fallback: !selected_backend_is_native,
        }
    }

    pub fn enforces_all_native_portability(self) -> bool {
        self.portability_target == PortabilityTarget::AllNativeOses
    }
}

/// Return portability issues matching a bounded read selector.
pub fn query_portability_issues<'a>(
    report: &'a PortabilityReport,
    query: &PortabilityIssueQuery,
) -> Vec<&'a PortabilityIssue> {
    let mut issues: Vec<&PortabilityIssue> = report
        .issues
        .iter()
        .filter(|issue| portability_issue_matches(issue, query))
        .collect();

    sort_portability_issues(&mut issues, query.sort);
    if let Some(limit) = query.limit {
        issues.truncate(limit);
    }
    issues
}

/// Summarize a portability report by affected backend family.
pub fn summarize_portability_by_backend(
    report: &PortabilityReport,
) -> Vec<PortabilityBackendSummary> {
    let mut summaries: Vec<PortabilityBackendSummary> = Vec::new();

    for issue in &report.issues {
        for backend in &issue.unsupported_backends {
            let summary = match summaries
                .iter_mut()
                .find(|summary| summary.backend == *backend)
            {
                Some(summary) => summary,
                None => {
                    summaries.push(PortabilityBackendSummary {
                        backend: *backend,
                        issue_count: 0,
                        blocked_fields: Vec::new(),
                    });
                    summaries
                        .last_mut()
                        .expect("summary was just pushed and must exist")
                }
            };

            summary.issue_count += 1;
            push_unique_string(&mut summary.blocked_fields, &issue.field);
        }
    }

    summaries.sort_by_key(|summary| backend_sort_key(summary.backend));
    for summary in &mut summaries {
        summary.blocked_fields.sort();
    }
    summaries
}

/// Summarize a portability report by spec field.
pub fn summarize_portability_by_field(report: &PortabilityReport) -> Vec<PortabilityFieldSummary> {
    let mut summaries: Vec<PortabilityFieldSummary> = Vec::new();

    for issue in &report.issues {
        let summary = match summaries
            .iter_mut()
            .find(|summary| summary.field == issue.field)
        {
            Some(summary) => summary,
            None => {
                summaries.push(PortabilityFieldSummary {
                    field: issue.field.clone(),
                    issue_count: 0,
                    unsupported_backends: Vec::new(),
                });
                summaries
                    .last_mut()
                    .expect("summary was just pushed and must exist")
            }
        };

        summary.issue_count += 1;
        for backend in &issue.unsupported_backends {
            push_unique_backend(&mut summary.unsupported_backends, *backend);
        }
    }

    summaries.sort_by(|left, right| left.field.cmp(&right.field));
    for summary in &mut summaries {
        summary
            .unsupported_backends
            .sort_by_key(|backend| backend_sort_key(*backend));
    }
    summaries
}

/// Summarize backend support for a caller-selected backend set.
pub fn summarize_portability_backend_statuses(
    report: &PortabilityReport,
    backends: &[BackendKind],
) -> Vec<PortabilityBackendStatus> {
    let mut statuses: Vec<PortabilityBackendStatus> = backends
        .iter()
        .copied()
        .map(|backend| PortabilityBackendStatus {
            backend,
            is_supported: true,
            issue_count: 0,
            blocked_fields: Vec::new(),
        })
        .collect();

    for issue in &report.issues {
        if issue.unsupported_backends.is_empty() {
            for status in &mut statuses {
                apply_issue_to_backend_status(status, &issue.field);
            }
            continue;
        }

        for backend in &issue.unsupported_backends {
            if let Some(status) = statuses
                .iter_mut()
                .find(|status| status.backend == *backend)
            {
                apply_issue_to_backend_status(status, &issue.field);
            }
        }
    }

    statuses.sort_by_key(|status| backend_sort_key(status.backend));
    for status in &mut statuses {
        status.blocked_fields.sort();
    }
    statuses
}

/// Pure fallback backend for tests, development sandboxes, and constrained hosts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InProcessFallbackBackend;

impl JobBackend for InProcessFallbackBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::InProcess
    }

    fn install_plan(&self, spec: &JobSpec) -> Result<InstallPlan, JobError> {
        spec.validate().into_result()?;
        Ok(InstallPlan {
            backend: BackendKind::InProcess,
            files_to_write: Vec::new(),
            commands_to_run: Vec::new(),
            permissions_needed: vec![PermissionRequirement {
                scope: "process-lifetime".to_string(),
                detail: format!(
                    "job `{}` runs only while the hosting process is alive",
                    spec.job_id
                ),
            }],
        })
    }
}

fn validate_all_native_oses(spec: &JobSpec) -> PortabilityReport {
    let mut report = PortabilityReport::new();

    if spec.action.has_input() {
        report.push_issue(
            "action.input",
            "stdin payloads are not supported across the pure macOS, Linux, and Windows backends",
            vec![
                BackendKind::Launchd,
                BackendKind::SystemdUser,
                BackendKind::WindowsTaskScheduler,
            ],
        );
    }

    if !spec.env.is_empty() {
        report.push_issue(
            "env",
            "environment injection is not portable because the Windows XML backend rejects it",
            vec![BackendKind::WindowsTaskScheduler],
        );
    }

    if spec.timeout_seconds.is_some() {
        report.push_issue(
            "timeout_seconds",
            "timeouts are not portable because the launchd backend has no native timeout field",
            vec![BackendKind::Launchd],
        );
    }

    if spec.output_policy != OutputPolicy::default() {
        report.push_issue(
            "output_policy",
            "custom stdout/stderr routing is not portable because the Windows XML backend does not render output paths",
            vec![BackendKind::WindowsTaskScheduler],
        );
    }

    if spec.retry_policy != RetryPolicy::default() {
        report.push_issue(
            "retry_policy",
            "retry/backoff policies are not implemented in the pure native backends yet",
            vec![
                BackendKind::Launchd,
                BackendKind::SystemdUser,
                BackendKind::WindowsTaskScheduler,
            ],
        );
    }

    if !matches!(
        spec.concurrency_policy,
        os_job_core::ConcurrencyPolicy::Skip
    ) {
        report.push_issue(
            "concurrency_policy",
            "only `skip` is currently treated as the portable subset across all native backends",
            vec![
                BackendKind::Launchd,
                BackendKind::SystemdUser,
                BackendKind::WindowsTaskScheduler,
            ],
        );
    }

    match &spec.trigger {
        JobTrigger::Once { .. } => {
            report.push_issue(
                "trigger",
                "one-shot timestamps are not portable because the launchd LaunchAgent backend cannot represent a year-qualified exact run time",
                vec![BackendKind::Launchd],
            );
        }
        JobTrigger::Interval {
            every_seconds,
            anchor,
        } => {
            if *every_seconds < 60 {
                report.push_issue(
                    "trigger.interval.every_seconds",
                    "interval jobs must be at least 60 seconds to work on Windows Task Scheduler",
                    vec![BackendKind::WindowsTaskScheduler],
                );
            }
            if anchor.is_some() {
                report.push_issue(
                    "trigger.interval.anchor",
                    "anchored intervals are not portable because launchd and systemd --user do not preserve a portable anchor in the pure backends",
                    vec![BackendKind::Launchd, BackendKind::SystemdUser],
                );
            }
        }
        JobTrigger::AtBoot => {
            report.push_issue(
                "trigger",
                "boot triggers are not portable because the pure macOS backend targets LaunchAgents and the Linux backend targets systemd --user",
                vec![BackendKind::Launchd, BackendKind::SystemdUser],
            );
        }
        JobTrigger::Daily { .. }
        | JobTrigger::Weekly { .. }
        | JobTrigger::Monthly { .. }
        | JobTrigger::AtLogin => {}
    }

    report
}

fn portability_issue_matches(issue: &PortabilityIssue, query: &PortabilityIssueQuery) -> bool {
    if let Some(field) = &query.field {
        if issue.field != *field {
            return false;
        }
    }

    if let Some(backend) = query.backend {
        if !issue.unsupported_backends.contains(&backend) {
            return false;
        }
    }

    true
}

fn sort_portability_issues(issues: &mut Vec<&PortabilityIssue>, sort: PortabilityIssueSort) {
    match sort {
        PortabilityIssueSort::OriginalOrder => {}
        PortabilityIssueSort::Field => {
            issues.sort_by(|left, right| {
                left.field
                    .cmp(&right.field)
                    .then_with(|| left.message.cmp(&right.message))
            });
        }
        PortabilityIssueSort::BackendThenField => {
            issues.sort_by(|left, right| {
                first_unsupported_backend_sort_key(left)
                    .cmp(&first_unsupported_backend_sort_key(right))
                    .then_with(|| left.field.cmp(&right.field))
                    .then_with(|| left.message.cmp(&right.message))
            });
        }
    }
}

fn first_unsupported_backend_sort_key(issue: &PortabilityIssue) -> usize {
    issue
        .unsupported_backends
        .iter()
        .map(|backend| backend_sort_key(*backend))
        .min()
        .unwrap_or(usize::MAX)
}

fn backend_sort_key(backend: BackendKind) -> usize {
    match backend {
        BackendKind::Launchd => 0,
        BackendKind::SystemdUser => 1,
        BackendKind::WindowsTaskScheduler => 2,
        BackendKind::InProcess => 3,
    }
}

fn push_unique_string(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

fn push_unique_backend(values: &mut Vec<BackendKind>, value: BackendKind) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn apply_issue_to_backend_status(status: &mut PortabilityBackendStatus, field: &str) {
    status.is_supported = false;
    status.issue_count += 1;
    push_unique_string(&mut status.blocked_fields, field);
}

fn resolve_backend_kind(selection: BackendSelection) -> Result<BackendKind, JobError> {
    match selection {
        BackendSelection::Launchd => Ok(BackendKind::Launchd),
        BackendSelection::SystemdUser => Ok(BackendKind::SystemdUser),
        BackendSelection::WindowsTaskScheduler => Ok(BackendKind::WindowsTaskScheduler),
        BackendSelection::InProcess => Ok(BackendKind::InProcess),
        BackendSelection::CurrentOs => current_platform_backend(),
    }
}

fn current_platform_backend() -> Result<BackendKind, JobError> {
    #[cfg(target_os = "macos")]
    {
        return Ok(BackendKind::Launchd);
    }

    #[cfg(target_os = "linux")]
    {
        return Ok(BackendKind::SystemdUser);
    }

    #[cfg(target_os = "windows")]
    {
        return Ok(BackendKind::WindowsTaskScheduler);
    }

    #[allow(unreachable_code)]
    Err(JobError::UnsupportedPlatform(
        "os-job-runtime currently supports macOS, Linux, and Windows only".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use os_job_core::{
        ConcurrencyPolicy, EnvironmentEntry, JobAction, JobTrigger, OutputPolicy, RetryPolicy,
    };

    use super::*;

    fn sample_job() -> JobSpec {
        JobSpec {
            job_id: "store-compaction".to_string(),
            name: "Store Compaction".to_string(),
            description: "Compact artifact and memory indexes".to_string(),
            action: JobAction::Command {
                program: "/usr/local/bin/chief-of-staff".to_string(),
                args: vec!["compact-stores".to_string()],
                input: None,
            },
            trigger: JobTrigger::Daily {
                hour: 1,
                minute: 15,
            },
            concurrency_policy: ConcurrencyPolicy::Skip,
            retry_policy: RetryPolicy::default(),
            timeout_seconds: None,
            env: Vec::new(),
            working_directory: None,
            output_policy: OutputPolicy::default(),
            enabled: true,
        }
    }

    fn non_portable_report() -> PortabilityReport {
        let mut job = sample_job();
        job.env = vec![EnvironmentEntry {
            key: "COS_PROFILE".to_string(),
            value: "prod".to_string(),
        }];
        job.timeout_seconds = Some(60);
        job.trigger = JobTrigger::Interval {
            every_seconds: 30,
            anchor: Some(os_job_core::DateTimeParts {
                year: 2026,
                month: 5,
                day: 9,
                hour: 8,
                minute: 30,
                second: 0,
            }),
        };

        NativeJobRuntime::default().validate_portability(&job)
    }

    #[test]
    fn explicit_launchd_selection_dispatches_to_launchd_backend() {
        let plan = NativeJobRuntime::for_backend(BackendSelection::Launchd)
            .install_plan(&sample_job())
            .expect("launchd plan should compile");

        assert_eq!(plan.backend, BackendKind::Launchd);
    }

    #[test]
    fn explicit_systemd_selection_dispatches_to_systemd_backend() {
        let plan = NativeJobRuntime::for_backend(BackendSelection::SystemdUser)
            .install_plan(&sample_job())
            .expect("systemd plan should compile");

        assert_eq!(plan.backend, BackendKind::SystemdUser);
    }

    #[test]
    fn explicit_windows_selection_dispatches_to_windows_backend() {
        let plan = NativeJobRuntime::for_backend(BackendSelection::WindowsTaskScheduler)
            .install_plan(&sample_job())
            .expect("windows plan should compile");

        assert_eq!(plan.backend, BackendKind::WindowsTaskScheduler);
    }

    #[test]
    fn explicit_in_process_selection_compiles_fallback_plan() {
        let runtime = NativeJobRuntime::for_in_process();
        let plan = runtime
            .install_plan(&sample_job())
            .expect("in-process plan should compile");

        assert_eq!(runtime.backend_kind().unwrap(), BackendKind::InProcess);
        assert_eq!(plan.backend, BackendKind::InProcess);
        assert!(plan.files_to_write.is_empty());
        assert!(plan.commands_to_run.is_empty());
        assert_eq!(plan.permissions_needed[0].scope, "process-lifetime");
    }

    #[test]
    fn in_process_selection_can_skip_native_portability_rejections() {
        let mut job = sample_job();
        job.trigger = JobTrigger::Once {
            at: os_job_core::DateTimeParts {
                year: 2026,
                month: 4,
                day: 18,
                hour: 10,
                minute: 0,
                second: 0,
            },
        };

        assert!(!NativeJobRuntime::default()
            .validate_portability(&job)
            .is_portable());
        assert!(NativeJobRuntime::for_in_process()
            .validate_portability(&job)
            .is_portable());
        assert_eq!(
            NativeJobRuntime::for_in_process()
                .install_plan(&job)
                .unwrap()
                .backend,
            BackendKind::InProcess
        );
    }

    #[test]
    fn supported_backends_include_in_process_fallback() {
        assert!(supported_backends().contains(&BackendKind::InProcess));
        assert_eq!(
            native_backends(),
            [
                BackendKind::Launchd,
                BackendKind::SystemdUser,
                BackendKind::WindowsTaskScheduler
            ]
        );
    }

    #[test]
    fn runtime_backend_summary_describes_selection_and_portability_target() {
        let launchd = NativeJobRuntime::for_backend(BackendSelection::Launchd)
            .backend_summary()
            .unwrap();

        assert_eq!(launchd.selected_backend, BackendKind::Launchd);
        assert_eq!(launchd.portability_target, PortabilityTarget::AllNativeOses);
        assert_eq!(launchd.supported_backend_count, 4);
        assert_eq!(launchd.native_backend_count, 3);
        assert_eq!(launchd.fallback_backend_count, 1);
        assert!(launchd.selected_backend_is_native);
        assert!(!launchd.selected_backend_is_fallback);
        assert!(launchd.enforces_all_native_portability());

        let fallback = NativeJobRuntime::for_in_process()
            .backend_summary()
            .unwrap();

        assert_eq!(fallback.selected_backend, BackendKind::InProcess);
        assert_eq!(
            fallback.portability_target,
            PortabilityTarget::SelectedBackendOnly
        );
        assert!(!fallback.selected_backend_is_native);
        assert!(fallback.selected_backend_is_fallback);
        assert!(!fallback.enforces_all_native_portability());
    }

    #[test]
    fn portability_validation_rejects_one_shot_jobs() {
        let mut job = sample_job();
        job.trigger = JobTrigger::Once {
            at: os_job_core::DateTimeParts {
                year: 2026,
                month: 4,
                day: 18,
                hour: 10,
                minute: 0,
                second: 0,
            },
        };

        let report = NativeJobRuntime::default().validate_portability(&job);

        assert!(!report.is_portable());
        assert!(report.issues.iter().any(|issue| issue.field == "trigger"
            && issue.unsupported_backends.contains(&BackendKind::Launchd)));
    }

    #[test]
    fn portability_validation_rejects_env_and_timeouts() {
        let mut job = sample_job();
        job.env = vec![EnvironmentEntry {
            key: "COS_PROFILE".to_string(),
            value: "prod".to_string(),
        }];
        job.timeout_seconds = Some(60);

        let report = NativeJobRuntime::default().validate_portability(&job);

        assert!(!report.is_portable());
        assert!(report.issues.iter().any(|issue| issue.field == "env"));
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.field == "timeout_seconds"));
    }

    #[test]
    fn install_plan_rejects_non_portable_jobs_before_backend_planning() {
        let mut job = sample_job();
        job.trigger = JobTrigger::Interval {
            every_seconds: 30,
            anchor: None,
        };

        let error = NativeJobRuntime::default()
            .install_plan(&job)
            .expect_err("non-portable jobs should be rejected before backend planning");

        assert!(matches!(error, JobError::PortabilityValidationFailed(_)));
    }

    #[test]
    fn portability_issue_query_filters_sorts_and_limits() {
        let report = non_portable_report();

        let windows_issues = query_portability_issues(
            &report,
            &PortabilityIssueQuery::new()
                .with_backend(BackendKind::WindowsTaskScheduler)
                .with_sort(PortabilityIssueSort::Field),
        );

        assert_eq!(windows_issues.len(), 2);
        assert_eq!(windows_issues[0].field, "env");
        assert_eq!(windows_issues[1].field, "trigger.interval.every_seconds");

        let timeout_issues = query_portability_issues(
            &report,
            &PortabilityIssueQuery::new()
                .with_field("timeout_seconds")
                .with_limit(1),
        );

        assert_eq!(timeout_issues.len(), 1);
        assert_eq!(
            timeout_issues[0].unsupported_backends,
            vec![BackendKind::Launchd]
        );
    }

    #[test]
    fn portability_summaries_group_by_backend_and_field() {
        let report = non_portable_report();

        let backend_summaries = summarize_portability_by_backend(&report);
        let launchd_summary = backend_summaries
            .iter()
            .find(|summary| summary.backend == BackendKind::Launchd)
            .expect("launchd should have blocked fields");
        let windows_summary = backend_summaries
            .iter()
            .find(|summary| summary.backend == BackendKind::WindowsTaskScheduler)
            .expect("windows should have blocked fields");

        assert_eq!(launchd_summary.issue_count, 2);
        assert_eq!(
            launchd_summary.blocked_fields,
            vec![
                "timeout_seconds".to_string(),
                "trigger.interval.anchor".to_string()
            ]
        );
        assert_eq!(windows_summary.issue_count, 2);
        assert_eq!(
            windows_summary.blocked_fields,
            vec![
                "env".to_string(),
                "trigger.interval.every_seconds".to_string()
            ]
        );

        let field_summaries = summarize_portability_by_field(&report);
        let anchored_interval = field_summaries
            .iter()
            .find(|summary| summary.field == "trigger.interval.anchor")
            .expect("anchor field should be summarized");

        assert_eq!(anchored_interval.issue_count, 1);
        assert_eq!(
            anchored_interval.unsupported_backends,
            vec![BackendKind::Launchd, BackendKind::SystemdUser]
        );
    }

    #[test]
    fn portability_backend_statuses_show_supported_and_blocked_backends() {
        let report = non_portable_report();
        let statuses = summarize_portability_backend_statuses(&report, &native_backends());

        let launchd = statuses
            .iter()
            .find(|status| status.backend == BackendKind::Launchd)
            .expect("launchd status should exist");
        let systemd = statuses
            .iter()
            .find(|status| status.backend == BackendKind::SystemdUser)
            .expect("systemd status should exist");
        let windows = statuses
            .iter()
            .find(|status| status.backend == BackendKind::WindowsTaskScheduler)
            .expect("windows status should exist");

        assert!(!launchd.is_supported);
        assert_eq!(launchd.issue_count, 2);
        assert_eq!(
            launchd.blocked_fields,
            vec![
                "timeout_seconds".to_string(),
                "trigger.interval.anchor".to_string()
            ]
        );
        assert!(!systemd.is_supported);
        assert_eq!(
            systemd.blocked_fields,
            vec!["trigger.interval.anchor".to_string()]
        );
        assert!(!windows.is_supported);
        assert_eq!(
            windows.blocked_fields,
            vec![
                "env".to_string(),
                "trigger.interval.every_seconds".to_string()
            ]
        );

        let portable = summarize_portability_backend_statuses(
            &NativeJobRuntime::default().validate_portability(&sample_job()),
            &native_backends(),
        );

        assert!(portable.iter().all(|status| status.is_supported));
        assert!(portable
            .iter()
            .all(|status| status.issue_count == 0 && status.blocked_fields.is_empty()));
    }

    #[test]
    fn runtime_backend_statuses_follow_portability_target() {
        let mut job = sample_job();
        job.trigger = JobTrigger::Once {
            at: os_job_core::DateTimeParts {
                year: 2026,
                month: 4,
                day: 18,
                hour: 10,
                minute: 0,
                second: 0,
            },
        };

        let native_statuses = NativeJobRuntime::for_backend(BackendSelection::Launchd)
            .portability_backend_statuses(&job)
            .expect("native statuses should be available");

        assert_eq!(native_statuses.len(), 3);
        assert!(native_statuses
            .iter()
            .any(|status| status.backend == BackendKind::Launchd && !status.is_supported));

        let fallback_statuses = NativeJobRuntime::for_in_process()
            .portability_backend_statuses(&job)
            .expect("fallback statuses should be available");

        assert_eq!(
            fallback_statuses,
            vec![PortabilityBackendStatus {
                backend: BackendKind::InProcess,
                is_supported: true,
                issue_count: 0,
                blocked_fields: Vec::new(),
            }]
        );
    }
}
