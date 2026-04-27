# D18E - Chief of Staff Job Portability Validator

## Overview

For now, Chief of Staff should reject jobs that do not work across every
supported native operating system backend.

That means a job is accepted only if the repository's current pure native
implementations can all realize it:

- macOS `launchd` LaunchAgents
- Linux `systemd --user`
- Windows Task Scheduler XML

This validator exists to keep engineers from needing to memorize platform gaps.
The runtime should reject non-portable jobs early and explain exactly which OS
backends are blocking the spec.

---

## Design Rule

The default portability mode is:

```text
strict_all_native_oses
```

In this mode:

1. validate the normal `JobSpec`
2. validate portability across all supported native backends
3. reject the job if any backend cannot faithfully realize it

No hidden compatibility daemon should be assumed in this phase.

---

## Runtime API

The runtime should expose a portability validation step before install planning:

```text
validate_portability(spec, target) -> PortabilityReport
```

Where:

```text
PortabilityTarget
|-- all_native_oses
```

and:

```text
PortabilityReport
|-- issues[]
```

```text
PortabilityIssue
|-- field
|-- message
|-- unsupported_backends[]
```

If `issues[]` is non-empty, install planning must fail with a portability
validation error instead of silently producing an OS-specific plan.

---

## Portable subset for Phase 1

The strict cross-platform subset is intentionally smaller than the full
`JobSpec` schema.

### Allowed

- `action.kind`
  `command`, `agent_run`, `function`
- `action.input`
  must be absent
- `trigger`
  `interval` with:
  - no anchor
  - period >= 60 seconds
  `daily`
  `weekly`
  `monthly`
  `at_login`
- `concurrency_policy`
  `skip`
- `retry_policy`
  default only
- `timeout_seconds`
  absent
- `env`
  empty
- `working_directory`
  allowed
- `output_policy`
  default only
- `enabled`
  allowed

### Rejected

- `trigger.once`
  rejected because the pure `launchd` LaunchAgent backend cannot represent a
  year-qualified exact one-shot time
- `trigger.at_boot`
  rejected because the pure macOS backend uses LaunchAgents and the Linux
  backend uses `systemd --user`
- `trigger.interval.anchor`
  rejected because the pure `launchd` and `systemd --user` backends do not
  preserve a portable anchor
- `trigger.interval.every_seconds < 60`
  rejected because Windows Task Scheduler repetition intervals have a higher
  practical floor in the pure XML backend
- `action.input`
  rejected because none of the current pure native backends expose portable
  stdin injection
- non-empty `env`
  rejected because the pure Windows XML backend does not support shell-free env
  injection yet
- non-default `retry_policy`
  rejected because retry/backoff semantics are not implemented in the current
  pure backends
- present `timeout_seconds`
  rejected because the pure `launchd` backend has no native timeout field
- non-default `output_policy`
  rejected because the pure Windows XML backend does not currently render output
  path configuration
- `concurrency_policy != skip`
  rejected because the current strict portable subset treats `skip` as the only
  concurrency policy guaranteed to behave consistently across the pure native
  backends

---

## Error style

Rejections must be explicit and engineer-facing.

Examples:

- `trigger.once`: exact one-shot schedules are unsupported on `launchd`
- `env`: environment injection is unsupported on `windows-task`
- `timeout_seconds`: timeouts are unsupported on `launchd`
- `retry_policy`: retry/backoff is unsupported on `launchd`, `systemd-user`,
  and `windows-task`

The validator should name:

- the field that failed
- the reason
- the specific backends that block portability

---

## Why strict rejection first

This phase deliberately prefers explicit rejection over hidden emulation.

That gives us:

- predictable semantics
- easier testing
- clearer install plans
- a clean place to later add optional compatibility services without pretending
  they are native

Future specs can introduce:

- compatibility-service-backed portability profiles
- backend-specific opt-in modes
- richer install-time suggestions
