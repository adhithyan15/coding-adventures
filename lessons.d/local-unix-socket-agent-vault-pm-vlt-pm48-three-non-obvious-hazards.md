# Local Unix-socket agent (vault-pm VLT-PM48): three non-obvious hazards

Building a permission-checked local-agent IPC transport (Unix domain socket
under a per-user runtime directory, `SO_PEERCRED`/`getpeereid` peer
verification, a detached long-lived process spawned from a one-shot CLI)
surfaced three bugs that were each surprising in isolation and easy to miss
in review.

1. **A generic "walk every ancestor with `O_NOFOLLOW`" private-directory
   helper is the wrong tool for a path rooted at the *system* temp
   directory.** On macOS both `/tmp` and `/var` are themselves
   platform-placed symlinks (`/tmp -> private/tmp`). A helper that resolves
   `/`, then `var`, then `folders`, ... with `O_NOFOLLOW` at every step
   rejects the walk with "unsafe object type" before it ever reaches the
   directory the code actually owns — even though nothing attacker-controlled
   is involved. Fix: trust the *parent* (opened via ordinary path resolution,
   no `O_NOFOLLOW`) and defend only the *leaf* directory this crate itself
   creates, with `O_NOFOLLOW` + owner/mode verification on that one component.
   General rule: a "walk-and-verify-every-ancestor" directory helper is
   correct only for a root the crate owns end to end; a root anchored under a
   platform-managed directory (system temp, `/var/run`, etc.) needs a
   leaf-only variant, and the platform ancestry is trusted the same way
   `ProjectDirs`' own resolved roots already are.

2. **A bare `connect()` succeeding is not proof a Unix-domain-socket server
   is actually serving requests.** A listener that has stopped calling
   `accept()` (e.g. mid-shutdown, before its process has fully exited) can
   still have a connection sit in its kernel backlog long enough for a
   `connect()` from a *different* process to succeed. A "is another instance
   already running" check built on bare `connect().is_ok()` intermittently
   misreports a dying server as live — reproduced as a real, non-flaky-once
   race between `agent stop` immediately followed by `agent start`, and
   independently as CI-parallelism-sensitive flake in a test that bound,
   dropped, and immediately rebound a raw `UnixListener` in the same
   process (passed alone, ~20% failure rate under `cargo test`'s default
   thread count). Fix: the liveness check must be a real bounded
   request/response round trip (send `Ping`, wait for `Ok` with a timeout),
   never a bare connect.

3. **An "opportunistically reuse a cached credential" feature and a
   "sensitive command should always re-collect its own credential fresh"
   feature will silently interact if the second command's collection path
   is not explicitly exempted from the first's reuse seam.** Wiring
   `passphrase rotate`'s *current*-passphrase prompt through the same
   opportunistic-agent-reuse helper every other authenticated command uses
   made rotation silently skip its first prompt whenever a long-lived agent
   already had that vault unlocked — an E2E test that scripted rotate's two
   expected prompts then hung for the test suite's full 60-second read
   timeout waiting for a prompt that opportunistic reuse had made not
   happen. The fix was a deliberate carve-out, not a protocol change:
   `passphrase_rotate` never consults the cache, matching this codebase's
   pre-existing "the interactive shell refuses to delegate `passphrase
   rotate` at all" rule for the same underlying reason (a rotation's whole
   point is proving fresh knowledge of the current secret). General rule:
   before wiring a new "skip re-collecting this value" seam through an
   existing multi-prompt/multi-step command, check whether any step of that
   command is a *deliberate* re-confirmation ceremony rather than an
   ordinary unlock, and audit the E2E test itself when a PTY-based test
   hangs at a `read_until` for a prompt — a hang there just as often means
   "the prompt legitimately stopped happening" as "the process is stuck."
