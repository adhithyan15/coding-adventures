# Language-owned corpus tests

Each language owns its exact corpus assertions in this directory. Put a new
language-specific regression in `<language>.test.ts`, even when it is only one
test. Keep the top-level test files for algorithm fixtures and genuine
cross-language invariants; do not add another language's expected totals there.

The generated continuity and ramp ledger lives in
`core/gentle-ramp-snapshots/<language>.json`, while modality lives in
`core/lesson-modality/<language>.json`. The shared assertion helper verifies
both shards from one track at a time, so parallel language PRs update disjoint
test and data files.
