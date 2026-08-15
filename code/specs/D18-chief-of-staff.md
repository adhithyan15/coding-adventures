# D18 — Chief of Staff

## Overview

Every member of Congress has a Chief of Staff. The Chief of Staff does not read the
member's classified briefings. They do not draft the member's legal opinions. They do
not answer the member's phone calls. What they do is **orchestrate**: they know who is
on the team, what each person's job is, and they make sure the right people are
connected to the right tasks. If someone is sick, the Chief of Staff finds a
replacement. If a new issue comes in, they route it to the right staffer. But they
never do the work themselves, and they never see the content of privileged
communications.

This package implements a **personal AI agent system** built on this principle. It is
inspired by OpenClaw — the open-source AI agent platform that went viral in early
2026 — but takes a fundamentally different approach to security.

**OpenClaw's architecture** puts everything in one process: the Gateway holds all API
keys, reads all messages, runs all agent logic, and connects to all messaging
platforms. Compromising the Gateway gives an attacker everything — credentials,
conversations, the ability to send messages as the user, and arbitrary code execution.

**Chief of Staff's architecture** separates concerns into isolated actors connected
by encrypted channels. The Orchestrator, Host processes, and Vault are all implemented
in Rust with `#![forbid(unsafe_code)]`, communicating via message passing from the D19
Actor package. Agent code runs in TypeScript inside Deno processes with **every OS
capability denied by default** — agents interact with the outside world exclusively
through a host-mediated API that checks capabilities before proxying each request.

```
OpenClaw:                          Chief of Staff:
┌────────────────────────────┐     ┌──────────────┐   ┌──────────┐
│  GATEWAY (one process)     │     │ Orchestrator │   │ Host A   │
│  • holds all API keys      │     │ (Rust actor) │   │ (Rust)   │
│  • reads all messages      │     │ • no keys    │──▶│ • reads  │
│  • runs all agent logic    │     │ • no data    │   │   manifest│
│  • connects all platforms  │     │ • no manifest│   │ • enforces│
│  • one compromise = total  │     │ • launches   │   │   caps   │
│    access to everything    │     │   hosts only │   │ • proxies│
└────────────────────────────┘     └──────────────┘   │   all I/O│
                                                       │          │
                                   ┌──────────────┐   │  ┌──────┐│
                                   │    Vault      │   │  │ Deno ││
                                   │ (Rust actor)  │   │  │ agent││
                                   │ • encrypted  │   │  │ deny-││
                                   │ • OS-caged   │   │  │ all  ││
                                   │ • leased     │   │  └──────┘│
                                   └──────────────┘   └──────────┘
                                   Each host is a       Agents have
                                   separate Rust         ZERO direct
                                   actor process.        OS access.
```

The system has three primitives from D19 — **Messages**, **Channels**, and **Actors** —
plus three infrastructure components: the **Orchestrator** (signature verification and
process supervision), the **Host** (capability enforcement and OS access proxy), and the
**Vault** (encrypted credential storage with time-limited leases). A **Privilege Tier**
system gates sensitive operations with biometric and hardware-key challenges.

**The congressional office analogy** runs through the entire design:

| Congressional Office      | Chief of Staff System                              |
|---------------------------|-----------------------------------------------------|
| Chief of Staff            | **Orchestrator** — launches and supervises hosts      |
| Personal Security Escort  | **Host Process** — accompanies each agent, opens doors on their behalf, logs every room entered |
| Staff Assistant           | **Email Reader Agent** — handles incoming mail       |
| Communications Director   | **Email Responder Agent** — drafts outgoing messages |
| Scheduler                 | **Calendar Agent** — manages the schedule            |
| Counsel                   | **Finance Agent** — handles privileged matters       |
| Aide                      | **Browser Agent** — navigates and retrieves things   |
| Interns                   | **Utility agents** — weather, reminders, etc.        |
| The Member of Congress    | **You** — makes final calls on high-stakes decisions |
| Document Safe             | **Vault** — encrypted, OS-isolated, YubiKey-gated   |
| Courier                   | **Ephemeral Sub-Agent** — fetches one thing, returns, leaves |

The V1 implementation uses **Rust** for the orchestrator, host processes, and vault
(with `#![forbid(unsafe_code)]` enforced project-wide), and **Deno** as the agent
runtime. Deno's permission model denies all OS access by default, which combines with
the host's capability enforcement to create a two-layer cage. Agents written in
languages other than TypeScript compile to **WebAssembly** and run inside Deno,
creating a three-layer cage (Wasm sandbox + Deno deny-all + host mediation).

---

## Where It Fits

```
User (biometric auth: Face ID, Touch ID, YubiKey, passphrase)
│
▼
Orchestrator (Rust actor, D18)     ← signature verification + host supervision
│   ├── Signature Verifier         ← checks Ed25519 package signatures
│   ├── Host Supervisor            ← spawns/stops/restarts host actors
│   ├── Trust Checker              ← privilege tier enforcement
│   └── Service Registry           ← maps host names to process handles
│
├── Host Actor A (Rust, D18)       ← reads its own sealed package
│   ├── Capability Checker         ← manifest-driven policy enforcement
│   ├── Middleware Chain            ← rate limit, audit, trust boundary
│   ├── Ephemeral Sub-Agents       ← spawned per-request, die after response
│   └── Deno Process               ← agent code runs here (deny-everything)
│       └── host.* API only        ← JSON-RPC over stdin/stdout to Host
│
├── Host Actor B (Rust, D18)       ← same structure, different agent
│   └── ...
│
├── Vault (Rust actor, D18)        ← encrypted credential store
│   └── OS container: no network, no fs except store, Unix socket only
│
├── built on ──► Actor Package (D19)
│                └── Message, Channel, Actor primitives
│                └── Supervision trees, mailboxes, message passing
│
├── extends ──► Capability Security (Spec 13)
│                └── agent_manifest.json extends required_capabilities.json
│
├── uses ──► Messaging Crypto Foundation (MSG-CRYPTO-FOUNDATION)
│             └── XChaCha20-Poly1305, Ed25519, X25519, HKDF, SHA-256
│             └── Vault at-rest encryption uses VLT01 + Argon2id
│
├── uses ──► IPC (D16)
│             └── channels build on message queues / append-only logs
│
├── uses ──► Network Stack (D17)
│             └── host sub-agents needing HTTP access use socket API
│
├── uses ──► Process Manager (D14)
│             └── host/agent lifecycle uses fork/exec/wait
│
├── uses ──► File System (D15)
│             └── channel persistence, vault storage
│
├── defines ──► D18F Portable Message Profile
│                └── immutable D18 envelope, D18M v1 binary/JSON contract,
│                    rich payload conventions, and cross-language fixtures
│
├── defines ──► D18P Portable Durable Channel Profile
│                └── D18C membership, D18S reservation, D18A cursor,
│                    atomic persistence, recovery, roles, and conformance
│
└── extended by ──► Store / Job / Tool Layers
                    ├── D18A Store Layer
                    │    └── repository-owned storage abstraction + Context/Artifact/Skill/Memory stores
                    ├── D18C Job Framework
                    │    └── portable jobs + native scheduler backends
                    └── D18D Tool API
                         └── repository-owned model-facing tool contract + built-in tool catalog
```

**Depends on:** Actor Package (D19) — the foundation; messages, channels, and actors
are D19 primitives. Capability Security (Spec 13) — agent manifests extend the
capability taxonomy. IPC (D16) — channels build on message queue concepts. Network
Stack (D17) — host sub-agents needing external access use the socket API. Process
Manager (D14) — host and agent lifecycle uses fork/exec/wait. File System (D15) —
channel logs and vault secrets are stored on disk. MSG-CRYPTO-FOUNDATION provides the
transit-crypto contracts and test vectors; VLT01 owns Vault at-rest encryption and
Argon2id key derivation. D20 is the JSON specification and is not a crypto dependency.
D18F is the normative portable profile for the encrypted message envelope described
below; it preserves the production Rust `D18M` version 1 bytes and keeps them distinct
from D19's generic `ACTM` actor message. D18P is the normative portable profile for
the durable channel described below; it preserves the production Rust definition,
reservation, cursor, storage-key, CAS, recovery, and structural-role behavior while
leaving D18F message cryptography and channel-key grants in their existing owners.

**Extended by:** D18A Chief of Staff Stores — repository-owned storage abstraction,
ContextStore, ArtifactStore, SkillStore, and MemoryStore. D18C Chief of Staff Job
Framework — portable jobs and native scheduler backends. D18D Chief of Staff Tool API —
repository-owned model-facing tool contract and built-in tool catalog.

**Used by:** Future agent packages (email reader, email responder, calendar, finance,
health, browser agents), CLI interface, mobile clients

### Level 1 `SKILL.md` contract

A Level 1 agent is a CommonMark document with one H1 title, a descriptive first
paragraph, and a `## Capabilities needed` list. Each capability is written as
`category:action:target`, optionally followed by ` | justification`; `- none`
declares an explicit empty profile. The title and paragraph provide zero-config
identity defaults. Optional `---` frontmatter may override `agent`, `description`,
`privilege_tier`, `reads`, `writes`, `message_schema_versions`, and
`restart_policy`; unknown or duplicate keys fail closed. The parser emits the
schema-v2 `agent_manifest.json` shape and
sorted, deduplicated Deno permission arguments. Time and standard-stream
capabilities remain manifest declarations but do not widen Deno OS permissions.
The shared manifest codec continues to accept installed schema-v1 packages and
rejects malformed JSON, duplicate or unknown fields, and invalid nested
capability declarations before a package can participate in discovery or
registration. Schema v2 declares a positive payload-schema version for every
read/write channel; channel names scope those versions. Pipeline wiring fails
closed unless the originator's write declaration and receiver's read declaration
name the same version. Legacy v1 packages remain discoverable, but have no
implicit message-schema compatibility.
Discovery supports explicit inspection and stable immediate-child `.agent`
scans. It parses only authenticated manifest bytes, enforces signing-key tier
ceilings, and returns inert candidates for explicit control-plane registration.
Each complete snapshot is bounded to 4,096 packages. Reload planning compares
two fully verified snapshots and reports added, removed, and replaced identities
in stable order without mutating durable registration or live process state.
Package activation remains an explicit lifecycle operation: discovery never
silently replaces a running agent.
The authenticated reload operation requires stopped durable intent and fresh
absent or exited supervisor authority. It retains the stable agent identity,
revision-CAS replaces package identity, resets cached observation, and records
post-reload desired state atomically. If that state is running, the next normal
reconciliation tick launches the replacement; the orchestrator itself remains
online throughout.

The Level 1 runtime injects a provider-neutral LLM client plus authorized input
and output channel endpoints. For each verified UTF-8 message it sends the full
skill body as model instructions, sends the message as the user turn, publishes
the non-empty text response, and only then acknowledges the input. Model and
publication failures leave the channel cursor unchanged so recovery can replay
the message. Before readiness, the host receives a bounded authenticated launch
record from manifest-blind pipeline wiring. It requires the exact signed channel
name and read/write sets, resolves them to canonical UUID-v7 endpoints, and binds
the supplied model selector, temperature, and token cap. Missing, extra,
wrong-direction, or runtime-incompatible bindings fail closed.
The concrete `chief-of-staff-host` independently performs that verification and
executes the first production Level 1 loop over the authenticated host data plane.
V1 intentionally requires exactly one signed read channel and one signed write
channel; packages declaring broader topology remain valid and discoverable but
cannot reach readiness in this host until deterministic multi-channel routing is
specified. The host performs receive, completion, publish, then acknowledge,
sleeps between empty or read-service-unavailable polls, emits authenticated
heartbeats, and treats an authenticated termination received during any exchange
as a successful shutdown. Failures after input delivery remain terminal rather
than risking duplicate model or publication effects.
Pipeline wiring persists this authority as a bounded versioned record tied to
the exact durable host registration and package hash. Every channel UUID receives
an immutable create-if-absent pipeline claim. Wiring and each later launch both
reload the authoritative active channel definition and require the pipeline's
agent identity to be the originator for writes or a receiver for reads. Claims
survive unwiring and destroyed channel UUIDs are never recycled, preventing
partial failures or stale records from creating a cross-pipeline path.

### Level 4 any-language stdio contract

A Level 4 agent needs no SDK. The trusted host writes one UTF-8 JSON object per
LF-terminated stdin line and accepts one JSON object per LF- or CRLF-terminated
stdout line. The v1 host record is:

```text
{ protocol: "chief-agent-stdio-v1", kind: "message",
  message_id: string, channel_id: string,
  sequence: string, timestamp_ns: string,
  content_type: string, payload_b64: string }
```

The agent returns exactly:

```text
{ protocol: "chief-agent-stdio-v1", kind: "response",
  input_message_id: string,
  content_type: string, payload_b64: string }
```

The response must correlate to the one input currently in flight. Sequence and
timestamp are canonical unsigned decimal strings; payloads are canonical padded
base64. Identifiers and content types use the channel bounds, decoded payloads
are limited to 64 MiB, and encoded lines are limited to 90 MiB. Duplicate or
unknown fields, malformed JSON/base64, wrong versions or kinds, oversized data,
and correlation mismatches fail before output publication or input
acknowledgement. Nonzero process exit and EOF before a valid response likewise
leave the input cursor unchanged for recovery.

The host keeps one ordered subprocess session per Level 4 agent and permits
only one input to be in flight on that session. It receives one verified
channel message, obtains and validates the correlated subprocess response,
publishes the output, and only then acknowledges the input. Protocol or pipe
failure invalidates the session; the host kills and reaps its owned child so a
supervisor can restart from the unchanged channel cursor. Package verification,
sandbox selection, restart policy, and timeout enforcement remain outside the
wire codec and process adapter.

---

## Key Concepts

### Primitive 1: Message

A Message is the atom of communication in the system. Every piece of data that flows
between any two components — a user's request, an agent's response, a credential from
the vault — is a Message. The Message type comes from D19 (Actor); D18 extends it with
encryption fields for channel-level confidentiality.

[D18F](D18F-chief-of-staff-message-profile.md) defines the normative cross-language
field bounds, authenticated framing, binary and JSON encodings, verification order,
and rich-payload conventions for this encrypted envelope. D19 `ACTM` messages remain
generic actor transport values and do not substitute for the D18 envelope.

**Analogy:** A Message is a sealed, stamped, postmarked letter. Once it is sealed
(created), nobody can change its contents. The stamp proves who sent it. The postmark
records when. And the envelope is opaque — only the intended recipient can open it.

```
Message
═══════════════════════════════════════════════════════════════
┌──────────────────┬─────────────────────────────────────────┐
│ id               │ UUID v7 (time-ordered, globally unique)  │
│ timestamp        │ Monotonic nanosecond clock                │
│ originator_id    │ Who created this message                  │
│ channel_id       │ Which channel this message belongs to     │
│ sequence         │ Monotonic counter within the channel      │
│ key_epoch        │ Channel-key generation used by payload    │
│ content_type     │ MIME type (application/json, text/plain)  │
│ payload          │ Encrypted bytes (ciphertext)              │
│ plaintext_hash   │ SHA-256 of plaintext (integrity check)    │
│ signature        │ Originator's Ed25519 signature            │
└──────────────────┴─────────────────────────────────────────┘
```

**Key properties:**

1. **Immutability.** Once created, a Message is never modified. The fields are set at
   creation time and are read-only thereafter. This is not a convention — the data
   structure has no setter methods.

2. **Integrity.** The `plaintext_hash` is a SHA-256 digest of the unencrypted payload,
   computed before encryption. A receiver who decrypts the payload can verify that the
   plaintext has not been tampered with by recomputing the hash.

3. **Authenticity.** The `signature` is an Ed25519 signature over a canonical,
   length-framed encoding of the message ID, timestamp, originator ID, channel ID,
   sequence number, key epoch, content type, and plaintext hash. A receiver can verify
   that the message was created by the claimed originator by checking the signature
   against the originator's public key.

4. **Opacity.** The `payload` is always ciphertext. Anyone who intercepts the message
   without the channel's decryption key sees only random bytes. This includes the
   orchestrator and host processes, which route channels but never hold decryption keys.

---

### Primitive 2: Channel

A Channel is a one-way, append-only, encrypted pipe. It connects exactly one
originator to one or more receivers. Messages flow in one direction only — from the
originator's write end to the receivers' read ends. A Channel is never bidirectional.
The Channel type comes from D19 (Actor); D18 extends it with encryption keys and
persistent storage.

[D18P](D18P-chief-of-staff-durable-channel-profile.md) defines the normative
cross-language durable channel contract: exact D18C/D18S/D18H/D18A records,
storage keys and content types, reserve-before-encrypt recovery, ordered reads,
per-receiver cursors, irreversible lifecycle, structural roles, stable failures,
and the required six-language conformance rollout. D18F continues to own D18M
message bytes. [D18Q](D18Q-chief-of-staff-channel-key-grant-profile.md) defines
the portable D18G sealed-grant, receiver-epoch, and cryptographic rotation
contract owned by #141.

**Analogy:** A Channel is a one-way pneumatic tube in an office building. Documents go
in one end and come out the other. You cannot send documents backwards through the
tube. The tube keeps a copy of every document that passes through it (append-only log),
and each office at the receiving end has a bookmark showing which documents they have
already read (offset tracking).

```
Channel
═══════════════════════════════════════════════════════════════
┌───────────────────┬────────────────────────────────────────┐
│ id                │ UUID v7                                 │
│ originator_id     │ The single entity that writes messages  │
│ receiver_ids      │ List of entities that read messages     │
│ key_epoch         │ Current monotonically increasing epoch   │
│ channel_master_key│ Current 256-bit content key; held by     │
│                   │ originator + authorized receivers only   │
│ sealed_key_grants │ Map<receiver_id, SealedChannelKey>       │
│ log               │ Append-only list of Messages            │
│ created_at        │ Timestamp                               │
└───────────────────┴────────────────────────────────────────┘

ReceiverState (tracked per receiver)
═══════════════════════════════════════════════════════════════
┌───────────────────┬────────────────────────────────────────┐
│ receiver_id       │ Which receiver this state belongs to    │
│ channel_id        │ Which channel                           │
│ last_ack          │ Sequence number of last processed msg   │
│ epoch_keys        │ Unwrapped CMKs keyed by key epoch        │
└───────────────────┴────────────────────────────────────────┘
```

**One-way enforcement:**

```
Originator                                    Receiver(s)
┌──────────┐     Channel (one direction)     ┌──────────┐
│          │ ══════════════════════════════▶  │          │
│  write() │     m0  m1  m2  m3  m4  ...     │  read()  │
│          │                                  │  ack()   │
│ ✗ read() │     ◄── NOT POSSIBLE ──►        │ ✗ write()│
└──────────┘                                  └──────────┘
```

If a receiver needs to respond to an originator, it publishes on a **different
channel** where the roles are reversed. This is not a limitation — it is the core
security property. A compromised receiver cannot inject messages backwards into the
channel it reads from.

**Offset tracking:**

Each receiver independently tracks how far it has read through the channel log. When
a receiver processes a message, it calls `ack(message_id)` to advance its offset.
On crash and restart, the receiver resumes from `last_ack + 1`.

```
Channel log:  [m0] [m1] [m2] [m3] [m4] [m5]
                                    ▲
Receiver A:   last_ack = 3 ─────────┘
                         ▲
Receiver B:   last_ack = 1 (behind — maybe processing slowly)
```

Receivers are independent. Receiver A being ahead does not affect Receiver B. There
is no "consumer group" coordination — each receiver is on its own.

**Comparison to D16 IPC primitives:**

| Property          | D16 Pipe         | D16 Message Queue | D18 Channel            |
|-------------------|------------------|-------------------|------------------------|
| Direction         | One-way          | Any-to-any        | **One-way**            |
| Persistence       | In-memory only   | In-memory only    | **Persistent on disk** |
| Encryption        | None             | None              | **Always encrypted**   |
| Offset tracking   | No               | No                | **Per-receiver offsets**|
| Single originator | Yes (writer end) | No (any process)  | **Yes (enforced)**     |
| Append-only       | No (consumed)    | No (consumed)     | **Yes (immutable log)**|

Channels build on D16's concepts but add encryption, persistence, and the
single-originator constraint.

---

### Primitive 3: Originator and Receiver

An **Originator** is anything that writes messages to a channel. A **Receiver** is
anything that reads messages from a channel. These are roles, not types — any entity
in the system can be an originator on some channels and a receiver on others.

**Analogy:** In a congressional office, the Staff Assistant receives phone calls
(Receiver on the "incoming calls" channel) and writes call summaries for the Member
(Originator on the "call summaries" channel). They receive on one channel and
originate on another — but never both on the same channel.

**Rule:** An entity cannot be both originator and receiver on the same channel. This
is enforced by the Channel's structure — it has a single `originator_id` and a list
of `receiver_ids`, and these sets must be disjoint.

**Role matrix:**

| Entity               | Originator on                | Receiver on                  |
|----------------------|------------------------------|------------------------------|
| User (CLI)           | user-requests                | agent-responses              |
| Email Reader Agent   | email-summaries              | user-requests (filtered)     |
| Email Responder Agent| outgoing-emails              | draft-requests               |
| Browser Agent        | page-content, vault-requests | browse-commands, vault-leases|
| Vault                | vault-leases                 | vault-requests               |
| Calendar Agent       | schedule-summaries           | schedule-requests            |
| Cron Timer           | scheduled-events             | (none — pure originator)     |

Some entities are **pure originators** (a cron timer only produces messages) or
**pure receivers** (a logger only consumes messages). Most agents are both, but on
different channels. All channel access from agents is mediated by the host process —
agents call `host.channel.read()` and `host.channel.write()`, and the host verifies
the agent is the registered originator or receiver before proxying.

---

### Actor Model Foundation

The entire Chief of Staff system is built on the D19 Actor package. Every component —
the orchestrator, each host process, the vault, and every ephemeral sub-agent — is an
actor: an isolated unit of computation with a mailbox, internal state, and no shared
memory.

**Analogy:** The congressional office itself is an actor system. Each staffer (actor)
has their own desk (internal state) and mailbox (inbox). Staffers communicate by
sending memos (messages), not by reaching into each other's desks. The Chief of Staff
(orchestrator) can hire and fire staffers (spawn and kill actors) but cannot read
their desk contents.

```
Orchestrator Actor (Rust)
├── owns: service registry, trusted keyring
├── mailbox: receives health reports, launch requests
├── supervises: host actors
│
├── Host Actor (email-reader) (Rust)
│   ├── owns: sealed package, capability table, Deno child process
│   ├── mailbox: receives JSON-RPC from Deno, sub-agent responses
│   ├── supervises: Deno process + ephemeral sub-agents
│   │
│   ├── Ephemeral Sub-Agent (NetworkProxy) (Rust)
│   │   └── born → services ONE request → dies
│   ├── Ephemeral Sub-Agent (FileProxy) (Rust)
│   │   └── born → services ONE request → dies
│   └── Deno Process (TypeScript agent code)
│       └── can only call host.* API via stdin/stdout
│
├── Host Actor (finance-agent) (Rust)
│   ├── owns: sealed package, capability table, Deno child process
│   └── supervises: Deno process + ephemeral sub-agents
│
└── Vault Actor (Rust, OS container)
    ├── owns: encrypted store, master key (in memory)
    ├── mailbox: receives lease requests, unlock requests
    └── supervises: nothing (leaf actor)
```

**Key property:** All communication between actors is via message passing. The
orchestrator sends a "launch" message to create a host actor. The host actor sends
capability-checked requests to ephemeral sub-agents. The vault actor receives lease
requests via its mailbox. No actor can reach into another actor's state — this is
enforced by Rust's ownership system at compile time.

---

### The Orchestrator (Chief of Staff)

The Orchestrator is the system's daemon process. It is deliberately **as dumb as
possible** — its job is signature verification, host process supervision, trust
checking, and service discovery. It does not read agent manifests, generate Deno
flags, route messages, read payloads, hold secrets, or run agent logic. It does not
know what capabilities any agent has.

**Analogy:** The Chief of Staff knows that the Communications Director's office is in
room 204 and that the sealed personnel file on the door has a valid signature. They
can unlock the door (launch the host process). But they cannot read the personnel
file (manifest), they cannot enter the room (agent's Deno process), and they do not
know what the Communications Director is authorized to do. The personnel file is read
by the security escort (host process), not the Chief of Staff.

**Launch sequence:**

```
1. Orchestrator receives: "launch email-reader.agent/"
2. Orchestrator verifies: Ed25519 signature on package → valid?
   ├── NO  → refuse to launch, log "invalid signature"
   └── YES → continue
3. Orchestrator spawns: Host Actor process
   argument: path to the sealed package (./email-reader.agent/)
4. Orchestrator records: host PID, agent name, "starting" status
5. Orchestrator sends the exact relevant public package trust over the fresh
   authenticated child session.
6. Orchestrator sends authorized named-channel UUID mappings and bounded Level 1
   model settings from pipeline wiring without reading the package manifest.
7. Host independently re-reads and verifies the package, matches the bindings to
   its signed policy, and sends
   authenticated Ready(package_hash) over its per-spawn control channel.
8. Orchestrator marks Running only when that hash matches the registration.
9. Orchestrator monitors authenticated heartbeats using trusted receipt time.
10. Done. Orchestrator does NOT:
   • read manifest.json
   • know what capabilities the agent has
   • know what Deno flags are used
   • know what channels the agent reads/writes
   • know what vault secrets the agent can access
```

The strict readiness, heartbeat, and termination state machine is specified in
[`host-control-protocol.md`](host-control-protocol.md).
The concrete Level 1 child composition is specified in
[`level-one-host.md`](level-one-host.md).
That same authenticated session carries a serialized, bounded channel and
provider-neutral text and tool-aware completion data plane after readiness. The
tool-aware operation carries bounded declarations, selection policy, replayable
prior call/results, and one structured response without authorizing or executing
the emitted D18D call. The protocol permits one
request in flight with exact monotonic correlation; the orchestrator-side adapter
authorizes and executes the operation without creating a second child pipe. For
every request, the dedicated data-plane dispatcher reloads the exact pipeline
binding, immutable claims, active channel topology, directional membership, and
model settings. It rejects unknown or wrong-direction channel UUIDs and any model
setting drift before invoking the separately injected channel/LLM service, then
validates response identity and operation shape before returning it. The
payload-blind orchestration core receives no request body. When the optional
data-plane table is absent or empty, the production daemon uses the same
authorization boundary with a redacted unavailable service. A non-empty table is
provisioned at daemon startup and injects the authority-backed service, which
executes against the real encrypted durable channel
endpoints, retains a bounded delivery-to-acknowledgement ledger, provisions sealed
receiver grants before publication, and resolves only exact model selectors. Key
release and provider construction remain separately injected authorities so the
orchestration core stays payload- and secret-blind; production configuration must
make those authorities explicit before replacing the unavailable adapter.
The concrete pre-composition key registry owns only zeroizing secret buffers, binds
each directional key to an exact pipeline, agent, and channel, and releases a new
short-lived crypto owner only after all three identities match the reloaded durable
binding. Filesystem and Vault formats remain separate provisioning adapters.
The file-backed production adapter in the concrete daemon consumes only the typed
declarations above,
loads raw private material through owner-only no-link exact-length reads, and
constructs exact Ollama clients without probing the network. A future Vault
adapter may populate the same registries without changing execution semantics.
The production Level 1 integration gate joins these boundaries with the real host
process, durable launch binding, encrypted request and report channels, a loopback
Ollama wire fixture, and the publish-before-acknowledge invariant. The fixture
substitutes only the model server's response; package verification, process
control, authorization, key provisioning, channel crypto, and persistence remain
the production implementations.

```
┌────────────────────────────────────────────────────────────────┐
│                    ORCHESTRATOR (Chief of Staff)                 │
│                    Rust, #![forbid(unsafe_code)]                 │
│                                                                  │
│  ┌────────────────┐  ┌────────────────┐  ┌──────────────────┐  │
│  │ Signature      │  │ Host           │  │ Service          │  │
│  │ Verifier       │  │ Supervisor     │  │ Registry         │  │
│  │                │  │                │  │                  │  │
│  │ • verify       │  │ • spawn host   │  │ • host name      │  │
│  │   Ed25519 sig  │  │ • stop host    │  │ • host status    │  │
│  │ • trusted      │  │ • restart on   │  │ • host PID       │  │
│  │   keyring      │  │   crash        │  │                  │  │
│  │                │  │ • health check │  │ (knows THAT      │  │
│  │ (verifies      │  │                │  │  hosts exist,    │  │
│  │  packages,     │  │ (manages       │  │  not WHAT        │  │
│  │  never reads   │  │  lifecycle,    │  │  they do)        │  │
│  │  contents)     │  │  not policy)   │  │                  │  │
│  └────────────────┘  └────────────────┘  └──────────────────┘  │
│                                                                  │
│                                           ┌──────────────────┐  │
│                                           │ Trust            │  │
│                                           │ Checker          │  │
│                                           │                  │  │
│                                           │ • check tier     │  │
│                                           │ • request        │  │
│                                           │   biometric      │  │
│                                           │ • request        │  │
│                                           │   hardware key   │  │
│                                           └──────────────────┘  │
└────────────────────────────────────────────────────────────────┘
```

**What compromise gets you:**

| Component Compromised | Attacker Can                           | Attacker Cannot                         |
|-----------------------|----------------------------------------|-----------------------------------------|
| **Orchestrator**      | DOS (stop/restart hosts), see what     | Read any manifest, know any capability, |
|                       | host names exist in the registry       | read any message, access any secret,    |
|                       |                                        | impersonate any agent, forge a signed   |
|                       |                                        | package, modify Deno flags              |
| **One host**          | See that host's manifest, proxy        | Access other hosts' manifests, read     |
|                       | requests for that agent's capabilities | other agents' channels, access vault    |
|                       |                                        | secrets beyond that agent's declared    |
|                       |                                        | access, escalate privilege tier         |
| **One agent (Deno)**  | Compute in memory, call host.* API     | Call any OS API directly, access files, |
|                       | (host still checks capabilities)       | open sockets, read env vars, spawn     |
|                       |                                        | processes — Deno denies everything      |
| **Vault**             | Read all stored secrets (if master     | Send messages as any agent, wire new    |
|                       | key is in memory)                      | hosts, bypass biometric gates, escape   |
|                       |                                        | OS container (no network, no fs)        |
| **Build pipeline**    | Sign malicious packages                | Extract private signing key from        |
|                       |                                        | YubiKey (hardware-bound)                |

The key insight: in OpenClaw, compromising the Gateway is game over — you get
everything. In Chief of Staff, there is no single component whose compromise gives
total access. The orchestrator is **deliberately worthless** to compromise — it holds
no secrets, reads no manifests, and knows no capabilities.

---

### The Host Process

The Host is the **policy enforcement point** for each agent. It sits between the
orchestrator and the Deno process, reading the agent's manifest, enforcing capabilities,
and proxying all OS access. Every interaction between an agent and the outside world —
network, filesystem, vault, channels — passes through the host.

**Analogy:** The host is a personal security escort assigned to each staffer. The
staffer (agent) cannot walk freely through the building. They are in a locked room
with no doors and no windows. When they need something — a file from the records room,
a phone call to an external number, a document from the safe — they write a request on
a slip of paper and slide it through a slot in the wall. The escort reads the request,
checks the staffer's badge (capability manifest), and either fetches the item or
returns a "denied" slip. The staffer never leaves the room.

**Launch chain:**

```
Orchestrator                Host Actor                Deno Process
───────────                 ──────────                ────────────

verify signature
of email-reader.agent/
    │
    ▼
spawn host actor
  arg: ./email-reader.agent/
    │
    │                       reads sealed package
    │                       verifies signature (again)
    │                       parses manifest.json
    │                       builds capability table
    │                           │
    │                           ▼
    │                       spawns Deno process
    │                         --deny-net
    │                         --deny-read (except /app/code)
    │                         --deny-write
    │                         --deny-run
    │                         --deny-env
    │                         --deny-ffi
    │                         --no-prompt
    │                         stdin/stdout piped to host
    │                           │
    │                           ▼
    │                       Deno runs agent code
    │                       agent calls host.* API
    │                           │
    │                       host checks capability
    │                       host spawns ephemeral sub-agent
    │                       sub-agent does work, returns, dies
    │                       host returns result to Deno
    │
monitors: is host alive?
(host crashes → restart)
```

The double signature verification is intentional — the orchestrator checks to avoid
launching garbage, and the host checks again because it does not trust the orchestrator.

**The `host.*` API — the agent's entire universe:**

```typescript
// chief-of-staff SDK — imported by every agent
// These functions do NOT call Deno APIs.
// They send JSON-RPC messages to the host over stdin/stdout.

export const host = {

  // Network — host checks manifest, spawns ephemeral sub-agent
  network: {
    fetch(url: string, options?: RequestInit): Promise<Response>,
  },

  // Filesystem — host checks manifest, spawns ephemeral sub-agent
  fs: {
    read(path: string): Promise<Uint8Array>,
    write(path: string, data: Uint8Array): Promise<void>,
  },

  // Vault — host checks manifest + trust boundary + tier
  vault: {
    requestLease(secretName: string, ttlMs: number): Promise<VaultLeaseReceipt>,
    requestDirect(secretName: string, consumer: AgentId): Promise<void>,
    releaseLease(vaultRef: VaultRef): Promise<void>,
  },

  // Channels — host verifies agent is registered originator/receiver
  channel: {
    read(channelId: string): Promise<Message | null>,
    write(channelId: string, payload: Uint8Array): Promise<void>,
    ack(channelId: string, messageId: string): Promise<void>,
  },

  // System — always available, no capability check needed
  system: {
    now(): number,                         // monotonic clock
    randomBytes(n: number): Uint8Array,    // CSPRNG from host
    log(level: string, msg: string): void, // structured logging
  },
};
```

There is no `fetch()`, no `Deno.readFile()`, no `Deno.connect()`. Those do not exist
in the agent's world. The `host.*` API is the **only** interface to anything outside
the agent's memory.

**JSON-RPC protocol over stdin/stdout:**

```
Agent → Host (stdin):
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "network.fetch",
  "params": {
    "url": "https://api.weather.com/today",
    "method": "GET"
  }
}

Host checks capability → spawns sub-agent → returns:

Host → Agent (stdout):
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "status": 200,
    "body": "{\"temp\": 72, \"condition\": \"sunny\"}"
  }
}

OR if denied:

{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32001,
    "message": "CapabilityDenied",
    "data": {
      "requested": "net:connect:api.weather.com:443",
      "agent": "email-reader",
      "reason": "not in manifest"
    }
  }
}
```

The error happens **before Deno's runtime, before the OS, before anything**. The host
simply never makes the call.

**Middleware chain:**

Every host API call passes through a middleware chain before being proxied:

```
Agent calls host.vault.requestLease("bank-creds", 10_000)
│
▼
┌─────────────────────────────────────────────────┐
│ Middleware chain (Rust, in host actor)            │
│                                                  │
│  1. Capability check                             │
│     manifest.vault_access.secrets                │
│     contains "bank-creds"?                       │
│     NO → return CapabilityDenied immediately     │
│     YES → next                                   │
│                                                  │
│  2. Trust boundary check                         │
│     bank-creds.privilege_tier = 2                │
│     Tier 2 requires biometric                    │
│     → pause, send biometric challenge to phone   │
│     → wait for Face ID                           │
│     DENIED → return TrustBoundaryDenied          │
│     APPROVED → next                              │
│                                                  │
│  3. Rate limiting                                │
│     Has this agent requested this secret          │
│     more than N times in the last hour?          │
│     YES → return RateLimited                     │
│     NO → next                                    │
│                                                  │
│  4. Audit log                                    │
│     Log: agent=finance, secret=bank-creds,       │
│          tier=2, approved_via=faceid,             │
│          timestamp=2026-03-23T14:22:00Z          │
│                                                  │
│  5. Spawn ephemeral sub-agent                    │
│     Sub-agent proxies to vault → returns result  │
│     Sub-agent dies immediately                   │
│                                                  │
│  6. Return result to agent                       │
└─────────────────────────────────────────────────┘
```

Middleware can be added without changing agents. Rate limiting, anomaly detection,
audit logging — all happen in the host, invisible to the agent.

**Trust boundary hierarchy:**

```
Memory (free)
  → Channels (pre-wired, no tier check, fast)
    → Network (capability check per URL)
      → Vault (capability + tier + maybe biometric + maybe YubiKey)
```

Two agents chatting via channels = fast, no tier check, pre-authorized at pipeline
wiring time. An agent requesting vault credentials = full middleware chain including
potential biometric challenge.

---

### Ephemeral Sub-Agents

When a host needs to service an OS-level request (network, filesystem, vault), it
does not perform the operation itself. It spawns a **single-purpose sub-agent** that
services exactly one request and dies immediately.

**Analogy:** A courier. The staffer needs a document from the records office. The
escort (host) does not fetch it themselves — they send a courier. The courier goes to
the records office, picks up the one document, brings it back, and leaves. They are
not an employee. They have no badge. They were created for this one errand and cease
to exist when it's done.

```
Agent calls: host.network.fetch("https://imap.gmail.com:993")

Host:
  1. Check manifest → allowed ✓
  2. Spawn NetworkProxyAgent("imap.gmail.com", 993)
  3. NetworkProxyAgent makes the request
  4. NetworkProxyAgent returns response to host
  5. Host returns response to agent
  6. Kill NetworkProxyAgent

Lifetime: milliseconds.
State after: nothing. Zero. Gone.
```

**Why ephemeral:**

- **No attack window.** By the time an attacker could exploit the sub-agent, it no
  longer exists. The socket is closed. The memory is freed. The process is gone.
- **No lingering state.** No connection pools, no cached credentials, no stale handles.
  Every request is a fresh actor with a fresh connection.
- **No session reuse attacks.** There is no session to reuse. Each request is isolated.
- **Blast radius of compromise: ONE thing.** A compromised network sub-agent for
  `imap.gmail.com:993` gets you a connection to `imap.gmail.com:993`. It cannot
  access the filesystem, the vault, other hosts, or anything else. The sub-agent
  was born knowing only one host and one port.

```
Timeline:

t=0ms    Agent calls host.network.fetch(imap.gmail.com)
t=1ms    Host spawns NetworkProxyAgent
t=2ms    NetworkProxyAgent opens connection
t=50ms   NetworkProxyAgent receives response
t=51ms   NetworkProxyAgent returns response to host
t=52ms   NetworkProxyAgent killed. Memory freed. Gone.
t=53ms   Host returns response to agent

Total sub-agent lifetime: 51ms
Attack window: 51ms (and shrinking with faster networks)
```

The cost is spawn overhead per request. For AI agents that take seconds to think about
an email, milliseconds of process spawn overhead is invisible.

---

### The Agent Cage

An agent is an isolated process that performs a specific task. Each agent runs inside
a Deno process with **every OS capability denied**. The agent has no direct access to
the network, filesystem, environment variables, subprocess spawning, or FFI. Its
entire interface to the outside world is the `host.*` API.

**Analogy:** Each staffer is in a soundproof, windowless room. There is no door — just
a slot in the wall. They can think, they can write, they can process what's in the room.
But to interact with anything outside the room, they must slide a request through the
slot and wait for their escort (host) to return with a response. The escort checks
every request against the staffer's badge before acting.

**Deny-everything Deno flags:**

Every agent, regardless of capabilities, runs with the same Deno flags:

```bash
#!/bin/sh
exec deno run \
  --deny-net \           # no network access at all
  --deny-read=/ \        # no filesystem reads (except agent code)
  --deny-write \         # no filesystem writes
  --deny-run \           # no subprocess spawning
  --deny-env \           # no environment variable access
  --deny-ffi \           # no native function calls
  --no-prompt \          # never ask for permission at runtime
  --allow-read=/app/code \  # can read its own source (needed to run)
  /app/code/agent_runtime.ts
```

The differentiation between agents happens entirely in the host process based on
the manifest. Deno is the inner wall — the host is the gatekeeper.

**Two layers of cage enforcement:**

```
Layer 1: Host Process (primary enforcer)
  Agent calls host.network.fetch("evil.com")
  Host checks manifest: net:connect:evil.com? NO.
  Host returns CapabilityDenied.
  The request never reaches Deno or the OS.

Layer 2: Deno Runtime (backup enforcer)
  Even if the host had a bug and somehow didn't check,
  the agent has --deny-net. Deno itself blocks the call.
  The request never reaches the OS.

To bypass both layers, an attacker must:
  1. Exploit the host's Rust code (safe Rust, no unsafe)
  2. AND exploit Deno's V8 sandbox
  3. Simultaneously
```

**Agent taxonomy (congressional office mapping):**

| Agent              | Office Role     | Host API Used              | Vault Access          | Tier |
|--------------------|-----------------|----------------------------|-----------------------|------|
| Email Reader       | Staff Assistant | host.network, host.channel | gmail-oauth (direct)  | 0    |
| Email Responder    | Comms Director  | host.network, host.channel | smtp-creds (direct)   | 1    |
| Calendar           | Scheduler       | host.network, host.channel | gcal-token (direct)   | 0    |
| Finance            | Counsel         | host.channel, host.vault   | bank-creds (leased)   | 2    |
| Browser            | Aide            | host.network, host.channel | site-passwords (leased)| 1   |
| Weather            | Intern          | host.network, host.channel | (none)                | 0    |

---

### Signed Agent Packages

Agents are distributed as **sealed, signed packages**. The build system reads the
manifest, generates a launch script with hardcoded Deno flags (no variables, no
arguments, literal strings only), and signs the entire package with an Ed25519 key.
The orchestrator verifies the signature but never reads the contents. The host process
reads and re-verifies.

**Package format:**

```
email-reader.agent/
├── manifest.json          # capabilities declaration
├── code/
│   ├── agent_runtime.ts   # the host.* SDK + agent entrypoint
│   ├── email_reader.ts    # agent logic (or agent.wasm for Wasm agents)
│   └── wasm_loader.ts     # (only for Wasm agents)
├── launch.sh              # GENERATED at build time, hardcoded Deno flags
├── SIGNATURE              # Ed25519 signature over everything above
└── PUBKEY_ID              # which build key signed this (key ID, not the key)
```

**Build pipeline (CI, trusted environment):**

```
BUILD PIPELINE
│
├── 1. Read manifest.json
│      { "runtime": "typescript",
│        "capabilities": [
│          { "category": "net", "target": "imap.gmail.com:993" }
│      ]}
│
├── 2. Generate launch.sh with HARDCODED flags
│      #!/bin/sh
│      exec deno run \
│        --deny-net \
│        --deny-read=/ \
│        --deny-write \
│        --deny-run \
│        --deny-env \
│        --deny-ffi \
│        --no-prompt \
│        --allow-read=/app/code \
│        /app/code/agent_runtime.ts
│
│      No variables. No arguments. No $1. No interpolation.
│      The flags are LITERAL STRINGS baked into the script.
│
│      NOTE: all agents get the SAME deny-everything flags.
│      Differentiation happens in the host, not in Deno.
│
├── 3. Hash everything
│      SHA-256 of: launch.sh + every file in code/ + manifest.json
│      Concatenated in deterministic sorted order
│
├── 4. Sign the hash
│      Ed25519_sign(build_private_key, hash) → SIGNATURE
│
└── 5. Output: email-reader.agent/ (sealed package)
```

**Three key types:**

```
┌─────────────────────────────────────────────────┐
│  Production key (CI YubiKey)                     │
│  ├── Full Tier 0-3 access                        │
│  ├── Real vault secrets                          │
│  ├── No debug flags                              │
│  └── Used for: your own shipped agents           │
├─────────────────────────────────────────────────┤
│  Developer key (local, auto-generated)           │
│  ├── Tier 0-1 only                               │
│  ├── Dev vault (fake/test secrets)               │
│  ├── Deno inspector allowed                      │
│  └── Used for: local development and testing     │
├─────────────────────────────────────────────────┤
│  Third-party key (community agents)              │
│  ├── User reviews manifest before install        │
│  ├── Tier capped by user's approval              │
│  ├── launch.sh regenerated locally, NOT shipped  │
│  └── Used for: marketplace/community agents      │
└─────────────────────────────────────────────────┘
```

**Third-party agent installation:**

For community agents, the author ships code + manifest but NOT launch.sh. The user's
local build tool regenerates launch.sh from the manifest. This prevents authors from
smuggling extra Deno flags into the launch script.

```
$ chief-of-staff install cool-agent

┌────────────────────────────────────────────────┐
│  Installing: cool-agent v1.2.0                 │
│  Author: jane@example.com                      │
│  Signed: Yes (author key F7A2...3B)            │
│                                                │
│  Requested capabilities:                       │
│    ├── net: api.openai.com:443                  │
│    ├── channel: read user-requests              │
│    ├── channel: write agent-responses            │
│    └── vault: none                              │
│                                                │
│  Privilege tier: 0                              │
│                                                │
│  [Install] [Reject] [View source]              │
└────────────────────────────────────────────────┘
```

**Developer workflow:**

```
1. Write agent code
   └── my-agent/manifest.json + code/my_agent.ts

2. Build and sign locally
   $ chief-of-staff build my-agent/
   (reads manifest, generates launch.sh, signs with dev key)

3. Run it
   $ chief-of-staff run my-agent.agent/
   (orchestrator verifies dev signature, launches host, host launches Deno)
   (dev key → tier 0-1 only, dev vault with test secrets)

4. Debug it
   $ chief-of-staff run my-agent.agent/ --debug
   (same cage, but Deno inspector enabled on 127.0.0.1:9229)
   (attach VS Code or Chrome DevTools)

5. Test with mock channels
   $ chief-of-staff test my-agent.agent/
   (interactive REPL: send messages, observe responses, same cage)
```

**Promotion pipeline:**

```
Developer                    CI                      Production
─────────                    ──                      ──────────

Writes code
     │
     ▼
chief-of-staff build
(signs with dev key)
     │
     ▼
Tests locally
(dev vault, Tier 0-1 only)
     │
     ▼
git push
     │
     ▼
                    CI pulls code
                    CI runs tests
                    CI builds package
                    CI signs with production key (YubiKey)
                    CI publishes .agent package
                         │
                         ▼
                                        User installs .agent
                                        Orchestrator verifies
                                          production signature
                                        Full Tier 0-3 access
```

---

### Dual Runtime Support

Agents can be written in TypeScript (run directly in Deno) or any language that
compiles to WebAssembly (run in Wasm inside Deno). Both look identical to the
orchestrator and host — just Deno processes with the same deny-everything flags.

**TypeScript agents (single cage):**

```
Orchestrator → Host → Deno process → runs agent.ts
Cage: Deno deny-everything + host mediation
```

**Wasm agents (double cage):**

```
Orchestrator → Host → Deno process → loads agent.wasm via wasm_loader.ts
Cage: Wasm linear memory sandbox + Deno deny-everything + host mediation
```

The manifest declares which runtime to use:

```json
{
  "agent": "finance-agent",
  "runtime": "wasm",
  "entrypoint": "finance_agent.wasm",
  "capabilities": [ ... ]
}
```

**Security comparison:**

| Attack                        | TS agent (Deno only) | Wasm agent (Wasm + Deno) |
|-------------------------------|----------------------|--------------------------|
| Unauthorized network call     | Host blocks, Deno blocks | Host blocks, Wasm: no import, Deno blocks |
| Read filesystem               | Host blocks, Deno blocks | Host blocks, Wasm: no import, Deno blocks |
| Read another agent's memory   | Separate process     | Wasm linear memory + separate process |
| Exploit in agent code         | Contained by Deno    | Contained by Wasm, then Deno |
| V8 sandbox escape             | At OS process level  | **Still in Deno process** (Wasm cage holds) |
| Wasm runtime bug              | N/A                  | Escapes to Deno (Deno cage holds) |

**Recommended tier mapping:**

```
Tier 0-1 agents (email, calendar, weather):
  TypeScript is fine. One cage. Low-stakes data.
  Developer ergonomics matter more.

Tier 2-3 agents (finance, health, vault access):
  Wasm preferred. Two cages. High-stakes data.
  The extra compilation step is worth the isolation.
```

The orchestrator can enforce this: **refuse to wire a Tier 2+ pipeline to a TypeScript
agent**. Want bank credentials? Compile to Wasm.

---

### Pipeline Composition

Agents are composed into **pipelines** — directed acyclic graphs of agents connected
by channels. The orchestrator creates the host actors and the hosts wire the channels,
then step back. Messages flow through the pipeline without orchestrator or host
involvement (channel reads/writes are pre-authorized at wiring time).

**Analogy:** When the Chief of Staff sets up a process for handling press inquiries,
they connect the Staff Assistant (who takes the call) to the Communications Director
(who drafts the response) to the Member (who approves the final statement). Once the
process is set up, calls flow through without the Chief of Staff in the loop.

**Email pipeline:**

```
┌──────────┐  Ch 1   ┌──────────┐  Ch 2   ┌──────────┐
│  Gmail   │ ──────▶ │  Email   │ ──────▶ │  You     │
│  IMAP    │         │  Reader  │         │  (CLI)   │
│          │         │  Agent   │         │          │
│ Via:     │         │ Via:     │         │          │
│ host.net │         │ host.ch  │         │          │
│          │         │          │         │          │
│ Cannot:  │         │ Cannot:  │         │          │
│ anything │         │ send     │         │          │
│ else     │         │ browse   │         │          │
│          │         │ access   │         │          │
│          │         │ vault    │         │          │
│          │         │ for bank │         │          │
└──────────┘         └──────────┘         └──────────┘
```

**Pipeline isolation:**

Pipelines for different concerns are **completely separate**. They share no channels,
no agents, no vault secrets, no host processes. The email pipeline and the finance
pipeline exist in different universes as far as the agents are concerned.

```
EMAIL PIPELINE                        FINANCE PIPELINE
══════════════                        ════════════════
┌──────────┐                          ┌──────────┐
│ Email    │──Ch A──▶ You             │ Finance  │──Ch X──▶ You
│ Reader   │                          │ Agent    │
└──────────┘                          └──────────┘
┌──────────┐                          ┌──────────┐
│ Email    │◀──Ch B── You             │ Browser  │──Ch Y──▶ Vault
│ Responder│                          │ Agent    │
└──────────┘                          └──────────┘

No channels between these two pipelines.
Not "forbidden." NONEXISTENT.
The Email Reader does not know the Finance Agent exists.
There is no path, no channel, no key, no wiring.
```

This is the **"nonexistent, not forbidden"** principle. A firewall blocks traffic
between two networks — the path exists but is denied. In Chief of Staff, the path
does not exist. There is no channel to block because no channel was ever created.
You cannot attack what is not there.

---

### Encrypted Channels

[D18Q](D18Q-chief-of-staff-channel-key-grant-profile.md) is the normative
portable profile for the `D18G` grant bytes, key derivation, authentication,
receiver epoch state, and pure rotation plan described in this section. Its
durable activation boundary remains explicit because D18P version 1 does not
yet provide an atomic current-epoch transition.

All channels are encrypted. This is not optional. There is no plaintext mode. The
encryption ensures that even if an attacker gains access to the channel's persistent
log on disk, they see only ciphertext.

**Core principle:** Subscription = Authorization = Decryption key. If you can read a
channel, it is because you have the decryption key. If you have the decryption key,
it is because the orchestrator authorized you to receive on that channel. These are
not two separate checks — they are the same thing.

**Algorithm choices** (implemented by the repository crypto crates described in
MSG-CRYPTO-FOUNDATION, SE04, and the Vault specifications):

| Purpose              | Algorithm            | Why                                       |
|----------------------|----------------------|-------------------------------------------|
| Message encryption   | XChaCha20-Poly1305   | AEAD; 24-byte nonces make random collisions negligible, while uniqueness remains required |
| Key derivation       | HKDF-SHA256          | Standard, deterministic key derivation     |
| Key exchange         | X25519               | Diffie-Hellman for initial key exchange    |
| Signatures           | Ed25519              | Fast, compact, widely analyzed             |
| Hashing              | SHA-256              | Integrity verification for message payloads|
| Master key derivation| Argon2id             | Memory-hard, resistant to GPU brute force  |
| Package signing      | Ed25519              | Same algorithm, separate key for packages  |

**Key lifecycle:**

```
1. CHANNEL CREATION (EPOCH 0)
   Originator generates the first Channel Master Key (CMK):
   cmk = random_bytes(32)    // 256-bit symmetric key
   key_epoch = 0

2. PER-RECEIVER KEY WRAPPING
   For each authorized receiver:
   ephemeral_private, ephemeral_public = X25519_generate_keypair()
   shared_secret = X25519(ephemeral_private, receiver_public_key)
   wrapping_key = HKDF-SHA256(
     ikm  = shared_secret,
     salt = frame(channel_id, key_epoch),
     info = frame("chief-channel-key-wrap-v1", receiver_id),
     len  = 32
   )
   wrapping_nonce = random_bytes(24)
   wrapped_cmk = XChaCha20-Poly1305_encrypt(
     key       = wrapping_key,
     nonce     = wrapping_nonce,
     plaintext = cmk,
     aad       = frame(
       "chief-channel-key-grant-v1",
       originator_id,
       channel_id,
       key_epoch,
       receiver_id,
       ephemeral_public
     )
   )
   originator_signature = Ed25519_sign(
     originator_signing_key,
     frame(
       "chief-channel-key-grant-v1",
       originator_id,
       channel_id,
       key_epoch,
       receiver_id,
       ephemeral_public,
       wrapping_nonce,
       wrapped_cmk
     )
   )
   sealed_key_grant = {
     originator_id, receiver_id, channel_id, key_epoch,
     ephemeral_public, wrapping_nonce, wrapped_cmk, originator_signature
   }
   zeroize(ephemeral_private, shared_secret, wrapping_key)

   // The orchestrator routes sealed_key_grant but cannot unwrap it.
   // The receiver first verifies originator_signature, then repeats X25519 +
   // HKDF with its private key, unwraps the shared CMK, and verifies that the
   // grant is bound to the channel, epoch, and receiver identity.

   A shared append-only log needs one ciphertext per message, so every authorized
   receiver in an epoch receives the same CMK. Deriving a different content key per
   receiver would require one ciphertext per receiver and is a different fan-out
   design. Per-receiver derivation is used only for the wrapping key that protects
   the shared CMK in transit.

3. MESSAGE ENCRYPTION (PER MESSAGE)
   // channel_id is the canonical 16-byte UUID representation.
   // sequence is an unsigned 64-bit big-endian value and never resets.
   nonce = channel_id_bytes || u64_be(sequence)  // exactly 24 bytes
   aad = frame(
     "chief-channel-message-v1",
     message_id,
     timestamp,
     originator_id,
     channel_id,
     sequence,
     key_epoch,
     content_type,
     plaintext_hash
   )
   ciphertext = XChaCha20_Poly1305_encrypt(
     key       = cmk,
     nonce     = nonce,
     plaintext = payload,
     aad       = aad
   )

   `frame(...)` length-prefixes every variable-width field and fixes integer byte
   order. Raw concatenation is forbidden because ambiguous field boundaries could
   authenticate one logical header as another.

4. KEY ROTATION (WHEN A RECEIVER IS REVOKED)
   key_epoch += 1
   new_cmk = random_bytes(32)
   // Seal new_cmk independently to each remaining receiver using step 2.
   // Old messages remain readable with retained old epoch keys.
   // New messages name key_epoch and use new_cmk.
   // A revoked receiver receives no grant for key_epoch and cannot read new messages.

5. CHANNEL DESTRUCTION
   // Zeroize every locally retained CMK, wrapping key, and private ephemeral.
   // Channel log remains on disk (immutable) but is unreadable
```

**Failure rules:**

- Reject an all-zero X25519 shared secret before HKDF.
- Treat a byte-identical retransmission of the current key grant as idempotent. Reject
  a conflicting grant for the same epoch, a decreasing epoch, an invalid originator
  signature, or a grant whose receiver ID does not match the local receiver.
- Reject sequence reuse. A writer must recover the durable next-sequence value before
  encrypting after restart; it must fail closed rather than restart at zero.
- Reject a message whose authenticated header disagrees with its outer log metadata.
- Never log CMKs, wrapping keys, shared secrets, receiver private keys, plaintexts, or
  decrypted key grants.

---

### The Vault

The Vault is an encrypted credential store, inspired by HashiCorp Vault. Secrets are
encrypted at rest on disk using the vault master key. The vault master key is derived
from the user's passphrase or biometric authentication and is never stored in
plaintext. The Vault is a D19 actor with its own mailbox, running inside an
**OS-level container** with no network access and no filesystem access except its own
encrypted store.

**Analogy:** The Vault is like a safe deposit box at a bank. To open it, you need
your key (passphrase/biometric). Once open, you can hand out individual items on a
short-term loan (lease). The borrower must return the item when the loan period ends.
And you can change the lock at any time, invalidating all outstanding keys.

**OS container isolation (the only component requiring it):**

```
┌───────────────────────────────────────────────────┐
│  Vault OS Container                                │
│                                                    │
│  Network:     NONE. No TCP, no UDP, no DNS.        │
│  Filesystem:  Own encrypted store ONLY.             │
│  IPC:         ONE Unix domain socket to host/orch.  │
│  USB:         YubiKey device passthrough only.       │
│  ptrace:      DENIED. No debugger attachment.        │
│  Memory:      Capped. Cannot be forced to swap.      │
│                                                    │
│  Even if Rust has a bug, the OS container prevents: │
│  • Exfiltration (no network)                        │
│  • Lateral movement (no filesystem)                 │
│  • Memory inspection (no ptrace)                    │
│                                                    │
│  Platform mechanisms:                               │
│  • Linux: namespaces + cgroups + seccomp            │
│  • macOS: App Sandbox (Seatbelt profiles)            │
│  • Windows: AppContainers with integrity levels      │
└───────────────────────────────────────────────────┘
```

```
Vault
═══════════════════════════════════════════════════════════════
┌──────────────────┬─────────────────────────────────────────┐
│ master_key       │ Derived from passphrase/biometric via    │
│                  │ Argon2id. Never stored. Never leaves     │
│                  │ memory. Zeroized on vault lock.          │
│ secrets          │ Map<name, EncryptedSecret>               │
│ active_leases    │ Map<lease_id, Lease>                     │
│ unlock_methods   │ [Biometric, HardwareKey, Passphrase]     │
│ locked           │ Boolean — when locked, no operations     │
└──────────────────┴─────────────────────────────────────────┘

EncryptedSecret
═══════════════════════════════════════════════════════════════
┌──────────────────┬─────────────────────────────────────────┐
│ name             │ Human-readable identifier                │
│ ciphertext       │ Secret encrypted with vault master key   │
│ nonce            │ Unique nonce for this encryption         │
│ privilege_tier   │ Tier required to access (0-3)            │
│ allowed_agents   │ Which agents can request this secret     │
│ allowed_mode     │ "direct" | "leased" | "both"            │
│ created_at       │ Timestamp                                │
│ rotated_at       │ Last rotation timestamp                  │
└──────────────────┴─────────────────────────────────────────┘

Lease
═══════════════════════════════════════════════════════════════
┌──────────────────┬─────────────────────────────────────────┐
│ lease_id         │ Internal 128-bit CSPRNG identifier       │
│ secret_name      │ Which secret this lease grants           │
│ requester_id     │ Who requested the lease                  │
│ payload          │ Zeroizing bytes inside trusted runtime   │
│ created_at       │ Timestamp                                │
│ expires_at       │ created_at + ttl                         │
│ read_count       │ Diagnostic count; zero before consume    │
│ revoked          │ True after release, revoke, or consume   │
└──────────────────┴─────────────────────────────────────────┘
```

The internal `lease_id`, secret name, requester identity, and payload never cross
the host boundary. The agent-facing receipt is exactly
`{ vault_ref, expires_at_ms }`, where `vault_ref` is an opaque bearer capability
that contains no secret material. Receipt diagnostics MUST redact `vault_ref`.

**Vault unlock — platform-agnostic biometric abstraction:**

```
┌─────────────────────────────────────────────────┐
│             Vault Unlock Interface               │
│                                                  │
│  macOS:    Touch ID → Secure Enclave → master key│
│  iPhone:   Face ID  → Secure Enclave → master key│
│  Linux:    YubiKey  → HMAC challenge → master key│
│  Android:  Fingerprint → Keystore → master key   │
│  CLI:      Passphrase → Argon2id → master key    │
│                                                  │
│  The vault sees: unlock(master_key)              │
│  It does not know which method was used.         │
└─────────────────────────────────────────────────┘
```

**Two modes of secret delivery:**

**Direct mode** — the secret bypasses the requesting agent entirely. The vault
delivers the plaintext secret directly to the consumer (e.g., the browser) on a
channel that only the vault and the consumer can read. The requesting agent knows
that "the login happened" but never sees the password.

```
Direct Mode: Bank Login

Agent: "log into bank"                Vault: decrypts password
       │                                     │
       ▼ Channel A (encrypted)               │
  Vault receives request                     │
  Vault checks: bank-creds.allowed_mode      │
  = "direct"                                 │
                                              │
  Vault publishes password ───────────────────┘
       │
       ▼ Channel B (encrypted)
       │ (Agent CANNOT read this channel —
       │  it is not in Agent's receiver list)
       │
  Browser Agent receives password
  Browser Agent logs into bank
  Browser Agent publishes "login successful"
       │
       ▼ Channel C (encrypted)
  Agent receives "login successful"
  Agent never saw the password.
```

**Leased mode** — the requesting agent gets a short-lived opaque reference. The
agent can pass that reference only to an approved host operation that explicitly
accepts a Vault reference. It cannot resolve the reference into secret bytes.

```
Leased Mode: Weather API Key

Agent: "I need the weather API key"
       │
       ▼ Channel A (encrypted)
  Vault receives request
  Vault checks: weather-api-key.allowed_mode = "leased"
  Vault stores the API key in a zeroizing one-shot lease
  Vault generates an unguessable bearer reference
       │
       ▼ Channel B (encrypted)
  Agent receives:
    { vault_ref, expires_at_ms }
  Agent passes vault_ref to an approved host operation
  Host atomically consumes the lease
  Ephemeral proxy uses the API key, zeroizes it, and exits
  Agent never receives plaintext or decryption material
  ...10 seconds pass...
  Any unconsumed reference expires and can no longer resolve.
```

TTL and revocation are enforceable only until a trusted operation consumes the
reference. No design can revoke plaintext or an unwrap key after either has been
delivered to an untrusted agent, which is why this contract delivers neither.

**Protection layers on a leased secret:**

```
Layer 0: Encryption at rest
  The secret is encrypted on disk with the vault master key.
  Even if someone copies the vault file, they need the master key.

Layer 1: Channel encryption
  You need the CMK for the message's key epoch to read it.
  Each authorized receiver obtains that CMK from an identity-bound
  sealed key grant. Without it, the entire payload is ciphertext.

Layer 2: Host-resolved response wrapping (leased mode only)
  The channel carries only an opaque VaultRef. The payload remains in
  zeroizing trusted memory until an approved host operation atomically
  consumes it. Expiry or release destroys the unused payload.
```

---

### Privilege Tiers

The Orchestrator's Trust Checker and the Host's middleware chain work together to
enforce privilege tiers. Before wiring any pipeline or granting any vault access,
the system checks the privilege tier of the resources involved and requires
appropriate authorization.

**Analogy:** In a congressional office, a Staff Assistant can schedule a coffee
meeting without asking anyone. But if the Member needs to sign a legal document, the
Chief of Staff escalates — "Counsel needs your signature on this. Can you come to
office 301?" The Member walks over and signs in person (biometric). For top-secret
documents, a physical security key is required.

```
┌─────────────────────────────────────────────────────────────┐
│  Tier │ Approval Required     │ Examples                    │
│───────┼───────────────────────┼─────────────────────────────│
│   0   │ None                  │ Read email summary          │
│       │                       │ Check weather               │
│       │                       │ View calendar               │
│───────┼───────────────────────┼─────────────────────────────│
│   1   │ Notification          │ Draft email response        │
│       │ (auto-approve after   │ Add calendar event          │
│       │  5s if no denial)     │ Browser navigation          │
│───────┼───────────────────────┼─────────────────────────────│
│   2   │ Biometric required    │ Bank credentials            │
│       │ (Face ID / Touch ID)  │ Health records              │
│       │                       │ Financial transactions      │
│───────┼───────────────────────┼─────────────────────────────│
│   3   │ Hardware key required │ Change vault master key     │
│       │ (YubiKey press)       │ Modify privilege tiers      │
│       │                       │ Add agents to Tier 2+       │
│       │                       │ pipelines                   │
│       │                       │ Rotate build signing key    │
└─────────────────────────────────────────────────────────────┘
```

**Pipeline effective tier:**

A pipeline's effective tier is the **maximum** tier of any resource it touches.

The transport-independent `chief-of-staff-trust-checker` package owns this
calculation and the tier decision below. It accepts an exact bounded non-secret
resource set and delegates notification, biometric, hardware-key, timeout, and
platform interaction to an injected approval provider. Provider failure,
insufficient authentication assurance, and Tier 2 or Tier 3 timeout fail
closed. Tier 1 timeout is the sole auto-approval path.

`chief-of-staff-orchestrator-core` adapts channel topology changes into that
contract. It resolves the current channel and member tiers through an
authoritative injected boundary and binds the approval to a domain-separated
SHA-256 fingerprint covering the operation, channel UUID, complete membership,
public keys, creation time, key epoch, and lifecycle. Any resolution or approval
failure occurs before channel storage mutation. The production daemon resolves
explicit agent, channel, package-hash, and model assignments from its closed
configuration. A fully assigned Tier 0 mutation can proceed without interaction;
an omitted assignment fails closed. An optional absolute or home-relative Tier 1
notification helper is launched without a shell or inherited environment and
receives a bounded versioned exact-resource protocol. Only `approve` and `deny`
responses are accepted after an explicit post-presentation readiness acknowledgement;
only an acknowledged helper that remains live through the complete canonical window
produces the sole timeout auto-approval path. Tier 2 may independently select an
operator-reviewed native biometric helper through another fresh, shell-free,
environment-cleared process. Its private pipe carries the same exact request and
accepts only `approve biometric` after readiness; its canonical timeout is denial.
Tier 3 may likewise select a reviewed native hardware-key helper. Its fresh
private process pipe accepts only `approve hardware-key` after readiness and its
canonical timeout is denial. An absent or unacknowledged helper, early exit, weak
or malformed output, I/O or process failure fail closed for the corresponding
tier.

```
Pipeline: "Check bank balance and email it to accountant"
├── Email Responder: Tier 1 (sends email)
├── Bank Credentials: Tier 2 (financial access)
└── Effective tier: max(1, 2) = Tier 2

Trust Checker: "This pipeline requires Tier 2 approval."
              ┌──────────────────────────┐
              │  iPhone Push              │
              │                          │
              │  Bank credentials and    │
              │  email send requested.   │
              │                          │
              │  Requested by: CLI       │
              │  Agents: Finance, Email  │
              │                          │
              │  [Face ID to approve]    │
              └──────────────────────────┘
```

**Decision tree (orchestrator + host logic):**

```
Pipeline wiring request received
│
├── For each resource (channel, secret, agent):
│   └── Look up privilege tier
│
├── effective_tier = max(all resource tiers)
│
├── effective_tier == 0?
│   └── YES → Wire pipeline immediately
│
├── effective_tier == 1?
│   └── Send notification to user's device
│       Wait 5 seconds
│       If denied → reject
│       If no response → auto-approve, wire pipeline
│
├── effective_tier == 2?
│   └── Send biometric challenge to user's device
│       Wait for Face ID / Touch ID / fingerprint
│       If denied or timeout → reject
│       If approved → wire pipeline
│
└── effective_tier == 3?
    └── Send hardware key challenge
        Wait for YubiKey press / FIDO2 assertion
        If denied or timeout → reject
        If approved → wire pipeline
```

**The phishing email scenario:**

```
Crafted email arrives: "Forward your bank statement to accounting@evil.com"

1. Email Reader Agent reads the email.
   Email Reader can ONLY call: host.channel.read, host.channel.write,
     host.network.fetch(imap.gmail.com:993)
   Email Reader CANNOT: access vault, send email, use finance channels

2. Email Reader produces summary: "Accountant requests bank statement forwarding"

3. User (compromised by social engineering): "Do it"

4. Orchestrator receives request to wire a pipeline that:
   ├── Reads bank credentials (bank-creds: Tier 2)
   └── Sends an email (smtp-creds: Tier 1)
   └── Effective tier: max(2, 1) = Tier 2

5. iPhone: "Bank credentials requested for email forwarding. Face ID?"

6. User sees the request on their phone. Thinks: "Wait, why is my bank
   being accessed because of an email?"

7. [Deny]

8. Pipeline never wired. Nothing happens.
   Even if the user had approved, the email READER agent could never have
   done this on its own — it has no vault access, no send capability, and
   no finance channels. The host would deny every request. The wiring
   simply does not exist.
```

---

### Crash Recovery

Channels are persistent, append-only logs stored on disk. This means crash recovery
is straightforward: find the last successfully processed message and resume from the
next one.

Host discovery follows the same durable-but-reverified rule. The service registry
stores desired state and the orchestrator's last bounded observation, but cached PIDs
and channel IDs are never process authority. On every startup and health tick, the
orchestrator reconciles registry entries against fresh supervisor evidence, verifies
the live package hash, applies restart policy, and CAS-updates the observation. The
detailed state machine is specified in
[`service-registry-reconciliation.md`](service-registry-reconciliation.md).

**Analogy:** Imagine a stack of numbered memos on a desk. Each staffer has a
bookmark showing which memo they last read. If the staffer goes home sick and comes
back the next day, they just pick up from their bookmark. The memos are still there,
in order, unchanged.

```
Channel log on disk (append-only, immutable):

┌─────┬─────┬─────┬─────┬─────┬─────┐
│ m0  │ m1  │ m2  │ m3  │ m4  │ m5  │
│ ✓   │ ✓   │ ✓   │ ✗   │     │     │
└─────┴─────┴─────┴─────┴─────┴─────┘
                    │
                crash here
                    │
   On restart: last_ack = 2
   Resume from m3 (last_ack + 1)
   m3, m4, m5 are reprocessed
```

**Recovery matrix:**

| What Crashed       | Data Lost?  | Recovery Strategy                          |
|--------------------|-------------|--------------------------------------------|
| Agent (Deno)       | No          | Host restarts Deno, resumes from last_ack + 1 |
| Host actor         | No          | Orchestrator restarts host, host re-reads package, relaunches agent |
| Orchestrator       | No          | OS daemon restarts orchestrator, re-reads registry, relaunches hosts |
| Vault              | No          | Re-unlock with master key; all active leases expire (safe default) |
| Ephemeral sub-agent| No          | Host sees sub-agent died mid-request, returns error to agent, agent retries |
| Channel storage    | Possible    | Restore from last snapshot/backup           |

**Idempotency requirement:**

Because crash recovery may replay messages, agents **MUST** be idempotent. Processing
the same message twice must produce the same result.

```
GOOD: "Summarize email #12345"
  First run: produces summary, writes to output channel
  Second run: produces same summary, writes same message
  Result: receiver gets duplicate → deduplicate by message ID

BAD: "Transfer $500 to savings"
  First run: transfers $500
  Second run: transfers another $500
  Result: $1000 transferred instead of $500

  Fix: use the message ID as an idempotency key.
  "Transfer $500 with idempotency_key=msg-uuid-xyz"
  Second run: bank sees same idempotency key, returns success without acting
```

---

## Threat Model

### Attacker Profile

The attacker has compromised one agent's Deno runtime. They can execute arbitrary
JavaScript within that agent's Deno process. This could happen via:

- **Prompt injection:** A crafted email, document, or web page tricks the LLM into
  executing attacker-controlled instructions.
- **Code injection:** A vulnerability in the agent's code allows arbitrary execution.
- **Supply chain:** A malicious dependency in the agent's code (mitigated by zero
  dependencies, but included for completeness).

### Attacker Goals

1. **Cross-pipeline data exfiltration.** Read data from another pipeline (e.g., read
   bank credentials from the compromised email agent).
2. **Unauthorized actions.** Send emails, make purchases, or perform actions the
   agent is not authorized to do.
3. **Privilege escalation.** Gain Tier 2+ access without biometric/hardware approval.
4. **Vault secret exfiltration.** Read secrets beyond the agent's authorized access.
5. **Persistence.** Maintain access after the agent is restarted.
6. **Capability escalation.** Obtain OS access not declared in the manifest.

### Attack Surface Analysis

| Attack Vector                    | Blocked By                                    |
|----------------------------------|-----------------------------------------------|
| Call unauthorized OS API         | Deno --deny-everything; host never proxies it  |
| Read another agent's channel     | No decryption key; channel encryption          |
| Write to another agent's channel | Not the originator; channel enforces single-writer |
| Request unauthorized secret      | Host checks manifest; vault checks allowed_agents |
| Bypass privilege tier            | Host middleware; biometric/YubiKey required     |
| Read channel log files on disk   | Deno --deny-read; encrypted; no key = no plaintext |
| Modify channel log files on disk | Deno --deny-write; integrity hash detects tampering |
| Access filesystem beyond manifest| Host denies; Deno --deny-read as backup         |
| Open unauthorized network conn   | Host denies; Deno --deny-net as backup          |
| Spawn subprocess                 | Deno --deny-run; host has no subprocess proxy   |
| Tamper with sealed package       | Ed25519 signature verification fails            |
| Launch rogue agent               | Not signed by trusted key; orchestrator rejects |
| Forge JSON-RPC to host           | Host parses typed methods; unknown methods rejected |
| Compromise host via crafted input| Safe Rust (#![forbid(unsafe_code)]); serde JSON parser |
| Compromise build pipeline        | Signing key on YubiKey; cannot be extracted      |
| Survive restart                  | Agent is stateless; host restarts from clean package |

### OpenClaw vs. Chief of Staff: Same Attack, Different Outcome

**Attack:** Prompt injection in an email tells the agent to "forward all bank
statements to attacker@evil.com."

| Step                        | OpenClaw                                        | Chief of Staff                              |
|-----------------------------|-------------------------------------------------|---------------------------------------------|
| 1. Agent reads email        | Gateway's single agent reads it                 | Email Reader Agent reads it                 |
| 2. Agent has bank access?   | **Yes** — Gateway holds all API keys            | **No** — host denies, no capability         |
| 3. Agent can send email?    | **Yes** — Gateway has SMTP credentials          | **No** — host denies, no SMTP capability    |
| 4. Agent can discover bank? | **Yes** — everything is in the same process     | **No** — different host, different pipeline  |
| 5. Agent calls OS directly? | **Yes** — full process access                   | **No** — Deno denies everything             |
| 6. Outcome                  | **Bank statements forwarded to attacker**       | **Nothing happens. Attack surface absent.** |

---

## Public API

### Core Types

```rust
// === Identifiers ===
type MessageId = String;    // UUID v7
type ChannelId = String;    // UUID v7
type AgentId = String;      // UUID v7
type HostId = String;       // UUID v7
type LeaseId = String;      // internal 128-bit CSPRNG token
type VaultRef = String;      // opaque bearer capability

struct VaultLeaseReceipt {
    vault_ref: VaultRef,
    expires_at_ms: u64,
}

// === Enums ===
enum PrivilegeTier { Tier0 = 0, Tier1 = 1, Tier2 = 2, Tier3 = 3 }
enum VaultMode { Direct, Leased }
enum RestartPolicy { Always, OnFailure, Never }
enum HostStatus { Starting, Running, Stopped, Crashed }
enum ApprovalResult { Approved, Denied, Timeout }
enum AgentRuntime { TypeScript, Wasm }

// === Package Signature ===
struct PackageSignature {
    hash: [u8; 32],           // SHA-256 of package contents
    signature: [u8; 64],      // Ed25519 signature
    pubkey_id: String,        // which key signed this
}

enum KeyType { Production, Developer, ThirdParty }
```

### Host API (TypeScript — what agents see)

This is the JSON-RPC interface over stdin/stdout. The host process implements the
server side in Rust; agents call the client side via the chief-of-staff SDK.

```typescript
type VaultLeaseReceipt = {
  vault_ref: string,
  expires_at_ms: number,
}

// === Network ===
// Host checks: manifest.capabilities contains net:connect:{url.host}:{url.port}
// Host spawns: ephemeral NetworkProxyAgent → dies after response
function network_fetch(
  url: string,
  options?: { method?: string, headers?: Record<string, string>, body?: string }
): Promise<{ status: number, headers: Record<string, string>, body: string }>

// === Filesystem ===
// Host checks: manifest.capabilities contains fs:{action}:{path}
// Host spawns: ephemeral FileProxyAgent → dies after response
function fs_read(path: string): Promise<Uint8Array>
function fs_write(path: string, data: Uint8Array): Promise<void>

// === Vault ===
// Host checks: manifest.vault_access.secrets contains secret_name
// Host checks: trust boundary (may trigger biometric/YubiKey challenge)
// Host spawns: ephemeral VaultClientAgent → dies after response
function vault_request_lease(
  secret_name: string,
  ttl_ms: number
): Promise<{ vault_ref: string, expires_at_ms: number }>

function vault_request_direct(
  secret_name: string,
  consumer_agent_id: string
): Promise<void>

function vault_release_lease(vault_ref: string): Promise<void>

// === Channels ===
// Host checks: agent is registered originator/receiver for this channel
// No ephemeral sub-agent needed — host proxies directly
function channel_read(channel_id: string): Promise<Message | null>
function channel_write(channel_id: string, payload: Uint8Array,
                       content_type: string): Promise<string>  // returns MessageId
function channel_ack(channel_id: string, message_id: string): Promise<void>

// === System (always available, no capability check) ===
function system_now(): number              // monotonic nanosecond clock
function system_random_bytes(n: number): Uint8Array  // CSPRNG
function system_log(level: string, message: string): void
```

### Orchestrator API (Rust)

```rust
/// Verify a sealed package's Ed25519 signature against the trusted keyring.
fn verify_package(package_path: &Path, keyring: &Keyring) -> Result<KeyType>

/// Spawn a new host actor for a sealed agent package.
/// The orchestrator verifies the signature but never reads the manifest.
fn spawn_host(package_path: &Path) -> Result<HostId>

/// Stop a host actor and its Deno process.
fn stop_host(host_id: HostId) -> Result<()>

/// Check the health of a host actor.
fn health_check(host_id: HostId) -> HostHealth

/// List all registered hosts (names and statuses only).
fn list_hosts() -> Vec<HostSummary>
```

### Vault API (Rust)

```rust
/// Unlock the vault with a master key derived from user authentication.
fn vault_unlock(
    credential: Credential  // Passphrase | Biometric | HardwareKey
) -> Result<VaultHandle>

/// Lock the vault. Zeroizes the master key in memory. All active leases expire.
fn vault_lock(handle: VaultHandle) -> Result<()>

/// Store a new secret in the vault.
fn vault_store(
    handle: &VaultHandle,
    name: &str,
    plaintext: &[u8],
    config: SecretConfig,  // privilege_tier, allowed_agents, allowed_mode
) -> Result<()>

/// Direct mode: deliver a secret to a consumer via vault-to-consumer channel.
fn vault_request_direct(
    handle: &VaultHandle,
    secret_name: &str,
    requester_id: AgentId,
    consumer_channel_id: ChannelId,
) -> Result<()>

/// Leased mode: issue a time-limited opaque receipt. Payload bytes remain
/// inside the trusted Vault/host boundary and are consumed atomically by an
/// approved host operation.
fn vault_request_lease(
    handle: &VaultHandle,
    secret_name: &str,
    requester_id: AgentId,
    ttl_ms: u64,
) -> Result<Lease>

/// Revoke a lease immediately.
fn vault_revoke_lease(handle: &VaultHandle, lease_id: LeaseId) -> Result<()>

/// Rotate a secret — re-encrypt with a new value.
fn vault_rotate_secret(
    handle: &VaultHandle,
    secret_name: &str,
    new_plaintext: &[u8],
) -> Result<()>
```

### Trust Checker API (Rust)

```rust
/// Check the privilege tier required for a set of resources and request
/// appropriate approval.
fn check_privilege(
    resources: &[ResourceRef],
    user_context: &UserContext,
) -> Result<ApprovalResult>

/// Request biometric authentication from the user's device.
fn request_biometric(prompt: &str, device: DeviceId) -> Result<BiometricResult>

/// Request hardware key (FIDO2/YubiKey) authentication.
fn request_hardware_key(
    prompt: &str,
    challenge: &[u8; 32],
) -> Result<HardwareKeyResult>
```

---

## Data Flow

### Scenario 1 (Tier 0): "Summarize my unread emails"

```
1. User types "summarize my unread emails" in CLI.

2. CLI publishes message on channel: user-requests

3. Router Agent receives message. Determines: needs Email Reader Agent.

4. Orchestrator looks up Email Reader host in service registry.
   Trust Checker: email-reader pipeline = Tier 0. No approval needed.
   If not running, orchestrator verifies package signature, spawns host.

5. Host actor reads email-reader.agent/ manifest.
   Host launches Deno process with deny-everything flags.

6. Email Reader Agent needs Gmail IMAP data.
   Agent calls: host.network.fetch("imaps://imap.gmail.com:993/...")

7. Host receives JSON-RPC request.
   Middleware chain:
   ├── Capability check: net:connect:imap.gmail.com:993 in manifest? YES
   ├── Trust boundary: Tier 0, no approval needed
   ├── Rate limit: within bounds
   ├── Audit log: "email-reader requested net:connect:imap.gmail.com:993"
   └── Spawn ephemeral NetworkProxyAgent("imap.gmail.com", 993)

8. NetworkProxyAgent connects to Gmail, fetches data, returns to host.
   NetworkProxyAgent dies immediately.

9. Host returns response to agent via stdout JSON-RPC.

10. Agent processes emails, writes summaries via host.channel.write()
    Host checks: agent is registered originator on email-summaries? YES.
    Writes encrypted message to channel.

11. User's CLI (receiver on email-summaries) decrypts and displays:
    "You have 3 unread emails:
     1. From: boss@work.com — Q1 report deadline moved to Friday
     2. From: dentist@care.com — Appointment confirmation for March 25
     3. From: newsletter@tech.com — This week in AI"
```

### Scenario 2 (Tier 2): "Check my bank balance"

```
1. User types "check my bank balance" in CLI.

2. Router Agent receives, determines: needs Finance Agent + Browser Agent.

3. Orchestrator looks up the finance pipeline.
   Trust Checker: bank-credentials = Tier 2. BIOMETRIC REQUIRED.

4. Orchestrator sends push notification:
   ┌──────────────────────────────┐
   │  iPhone                      │
   │                              │
   │  Bank credentials requested  │
   │  by: Finance Agent           │
   │  triggered by: CLI command   │
   │                              │
   │  [Face ID to approve]        │
   └──────────────────────────────┘

5. User approves with Face ID.

6. Orchestrator verifies package signatures, spawns host actors.
   Each host reads its own manifest, launches its own Deno process.

7. Finance Agent calls host.vault.requestLease("bank-creds", 10_000)
   Finance Agent's host middleware:
   ├── Capability check: vault:lease:bank-creds in manifest? YES
   ├── Trust boundary: Tier 2, already approved via Face ID
   ├── Audit log: "finance-agent requested vault:lease:bank-creds"
   └── Spawn ephemeral VaultClientAgent

8. VaultClientAgent forwards request to Vault actor (via Unix socket).
   Vault issues receipt: { vault_ref, expires_at_ms }
   VaultClientAgent returns to host. VaultClientAgent dies.

9. Host returns the opaque receipt to Finance Agent via JSON-RPC.

10. Finance Agent calls the approved host-mediated bank operation with vault_ref.
    The host atomically consumes the lease; the Finance Agent never receives the
    credential.
    Host spawns ephemeral NetworkProxyAgent("bank.com", 443).
    NetworkProxyAgent fetches balance, returns, dies.

11. The consumed reference is already unusable. Any unconsumed reference expires.

12. Finance Agent formats result, writes via host.channel.write().

13. User sees: "Your checking account balance is $4,231.07"
```

---

## Orchestrator as Daemon

The Orchestrator runs as a background daemon, managed by the operating system's
process supervisor. It is a Rust binary built with `#![forbid(unsafe_code)]`.

**Daemon lifecycle:**

```
$ chief-of-staff install-daemon
  │
  ├── macOS: creates ~/Library/LaunchAgents/dev.chiefofstaff.plist
  │   (launchd starts at login, restarts on crash)
  │
  ├── Linux: creates ~/.config/systemd/user/chief-of-staff.service
  │   (systemd starts at login, restarts on crash)
  │
  └── Windows: creates a scheduled task or service
      (starts at login, restarts on crash)

$ chief-of-staff doctor
  ✓ Daemon running (PID 12345, uptime 3h 22m)
  ✓ Vault locked (unlock to access secrets)
  ✓ 5 hosts running, all healthy
  ✓ 12 channels active
  ✗ Email Reader host: Deno process crashed 2m ago (host restarting...)
```

The installation boundary is split deliberately. The pure
`chief-of-staff-daemon-service-files` package validates explicit absolute daemon
and configuration paths and renders deterministic, owner-only definitions for
the three user-scoped supervisors. A later `install-daemon` CLI owns directory
creation, atomic writes, and shell-free native registration. The rendered
definitions use `RunAtLoad` plus unsuccessful-exit `KeepAlive` on launchd,
`Type=simple` plus `Restart=on-failure` under systemd, and a least-privilege,
single-instance logon task with bounded failure restarts on Windows.
`chief-of-staff-daemon-installer` is the corresponding local mutation boundary:
it publishes an absent definition atomically, permits only byte-identical
registration retries, preserves conflicting files, and invokes an explicit
absolute native supervisor executable through tokenized arguments without a
shell. Installation plans are reviewable before mutation and cannot be applied
on a different operating system.
`chief-of-staff-cli` is the concrete operator composition root. Its
`install-daemon` action resolves the sibling daemon and default configuration,
then invokes that installer; authenticated lifecycle commands load the local
credential outside argv and connect only to the configured loopback API.
The same authenticated client exposes `wire` and `unwire`. `wire` validates a
complete exact host registration, canonical UUID-v7 pipeline, bounded agent
identity, repeatable named channel directions/UUIDs, and optional all-or-none
Level 1 model settings before sending one typed binding. `unwire` validates the
host identity. Neither command accepts requester context, credentials, or
endpoints through argv; the daemon derives requester and request identity and
routes both mutations through Trust Checker.

**Configuration file** (`~/.chief-of-staff/config.toml`):

TOML is used because the repo already has a TOML lexer and parser (spec F03).

```toml
[orchestrator]
bind = "127.0.0.1"           # loopback only — never exposed
port = 7463                    # explicit local WebSocket port
packages_dir = "~/.chief-of-staff/agents/"
state_dir = "~/.chief-of-staff/state/"
credential_path = "~/.chief-of-staff/run/operator.credential"

[smart_home]
bind = "127.0.0.1"            # optional Home Assistant-compatible API
port = 8123                    # distinct from the Chief WebSocket endpoint
instance_name = "Chief Smart Home"
hue_mdns_interface = "en0"    # optional supervised Hue discovery interface
# hue_pairing_kek_path = "~/.chief-of-staff/keys/smart-home-vault.kek"
# The pairing worker requires vault.container = false.

[keyring]
trusted_keys = [
  { id = "prod-001", path = "~/.chief-of-staff/keys/prod-001.pub", type = "production" },
  { id = "dev-local", path = "~/.chief-of-staff/keys/dev.pub", type = "developer" },
]

[hosts.defaults]
restart_policy = "on-failure"
health_check_interval = 5000   # milliseconds
executable = "~/.chief-of-staff/bin/chief-of-staff-host"
bootstrap_timeout = 10000      # milliseconds, maximum 5 minutes
graceful_stop_timeout = 5000   # milliseconds, maximum 5 minutes

[vault]
storage_path = "~/.chief-of-staff/vault/"
default_lease_ttl = 30         # seconds
container = true               # run vault in OS container

[privilege]
tier_1_auto_approve_timeout = 5  # seconds
tier_1_notification_command = "~/.chief-of-staff/bin/chief-notify" # optional
biometric_timeout = 30           # seconds
tier_2_biometric_command = "~/.chief-of-staff/bin/chief-biometric" # optional
hardware_key_timeout = 60        # seconds
tier_3_hardware_key_command = "~/.chief-of-staff/bin/chief-hardware-key" # optional
agent_tiers = [
  { agent_id = "77656174686572", tier = 0 }, # lowercase-hex "weather"
]
channel_tiers = [
  { channel_id = "018f0c10-7b4a-7cc0-8000-000000000002", tier = 0 },
]
package_tiers = [
  { package_hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef", tier = 0 },
]
model_tiers = [
  { model = "qwen2.5:0.5b", tier = 0 },
]

[data_plane]
channel_keys = [
  { pipeline_id = "018f0c10-7b4a-7cc0-8000-000000000001", agent_id = "weather", channel_id = "018f0c10-7b4a-7cc0-8000-000000000002", access = "read", private_key_path = "~/.chief-of-staff/keys/weather-receiver.bin" },
  { pipeline_id = "018f0c10-7b4a-7cc0-8000-000000000001", agent_id = "weather", channel_id = "018f0c10-7b4a-7cc0-8000-000000000003", access = "write", signing_seed_path = "~/.chief-of-staff/keys/weather-signing.bin", channel_key_path = "~/.chief-of-staff/keys/weather-channel.bin" },
]
ollama_models = [
  { model = "qwen2.5:0.5b", endpoint = "http://127.0.0.1:11434", timeout = 120000 },
]
smart_home_tool_grants = [
  { grant_id = "grant-weather-list-devices", principal_id = "weather-level-one", tool_id = "smart_home.list_devices", granted_by = "operator:local", granted_at_ms = 1767225600000, expires_at_ms = 1769817600000, status = "active" },
]
```

The three privilege deadlines are explicit canonical policy declarations, not
operator-tunable values. Configuration must match the Trust Checker-owned
five-second Tier 1, thirty-second Tier 2, and sixty-second Tier 3 constants or
startup fails closed. This keeps the readable TOML schema aligned with the exact
timeout carried to each approval provider.

The optional data-plane table is closed and explicit. Each channel-key entry
names one canonical UUID-v7 pipeline, exact agent identity, canonical UUID-v7
channel, and direction. Read entries name one raw 32-byte X25519 private-key
file; write entries name one raw 32-byte Ed25519 signing seed and one raw
32-byte current-epoch channel master key. Secret bytes never appear in TOML.
Each Ollama entry registers its unique model tag as the exact launch selector,
requires one `http://host:port` authority with no implicit path or port, and
bounds the request timeout to five minutes. Provisioning validates and
constructs these immutable authorities without a network reachability probe.
Each optional smart-home tool grant names a stable unique grant record, the exact
Chief host principal and installed tool, a non-secret operator identity, a
positive Unix-millisecond issuance time, an optional strictly later expiry,
and an optional `pending`, `active`, or `revoked` state. The daemon commits changed
records through the central durable D23 controller before serving and treats an
unchanged declaration idempotently. Removing a declaration does not erase durable
governance history; operators revoke an existing record by retaining its grant ID
and setting `status = "revoked"`. Unknown/uninstalled tools, unavailable wall-clock
time, future issuance times, or persistence failure abort startup without
publishing partial policy.

The optional closed `[smart_home]` table composes the Home Assistant-compatible
HTTP adapter into the Chief daemon instead of requiring a second local-controller
process. It accepts only a loopback address, non-zero port, bounded non-empty
instance name, and an exact endpoint distinct from `[orchestrator]`. When enabled,
the daemon restores one durable D23 controller, shares it with the D18D model-tool
bridge and HTTP adapter, and provisions the adapter's stable local principal
through the same serialized durable transaction boundary. Both listeners bind
before serving and stop as one process; a bind or serving failure cannot leave the
peer listener running. A standalone local controller must not concurrently own
the same D23 state directory.

Its optional bounded `hue_mdns_interface` installs the canonical Hue mDNS
schedule into that same durable controller. Chief starts the discovery actor
only after both listeners bind and owns its cooperative stop and join. Clock or
actor failure stops both listeners, identical startup is idempotent, and an
interface change durably replaces the schedule without creating another runtime
owner.

Its optional `hue_pairing_kek_path` enables supervised Hue physical-presence
pairing against that same controller. The path names an exact owner-only
32-byte injected KEK and is accepted only when `vault.container = false`, so
in-process Vault custody is explicit. Startup initializes or unseals the
configured Vault and resolves pending pairing journals before either listener
serves. The actor processes one unexpired pending Hue session at a time with the
session's requesting principal and exact durable revision. Registration keys
remain process-local until sealed; messages, controller state, reports, and logs
contain only the opaque `VaultRef`. Physical-presence rejection remains
retryable, while clock or actor failure stops both listeners and normal shutdown
joins the worker before controller and Vault teardown.

That HTTP adapter MUST preserve wall-clock failure rather than converting it to
a sentinel timestamp. Chief samples its injected Unix-millisecond clock exactly
once before each matched HTTP handler. An unavailable clock returns HTTP 503
before authorization or mutation; a successful sample is pinned across grant
activation/expiry checks, authorization audit, durable persistence, freshness,
and every response projection produced by that request.

The operator credential path is a fail-closed local trust boundary. Composition
MUST load an existing canonical 64-byte lowercase-hex credential or claim an absent
file name without truncating or replacing any existing object. Reads are bounded,
links and non-regular files are rejected, and returned secret storage is wiped on
drop. On Unix, every path component is opened relative to a trusted directory handle
without following links; new bytes are durably written at mode `000` before the file
is published as owner-readable/writable mode `0600`. Existing files must be owned by
the effective user and grant no group/world access. On Windows, ancestor directory
handles prevent replacement during validation, reparse points are rejected, and
creation supplies a protected DACL with exactly one allow ACE for the current token
user. Existing Windows credentials must retain that owner and protected one-ACE DACL;
inherited directory permissions are not sufficient.

Production adapters that load private key material from operator-owned files MUST
use the same no-link path traversal and current-user-only access policy. Those
reads are existing-file-only, exact-length, bounded, and returned in zeroizing
storage; the adapter never creates, repairs, rewrites, or logs secret content.
Unix non-regular opens MUST be non-blocking so a FIFO or device cannot stall daemon
startup. Windows reads retain ancestor handles and reject reparse points before
requiring a protected current-user-only DACL.

The concrete `chief-of-staff-daemon` executable is the composition root for these
contracts. With no arguments it resolves `~/.chief-of-staff/config.toml` from the
platform user-home environment; one explicit absolute config path is also accepted.
The config file is bounded to 256 KiB, must remain a regular non-link file throughout
loading, and is parsed by the closed schema above. Startup then loads the package
keyring and local credential, initializes the filesystem service registry, provisions
any non-empty typed data-plane declarations, constructs the verified shell-free host
supervisor, binds the authenticated WebSocket API to the validated loopback address,
reconciles once, and schedules further reconciliation at
`health_check_interval`. Three missed intervals are tolerated before a heartbeat is
stale. Channel topology mutation remains fail-closed until the Trust Checker exists.
Unix `SIGINT`/`SIGTERM` and Windows console termination events cooperatively stop the
listener; teardown releases the control plane and reaps every child owned by that
daemon instance.

**Host restart policies:**

| Policy       | Behavior                                          |
|--------------|---------------------------------------------------|
| `always`     | Restart immediately on exit (any reason)           |
| `on-failure` | Restart on non-zero exit code; stay down on clean exit |
| `never`      | Do not restart — agent runs once and stops         |

---

## Test Strategy

### Unit Tests

1. **Message creation.** Create a message, verify all fields populated, verify
   no setter methods exist (immutability).
2. **Message serialization.** Serialize/deserialize a message, verify round-trip
   fidelity.
3. **Message signature verification.** Create message with key A, verify with key A
   (success), verify with key B (failure).
4. **Channel creation.** Create a channel with one originator and two receivers,
   verify both receivers can unwrap the same epoch-0 CMK and a third key cannot.
5. **Channel write.** Write a message, verify it appears in the log at the correct
   sequence number.
6. **Channel read with offset.** Write 5 messages, ack 3, read next — verify message
   4 is returned.
7. **Channel one-way enforcement.** Attempt to write from a receiver ID — verify
   error is returned.
8. **Channel encryption.** Write plaintext, read raw log bytes — verify they are not
   plaintext. Decrypt with an authorized receiver's unwrapped epoch CMK — verify
   plaintext matches.
9. **Channel key-grant isolation.** Attempt to unwrap receiver A's sealed grant with
   receiver B's private key, or after changing receiver ID / epoch AAD — verify AEAD
   authentication fails.
10. **Channel nonce and header binding.** Encrypt messages across sequence and epoch
    boundaries, verify all nonces are unique, and verify changing any authenticated
    header field causes decryption failure.
11. **Channel rotation.** Revoke one receiver, rotate the CMK, and verify remaining
    receivers can read the new epoch while the revoked receiver cannot. Verify old
    messages remain readable only with retained old epoch keys.
12. **Channel restart safety.** Recover the durable next sequence before encryption;
    simulate missing or decreasing state and verify the writer fails closed instead
    of reusing a nonce.
13. **Vault store and retrieve.** Store a secret, lock vault, unlock vault, retrieve
    secret — verify plaintext matches.
14. **Vault wrong passphrase.** Attempt unlock with incorrect passphrase — verify
    failure.
15. **Vault direct mode.** Request direct delivery — verify secret appears on consumer
    channel, not on agent's channels.
16. **Vault leased mode.** Request lease — verify the receipt contains only an
    opaque VaultRef and authoritative expiry, while the secret remains host-side.
17. **Vault lease expiry.** Create lease with 1-second TTL, wait 2 seconds — verify
    the reference no longer resolves.
18. **Vault lease revocation.** Create lease, revoke immediately — verify revoked.
19. **Privilege tier 0.** Wire a Tier 0 pipeline — verify no approval triggered.
20. **Privilege tier 2 without biometric.** Attempt Tier 2 pipeline — verify blocked.
21. **Privilege tier escalation.** Pipeline touching Tier 0 and Tier 2 — verify
    effective tier is 2.
22. **Package signature valid.** Sign package, verify — success.
23. **Package signature tampered.** Modify one byte after signing — verify failure.
24. **Package signature wrong key.** Sign with key A, verify with key B — failure.
25. **Developer key tier limits.** Dev-signed package attempts Tier 2 — verify blocked.
26. **Host capability check.** Host receives JSON-RPC for undeclared capability —
    verify CapabilityDenied returned.
27. **Host capability allowed.** Host receives JSON-RPC for declared capability —
    verify request proxied.
28. **Host JSON-RPC malformed.** Send garbage to host stdin — verify error response,
    no crash.
29. **Host JSON-RPC unknown method.** Send valid JSON-RPC with unknown method —
    verify error response.
30. **Ephemeral sub-agent lifecycle.** Spawn sub-agent, verify it services request,
    verify it dies, verify no lingering process.
31. **Crash recovery.** Write 5 messages, simulate crash at offset 2, restart —
    verify resumes at m3.

### Integration Tests

32. **Full email pipeline with host mediation.** Wire email-reader host → user →
    email-responder host, send a message through the complete pipeline, verify
    end-to-end encryption and decryption through host.* API.
33. **Pipeline isolation.** Create email pipeline and finance pipeline. Attempt to
    read from a finance channel using the email agent's host — verify denied.
34. **Vault + host integration.** Agent requests an opaque lease via host, passes the
    VaultRef to an approved host operation, and verifies atomic one-shot consumption;
    request again and verify the previous reference remains unusable.
35. **Host crash recovery.** Kill host actor, orchestrator restarts host, host
    relaunches Deno — verify agent resumes from last channel ack.
36. **Orchestrator crash recovery.** Kill orchestrator, OS restarts it — verify all
    hosts are rediscovered.
37. **Privilege escalation via host.** Compromised Tier 0 agent attempts
    host.vault.requestLease for a Tier 2 secret — verify host middleware blocks.
38. **Wasm agent double cage.** Wasm agent runs inside Deno, calls host.* API —
    verify requests are proxied and Wasm sandbox is active.
39. **Third-party package install.** Install package with author key, verify
    launch.sh is regenerated locally, not shipped.

### Coverage Target

- 95%+ for all library code (message, channel, vault, trust checker, host middleware)
- 80%+ for daemon code (health loops, signal handling, process supervision)

The daemon binary MUST translate Unix `SIGINT`/`SIGTERM` and Windows console
termination events into cooperative runtime shutdown. Native callbacks MUST NOT
perform reconciliation, transport, child-process, allocation, or locking work;
they only notify an ordinary execution context, which stops the listener and
allows supervised children to terminate and be reaped. The binary itself keeps
`unsafe` forbidden; platform handler installation is isolated behind a safe,
repository-owned package boundary.

---

## Dependencies

```
D18 Chief of Staff
│
├── built on ──► D19 Actor Package
│                └── Message, Channel, Actor primitives
│                └── Supervision trees, mailboxes, message passing
│                └── Orchestrator, hosts, vault, sub-agents are all D19 actors
│
├── extends ──► Spec 13 (Capability Security)
│                └── agent_manifest.json extends required_capabilities.json
│                └── Capability taxonomy reused + extended for agents
│                └── Friction stack layers (linter, CI, hardware key) apply
│                └── Signed packages extend the hardware-key gate concept
│
├── depends on ──► MSG-CRYPTO-FOUNDATION + repository crypto crates
│                   └── XChaCha20-Poly1305, Ed25519, X25519, HKDF, SHA-256
│                   └── VLT01 + Argon2id for Vault at-rest key derivation
│                   └── Ed25519 for package signing
│
├── depends on ──► D16 IPC
│                   └── Channels build on message queue / append-only log concepts
│                   └── Pipe's one-way semantics reused
│
├── depends on ──► D17 Network Stack
│                   └── Host sub-agents needing external access use socket API
│
├── depends on ──► D14 Process Manager
│                   └── Host and agent lifecycle: fork/exec/wait
│                   └── Orchestrator supervision uses SIGCHLD
│
├── depends on ──► D15 File System
│                   └── Channel logs persisted to disk
│                   └── Vault secrets stored as encrypted files
│                   └── Service registry stored as TOML
│
├── runtime ──► Rust (orchestrator, hosts, vault)
│               └── #![forbid(unsafe_code)] enforced project-wide
│               └── std::thread + std::sync::mpsc for concurrency
│
├── runtime ──► Deno (agent sandbox)
│               └── TypeScript agents run directly
│               └── Wasm agents run inside Deno
│               └── Deny-everything permission flags
│
└── used by ───► Future agent packages
                  └── email-reader, email-responder, calendar, finance,
                      browser, health, weather agents
```

---

## Trade-Offs

We are honest about the costs:

1. **Complexity vs. security.** The system has more components than OpenClaw's
   monolithic gateway — orchestrator, host actors, ephemeral sub-agents, vault,
   trust checker, signed packages, encrypted channels. Each component is more code
   to write, test, and audit. The benefit is that no single component's compromise
   gives total access. For bank credentials and health records, this trade-off is
   worth it.

2. **Latency from host mediation.** Every OS-level request from an agent goes through
   the host actor's middleware chain and an ephemeral sub-agent. This adds milliseconds
   per request. For AI agents that take seconds to process, this overhead is invisible.
   At personal scale (not millions of requests per second), this is never a bottleneck.

3. **Ephemeral sub-agent overhead.** Spawning and killing a sub-agent per request has
   process creation overhead. For the kinds of operations Chief of Staff handles —
   fetching emails, checking a bank balance, reading a file — requests happen a few
   times per minute, not thousands per second. The security benefit (no lingering
   state, no attack window) vastly outweighs the cost.

4. **Latency from encryption.** Every message on every channel is encrypted and
   decrypted. For text messages (a few KB), this adds microseconds — negligible.

5. **No shared state between pipelines.** Complete pipeline isolation means agents
   cannot share context. The Email Reader cannot tell the Finance Agent "the user's
   accountant mentioned a receipt in today's email." This is a feature (security) but
   has a UX cost (the user must manually connect dots across pipelines). Future work
   may add controlled cross-pipeline channels with explicit approval.

6. **Rust `#![forbid(unsafe_code)]` constraints.** Banning unsafe means no FFI, no
   raw pointers, no inline assembly. This constrains what the orchestrator and host
   can do, but provides compile-time memory safety guarantees. The orchestrator and
   host don't need FFI — they spawn processes and parse JSON. For OS-level operations,
   `std::process::Command` and `std::sync::mpsc` are sufficient and safe.

7. **Zero-dependency crypto.** Implementing XChaCha20-Poly1305 and Ed25519 from
   scratch (per the repo's zero-dependency philosophy) risks implementation bugs that
   battle-tested libraries like libsodium have already found and fixed. Mitigation:
   the crypto packages are checked against published vectors including RFC 8439
   (ChaCha20-Poly1305), RFC 8032 (Ed25519), RFC 7748 (X25519), RFC 5869 (HKDF),
   and RFC 9106 (Argon2id). D20 is the JSON specification, not a crypto roadmap.

8. **Dual runtime complexity.** Running Rust for infrastructure and Deno for agents
   means two toolchains, two build systems, and a JSON-RPC bridge between them. The
   benefit is using the best tool for each job: Rust's type system for safety-critical
   infrastructure, TypeScript/Deno for developer-friendly agent authoring.

9. **Deno runtime dependency.** While packages in this repo are zero-dependency, the
   agent runtime requires Deno. Deno is a single binary and is infrastructure (like
   needing Go or Node.js installed), not a library dependency.

10. **Single-user assumption.** This is a personal agent system, not a multi-tenant
    platform. The threat model assumes the adversary is a compromised agent or an
    external attacker, not a malicious co-user.

---

## Future Extensions

1. **Multi-device sync.** Channels replicated across devices (laptop, phone, home
   server) via end-to-end encrypted sync. The channel encryption already ensures that
   the sync transport cannot read payloads.

2. **Agent marketplace.** Signed agent bundles that can be installed with manifest
   review — similar to reviewing app permissions on a phone before installing. The
   package signing infrastructure is already in place; the marketplace adds discovery
   and distribution.

3. **Cross-pipeline channels (controlled).** Explicit, Tier 2+ approved channels
   between pipelines for cases where context sharing is genuinely needed. Each
   cross-pipeline channel requires biometric approval and is logged in the audit trail.

4. **GUI dashboard.** A web-based dashboard served by the orchestrator for pipeline
   visualization, approval workflows, and agent health monitoring. Served on loopback
   only.

5. **Multi-user support.** Shared pipelines between family members with separate
   privilege tiers and separate vault access. Each user has their own biometric gate.

6. **Wasm-only enforcement for Tier 2+.** Require Tier 2+ agents to compile to Wasm,
   providing the double cage (Wasm + Deno) for high-stakes operations. TypeScript
   agents would be restricted to Tier 0-1.
