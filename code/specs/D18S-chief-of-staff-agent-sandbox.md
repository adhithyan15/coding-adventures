# D18S — Chief of Staff Agent Sandbox and Capability Broker

## Overview

An agent declares what it needs, is launched into an OS sandbox that can grant
nothing else, and reaches the rest of the system through one inherited channel.
Everything it is allowed to do, it does by asking. Everything it is not allowed
to do, it cannot express.

This spec fixes where enforcement lives. Four mechanisms in this stack look like
they enforce capabilities, and only two of them do:

| Mechanism | What it actually gives you |
|---|---|
| Sign-time source analysis | The declaration is honest. A **check**, not a boundary. |
| Generated capability SDK | The sanctioned path is typed and obvious. **Ergonomics**, not a boundary. |
| OS sandbox at spawn | The agent cannot *express* the operation. **A boundary.** |
| Broker request handling | The agent's request is not *permitted to achieve* the operation. **A boundary.** |

Conflating the first two with the last two is the central risk. A generated SDK
that omits a function has removed a convenience, not a capability: the process
still owns its address space and can write whatever bytes it likes to whatever
descriptor it holds.

**Depends on:** D18 Chief of Staff, D18D Tool API, D18R Supervision Tree,
spec 13 Capability Security, D21 Capability Cage.

**Amends D18R.** D18R's "What this spec does not decide" left two questions
open. This spec closes both: host profiles live in the **signed agent
manifest**, and supervisor-to-agent transport is **stdio, and is the only
channel the agent has** (S-I2).

**Amends spec 13.** Spec 13's threat-scenario tables describe Layers 3-5 as
*blocking* an attacker. Under S-B1 they detect and refuse to sign; they do not
prevent execution. Those tables are reworded accordingly. Spec 13's Layer 0b
(No Install Hooks) is retained and is load-bearing for S-I4.

**Removes a dependency.** Today's deny-all is Deno's permission system
(`chief-of-staff-host-runtime/src/package.rs`, `DENO_FLAGS`). That is a real
boundary, but available to one language runtime. This spec adds an OS-level
boundary that is language-agnostic by construction, so an agent may be written
in any language.

---

## Terminology

Continues D18R's OTP vocabulary. New terms:

| Term | Meaning |
|---|---|
| **broker** | The supervisor-launched process that holds the secure channel and mediates every capability the agent exercises. A boundary, not a convenience. |
| **channel** | The inherited descriptor pair connecting an agent to its broker. The agent's entire view of the outside world. |
| **shim** | The supervisor-owned trusted stage that installs the sandbox inside the agent process before agent code runs. |
| **direct grant** | A capability the OS can enforce narrowly, granted to the agent process itself. |
| **brokered capability** | A capability denied at the OS level and available only by asking the broker. |
| **declaration check** | Sign-time or CI analysis. Confirms the manifest matches the source. Not enforcement. |

`host` remains reserved for `chief-of-staff-host`, per D18R.

---

## Where enforcement lives

Spec 13 defines Layers 0 through 6. This spec adds Layer 7 and states which
layers an attacker must actually defeat:

```
Layer 0   Zero external dependencies      declaration
Layer 0b  No install hooks                declaration  (load-bearing for S-I4)
Layer 1   Capability manifest             declaration
Layer 2   Secure wrappers                 declaration
Layer 3   Linter + banned constructs      declaration check
Layer 4   CI gate (static analysis)       declaration check
Layer 5   Hardware-key approval           authorization of the declaration
Layer 6   Sandbox fuzz verification       declaration check (CI, dynamic)
Layer 7   OS sandbox at spawn             ENFORCEMENT      <- new in D18S
          Broker request handling         ENFORCEMENT      <- new in D18S
```

Note that spec 13's Layer 6 is a CI fuzz harness, not the OS sandbox. An
engineer told to "make it impossible at layer 6" would land on a test job.

### S-B1 — enforcement lives in exactly two places, and neither is a document

The **OS sandbox** governs what the agent can express. The **broker** governs
what the agent's requests are permitted to achieve. Layers 0-6 govern what an
agent *says* it does and who approved it.

No design document, review, or gate may describe Layers 0-6 as preventing an
agent from performing an operation. They establish that the agent *declared*
the operation and that a human approved the declaration. If an operation must
be impossible, it must be impossible at Layer 7.

The corollary is the part most easily missed: because S-I6 brokers capabilities
by default, **the broker mediates most operations in the running system**. It
acts with supervisor privilege on agent-supplied arguments and is therefore a
classic confused deputy. It is reviewed as a boundary, and S-K5 governs it.

### S-B2 — sign-time analysis states its own confidence, per language

`ca-capability-analyzer` walks a Go AST, compares detected capabilities against
declared ones, and bans the escape hatches that would defeat it —
`unsafe.Pointer`, `import "C"`, `plugin.Open`, `reflect.Value.Call`,
`reflect.Value.MethodByName`, `//go:linkname`. That is the correct technique:
rather than analyzing around reflection and FFI, shrink the language to an
analyzable subset and require an explicit declared `ffi:call:*` capability to
opt back out.

The technique's cost scales with how dynamic the language is, and must be
recorded rather than glossed:

| Language | Confidence | Why |
|---|---|---|
| Go | **High**, with gaps | The analyzer walks `go/ast`. Hand-written assembly (`.s`), `//go:cgo_import_dynamic`, and direct `syscall`/`x/sys` calls are outside its view. |
| Rust | **Unassessed** | No analyzer exists in this repo. `build.rs` and procedural macros execute arbitrary code *at build time*, before any manifest is meaningful. |
| Python, JS, Ruby | **Best-effort** | `getattr`, `eval`, `__import__`, `globalThis[x]`, dynamic `import()` are idiom, not escape hatches. Banning them rejects the ecosystem. |
| Any language with native extensions | **None for the native part** | A C extension calls `open(2)` directly. No source analysis sees it. |

Two problems compound this and are not solvable by better parsing: the analyzer
must cover the **whole third-party dependency closure**, not just the agent's
own files, and **compiled native code is invisible to source analysis**.

This is acceptable precisely because of S-B1. With Layer 7 holding, an
imperfect analyzer degrades from a security hole into a developer-experience
feature: a clear rejection at sign time instead of an opaque `EPERM` at runtime.

---

## Isolation rules

### S-I1 — the agent process is deny-all by default

Every agent is launched with no network, no filesystem, no subprocess
execution, no FFI or dynamic loading, and no environment inheritance, except
what S-I6 grants explicitly.

The design makes this cheap: because capabilities are exercised by asking the
broker, the sandbox never has to express "allow reading this path" in OS terms.
Expressing *allow* rules is where OS sandboxing becomes version-dependent and
expensive; expressing deny-all is the cheapest thing every platform primitive
does, and two of them do it natively.

**Denied syscall classes.** Path-based mediation does not cover the IPC
surface. Every agent filter denies, at minimum:

- `io_uring_setup`, `io_uring_enter`, `io_uring_register` (see S-P1)
- `ptrace`, `process_vm_readv`, `process_vm_writev`, `kcmp`
- `kill`, `tgkill`, `tkill`
- SysV IPC (`shmget`, `semget`, `msgget`) and POSIX message queues (`mq_open`)
- `bpf`, `perf_event_open`, `userfaultfd`, `keyctl`, `add_key`
- `mount`, `pivot_root`, `unshare`, `setns`, `personality`
- `socketcall` and every other syscall multiplexer

`/proc`, `/sys`, `/dev/shm`, and `/dev` (except `/dev/null` and
`/dev/urandom`) are unreachable in every agent's filesystem domain.

### S-I1a — the base deny is unconditional and is not derived from the manifest

The deny-all base policy is installed identically for a zero-capability agent
and for a maximally-capable one. Manifest lowering may only **subtract** from
it.

This rule exists because the natural implementation inverts it. A plan built as
`manifest.capabilities().map(lower_capability)` produces, for a
zero-capability agent — the strictest agent in the system — an **empty plan**,
and "empty plan" becomes "install nothing," which is allow-all. That is
fail-closed for Landlock (an empty ruleset denies every path) and fail-**open**
for seccomp, for the environment block, and for descriptor handling.

The plan type carries the base policy as a first-class term, and an enforcer
refuses to launch when the base policy is absent.

### S-I2 — the agent's channel reaches the broker, not the supervisor

The channel is the agent's entire view of the outside world. It carries
`chief-agent-stdio-v1` (S-K4) on stdin and stdout; no fourth descriptor is
inherited (S-I3).

**fd 2 is decided here rather than derived.** It is bound to a broker-side sink
(or `/dev/null`) that is never read as protocol. It is not the protocol stream:
runtime diagnostics — Python tracebacks, Node warnings, glibc assertion
messages — would otherwise be injected mid-frame into the broker's input. Nor
does it go to the supervisor, which would contradict this rule's own headline
claim by opening an unmediated agent-to-supervisor byte channel. Nor is it left
closed, because the runtime's first `open()` would then land on fd 2 and
stderr writes would corrupt it.

The protocol is length-framed, and a frame that fails to parse **terminates the
channel** rather than resynchronizing. It does not reach the
supervisor. An agent that discovers the supervisor's address, socket path, or
PID can do nothing with the knowledge, because it has no syscall with which to
act on it.

State the reason correctly. The agent cannot reach the supervisor **because the
sandbox removed its ability to create a channel** — not because no channel was
configured for it. A design that relies on "we did not give it the code" has
returned to the error S-B1 exists to prevent.

**No channel descriptor may be a terminal.** Each is a `socketpair` or pipe,
verified with `isatty` before control transfers. A tty grants
`ioctl(TIOCSTI)`, which pushes characters into the controlling terminal's input
queue — command execution as the user running the supervisor. `TIOCSTI` and
`TIOCLINUX` are denied in every filter regardless, because Landlock does not
mediate `ioctl` on device nodes below ABI v5 (kernel 6.10).

### S-I3 — descriptor isolation is enforced by construction, not by audit

Every descriptor in the supervisor is opened `O_CLOEXEC` (Windows:
`WSA_FLAG_NO_HANDLE_INHERIT`) **atomically at the open site**, and the child
calls `close_range(3, ~0U, 0)` between fork and exec. On Windows, handles are
passed with an explicit `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`, never with
`bInheritHandles=TRUE` alone.

A spawn-time descriptor-table audit is retained as a CI lint, not as the
mechanism. The supervisor is thread-per-child; any other thread that calls
`open()` without `O_CLOEXEC` between the audit and the `execve` leaks that
descriptor. An audit is a check, and by S-B1's own logic a check is a
declaration.

A leaked descriptor is a complete escape that needs no syscall the sandbox
denies, because the process already holds the object.

### S-I4 — the sandbox is installed before the runtime creates a thread or runs agent code

Interpreted runtimes need filesystem access to start: Python reads its standard
library and `dlopen`s shared objects, Node resolves its own modules. A sandbox
applied at `exec` therefore cannot be literally deny-all for these runtimes.
The naive resolution — "let the runtime boot, then sandbox" — fails three ways,
and each failure is silent.

**S-I4a — runtime initialization is inside the agent's trust domain.**
"Before agent code" is false wherever module resolution reads from the agent
package. Python executes `.pth` files in `site-packages`, plus
`sitecustomize.py`, `usercustomize.py`, and `PYTHONSTARTUP`. Node honours
`NODE_OPTIONS=--require ./x.js`. Every ELF/Mach-O runtime honours `LD_PRELOAD`,
`LD_AUDIT`, `DYLD_INSERT_LIBRARIES`, `GCONV_PATH`.

The environment passed to an agent is therefore a **closed set defined by the
supervisor**: anything not enumerated is dropped. It is not a manifest-extensible
allowlist with a deny-list overlay, because a deny-list is an enumeration and
this spec claims language-agnosticism by construction — a new runtime would
need a new deny-list entry before it was safe. Grantable names are a fixed set
containing no loader, interpreter, profiler, locale, or module-search variable.

A deny-list is retained only as a redundant second check. It must name at least
`LD_*`, `GLIBC_TUNABLES` (the CVE-2023-4911 vector), `DYLD_*`, `PYTHON*`,
`NODE_OPTIONS`, `NODE_PATH`, `ELECTRON_RUN_AS_NODE`, `BASH_ENV`, `ENV`,
`SHELLOPTS`, `PS4`, `RUBYOPT`, `RUBYLIB`, `GEM_HOME`, `GEM_PATH`, `PERL5OPT`,
`PERL5LIB`, `CLASSPATH`, `JAVA_TOOL_OPTIONS`, `_JAVA_OPTIONS`,
`JDK_JAVA_OPTIONS`, `DOTNET_STARTUP_HOOKS`, `CORECLR_PROFILER`, `COMPlus_*`,
`DOTNET_*`, `LUA_PATH`, `LUA_CPATH`, `JULIA_LOAD_PATH`, `R_PROFILE`,
`GCONV_PATH`, and `LOCPATH` — every one of which achieves code execution during
runtime initialization. The list is illustrative of the closed set's necessity,
never a substitute for it. A manifest naming one is rejected at sign
time and again at launch. The shim runs before site and module initialization
(`-S -I` for CPython, no preload for Node), or the runtime image is a
supervisor-owned tree the agent cannot write. Spec 13's Layer 0b is what keeps
install hooks from re-opening this.

**S-I4b — the sandbox must be installed while the process is single-threaded.**
On Linux, `prctl(PR_SET_SECCOMP, ...)` applies to the **calling thread only**;
only `seccomp(SECCOMP_SET_MODE_FILTER, SECCOMP_FILTER_FLAG_TSYNC, ...)`
synchronizes siblings, and TSYNC fails if any sibling cannot be synced.
`landlock_restrict_self()` has **no thread-sync flag at all** — threads created
before the call never receive a domain.

By the time an interpreted runtime has initialized it is multithreaded: Node
starts the libuv threadpool, the JVM and CPython-with-native-extensions
likewise. Any agent code reaching a libuv worker then executes with no filter
and no Landlock domain, while the install reported success.

The shim installs the sandbox before the runtime creates any thread, or the
launch fails. It verifies this by reading `/proc/self/task` and requiring
exactly one thread. Where seccomp must be installed later, it uses TSYNC and
aborts on TSYNC failure. **A Landlock domain installed after runtime boot on a
multithreaded runtime is not an admissible resolution.**

**S-I4c — nothing acquired before the sandbox may survive it.** `cap_enter()`,
`pledge()`, and `unveil()` do not revoke already-open descriptors — that is the
Capsicum design S-I2 cites approvingly. A single pre-sandbox
`open("/", O_DIRECTORY)` yields whole-filesystem access under Capsicum via
`openat`, permanently. Likewise, seccomp filters, Landlock domains, and pledge
promises are inherited only by processes created *after* installation: a
`fork()` in the pre-shim window produces a permanently unsandboxed sibling
holding the channel.

Before transferring control the shim closes every descriptor except the channel
(`close_range`), and verifies that no child process and no additional thread
was created during initialization. **On any platform whose primitive does not
revoke open descriptors — Capsicum, pledge/unveil — an unsandboxed
initialization window is inadmissible: the runtime is sandboxed at `exec` or
the launch fails.**

Compiled agents (Go, Rust) require none of this and get deny-all at `exec`.

**S-I4d — the shim is supervisor-owned, and the mechanism is a pre-exec
wrapper.** A shim linked by the agent package is inside the agent's trust
domain and may simply decline to call itself — the exact error S-K1 identifies
for the broker. But naming the constraint is not enough: S-I4a forbids
`LD_PRELOAD`/`LD_AUDIT`/`DYLD_INSERT_LIBRARIES`, S-I4d forbids linking, and no
Unix has an exec-time sandbox application — `pledge`, `cap_enter`, `seccomp`,
and `landlock_restrict_self` are all *self*-applied. An implementer facing
three mutually-exclusive constraints picks the convenient escape and rebuilds
the defect.

The mechanism is therefore stated, not left open. The supervisor `exec`s a
**supervisor-owned wrapper**, passing the agent binary as a pre-opened
descriptor. The wrapper, in its own process, in this order:

1. sets `PR_SET_NO_NEW_PRIVS` (aborting on failure, per S-P1),
2. runs `close_range` over everything but the channel (S-I4c),
3. installs the base deny policy (S-I1a) while single-threaded (S-I4b),
4. runs the S-P4 self-test,
5. `fexecve`s the agent descriptor.

This works because sandbox state survives `execve`:

| Mechanism | Survives `execve`? |
|---|---|
| seccomp filter (with `NO_NEW_PRIVS`) | yes |
| Landlock domain | yes |
| `pledge`/`unveil` (with `execpromises`) | yes |
| Capsicum capability mode | yes — and `execve` **by path** is then unavailable, so `fexecve` is mandatory, not preferred |

"Sandboxed at `exec`" in S-I4c therefore means: **the first agent-supplied
instruction executes with the plan already installed.** That is achievable on
every platform here, including the BSDs where S-I4c forbids an in-process
initialization window.

### S-I5 — the agent never holds channel keys, and principals are separated

Secure-channel keys live in the broker, which the supervisor launched. They are
never present in the agent's address space.

That claim is **conditional on principal separation**, and is void without it.
Under a shared UID an agent reaches broker memory via `ptrace(PTRACE_ATTACH)`,
`process_vm_readv`, or `/proc/<pid>/mem`, and can signal the broker, the
supervisor, and sibling agents.

Agent, broker, and supervisor are therefore distinct OS principals: distinct
UID on Unix, distinct AppContainer SID on Windows.

**The privilege to allocate them must be provisioned, and its absence is a
launch failure — not a downgrade.** Allocating a distinct UID needs
`CAP_SETUID` or a delegated subuid/subgid range, which sits awkwardly beside
S-P1's "Privilege: none" column for the sandbox primitives themselves. Those
are different privileges at different times and the spec must say so: the
**supervisor** runs privileged, or is given a delegated subuid/subgid range at
install time; the **agent** needs no privilege for any S-P1 primitive. The
allocated principal is carried in the plan as a first-class term, exactly as
S-I1a requires of the base policy, and inability to allocate distinct
principals for agent, broker, and supervisor fails the launch under S-P3.

Without this the natural unprivileged implementation runs every agent and the
broker under one UID, S-I5's confidentiality claim is void **by its own terms**,
S-I7 fails wholesale, and nothing in S-P3 catches it — the launch proceeds and
the audit record says "sandboxed". Broker and supervisor set
`PR_SET_DUMPABLE=0`. `ptrace`, `process_vm_readv`, `process_vm_writev`, and
`kill` are denied in every agent filter (S-I1), and `/proc` and `/sys` are
unreachable in the agent's domain. S-I5's confidentiality claim is not restated
anywhere without these preconditions.

### S-I6 — capabilities are brokered by default; direct grants are opt-in and bounded

A declared capability defaults to **brokered**: denied at the OS level,
available only by asking. A capability may be promoted to a **direct grant**
only where the OS can enforce it narrowly and a measurement justifies it —
brokering per-read over a large data directory is the motivating case.

**Never grantable, direct or brokered:** any `fs:write` whose target
intersects the runtime image, the shim, the broker binary, the agent package
directory, or the manifest. A manifest declaring one is rejected at sign time
and again at launch.

This is a bar on the *capability*, not on promotion, and the distinction is the
whole point. Barring these from promotion alone would pin them to **brokered** —
and brokered means the supervisor-privileged broker performs the write on the
agent's behalf. The agent asks the broker to rewrite the shim, the broker
consults the signed manifest, sees the capability declared, and complies. The
harm S-I4 exists to prevent is target-dependent and mechanism-independent, so
the bar must be too.

**Never eligible for promotion to direct:** any `ffi:*` and any `proc:exec`
(see S-K6, which governs what brokering them may mean). Promotion requires an
OS-level target-exact grant; wildcard targets are never promotable.

Every promotion is recorded in the manifest and visible in review. The default
direction matters: brokered-by-default fails closed, and a direct-by-default
system silently grants whatever the OS happened to allow.

### S-I7 — agents are mutually isolated

Every rule above is written agent-versus-outside; this one states the
horizontal property. An agent may not read, write, signal, or ptrace another
agent. This is enforced by principal separation — distinct UID, distinct
AppContainer profile and SID, distinct jail — and not by the sandbox plan.

An AppContainer profile *is* a SID: sharing one profile across agents gives
them a common security principal, so they share ACL identity and can open each
other's files and named objects. Profiles are never shared; the creation cost
is paid per agent.

---

## Broker rules

### S-K1 — the supervisor launches the broker, and verifies the object it executes

If the agent package ships the broker and the agent starts it, the broker is
inside the agent's trust domain and mediates nothing.

Verifying a path and then `exec`ing that path is a TOCTOU: anything that can
replace the file between the two wins. Digest verification and execution
reference the **same object** — open the file, hash the descriptor, `fexecve`
that descriptor; on Windows, verify and launch from the already-open handle.
All executables are invoked by absolute path with no `PATH` resolution.

### S-K2 — identity is bound at launch and never asserted by the agent

The broker is told which `agent_id` it speaks for, by the supervisor that
already holds the attested value. No field on the wire carries an identity the
agent supplies.

This preserves the existing attestation chain — signed manifest
`manifest.agent` to `HostName` to `HostRegistration` to `agent_id` — which is
what makes the vault's per-secret `allowed_agents` meaningful. An agent
permitted to state its own identity makes every per-secret policy in VLT06
worthless.

### S-K3 — the broker enforces the manifest; the SDK does not

The broker re-validates every request against the signed manifest. The
generated SDK is the manifest's ergonomic projection: typed, attenuated,
pleasant. When the two disagree, the broker wins and the request is refused.

A generated SDK is still worth building — it makes the sanctioned path the
obvious one and gives compile-time feedback — but it is on the declaration side
of S-B1.

### S-K4 — the transport is language-neutral

`chief-agent-stdio-v1` (D18, Level 4) requires only stdin, stdout, JSON, and
base64. Every language can speak it with no SDK, no FFI, and no runtime
dependency. The generated SDK layers over this protocol; it does not replace
it.

### S-K5 — the broker treats every field of every request as hostile

The broker acts with supervisor privilege on agent-supplied arguments. It is
the confused deputy in this design and is reviewed as one.

- **Path arguments** are resolved beneath a pre-opened root descriptor, by the
  platform's own primitive. A validated path is never re-resolved from a string
  afterwards. `openat2` is Linux-only and this spec implements BSD first, so the
  rule is stated per platform rather than left to the implementer — the default
  reach is `realpath()` + `open()`, which is a TOCTOU on a supervisor-privileged
  process taking agent-supplied paths, i.e. the confused deputy S-K5 exists to
  prevent, and it fails silently.

  | Platform | Primitive |
  |---|---|
  | Linux ≥ 5.6 | `openat2` with `RESOLVE_BENEATH \| RESOLVE_NO_SYMLINKS \| RESOLVE_NO_MAGICLINKS` |
  | FreeBSD ≥ 13 | `O_RESOLVE_BENEATH`, or capability-mode `openat` from a rights-limited directory descriptor |
  | OpenBSD | `unveil`-confined `openat` with `O_NOFOLLOW` on every component |
  | macOS ≥ 12 | `openat` with `O_NOFOLLOW_ANY` |
  | Windows | `FILE_FLAG_OPEN_REPARSE_POINT` plus a normalized-prefix check |

  Absence of a beneath-resolution primitive is a launch failure under S-P3, not
  a soft fallback.
- **Requests are length-bounded and rate-limited**, per agent.
- **Parsing is total.** Agent-supplied JSON and base64 are decoded by a parser
  that cannot be driven to unbounded allocation or recursion.
- **The broker never returns a descriptor to the agent** over `SCM_RIGHTS`
  unless that grant is itself a declared direct capability — and the descriptor
  is attenuated before transfer. Passed descriptors are **regular files only,
  never directories**: a directory descriptor hands over the whole subtree via
  `openat`, permanently, and invisibly to Landlock, which mediates path
  resolution rather than held descriptors. Under Capsicum that is exactly the
  whole-filesystem escalation S-I4c warns about, reintroduced through the
  sanctioned path. Descriptors carry the exact rights the capability names and
  no more, never execute rights, and are `cap_rights_limit`ed before transfer
  on FreeBSD.
- **The broker's pre-opened roots are supervisor-chosen** and are proven
  disjoint from S-I6's never-grantable set at broker start, before any request
  is served. A root that *is* the agent package directory makes beneath-resolution
  vacuous.

### S-K6 — a brokered request may never yield authority the agent could not hold directly

This is the invariant that makes brokering safe, and it does not hold by
construction. S-I6 brokers by default; barring a class from *direct* grant
therefore pins it to *brokered*, where the operation is performed by the one
process holding the channel keys (S-I5), the vault path, and — if the broker is
per-supervisor — every other agent's channel. For two capability classes that
inverts the boundary completely.

**`ffi:*` and every `dlopen`-shaped capability are `Unsupported` by default and
are never executed in the broker's address space, under any circumstances.**
Calling an agent-chosen native function with agent-chosen arguments inside a
supervisor-privileged process is a complete Layer 7 defeat reachable through a
declared, reviewed, hardware-key-approved capability. Where supported at all,
such a call runs in a fresh helper launched under a plan at least as strict as
the requesting agent's, as the **agent's** principal.

**Brokered `proc:exec` spawns only through the same `spawn_verified` path used
for agents**: supervisor-owned wrapper, the agent's principal, a sandbox plan
that is a subset of the requester's, and targets restricted to a
manifest-declared digest-pinned set. Never `PATH`-resolved. A broker-spawned
child must not inherit the broker's principal or its unsandboxed state.

### S-K7 — the broker is itself contained

S-I3's descriptor discipline applies to **every** supervisor-launched process,
the broker included: it does not inherit the supervisor's descriptor table —
vault handles, audit-log descriptors, other agents' channels.

The broker runs under its own sandbox plan, holding no authority beyond the
union of the capabilities its agents declare.

Whether there is one broker per agent or one per supervisor is therefore a
**blast-radius** decision before it is a capacity one: it determines whether an
S-K5 parsing defect compromises one agent or all of them. Per-supervisor is
admissible only if the broker is internally compartmented per agent; otherwise
per-agent is required.

---

## Platform rules

### S-P1 — every capability maps to a primitive with an honest coverage label

`capability-os-sandbox` lowers manifests into per-platform plans labelled
`direct`, `brokered`, `launch_time`, or `advisory`. Those labels are
load-bearing and are never rounded up.

| Platform | Primitives | Privilege | Floor |
|---|---|---|---|
| **OpenBSD** | `pledge`, `unveil`, `fd_inheritance` | none | 5.9 / 6.4 |
| **FreeBSD** | Capsicum `capability_mode`, `cap_rights`, `fd_rights`; jail `rctl`, `vnet_firewall` | none | **11+** |
| **Linux** | `seccomp` (process, clock, brokered resolver), `landlock.path_beneath`, `fd_table` | none | 3.5 seccomp; Landlock ABI-negotiated |
| **Linux, privileged mode only** | `mount_namespace`, `cgroup_bpf.sock_addr` | `CAP_BPF`/`CAP_NET_ADMIN`, cgroup delegation | — |
| **macOS** | `seatbelt.profile`, `posix_spawn.file_actions`, env allowlist | none | 10.5+ |
| **Windows** | AppContainer ACL and network capability, `restricted_token`, `job_object`, `process_mitigation.dll_policy`, `handle_inheritance` | none for AppContainer | 8+ |

**Required seccomp filter shape.** "Deny `socket`" is not a filter design.

1. Every filter begins with an `arch` equality test against the single
   permitted ABI and kills on mismatch. Without it the filter is bypassable via
   the i386 compat ABI or x32, where syscall numbers differ.
2. **`io_uring_setup`, `io_uring_enter`, and `io_uring_register` are denied
   unconditionally.** seccomp filters syscall *entry*; io_uring submits
   `IORING_OP_SOCKET`, `IORING_OP_CONNECT`, `IORING_OP_OPENAT` to kernel-side
   workers that never pass through the filter. A filter permitting io_uring is
   not a boundary. Landlock does not cover the gap: it has no network rules
   below ABI v4 (6.7), which S-P1's own version analysis excludes.
3. Multiplexers are denied alongside their unmultiplexed forms — `socketcall`
   reaches `socket`/`connect`/`bind` on 32-bit ABIs.
4. `PR_SET_NO_NEW_PRIVS` is set before any sandbox install; its failure aborts
   the launch. Without it `seccomp(SET_MODE_FILTER)` and
   `landlock_restrict_self()` both fail unprivileged, and it is the only thing
   that neutralizes setuid binaries.
5. Filters are a deny-by-default **allowlist** of syscall numbers, never a
   denylist.

**Landlock is ABI-negotiated, not version-assumed.** The running ABI comes from
`landlock_create_ruleset(NULL, 0, LANDLOCK_CREATE_RULESET_VERSION)`.

| Right | ABI | Kernel |
|---|---|---|
| `FS_REFER` | v2 | 5.19 |
| `FS_TRUNCATE` | v3 | **6.2** |
| network TCP bind/connect | v4 | 6.7 |
| `FS_IOCTL_DEV` | v5 | 6.10 |
| `SCOPE_SIGNAL`, `SCOPE_ABSTRACT_UNIX_SOCKET` | v6 | 6.12 |

Debian 12 ships 6.1 and RHEL 9 ships 5.14, so on both **`truncate(2)` is
unmediated by Landlock**: a grant described as read-only does not stop the
agent truncating any file DAC permits, including vault and audit files. Rights
unavailable at the running ABI are never silently dropped — the kernel's
`best_effort` idiom is forbidden here by S-P3. The capability is downgraded to
brokered, or the launch fails. **No filesystem capability may be granted
`direct` below ABI v3.**

**Network denial is seccomp's job**, not Landlock's, given those floors.

**Unprivileged user namespaces are restricted by default** on Ubuntu 23.10+ via
AppArmor and historically on Debian. Nothing in the required path depends on
namespaces.

**macOS `sandbox_init` is formally deprecated** (since 10.8) and remains what
production browsers use. There is no supported alternative for spawning
sandboxed children; this is accepted with the deprecation recorded.

**Every sandbox-install call site checks its return value and aborts the launch
on failure.** `ENOSYS` from `cap_enter` on a kernel without `options
CAPABILITIES`, `EOPNOTSUPP` from `landlock_create_ruleset`, and `EINVAL` from
`seccomp` are launch failures under S-P3, never warnings. An unchecked return
leaves the process entirely unsandboxed while the plan reports success.

### S-P2 — advisory coverage is not a grant

`Advisory` may not appear in an accepted plan. Lowering that would yield
`Advisory` yields `Brokered` instead, or `Unsupported`, which is a launch
failure. `advisory` describes a primitive that narrows a class of behavior
without constraining the exact target: a useful defence, not an enforcement
claim.

The current lowering violates this — `lower_openbsd` maps `Category::Net` and
`Category::Proc` to `pledge`/`Advisory`, and `lower_linux` emits `Advisory` for
wildcard `fs` and for `time`. OpenBSD is implemented first in the build order
and is where the lowering is most advisory, so this rule is a precondition of
step 3, not a later cleanup.

### S-P3 — a platform may not silently degrade

If a target platform cannot enforce a declared capability at the level the
manifest requires, the launch fails loudly. It does not proceed with weaker
enforcement. A sandbox that quietly becomes advisory is worse than no sandbox,
because the deployment believes it is contained.

`SandboxCoverage` gains an `Unsupported` variant so this is representable in
the model rather than by convention, and `summary().advisory_rules == 0` is a
launch precondition.

### S-P4 — enforcement is confirmed before agent code runs

S-P3 requires loud failure when a platform *cannot* enforce. This rule requires
positive confirmation that enforcement *is* live, because the failures in
S-I1a, S-I4b, and S-P1's return-value checks are otherwise silent.

The base filter's default action is `SECCOMP_RET_KILL_PROCESS` (S-P1 item 5).
A self-test that probed such a syscall would die rather than report, so the
launch-time and CI halves are separated rather than conflated:

- **At launch**, the wrapper probes only a narrow set of classes deliberately
  mapped to `SECCOMP_RET_ERRNO` and named in the plan, and requires the
  expected `EPERM`/`EACCES`. Any success aborts the launch.
- **In CI**, the full-coverage negative test runs in the Layer 6 harness, where
  dynamic verification already lives and a killed process is an expected
  outcome.

Demoting every denied syscall to `RET_ERRNO` so a single probe can cover
everything is not admissible: it loses the crash signal, lets an attacker
enumerate the filter at leisure, and is dangerous wherever a caller ignores a
return value. Forking a probe child is likewise barred by S-I4c.

The audit record states which classes were verified at launch and which only in
CI, so it does not overclaim.

---

## Capacity

The sandbox is not the scaling constraint, and design effort should not be
spent as though it were.

**Steady state.** seccomp runs a cBPF filter per syscall — order 100-300ns with
a well-structured filter, roughly 1-3% on syscall-heavy workloads and
unmeasurable on compute. Landlock hooks path operations only, not every
syscall. AppContainer resolves to token checks the OS already performs. This
design's agents are close to the best case: an agent whose only egress is one
channel makes almost no syscalls.

**Spawn cost.** Linux seccomp plus Landlock is sub-millisecond. macOS
`sandbox_init` compiles a profile, single-digit milliseconds. Windows
AppContainer is tens of milliseconds and is the expensive one — and per S-I7
that cost is paid per agent, because profiles are not shared. Against a
30-300ms language-runtime boot it remains noise.

**What actually limits fan-out**, in order:

1. **Memory.** At 300 agents: Python around 40MB each (~12GB), Node around
   55MB (~16GB), Go or Rust around 8MB (~2.4GB). Language choice is a ~5x
   scaling lever and dominates every sandbox decision.
2. **Descriptors in the supervisor.** Three per agent; 300 agents is ~900
   against a soft `RLIMIT_NOFILE` still commonly 1024. This fails before
   anything else and looks like an unexplained spawn error.
3. **Supervisor I/O model.** `chief-of-staff-process-supervisor` is
   thread-per-child with blocking reads. Threads blocked on `read` are not
   runnable and Rust's 2MiB default stack is virtual, so several hundred agents
   cost single-digit MB resident. Adequate to the low hundreds; revisit past
   ~1000.

**One non-obvious cost.** Installing a seccomp filter causes many kernels to
apply speculative-store-bypass mitigation to the process
(`spec_store_bypass_disable=seccomp`), which can cost several percent. It is
tunable per-process via `prctl(PR_SET_SPECULATION_CTRL)`, **but on a host whose
purpose is running mutually distrusting agents, disabling Spectre-v4 mitigation
to recover a few percent is a bad trade.** SSBD is relaxed only for an agent
tier explicitly marked trusted, never for an agent running third-party code,
and the relaxation is recorded in the manifest and visible in review.

---

## What this spec does not decide

- **Which capabilities earn direct grants.** S-I6 sets the default, bars a
  class outright, and requires a measurement; it pre-approves nothing.
- **The sign-time analyzer for languages other than Go.** S-B2 requires
  confidence be stated per language. It does not specify the analyzers.
- **Whether the broker is one process per agent or one per supervisor.** Per
  agent is simpler to reason about and matches S-K2; per supervisor amortizes
  the channel. It interacts with the capacity numbers above.
- **Whether `DENO_FLAGS` is deleted or retained.** The spec calls it a real
  boundary. Deleting a working in-process boundary once the OS boundary lands
  is a net reduction, not a cleanup; retaining it as defence in depth for Deno
  agents costs nothing. Decide with a measurement, not on tidiness.

Decided, and recorded here because earlier drafts left it open: the launcher
shim is **supervisor-owned and injected** (S-I4d), never a library the agent
package links.

---

## Build order

Tier A covers realistic deployment. Tier B is small and validates the model
before the expensive platform is attempted.

**Tier B first, deliberately.** OpenBSD `pledge`/`unveil` and FreeBSD Capsicum
are the two platforms where "only the descriptor you were given" is enforced
natively and in a few lines. Getting them working proves the shape of S-I1
through S-I3 before two weeks are spent on Windows.

1. **Base-deny plan model** (S-I1a, S-P2, S-P3): add the unconditional base
   policy term and the `Unsupported` coverage variant; assert no `Advisory`
   rule survives lowering. Platform independent, and every later step is
   unsound without it.
2. **Descriptor isolation and the channel contract** (S-I2, S-I3):
   `O_CLOEXEC` at every open site, `close_range` in the child, no-tty check.
3. **OpenBSD `pledge`/`unveil`; FreeBSD Capsicum** (Tier B). Days, not weeks.
   Proves the model. Requires step 1 because the current OpenBSD lowering is
   the most advisory in the tree.
4. **Linux `seccomp` + Landlock** (Tier A): arch check, io_uring denial,
   allowlist filter, ABI-negotiated Landlock. Covers CI and most deployment.
5. **The shim** (S-I4, S-P4): single-thread precondition, env deny-list,
   `close_range`, negative self-test. Required before any interpreted agent
   gets true deny-all.
6. **Broker hardening** (S-K5) and principal separation (S-I5, S-I7).
7. **macOS Seatbelt** (Tier A).
8. **Windows AppContainer** (Tier A). The expensive one; schedule accordingly.
9. **Wire into `spawn_verified`.** Deno becomes one supported runtime rather
   than the enforcement mechanism.

Steps 1-4 are the smallest useful increment: they give compiled agents a real
OS boundary on Linux, which is where CI runs.

**No step before 6 is deployable outside a single-tenant CI runner.** Principal
separation (S-I5, S-I7) and broker hardening (S-K5, S-K6, S-K7) land at step 6;
before it, S-I5's confidentiality claim is void by its own terms and the
confused-deputy surface is unhardened. `spawn_verified` wiring (step 9) is the
only supported entry point for a multi-agent deployment.

---

## Citations

- Watson, Anderson, Laurie & Kennaway, *Capsicum: practical capabilities for
  UNIX* — the pre-opened-descriptor model behind S-I2, and the
  descriptors-survive property behind S-I4c.
- de Raadt et al., OpenBSD `pledge(2)` and `unveil(2)` — the minimal-surface
  argument behind Tier B.
- Saltzer & Schroeder, *The Protection of Information in Computer Systems* —
  least privilege and complete mediation; S-B1 is complete mediation restated,
  and S-K5 is why the broker is inside the mediation boundary.
- Hardy, *The Confused Deputy* — the failure mode S-K5 exists to prevent.
- spec 13 Capability Security — Layers 0 through 6 and the 19-pair taxonomy.
- D21 Capability Cage — the enforcement rings this spec's Layer 7 completes.
- D18R Chief of Staff Supervision Tree — tier vocabulary and `spawn_verified`.
