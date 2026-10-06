---
category: Testing & coverage
---

# A stroke-data change is pinned outside script-ductus and human-language-data too

**What went wrong.** The Telugu native-lift batches changed `penLifts` and
`strokeOrder` in `data/scripts/telugu.json`. They passed the full script-ductus
and human-language-data suites and every `check:*` gate. CI then failed twice,
in two places none of those runs covered:

- `code/programs/typescript/language-ladder/tests/glyph-evidence/*.evidence.ts`
  pins per-script glyph evidence, including the independent vowels' lift
  counts, and still expected the old values.
- `code/learning/human-languages/data/scripts/generate_syllabary.py` declares
  some Dravidian rows explicitly in `VERIFIED_INDEPENDENT_VOWELS`. An explicit
  declaration wins over the committed row, so
  `test_committed_outputs_are_regeneration_stable` failed: a regeneration
  would have restored the old lifts.

**The fix.** Update the ladder pins after checking the new values against the
cited evidence. Make the generator's explicit rows equal the committed rows,
field for field.

**What to do differently.** Before pushing any change to stroke data (records,
ductus, lifts) run all of these as well:

```bash
(cd code/programs/typescript/language-ladder && npx vitest run)
python3 code/learning/human-languages/data/scripts/test_generate_syllabary.py
python3 code/learning/human-languages/data/scripts/test_sharded_ledger.py
```

When working in a secondary git worktree, do not symlink a package's whole
`node_modules` from another checkout. Its relative `@coding-adventures/*`
links then resolve into that other checkout, so the tests silently read the
wrong tree's lessons and data. Link each dependency on its own, and point the
`@coding-adventures/*` entries back into the current worktree.
