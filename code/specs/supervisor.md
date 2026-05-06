# Supervisor

## Overview

A supervisor is an actor whose only job is to start, monitor, and restart other
actors. Supervisors do not do work; they delegate work and respond to failures.
When something goes wrong inside a supervised actor, the supervisor decides
whether to restart it, restart its siblings, or give up and propagate the failure
to its own parent.

Supervisors compose recursively: a supervisor can supervise other supervisors.
This produces a tree — the **supervision tree** — that mirrors the structure of a
real organization. The root is the most senior actor (in our analogy, the Chief
of Staff). Departments are sub-supervisors. Workers are leaves. A failure inside
a worker is handled by its immediate supervisor without bothering anyone above.
A supervisor that cannot recover its children gives up and lets *its* supervisor
deal with the larger failure. The pattern is called **let it crash**: rather than
trying to handle every conceivable error inline, code declares its assumptions
clearly, fails loudly when those assumptions are violated, and trusts the
supervision tree to recover the system to a known-good state.

This package is a Rust implementation of the supervision model that Erlang/OTP
has refined over thirty years of telecom production. OTP's `:supervisor` and
`:DynamicSupervisor` behaviors are the direct inspiration for the strategies,
restart policies, and escalation semantics defined here. The novel parts are
**capability inheritance** (a child's manifest is structurally a subset of its
parent's, enforced at registration), **two-level supervision** (the OS process
tree and the in-process actor tree are separate but use the same model), and
**vault/channel scoping** (a supervisor owns a namespace; its children can only
see resources inside it).

**Analogy.** Imagine a congressional office. The Chief of Staff does not manage
every intern directly. The Press Secretary manages a press team. The
Communications Director supervises the Press Secretary, the Email Reader, and
the Email Responder. The Chief of Staff supervises the Communications Director
along with the Legislative Director, the Counsel, and the Scheduler. When an
intern faints, the Press Secretary decides whether to send them home or call
another intern. The Chief of Staff never hears about it. But if the Press
Secretary keeps losing interns and cannot stabilize the team, the
Communications Director takes over — perhaps reassigning the entire team. If
the Communications Director cannot restore order, the Chief of Staff is
notified, and now the failure is escalated to the level that can actually fix
it.

This is the property that makes supervision trees so powerful: failures are
contained at the smallest scope that can recover, and only escalate when that
scope cannot.

---

## Where It Fits

```
Orchestrator (uses Supervisor at the top of the tree)
│
├── builds on ──► Supervisor ← YOU ARE HERE
│                   │   start_child / terminate_child / restart_child
│                   │   strategy enforcement on child failure
│                   │   max_restarts / max_seconds escalation
│                   │   capability inheritance check at registration
│                   │   vault / channel scope inheritance
│                   ▼
│                 Actor (D19)
│                   │   mailbox, behavior, state
│                   │   Message, Channel, ActorSystem
│                   ▼
│                 Process Manager (D14)
│                   │   fork/exec/wait for OS-process supervision
│                   ▼
│                 IPC (D16)
│                     channels for cross-process supervisor messages
│
└── used by ────► Host Runtime
                    │   each host runs its own in-process supervisor
                    │   for its agent's internal actors
                    ▼
                  Capability Cage
                      manifest the supervisor compares against
                      when validating child specs
```

**Depends on:** Actor (D19) for messages, channels, and the in-process actor
runtime. Process Manager (D14) for OS-process supervision in the upper tree.

**Used by:** Orchestrator (top of the tree, supervises hosts as OS processes),
Host Runtime (each host runs its own in-process actor supervisor), every future
agent that needs internal supervision.

---

## Key Concepts

### The Two-Level Model

Erlang's supervision works inside a single BEAM virtual machine because BEAM
gives you preemptive scheduling, isolated per-process heaps, and garbage
collection that can pause one process without touching another. A panic in one
Erlang process does not corrupt another's state. A runaway loop is preempted by
the scheduler. Linked exits propagate cleanly as messages, not as memory
corruption.

Rust does not give us those properties inside a single OS process. A `panic!()`
in a `tokio::spawn`-ed task can be caught with `catch_unwind`, but a logic bug
that corrupts shared memory cannot. A tight CPU loop in one task can starve
another because async tasks are cooperative. There is no per-task heap to
isolate.

The fix: **the strong isolation boundary is the OS process**, and the supervision
tree is fractal across that boundary. Two trees, glued together at every host:

```
  PROCESS TREE (OS-level, kernel-enforced isolation)
  ════════════════════════════════════════════════════════════════
  orchestrator (root)
  ├── host-comms          ← OS process. If this dies, OS reaps its
  │     │                   children and the orchestrator restarts it.
  │     │
  │     │   ACTOR TREE (in-process, cooperative, OTP-style)
  │     │   ════════════════════════════════════════════════════
  │     ├── press-secretary actor
  │     │     ├── speech-writer actor
  │     │     └── intern-pool dyn-supervisor
  │     │           ├── intern-1 actor
  │     │           └── intern-2 actor
  │     ├── email-reader actor
  │     └── email-responder actor
  │
  ├── host-legis          ← OS process. Independent ACTOR TREE inside.
  │     │
  │     │   ACTOR TREE
  │     │   ═══════════════════════════════════════════
  │     ├── counsel actor
  │     ├── bill-tracker actor
  │     └── vote-whip actor
  │
  └── vault               ← OS process, peer of departments.
```

Both trees use **the same supervisor primitives** defined in this spec. They
differ only in the failure mechanism:

| Level                | Failure unit         | Detection                    | Recovery                    |
|----------------------|----------------------|------------------------------|------------------------------|
| OS process tree      | a child OS process   | `wait()` / `WaitForSingleObject` returns | spawn a new OS process       |
| In-process actor tree | a child actor        | actor returns `Stop`, panics caught, or mailbox-drain timeout | re-instantiate the actor     |

Everything else — child specs, strategies, restart policies, max restart
intensity, capability inheritance — is identical across the two levels. A
supervisor implementation does not know whether its children are OS processes
or in-process actors; it works against a `Child` trait that abstracts the
lifecycle.

This is the property that lets us write the supervisor logic once and reuse it
at both levels.

---

### Child Specifications

Every child of a supervisor is registered through a `ChildSpec`. The spec
describes how to start the child, how to restart it on failure, how to shut it
down gracefully, and what its security boundary is.

```
ChildSpec
═══════════════════════════════════════════════════════════════
┌──────────────────┬─────────────────────────────────────────┐
│ id               │ Unique identifier within the parent.     │
│                  │ Used for restart, terminate, and lookup. │
├──────────────────┼─────────────────────────────────────────┤
│ kind             │ Worker | Supervisor | HostProcess        │
│                  │ Worker = leaf actor (does work)          │
│                  │ Supervisor = sub-supervisor (delegates)  │
│                  │ HostProcess = OS process child           │
├──────────────────┼─────────────────────────────────────────┤
│ start            │ A function that produces a running Child │
│                  │ when invoked. Called on initial start    │
│                  │ and on every restart.                    │
├──────────────────┼─────────────────────────────────────────┤
│ restart          │ Permanent | Transient | Temporary        │
│                  │ When this child should be restarted.     │
├──────────────────┼─────────────────────────────────────────┤
│ shutdown         │ Brutal | Graceful(Duration) | Infinity   │
│                  │ How to stop this child during shutdown.  │
├──────────────────┼─────────────────────────────────────────┤
│ manifest         │ Capability manifest. MUST be a subset of │
│                  │ the parent's manifest.                   │
├──────────────────┼─────────────────────────────────────────┤
│ vault_namespace  │ Vault paths visible to this child. MUST  │
│                  │ be a subset of parent's namespace.       │
├──────────────────┼─────────────────────────────────────────┤
│ channel_scope    │ Channels visible to this child. MUST be  │
│                  │ inherited or local; never crosses peers. │
└──────────────────┴─────────────────────────────────────────┘
```

The capability, vault, and channel fields are the security wins from
hierarchical supervision: the tree shape becomes the privilege boundary.
Compromise of a leaf cannot pivot to its sibling subtrees because the
manifest at registration time was provably narrower.

---

### Restart Policies

A child's restart policy answers the question: **when should I be restarted?**

```
Permanent   The child is always restarted, regardless of why it stopped.
            Use for components that should always be running (vault, hosts
            that own external resources).

Transient   The child is restarted only on abnormal termination (panic,
            non-zero exit code, unhandled error). A clean exit is honored.
            Use for tasks that have a defined completion (a one-shot
            speech-writer, a backfill job).

Temporary   The child is never restarted. If it dies for any reason, it
            stays dead. Use for ephemeral workers (an intern handling one
            inquiry, a request-scoped task).
```

The decision is per-child, not per-supervisor. A supervisor may have a mix.
For example, the Press Secretary supervisor has the Press Secretary itself as
`permanent`, the speech-writer as `transient`, and individual intern
spawnings as `temporary`.

```
                                Stopped normally?       Stopped abnormally?
Restart policy                  ─────────────────       ───────────────────
Permanent                       restart                  restart
Transient                       leave dead               restart
Temporary                       leave dead               leave dead
```

---

### Restart Strategies

A supervisor's restart strategy answers the question: **when one of my children
fails, which children do I restart?** OTP defines four; we adopt all four.

```
┌────────────────────────────────────────────────────────────────┐
│ one_for_one                                                     │
│   When child C fails, restart only C.                           │
│   Use when children are independent (no shared invariants).     │
│   Default. Safest. Smallest restart blast radius.               │
└────────────────────────────────────────────────────────────────┘

  Before crash:   [A] [B] [C] [D]    │ │ │ │
  C crashes:      [A] [B]  X  [D]    │ │   │
  Recovery:       [A] [B] [C] [D]    │ │ │ │   ← only C re-spawned

┌────────────────────────────────────────────────────────────────┐
│ one_for_all                                                     │
│   When any child fails, restart every child.                    │
│   Use when children share state or coordinate tightly: if one   │
│   dies, the others' state is suspect and must be reset.         │
│   Larger blast radius. Slower recovery. Strongest consistency.  │
└────────────────────────────────────────────────────────────────┘

  Before crash:   [A] [B] [C] [D]
  C crashes:       X   X   X   X     ← supervisor stops survivors
  Recovery:       [A'] [B'] [C'] [D'] ← all four re-spawned fresh

┌────────────────────────────────────────────────────────────────┐
│ rest_for_one                                                    │
│   When child C fails, restart C and every child started AFTER   │
│   C, in start order. Children started before C are unaffected.  │
│   Use when children form an ordered dependency chain: D depends │
│   on C's state, but A and B do not.                             │
└────────────────────────────────────────────────────────────────┘

  Start order:    [A] [B] [C] [D]
  C crashes:      [A] [B]  X   X     ← D stopped because it depends on C
  Recovery:       [A] [B] [C'] [D']  ← C and D re-spawned

┌────────────────────────────────────────────────────────────────┐
│ dynamic (a.k.a. simple_one_for_one in OTP)                      │
│   All children share one ChildSpec template. Children are       │
│   added at runtime via start_child(template_args), not          │
│   declared at supervisor creation.                              │
│   Use for pools of short-lived workers (per-request handlers,   │
│   intern pool, per-connection actors).                          │
└────────────────────────────────────────────────────────────────┘

  start:          [pool]  ← supervisor starts empty
  start_child:    [worker-1]
  start_child:    [worker-2]   ← runtime children, all from one template
  worker-1 dies:  [worker-2]   ← typically temporary; not restarted
                  start a new one if the work demands it
```

When choosing a strategy, the principle is: **what invariants does my children
collectively maintain? If a failure could violate them, what is the smallest
group I must restart together to restore them?** That group size is the
strategy.

---

### Max Restart Intensity

A supervisor that restarts a child every time it crashes can mask a persistent
bug: the system stays "up" forever, but never makes progress. To prevent this,
every supervisor declares a **restart intensity limit**:

```
SupervisorSpec
  max_restarts:  N      // maximum number of child restarts
  max_seconds:   S      // within this rolling time window
```

If the supervisor would perform more than `N` restarts within any window of
`S` seconds, it does not restart. Instead, it **gives up, terminates all
remaining children, and stops itself** — propagating the failure to its
own supervisor.

```
Press Secretary (max_restarts=5, max_seconds=10)
│
├── Speech Writer  (transient)
│
└── Intern Pool

  t=0s   intern-1 crashes  → restart 1/5
  t=1s   intern-1 crashes  → restart 2/5
  t=2s   intern-1 crashes  → restart 3/5
  t=3s   intern-1 crashes  → restart 4/5
  t=4s   intern-1 crashes  → restart 5/5
  t=5s   intern-1 crashes  → LIMIT EXCEEDED
                            → Press Secretary terminates Speech Writer
                            → Press Secretary stops itself
                            → propagates failure to Communications Director

Communications Director (max_restarts=3, max_seconds=60)
  Restarts Press Secretary (which restarts Speech Writer + fresh Intern Pool)
  If Press Secretary keeps failing within 60s, Communications Director
  also escalates upward.

Eventually:
  Orchestrator at the root has nowhere to escalate to.
  Logs the failure, pages a human, possibly enters degraded mode.
```

This is the genius of OTP's model: failures **escalate to the level that can
actually fix them**. A bad input crashes one intern, gets contained. A bad
deployment crashes every intern, propagates one level up — maybe the whole
team needs to be reset. A systemic bug crashes every team, propagates to
the orchestrator — now a human is paged.

The default values for new supervisors are **`max_restarts = 3`,
`max_seconds = 5`**, mirroring OTP. These are conservative; tune per-supervisor
based on the expected failure frequency of the work.

---

### Capability Inheritance

This is the security extension over plain OTP supervision. **Every child's
manifest must be a subset of its parent supervisor's manifest, verified at
the moment the child is registered.** A subtree's privilege envelope is the
intersection of every manifest from the root down to it.

```
Orchestrator               manifest: [supervise, vault:admin, fs:read:./pkgs/]
│
└── Communications Dept    manifest: [supervise, vault:read:gmail-*]
    │
    └── Press Secretary    manifest: [vault:read:gmail-press-*,
        │                              net:connect:gmail.com:443]
        │
        └── Speech Writer  manifest: [net:connect:openai.com:443]   ❌ REJECTED
                           reason: parent has no net:connect:openai.com:443
```

The Speech Writer's spec is rejected at registration. Not at runtime — at the
*structural* moment of being added to the tree. The supervisor refuses to
accept a child whose manifest contains capabilities the supervisor itself does
not hold.

Why this is powerful:

1. **Tree position equals privilege ceiling.** You can read any subtree's
   maximum possible capabilities by walking from the root to the leaf and
   intersecting manifests. No runtime analysis needed.

2. **Lateral movement is structurally impossible.** A compromised Speech
   Writer cannot acquire capabilities that Press Secretary did not have. A
   compromised Press Secretary cannot pivot to capabilities Communications
   Department did not have.

3. **Re-organizing the tree re-shapes the security model.** Moving an actor
   to a different subtree changes its privilege ceiling automatically. No
   policy file to keep in sync.

4. **Diff-friendly.** A PR that adds a child with broader capabilities than
   its parent fails the registration check at supervisor build time, not at
   deploy time. Security regressions caught in code review.

The capability check is performed by the supervisor when `start_child` is
called, **before** the child's `start` function runs. A child whose manifest
violates the subset rule is rejected with `ChildSpecError::CapabilityEscalation`.

---

### Vault and Channel Scoping

Two more inheritance fields, with the same subset rule:

**Vault namespace.** Each supervisor owns a vault namespace prefix. Its
children can read secrets only at or below that prefix. The Communications
Department might own `vault://comms/*`; the Press Secretary inherits a
default of `vault://comms/press/*` and may narrow it further when registering
its own children.

**Channel scope.** Channels declared at a supervisor's level are visible to
that supervisor's descendants but not to its siblings. This produces the
"nonexistent, not forbidden" property at the tree level: an actor in the
Legislative Department literally cannot name a channel declared inside the
Communications Department, because that channel does not exist in its scope.

Both fields are enforced at registration, the same way capabilities are.

---

### Shutdown Semantics

When a supervisor is asked to stop (either because its parent is shutting it
down, or because it is escalating after exceeding restart intensity), it
shuts down its children **in the reverse of the order they were started**.
Each child has a `shutdown` policy:

```
Brutal              Send terminate signal immediately. No grace period.
                    Use only for actors that cannot block on cleanup.

Graceful(Duration)  Send graceful-stop signal. Wait up to Duration for the
                    actor to drain its mailbox and call its on_stop hook.
                    If the timeout expires, escalate to brutal termination.

Infinity            Send graceful-stop signal. Wait as long as needed.
                    Use only for supervisors of supervisors, where children
                    have their own shutdown timeouts that bound the wait.
```

Reverse-start-order shutdown matches the dependency invariants implied by
`rest_for_one`: if D depends on C, D must shut down before C does.

The orchestrator at the root uses `Graceful(30s)` by default; in-process
actors use `Graceful(5s)` by default. These are tunable per-spec.

---

## Algorithms

### Starting a Supervisor

```
Supervisor.start(spec: SupervisorSpec) -> Result<SupervisorRef>

  1. Validate the spec:
     a. Every ChildSpec id is unique within children.
     b. For each child:
        - manifest    is subset of supervisor's manifest
        - vault_ns    is subset of supervisor's vault_ns
        - channels    are visible at this supervisor's scope
     c. max_restarts >= 0, max_seconds >= 1.

  2. Create the supervisor actor with:
     - empty restart history
     - empty children registry
     - the spec's strategy and limits

  3. For each ChildSpec in declaration order:
     start_child(spec)
     If any start_child fails, terminate already-started children
     in reverse order and return the failure.

  4. Return SupervisorRef.
```

### Starting a Child

```
Supervisor.start_child(spec: ChildSpec) -> Result<ChildRef>

  1. Verify spec.manifest ⊆ supervisor.manifest
     Verify spec.vault_namespace ⊆ supervisor.vault_namespace
     Verify spec.channel_scope is permitted at supervisor's level
     If any check fails, return ChildSpecError::CapabilityEscalation.

  2. Verify spec.id is unique among current children.
     If duplicate, return ChildSpecError::DuplicateId.

  3. Call spec.start() to produce a Child.
     If start() fails:
       - record the failure for restart-intensity tracking
       - return the underlying error

  4. Wire failure detection:
     - if Worker:    spawn a watcher that monitors actor.status
     - if Supervisor: link bidirectionally so its escalation reaches us
     - if HostProcess: register the PID for OS-level wait()

  5. Insert the child in the registry, remembering start order.

  6. Return ChildRef.
```

### Handling a Child Failure

```
Supervisor.on_child_exit(id: ChildId, reason: ExitReason)

  1. Look up the child's restart policy and the supervisor's strategy.

  2. Determine "should we restart?":
     match (restart_policy, reason):
       (Permanent,  _)              -> restart
       (Transient,  Normal)         -> do not restart, remove from registry
       (Transient,  Abnormal)       -> restart
       (Temporary,  _)              -> do not restart, remove from registry

  3. If not restarting, return.

  4. Record the restart in the rolling window.
     If count within max_seconds > max_restarts:
       a. Stop all remaining children in reverse start order.
       b. Stop self.
       c. Propagate Escalated to parent supervisor.
       Return.

  5. Apply the strategy to determine which children to restart:
     match strategy:
       OneForOne   -> restart only this child
       OneForAll   -> stop all other children, restart all of them
       RestForOne  -> stop children started after this one,
                      restart this one and all after it
       Dynamic     -> typically Temporary; do nothing if not restarting

  6. For each child to restart:
     stop child (graceful with timeout per spec.shutdown)
     call spec.start() to produce a fresh instance
     update registry
     If start fails for any child, treat as a new failure and recurse.
```

### Terminating a Supervisor

```
Supervisor.terminate()

  1. Mark self as shutting down. Reject new start_child calls.

  2. For each child in REVERSE start order:
     match child.shutdown:
       Brutal                 -> send Terminate signal, do not wait
       Graceful(d)            -> send Stop signal, wait up to d,
                                 then send Terminate if still alive
       Infinity               -> send Stop signal, wait indefinitely

     Remove child from registry.

  3. Stop self.
```

### Restart-Intensity Tracking

```
A supervisor maintains a ring buffer of recent restart timestamps.

on_restart():
  now = monotonic_now()
  recent = restart_history.filter(|t| now - t < max_seconds)
  if recent.length >= max_restarts:
    return ESCALATE
  restart_history.push(now)
  return OK

The ring buffer is bounded; old entries are pruned implicitly by the filter
on every check.
```

---

## Public API

```rust
// ─────────────────────────────────────────────────────────────────
// Strategies
// ─────────────────────────────────────────────────────────────────

pub enum Strategy {
    OneForOne,
    OneForAll,
    RestForOne,
    Dynamic { template: Box<ChildSpec> },
}

// ─────────────────────────────────────────────────────────────────
// Restart policy (per child)
// ─────────────────────────────────────────────────────────────────

pub enum Restart {
    Permanent,
    Transient,
    Temporary,
}

// ─────────────────────────────────────────────────────────────────
// Shutdown policy (per child)
// ─────────────────────────────────────────────────────────────────

pub enum Shutdown {
    Brutal,
    Graceful(std::time::Duration),
    Infinity,
}

// ─────────────────────────────────────────────────────────────────
// Child kind
// ─────────────────────────────────────────────────────────────────

pub enum ChildKind {
    Worker,        // a leaf actor
    Supervisor,    // a sub-supervisor
    HostProcess,   // an OS-process child
}

// ─────────────────────────────────────────────────────────────────
// Why a child stopped
// ─────────────────────────────────────────────────────────────────

pub enum ExitReason {
    Normal,        // returned Stop, exit code 0, etc.
    Abnormal {     // panic, non-zero exit, killed by OS, etc.
        message: String,
    },
    Escalated {    // a sub-supervisor exceeded its restart intensity
        from: ChildId,
    },
    Shutdown,      // parent asked us to stop
}

// ─────────────────────────────────────────────────────────────────
// Child specification
// ─────────────────────────────────────────────────────────────────

pub struct ChildSpec {
    pub id:               ChildId,
    pub kind:             ChildKind,
    pub start:            Box<dyn Fn() -> Result<Child, StartError> + Send + Sync>,
    pub restart:          Restart,
    pub shutdown:         Shutdown,
    pub manifest:         CapabilityManifest,
    pub vault_namespace:  VaultNamespace,
    pub channel_scope:    ChannelScope,
}

// ─────────────────────────────────────────────────────────────────
// Supervisor specification
// ─────────────────────────────────────────────────────────────────

pub struct SupervisorSpec {
    pub strategy:         Strategy,
    pub max_restarts:     u32,
    pub max_seconds:      u32,
    pub children:         Vec<ChildSpec>,
    pub manifest:         CapabilityManifest,
    pub vault_namespace:  VaultNamespace,
    pub channel_scope:    ChannelScope,
}

impl Default for SupervisorSpec {
    fn default() -> Self {
        Self {
            strategy:        Strategy::OneForOne,
            max_restarts:    3,
            max_seconds:     5,
            children:        vec![],
            manifest:        CapabilityManifest::empty(),
            vault_namespace: VaultNamespace::root(),
            channel_scope:   ChannelScope::local(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Supervisor handle
// ─────────────────────────────────────────────────────────────────

pub trait Supervisor: Send + Sync {
    fn start_child(&mut self, spec: ChildSpec) -> Result<ChildRef, ChildSpecError>;
    fn terminate_child(&mut self, id: &ChildId) -> Result<(), SupError>;
    fn restart_child(&mut self, id: &ChildId) -> Result<ChildRef, SupError>;
    fn which_children(&self) -> Vec<ChildInfo>;
    fn count_children(&self) -> ChildCounts;
    fn terminate(self) -> Result<(), SupError>;
}

pub struct ChildInfo {
    pub id:        ChildId,
    pub kind:      ChildKind,
    pub status:    ChildStatus,
    pub restarts:  u32,    // total restarts in this supervisor's lifetime
}

pub enum ChildStatus {
    Starting,
    Running,
    Restarting,
    Stopping,
    Stopped,
}

pub struct ChildCounts {
    pub specs:        u32,    // declared in supervisor
    pub active:       u32,    // currently running
    pub supervisors:  u32,    // children that are themselves supervisors
    pub workers:      u32,
}

// ─────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────

pub enum ChildSpecError {
    CapabilityEscalation { child: ChildId, missing: Vec<Capability> },
    VaultNamespaceEscalation { child: ChildId, requested: VaultNamespace },
    ChannelScopeViolation { child: ChildId, channel: ChannelId },
    DuplicateId(ChildId),
    StartFailed { child: ChildId, source: StartError },
}

pub enum SupError {
    NotFound(ChildId),
    NotRestartable(ChildId),
    AlreadyStopping,
    Escalated,
}
```

---

## Examples

### Example 1: A Department of Three Workers

```rust
let department = SupervisorSpec {
    strategy:     Strategy::OneForOne,
    max_restarts: 3,
    max_seconds:  60,
    manifest:     manifest!["vault:read:comms-*", "net:connect:*:443"],
    children: vec![
        ChildSpec {
            id:      ChildId::new("press-secretary"),
            kind:    ChildKind::Worker,
            start:   Box::new(|| start_press_secretary()),
            restart: Restart::Permanent,
            shutdown: Shutdown::Graceful(Duration::from_secs(5)),
            manifest: manifest!["vault:read:comms-press-*",
                                "net:connect:gmail.com:443"],
            ..Default::default()
        },
        ChildSpec {
            id:      ChildId::new("email-reader"),
            kind:    ChildKind::Worker,
            start:   Box::new(|| start_email_reader()),
            restart: Restart::Permanent,
            shutdown: Shutdown::Graceful(Duration::from_secs(5)),
            manifest: manifest!["net:connect:imap.gmail.com:993"],
            ..Default::default()
        },
        ChildSpec {
            id:      ChildId::new("email-responder"),
            kind:    ChildKind::Worker,
            start:   Box::new(|| start_email_responder()),
            restart: Restart::Permanent,
            shutdown: Shutdown::Graceful(Duration::from_secs(5)),
            manifest: manifest!["vault:read:smtp-creds",
                                "net:connect:smtp.gmail.com:465"],
            ..Default::default()
        },
    ],
    ..Default::default()
};

let comms = Supervisor::start(department)?;
```

If `email-reader` panics, only it is restarted. The Press Secretary and the
Email Responder are unaffected.

### Example 2: Tightly-Coupled Children with one_for_all

```rust
let bridge_supervisor = SupervisorSpec {
    strategy:     Strategy::OneForAll,
    max_restarts: 5,
    max_seconds:  30,
    children: vec![
        ChildSpec { id: ChildId::new("zigbee-coordinator"), .. },
        ChildSpec { id: ChildId::new("zigbee-router"), .. },
        ChildSpec { id: ChildId::new("zigbee-device-table"), .. },
    ],
    ..Default::default()
};
```

If the Zigbee coordinator dies (the serial-port owner), the router and the
device table reference state inside it that is now stale. `OneForAll` restarts
all three so the rest of the bridge restarts cleanly.

### Example 3: Dynamic Pool of Workers

```rust
let intern_pool = SupervisorSpec {
    strategy: Strategy::Dynamic {
        template: Box::new(ChildSpec {
            id:       ChildId::placeholder(),  // assigned at start_child
            kind:     ChildKind::Worker,
            start:    Box::new(|| start_intern()),
            restart:  Restart::Temporary,
            shutdown: Shutdown::Graceful(Duration::from_secs(2)),
            manifest: manifest!["net:connect:openai.com:443"],
            ..Default::default()
        }),
    },
    max_restarts: 100,
    max_seconds:  60,
    children:     vec![],     // no static children for dynamic
    ..Default::default()
};

let pool = Supervisor::start(intern_pool)?;

// At runtime, spawn interns as needed:
let intern = pool.start_child_dynamic(intern_args)?;
// When intern is done, it returns Stop. Because Restart::Temporary,
// it is not restarted; just removed from the pool.
```

### Example 4: A Top-Level Process Tree

```rust
let orchestrator = SupervisorSpec {
    strategy:     Strategy::OneForOne,
    max_restarts: 3,
    max_seconds:  60,
    manifest:     manifest!["supervise", "vault:admin", "proc:fork", "proc:exec:*"],
    children: vec![
        ChildSpec {
            id:    ChildId::new("vault"),
            kind:  ChildKind::HostProcess,
            start: Box::new(|| spawn_host_process("vault.exe")),
            restart: Restart::Permanent,
            shutdown: Shutdown::Graceful(Duration::from_secs(30)),
            manifest: load_manifest("vault.agent/required_capabilities.json"),
            ..Default::default()
        },
        ChildSpec {
            id:    ChildId::new("comms-dept"),
            kind:  ChildKind::HostProcess,
            start: Box::new(|| spawn_host_process("host-comms.exe")),
            restart: Restart::Permanent,
            shutdown: Shutdown::Graceful(Duration::from_secs(30)),
            manifest: load_manifest("comms.agent/required_capabilities.json"),
            ..Default::default()
        },
        // ... more departments ...
    ],
    ..Default::default()
};

let root = Supervisor::start(orchestrator)?;
```

The same `Supervisor` trait, the same `ChildSpec`, the same strategy logic —
but children are OS processes. Failure detection is `wait()` instead of
mailbox status; restart is `spawn_host_process` instead of in-process actor
creation.

---

## Test Strategy

### Unit Tests

1. **Strategy correctness**
   - `OneForOne`: only the failed child is restarted; siblings undisturbed.
   - `OneForAll`: all children restart, in declared order.
   - `RestForOne`: only children after the failed one (in start order)
     are restarted, in start order.
   - `Dynamic`: failed children are not auto-restarted unless `Restart`
     dictates; `start_child_dynamic` adds new children correctly.

2. **Restart-policy correctness**
   - `Permanent`: restarted on both Normal and Abnormal exits.
   - `Transient`: restarted on Abnormal only; left dead on Normal.
   - `Temporary`: never restarted.

3. **Restart intensity**
   - Restarts under the limit succeed.
   - Restarts at exactly `max_restarts` within `max_seconds` succeed.
   - The `(max_restarts + 1)`-th restart in window triggers escalation.
   - Old restart timestamps outside the window are pruned correctly.

4. **Capability inheritance**
   - Child with strictly-subset manifest is accepted.
   - Child with equal manifest is accepted.
   - Child with one extra capability is rejected with
     `CapabilityEscalation { missing: [that_capability] }`.
   - Vault namespace and channel scope follow the same rule.

5. **Shutdown ordering**
   - On `terminate`, children stop in reverse of their start order.
   - `Graceful(d)`: child has up to `d` to stop, then forced.
   - `Brutal`: child receives no grace period.
   - `Infinity`: supervisor waits without bound.

6. **Idempotency and validation**
   - Duplicate `ChildId` at registration is rejected.
   - `start_child` after `terminate` is rejected with `AlreadyStopping`.
   - `restart_child` on an unknown id returns `NotFound`.
   - `restart_child` on a `Temporary` child returns `NotRestartable`.

### Integration Tests

1. **Two-level fault containment.** Spawn a host process that hosts an
   in-process supervisor with a chain of actors. Crash the deepest actor;
   verify the in-process supervisor restarts it without the orchestrator
   noticing. Crash the host process itself; verify the orchestrator restarts
   the whole process tree.

2. **Escalation chain.** Build a 3-level tree (orchestrator → dept →
   worker-supervisor → workers). Configure intensity to escalate within
   seconds. Repeatedly crash a worker until the worker-supervisor escalates,
   and verify the dept supervisor restarts the worker-supervisor with a fresh
   restart history.

3. **Capability inheritance end-to-end.** Try to spawn a child with extra
   capabilities; verify rejection. Modify a parent's manifest; verify
   children that previously passed validation no longer can.

4. **Crash storms.** Inject a child that crashes immediately on start.
   Verify the supervisor escalates within the configured window and does
   not loop.

5. **Graceful shutdown of nested trees.** Send terminate to root; verify
   leaves stop first, then their parents, then their parents, all the way
   up — even with `Graceful` timeouts that vary per child.

### Coverage Target

`>=95%` line coverage. Supervisor logic is foundational; bugs here corrupt
every system built on top.

---

## Dependencies

- **actor (D19)** — for in-process child kinds (Worker, Supervisor as actor).
- **process-manager (D14)** — for HostProcess child kind (`fork`/`exec`/`wait`).
- **capability-cage** — for `CapabilityManifest` and the subset check.
- **vault** — for `VaultNamespace`.
- **time** — for monotonic timestamps in restart-intensity tracking.

---

## Trade-Offs

**Sequential start, sequential restart.** A supervisor with many children
starts them serially. Parallel start is a future enhancement; for V1 the
predictability of serial start (clear ordering for `RestForOne`,
deterministic test behavior) outweighs the latency cost.

**No automatic relationship discovery.** A supervisor only knows about
children it was explicitly given. There is no cross-tree dependency tracking
("Calendar depends on Vault unlock"). Such relationships, if needed, are
expressed by ordering children under the same supervisor with a strategy
that captures the dependency (`RestForOne` or `OneForAll`).

**Capability inheritance is checked at registration, not at runtime.** A
child cannot escalate its manifest mid-life. To "promote" a child to broader
capabilities, you must terminate it and re-register a new spec with the
broader manifest — and the parent's manifest must permit that breadth.

**Restart-intensity is per-supervisor, not per-child.** A supervisor's
intensity counter aggregates all of its children's restarts. A noisy child
can therefore burn the supervisor's budget and trigger escalation even
though its siblings are healthy. This is intentional: the supervisor's
job is to maintain the health of its subtree, and a single misbehaving
child *is* a subtree health problem.

**The two-level model duplicates supervision logic at process and actor
levels.** The trait abstracts this, but a developer must understand both
levels to design a system. The pay-off is genuine isolation that a single
runtime cannot provide in safe Rust.

---

## Future Extensions

- **Hot reload.** Replace a child's `start` function without terminating
  the supervisor, propagating the new spec to the next restart.
- **Distributed supervisors.** A supervisor whose children live on remote
  hosts, with the same strategy semantics over the network.
- **Observability hooks.** Channels published per supervisor that emit
  lifecycle events (`Started`, `Restarted`, `Escalated`, `Stopped`) for
  external monitoring.
- **Adaptive intensity.** Tune `max_restarts` and `max_seconds` based on
  observed crash patterns rather than declaring them statically.

These are deliberately out of scope for V1.
