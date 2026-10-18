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

- **A principal of its own.** An AppContainer SID derived from the name
  `chief-of-staff.<32 hex>`, the SHA-256 of its attested `agent_id`. Its
  profile, which `CreateProcessW` requires, is registered just before the
  creation and removed while the agent is still suspended, so it has no
  storage folder or registry key to keep state in. The folder must be gone
  after the removal, and an agent runs as one instance at a time (a second
  spawn is refused until the first child is dropped and has exited), since
  any process with its SID could write into the profile while it exists.
  It has no capabilities: no network, no libraries, no devices. It is LPAC,
  so `ALL APPLICATION PACKAGES` grants it nothing.
- **Its own image, readable.** One ACE lets its SID read and execute the
  agent's binary. Beyond that, only the plan's `Direct` file reads are
  granted (P2.8c), one ACE each, taken back when the agent exits.
- **Direct read grants.** A `Direct` `fs:read` of one exact file becomes an
  allow ACE for the agent's SID, with read rights only.
  - The file must be a regular file at exactly its final path, not a
    reparse point, owned and writable only as an executable must be, and
    not the agent's own image.
  - It is held, undeletable, until the agent has exited. Then every entry
    for the SID comes off.
  - A `Direct` write is refused. The agent is Low integrity, and Windows'
    no-write-up policy blocks its writes to ordinary files; lowering a
    file's label would open it to every Low process of the user. So
    writes stay brokered.
  - Create, delete and list are refused: each would need rights over a
    whole directory.
- **Exactly its handles.** stdin and stdout are pipes, stderr is `NUL`,
  plus any handles passed on. Their values arrive in
  `CHIEF_OF_STAFF_HANDLES`.
- **A job.** The job is killed on close and allows one active process. The
  process dies on an unhandled exception and has no UI. Dropping the
  `ConfinedChild` kills it.
- **No console.** It is created detached; the standard handles are pipes
  and `NUL`.
- **Mitigations.** ASLR (with DEP and SEHOP for a 32-bit agent), heap termination, strict handle
  checks, no win32k, no extension points, no dynamic code,
  Microsoft-signed DLLs only, no non-system fonts, and no remote or
  low-label images. The library search starts in system32.
- **A closed environment.** It sees only grantable names with plain values,
  plus `SystemRoot` and `LOCALAPPDATA`. `CreateProcessW` requires the
  latter for an AppContainer, so it names the container's own folder,
  which is removed before the agent runs. Windows may point `TEMP` and
  `TMP` there too.

## Confirmed before it runs (S-P4)

The process is created suspended. Before `ResumeThread`, the applier checks
from outside that:

- the token is an AppContainer's, LPAC, with this SID, no capabilities and
  Low integrity. LPAC is read from `TokenIsLessPrivilegedAppContainer`,
  or, where Windows refuses that query (windows-2025 seems to), from the
  token's `WIN://NOALLAPPPKG` security attribute
  (`is_less_privileged_app_container`);
- the process is in its job;
- every mitigation policy asked for is in force;
- the image is the verified file. `VerifiedExecutable` holds the file open
  sharing read only, so it cannot be changed, renamed or deleted, and
  refuses one that anyone else may write by its access list. The child's
  image name and the held handle's final path must both still be the
  verified path.

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
  - no executable memory can be allocated, and win32k is off;
  - only listed handles are inherited: a write to a stray handle's number
    never reaches its pipe;
  - no storage folder exists for it;
  - passed handles arrive;
  - kill and drop end a sleeping agent;
  - plans it cannot enforce are refused;
  - a non-canonical path is refused.

```sh
cargo test -p chief-of-staff-windows-sandbox
```
