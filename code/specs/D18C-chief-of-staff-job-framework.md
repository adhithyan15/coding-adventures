# D18C - Chief of Staff Job Framework

## Overview

Chief of Staff needs a job framework for recurring and one-shot work:

- reminders
- digests
- follow-ups
- nightly indexing
- store compaction
- memory extraction
- scheduled agent runs

The framework must be easy to target from every language in the repository while still
using native operating-system schedulers where possible.

This spec therefore separates:

- a **shared, pure-language job contract**
- a **portable runtime API**
- **OS-specific installers and adapters**
- **optional native acceleration packages**

---

## Design Principles

1. **Jobs are declared canonically once.**
2. **Native schedulers are preferred over an always-on custom timer loop.**
3. **Pure-language SDKs are the default authoring experience.**
4. **Native extensions are optional accelerators, not mandatory dependencies.**
5. **One-shot run semantics are identical everywhere, even when installation differs.**

---

## Layers

```text
Job author (TS / Python / Ruby / Go / Rust / ...)
    |
    v
os-job-core
  - JobSpec
  - validation
  - trigger math
  - portable client API
    |
    v
os-job-runtime
  - install
  - uninstall
  - run_now
  - inspect
    |
    +--> launchd backend
    +--> systemd user backend
    +--> Windows Task Scheduler backend
    +--> in-process fallback backend
```

---

## JobSpec

Jobs should not be defined as raw cron strings. The repository owns the schema.

```text
JobSpec
|-- job_id
|-- name
|-- description
|-- action
|-- trigger
|-- concurrency_policy    allow | skip | replace
|-- retry_policy
|-- timeout_seconds
|-- env
|-- working_directory?
|-- output_policy
|-- enabled
```

### Action

```text
JobAction
|-- kind          command | agent_run | function
|-- target        executable, agent id, or function id
|-- args[]
|-- input?
```

### Trigger

```text
JobTrigger
|-- once(at)
|-- interval(every, anchor?)
|-- daily(hour, minute)
|-- weekly(days[], hour, minute)
|-- monthly(day, hour, minute)
|-- at_login
|-- at_boot
```

The first version should support the trigger types above. File-watch and event-driven
triggers can be specified later in a separate extension spec.

---

## Shared API

Each language should provide an easy authoring API over the same wire schema.

```typescript
type JobRuntime = {
  validate(spec: JobSpec): ValidationResult;
  install(spec: JobSpec): Promise<InstalledJob>;
  uninstall(jobId: string): Promise<void>;
  runNow(jobId: string): Promise<JobRunReceipt>;
  list(): Promise<InstalledJob[]>;
  status(jobId: string): Promise<JobStatus>;
};
```

Equivalent APIs in Ruby, Python, Go, Rust, and other languages must preserve the same
field names and behavior.

For strict cross-platform admission rules, see companion spec `D18E - Chief of
Staff Job Portability Validator`.

---

## OS backends

### macOS

Primary backend: `launchd`

- LaunchAgents for per-user jobs
- plist generation owned by backend package
- backend maps `interval`, calendar triggers, `RunAtLoad`, and restart behavior into
  launchd fields

### Linux

Primary backend: `systemd --user`

- `.service` plus `.timer` units for recurring jobs
- oneshot services for ad hoc runs
- backend owns unit-file generation and install lifecycle

### Windows

Primary backend: Task Scheduler

- XML or COM-backed task definitions
- supports login, boot, daily, weekly, monthly, and interval-style schedules

### Phase 1 backend support matrix

The shared `JobSpec` supports all trigger kinds above, but each native backend must
surface unsupported cases explicitly instead of silently approximating them.

- `macos-job-backend-launchd-files`
  Supports `interval`, `daily`, `weekly`, `monthly`, and `at_login`
- `macos-job-backend-launchd-files`
  Rejects `once`, `at_boot`, stdin payloads, and interval anchors in the pure
  LaunchAgent path
- `linux-job-backend-systemd-files`
  Supports `once`, `interval` without anchor, `daily`, `weekly`, `monthly`, and
  `at_login`
- `linux-job-backend-systemd-files`
  Rejects `at_boot` in the `systemd --user` scope, plus stdin payloads and
  interval anchors
- `windows-job-backend-task-xml`
  Supports `once`, `interval` with a minimum period of 60 seconds, `daily`,
  `weekly`, `monthly`, `at_login`, and `at_boot`
- `windows-job-backend-task-xml`
  Rejects stdin payloads and environment injection in the pure XML path until a
  shell-free Windows launcher exists

### Fallback backend

`in-process`

Used only when native scheduler installation is impossible or explicitly disabled.
This backend is useful for tests, development sandboxes, and constrained environments.

---

## Pure vs native implementation policy

### Required pure packages

- `os-job-core`
  Pure data types, validation, trigger calculations, serialization
- `macos-job-backend-launchd-files`
  Generates plist files as plain text without native dependencies
- `linux-job-backend-systemd-files`
  Generates unit files as plain text
- `windows-job-backend-task-xml`
  Generates Task Scheduler XML as plain text

These packages should work everywhere, even if installation must be handed off to a
small helper binary or CLI command.

### Optional native packages

- `macos-job-backend-native`
- `windows-job-backend-com`
- `linux-job-backend-dbus-systemd`

These can improve installation, status inspection, and error reporting, but they are
optional.

---

## Installation model

Portable code should produce a canonical install plan first:

```text
InstallPlan
|-- backend          launchd | systemd-user | windows-task | in-process
|-- files_to_write[]
|-- commands_to_run[]
|-- permissions_needed[]
```

This allows:

- dry runs
- deterministic tests
- language SDKs that can present the same plan before mutating the OS

---

## Integration with D18

The job framework should integrate with Chief of Staff in two main ways:

- `agent_run` jobs trigger a specific agent or workflow
- internal maintenance jobs keep stores and indexes healthy

Examples:

- nightly `memory-extract`
- hourly `digest-email`
- weekly `artifact-gc`
- daily `context-compact`

---

## Output and observability

Every run must produce:

```text
JobRunReceipt
|-- run_id
|-- job_id
|-- started_at
|-- finished_at
|-- exit_status
|-- output_refs[]
|-- error?
```

The receipt should be written through the storage abstraction or artifact store, not
just printed to a terminal.

---

## Test Strategy

1. one JobSpec serializes identically across languages
2. trigger calculations match across languages and time zones
3. launchd plan generation is deterministic
4. systemd plan generation is deterministic
5. Windows XML plan generation is deterministic
6. fallback backend executes identical run semantics
7. install/uninstall are idempotent

---

## Initial package plan

- `code/packages/rust/os-job-core`
- `code/packages/rust/os-job-runtime`
- `code/packages/rust/macos-job-backend-launchd-files`
- `code/packages/rust/linux-job-backend-systemd-files`
- `code/packages/rust/windows-job-backend-task-xml`
- `code/packages/typescript/os-job-core`
- `code/packages/python/os-job-core`
- `code/packages/ruby/os-job-core`

The shared schema and validation rules should be identical in every language, with Rust
acting as the reference implementation for backend adapters.
