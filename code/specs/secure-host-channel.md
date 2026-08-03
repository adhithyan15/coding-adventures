# Secure Host Channel

## Overview

The Host Protocol defines **what** an agent says to its host (the JSON-RPC
methods, their params and results, the error model). This spec defines
**how the bytes get there safely**: the encrypted, ratcheted, flow-controlled
channel that carries every `host.*` message between an orchestrator, a host,
and an agent runtime.

Three guarantees this channel provides, in order of when each one bites:

1. **Confidentiality and integrity per message.** Every byte that crosses
   the channel is authenticated-encrypted with a key that is unique to that
   message. An attacker with a packet capture sees only ciphertext and cannot
   forge new messages.

2. **Forward and post-compromise security via ratcheting.** Compromising a
   child's session state at time T does not reveal messages sent before T
   (forward secrecy via the symmetric KDF chain) and does not reveal
   messages sent after the next DH ratchet step (post-compromise security
   via the periodic ephemeral DH exchange). Each child has an independent
   ratchet, so a compromised child does not compromise its siblings.

3. **DOS resistance.** A compromised or buggy child cannot saturate the
   orchestrator's mailbox, exhaust the orchestrator's CPU on decryption, or
   crash the orchestrator with malformed input. Three layered mechanisms —
   per-child token-bucket rate limiting, AIMD credit-window flow control,
   and a circuit breaker — keep the orchestrator's resources fairly
   distributed across all children, regardless of any one child's behavior.

The cryptographic machinery comes from the existing `vault-secure-channel`
package (X3DH for initial agreement + Double Ratchet for per-message
re-keying). The novel work in this spec is the **bootstrap** (how
orchestrator and child establish keys at spawn time), the **wire
integration** (how Host Protocol JSON-RPC messages ride on top of the
ratcheted channel), and the **DOS protection layer** (which has no
analog in Signal-protocol-like systems and is tuned for our adversary
model).

---

## Where It Fits

```
   Host Protocol  (host.* JSON-RPC method semantics)
        │
        │  every request / response / notification
        ▼
   Secure Host Channel  ← THIS SPEC
   ┌────────────────────────────────────────────────────────┐
   │  Outbound:                                              │
   │    1. priority queue (control / normal / low)           │
   │    2. credit-window flow control (AIMD)                 │
   │    3. token-bucket rate limit                           │
   │    4. circuit-breaker check                             │
   │    5. ratcheted AEAD encryption (vault-secure-channel)  │
   │    6. length-prefixed framing                           │
   │  Inbound:                                               │
   │    1. length-prefixed deframing                         │
   │    2. ratcheted AEAD decryption                         │
   │    3. authenticity / replay check                       │
   │    4. credit return (window grow on success)            │
   │    5. delivery to Host Protocol dispatcher              │
   └────────────────────────────────────────────────────────┘
        │
        │  raw bytes
        ▼
   Transport (stdio / Unix socket / Windows named pipe / in-process channel)
```

**Depends on:**
- `vault-secure-channel` (X3DH + Double Ratchet)
- `x3dh`
- `double-ratchet`
- `hkdf`
- `chacha20-poly1305`
- `vault-key-custody` (for orchestrator's long-term identity key)
- `actor` (for in-process channels and mailboxes used by the priority queue)

**Used by:**
- `orchestrator` (root of the channel; one channel per child host)
- `host-runtime-rust` (Tier 1 host's end of its child agent's channel)
- `host-runtime-wasm` (Tier 2 host's end)
- `host-runtime-subprocess` (Tier 3 host's end)
- Any future cross-process actor channel that needs encryption + DOS
  protection.

---

## Threat Model

The channel defends against all of the following adversaries:

| Adversary                           | Defended? | By what mechanism                                          |
|-------------------------------------|-----------|------------------------------------------------------------|
| Passive eavesdropper on the transport | Yes     | AEAD encryption (every byte is ciphertext)                 |
| Active MITM on the transport        | Yes       | Authenticated key agreement (X3DH binds to identity keys) |
| Replay of past encrypted messages   | Yes       | Per-message ratchet + nonce                                |
| Compromise of one child's state at time T, reading past traffic | Yes | Forward secrecy (KDF chain deletes message keys after use) |
| Compromise of one child's state at time T, reading future traffic | Yes after next DH ratchet | Post-compromise security (ephemeral DH re-keys the chain) |
| Compromise of one child reading another child's traffic | Yes | Each child has an independent ratchet rooted in a per-spawn key |
| Compromised child exhausting orchestrator CPU on decryption | Yes | Token-bucket rate limiting; circuit breaker                |
| Compromised child filling orchestrator's mailbox | Yes | AIMD credit-window flow control; bounded queues per child  |
| Compromised child sending malformed messages to crash the orchestrator | Yes | Strict length limits; parser fuzz tests; AEAD verification rejects malformed before parse |
| Compromised child sending high-priority traffic to starve other children | Yes | Per-child priority queues; orchestrator round-robin within priority |
| Compromise of vault master key                | No        | Out of scope; vault has its own threat model               |

What we explicitly do **not** defend against:

- A persistent attacker with simultaneous access to *both* endpoints' state.
  If they can read the keys on both sides, they can read traffic. This is
  fundamental.
- A buggy host that exfiltrates secrets via side channels (e.g., timing
  of vault.requestLease responses). This is a host bug, not a channel bug.
- DOS via legitimate methods that the manifest permits (e.g., a child with
  `net:connect:*` that asks the orchestrator to fetch huge URLs). This is
  what the rate limiter and the manifest's specificity are for, but no
  channel protocol can substitute for a tight manifest.

---

## Cryptographic Construction

The channel uses **vault-secure-channel** without modification. From the
existing implementation:

- **X3DH** (Extended Triple Diffie-Hellman) for the initial key agreement,
  binding the channel to the orchestrator's long-term identity key and the
  child's per-spawn ephemeral key.
- **Double Ratchet** (Signal Protocol) for per-message keys, combining a
  symmetric KDF chain (forward secrecy) with a periodic Diffie-Hellman
  ratchet step (post-compromise security).
- **XChaCha20-Poly1305** as the AEAD with caller-supplied AAD that binds the
  ciphertext to the channel context.
- **HKDF-SHA-256** for all key derivations.

This spec adds nothing to the cryptography. Its job is to define how the
channel is **bootstrapped** (where keys come from at spawn) and how each
Host Protocol frame becomes a ciphertext.

### Bootstrap at Child Spawn

When the orchestrator (or a host) spawns a child, the channel is
established before any Host Protocol message flows:

```
ORCHESTRATOR                                          CHILD HOST PROCESS
────────────                                          ──────────────────

(holds long-term identity key OIK after vault-key-custody unlock)
generate a fresh per-spawn signed X25519 prekey OSPK
build the standard X3DH PreKeyBundle
  { OIK_x25519_pub, OIK_ed25519_pub,
    OSPK_pub, OIK_signature(OSPK_pub) }

spawn child process with:
  - argv: <path to host package>
  - env:  CHANNEL_BOOTSTRAP_FD=3  (the fd carrying bootstrap)
  - fd 3: read end of pipe carrying:
          a bounded, versioned BootstrapOffer containing:
          { host_id, canonical UUID-v7 session_id,
            orch_prekey_bundle }
                                                       │
                                                       ▼
                                                 read bootstrap from fd 3
                                                 generate child identity key CIK
                                                 (X3DH generates a single-use
                                                  child ephemeral key CEK)

                                                 ChannelInitiator::open(
                                                   identity = CIK,
                                                   bundle   = OIK+OSPK,
                                                   payload  = "hello",
                                                   aad      = bootstrap_aad(
                                                                host_id,
                                                                session_id))
                                                 produces first wire message

                                                 send ClientHello containing
                                                 CIK_x25519_pub and the first
                                                 vault-channel wire message to
                                                 orchestrator over the bidirectional
                                                 transport (e.g., stdin/stdout)

orch reads ClientHello
ChannelResponder::accept(...)
uses CIK_x25519_pub plus the embedded CEK_pub
both sides now share a ratchet root
                                                       ◄── ratcheted from here on ──►

both sides immediately rotate:
  - orchestrator writes its first response with a fresh DH key
  - child receives, advances ratchet
each subsequent exchange may include DH ratchet step
```

Properties this gives us:

1. The child's identity key is generated at spawn time inside the child's
   sandbox. The orchestrator never holds it. If a child is compromised
   later, the attacker can read that one channel; without the child's
   identity key from the moment of spawn, the attacker cannot impersonate
   the orchestrator to other children.

2. The orchestrator's per-spawn signed prekey OSPK is rotated on every
   spawn. If a child is compromised and exfiltrates OSPK_pub plus its own
   state, the attacker still cannot read other children's traffic; their
   X3DH used a different OSPK.

3. The bootstrap travels over a fresh pipe fd known only to the parent and
   the immediate child. It does not pass through stdin/stdout (which a
   Tier 3 BYO process might log accidentally) and does not appear in
   environment variables (which could be inspected by a sibling process
   on a non-sandboxed system).

4. The AAD is derived internally from the protocol domain, host id,
   session id, direction, and sequence. Callers cannot accidentally omit
   or vary one of those fields. A captured ciphertext from one session
   cannot be replayed into another — the AEAD verification fails because
   the AAD does not match.

The protocol kernel does not open file descriptors or choose a transport.
It encodes/decodes the offer and hello and turns plaintext into bounded wire
frames. The process supervisor is responsible for carrying those byte strings
over an inherited pipe, stdio, a Unix socket, or a named pipe. Keeping I/O
behind that boundary makes the cryptographic state machine deterministic and
portable across all three CI operating systems.

### Per-Message Encryption

Every Host Protocol message — request, response, notification, stream
chunk — passes through the channel as an AEAD ciphertext:

```
plaintext  = utf-8 json (a single Host Protocol message)
aad        = len(domain) || domain || len(host_id) || host_id
             || session_id || direction_tag || sequence_be_u64
ciphertext = vault_secure_channel.send(plaintext, aad)
```

The direction is an unambiguous one-byte tag and the sequence is a monotonically
increasing big-endian `u64` per direction. Length-prefixing variable fields
prevents concatenation ambiguity. Including direction in AAD prevents an
attacker from replaying a child's request as if it came from the
orchestrator. Sequence exhaustion closes the channel rather than wrapping.
Although the host record reserves a `u64`, V1 closes at the underlying Double
Ratchet's `u32` message-counter limit; later versions may use the remaining wire
range only if the cryptographic primitive can do so without an inner wrap.

The Double Ratchet's internal counters and DH ratchet steps are managed by
`vault-secure-channel`; this spec does not override them.

If AEAD authentication fails, the protocol kernel permanently closes that
channel instance. The underlying ratchet consumes key material while attempting
decryption, so retrying after an authentication failure would operate from an
ambiguous state. Structural frame errors are rejected before invoking the
ratchet and do not advance the receive sequence.

### Key Rotation Schedule

The Double Ratchet rotates DH keys on every reply. Additionally, this spec
mandates:

- **Forced DH ratchet step every 60 seconds** of channel inactivity.
  Periodic re-keying limits the post-compromise window even when the
  channel is quiet.
- **Forced full re-handshake every 24 hours** of continuous channel
  uptime. Bounds the total state any single ratchet can accumulate and
  exercises the bootstrap path on long-running children.
- **Forced full re-handshake on host-runtime restart.** A restarted child
  is a new child; old keys are zeroized and a fresh bootstrap is run.

---

## DOS Protection

A child runtime is, by assumption, in an adversarial position relative to
the orchestrator. It may be honest, buggy, or actively compromised. The
channel must keep the orchestrator's resources fairly distributed across
all children no matter what one child does.

Three layered mechanisms.

### Layer 1: Per-Child Token Bucket

Every channel has a token bucket on the orchestrator's inbound side.
Tokens are added at a configured **refill rate** (per-child, per-method);
each inbound message consumes one or more tokens depending on its type.

```
TokenBucket
═══════════════════════════════════════════════════════════════
┌──────────────────┬─────────────────────────────────────────┐
│ capacity         │ Maximum tokens the bucket can hold       │
│ refill_rate      │ Tokens added per second                  │
│ tokens           │ Current count                            │
│ last_refill      │ Monotonic timestamp of last refill check │
└──────────────────┴─────────────────────────────────────────┘
```

**Default cost table:**

```
Message kind                          Tokens
──────────────────────────────────    ──────
system.now / system.unixTime          0  (free; orchestrator-internal)
system.log                            1
channel.read / channel.peek / .ack    1
fs.* (read/write/etc.)                5
vault.requestLease                    10
vault.requestDirect                   10
network.fetch                         10
proc.exec                             20
```

Defaults: capacity = 100 tokens, refill_rate = 50 tokens/sec. A well-behaved
child running steady-state operations stays well below this. A misbehaving
child making 1000 fetches/sec is choked at 5/sec (= 50 tokens / 10 cost)
and the rest are returned `RateLimited` errors.

The numbers are tunable per child via the manifest; a bulk-data agent may
get a higher `network.fetch` budget.

### Layer 2: AIMD Credit-Window Flow Control

The orchestrator advertises a **credit window** to each child: the maximum
number of outstanding (sent but not yet acknowledged) messages. The window
adapts using AIMD — Additive Increase, Multiplicative Decrease — borrowed
directly from TCP congestion control.

```
window_state
═══════════════════════════════════════════════════════════════
┌──────────────────┬─────────────────────────────────────────┐
│ cwnd             │ current window size (msgs in flight)     │
│ ssthresh         │ slow-start threshold                     │
│ in_flight        │ number of unacknowledged outbound msgs   │
│ congestion_count │ counter incremented on backpressure      │
└──────────────────┴─────────────────────────────────────────┘
```

**Slow start**: cwnd starts at 4. Every successful round-trip doubles
cwnd until it reaches `ssthresh` (default 64).

**Congestion avoidance**: above `ssthresh`, cwnd grows linearly: `cwnd +=
1 / cwnd` per round-trip.

**Congestion signal**: any of
- inbound mailbox depth exceeds 75% of capacity
- decryption latency exceeds 50 ms (sign of CPU contention)
- circuit breaker fires
triggers `ssthresh = cwnd / 2; cwnd = ssthresh`. Multiplicative decrease.
Recovery is gradual.

The child's SDK respects the window: if its in-flight count equals cwnd,
new outbound messages block (in-process) or are queued with a soft limit
(cross-process). If the queue overflows, the SDK returns
`ChannelBackpressure` to the agent code.

This is the same mechanism that keeps TCP from overwhelming a slow
receiver. We borrow the algorithm because it has been proven on the
internet for forty years.

### Layer 3: Circuit Breaker

Per-child circuit, three states:

```
                 ┌────────────┐
   normal ─────► │   CLOSED   │ ◄────── trial probe succeeds
   operation     │ all calls  │
                 │ permitted  │
                 └─────┬──────┘
                       │
                       │  N consecutive failures
                       │  OR rate > threshold
                       ▼
                 ┌────────────┐
                 │    OPEN    │ ────► all inbound messages from
                 │ all calls  │       this child rejected with
                 │  rejected  │       CircuitOpen error
                 └─────┬──────┘
                       │
                       │  cooldown timer expires (default 30s)
                       ▼
                 ┌────────────┐
                 │ HALF_OPEN  │ ────► one trial probe permitted
                 │ probing    │       success → CLOSED
                 │            │       failure → OPEN (reset timer)
                 └────────────┘
```

The circuit opens on:
- 10 consecutive `RateLimited` responses to a child (it isn't slowing down)
- 50 protocol parse failures in 60 seconds (malformed message storm)
- AEAD verification failures exceeding 5 in 60 seconds (possibly
  tampering, possibly a sync bug — either way, stop)

When OPEN, the channel is logically closed: the orchestrator drops
inbound messages without decrypting (saving CPU), and any attempt to send
to the child returns `ChannelClosed`. The supervisor is notified; per its
restart policy, it may terminate and restart the child. A restart begins
with a fresh bootstrap, a fresh ratchet, and the circuit returns to
CLOSED.

### Priority Queues

Inbound messages are sorted into three priority queues per child:

```
control     handshake, ping, _internal.cancel, _internal.hello
            → drained first, always
normal      all method calls (network.fetch, fs.*, vault.*, etc.)
            → drained round-robin across children
low         system.log notifications, telemetry
            → drained only when control and normal queues are empty
```

Round-robin within `normal` ensures that no single child can starve
others by sending many in-budget requests in a tight loop. A child with
50 pending normal-priority requests gets one per round, just like every
other child.

### Layer 4: Panic Broadcast

Rate limits, flow control, and circuit breakers protect the orchestrator
that holds the channel. But a determined attacker who has compromised
one child may try to exhaust the *entire* tree by attacking many
orchestrators at once, or by attacking one orchestrator from a child
that the breaker hasn't yet caught. We need a way for an orchestrator
under attack to **escalate fast** — not wait for the next restart
cycle, not rely on per-orchestrator state — and to inform the rest of
the tree so siblings can defend themselves before they get hit too.

This is the panic broadcast.

**Detection.** Any of these conditions trigger a panic on a channel:

| Condition                                              | Severity   |
|--------------------------------------------------------|------------|
| Circuit breaker opens                                  | Warn       |
| Two or more circuits open within 60 seconds            | Alert      |
| AEAD verification failures > 3 per minute (tampering?) | Alert      |
| Replay window violations > 10 per minute               | Alert      |
| Inbound mailbox at capacity for > 5 seconds            | Alert      |
| Decryption CPU usage > 50% sustained                   | Alert      |
| Three or more Alert-level conditions concurrent        | Critical   |
| Orchestrator's own scheduling latency > 1 second       | Critical   |

A `PanicSignal` is constructed at the moment of detection:

```
PanicSignal
═══════════════════════════════════════════════════════════════
┌──────────────────┬─────────────────────────────────────────┐
│ id               │ UUID v7 (time-ordered, unique)           │
│ origin           │ Orchestrator ID that detected the attack │
│ suspect          │ ChildId of the most likely attacker      │
│                  │   (the child whose channel triggered the │
│                  │    condition; may be "unknown" if many)  │
│ trigger          │ Which condition fired (enum above)       │
│ severity         │ Warn | Alert | Critical                  │
│ evidence         │ JSON object with concrete numbers:       │
│                  │   {circuits_open: 5,                     │
│                  │    aead_failures_60s: 12,                │
│                  │    mailbox_depth_pct: 100, ...}          │
│ timestamp        │ Monotonic nanosecond clock               │
│ recommended      │ KillSuspect | QuarantineSubtree |        │
│   action         │   RaiseAdjacentThresholds | NotifyHuman  │
└──────────────────┴─────────────────────────────────────────┘
```

**Broadcast topology.** Three directions, each carrying different
guarantees:

```
                     Root Orchestrator
                        ▲           │
                upward  │           │ downward (informational)
                (action)│           ▼
                     Parent Orchestrator (the responder)
                        ▲           │
                        │           │
              detected  │           │ sideways (informational)
                        │           ▼
                  Local Channel ──► Sibling Channels
                        │
                        ▼
                    Suspect Child
```

- **Upward to parent (REQUIRED, action path):** Sent over a dedicated
  `panic` priority channel that bypasses the normal flow control and
  rate limiting (it is itself a control-priority message). The parent
  receives this signal *immediately* and acts on it.
- **Sideways to siblings (informational):** Sent so siblings can
  proactively raise their own thresholds (lower bucket capacities,
  smaller cwnd, faster circuit-open). Siblings do not act on the
  suspect — they don't have authority over it — but they prepare to
  defend themselves from a coordinated attack.
- **Downward to root (informational):** Echoed up the chain so the
  root orchestrator sees a global view. The root may decide to log,
  page a human, or initiate a tree-wide quarantine.

**Parent's response actions.** When a parent receives a panic signal
naming a suspect that is its child, it executes immediately, in this
order:

1. **Brutal terminate the suspect.** Skip graceful shutdown; the
   child is presumed compromised. Send `Terminate` directly; do not
   wait for any grace period. Zeroize the channel keys.

2. **Quarantine.** Add the suspect's `(child_id, manifest_hash)` to a
   quarantine list with a cooldown timer (default 5 minutes). The
   supervisor's restart logic checks this list before re-spawning;
   children on the list are not restarted until their cooldown
   expires. After cooldown, a single trial restart is permitted
   (quarantine half-open); a fresh panic resets the cooldown and
   doubles it (exponential backoff, max 24 hours).

3. **Raise adjacent thresholds.** Children in the same subtree as the
   suspect have their channel policies tightened: bucket capacity ÷ 2,
   cwnd ÷ 2, circuit failure threshold ÷ 2. After 60 seconds without
   another panic, thresholds gradually return to baseline.

4. **Audit log.** Write a structured record to the audit log with the
   full PanicSignal, the action taken, and a snapshot of the suspect's
   recent traffic (for forensics). The audit log is on its own channel
   that is never affected by panic-induced throttling.

5. **Acknowledge upstream.** Send an `Ack` notification back along the
   path the panic arrived on, including the action taken. This lets
   the originator know the kill was successful.

**Sibling's response actions** (informational receipt, no authority
over the suspect):

1. Lower its own bucket capacity, cwnd, and circuit threshold by half.
2. Schedule restoration after 60 seconds of quiet.
3. Write the receipt to the audit log.

**Root's response actions** (global view):

1. Aggregate panic signals across the entire tree.
2. If panic-signal rate across the tree exceeds a global threshold
   (default: 5 distinct alerts in 60 seconds), trigger
   **tree-wide quarantine**: pause all new child spawns, freeze
   manifest installations, and page a human. The tree continues
   serving existing children but accepts no new ones until a human
   clears the quarantine.
3. Persist panic history to the audit log for postmortem analysis.

**Authentication of panic signals.** A compromised child must not be
able to forge panics about its siblings. Two defenses:

1. The panic-priority channel between an orchestrator and its parent
   is the *same ratcheted channel* the rest of the protocol uses.
   Forging a panic requires forging a signed-by-the-orchestrator
   message, which requires the orchestrator's keys.
2. The signal's `origin` field is set at the orchestrator that
   detected the condition; a downstream orchestrator forwarding the
   signal preserves the original `origin`. Receivers can verify the
   chain of forwarders against the supervision tree topology.

**Bandwidth considerations.** A panic signal is a single small JSON
message (~1 KB). At Critical severity it bypasses flow control. We
explicitly accept this tradeoff: if there is a real attack, the few
KB of panic traffic is dwarfed by the attack's own bandwidth, and
suppressing the signal would be far worse than allowing it.

**Replay defense.** Panic signals are not replayed because each
carries a UUID v7 (time-ordered, unique) and a monotonic timestamp;
receivers track the highest seen `id` per `origin` and reject older
ones. A replay window of 10,000 IDs is maintained per origin.

---

## Wire Format

Bootstrap and encrypted records use separate magic values and one common
version. Every encrypted data frame on the wire is:

```
┌──────────────┬────────────────────────────────────────────────┐
│ "D18F"       │ version │ direction │ session UUID │ sequence  │
├──────────────┼─────────┴───────────┴──────────────┴───────────┤
│ length: u32  │ vault-secure-channel ciphertext                │
│ big-endian   │ ("CN" || DR header || inner length || AEAD)     │
└──────────────┴────────────────────────────────────────────────┘
```

The host frame header is authenticated indirectly because its direction,
session, and sequence fields are reconstructed as AEAD AAD. The receiver
validates magic, version, direction, canonical UUID-v7 bits, expected sequence,
and the declared ciphertext length before copying or decrypting ciphertext.
Unknown versions, truncated records, padded records, wrong-session records,
wrong-direction records, oversized lengths, and trailing bytes fail closed.

`BootstrapOffer` uses magic `D18O`; `ClientHello` uses magic `D18H`. Both carry
the same version, bounded host id, and canonical UUID-v7 session id as the data
frame. The offer additionally carries the orchestrator X3DH public bundle. The
hello additionally carries the child's X25519 identity public key and a bounded
vault `FirstMessage`. Each decoder validates every declared length before
copying the corresponding bytes and requires exact end-of-record consumption.

For stdio transport, the line-delimited JSON format from
`host-protocol.md` is **replaced** by length-prefixed framing once the
secure channel is established. The bootstrap message itself uses
length-prefixed framing from the very first byte (the orchestrator and
the child agree never to interleave plaintext JSON with ciphertext
frames).

**Maximum frame size:** 1 MiB. A frame larger than this is rejected
without decryption. Streaming large payloads is done with multiple
chunked frames (as defined in `host-protocol.md`).

**Replay policy:** V1 supervisor transports are ordered byte streams, so the
protocol kernel requires the exact next sequence. Replays and gaps fail before
decryption. A future datagram adapter may add a bounded reorder window outside
the cryptographic state machine; it must not weaken the in-order stream mode.

---

## Key Custody

Long-term identity keys are stored in the vault under
`vault://orchestrator/identity/`. The orchestrator unlocks them at
startup using the same authentication path as the rest of the vault
(passphrase, biometric, hardware key — see `VLT05-vault-auth.md`).

Per-spawn ephemeral keys live only in the orchestrator's memory and are
zeroized when the child terminates. They are never written to disk and
never appear in logs.

Per-child session keys derived during X3DH live only in the running
ratchet state on each side. They are zeroized in two cases:
- on every DH ratchet step, after the new key is derived
- when the child terminates

The spec mandates use of `Zeroizing<>` types for all key material in the
Rust implementation, matching the existing vault crates.

---

## Public API

```rust
// ─────────────────────────────────────────────────────────────────
// Channel handle (used by both orchestrator and child sides)
// ─────────────────────────────────────────────────────────────────

pub struct OrchestratorBootstrap<'a> { /* borrowed OIK + per-spawn OSPK */ }
pub struct ChildBootstrap { /* per-spawn CIK */ }
pub struct BootstrapOffer(Vec<u8>);
pub struct ClientHello(Vec<u8>);

pub struct SecureHostChannel {
    /* opaque ratchet, role, host/session identity, and counters */
}

impl<'a> OrchestratorBootstrap<'a> {
    pub fn new(identity: &'a IdentityKeyPair, host: HostId, session: SessionId)
        -> Result<Self, ChannelError>;
    pub fn offer(&self) -> Result<BootstrapOffer, ChannelError>;
    pub fn accept(self, hello: &ClientHello)
        -> Result<SecureHostChannel, ChannelError>;
}

impl ChildBootstrap {
    pub fn open(offer: &BootstrapOffer)
        -> Result<(SecureHostChannel, ClientHello), ChannelError>;
}

impl SecureHostChannel {
    pub fn send(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, ChannelError>;
    pub fn receive(&mut self, frame: &[u8]) -> Result<Vec<u8>, ChannelError>;
    pub fn role(&self) -> ChannelRole;
    pub fn host_id(&self) -> &HostId;
    pub fn session_id(&self) -> SessionId;
}

// ─────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────

pub enum ChannelError {
    InvalidHostId,
    InvalidSessionId,
    MalformedBootstrap,
    MalformedFrame,
    UnsupportedVersion,
    FrameTooLarge,
    WrongSession,
    WrongDirection,
    UnexpectedSequence,
    SequenceExhausted,
    Crypto,
    Closed,
}

// ─────────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────────

pub struct ChannelPolicy {
    pub bucket_capacity:       u32,
    pub bucket_refill_rate:    u32,
    pub method_costs:          MethodCostTable,
    pub initial_cwnd:          u32,
    pub ssthresh:              u32,
    pub max_cwnd:              u32,
    pub max_frame_size:        u32,
    pub circuit_failure_threshold: u32,
    pub circuit_cooldown:      Duration,
    pub idle_dh_rotation:      Duration,
    pub max_session_uptime:    Duration,
}

impl Default for ChannelPolicy {
    fn default() -> Self {
        Self {
            bucket_capacity:       100,
            bucket_refill_rate:    50,
            method_costs:          MethodCostTable::default(),
            initial_cwnd:          4,
            ssthresh:              64,
            max_cwnd:              256,
            max_frame_size:        1 * 1024 * 1024,
            circuit_failure_threshold: 10,
            circuit_cooldown:      Duration::from_secs(30),
            idle_dh_rotation:      Duration::from_secs(60),
            max_session_uptime:    Duration::from_secs(60 * 60 * 24),
        }
    }
}
```

The first implementation slice is the transport-independent protocol kernel:
X3DH bootstrap, strict offer/hello/frame codecs, AAD construction, authenticated
encryption, exact stream sequencing, and fail-closed state transitions. Queueing,
token buckets, AIMD, circuit breakers, timers, process termination, and panic
broadcast remain supervisor/transport layers. They retain the acceptance tests
below and are not silently approximated inside the cryptographic kernel. A
future datagram adapter may define its own bounded reorder policy; the base
`ChannelPolicy` deliberately has no replay-window field.

---

## Test Strategy

### Unit Tests

1. **Bootstrap correctness.** Orchestrator and child complete the X3DH
   handshake and exchange authenticated application messages.
2. **Per-message encryption / decryption round-trip.** Send 1000 messages
   in each direction; every one decrypts correctly with the right AAD.
3. **Forward secrecy.** Snapshot the channel state at message N. Modify
   subsequent messages 1..N-1 to use the snapshot's keys; verify
   decryption fails.
4. **Post-compromise security.** Snapshot at message N. Trigger a DH
   ratchet step at message N+1. Attempt to decrypt N+2 with the snapshot;
   verify failure.
5. **Per-child isolation.** Bootstrap two children with the same
   orchestrator. Snapshot child A's keys. Verify they cannot decrypt
   child B's traffic.
6. **AAD binding.** Reuse a frame with a different host, session, direction,
   or sequence; verify it is rejected.
7. **Replay rejection.** Re-send a captured ciphertext; verify the exact-next
   sequence policy rejects it before decryption.
8. **Strict wire format.** Every truncated prefix of an offer, hello, and frame
   is rejected. Unknown versions, invalid host/session values, wrong direction,
   gaps, oversized declared lengths, padding, and trailing bytes are rejected.
9. **State transitions.** A structural rejection does not advance the receive
   sequence, so the valid expected frame still succeeds. An AEAD authentication
   failure closes the channel and all subsequent operations return `Closed`.
10. **Sequence exhaustion.** Sending or receiving never wraps a `u64` sequence.

### DOS Protection Tests

11. **Token bucket.** A child sending at 2x its budget receives
   `RateLimited` for the excess. Tokens refill at the configured rate.
12. **AIMD slow start.** With cwnd=4 and ssthresh=64, after 4 successful
    round-trips cwnd reaches 64.
13. **AIMD multiplicative decrease.** Inject a congestion signal at
    cwnd=128; verify cwnd halves to 64 and growth resumes linearly.
14. **Mailbox saturation.** Drive a child's inbound rate above the
    orchestrator's drain rate; verify queue depth triggers cwnd
    reduction.
15. **Circuit open.** Trigger 10 consecutive `RateLimited` responses;
    verify circuit opens, all subsequent inbound messages dropped
    without decryption.
16. **Circuit half-open.** After the cooldown, one probe is permitted.
    Success closes the circuit; failure re-opens with reset timer.
17. **Priority starvation.** A child sending many `system.log`
    notifications cannot delay another child's `network.fetch` requests.
18. **No cross-child impact.** A misbehaving child A (max rate, max
    queue depth, circuit thrashing) does not affect throughput or
    latency for child B.

### Panic Broadcast Tests

19. **Panic on circuit open.** Trigger a circuit open; verify a
    PanicSignal with severity Warn is emitted on the parent panic
    channel within 100 ms.
20. **Panic escalation.** Open two circuits within 60 seconds; verify
    severity escalates to Alert.
21. **Brutal kill on panic.** Parent receives a panic signal naming a
    child; verify the child process is killed within 100 ms without a
    graceful shutdown attempt.
22. **Quarantine.** A killed child is marked quarantined; the
    supervisor refuses to restart it for the cooldown period; after
    cooldown a single trial restart is permitted; a fresh panic doubles
    the cooldown.
23. **Threshold tightening on siblings.** A panic in subtree X causes
    siblings of X to halve their bucket and cwnd; thresholds restore
    after 60 seconds of quiet.
24. **Panic forging defense.** A child attempting to send a forged
    panic about its sibling cannot — the panic channel keys are not
    available to children, only to orchestrators.
25. **Panic replay rejection.** Re-sending a captured panic signal is
    rejected by the per-origin replay window.
26. **Tree-wide quarantine.** Five distinct alerts within 60 seconds
    across the tree triggers tree-wide quarantine at the root: no new
    spawns, no manifest installs, human paged.

### Integration Tests

27. **End-to-end with two real children.** Spawn two child host
    processes, exchange thousands of Host Protocol calls in parallel,
    verify all complete correctly.
28. **Child crash mid-channel.** Kill a child while it has outstanding
    requests. Orchestrator detects via transport EOF, zeroizes keys,
    notifies supervisor.
29. **Forced re-handshake.** Run a channel for 24 hours simulated time;
    verify the full re-handshake fires and the new keys differ from the
    old.
30. **Compromise simulation.** Capture a snapshot of one child's
    ratchet state. Continue running for one ratchet step. Verify the
    snapshot can no longer decrypt new messages.

### Coverage Target

`>=95%` line coverage. Both crypto and DOS protection logic are
foundational; bugs here corrupt every host's communication.

---

## Trade-Offs

**JSON inside ciphertext is 2-3× larger than a binary protocol would
be.** Acceptable: agent message rates are bounded by model latency, not
by serialization throughput. The debuggability win (record + replay,
manual inspection of decrypted traffic in tests) outweighs the
bandwidth cost.

**AIMD is from a different problem domain (network congestion control)
than ours (mailbox saturation).** The mathematics still apply: any
shared resource with feedback signals can use AIMD for fairness. We
borrow it deliberately because it is exhaustively analyzed and
implemented; bespoke flow control would take years to validate.

**Token costs are heuristic, not derived from first principles.** The
costs in the default table reflect rough resource estimates (e.g., a
network fetch involves a sub-agent spawn, a DNS lookup, a TLS
handshake, and an HTTP round-trip; far more orchestrator work than a
clock read). Tuning per agent is supported via the manifest. We expect
real workloads to inform refinements.

**A trusted child still gets a circuit breaker.** A first-party Tier 1
agent that we wrote ourselves still goes through the same flow control
and rate limiting. This is intentional: there is no special
"trusted child" code path. Bugs in trusted code are the most common
source of accidental DOS, and a circuit breaker that also catches them
costs us nothing.

**Forced re-handshake every 24 hours.** Long-running daemons will
re-handshake even when they don't need to. The cost is negligible
(milliseconds), and the practice exercises the bootstrap path
continuously, surfacing regressions quickly. We considered a longer
period; 24 hours felt like the right balance between exercise and
churn.

**Bootstrap fd vs. environment variable.** Passing the bootstrap as a
serialized blob via fd 3 is more complex than putting it in an
environment variable. We do it because env vars are inherited by
grandchildren, can leak via `/proc/<pid>/environ` on Linux, and may be
logged by misbehaving runtimes. A pipe fd is opened, read once, and
closed; the secret window is small.

**No support for connection migration.** If the underlying transport
breaks (the pipe closes, the socket is severed), the channel must be
fully re-bootstrapped — there is no "resume on a new transport." This
matches our model: a transport failure means the child process is gone
or unreachable, and a new bootstrap is appropriate.

---

## Future Extensions

- **Bandwidth-based credits.** Today the credit window counts messages.
  A future version may also constrain in-flight bytes for transports
  with strict bandwidth budgets.
- **Per-method circuit breakers.** Today the circuit is per-child. A
  future version may open a circuit only for one misbehaving method
  (e.g., `network.fetch` to a flaky upstream) while permitting others.
- **Distributed ratchets.** A future version may support an orchestrator
  on host A talking securely to a child on host B over the network,
  using the same channel primitive layered atop a network transport.
- **Shared-secret rotation hooks.** A future version may surface ratchet
  rotation events to the audit log and to monitoring channels for
  forensic visibility.

These are deliberately out of scope for V1.
