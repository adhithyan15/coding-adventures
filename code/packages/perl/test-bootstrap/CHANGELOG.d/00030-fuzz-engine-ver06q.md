## [0.01] - Coverage-guided fuzz engine (VER06q)

### Added

- The Perl lane of the Level 2 coverage-guided fuzz engine, spec VER06q, under
  `CodingAdventures::TestBootstrap::Fuzz`: `Limits`, `FailureIds`, `Mutate`, `Bitmap`,
  `Signature`, `Outcome`, `Options`, `Report`, `Handle`, `Executor`, `Engine`, `Hooks`, `Corpus`
  and the `Fuzz` facade (`run`, `run_outcomes`, `executor`, `verdict`, `verdict_for`). It reuses
  `Int`, `Prng`, `Safe::call`, `Safe::seal`, `Canon`, `StrictJson`, `Values` and `Tfv`.
- Conformance: 25 of 25 `fuzz-v1` cases, all 820 of the Go lane's reference vectors (parsed from the
  committed Go source), a negative control for every case and every corruption its op defines
  (every problem names its case id), and a sabotage hook for every engine entry point the
  runner calls.
- Cross-lane pins for the corpus blind spots, produced by running the Ruby lane: 136 mutator rows,
  every interest value and delta, energy rows with exact ties, real `sig_hash` values, TAP lines
  (`a#b`, spaces, `\n`, `\r`, e-acute + U+202E, over 128 bytes with a cut mid-character, an empty
  target), the rejection draw, the minimiser's bisection sequences and three whole campaigns.
- `t/lib/LineCov.pm`, a small line-coverage recorder (no `Devel::Cover`), and `required_capabilities.json`
  declaring `time:read` (the one-shot wall alarm).

### Design (what Perl changes, VER06q sections 4 and 5)

- Coverage is target-reported edges through a per-execution `Handle`; `Devel::*`, `$^P` and `DB::DB`
  are not used in the hot path (10x to 50x slower, one global slot, silently droppable).
- Each execution runs through `Safe::call` under a one-shot `Time::HiRes::alarm`; the caller's own
  alarm is restored. Perl's safe signals are delivered between ops, so one long op (a huge `x`, a
  pathological regex) and an unbounded `eval` loop cannot be interrupted; documented, with an
  out-of-process runner tracked as PT-F14. Memory is cooperative `reserve` accounting only.
- Handle state is inside-out (a private lexical keyed by `refaddr`); a retired, retained or forged
  handle is inert; sentinels are unique blessed objects compared by `refaddr`. Exhaustion is sticky
  both ways and judged after retirement from the sticky flag and a counters-versus-budgets check.
- A replaced or cancelled alarm and a tampered engine are detected, never a pass, and capped at four
  leaked executions in triage, the mutation loop, minimisation and re-verification (`verdict_for` shares
  one tracker). A forked child that returns into the engine exits through `POSIX::_exit`.
- Crashes are classified by `ref`/`blessed`/`reftype` only; frames come from a `__DIE__` recorder that
  never touches the error value.
- Caps are enforced before allocation: 256 MiB retained, 1 MiB inputs, a campaign-wide minimisation
  budget counting rejected candidates, an aggregate `mutate_sequence` work cap of 2^24 zeroed on breach;
  no bignum is built from input bytes.
- Engine functions and `Limits` constants are captured as lexicals at load, no `->can`/`->isa` is used,
  and the new subs are sealed with the existing read-only-glob mechanism.
