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
