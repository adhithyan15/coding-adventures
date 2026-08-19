# Changelog

## 0.1.0

- Added `state::AgentState`: an in-memory, per-vault-name passphrase
  retention store with an `Instant`-based idle bound, a background-sweep
  entry point, and expiry checked both by sweep and at the point of use.
- Added `transport`: length-prefixed framing over any `Read`/`Write` stream,
  checking a declared length against a caller-supplied ceiling before
  allocating a payload buffer.
- Added `peer` (Unix): kernel peer-credential verification via `SO_PEERCRED`
  (Linux) and `getpeereid` (macOS/BSD/NetBSD/OpenBSD/DragonFly), with the
  authorization comparison split out as a pure function so it is testable
  against a fabricated mismatched UID.
- Added `server` (Unix): `AgentServer::bind`/`run` — a permission-checked,
  non-blocking accept loop with a `Ping`-verified staleness check (not a bare
  `connect`, which can misread a listener mid-shutdown as still live), a
  background idle-bound sweep thread, and closed-taxonomy request dispatch.
  A stale socket left by an unclean exit is reclaimed only after verifying
  it is a socket this same user owns; anything else fails closed with
  `InsecureExistingSocket` rather than being deleted.
- Added `client` (Unix): bounded (1.5 s) connect-and-request helpers —
  `ping`, `unlock`, `get_passphrase`, `lock`, `status`, `shutdown`,
  `wait_until_ready` — plus `cached_passphrase` and `forget_on_rejection`,
  the two fail-open helpers `vault-pm-cli`'s opportunistic-reuse and
  self-heal seams call directly.
- Added `lifecycle` (Unix): `spawn_detached`, the double-fork/`setsid`
  background-process spawn `agent start` uses to launch the agent, separate
  from `vault-pm-cli-host::clipboard`'s clearer spawn because the two have
  different standard-stream contracts (the agent takes no piped parameter
  block at all).
- Security review, before first release: every accepted connection is now
  handled on its own short-lived thread instead of serially in the accept
  loop, and `CONNECTION_TIMEOUT` is lowered from 5 s to 2 s. A serial loop let
  any process running as this same local user starve every other caller by
  opening a connection, sending nothing, and repeating; per-connection
  threads remove that head-of-line dependency (`AgentState`'s `Mutex` already
  serializes the state the threads share), proven directly by a test that
  holds one silent connection open and confirms a concurrent `Ping` still
  completes promptly.
- Security review, before first release: `transport::read_frame` now returns
  `Zeroizing<Vec<u8>>` unconditionally, so a frame carrying a plaintext
  passphrase (an `Unlock` request or a `Passphrase` response) is wiped on
  drop on both the client and server side of every round trip, not only in
  `AgentState`'s own long-lived retention.
- Security review, before first release: `peer::peer_uid` (Linux) now also
  verifies `getsockopt`'s returned length still equals `sizeof(ucred)`
  before trusting the credentials it wrote, failing closed rather than
  reading a partially uninitialized structure.
- Security review, before first release (caught on re-review of the
  thread-per-connection fix above): unbounded per-connection thread spawn is
  itself a denial of service — a same-user process opening connections in a
  tight loop and sending nothing on each would grow this process's thread
  count without bound. `AgentServer` now caps concurrent connections at
  `MAX_CONCURRENT_CONNECTIONS` (16), reserved with an `AtomicUsize` before a
  handler thread is spawned and released by a `Drop` guard
  (`ConnectionSlotGuard`) so a panic inside a handler still frees its slot. A
  connection accepted past the cap is dropped immediately, the same
  no-response treatment an unauthorized peer or a malformed frame already
  gets. Thread creation itself now goes through the fallible
  `Builder::spawn_scoped` rather than the panicking `Scope::spawn`, so a
  platform unable to create one more thread degrades to "drop this
  connection" instead of crashing the whole agent process.
- Security review, before first release: `state::AgentState::unlock` never
  validated `vault_name` against a real configured vault list and retained
  unconditionally, so a same-user peer could grow the retention map — and
  this process's memory — without bound by naming a fresh, unique vault on
  every connection; the per-connection concurrency cap above does not see
  this, since each such request is its own short, sequential connection, not
  a concurrency problem. `unlock` now refuses a *new* name once
  `state::MAX_RETAINED_VAULTS` (64) is reached (replacing an already-retained
  name is never refused by it) and returns `false` rather than silently
  accepting; the server maps a refusal to the new
  `AgentErrorCode::CapacityExceeded` response. `unlock` also now clamps
  `idle_bound` to `state::MAX_IDLE_BOUND` (24 hours) regardless of what the
  request asks for, so a caller cannot make a retained passphrase
  effectively immortal by requesting a near-`u64::MAX` bound and defeating
  the sweep thread that way.
