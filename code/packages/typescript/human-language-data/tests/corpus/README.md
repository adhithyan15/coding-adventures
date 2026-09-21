# Language-owned corpus tests

Each language owns its exact corpus assertions in this directory. A low-churn
track may use `<language>.test.ts`. Once agents are working on the same track in
parallel, give it a `<language>/` directory and put each stable concern or chapter
regression in its own collision-resistant `*.case.ts` owner. The flat file then
becomes a tiny, stable `import.meta.glob` entrypoint: adding a regression creates
one owner without editing a shared manifest. The loader sorts owners and rejects
missing, unsafe, case-fold-colliding, or linked files before importing any case.
Keep the top-level test files for algorithm fixtures and genuine cross-language
invariants; do not add another language's expected totals there.

Malayalam and Hindi are the original same-language layouts. Their track,
opening/writing, exam, romanization/script-order, and chapter regressions have
independent owners under `corpus/<language>/`. A new chapter regression gets a
new chapter-named test file; it must not recreate the retired flat aggregate or
append to an unrelated concern. Their A1 tests derive totals from the inventory
and prove every mapped probe is actually taught. Hindi's content-budget suite
derives the canonical schema-v2 lesson count while retaining complete
measurement and zero-excess gates. Point-specific audit history stays with the
inventory point's `note`, not in an ever-growing executable comment followed by
hand-edited totals.
Hindi and Malayalam A1 point-specific audit history now lives with the
corresponding direct owner under `core/exam-inventory-<language>-a1.d/`; chapter
agents must not recreate either retired aggregate inventory.

The ten other Indian tracks use the stricter discovered-owner layout described
above. Their top-level entrypoints must stay declarative; chapter agents add a
new hashed case owner and never append executable assertions to the entrypoint.

Continuity and ramp reports are derived directly from each track's canonical
lessons and authored policy. The shared assertion helper retains stable hard
invariants—complete ordering, dependency safety, measurement arithmetic, and the
one-writing-system step—without copying current counts into generated owners.
Modality remains a consumer artifact under `core/lesson-modality/<language>.d/`.

Exact writing-stage evidence, root-ledger payoffs, and compiled objective
activity IDs belong in the same language-owned test surface. Shared suites may
assert uniqueness, schema validity, aggregation arithmetic, and other genuine
cross-language invariants; they must not contain an exact corpus-wide list or a
literal total that every language PR has to update.
