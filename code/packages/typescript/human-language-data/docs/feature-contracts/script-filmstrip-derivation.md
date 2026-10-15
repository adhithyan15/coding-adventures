# Script filmstrips: derived targets and composition limits

Most filmstrip targets are not declared at all: `figure-targets.ts` derives one
for each `type: writing` lesson on a switched-on track whose headword is one
cited letter. A headword that is a **list** of single letters (`வ, க`,
`ક — ણ — શ`, `в, р`) or a **word** in a script whose letters stand apart
(`はい`) becomes a *sequence* target with `letters: [...]`, drawn as each
letter's own cited strip in writing order, one labelled group per letter.
It prints only if every letter is cited. Words are not composed in
Devanagari (one shared headline), the Arabic family (joined, positional
forms) or Cyrillic (cursive joins), nor when they carry a vowel sign or
virama that no written-order table places, because the per-letter ductus
cannot draw those honestly.

Single-letter writing lessons on the tracks in `DERIVED_FILMSTRIP_SCRIPTS`
(`figure-targets.ts`) need no declaration: each becomes a candidate, and it
is drawn when its letter has a cited ledger entry.
