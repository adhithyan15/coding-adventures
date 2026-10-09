# chief-of-staff-macos-sandbox

The D18S macOS applier (build step 7). It confines a compiled Chief of Staff
agent under a deny-by-default Seatbelt profile, from the agent's first
instruction.

## Why a shim

On Linux, `chief-of-staff-linux-sandbox` installs seccomp and Landlock
between `fork` and `exec`. macOS rules that out:

- there is no `execveat`;
- `sandbox_init` allocates, so it cannot run in a forked child of a
  multi-threaded process;
- nothing can allow exactly one exec.

So the profile is installed by **`chief-of-staff-seatbelt-shim`**, a small
binary pinned by SHA-256 like the broker. In a fresh process image the
shim:

1. places any inherited descriptors at 3, 4, ...;
2. refuses any other open descriptor;
3. sets `RLIMIT_CORE` to 0;
4. calls `sandbox_init`;
5. runs the S-P4 launch probes (its own image must be unreadable, and a
   connect must be refused by the sandbox);
6. checks the agent's inode once more;
7. execs the agent.

Any failure exits 127 without running the agent.

## Usage

```rust,ignore
use chief_of_staff_macos_sandbox::MacosConfinement;
use chief_of_staff_spawn_isolation::VerifiedExecutable;

let agent = VerifiedExecutable::open(agent_path, agent_sha256)?;
let shim = VerifiedExecutable::open(shim_path, shim_sha256)?;
let confinement = MacosConfinement::prepare_verified(&plan, &agent, &shim)?;

let mut command = confinement.command();     // runs the shim
command.args(["--serve"]).stdin(Stdio::piped()).stdout(Stdio::piped());
confinement.apply(&mut command)?;             // last
let child = command.spawn()?;
```

`apply_inheriting(&mut command, descriptors)` also hands the agent those
descriptors at 3, 4, ... (a broker's keys).

## What the profile allows

- exec of the agent's own path;
- reading and mapping the agent and the system libraries;
- `/dev/null`, `/dev/urandom` and `/dev/random`;
- metadata of any path, and `sysctl` reads;
- signals and process info for itself only;
- each `Direct` grant: an exact, canonical, existing regular file, read
  or written.

Everything else is denied, including the network, `mach-lookup`, task
ports, `fork` and every other path.

## Where it fits

- `capability-os-sandbox` lowers a manifest into a `SandboxPlan`.
- `chief-of-staff-spawn-isolation` provides `VerifiedExecutable` and
  descriptor isolation.
- This crate enforces the plan on macOS.
- The broker launcher uses it on macOS from P2.7b.

## Tests

`cargo test -p chief-of-staff-macos-sandbox`. The profile, grant, path and
instruction checks are unit-tested on every Unix. `tests/confinement.rs`
runs only on macOS (CI's `macos-latest`). It runs the probe agent through
the real shim and shows each denial against an unconfined control.
