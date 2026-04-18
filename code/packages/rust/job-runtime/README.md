# job-runtime

`job-runtime` is the backend-selection layer for the scheduling stack.

It gives higher-level code one entry point:

- pick the native scheduler for the current OS
- or force a backend explicitly for tests and tooling
- then compile a `JobSpec` into an `InstallPlan`

## Example

```rust
use job_core::{
    ConcurrencyPolicy, JobAction, JobSpec, JobTrigger, OutputPolicy, RetryPolicy,
};
use job_runtime::NativeJobRuntime;

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
        days: vec![job_core::Weekday::Sunday],
        hour: 1,
        minute: 0,
    },
    concurrency_policy: ConcurrencyPolicy::Skip,
    retry_policy: RetryPolicy::default(),
    timeout_seconds: Some(1200),
    env: Vec::new(),
    working_directory: None,
    output_policy: OutputPolicy::default(),
    enabled: true,
};

let runtime = NativeJobRuntime::for_current_os();
let plan = runtime.install_plan(&spec).unwrap();

assert!(!plan.files_to_write.is_empty());
```

## Dependencies

- job-core
- job-backend-launchd-files
- job-backend-systemd-files
- job-backend-windows-xml

## Development

```bash
bash BUILD
```
