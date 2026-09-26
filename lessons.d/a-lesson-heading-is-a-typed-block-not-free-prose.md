---
category: Repo policy / workflow reminders
---

# A lesson heading is a typed block, not free prose

Fourteen of HL-C439's first-draft headings were rejected by `validate` with
`heading '<title>' has no stable block type`. Every one of them named its
content directly — `पेट — the trail that stops`, `The four exchanges, by what
each one does`, `Why this page exists` — which is exactly what makes a heading
good and exactly what the parser refuses.

`classifyBlock` in `src/parse.ts` maps a level-two heading to a block type by
**prefix match on the lowercased title**, and schema v2 rejects the `unknown`
fallback outright. The accepted openings are a closed list:

```
Warm-up / Warmup          You'll want to know...     Sounds you'll need...
Script...                 ...letters in this word    Writing...
Across the family...      ...taken apart...          Reading...
Why it's said this way... Grammar Lens...            Guided Practice
How to answer...          Wrap-up Recall...          What you've built...
The exchange...           The two words...
```

The prefix carries the type and the rest of the heading is yours, so the fix is
mechanical once you know the list: `The word, taken apart: पेट`,
`The exchange — four of them, by what each one does`,
`You'll want to know: why this page gives you no cues`. A block whose content is
register or pragmatics has a home too — `Why it's said this way — three
farewells, three distances`.

Two structural rules ride alongside it: the FIRST block must be `warmup` and
the LAST must be `recall`. Nothing constrains the middle.

Read `classifyBlock` before writing the headings. Choosing the prefix first
costs nothing; retrofitting fourteen of them costs a validator round trip each.
