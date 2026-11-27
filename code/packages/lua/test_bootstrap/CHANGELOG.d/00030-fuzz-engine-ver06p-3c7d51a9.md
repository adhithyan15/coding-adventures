## 0.2.0

- Add the Lua lane of the coverage-guided fuzz engine (VER06p): `Fuzz` in
  `coding_adventures.test_bootstrap`, modules `fuzz/{limits,mutate,bitmap,signature,failure_ids,
  outcome,options,report,handle,executor,engine,hooks,corpus}`.
- Conforms to the shared `fuzz-v1` corpus (25 of 25) and replays the Go lane's 820 reference
  vectors; cross-lane pins from the Ruby lane cover unequal splices, every interest value and
  delta, flips with 1,000-digit counts, energy ties, real `sig_hash` values, the rejection draw,
  minimiser candidate sequences, TAP token edge cases and three whole campaigns.
- Coverage is target-reported edges through a per-execution `Handle`; line hooks are rejected for
  the hot path (spec 4.2).
- Each execution runs in a fresh coroutine under a count hook (instruction budget, `os.clock()`
  wall bound, best-effort `collectgarbage("count")` memory budget); coroutines created by the
  target through `coroutine.create`/`wrap` get the same hook. Handles are retired at the end of
  the execution; retired, retained and forged handles are inert; sentinels are unique tables
  recognised by identity; exhaustion is sticky both ways and judged after retirement.
- A target that yields or clears the hook is a crash (`yielded`, `hook-cleared`) counted as a
  leaked execution; four aborts the campaign, in triage, mutation, minimisation and
  re-verification (`verdict_for` shares the tracker).
- Errors are classified by `type()` only; frames are `basename#line` from the dead coroutine.
- Caps: 1 MiB inputs, 256 MiB retained, campaign-wide minimisation budget counting rejected
  candidates, `2^24` aggregate `mutate_sequence` work zeroed on breach; flips are byte-wise and
  build no big integer from input bytes.
- `required_capabilities.json` declares `time:read` (`os.clock`).
- Documented limits: an unbounded `pcall` loop, a single long C call and a coroutine created through
  a pre-captured `coroutine.create` cannot be stopped in-process.
