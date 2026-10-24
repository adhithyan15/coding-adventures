---
category: CI & GitHub Actions
---

# A script inventory change must also update language-ladder's glyph-evidence pins

The Bengali B1 core added nine letter rows to
`code/learning/human-languages/data/scripts/bengali.json` and updated the
human-language-data inventory evidence (`tests/script-inventories/bengali.evidence.ts`),
but the `validate` job failed in a different program: the language-ladder app
pins each script tab's letters too, in
`code/programs/typescript/language-ladder/tests/glyph-evidence/<NNN>-<script>.evidence.ts`
(letter count, the first four glyphs in inventory order, and the cited rows).
The data package's own test selection never runs those files.

Fix: update the app's evidence file with the new count and first glyphs, and say in
its comment which letters the new rows are.

Next time a script inventory gains or loses rows, grep for the script in BOTH
`code/packages/typescript/human-language-data/tests/` and
`code/programs/typescript/language-ladder/tests/glyph-evidence/`, and run
`npx vitest run tests/independentvowels.test.ts` in language-ladder (after its
BUILD's `npm ci` chain) before pushing.
