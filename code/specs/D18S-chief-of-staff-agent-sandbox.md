# D18S — Chief of Staff Agent Sandbox and Capability Broker

## Overview

An agent declares what it needs, is launched into an OS sandbox that can grant
nothing else, and reaches the rest of the system through exactly one inherited
descriptor. Everything it is allowed to do, it does by asking. Everything it is
not allowed to do, it cannot express.

This spec fixes where the boundary is. Three mechanisms in this stack look like
they enforce capabilities, and only one of them does:

| Mechanism | What it actually gives you |
|---|---|
| Sign-time source analysis | The declaration is honest. A **check**, not a boundary. |
| Generated capability SDK | The sanctioned path is typed and obvious. **Ergonomics**, not a boundary. |
| OS sandbox at spawn | The process cannot do the thing. **The boundary.** |

Conflating these is the central risk. A generated SDK that omits a function has
removed a convenience, not a capability: the process still owns its own address
space and can write whatever bytes it likes to whatever descriptor it holds.
Only the third row survives an agent that is actively trying.

**Depends on:** D18 Chief of Staff, D18D Tool API, D18R Supervision Tree,
spec 13 Capability Security, D21 Capability Cage.

**Amends D18R.** D18R's "What this spec does not decide" left two questions
open. This spec closes both: host profiles live in the **signed agent
manifest** (S-D1), and supervisor-to-agent transport is **stdio, and is the
only channel the agent has** (S-I2).

**Removes a dependency.** The current deny-all sandbox is Deno's permission
system (`chief-of-staff-host-runtime/src/package.rs`, `DENO_FLAGS`). That works,
and it is a real boundary, but it is available only to one language runtime.
This spec replaces it with an OS-level boundary that is language-agnostic by
construction, so an agent may be written in any language.

---

## Terminology

Continues D18R's OTP vocabulary. New terms:

| Term | Meaning |
|---|---|
| **broker** | The supervisor-launched process that holds the secure channel and mediates every capability the agent exercises. |
| **channel descriptor** | The single inherited file descriptor connecting an agent to its broker. The agent's entire view of the outside world. |
| **direct grant** | A capability the OS can enforce narrowly, granted to the agent process itself. |
| **brokered capability** | A capability denied at the OS level and available only by asking the broker. |
| **declaration check** | Sign-time static analysis. Confirms the manifest matches the source. Not an enforcement mechanism. |

`host` remains reserved for `chief-of-staff-host`, per D18R.

---

## Where the boundary is

Spec 13 defines six layers. This spec states plainly which of them an attacker
has to defeat:

```
Layer 1  Capability manifest          declaration
Layer 2  Secure wrappers              declaration
Layer 3  Linter + banned constructs   declaration check
Layer 4  CI / sign-time gate          declaration check
Layer 5  Hardware-key approval        authorization of the declaration
Layer 6  OS sandbox                   THE BOUNDARY
```

Layers 1-5 govern what an agent *says* it does and who approved it. They are
worth having: they catch honest mistakes, make review tractable, and make the
manifest trustworthy as a description. None of them stop code that lies.

### S-B1 — the sandbox is the only enforcement claim this system makes

No design document, review, or gate may describe layers 1-5 as preventing an
agent from performing an operation. They establish that the agent *declared*
the operation. If an operation must be impossible, it must be impossible at
layer 6.

### S-B2 — sign-time analysis states its own confidence, per language

`ca-capability-analyzer` walks a Go AST, compares detected capabilities against
declared ones, and bans the escape hatches that would defeat it —
`unsafe.Pointer`, `import "C"`, `plugin.Open`, `reflect.Value.Call`,
`reflect.Value.MethodByName`, `//go:linkname`. That is the correct technique:
rather than analyzing around reflection and FFI, it shrinks the language to an
analyzable subset and requires an explicit declared `ffi:call:*` capability to
opt back out.

The technique's cost scales with how dynamic the language is, and this must be
recorded rather than glossed:

| Language | Confidence | Why |
|---|---|---|
| Go, Rust | **High** | Static, compiled, closed symbol set. Banning reflection and FFI is a livable constraint. |
| Python, JS, Ruby | **Best-effort** | `getattr`, `eval`, `__import__`, `globalThis[x]`, dynamic `import()` are idiom, not escape hatches. Banning them rejects the ecosystem. |
| Any language with native extensions | **None for the native part** | A C extension calls `open(2)` directly. No source analysis sees it. |

Two problems compound this and are not solvable by better parsing: the analyzer
must cover the **whole third-party dependency closure**, not just the agent's
own files, and **compiled native code is invisible to source analysis**.

This is acceptable precisely because of S-B1. With the sandbox holding, an
imperfect analyzer degrades from a security hole into a developer-experience
feature: a clear rejection at sign time instead of an opaque `EPERM` at
runtime.

---

## Isolation rules

### S-I1 — the agent process is deny-all by default

Every agent is launched with no network, no filesystem, no subprocess
execution, no FFI/dynamic loading, and no environment inheritance, except what
S-I6 grants explicitly.

The design makes this cheap. Because capabilities are exercised by asking the
broker, the sandbox never has to express "allow reading this path" in OS terms.
Expressing *allow* rules is where OS sandboxing becomes version-dependent and
expensive; expressing deny-all is the cheapest thing every platform primitive
does, and two of them do it natively.

### S-I2 — the agent holds exactly one descriptor, and it reaches the broker

The channel descriptor is the agent's entire view of the outside world. It does
not reach the supervisor. An agent that discovers the supervisor's address,
socket path, or PID can do nothing with the knowledge, because it has no
syscall with which to act on it.

State the reason correctly. The agent cannot reach the supervisor **because the
sandbox removed its ability to create a channel** — not because no channel was
configured for it. A design that relies on "we did not give it the code" has
returned to the error S-B1 exists to prevent.

### S-I3 — no other descriptor is inherited

Every descriptor the supervisor holds must be `CLOEXEC` (or its
platform equivalent) except the channel descriptor, which is passed
deliberately. A descriptor opened anywhere in the supervisor without `CLOEXEC`
leaks into every agent spawned afterwards and is a complete escape: it needs no
syscall the sandbox denies, because the process already holds the object.

This is enforced at spawn by an explicit descriptor-table audit, not by
convention. `linux.fd_table`, `openbsd.fd_inheritance`, and
`windows.handle_inheritance` exist in `capability-os-sandbox` for this rule.

### S-I4 — the sandbox is applied after runtime boot and before agent code

Interpreted runtimes need filesystem access to start. Python reads its standard
library and `dlopen`s shared objects before a line of agent code executes;
Node resolves its own modules. A sandbox applied at `exec` therefore cannot be
literally deny-all for these runtimes.

Two admissible resolutions, per platform capability:

1. **Launcher shim** — a small trusted stage runs inside the agent process,
   completes runtime initialization, installs the sandbox, then transfers
   control to agent code. Preferred: it yields true deny-all.
2. **Read-only runtime view** — the runtime image is granted read-only via a
   path-scoped primitive (`linux.landlock.path_beneath`,
   `linux.mount_namespace.library_view`) and everything else denied. Weaker,
   because the runtime image remains readable.

Compiled agents (Go, Rust) require neither and get deny-all directly. Capsicum
and `pledge` are strictest here and correspondingly hardest for interpreted
runtimes; `cap_enter()` breaks lazy imports outright.

### S-I5 — the agent never holds channel keys

Secure-channel keys live in the broker, which the supervisor launched. They are
never present in the agent's address space. This is what the broker buys that
codegen cannot: a secret the agent cannot reach regardless of what code it
runs.

### S-I6 — capabilities are brokered by default; direct grants are opt-in

A declared capability defaults to **brokered**: denied at the OS level,
available only by asking. A capability may be promoted to a **direct grant**
only where the OS can enforce it narrowly and a measurement justifies it —
brokering per-read over a large data directory is the motivating case.

Every promotion is recorded in the manifest and is visible in review. The
default direction matters: brokered-by-default fails closed, and a
direct-by-default system silently grants whatever the OS happened to allow.

---

## Broker rules

### S-K1 — the supervisor launches the broker, not the agent

If the agent package ships the broker and the agent starts it, the broker is
inside the agent's trust domain and mediates nothing. The supervisor launches
it, from a binary whose digest it verifies, exactly as `spawn_verified` already
re-checks `package.digest()` against `registration.package_hash()`.

### S-K2 — identity is bound at launch and never asserted by the agent

The broker is told which `agent_id` it speaks for, by the supervisor that
already holds the attested value. No field on the wire carries an identity the
agent supplies.

This preserves the existing attestation chain — signed manifest `manifest.agent`
to `HostName` to `HostRegistration` to `agent_id` — which is what makes the
vault's per-secret `allowed_agents` meaningful. An agent permitted to state its
own identity makes every per-secret policy in VLT06 worthless.

### S-K3 — the broker enforces the manifest; the SDK does not

The broker re-validates every request against the signed manifest. The
generated SDK is the manifest's ergonomic projection: typed, attenuated,
pleasant. When the two disagree, the broker wins and the request is refused.

A generated SDK is still worth building — it makes the sanctioned path the
obvious one and gives compile-time feedback — but it is on the declaration
side of S-B1.

### S-K4 — the transport is language-neutral

`chief-agent-stdio-v1` (D18, Level 4) already requires only stdin, stdout,
JSON, and base64. Every language can speak it with no SDK, no FFI, and no
runtime dependency. The generated SDK layers over this protocol; it does not
replace it.

---

## Platform rules

### S-P1 — every capability maps to a primitive with an honest coverage label

`capability-os-sandbox` already lowers manifests into per-platform plans and
labels each rule `direct`, `brokered`, `launch_time`, or `advisory`. Those
labels are load-bearing and must not be rounded up.

| Platform | Primitives | Privilege | Floor |
|---|---|---|---|
| **OpenBSD** | `pledge`, `unveil`, `fd_inheritance` | none | 5.9 / 6.4 |
| **FreeBSD** | Capsicum `capability_mode`, `cap_rights`, `fd_rights`; jail `rctl`, `vnet_firewall` | none | 9+ |
| **Linux** | `seccomp` (process, clock, brokered resolver), `landlock.path_beneath`, `mount_namespace`, `cgroup_bpf.sock_addr`, `fd_table` | none for seccomp + Landlock | 3.5 seccomp; 5.13 Landlock |
| **macOS** | `seatbelt.profile`, `posix_spawn.file_actions`, env allowlist | none | 10.5+ |
| **Windows** | AppContainer ACL and network capability, `restricted_token`, `job_object`, `process_mitigation.dll_policy`, `handle_inheritance` | none for AppContainer | 8+ |

Version floors that govern design choices:

- **Landlock network rules require kernel 6.7+.** Debian 12 ships 6.1; RHEL 9
  ships 5.14. Network denial is therefore **seccomp's** job (deny `socket`),
  which works back to 2012 — not Landlock's.
- **Unprivileged user namespaces are restricted by default** on Ubuntu 23.10+
  via AppArmor, and historically on Debian. Nothing in the required path may
  depend on namespaces.
- **macOS `sandbox_init` is formally deprecated** (since 10.8) and remains what
  production browsers use. There is no supported alternative for spawning
  sandboxed children; this is accepted with the deprecation recorded.

### S-P2 — anything not enforceable directly is brokered

A capability whose platform coverage is `advisory` is not granted. It is
brokered. `advisory` describes a primitive that narrows a class of behavior
without constraining the exact target, which is a useful defence and not an
enforcement claim.

### S-P3 — a platform may not silently degrade

If a target platform cannot enforce a declared capability at the level the
manifest requires, the launch fails loudly. It does not proceed with weaker
enforcement. A sandbox that quietly becomes advisory is worse than no sandbox,
because the deployment believes it is contained.

---

## Capacity

The sandbox is not the scaling constraint, and design effort should not be
spent as though it were.

**Steady state.** seccomp runs a cBPF filter per syscall — order 100-300ns with
a well-structured filter, roughly 1-3% on syscall-heavy workloads and
unmeasurable on compute. Landlock hooks path operations only, not every
syscall. AppContainer resolves to token checks the OS already performs. This
design's agents are close to the best case: an agent whose only egress is one
descriptor makes almost no syscalls.

**Spawn cost.** Linux seccomp plus Landlock is sub-millisecond. macOS
`sandbox_init` compiles a profile, single-digit milliseconds. Windows
AppContainer is tens of milliseconds and is the expensive one, though profiles
can be created once and reused. Namespaces cost milliseconds and are excluded
by S-P1 anyway. Against a 30-300ms language-runtime boot, all of it is noise.

**What actually limits fan-out**, in order:

1. **Memory.** At 300 agents: Python around 40MB each (~12GB), Node around
   55MB (~16GB), Go or Rust around 8MB (~2.4GB). Language choice is a ~5x
   scaling lever and dominates every sandbox decision.
2. **Descriptors in the supervisor.** Two to three per agent; 300 agents is
   ~900 against a soft `RLIMIT_NOFILE` still commonly 1024. This fails before
   anything else and looks like an unexplained spawn error.
3. **Supervisor I/O model.** `chief-of-staff-process-supervisor` is
   thread-per-child with blocking reads. Threads blocked on `read` are not
   runnable and Rust's 2MiB default stack is virtual, so several hundred
   agents cost single-digit MB resident. Adequate to the low hundreds;
   revisit past ~1000.

**One non-obvious cost.** Installing a seccomp filter causes many kernels to
apply speculative-store-bypass mitigation to the process
(`spec_store_bypass_disable=seccomp`), which can cost several percent. It is
tunable per-process via `prctl(PR_SET_SPECULATION_CTRL)`. Measure it on the
target kernel before drawing conclusions from a first benchmark; it is the
usual explanation for seccomp underperforming published numbers.

---

## What this spec does not decide

- **Whether the launcher shim (S-I4 option 1) is per-language or one binary.**
  A shim must run inside the agent's process, so it is at minimum per-runtime.
  Whether that is a library the SDK links or a trusted stage the supervisor
  injects is open.
- **Which capabilities earn direct grants.** S-I6 sets the default and requires
  a measurement; it does not pre-approve any promotion.
- **The sign-time analyzer for languages other than Go.** S-B2 requires that
  confidence be stated per language. It does not specify the analyzers.
- **Whether the broker is one process per agent or one per supervisor.** Per
  agent is simpler to reason about and matches S-K2; per supervisor amortizes
  the channel. Unresolved, and it interacts with the capacity numbers above.

---

## Build order

Tier A covers realistic deployment. Tier B is small and validates the model
before the expensive platform is attempted.

**Tier B first, deliberately.** OpenBSD `pledge`/`unveil` and FreeBSD Capsicum
are the two platforms where "only the descriptor you were given" is enforced
natively and in a few lines. Getting them working proves the shape of S-I1
through S-I3 before two weeks are spent on Windows.

1. **Descriptor-table audit and the channel contract** (S-I2, S-I3). Platform
   independent, and every later step depends on it.
2. **OpenBSD `pledge`/`unveil`; FreeBSD Capsicum** (Tier B). Days, not weeks.
   Proves the model.
3. **Linux `seccomp` + Landlock** (Tier A). Deny-all with one descriptor;
   network denial via seccomp per S-P1. Covers CI and most deployment.
4. **The launcher shim** (S-I4). Required before interpreted agents get true
   deny-all.
5. **macOS Seatbelt** (Tier A).
6. **Windows AppContainer** (Tier A). The expensive one; schedule accordingly.
7. **Wire into `spawn_verified`**, and delete `DENO_FLAGS`. Deno becomes one
   supported runtime rather than the enforcement mechanism.

Steps 1-3 are the smallest useful increment: they remove the Deno dependency
on Linux, which is where CI runs.

---

## Citations

- Watson, Anderson, Laurie & Kennaway, *Capsicum: practical capabilities for
  UNIX* — the pre-opened-descriptor model behind S-I2.
- de Raadt et al., OpenBSD `pledge(2)` and `unveil(2)` — the minimal-surface
  argument behind Tier B.
- Saltzer & Schroeder, *The Protection of Information in Computer Systems* —
  least privilege and complete mediation; S-B1 is complete mediation restated.
- spec 13 Capability Security — the six-layer model and the 19-pair taxonomy.
- D21 Capability Cage — the three enforcement rings this spec's layer 6 completes.
- D18R Chief of Staff Supervision Tree — the tier vocabulary and `spawn_verified`.
