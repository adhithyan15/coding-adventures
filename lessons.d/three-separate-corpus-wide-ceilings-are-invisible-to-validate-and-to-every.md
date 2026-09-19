---
category: Testing & coverage
---

# Three separate corpus-wide ceilings are invisible to validate and to every check script, and only the full vitest suite holds them

Two consecutive Malayalam chapters tripped three different ceilings. In every
case `npm run validate` returned 21/21, all twelve `check:` gates returned OK,
and `check-book-compile.sh --strict` compiled. The failure appeared only under
`npx vitest run`, after the work was written.

| test | what it holds | what tripped it |
|---|---|---|
| `chapter-references.test.ts` | cross-chapter prose references per track | a chapter that cited earlier chapters by number, 46 → 69 |
| `script-closure.test.ts` | `neverTaughtGlyphs === 0` for Malayalam | one word needing a character in no headword and no script-lesson example |
| `info-dump.test.ts` | `ruleStatements` corpus-wide | one sentence opening "The rule is general" |

**They are not the same kind of check as the gates.** The twelve `check:` scripts
verify that generated artifacts match their sources — they re-derive and compare.
These three measure a *property of the corpus as a whole* and pin it against a
recorded number. New writing can move such a number without any artifact being
stale, so nothing in the generate/check cycle notices.

**Each one's comment states the posture, and it is the same posture: a ceiling
that may fall and never grow.** The fix is therefore never to raise the number.
For `info-dump` the test's own history shows the method — four separate tranches
pushed it, and each kept only the statement whose *entire lesson is a rule*,
rewriting the incidental ones. For `chapter-references` the prescription is in
the test: name the thing, not the number. For `script-closure` it is to teach the
letter, in reading order, before the word that needs it.

**What to do differently.** Run the full suite **before** writing the chapter
note and the changelog, not after — all three of these were found after those
records had been written, and each one then needed the record corrected too.
Cheaper still, check the three properties directly while drafting:

```
grep -cE "[Cc]hapter [0-9]+" <new lessons>          # must add zero
grep -E "\bthe rule (is|for|here)\b" <new lessons>   # and the other RULE_PATTERNS
```

and for any character that looks unusual, grep every headword **and** every
script lesson for it before committing to the word.
