---
category: CI & GitHub Actions
---

# A prelude change must re-run gen_evidence --check across every adopter on the latest main right before it merges; adopters merged during its CI are otherwise left stale

**What went wrong.** WD-HARN1 (#19530) changed `evidence_prelude.rs.txt` and regenerated every adopter's `tests/generated/warrant_evidence.rs` that existed on its branch. While it was in CI, #19539 merged `vos-sched-fp-bitmap`, whose harness had been generated with the old prelude. Neither PR was wrong on its own base; their union was: `gen_evidence --check vos-sched-fp-bitmap` failed on main, and the first PR to rebuild that package would have failed for a reason unrelated to its change.

**The fix.** Regenerate the stale harness with `gen_evidence` (no hand edit) in its own commit, against the unchanged prelude, and confirm `evidence/` does not change and the package's evidence script gives the same result as before.

**Do differently.** A change to a shared generator template is only as current as the adopters it saw. Right before it merges, fetch and merge the latest main and run `cargo run -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check` with no package argument (it covers every directory with an `evidence.bindings`, and a per-package loop shows every stale one instead of stopping at the first). Regenerate what is stale in the same PR. Keep template edits inside a feature region (`//@if obs`) so only that region's adopters can go stale.
