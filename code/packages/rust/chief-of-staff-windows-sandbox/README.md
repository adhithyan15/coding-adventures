# chief-of-staff-windows-sandbox

The D18S Windows applier (build step 8, P2.8a). It runs a compiled agent in
its own **less privileged AppContainer**, inside a one-process job, with
exactly the handles it is given. The process is confirmed from outside
before its first instruction runs.

## Where it fits

`capability-os-sandbox` turns an agent's manifest into a `SandboxPlan`. The
per-platform appliers make the OS enforce it:

| Platform | Crate | Mechanism |
|---|---|---|
| Linux | `chief-of-staff-linux-sandbox` | seccomp + Landlock, installed between fork and `execveat` |
| macOS | `chief-of-staff-macos-sandbox` | a pinned shim installs a Seatbelt profile, then execs |
| Windows | this crate | `CreateProcessW` with an attribute list: AppContainer, LPAC, handle list, job, mitigations |

std's `Command` cannot pass a process-thread attribute list on stable Rust,
so this crate creates the process itself and returns a `ConfinedChild`.

## What the agent gets

- **A principal of its own.** An AppContainer profile named
  `chief-of-staff.<32 hex>`, from the SHA-256 of its attested `agent_id`.
  It has no capabilities: no network, no libraries, no devices. It is LPAC,
  so `ALL APPLICATION PACKAGES` grants it nothing.
- **Its own image, readable.** One ACE lets its SID read and execute the
  agent's binary. Nothing else is granted (Direct file grants are P2.8c).
- **Exactly its handles.** stdin and stdout are pipes, stderr is `NUL`,
  plus any handles passed on. Their values arrive in
  `CHIEF_OF_STAFF_HANDLES`.
- **A job.** The job is killed on close and allows one active process. The
  process dies on an unhandled exception and has no UI. Dropping the
  `ConfinedChild` kills it.
- **Mitigations.** DEP, SEHOP, ASLR, heap termination, strict handle
  checks, no win32k, no extension points, no dynamic code,
  Microsoft-signed DLLs only, no non-system fonts, and no remote or
  low-label images. The library search starts in system32.
- **A closed environment.** It sees only grantable names with plain values,
  plus `SystemRoot`.

## Confirmed before it runs (S-P4)

The process is created suspended. Before `ResumeThread`, the applier checks
from outside that:

- the token is an AppContainer's, LPAC, with this SID, no capabilities and
  Low integrity;
- the process is in its job;
- every mitigation policy asked for is in force;
- the image is the verified file. `VerifiedExecutable` holds the file open
  sharing read only, so it cannot be changed, renamed or deleted. The
  child's image name must equal the held handle's final path.

Any failure ends the process through its job, and the launch is refused.

## Usage

```rust,ignore
use chief_of_staff_windows_sandbox::{SpawnRequest, WindowsConfinement};

let confinement = WindowsConfinement::prepare_verified(&plan, &agent, "weather-reporter")?;
let mut child = confinement.spawn(SpawnRequest {
    arguments: vec!["--once".into()],
    environment: vec![("TZ".into(), "UTC".into())],
    inherited: Vec::new(),
})?;
let status = child.wait()?;
```

## Tests

- **Unit tests, on every platform:**
  - the container name;
  - the mitigation bit positions;
  - Windows command-line quoting;
  - the environment set and block;
  - final-path prefixes;
  - the plan check.
- **`tests/confinement.rs`, on `windows-2025` only:** the probe agent
  (`windows-sandbox-probe`) runs confined and as a control. The confined
  run must prove:
  - the token is LPAC at Low integrity;
  - arguments arrive exactly;
  - the environment holds only what was given;
  - files outside the container are denied, for read and for write;
  - the network, process creation, other processes' memory and the
    registry are denied;
  - no executable memory can be allocated, and `user32.dll` cannot load;
  - only listed handles are inherited;
  - passed handles arrive;
  - kill and drop end the agent;
  - plans it cannot enforce are refused;
  - a non-canonical path is refused.

```sh
cargo test -p chief-of-staff-windows-sandbox
```
