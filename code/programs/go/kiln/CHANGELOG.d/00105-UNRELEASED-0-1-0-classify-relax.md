## [0.1.0] - UNRELEASED

### Changed

- `internal/traced` (KILN04 5.5): three narrow hit-rate relaxations of the
  strict stability rule, each applying only when its precondition provably
  holds and staying strict otherwise.
  - **Opens settle the final name.** For a successful open whose `-y` path
    equals the walked path, the final component is no longer noted as a
    followed final or a missing stop: the kernel's own path proves it was not a
    link at that moment. Intermediate components keep the full rule.
  - **Open-only change logs.** A followed final name of a successful `stat` or
    `exec` whose whole change log is `O_CREAT` opens keyed by their `-y` path,
    optionally ending in one `unlink` that starts after the last lookup
    completed, is accepted when the first open completed before the first
    lookup started and the name was never a hard-link endpoint. This is the
    compiler output that is stat'ed later, and the temporary that is created,
    read and unlinked.
  - **`$HOME` itself.** A stat or listing of the `Home` directory node is
    allowed even when the checkout lies beneath it (hosted runners); nothing
    under it is.
- Two tests that asserted the refused patterns (`TestExecOfFileTheActionMade`,
  `TestFuzzFindingsRemovedOrReplacedLinks`) now assert the relaxed result; the
  boundary tests are in `relax_test.go`.
- The differential fuzzer gains "bursts": the patterns above issued on one
  name by two processes with random overlap, plus perturbations (a rename onto
  the name, a hard link, a symlink, an exchange, a second unlink). A seeded
  deterministic run of the oracle (`TestDifferentialSeeded`) joins the normal
  test run.

### Fixed

- An `O_PATH` open is a plain observation (the kernel ignores `O_CREAT`; with
  `O_NOFOLLOW` it opens a link itself), and `O_NOFOLLOW` opens are never
  `O_CREAT` log entries, so they cannot satisfy the open-only or removed-name
  rules. The removed-name rule now covers any stopped component, and only
  clean creations and keyed `O_CREAT` opens count as "made". Listing `$HOME`
  stays allowed (spec 5.5(c)). The differential model gains `O_PATH` and
  `O_NOFOLLOW` open modes.

- A successful followed call (stat, chdir, chmod, ...) on a name the post-run
  disk shows absent, that the action never made, is now `unstable-path`. The
  stability rule had a hole: stat through an untracked leftover link, then
  unlink the link, left the link's target unobserved. Found by the new
  "leftover replaced" burst of the differential fuzzer; the fix is fail-closed.
