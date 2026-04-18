//! # job-runtime
//!
//! `job-runtime` is the thin delegating layer that picks a backend and asks it
//! for a native install plan.
//!
//! The important architectural boundary is this:
//!
//! - backend crates know scheduler syntax
//! - `job-runtime` knows backend selection
//! - callers only need to hand over a [`job_core::JobSpec`]
//!
//! That keeps the rest of Chief of Staff insulated from plist XML, unit-file
//! syntax, and Task Scheduler schema details.

use job_backend_launchd_files::LaunchdFileBackend;
use job_backend_systemd_files::SystemdUserFileBackend;
use job_backend_windows_xml::WindowsTaskSchedulerXmlBackend;
use job_core::{BackendKind, InstallPlan, JobBackend, JobError, JobSpec};

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

/// Portable entry point used by higher-level language bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeJobRuntime {
    selection: BackendSelection,
}

impl NativeJobRuntime {
    /// Construct a runtime that follows the current OS.
    pub fn for_current_os() -> Self {
        Self {
            selection: BackendSelection::CurrentOs,
        }
    }

    /// Construct a runtime pinned to an explicit backend.
    pub fn for_backend(selection: BackendSelection) -> Self {
        Self { selection }
    }

    /// Return the backend the runtime will use.
    pub fn backend_kind(&self) -> Result<BackendKind, JobError> {
        resolve_backend_kind(self.selection)
    }

    /// Compile a job spec into the install plan for the selected backend.
    pub fn install_plan(&self, spec: &JobSpec) -> Result<InstallPlan, JobError> {
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
        "job-runtime currently supports macOS, Linux, and Windows only".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use job_core::{ConcurrencyPolicy, JobAction, JobTrigger, OutputPolicy, RetryPolicy};

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
            timeout_seconds: Some(1800),
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
}
