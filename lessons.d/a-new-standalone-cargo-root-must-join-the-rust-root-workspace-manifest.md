---
category: BUILD files & dependency management
---

# A new standalone cargo root must join the rust-root-workspace-manifest-cross-consumers boundary and re-pin its digest

The vos-capacity-probe builder concluded no build-tool-v1 boundary change was needed. That holds for a crate added as a workspace member (vos-sched-rr), but not for a standalone cargo root under `code/programs/rust` that runs cargo against the workspace, as vos-link (#19529) and vos-capacity-probe do. CI's "Repo-wide metadata contracts" job caught it in `test_repository_source_input_boundary_is_closed_and_canonical`.

The rule: when you add a standalone cargo root, sync the boundary in the same PR: the JSON fixture, the Go, C# and Swift projections, the pinned `boundary_sha256` at every site (`git grep` the old digest), the scope and authorization counts, and a `CHANGELOG.d` shard. Run `code/scripts/tests/test_build_tool_conformance_runner.py` locally before pushing (it needs `jsonschema==4.26.0` and Python 3.10+). The root list is sorted by UTF-8 bytes, so insert it in sorted position, not at the end.

Also: CI clippy runs 1.100.0 (`dtolnay/rust-toolchain` pin in ci.yml); an older local toolchain misses `needless_range_loop` findings. `rustup toolchain install 1.100.0 -c clippy` and lint with it.
