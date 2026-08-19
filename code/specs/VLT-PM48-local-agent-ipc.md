# VLT-PM48 — Local Agent, Permission-Checked IPC, and Auto-Lock

## Status

Normative Phase 1B contract for the local `vault-pm` agent process, its
Unix-domain-socket transport, and the pre-emptive auto-lock it enables.
Windows named-pipe support is explicitly deferred; see §9.

## 1. Purpose and boundary

VLT-PM00 §14.5 names this slice directly:

> Phase 1B adds a local user agent over a permission-checked Unix-domain
> socket or Windows named pipe. The agent is optional; one-shot operation
> always remains. No master password is accepted through argv, an
> environment variable, command history, URL, or config.

`VLT-PM40-cli-interactive-shell.md` already delivered the *foreground* half
of that promise — a process that stays open and retains a passphrase between
commands — and stated plainly what it deferred:

> A pre-emptive timer that locks an idle session while it waits is
> deliberately **not** delivered here. It requires either a background thread
> holding secret material or a non-blocking terminal read loop, and VLT-PM00
> §23 schedules auto-lock with the Phase 1B local agent (item 12).

This document is that agent. It adds no new *capability* to the product in
the sense VLT-PM40 means it — every command the agent makes faster is a
command that already exists — but it adds two things the shell structurally
cannot: a passphrase retained *across separate one-shot processes* instead of
within one foreground process, and a *real* pre-emptive idle timer, because a
background agent has no terminal read to block on.

### 1.1 What is in scope

- A `vault-pm agent` process, started detached and stopped explicitly, that
  retains one passphrase per vault name in memory.
- A Unix-domain-socket transport, bound at a short, deterministic,
  owner-private path, that every ordinary one-shot command can opportunistically
  reach.
- Peer-credential verification on every accepted connection — the hard
  requirement VLT-PM00 §14.5 states and this document's title repeats.
- A real background sweep that wipes a retained passphrase once its
  `auto_lock_seconds` bound elapses, whether or not any command asks for it
  in the meantime.
- `vault-pm agent start|stop|status|unlock|lock` and the hidden
  `run-foreground` verb `start` re-executes this binary as.

### 1.2 What is out of scope

- Windows named-pipe support (§9).
- Any change to what a command *does* once authenticated. The agent changes
  where a passphrase came from, never what happens after unlock.
- A session cache. See §3 for why the agent retains a passphrase, not an
  unlocked session, mirroring VLT-PM40's own choice and reusing its argument
  almost unchanged.
- Automatic, implicit registration of a passphrase into the agent merely
  because a one-shot command happened to collect one. The agent is populated
  by exactly one path — `agent unlock` — never as a side effect of an
  ordinary command succeeding. See §4.3.
- Browser extension / native-messaging reuse of this transport. VLT-PM00 §18
  names that as a later consumer of "local agent lifecycle and
  permission-checked IPC"; this document's transport crate is deliberately
  free of any vault-pm-application dependency so that reuse remains possible,
  but no browser-facing protocol is defined here.

## 2. Package placement

```text
code/packages/rust/vault-pm-agent-protocol   bounded wire format; no I/O
code/packages/rust/vault-pm-agent-host       socket transport, peer-credential
                                              check, retention store, detached
                                              spawn; the new host-side capability
code/packages/rust/vault-pm-local-host       extended: runtime_root() and
                                              agent_socket_path()
code/packages/rust/vault-pm-cli              src/agent.rs — command surface,
                                              lifecycle handlers, and the
                                              `passphrase_for` opportunistic-
                                              reuse seam
```

Two new packages, not one. `vault-pm-agent-protocol` is pure serialization —
it depends on nothing but `coding_adventures_zeroize` and performs no I/O —
because VLT-PM00 §17 lists "local agent lifecycle and permission-checked IPC"
as a desktop-specific responsibility later, and a wire format with zero
dependency on sockets, threads, or this product's own crypto stack is the
one artifact both the CLI agent and a future desktop agent can share without
either pulling in the other's concerns. `vault-pm-agent-host` is the actual
transport and lifecycle: it depends on the protocol crate and on
`vault-pm-local-host` for the socket path, and on nothing from
`vault-pm-application` — see §4.2 for why that absence is load-bearing, not
incidental.

The agent process itself is not a new binary. `agent start` re-executes the
same `vault-pm` executable as `vault-pm agent run-foreground`, detached — the
same pattern `VLT-PM46-cli-clipboard.md` §4.3 already established for the
timed clipboard clear, for the same reason: a one-shot composition root that
must occasionally outlive its own invocation is simpler as one binary with an
internal re-exec than as a second program crate with its own build,
packaging, and crash-injection exclusion story.

## 3. Why the agent retains a passphrase, not a session

`VLT-PM40-cli-interactive-shell.md` §3.1 already made this argument for the
foreground shell, and it applies unchanged here:

> A `VaultAccessV1` session pins the repository heads it observed, and every
> access and mutation boundary consumes the session by value precisely so a
> stale pin cannot be reused after the repository has moved on.

An agent-retained *unlocked session*, shared across many separate one-shot
processes, would be worse than the shell's version of this problem: at least
a shell's session lives inside one process that runs its commands serially.
An agent would have to hand the *same* pinned session to concurrently
running one-shot processes, or maintain a pool of them, and either design
reintroduces the exact stale-pin hazard VLT-PM05's consumption rule exists to
prevent — now across process boundaries instead of within one.

So, exactly as the shell does, the agent retains the smallest thing that
removes the repeated prompt: the passphrase itself, wipe-on-drop, keyed by
vault name. Every command that reaches it still performs its own complete
verified open and obtains fresh pinned heads. The decrypted-vault exposure
window per command is identical to the one-shot window; only the
authenticator outlives any single command, now across process boundaries
rather than merely across commands inside one shell.

The trade this makes is the one VLT-PM40 §7.1 already named and accepted for
the shell: an attacker who can read this process's memory recovers the
master passphrase, not one vault's derived keys. Nothing about running the
retention store in a separate long-lived process makes that trade worse — the
passphrase was already the asset at risk the moment `vault-pm shell` shipped
— and the peer-credential check in §4.1 is what keeps that risk scoped to
"this same local user's own memory," not "any local process that can open a
socket."

## 4. IPC transport

### 4.1 Permission model — the hard requirement

VLT-PM00 §14.5 and this document's title both call the permission check
non-optional. Two independent layers apply, and neither is treated as
sufficient on its own:

1. **Filesystem.** The socket's parent directory is created and verified
   owner-only (`0700`) by `vault-pm-local-host`'s existing private-directory
   machinery, and the socket file itself is `chmod 0600` immediately after
   `bind`.
2. **Peer credentials.** Every accepted connection is checked against the
   kernel's own record of who actually opened it — `SO_PEERCRED` on Linux,
   `getpeereid` on macOS and the BSD family — *before* a single byte of the
   request is read. A mismatched or unavailable peer UID gets no response at
   all: not a typed rejection, not a protocol error, nothing. See
   `coding_adventures_vault_pm_agent_host::peer` for the full argument for why
   filesystem permissions alone are not trusted: a permissive umask, a
   misconfigured parent directory, or a filesystem that does not enforce Unix
   permissions the way the local host expects can all leave a `0600` file
   reachable by another local user, and the kernel-verified UID is the one
   fact about a connection that none of those can misrepresent.

Root is granted no special case. An agent started as an unprivileged user is
not reachable by root through this check, matching every other owner-only
object `vault-pm-local-host` already creates (config, writer lock, object
root).

### 4.2 The agent never verifies a passphrase

`vault-pm-agent-host`'s retention store (`state::AgentState`) trusts every
`Unlock` request it receives unconditionally. It has no dependency on
`vault-pm-application`, `vault-pm-repository`, or any cryptographic package in
this product — it cannot verify a passphrase even if it wanted to.

Verification happens once, in the CLI process handling `agent unlock`
(`vault-pm-cli::agent::agent_unlock`), *before* the passphrase is ever sent
to the agent: it performs the same authenticated open every other command
performs (`open_authenticated_access`'s own unlock step, reused for its
verification side effect and then immediately locked again), and only a
passphrase that already opened the real vault is handed to the agent for
retention.

This is a deliberate divergence from `ShellSession`'s own lazier model.
VLT-PM40 §3.3 retains lazily and validates only when a delegated command
actually uses the value, failing closed (`ShellSession::lock`) on rejection.
That is correct for a session confined to one person's one terminal: a wrong
passphrase costs one failed command, and the same person who typed it is
right there to retype it. An agent-cached value is different in kind — it is
read by every subsequent one-shot process on the machine, not by the next
line typed at one prompt — so seeding it with an unverified value would let
one mistyped passphrase poison every opportunistic lookup for the rest of its
idle bound, discovered only when some unrelated later command fails. Paying
one real unlock's cost once, at `agent unlock` time, buys a cache that is
never *wrongly* populated.

`passphrase rotate` is the one authenticated command that never consults the
agent for its current passphrase, always prompting fresh — see the doc
comment on `vault-pm-cli::passphrase_rotate` for the argument, which mirrors
`VLT-PM43-cli-passphrase-rotation.md` §3.1's reason the interactive shell
refuses to delegate `passphrase` at all.

### 4.3 Opportunistic reuse, and the fallback that makes it safe

Every other authenticated command funnels its passphrase collection through
one function, `agent::passphrase_for`:

```text
passphrase_for(host, paths, vault_name):
    if a running agent answers GetPassphrase(vault_name) with a value:
        return it
    else:
        return host.read_existing_passphrase()   # the ordinary terminal prompt
```

Every branch that is not "the agent is running, reachable, and already holds
an unexpired passphrase for exactly this vault" falls through to the
unmodified one-shot prompt. This is VLT-PM00 §2 requirement 4, restated as
code: one-shot operation is unconditionally correct with no agent present,
and this function can only ever make a command *skip a prompt*, never change
what the command does or what it is willing to accept.

The lookup itself (`vault-pm-agent-host::client::cached_passphrase`) is a
single bounded round trip (§4.5) that collapses every failure — no socket, a
stale one, a timeout, a malformed response — to `None`. It is the one
function in that crate permitted to discard an error rather than propagate
it, because discarding is exactly the fallback behavior this requirement
demands.

The agent is populated by exactly one path, `agent unlock`. No ordinary
command that happens to collect a passphrase from the terminal ever pushes it
into a running agent as a side effect. That asymmetry is deliberate: silently
promoting every one-shot unlock into a standing, cross-process credential
would be a surprising widening of what "run one command" does, and would
make the feature's exposure window a property of *whether an agent happens to
be running* rather than something a person opted into by name.

### 4.4 Self-healing a stale cache

Two situations make an agent's cached passphrase for one vault definitely or
possibly wrong after it was correctly retained:

1. **`passphrase rotate` succeeds.** The old passphrase is now certainly
   wrong. `passphrase_rotate` forgets that vault's cache immediately on
   success (`agent::forget_cached_passphrase_on_rejection`), so the very next
   opportunistic lookup falls back to a prompt for the *new* passphrase
   rather than trying, and failing with, the one that no longer works.
2. **Any command comes back `Locked`.** This might have used the agent's
   cache (a stale value, for example after an out-of-band rotation on
   another device) or might not have — `execute`'s post-dispatch check cannot
   tell, and does not try to. It unconditionally tells the agent to forget
   that vault, mirroring `ShellSession`'s own in-process self-heal
   (`dispatch`'s `if output.exit_code() == ExitCode::Locked { session.lock() }`,
   VLT-PM40 §3.4 rule 2). Both branches of this check — "the agent was the
   source" and "it was not, or nothing is running" — are harmless no-ops, so
   the check runs unconditionally rather than threading a "did this command
   use the agent" flag through every dispatch path.

### 4.5 Wire protocol

`vault-pm-agent-protocol` defines one bounded, hand-rolled binary frame
format — this workspace does not carry a general-purpose serialization
dependency, and every other wire format in this product line (`vault-pm-format`,
`vault-pm-cli-host`'s `ClipboardClearRequest`) is an exact, hand-checked byte
layout rather than a derived one. A frame is a 4-byte big-endian length
prefix followed by a payload of `[version byte][tag byte][fields]`; the
length is checked against a fixed ceiling *before* the payload buffer is
allocated, so a peer cannot force an unbounded allocation merely by claiming
one in the prefix.

Six requests, five responses:

```text
Ping                                    -> Ok
Unlock { vault, passphrase, idle_ms }   -> Ok
GetPassphrase { vault }                 -> Passphrase(bytes) | NotRetained
Lock { vault: Option<name> }            -> Ok
Status                                  -> Status([{ vault, remaining_ms }])
Shutdown                                -> Ok
(anything malformed)                    -> connection dropped, or Err(code)
```

Every carried vault name is restricted to the exact character set
`vault-pm-config::ConfigName` already enforces — ASCII alphanumeric, or
`_`/`-` past the first byte — checked on both encode and decode, not merely
bounded in length and required to be valid UTF-8. This closed a real finding
from this feature's own security review: the socket's peer check
authenticates only "the same local user," never "the genuine `vault-pm`
binary," so any same-user process can write a hand-crafted frame directly to
the socket. A name outside `ConfigName`'s character set would otherwise have
been retained verbatim and later rendered — unescaped — into `agent
status`'s plain-text and `--json` output, corrupting the JSON or carrying
raw terminal control sequences to whoever read it. Restricting the
character set at decode time closes this at the one place every renderer of
a vault name already trusts, rather than requiring every present and future
render site to escape correctly.

Every request/response round trip from the CLI side
(`vault-pm-agent-host::client`) is bounded by a fixed timeout
(`DEFAULT_TIMEOUT`, 1.5 seconds). A slow or wedged agent must fail exactly as
fast as a missing one — see §4.3 — so no client function in that crate
blocks indefinitely, retries, or waits on anything the caller did not
explicitly ask for (`wait_until_ready`, used only by `agent start`).

One connection carries exactly one request and one response. There is no
multiplexing and no keep-alive: the client connects, sends a frame, reads a
frame, and disconnects. This keeps the shutdown path simple, at the cost of
one connect/disconnect per opportunistic lookup — an acceptable cost against
a `AF_UNIX` socket on the same machine, and simpler to reason about than a
protocol that has to cancel an in-flight request out from under a client.

Every accepted connection is handled on its own short-lived thread rather
than serially in the accept loop — also a security-review fix, not the
original design. A serial accept loop would let any process running as this
same local user (already this transport's trusted-reachability boundary,
never its availability boundary) starve every legitimate caller by opening
one connection, sending nothing, and repeating: the per-connection read/write
timeout (`server::CONNECTION_TIMEOUT`, 2 seconds) bounds how long *that one*
connection can block, but a serial loop would still let it block every
connection queued behind it for the same span, indefinitely. One thread per
connection removes that head-of-line dependency; `AgentState`'s `Mutex`
already serializes the only state the threads actually share.

Every buffer that can carry a passphrase in plaintext — an encoded `Unlock`
request, an encoded `Passphrase` response, and every frame `transport::
read_frame` reads, since the transport layer cannot know in advance which
frames are secret-bearing — is `Zeroizing`, wiped on drop, the same
discipline every other passphrase buffer in this product already follows.
This closes the third security-review finding: `AgentState`'s own retained
copies were always wipe-on-drop, but the plaintext copies that necessarily
exist for the few milliseconds a passphrase is actually in transit were, in
the first draft, ordinary un-scrubbed heap buffers.

## 5. Auto-lock state machine

Each retained passphrase carries a `retained_at` instant and an
`idle_bound`, mirrored from `vaults.<name>.auto_lock_seconds` (VLT-PM07,
default 300) at the moment `agent unlock` sends its `Unlock` request. The
agent tracks this per vault name, independently:

```text
              Unlock(name, idle_bound)
                       |
                       v
     +----------------------------------------+
     |  retained: passphrase, retained_at      |
     +----------------------------------------+
       |  GetPassphrase(name)         |  background sweep tick
       |  before idle_bound elapses   |  after idle_bound elapses,
       |                              |  or explicit Lock(name)/Lock(None)
       v                              v
   Passphrase(bytes)              forgotten (removed from the map)
```

Two things distinguish this from `ShellSession`'s own bound, both because the
agent is a background process rather than a foreground one blocked on a
terminal read:

1. **A real background sweep**, not only a check at the point of use. The
   server runs a dedicated thread that wakes on a short fixed interval and
   removes every entry whose bound has elapsed, whether or not any client
   ever asks about it. This is the pre-emptive timer VLT-PM40 §3.5
   explicitly deferred to this document, because delivering it there would
   have required either a background thread holding secret material inside a
   foreground process, or a non-blocking terminal read loop — both out of
   scope for that slice, and both exactly what a dedicated background
   process is for.
2. **A monotonic clock, not the advisory host wall clock.** `ShellSession`
   measures elapsed time against an injected, advisory wall-clock reading
   because it must interoperate with a host trait every other method of
   which already uses wall time, and because a shell foreground process has
   no reason to prefer anything else — which is why VLT-PM40 §3.4 rule 4
   has to spend real space defending against a clock that reads unreadable
   or steps backwards. `AgentState` uses `std::time::Instant` instead, which
   removes that whole class of defense: a wall-clock step (an NTP correction,
   a manual clock change) can never make a retained passphrase in this store
   look fresher than it is, because nothing about its expiry check consults
   wall time at all.

Every `GetPassphrase` answer double-checks expiry at the point of use in
addition to the background sweep, for the same reason `ShellSession
::authenticator` does: a request that lands in the gap between two sweep
ticks must not be handed a passphrase that has, technically, already expired.

## 6. Command surface

```text
vault-pm agent start
vault-pm agent stop
vault-pm agent status [--json]
vault-pm [--vault NAME] agent unlock
vault-pm [--vault NAME] agent lock
vault-pm agent run-foreground        (hidden; the detached process itself)
```

`start`, `stop`, and `run-foreground` act on the agent process, not one
vault, and refuse a `--vault` selector for the same reason `password
generate` and `clipboard clear` do (VLT-PM44 §2.2, VLT-PM46 §2.1): the
selector would be a statement with no referent. `unlock` and `lock` are
vault-scoped and accept one, defaulting to the configured `default_vault`
exactly as every other authenticated command does. `status` accepts one
optionally, to narrow its report to a single vault.

`start` is idempotent: starting an already-running agent reports `Agent:
already running.` rather than failing. `stop` and `lock` are idempotent in
the other direction — stopping or locking an agent (or vault) that was
never running, or never unlocked, reports success, the same "repeating this
is harmless" contract `ShellSession::lock` and the interactive shell's own
`lock` verb already promise.

`agent unlock`'s liveness check (`AgentServer::bind`'s "is something already
listening here") sends a real `Ping` and waits for a real `Ok`, rather than
trusting a bare `connect()` succeeding. A listener mid-shutdown can still
have a connection sitting in its kernel backlog for a brief window after it
stops calling `accept`, and a bare connect would misread that window as "an
agent is already running" — reachable in practice by an `agent stop`
immediately followed by `agent start`.

### 6.1 The interactive shell refuses the whole noun

`vault-pm shell` refuses every `agent` verb, not only the ones that would be
individually unsafe. Most of them would be harmless inline — `agent unlock`
in particular would simply reuse the session's already-retained
authenticator instead of prompting again — but `agent run-foreground` is the
long-lived accept loop `agent start` re-executes this binary as, and running
it inline would block the session's own command prompt forever: the same
category of mistake a nested `shell` already is. One rule covering the whole
noun is easier to state and to keep correct than a rule that allows some of
its verbs and not others.

## 7. Socket path

`LocalVaultPaths::runtime_root()` resolves to a short, deterministic
directory beside the system temporary directory —
`vault-pm-<16 hex chars>`, where the suffix is a truncated SHA-256 digest of
the data root — rather than a subdirectory of `data_root` or the platform's
`XDG_RUNTIME_DIR`. Both alternatives were considered and rejected:

- **Nested under `data_root`.** `sockaddr_un.sun_path` is bounded to roughly
  100 bytes on Linux and macOS alike, and a verbose platform data directory
  (macOS's `~/Library/Application Support/...`, for one) can already consume
  most of that budget on its own before a socket filename is appended.
- **`XDG_RUNTIME_DIR` via `ProjectDirs::runtime_dir()`.** This resolves to
  one fixed path per login session, independent of which data root a given
  `LocalVaultPaths` was built from. Two installations sharing one login
  session — or two sandboxed test homes constructed in the same test
  process — would silently collide on one socket and one agent. Hashing the
  data root instead means every distinct installation gets its own runtime
  directory using only the roots this crate already receives, with nothing
  new to keep in sync, and it is what makes this feature fully testable
  through the same `HOME`/`XDG_*`-sandboxing every other `vault-pm-local-host`
  test already uses.

The directory is verified or created lazily
(`PreparedLocalVault::ensure_runtime_root`), not by every ordinary
`prepare()` call: a command that never touches the agent must not reach into
the system temporary root as a side effect of doing something unrelated.
Its ancestor path (the system temp directory) is trusted as the platform
gives it — the same trust `LocalVaultPaths::resolve` already extends to
`ProjectDirs`' platform roots — while the leaf directory itself is opened
with `O_NOFOLLOW` and its owner and mode verified exactly as every other
private root this crate resolves. See
`vault-pm-local-host::unix::ensure_private_runtime_directory`'s doc comment
for the full argument, including why the general-purpose
`ensure_private_directory` (which walks every ancestor from `/` with
`O_NOFOLLOW`) is the wrong tool here: on macOS both `/tmp` and `/var` are
themselves platform-placed symlinks, and the unresolved walk would reject
them before this crate's own directory is ever reached.

A parent directory that is world-writable without a sticky bit could let
another local user pre-create the runtime directory first, denying the
feature to whoever loses the race. That race is refused, not silently
accepted: an existing directory owned by another user fails closed with
`InsecureOwner`. The residual is a denial of the agent feature to whichever
party loses it, never a disclosure — no key, passphrase, or socket
connection is granted merely because a directory exists, and this is the
same posture `vault-pm-local-host` already takes toward a foreign-owned lock
file or config file.

## 8. Testing

`vault-pm-agent-protocol` and the pure parts of `vault-pm-agent-host`
(`state::AgentState`, `transport`) are unit-tested without a socket, a
thread, or a clock reading anything but an injected `Instant`.

`peer::is_same_user`'s comparison logic is tested against a fabricated
mismatched UID (`authorized(peer_uid, own_uid)`, a pure function taking both
values as arguments) rather than against a real connection from a second
local user, which a CI runner cannot provide without a second real account
and a setuid helper. The positive case — a real connected `UnixStream` pair
reports this process's own UID as the peer — is exercised directly.

`vault-pm-agent-host::server` is tested against real bound sockets in the
system temporary directory: bind/accept/dispatch round trips for every
request, a second bind refusing while the first is live, a stale socket
from an unclean exit being reclaimed, a non-socket path being refused rather
than deleted, and an oversized or garbage connection being dropped without a
response while the server keeps serving everything after it.

The end-to-end proof lives in `code/programs/rust/vault-pm-cli/tests/
local_cli_e2e.rs`, through the real executable: `agent start`, one real
`agent unlock` on a real pseudo-terminal, a subsequent authenticated command
run with *nothing at all* on its controlling terminal that still succeeds
(proving the prompt was actually removed, not merely that the agent
answered), `agent lock` restoring the prompt, and `agent stop` tearing the
socket down cleanly. A second test proves the passphrase-rotation self-heal:
after a real rotation, the next command prompts for the *new* passphrase
rather than silently retrying the stale cached one.

Every PTY-based test in that file uses the bounded-timeout read helpers
(`poll_pty_readable`, 60-second ceiling) and drops the spawning `Command`
promptly after `spawn()`, per that file's own hazard note: a leaked `Command`
keeps its copy of the PTY slave alive, which keeps a `drain_pty` read waiting
for a hang-up the real child already produced. The "no prompt occurs" test
in particular depends on this: if a regression reintroduced a required
prompt, the child would block reading `/dev/tty` (which nothing in that test
ever writes to) and the test fails loudly on the same bounded timeout, rather
than hanging the suite or passing for the wrong reason.

A real cross-UID rejection test — a second local account actually
connecting and being refused — is not exercised in CI, which has no such
second account available. This is recorded as an accepted residual, not a
silent gap.

## 9. Explicitly deferred

**Windows named-pipe support.** `coding_adventures_vault_pm_agent_host`
compiles on every target — its socket-touching modules
(`client`, `lifecycle`, `peer`, `server`) are `#[cfg(unix)]`, and every
public function `vault-pm-cli::agent` exposes has a `cfg(not(unix))` twin
that returns the closed `Unsupported` exit class without referencing them.
`vault-pm-local-host::ensure_private_runtime_directory` exists for Windows
today only as a thin reuse of the general-purpose recursive walk, since
nothing on that platform binds a socket yet; a real named-pipe
implementation should revisit it the same way the Unix build's leaf-only
check diverged from the general one (§7), because `%TEMP%` can also involve
reparse points that walk has not been proven against. This is a scope
decision stated once, here, rather than a claim of cross-platform support
this PR has not built and then quietly not tested.

**Desktop reuse.** `vault-pm-agent-protocol`'s independence from
`vault-pm-application` is deliberate preparation for VLT-PM00 §17's later
desktop agent, but no desktop-specific protocol extension, capability
negotiation, or biometric-gated unlock path is defined here. A future
desktop slice inherits a transport and a permission model, not a finished
feature.

**Multiple simultaneous vaults' auto-lock policy diverging per socket.**
Every vault an agent has ever unlocked is tracked in the same process with
its own `idle_bound`; there is no per-agent global override, no policy that
locks every vault when one expires, and no attempt to unify multiple running
agents (one agent process, one socket, one runtime directory per data root —
see §7). Multi-agent coordination is not a scenario Phase 1B's local,
single-installation product needs to solve.

## 10. References

- `VLT-PM00-local-first-password-manager.md` §14.2, §14.5, §17, §18, §23 item 12
- `VLT-PM07-config.md` — `auto_lock_seconds`
- `VLT-PM40-cli-interactive-shell.md` — the session-vs-authenticator argument
  this document reuses, and the pre-emptive-timer deferral this document
  fulfills
- `VLT-PM43-cli-passphrase-rotation.md` §3.1 — the reason `passphrase rotate`
  never delegates to a retained or cached authenticator
- `VLT-PM46-cli-clipboard.md` §4.3 — the detached self-re-exec pattern
  `agent start` reuses
- `VLT-PM08-cli-host.md` — controlling-terminal secret collection, unchanged
  by this document
