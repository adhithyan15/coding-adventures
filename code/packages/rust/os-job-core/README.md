# os-job-core

`os-job-core` defines the portable OS job contract for Chief of Staff scheduling.

Instead of leaking `launchd`, `systemd`, or Task Scheduler syntax into every
call site, this crate gives the repository one shared vocabulary:

- `JobSpec`
- `JobAction`
- `JobTrigger`
- `InstallPlan`
- `JobBackend`
- validation and repository-owned errors

## What this crate owns

- Portable job data types
- Validation for identifiers, triggers, retry policy, and environment entries
- A deterministic install-plan shape that higher layers can inspect before
  mutating the OS
- Backend traits that file-rendering or native backends implement
- Shared portability-report types used by the runtime's cross-platform validator

## What this crate does not own

- plist rendering
- systemd unit rendering
- Task Scheduler XML rendering
- direct interaction with `launchctl`, `systemctl`, or `schtasks`

Those responsibilities live in backend crates.

## Example

```rust
use os_job_core::{
    ConcurrencyPolicy, JobAction, JobSpec, JobTrigger, OutputPolicy, RetryPolicy,
};

let spec = JobSpec {
    job_id: "memory-extract".to_string(),
    name: "Memory Extract".to_string(),
    description: "Extract durable memories from recent sessions".to_string(),
    action: JobAction::AgentRun {
        agent_id: "memory-extractor".to_string(),
        args: vec!["--scope".to_string(), "daily".to_string()],
        input: None,
    },
    trigger: JobTrigger::Daily { hour: 3, minute: 15 },
    concurrency_policy: ConcurrencyPolicy::Skip,
    retry_policy: RetryPolicy::default(),
    timeout_seconds: Some(600),
    env: Vec::new(),
    working_directory: None,
    output_policy: OutputPolicy::default(),
    enabled: true,
};

assert!(spec.validate().is_valid());
```

## Development

```bash
bash BUILD
```
