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
    PortabilityReport, RetryPolicy,
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
}

/// The portability contract the runtime enforces before compiling an install
/// plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortabilityTarget {
    /// Accept only jobs that work across macOS, Linux, and Windows using the
    /// repository's pure native backends.
    AllNativeOses,
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

    /// Return the backend the runtime will use.
    pub fn backend_kind(&self) -> Result<BackendKind, JobError> {
        resolve_backend_kind(self.selection)
    }

    /// Return the portability target enforced by this runtime.
    pub fn portability_target(&self) -> PortabilityTarget {
        self.portability_target
    }

    /// Validate whether a job fits the repository's current portability
    /// contract before backend-specific planning.
    pub fn validate_portability(&self, spec: &JobSpec) -> PortabilityReport {
        validate_portability(spec, self.portability_target)
    }

    /// Compile a job spec into the install plan for the selected backend.
    pub fn install_plan(&self, spec: &JobSpec) -> Result<InstallPlan, JobError> {
        self.validate_portability(spec).into_result()?;
        match resolve_backend_kind(self.selection)? {
            BackendKind::Launchd => LaunchdFileBackend.install_plan(spec),
            BackendKind::SystemdUser => SystemdUserFileBackend.install_plan(spec),
            BackendKind::WindowsTaskScheduler => WindowsTaskSchedulerXmlBackend.install_plan(spec),
            BackendKind::InProcess => Err(JobError::UnsupportedPlatform(
                "the in-process fallback backend is not implemented yet".to_string(),
            )),
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
pub fn supported_backends() -> [BackendKind; 3] {
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

    if !matches!(spec.concurrency_policy, os_job_core::ConcurrencyPolicy::Skip) {
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

fn resolve_backend_kind(selection: BackendSelection) -> Result<BackendKind, JobError> {
    match selection {
        BackendSelection::Launchd => Ok(BackendKind::Launchd),
        BackendSelection::SystemdUser => Ok(BackendKind::SystemdUser),
        BackendSelection::WindowsTaskScheduler => Ok(BackendKind::WindowsTaskScheduler),
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
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.field == "trigger" && issue.unsupported_backends.contains(&BackendKind::Launchd)));
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
}
