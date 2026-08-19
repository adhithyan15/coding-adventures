# `coding_adventures_vault_pm_agent_host`

This crate is the local host-side capability VLT-PM00 §23 item 12 named: a
permission-checked Unix-domain-socket transport and an in-memory
passphrase-retention store for the vault-pm local agent, plus the client
helpers and detached-spawn lifecycle `vault-pm-cli` drives them through. It
knows the wire format (`coding_adventures_vault_pm_agent_protocol`) and knows
how to reach the socket (`coding_adventures_vault_pm_local_host`'s
`runtime_root`/`agent_socket_path`), and nothing else about vaults —
`state::AgentState` retains an opaque passphrase per vault *name* and never
imports `vault-pm-application`, `vault-pm-repository`, or any cryptographic
package in this product. See `VLT-PM48-local-agent-ipc.md` §4.2 for why that
absence is load-bearing: the agent trusts every `Unlock` request
unconditionally, because verification already happened one layer up, in the
CLI process that authenticated against the real vault before ever calling
`client::unlock`.

## Module map

- `state` — the pure retention store and idle-bound policy. Unit-tested
  without a socket, a thread, or a real clock reading.
- `transport` — length-prefixed framing over any `Read`/`Write` stream.
- `peer` (Unix only) — the authoritative permission check: `SO_PEERCRED` on
  Linux, `getpeereid` on macOS/BSD, checked against this process's own real
  UID before a single request byte is read.
- `server` (Unix only) — socket bind, a non-blocking accept loop, a
  background idle-bound sweep thread, and request dispatch.
- `client` (Unix only) — bounded connect-and-request helpers, including the
  fail-open `cached_passphrase` lookup every one-shot command uses.
- `lifecycle` (Unix only) — the double-fork, session-detached spawn `agent
  start` uses to launch the long-lived agent process.

## Permission model

Two layers, and neither is trusted alone:

1. **Filesystem.** The socket's parent directory is owner-only (`0700`,
   verified by `vault-pm-local-host`), and the socket file is `chmod 0600`
   immediately after bind.
2. **Peer credentials — the one that actually matters.** Every accepted
   connection's kernel-reported real UID is compared against this process's
   own, before anything is read from it. A mismatch gets no response: not a
   typed rejection, not a protocol error, nothing. A permissive umask, a
   misconfigured parent directory, or a filesystem that does not enforce Unix
   permissions the way the local host expects can all leave a `0600` file
   reachable by another local user; the kernel-verified UID cannot be
   misrepresented by any of those.

Root gets no special case: an agent started as an unprivileged user is not
reachable by root through this check.

## Retention and auto-lock

`AgentState` retains a wipe-on-drop passphrase per vault name, each with its
own `retained_at`/`idle_bound` pair measured against `std::time::Instant` —
monotonic, unlike the advisory wall clock `vault-pm-cli::shell::ShellSession`
has to defend against stepping backwards. A background sweep thread removes
every expired entry on a short fixed interval, independent of whether any
client asks about it; every `GetPassphrase` answer also double-checks expiry
at the point of use, for the same reason `ShellSession::authenticator` does.
This is the *pre-emptive* auto-lock timer `VLT-PM40-cli-interactive-shell.md`
§3.5 explicitly deferred to this crate, because a foreground shell blocked on
a terminal read has nowhere for that timer to run and a background process
does.

`AgentState` trusts every `Unlock` it receives unconditionally (see below)
and never validates `vault_name` against a real configured vault list, so
two bounds keep an untrusted same-user peer from using that trust to exhaust
this process's memory: `MAX_RETAINED_VAULTS` (64) caps how many *distinct*
vault names it will ever hold a passphrase for at once — a peer naming a
fresh, unique vault on every connection is refused, not silently accepted,
once the cap is reached — and `MAX_IDLE_BOUND` (24 hours, matching
`vault-pm-config::MAX_AUTO_LOCK_SECONDS`) clamps whatever `idle_bound_ms` a
caller asks for, so a request naming an effectively infinite bound cannot
defeat the sweep above by simply never expiring.

## Socket path

Resolved by `vault-pm-local-host::LocalVaultPaths::runtime_root`, not nested
under the platform data directory (`sockaddr_un.sun_path` is bounded to
roughly 100 bytes, and a verbose platform data path can already consume most
of that) and not derived from `XDG_RUNTIME_DIR` (one fixed path per login
session would let two different data roots collide on one agent). It is a
short, deterministic directory beside the system temporary directory, named
from a truncated hash of the data root, verified or created lazily and only
by code that actually needs it — an ordinary command that never touches the
agent never reaches into the system temporary root as a side effect.

## Windows

Deferred. Every socket-touching module here is `#[cfg(unix)]`; this crate
still compiles on every target so a caller (`vault-pm-cli::agent`) can gate
its own platform split cleanly rather than needing two separate
dependencies.

## Verification

Twenty-four tests cover the retention store's idle-bound policy (unlock,
expiry at point-of-use, replace-and-restart, forget-one/forget-all,
background sweep, and the two capacity bounds below), framing (round trip,
oversized-frame refusal before allocation, truncated reads), the
peer-authorization comparison against a fabricated mismatched UID, real
bind/accept/dispatch round trips for every request over a real socket
(including a second bind being refused, a stale socket being reclaimed only
after an ownership check, a non-socket path never being deleted, and an
oversized or garbage connection being dropped without a response while the
server keeps serving), and a real detached process outliving its parent.

Several tests are worth calling out — this feature went through three rounds
of security review before its first release, and each of the following
closes a real finding from it:

- `a_silent_connection_never_blocks_a_concurrent_well_behaved_one`: serving
  every accepted connection synchronously in the accept loop let any
  same-user process starve every legitimate caller by opening a connection
  and sending nothing, forever. Every connection is now served on its own
  thread.
- `connections_past_the_concurrency_cap_are_dropped_and_capacity_recovers`:
  the fix above then introduced its own finding — unbounded per-connection
  thread spawn is itself a denial of service. `MAX_CONCURRENT_CONNECTIONS`
  bounds it; this test fills every slot with silent connections, confirms
  the next one is closed without a response, and confirms capacity recovers
  once a slot frees up.
- `state::tests::unlock_refuses_a_new_name_once_the_retained_vault_cap_is_
  reached`: this store trusts every `Unlock` unconditionally and never
  validates `vault_name` against a real vault list, so nothing else stopped
  a same-user peer from growing this map without bound by naming an
  unlimited number of distinct vaults across many short, sequential
  connections — a problem the connection cap above cannot see, since each
  request is its own connection, not a concurrency issue.
  `MAX_RETAINED_VAULTS` bounds it.
- `state::tests::unlock_clamps_an_oversized_idle_bound`: a caller-supplied
  `idle_bound_ms` of `u64::MAX` would have defeated the auto-lock sweep
  entirely (the entry would simply never age out). `MAX_IDLE_BOUND` clamps
  it server-side regardless of what the wire value asks for.

Tarpaulin's LLVM engine measures 235 of 267 lines covered (88.0%); the
remainder is mostly client-side I/O error branches that require a genuinely
failing socket to reach, and the post-`fork` half of the detached-spawn
closure runs in a child process coverage instrumentation does not attribute
back to this crate's own measurement.

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_agent_host --all-targets -- -D warnings
```
