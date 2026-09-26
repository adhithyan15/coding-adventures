### Changed — a voiced kana is measured as a combination of its parts (HL-C443)

`letter-anchoring.ts` now splits a precomposed voiced kana into its base kana
and the spacing mark a lesson teaches. が becomes か + ゛, and ぽ becomes ほ + ゜.
A voiced kana is a combination, not a new letter, so it counts as written once
its parts have been written. Before this change, が was owed a lesson of its
own even though the reader had already written か and ゛ separately. That had
the gentle-writing rule backwards: one letter at a time, then combinations.

Only U+3099 and U+309A are split. A Devanagari nukta letter (क़) or any other
decomposable letter stays whole.

Measured effect, japanese only:

- unwritten: 5 → 0, so every track in the corpus is now at 0
- cold: 6 → 2
- builds-toward: 32 → 35. The chapter-3 lessons for か, さ and ゛, and the
  chapter-18 ゜, now have a word nearby instead of none.
