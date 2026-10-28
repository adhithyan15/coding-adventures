---
category: Testing & coverage
---

# A docs-only merge conflict can still carry a semantic change from the oracle a gate companion mirrors

When PR #18351 (`cortex-m7-gatelevel`) was brought up to date with main, git
reported conflicts only in CHANGELOG/README/backlog text. But main had also
merged #18343, which changed the *functional* Cortex-M7 simulator: `MSR
APSR_nzcvq` now preserves GE, and the GE-writing masks (`APSR_g`,
`APSR_nzcvqg`) are executed by the M7 layer because the inherited Cortex-M3
core refuses them. The gate companion still wrote GE on every APSR `MSR`, so
its oracle differential (`special_register_and_it_paths_match_the_checked_oracle`)
went red after a merge whose conflicts looked harmless. The masks `01`/`11`
would also have faulted in the gate while the oracle executed them.

The fix was a code change, not a conflict resolution: decode the mask bits
through gate networks, qualify the GE mux by bit `g` and the NZCVQ mux by bit
`nzcvq`, and give the companion its own path for the masks the inherited gate
engine refuses.

**Do instead:** when merging main into a branch whose tests compare against an
oracle crate, list main's commits that touched the oracle (and the oracle's
own dependencies) since the merge base — `git log <base>..origin/main --
<oracle paths>` — and rerun the differential tests after the merge, even when
every conflicted file is documentation.
