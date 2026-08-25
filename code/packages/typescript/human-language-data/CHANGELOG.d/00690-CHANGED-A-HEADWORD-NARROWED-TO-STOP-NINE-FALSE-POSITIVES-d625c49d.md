### Changed — a headword narrowed to stop nine false positives

The connective lesson was first written with the multi-word headword **`así que`**.
`taughtWords` strips a leading word of ≤3 characters — a heuristic meant for articles —
so it registered **`que`** as first taught at chapter 41 and flagged all nine earlier
lessons containing it. `que` is genuinely taught at chapter 7.

The headword is now **`así`**, the only word that is actually new, with the gloss
carrying the `que`. `forwardReferences` holds flat at 524. The loader heuristic is the
real bug and is recorded as such in the pin comment.

