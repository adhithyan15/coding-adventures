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

## Normative status

This document went through four adversarial security-review rounds. Every round
found a CRITICAL, and **every round's CRITICAL was created by the previous
round's fix**: round 2 by the brokered-by-default default, round 3 by the
capability bars, round 4 by the pre-exec wrapper. The invariants were never
overturned; only the mechanism kept breaking.

That is a fact about the activity, not only about the text. Syscall-level
mechanism is being specified in prose with nothing to run it against, and each
mechanical detail creates new interactions with the others. A wrapper that
actually runs, with a negative test suite, settles these questions in an
afternoon and settles them with evidence.

So the two halves of this spec have different status:

**Normative — the invariants.** Stable across all four rounds. Changing one is a
spec amendment.

| | |
|---|---|
| S-B1 | Enforcement is the sandbox and the broker. Layers 0-6 are declaration. |
| S-I1a | The base deny is unconditional and not derived from the manifest. |
| S-I2 | One channel, to the broker, and the *ability* to make another is removed. |
| S-I6 | Brokered by default; the never-grantable set is barred direct **and** brokered. |
| S-K6 | A brokered request may never yield authority the agent could not hold directly. |
| S-K2 | Identity is bound at launch, never asserted by the agent. |
| S-P3 | A platform that cannot enforce fails the launch. It never degrades quietly. |

**Non-normative — the mechanism.** Everything at the level of syscall names,
flag combinations, ordering within the wrapper, and per-platform primitive
choice is *implementation guidance*. It records what four review rounds
established so an implementer does not rediscover it, and it is expected to be
corrected by the implementation and its tests. A mechanism detail that turns out
wrong is a code review finding, not a spec amendment — **provided the invariant
above it still holds.**

The build order's step 1 exists to make this real: the base-deny plan model and
its tests come before any platform work, so the mechanism has something to be
validated against.

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
acts on agent-supplied arguments and is therefore a
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
- `socket(AF_UNIX, ...)` — denied wholesale when no declared capability needs
  it. seccomp cannot distinguish abstract-namespace from pathname unix sockets
  (the name lives in the `sockaddr`, not a register), so the family is denied or
  it is not.
- the `pidfd` family — `pidfd_open`, `pidfd_send_signal`, `pidfd_getfd` (which
  steals descriptors from another process), and `prlimit64` against a foreign
  pid, `process_madvise`, `sched_setaffinity`, `setpriority`, `ioprio_set`

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

**Two further terms are recorded in the plan and in the launch audit record
(S-P4): `principal_model` (`shared_uid` | `distinct_uid` | `appcontainer_sid`)
and `broker_topology` (`per_agent` | `per_supervisor`).** A relaxation is
defensible only when the weaker outcome is *visible*: without these a deployment
cannot tell from its own records which model it actually got, and "the launch
proceeds and the audit record says sandboxed" is the exact failure S-P3 and S-P4
exist to prevent. `broker_topology: per_supervisor` is a launch failure under
S-K7. Observing and refusing costs no privilege.

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
need a new deny-list entry before it was safe. Grantable names are **enumerated in this spec**, carried in the plan as a
first-class term alongside the base policy (S-I1a), and applied at both the
supervisor's `exec` of the wrapper and the `envp` handed to the agent exec.
Additions require a spec amendment. Defining the set negatively — "containing no
loader or interpreter variable" — would be a category deny-list, the construct
this rule rejects.

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
exactly one thread. Under S-I4d there is no case in which seccomp is
installed after threads exist, and no such case is admissible; TSYNC is retained
only as belt-and-braces on a provably single-threaded wrapper. **A Landlock domain installed after runtime boot on a
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
2. runs `close_range` over the complement of an **exactly enumerated** survivor
   set — stdin, stdout, the fd-2 sink (S-I2), and the agent binary descriptor —
   asserting afterwards that the surviving table is precisely those four. "All
   but the channel" taken literally closes the binary descriptor step 5 needs
   and the fd-2 sink S-I2 forbids leaving closed; the implementer then invents a
   wider retained range or re-opens after the policy is installed, reopening the
   S-I4c property this step exists to enforce. The binary descriptor is
   `O_CLOEXEC` so it does not survive into the agent,
3. installs the base deny policy (S-I1a) while single-threaded (S-I4b),
4. runs the S-P4 self-test,
5. `execveat`s the agent descriptor.

**The wrapper's own mechanism grants the agent `exec`, and this must be
constrained or S-I1 is false.** The wrapper installs the base policy at step 3
and execs at step 5, so the exec syscall must be in the base allowlist — and
seccomp filters are inherited unchanged across `execve`, as the table below
says. The agent therefore holds `execveat` permanently unless something stops
it. For a compiled agent with an empty Landlock ruleset the damage is bounded,
because no path carries EXECUTE. For an **interpreted** agent, which must be
granted read+execute over the supervisor-owned runtime image, it is a full
bypass: re-exec the interpreter with attacker-chosen argv, or any helper binary
in that tree, defeating S-K6's digest-pinned target set. Silently — the plan
reports success.

Either constraint is admissible, and one is required:

- `execveat` is permitted only with argument-matched `AT_EMPTY_PATH` on the
  single pre-opened binary descriptor number, **and**
  `LANDLOCK_ACCESS_FS_EXECUTE` is withheld from every path rule including the
  runtime image; or
- `SECCOMP_FILTER_FLAG_NEW_LISTENER`, with the supervisor permitting exactly
  one exec and denying thereafter.

A base allowlist containing unconstrained `execve`/`execveat` is not an
admissible plan. "Filters survive `execve`" cuts both ways.

OpenBSD is safe here by construction — `pledge` `execpromises` applies the
strict set *after* exec — and FreeBSD is safe because capability mode has no
path-based exec and the agent holds no directory descriptor. **Linux is the
hole, and Linux is Tier A.**

This works because sandbox state survives `execve`:

| Mechanism | Survives `execve`? |
|---|---|
| seccomp filter (with `NO_NEW_PRIVS`) | yes |
| Landlock domain | yes |
| `pledge`/`unveil` (with `execpromises`) | yes |
| Capsicum capability mode | yes — and `execve` **by path** is then unavailable, so descriptor-based exec is mandatory; the descriptor needs `CAP_FEXECVE` via `cap_rights_limit` *before* `cap_enter` |

The exec is issued as `execveat(fd, "", ..., AT_EMPTY_PATH)` on a descriptor
opened `O_RDONLY`, **never** `O_PATH` and never through a `/proc/self/fd`
fallback — glibc's `fexecve` resolves that way, and S-I1 makes `/proc`
unreachable, so the launch would fail opaquely *after* the sandbox is installed
and the convenient fix is to grant `/proc/self`, reopening the introspection
S-I5 depends on being closed. A `/proc`-dependent exec path is inadmissible.

S-K1's same-object rule extends to the agent binary: the wrapper execs only a
descriptor whose digest the supervisor verified against the signed manifest.

"Sandboxed at `exec`" in S-I4c therefore means: **the first agent-supplied
instruction executes with the plan already installed.** That is achievable on
every platform here, including the BSDs where S-I4c forbids an in-process
initialization window.

### S-I5 — the agent never holds channel keys

Secure-channel keys live in the broker, which the supervisor launched. They are
never present in the agent's address space.

That claim is **conditional on principal separation**, and is void without it.
Under a shared UID an agent reaches broker memory via `ptrace(PTRACE_ATTACH)`,
`process_vm_readv`, or `/proc/<pid>/mem`, and can signal the broker, the
supervisor, and sibling agents.

**Three things carry that claim, and none of them requires a privileged
supervisor:**

1. **One broker per agent** (S-K7). A broker holds only the keys of the agent
   it serves, so even a total compromise yields the agent nothing it did not
   already hold. This is what makes the remaining measures defence in depth
   rather than the sole line.
2. **The base filter denies process introspection** (S-I1): `ptrace`,
   `process_vm_readv`, `process_vm_writev`, `kcmp`, with `/proc` and `/sys`
   unreachable. An agent cannot read its broker's memory because the syscalls
   are gone, not because the UID differs.
3. **The sandbox scopes process access on every target platform**, and does so
   unprivileged:

| Platform | How agent-to-broker inspection is denied | Privilege |
|---|---|---|
| Linux | Landlock restricts ptracing outside the domain (ABI v1), and seccomp denies the syscalls outright. **Each agent must call `landlock_restrict_self()` in its own process after fork** — a domain shared across agents does not separate them. At negotiated **ABI 0 the seccomp filter is the sole mechanism, and distinct UIDs become required rather than recommended.** | none |
| OpenBSD | `pledge` lacking `ptrace`, `proc`, and `ps`; procfs has not existed since 5.7 | none |
| FreeBSD | capability mode returns `ECAPMODE` for every global-namespace call; the only process handles are `procdesc` descriptors, and the agent holds none | none |
| Windows | each agent gets its own AppContainer SID; `CreateAppContainerProfile` needs no administrator. **Container names derive from the attested `agent_id`; `ERROR_ALREADY_EXISTS` without a verified matching SID is a launch failure** — otherwise two agents silently share one principal. Agents run as LPAC, or `ALL APPLICATION PACKAGES` (S-1-15-2-1) is explicitly denied on supervisor-owned objects, since every AppContainer token carries it. | none |
| macOS | Seatbelt `(deny mach-priv-task-port)`, `(deny mach-lookup)`, `(deny process-info*)`. `process-info*` alone gates enumeration, **not Mach task-port acquisition, which is the actual memory-read vector.** The broker and supervisor are built with the Hardened Runtime and without `com.apple.security.get-task-allow`; absent that, a same-UID caller can take their task ports regardless of the agent's own profile. | none |

Distinct UIDs on Unix remain **recommended** as defence in depth against a gap
in the base filter — four review rounds established that filters are easy to get
wrong — and where a delegated subuid/subgid range is provisioned at install the
supervisor uses it. Except at Landlock ABI 0 (above), it is not required, its
absence is not a launch failure, and **the supervisor does not run privileged.**

Broker and supervisor suppress core dumps on every platform, not only Linux:
`PR_SET_DUMPABLE=0` on Linux, `PT_DENY_ATTACH` or the Hardened Runtime on macOS,
and a restrictive process DACL on Windows. A core dump is a key-disclosure path
and two of the five platforms had no rule. Broker and supervisor set
`PR_SET_DUMPABLE=0`. `ptrace`, `process_vm_readv`, `process_vm_writev`, and
`kill` are denied in every agent filter (S-I1), and `/proc` and `/sys` are
unreachable in the agent's domain. S-I5's confidentiality claim is not restated
anywhere without these preconditions.

### S-I6 — capabilities are brokered by default; direct grants are opt-in and bounded

A declared capability defaults to **brokered**: denied at the OS level,
available only by asking. A capability may be promoted to a **direct grant**
only where the OS can enforce it narrowly and a measurement justifies it —
brokering per-read over a large data directory is the motivating case.

**Never grantable, direct or brokered, for read *or* write:** any `fs:*` whose
target intersects the runtime image, the wrapper binary, the shim, the broker
binary, the supervisor binary, the agent package directory, the manifest, the
**vault backing store and its directory**, the **audit log and its directory**,
the **sandbox plan and policy files**, or the **principal or subuid mapping, where one exists**.

The vault and audit entries are not optional extras. VLT06's per-secret
`allowed_agents` is enforced by the vault, not by the filesystem, so reading the
backing store directly reads every secret regardless of policy — and under
brokering it is the *authority-bearing broker* that performs the read.
Writing the audit log destroys the record S-P4 and S-I5 depend on. **A
policy-mediated resource is never reachable as a raw path**: the vault is
reachable only through the vault capability. A manifest declaring one is rejected at sign time
and again at launch.

This is a bar on the *capability*, not on promotion, and the distinction is the
whole point. Barring these from promotion alone would pin them to **brokered** —
and brokered means the authority-bearing broker performs the write on the
agent's behalf. The agent asks the broker to rewrite the shim, the broker
consults the signed manifest, sees the capability declared, and complies. The
harm S-I4 exists to prevent is target-dependent and mechanism-independent, so
the bar must be too.

**Never eligible for promotion to direct:** any `ffi:*` and any `proc:exec`
(see S-K6, which governs what brokering them may mean). Promotion requires an
OS-level target-exact grant; wildcard targets are never promotable.

**Overlapping *writable* direct grants across agents are recorded and reviewed
as an inter-agent channel.** Two agents granted the same writable directory have
a fully sanctioned, unmediated bidirectional path, including `flock`-based
signalling. That is a manifest-review gap rather than an OS defect, and it is
exactly what S-I7's narrower wording is meant to keep reviewers looking for.

Per-agent memory bounds are set pre-exec with `RLIMIT_AS`/`RLIMIT_DATA`, which
needs no privilege. cgroup delegation would be stronger but sits in the
privileged row, and the Capacity section names memory as the first thing that
fails — without a bound an agent can drive the broker into the OOM killer.

Every promotion is recorded in the manifest and visible in review. The default
direction matters: brokered-by-default fails closed, and a direct-by-default
system silently grants whatever the OS happened to allow.

### S-I7 — there is no agent-to-agent surface to isolate

This rule states a property of the paradigm, not a mediation requirement.

**There is no *ambient* agent-to-agent surface.** An agent cannot discover,
enumerate, or address an agent it was not explicitly wired to. The only question
it may ask unprompted is whether a capability exists.

That is narrower than "agents do not know other agents exist", and the narrowing
is deliberate: the repo's artifacts do contain agent-addressing surfaces, and a
rule whose premise the artifacts contradict would not survive review.

| Surface | Where |
|---|---|
| `vault.request_direct(secret_name, consumer_agent_id)` — caller names a consumer | `D18-chief-of-staff.md:1996` |
| `ChannelDefinition`: one `originator.agent_id`, 1..1024 `receivers[].agent_id` | `D18P` §channel definition |
| `agent.spawn`, `agent.send`, `agent.await` — delegation tools | `D18D` §delegation |

Every one of those is a **declared, supervisor-wired path**, not an ambient one:
a channel exists because the supervisor wired it, and D18D states that a binding
treating a caller-supplied `consumer_agent_id` as proof of authorization is
non-conforming — the vault authorizes on the *attested* `requesting_agent_id`
(`chief-of-staff-vault-runtime/src/lib.rs:331`), never on the asserted field.
So the paradigm claim holds for *discovery and addressing*, which is what this
rule needs; it does not hold as a claim that no inter-agent path exists.

The OS-level discovery path is closed by S-I1 rather than by principal
separation: `/proc` and `/sys` are unreachable, and the process, IPC and
message-queue interfaces are denied. An agent cannot enumerate a sibling it was
never told about, because the interfaces that would enumerate it are gone.

**The mechanism is S-P1's strict allowlist, not S-I1's enumeration.** S-I1 lists
denied classes so a review can check them; the allowlist is what actually holds,
and anything not allowlisted is denied whether or not it appears in that list.

Earlier drafts of this rule required distinct OS principals as *the* enforcement
mechanism for mutual isolation. That imported a generic multi-tenant threat
model in which mutually distrusting workloads share a namespace and must be kept
apart. This design has no such namespace. Principal separation remains
**recommended defence in depth** (S-I5), not the mechanism, and its absence is
not a launch failure.

Where an OS supplies per-agent principals for free it is still used: an
AppContainer profile *is* a SID and `CreateAppContainerProfile` needs no
administrator, so each Windows agent gets its own and profiles are never
shared.

### S-I8 — an optional in-runtime layer, which may only ever subtract

Where a language runtime has its own capability enforcement, an agent may be
launched under it **in addition to** the OS sandbox. Compromising the agent then
takes two independent steps instead of one.

Three rules keep this defence in depth rather than defence instead of:

1. **It is derived from the same signed manifest.** Two capability sources that
   can disagree is a worse position than one. `chief-of-staff-skill-parser`
   already does this — `deno_permissions()` lowers the same
   `Capability{category, action, ...}` values into `--allow-read`,
   `--allow-net`, `--allow-run`, `--allow-ffi`.
2. **It may only subtract.** The in-runtime layer is never wider than the OS
   plan, and its presence **never** justifies relaxing the OS plan. A Deno agent
   does not get a looser Layer 7 because Deno is watching. That inversion is how
   defence in depth becomes a single point of failure wearing two names.
3. **It is not a boundary under S-B1.** It is enforced by code sharing an
   address space with the code it constrains. Deno's is strong — V8 isolates JS
   from the host and `--deny-ffi` closes the obvious escape — but strong is not
   the same as being the thing the design rests on.

**Availability is poor outside JavaScript, and the industry is moving away from
in-process sandboxing rather than toward it.** This is recorded so nobody plans
around a facility that no longer exists:

| Runtime | In-runtime enforcement | Status |
|---|---|---|
| **Deno** | `--deny-net`, `--deny-read`, `--deny-write`, `--deny-env`, `--deny-sys`, `--deny-run`, `--deny-ffi` | Supported, first-class. Already used here. |
| **Node.js** | `--permission` with `--allow-fs-read`, `--allow-child-process` | Real, but young (experimental in v20) and has had bypasses. Usable with the caveat recorded. |
| **Lua** | restricted environments | Supported; sandboxing is a design goal of the language. |
| **wasm / WASI** | capability-based by construction | Supported, and the only **language-agnostic** option here — see below. |
| **Java / JVM** | `SecurityManager` | **Gone.** Deprecated for removal by JEP 411, permanently disabled by JEP 486. No replacement is planned. |
| **.NET** | Code Access Security | **Removed** in .NET Core. |
| **Ruby** | `$SAFE` taint mode | **Removed** in Ruby 3.0 (deprecated 2.7). Nothing replaces it. |
| **Python** | none | `rexec`/`Bastion` were withdrawn in 2003 as unfixable. PEP 578 audit hooks are *observability*, not enforcement, and native extensions bypass them. RestrictedPython is a subset compiler its own authors decline to call a security boundary. |
| **Go, Rust** | none | Compiled; no runtime to enforce anything. |

So for the two languages most likely to be asked for after JavaScript — Python
and Ruby — **there is no inner layer to add, and there will not be one.** Their
agents run with Layer 7 alone, which is precisely why Layer 7 had to be the
boundary and not this.

### S-I8a — wasm is the intended destination; the table above is transitional

The per-runtime patchwork is a transitional state, not the target. The target is
that **agents are wasm modules and nothing else.** This repo is already building
toward it: W01 specifies a full WASM 1.0 runtime (~182 instructions, pluggable
host interface), W02 specifies the complete `wasi_snapshot_preview1` ABI, and
`wasm-runtime` implementations exist in Go, Python, Dart, Perl, and TypeScript.
The runtimes currently ship minimal WASI stubs, so the destination is real but
not yet reachable.

Recording it here because it changes what the layers are *for*, and an
implementer should know which way this is heading:

**Today** — Layer 7 contains the agent, because the agent is a native process
that can attempt syscalls. S-I8's inner layer is a bonus available only to
JavaScript.

**In the wasm end state** — the agent has **no syscalls at all**. A wasm module
can only call the host functions it was given, so capability enforcement moves
into the host interface and becomes total by construction rather than by
enumeration. Layer 7 does not go away: it contains the **engine**, which is
itself attack surface (wasm engines have had escape CVEs). Its role changes from
*the* boundary to the containment around a boundary that now holds by
construction.

Two consequences worth fixing now, while the choice is still cheap:

1. **An agent module must not be given `wasi_snapshot_preview1`'s file and
   socket surface.** WASI is a *syscall* interface; handing an agent
   `path_open` and `sock_connect` rebuilds the entire problem inside the
   sandbox, with a less mature enforcement layer. What agents get is the
   **broker's capability interface as the host imports** — S-K5's rules apply
   unchanged, because the broker is still the thing on the other side. WASI's
   value here is its module-isolation model, not its syscall surface.
2. **The invariants in this spec are runtime-independent, and were written to
   survive this transition.** S-B1, S-I1a, S-I2, S-I6, S-K6, S-K2 and S-P3 hold
   whether the agent is a native process or a module; only the mechanism moves.
   That is the payoff of separating them (see Normative Status).

The cost of the end state is a compilation constraint on agent authors, which
cuts against "any language you like" — though less each year, and it buys Python
and Ruby the isolation they can never get natively.

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

The broker acts on agent-supplied arguments. It is
the confused deputy in this design and is reviewed as one.

- **Path arguments** are resolved beneath a pre-opened root descriptor, by the
  platform's own primitive. A validated path is never re-resolved from a string
  afterwards. `openat2` is Linux-only and this spec implements BSD first, so the
  rule is stated per platform rather than left to the implementer — the default
  reach is `realpath()` + `open()`, which is a TOCTOU on a authority-bearing
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
process holding authority the agent does not is a complete Layer 7 defeat reachable through a
declared, reviewed, hardware-key-approved capability. There is no supported path for it in this spec.

That is stated flatly because the obvious escape hatch is unsatisfiable by
construction: a helper that performs the call must hold `dlopen` plus
read+execute on the target library — authority the requesting agent cannot hold,
since `ffi:*` is never direct. A rule permitting such a helper would contradict
this rule's own title. Enabling `ffi:*` requires a named amendment to this
document, not an implementer's judgement.

**Brokered `proc:exec` spawns only through the same `spawn_verified` path used
for agents**: supervisor-owned wrapper, and targets restricted to a
manifest-declared digest-pinned set. Never `PATH`-resolved.

The child receives **its own sandbox plan, computed as a subset of the
requesting agent's**, and inherits none of the broker's descriptors (S-I3) and
none of its plan. Stating this as "runs under the agent's principal, not the
broker's" would be vacuous wherever the two share a UID, which S-I5 now permits.

**The broker never execs.** A brokered exec is a *request to the supervisor*,
which performs `spawn_verified`; the broker's own plan therefore contains no
exec authority at all. Without this the broker needs exec to satisfy S-K6, and
S-K7's containment becomes unsatisfiable.

**"A plan that is a subset of the requester's" is a computed predicate, not a
adjective.** Plans are per-platform and heterogeneous, so the comparison is
defined per platform — seccomp allowlist subset; Landlock rights subset over
identical path sets; `pledge` promise-set subset; AppContainer capability-SID
subset — evaluated at helper launch. If it cannot be decided, the launch fails
under S-P3. Left undefined, an implementer compares declared capability *lists*
rather than lowered plans, or reuses "the same plan plus what the helper needs",
and either reintroduces the inversion this rule closes.

### S-K7 — the broker is itself contained

S-I3's descriptor discipline applies to **every** supervisor-launched process,
the broker included: it does not inherit the supervisor's descriptor table —
vault handles, audit-log descriptors, other agents' channels.

The broker runs under its own sandbox plan, holding **its own minimum — the
channel keys, its audit descriptor, its pre-opened roots — plus the union of its
agents' declared capabilities, and nothing else**, enumerated in the plan as a
first-class term. Stating the bound as "no more than its agents declare" alone
is unsatisfiable, since no agent declares the channel keys; an unsatisfiable
bound gets ignored.

Whether there is one broker per agent or one per supervisor is therefore a
**blast-radius** decision before it is a capacity one: it determines whether an
S-K5 parsing defect compromises one agent or all of them. **The broker is one process per agent.** This is normative, because S-I5 cites
it as the pillar that demotes principal separation to defence in depth, and a
pillar may not rest on something this spec declines to decide.

A per-supervisor broker multiplexing many agents through one process is
inadmissible: it would hold every agent's channel keys, so an S-K5 parsing
defect becomes a cross-agent compromise. Compartmenting it "per agent" inside
one address space — separate structs, or a thread each — satisfies the words and
is not a boundary; that is the S-B1 error this spec exists to prevent.

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

**A direct `net:*` grant reopens the abstract unix namespace below Landlock ABI
v6 (6.12).** Landlock v4 network rules mediate TCP bind/connect only. Once
`socket`/`bind`/`connect` are allowlisted for a direct grant, two agents on a
6.7-6.11 kernel can bind and connect abstract-namespace unix sockets to each
other with no mediation at all — exchanging bytes, `SCM_CREDENTIALS`, and
`SCM_RIGHTS` descriptors, which is an S-I3-class escape. Abstract sockets carry
no filesystem path and no DAC check, so **distinct UIDs would not stop this
either**; only a network namespace or Landlock v6 does, and namespaces are
excluded from the required path. Below ABI v6, a direct network grant therefore
requires AF_UNIX to be separately denied by argument-filtered seccomp, or the
capability stays brokered.

**Unprivileged user namespaces are restricted by default** on Ubuntu 23.10+ via
AppArmor and historically on Debian. Nothing in the required path depends on
namespaces.

**The "Privilege: none" column is now true of the whole required path**,
including principal handling — see S-I5. An earlier draft required a privileged
supervisor for per-agent UID allocation and contradicted this column; that
requirement is withdrawn.

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
  expected errno. Any success aborts the launch. **Each probe class is chosen so
  that its outcome *without* the sandbox is success** — verified in the Layer 6
  harness — and the filter-mapped errno is distinct from the ambient one, so the
  probe distinguishes enforcement from an ambient DAC denial. A probe that would
  return `EPERM` anyway under the agent's unprivileged UID passes identically
  whether or not the filter installed, which is the exact failure S-P4 exists to
  catch.
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
(Broker topology was listed here as undecided. It is decided: **one broker per
agent**, normatively, in S-K7 — S-I5 rests on it.)
(`DENO_FLAGS` was listed here as undecided. It is decided: **retained**, as the
Deno case of S-I8. Deleting a working in-process layer once the OS layer lands
is a net reduction, not a cleanup.)

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
   than the enforcement mechanism — its flags are retained beneath the OS plan
   as the S-I8 inner layer, not deleted.

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
