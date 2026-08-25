### Added - the cross-track cousin join (HL-C88, HL10 §6.7)

- Add `src/cousins.ts`: `buildCousinIndex` and `cousinsFor` find lessons in other
  Romance tracks that teach a reflex of the same etymon, keyed on `roots:`.
- Exclude the lesson's own language and Latin, take one word per language
  (earliest by reading order), and emit a fixed language order.
- Reach: 76 Spanish lessons; 25 under a single-token headword restriction. Both
  numbers are pinned, because the display rule is still an open decision.

