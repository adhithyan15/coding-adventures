# The super-gentle cross-language queue (HL17)

```bash
npm run report:gentle-ramp
npm run report:gentle-ramp -- --format json
```

The completion plan measures the climb to the assessment target. This smaller
report answers a different question: **where will the learner hit the next cliff?**
It joins duration, atom and glyph load, order, forward language, script closure,
writing, retrieval and chapter payoffs for every active track.

The output intentionally has no composite score. A row retains its own unit —
seconds, lessons, glyphs, dependency edges or retrieval windows — so a severe
opening defect cannot be averaged away. Writing exposure does not count as writing
practice, and the report names how many opening lessons precede the first real
writing/script lesson or block.

The exact report is derived on demand from canonical curriculum sources:

```bash
npm run report:gentle-ramp         # render the current learner-first queue
npm run check:gentle-snapshots     # derive every track and reject snapshot resurrection
```

The former 851 metric/finding owners were exact generated copies of those same
sources, not independent policy. They are retired: the compatibility checker is
read-only, derives all registered tracks, and rejects any case-fold variant of
`core/gentle-ramp-snapshots`. Hard invariants and authored budgets remain in their
source gates; reporting-only counts can now change without a generated-file edit.
See [`HL38`](../../../../../specs/HL38-retired-gentle-ramp-snapshots.md).

Level coverage is computed on demand from the canonical lessons, curricula, and shared
spine by `summarizeLevels()`. The level tests prove exact registered-track closure,
per-track and corpus arithmetic, unmapped accounting, and highest reached level. There
is deliberately no tracked `core/level-snapshots/` copy: it duplicated a fully derived
fact and forced two independent tranches in the same language to update one rollup.
The tests reject resurrection of that retired generated directory.
