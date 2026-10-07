### Fixed — no chapter payoff is left as an empty schema-v1 placeholder

Ten chapters had already been migrated to typed (schema-v2) lessons while their
ledger entry still carried the schema-v1 placeholder: `payoff.assesses: []`, or
a half-filled list, with a note saying representativeness stayed authored
"until migration". The representativeness gate scored the empty lists 0/N, so
the placeholder silently held whole tracks off the clean list (#12088, #12301).

- **Payoffs written** from the atoms each closing checkpoint actually assesses
  (its block-level `hl-knowledge` directives, in lesson order), and every stale
  note replaced with one that says what is and is not scored:
  - hindi 3, 4, 5 — 4/7, 4/6 and 5/6 (were 0/7, 0/6, 0/6);
  - italian 1 — 20/20 (was 0/20); portuguese 1 — 18/18 (was 0/18);
  - malayalam 2, 3, 4, 5 — 20/24, 10/11, 10/12 and 16/19 (were 11/24, 3/11,
    3/12 and 9/19: only the letter atoms had been named);
  - russian 1 — 8/18 (was 0/18). `RU-C01-practice` now also declares the
    formal/informal greeting register its recall and exchange already ask
    for. It stays below the 0.5 floor, and its note says why: the other ten
    atoms are Cyrillic letters the writing runway introduces after the recap.
- `payoffsNotRepresentative` falls from 94 to 85, and **italian** and
  **portuguese** join the clean-track list.
- The 19 `chapters.d/_meta.json` ledger notes no longer describe empty
  `assesses` lists as intentional.
- Russian chapter 1 book, narration, modality owner and both hash manifests
  regenerated; no other generated artifact changed (`payoff.assesses` and
  `payoff.note` are not printed).
- Tests: `chapters.test.ts` adds italian and portuguese to the pinned clean
  list, and a new corpus test fails if any payoff has an empty `assesses` list
  or a note still waiting "until migration". It fails on the pre-fix ledger
  (hindi 3-5, italian 1, portuguese 1, russian 1).
