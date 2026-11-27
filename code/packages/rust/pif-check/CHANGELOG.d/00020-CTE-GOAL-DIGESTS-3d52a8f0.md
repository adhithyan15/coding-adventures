## [0.2.0] - 2026-11-24

### Added

- PIF-003b (VER32 3.6, 9.2, G3 to G6, PIF-020): the canonical term encoding. `cte` computes the **binding CTE** (binder names and info erased, levels syntactic, 1 MiB cap) from the kernel store and the **display CTE** (names and info kept); `Dterm` holds a term as a tree built and emitted with explicit stacks, and `Dterm::parse` reads the display form strictly (one spelling per name and string).
- `goals`: goal digests recomputed from the theorem's type (the record's digest is a hint, a difference is `GoalDigestMismatch`), the goals list (one line per claim: claim id, space, display CTE) with `erase(display) = binding` checked (`DisplayBindingMismatch`), sign-off goal digests and aggregate checked in both directions, the goal-signing aggregate (SHA-256 of the sorted lines `claim-id goal-digest base-environment-digest`), a goal record in a base file refused, and R-GOL-4 repeated from the stored goal term.
- `pif-check --goals <file>`; the report prints each claim's `goal_digest`, whether it was compared with a goals-list line, and `goals_sha256`.
- Tests: the worked example's recomputed digest equals the VER32 4.6 value `bf1fa0a0...`; binder renaming leaves the digest unchanged; a changed level or constant changes it; levels are compared syntactically; the cap is hit when building, emitting and parsing; a depth-30,000 term is handled without recursion; display and binding disagreeing is refused; strict parser corpus; every error variant has a distinct code.

### Changed

- `LoadOptions` gains `goal_constants` (R-GOL-4, gate-fed to the reader from the goals list).
