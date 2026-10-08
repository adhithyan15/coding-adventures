# Changelog

## Unreleased

- **Per-host request budgets** (D18S S-K5; #13980 P2.6b). Every supervised
  host gets a token bucket, by default a burst of 128 refilled at 64/s.
  It is checked on the injected monotonic clock before a request reaches the
  dispatcher or the pending slot. A request over budget is answered at once
  with `Failed { Unavailable }`, so the wire protocol is unchanged.
  - New API: `RequestBudget` (with `DEFAULT`), `with_request_budget`, and
    `rate_limited_requests(host)` for the audit record.
  - Tests:
    - bucket unit tests: the burst, the exact refill, the cap, a backwards
      clock, a flat-out loop held to the rate, and extreme values;
    - an end-to-end test where the test child's new `FLOOD` mode sends 20
      requests against a budget of 5: 5 are dispatched and 15 refused.
  - Mutation-checked: disabling the bucket fails the flood test.
- **No host can hold the supervisor's thread** (review round 7). Before
  this, a host that never read its stdin could stall every host on a
  blocking write, and one that wrote faster than the supervisor could decrypt
  could keep `refresh` looping.
  - Responses now go through a per-host writer thread, with a queue of 8
    frames. A full queue makes the send fail, and the host is ended.
  - One `refresh` handles at most 64 records per host.
  - Frame length is checked at queue time.
  - Tests: a child that never reads fails within 9 sends and under 5 s;
    out-of-bound frames are refused.
- **What a host leaves behind dies with it** (review round 8).
  - Ending a host now kills its whole session (`kill_session`, from
    `chief-of-staff-spawn-isolation`). Its exit is settled the same way.
    Before this, a descendant holding its stdout kept the reader, and with it
    the supervisor's thread, blocked until the descendant exited. One
    holding its stdin leaked a writer thread.
  - Joining the reader is bounded at 2 s.
  - Startup frames (offer, trust, bindings) also go through the host's
    writer thread, so a host that stops reading during startup cannot
    block the supervisor either.
  - Test: the test host's new `ORPHAN` mode leaves a `sleep` holding its
    stdout. Stopping the host returns promptly, and the `sleep` is dead.
    Mutation-checked: without the session kill, it fails.
- **Review round 9:**
  - The session kill now always runs before the host is reaped, while its
    pid is still its own. `try_reap` sees the exit with `has_exited`
    (`waitid(WNOWAIT)`), kills the session, then reaps. `refresh`, `stop` and
    `hard_kill_and_reap` all go through it.
  - **Readiness deadline.** A host still `Starting` once the bootstrap
    timeout has passed since its spawn is ended, with `BootstrapTimeout`.
    Before this, a host that never sent Ready stayed `Starting` forever.
    Test: the test host's new `NEVER_READY` mode. Mutation-checked.

- **Descriptor isolation at the production agent spawn** (D18S S-I2, S-I3;
  #13980 P2.2). `spawn_verified` now calls
  `chief_of_staff_spawn_isolation::isolate`:
  - the agent's fd 2 is `/dev/null`, where it used to be the daemon's own
    stderr (a terminal, or the journal);
  - on Unix, nothing above fd 2 is inherited, the agent gets its own
    session, and a terminal on fd 0-2 refuses the spawn. On Windows, only
    stderr changes.

  A new end-to-end test leaks a descriptor without `FD_CLOEXEC` in the
  supervisor and checks from inside the agent that it did not arrive, and
  that fd 2 is `/dev/null`.

- Fix a race that masked a child exiting before `Ready` as a clean exit.  When
  `refresh` saw the child had exited, it settled on `Exited` without waiting
  for the stdout reader thread, so an end-of-stream failure the reader had not
  yet queued was never read and `inspect` kept returning `Ok(Exited)`.  This
  failed `wrong_ready_and_exit_before_ready_fail_closed` intermittently on
  macOS CI.  `refresh` now joins the reader and drains its last events before
  finishing the exit, so the failure surfaces (fail closed) every time.
- Carry binding-authorized model-tool catalog discovery through the child stream
  helper and prove the seventh authenticated data-plane operation over the real
  signed-package child pipe.
- Carry a model-returned call through a separate authenticated D18D execution
  exchange and prove the sixth data-plane operation over the real child pipe.
- Carry tool-aware completion turns through the child stream helper and prove the
  fifth authenticated data-plane operation over a real signed-package child pipe.
- Surface an authenticated `Terminate` received during a child data-plane
  exchange as a distinct graceful-termination condition.
- Accept an optional authenticated data-plane dispatcher, retain the exact host
  registration for each owned child, and automatically send validated responses
  without exposing request payloads to the orchestration core.
- Exercise automatic receive, publish, acknowledge, and completion dispatch
  through a real signed-package child process and encrypted cross-platform pipes.
- Add a production durable launch-binding provider backed by the pipeline
  binding store; every launch revalidates registration, channel claims,
  lifecycle, and directional membership before process creation.
- Require an injected manifest-blind launch-binding provider, authenticate its
  channel UUID and Level 1 model bindings before readiness, and fail closed when
  bindings are unavailable or incompatible with the verified package runtime.
- Deliver the exact relevant public package key, trust class, and tier over the
  fresh child session, removing the test child's hard-coded verification key.
- Pass the authenticated package runtime to the single configured host program
  as the reserved final `--package-runtime deno|skill` argument pair.
- Carry bounded correlated channel and completion exchanges over the established
  secure child pipe, with child request helpers and supervisor-side pending-request
  and response hooks.
- Exercise receive, publish, acknowledge, and completion failure through a real
  signed-package child process on the platform-neutral integration path.

## 0.1.0

- Add exact package re-verification before every host spawn.
- Add bounded pipe framing and fresh secure-channel bootstrap.
- Add authenticated readiness, heartbeat, and graceful termination handling.
- Add owned child reaping with hard-kill fallback and drop cleanup.
- Implement the D18 service reconciler's authoritative supervisor contract.
- Own shared keyring and zeroizing identity handles and require movable session sources for daemon composition.
