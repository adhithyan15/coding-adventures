# Language-owned corpus tests

Each language owns its exact corpus assertions in this directory. Put a new
language-specific regression in `<language>.test.ts`, even when it is only one
test. Keep the top-level test files for algorithm fixtures and genuine
cross-language invariants; do not add another language's expected totals there.

The generated continuity and ramp ledger lives in
`core/gentle-ramp-snapshots/<language>.d/{metrics,findings}/`, while modality lives in
`core/lesson-modality/<language>.d/`. The shared assertion helper verifies the
canonical direct owners for one track at a time, while the cross-language suites
prove exact directory and identity closure. Parallel language and metric PRs therefore
update disjoint test and data files.

Exact writing-stage evidence, root-ledger payoffs, and compiled objective
activity IDs belong in the same language-owned test file. Shared suites may
assert uniqueness, schema validity, aggregation arithmetic, and other genuine
cross-language invariants; they must not contain an exact corpus-wide list or a
literal total that every language PR has to update.
