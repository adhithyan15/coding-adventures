# D18R — Chief of Staff Supervision Tree and Leadership

## Overview

Chief of Staff is an OTP-shaped supervision tree with encryption and sandboxing
added. This spec fixes the vocabulary, states the tree's structure, and defines
the two properties the tree does not yet have: **surviving the death of its own
root**, and **placing agents on a supervisor that can carry them**.

It is deliberately ordered so that each part is useful on its own. Leader
election — the part that looks most interesting — is specified last and needed
least, for reasons the "Do you need consensus" section gives.

**Depends on:** D18 Chief of Staff, D18D Tool API, D18T Durable Epoch
Activation, VLT03 Vault Key Custody, VLT07 Vault Leases.

**Supersedes nothing.** It names structure that already exists and adds to it.

---

## Terminology

The word "host" is already taken. `chief-of-staff-host` is *"concrete
authenticated child host for D18 Chief agent packages"* — the sandboxed child
process that runs exactly one agent. Using "host" for the root of the tree as
well would give one word two meanings in a document about who supervises whom.

This spec therefore uses OTP's vocabulary, made specific:

| Term | OTP analogue | What it is |
|---|---|---|
| **daemon** | application | The OS-level service. Starts the root supervisor and nothing else. |
| **root supervisor** | top supervisor | First process spawned. Holds vault custody, spawns branch supervisors, receives every agent-launch request, verifies signatures, and places agents. |
| **branch supervisor** | intermediate supervisor | Supervises a set of agent hosts. Reports capacity. Eligible, if a voter, to become root. |
| **agent host** | worker | One sandboxed child process running one agent package. This is today's `chief-of-staff-host`. |
| **agent** | — | The signed package an agent host runs. |

Two more, from Raft, used only in the leadership sections:

| Term | What it is |
|---|---|
| **term** | A monotonically increasing integer identifying one leadership period. Every root supervisor acts under exactly one term. |
| **voter set** | The fixed subset of branch supervisors eligible to elect and to be elected. Not every branch supervisor is a voter — see "Membership is fixed". |

Where this document says *leader* it means the root supervisor of the current
term.

---

## The tree

```text
daemon                          OS service. Starts one child and restarts it.
  |
  +-- root supervisor           term N; vault custody; signature checks; placement
        |
        +-- branch supervisor A
        |     |
        |     +-- agent host    (weather)
        |     +-- agent host    (email)
        |
        +-- branch supervisor B
              |
              +-- agent host    (smart home)
              +-- agent host    (calendar)
```

Smart home appears in that tree as a leaf, next to weather and calendar, with
nothing above it that knows what it is. That is the whole point of the diagram.

### Everything domain-specific is an agent

The tree is generic. It knows about packages, signatures, capacity, and
restarts. It knows nothing about weather, email, or smart homes.

This is not aspirational: it is a correction. Today the supervisor tier
dispatches through one hardcoded `Arc<dyn HostDataPlaneDispatcher>`, and in the
shipping path that dispatcher is the smart-home tool bridge. Smart home is
therefore welded into the tree rather than running on it. It must become an
agent like any other, reached through the D18D tool surface, so that the
supervisor tier has no reason to know it exists.

The consequence worth stating plainly: **a generic tree means the tool surface
must be profile-backed.** A supervisor that dispatches one hardcoded surface
cannot host arbitrary agents, because there is no per-agent statement of what
that agent may call. See D18D section 7.1 V4, and `SupervisedOrchestratorRuntime`
in `chief-of-staff-host-runtime`, which already models this and has no consumers.

---

## Restart: the property OTP actually gives you

`chief-of-staff-process-supervisor` has no restart policy. Searching it for
`restart` or `respawn` finds nothing. An agent host that dies stays dead.

So the crash-survival this architecture is named for **is not implemented at any
level**, and leadership failover is a roof over an unbuilt wall. Restart comes
first.

### Restart rules

**R1 — every supervised process has a restart policy.** One of `permanent`
(always restart), `transient` (restart only on abnormal exit), or `temporary`
(never restart). Default is `transient`: a supervisor that restarts a process
which exited deliberately will restart it forever.

**R2 — restart intensity is bounded.** A supervisor that performs more than
`max_restarts` within `max_seconds` gives up and terminates itself, escalating
to its own supervisor. Without this a crash-looping agent consumes the machine
while looking healthy — the supervisor is, after all, doing its job.

**R3 — restarting an agent host re-verifies the package.** `spawn_verified`
already checks the signature and re-checks `package.digest()` against
`registration.package_hash()`. A restart is a spawn and takes the same path. A
restart that trusted the previous verification would let an attacker who can
replace a file on disk wait for a crash.

**R4 — the daemon restarts the root supervisor.** This is the single-machine
answer to root death, and it is strictly simpler than election: no votes, no
split brain, no fencing. It is also the OTP answer. What it does not survive is
the machine.

---

## Custody: the part that decides whether failover is possible

The root supervisor holds the vault connection. If a branch supervisor is
promoted, it needs the vault, and this is where automatic promotion either works
or does not.

VLT03 derives the vault master key from a passphrase or biometric via Argon2id
and **never stores it**. A promoted supervisor cannot re-derive it. So there are
exactly three options, and the choice is architectural rather than incidental:

1. **Promotion requires the user.** Honest, simple, and defeats the purpose —
   unattended failover is the reason to build any of this.
2. **The vault is its own process with its own lifecycle**, and "custody" means
   holding a connection rather than holding the key. Then promotion is
   reconnection. This moves the problem rather than solving it: the vault
   process is now the single point of failure.
3. **The key is shared in advance, k-of-n.** Each voter receives one share at
   startup. Promotion reconstructs the master key from a quorum of shares.

**This spec chooses (3).** `coding_adventures_shamir` already implements k-of-n
over GF(2⁸) with the property that fewer than k shares reveal nothing about the
secret — not "less information", *nothing*.

### Custody rules

**C1 — shares are distributed at startup, before any agent runs.** A share
handed out during an emergency is a share handed out by a process that may
already be the thing failing.

**C2 — k is a quorum of the voter set, and k > n/2.** Two disjoint quorums must
be impossible, or two leaders can each reconstruct the key.

**C3 — a share is not a capability.** Holding one grants nothing. Only
reconstruction does, and reconstruction requires k. This is what makes it
acceptable to hand shares to processes that are individually less trusted than
the root.

**C4 — reconstruction is fenced by term.** A supervisor may only reconstruct
after winning an election, and the reconstructed key is bound to that term. See
fencing below. Reconstructing without a term is how a compromised quorum steals
the vault at leisure.

**C5 — the old leader's key material is invalidated on term change**, to the
extent the vault can enforce it. See F2.

---

## Fencing: what actually prevents two leaders

Voting does not prevent split brain. A leader that is GC-pausing, swapping, or
merely slow is indistinguishable from a dead one, so a live leader can be voted
out while still running — still holding vault custody, still spawning agents.

The property needed is not agreement but **fencing**: the new leader must be
able to stop the old one from acting, without the old one's cooperation.

D18T already implements exactly this mechanism for channel epochs. Its core
invariant is that the active epoch lives in the *same versioned record* as the
pending write, so publication and activation contend on **one** compare-and-swap
and exactly one wins. A separate mutable "epoch head" is non-conforming because
two independent CAS operations cannot exclude each other.

Leadership terms are the same shape and should reuse it.

### Fencing rules

**F1 — every request the root supervisor makes of the vault carries its term.**

**F2 — the vault records the highest term it has seen and refuses any request
from a lower one.** This is what makes the old leader harmless: it does not need
to notice it was replaced, and it does not need to be reachable to be stopped.

**F3 — the term advances by compare-and-swap on the record that also holds
whatever the term authorises**, per D18T. Advancing the term and acting under it
must not be two operations, or the window between them is the bug.

**F4 — an agent host spawned under term N is terminated when the term
advances**, unless it can be re-adopted by the new leader. An orphaned agent
holding a lease minted under a dead term is a capability nobody is tracking.

**F5 — fencing is required; election is optional.** A single-machine deployment
using R4 still wants F1–F4, because a restarted root supervisor is a new leader
and the old one may still be exiting.

---

## Do you need consensus?

Probably not yet, and the honest answer shapes the build order.

Raft earns its complexity when the failed component is a **whole machine**, so
no local parent survives to restart it. If Chief of Staff runs as one daemon on
one laptop, the daemon *is* a surviving parent, R4 covers root death, and an
election protocol adds a distributed-systems failure surface to a problem that
does not have one.

Election becomes necessary when either is true:

- the tree spans machines, so a machine loss takes the root with it; or
- the root must survive daemon death, e.g. the daemon is itself being upgraded.

Until one holds, **implement R1–R4 and F1–F4 and stop.** They deliver the
crash-survival the architecture is named for. Election delivers survival of a
failure mode that does not exist yet on one machine.

---

## Election

Specified for when it is needed, not before.

**E1 — membership is fixed.** The voter set is configured, not derived from the
live branch-supervisor set. Raft's hardest correctness problems are membership
changes, and a voter set that grew and shrank with load — which is exactly what
capacity-driven supervisor spawning would do — would be changing membership
constantly and for reasons unrelated to consensus. Voters are a small fixed set;
branch supervisors may be spawned and reaped freely without touching it.

**E2 — a candidate must win a quorum of the voter set**, the same quorum as C2,
so that winning an election and reconstructing the key are the same threshold.

**E3 — the winner's first act is to advance the term** (F3), before
reconstructing custody (C4) and before accepting any placement request.

**E4 — a replacement branch supervisor is spawned for the promoted one**, so
capacity does not silently shrink by one on every failover.

**E5 — a supervisor that loses an election it started steps down and does not
retry until it has seen a higher term.** Retry storms among peers that cannot
reach a leader are how election protocols convert a slow leader into no leader.

---

## Placement

The root supervisor receives every agent-launch request, verifies the package,
chooses a branch supervisor, and delegates the spawn.

**P1 — verify before placing.** Signature and digest checks happen at the root,
once, before any branch supervisor is asked to do anything. `spawn_verified`
already does this; placement must not create a path around it.

**P2 — "overloaded" must be defined before it can be honoured.** There is no
resource accounting anywhere in the tree today, so placement currently has no
data to place on. At minimum a branch supervisor reports its live agent-host
count and its restart-intensity budget consumed. Anything richer — memory, CPU —
is a later refinement, and this spec deliberately does not invent a metric it
cannot yet measure.

**P3 — placement is advisory, spawning is authoritative.** The chosen branch
supervisor may refuse (it may have filled up since it reported), and the root
must handle refusal by choosing again rather than by forcing.

**P4 — an agent's identity is bound at placement.** The attested `agent_id` the
vault authorises against derives from the signed manifest through the
supervisor's `HostRegistration`. Placement must preserve that chain. A placement
path that let an agent supply its own identity would make per-secret
`allowed_agents` worthless — see D18D section 7.2.

---

## What this spec does not decide

- **Where host profiles live.** A generic tree needs a per-agent statement of
  allowed tools, tier, and capabilities. `chief-of-staff-daemon-config` has no
  such fields. The candidates are daemon config handed down, or the signed agent
  manifest. The manifest is already inside the integrity boundary that governs
  the code it describes, which argues for it, but this is unresolved.
- **Transport between tiers.** `SupervisedOrchestratorRuntime` is stdio/RPC
  shaped (`HostRpcRequest`); whether that is the right shape for supervisor-to-
  agent traffic is open.
- **Whether the vault is in-process or its own process.** C's option (3) works
  either way.

---

## Build order

Each step is independently useful, and each is a prerequisite for the next.

1. **Generic profile-backed supervisor dispatch**, smart home demoted to an
   agent. Unblocks everything; removes the hardcoded coupling.
2. **Restart policy** (R1–R3). The crash-survival the design is named for.
3. **Daemon restarts the root supervisor** (R4). Single-machine root recovery.
4. **Capacity reporting and placement** (P1–P3). Needed before more than one
   branch supervisor is useful.
5. **Terms and vault fencing** (F1–F4). Required by 3 as well as by election.
6. **Shamir custody sharing** (C1–C5). Makes promotion possible at all.
7. **Election** (E1–E5). Only when the tree spans machines.

---

## Citations

- Armstrong, *Making reliable distributed systems in the presence of software
  errors* — OTP supervision trees, restart intensity, the let-it-crash argument.
- Ongaro & Ousterhout, *In Search of an Understandable Consensus Algorithm* —
  terms, quorum, and the membership-change caveat behind E1.
- Shamir, *How to Share a Secret* — the k-of-n threshold behind C3.
- D18T-chief-of-staff-durable-epoch-activation-profile.md — the single-CAS
  fencing mechanism reused by F3.
