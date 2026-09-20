# Language-owned corpus tests

Each language owns its exact corpus assertions in this directory. A low-churn
track may use `<language>.test.ts`. Once agents are working on the same track in
parallel, give it a `<language>/` directory and put each stable concern or chapter
regression in its own `*.test.ts` owner. Keep the top-level test files for
algorithm fixtures and genuine cross-language invariants; do not add another
language's expected totals there.

Malayalam and Hindi are the reference same-language layouts. Their track,
opening/writing, exam, romanization/script-order, and chapter regressions have
independent owners under `corpus/<language>/`. A new chapter regression gets a
new chapter-named test file; it must not recreate the retired flat aggregate or
append to an unrelated concern. Their A1 tests derive totals from the inventory
and prove every mapped probe is actually taught. Hindi's content-budget suite
derives the canonical schema-v2 lesson count while retaining complete
measurement and zero-excess gates. Point-specific audit history stays with the
inventory point's `note`, not in an ever-growing executable comment followed by
hand-edited totals.

The generated continuity and ramp ledger lives in
`core/gentle-ramp-snapshots/<language>.d/{metrics,findings}/`, while modality lives in
`core/lesson-modality/<language>.d/`. The shared assertion helper verifies the
canonical direct owners for one track at a time, while the cross-language suites
prove exact directory and identity closure. Parallel language and metric PRs therefore
update disjoint test and data files.

Exact writing-stage evidence, root-ledger payoffs, and compiled objective
activity IDs belong in the same language-owned test surface. Shared suites may
assert uniqueness, schema validity, aggregation arithmetic, and other genuine
cross-language invariants; they must not contain an exact corpus-wide list or a
literal total that every language PR has to update.
