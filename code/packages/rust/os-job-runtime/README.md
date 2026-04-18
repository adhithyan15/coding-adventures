# os-job-runtime

`os-job-runtime` is the backend-selection layer for the scheduling stack.

It gives higher-level code one entry point:

- pick the native scheduler for the current OS
- or force a backend explicitly for tests and tooling
- validate that a `JobSpec` fits the current repository portability contract
- then compile a portable `JobSpec` into an `InstallPlan`

## Portability rule

For now, `os-job-runtime` enforces a strict portability target across the repository's
three pure native backends:

- macOS `launchd`
- Linux `systemd --user`
- Windows Task Scheduler XML

Jobs that rely on backend-specific features are rejected before install planning
with a portability validation error.

## Example

```rust
use os_job_core::{
    ConcurrencyPolicy, JobAction, JobSpec, JobTrigger, OutputPolicy, RetryPolicy,
};
use os_job_runtime::NativeJobRuntime;

let spec = JobSpec {
    job_id: "artifact-gc".to_string(),
    name: "Artifact GC".to_string(),
    description: "Clean stale artifacts".to_string(),
    action: JobAction::Command {
        program: "/usr/local/bin/chief-of-staff".to_string(),
        args: vec!["artifact-gc".to_string()],
        input: None,
    },
    trigger: JobTrigger::Weekly {
        days: vec![os_job_core::Weekday::Sunday],
        hour: 1,
        minute: 0,
    },
    concurrency_policy: ConcurrencyPolicy::Skip,
    retry_policy: RetryPolicy::default(),
    timeout_seconds: None,
    env: Vec::new(),
    working_directory: None,
    output_policy: OutputPolicy::default(),
    enabled: true,
};

let runtime = NativeJobRuntime::for_current_os();
assert!(runtime.validate_portability(&spec).is_portable());
let plan = runtime.install_plan(&spec).unwrap();

assert!(!plan.files_to_write.is_empty());
```

## Dependencies

- os-job-core
- macos-job-backend-launchd-files
- linux-job-backend-systemd-files
- windows-job-backend-task-xml

## Development

```bash
bash BUILD
```
