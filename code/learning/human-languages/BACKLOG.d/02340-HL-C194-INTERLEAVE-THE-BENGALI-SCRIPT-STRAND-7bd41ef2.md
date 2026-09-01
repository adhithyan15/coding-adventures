## HL-C194 — Interleave the Bengali script strand into the content chapters

Bengali's script strand is now four chapters long — 16 through 19, forty-five
lessons, twenty-six pieces taught — and every one of them sits **after** all
fifteen content chapters. That placement is what keeps `scriptClosureViolations`
pinned at 65 while `neverTaughtGlyphs` falls from 48 to 22 across two tranches:
the closure measurement walks lessons in reading order, so a glyph taught in
chapter 17 is untaught for every lesson in chapters 1–15 that shows it.

The size of the prize was measured before chapters 17–19 were authored, by
replaying `measureScriptClosure` over the Bengali corpus with a hypothetical
pre-taught glyph set:

| scenario | violations | neverTaught |
|---|---|---|
| as shipped | 65 | 22 |
| chapter 16's nine glyphs moved to chapter 1 | 62 | 22 |
| chapters 16–19's twenty-six glyphs taught by chapter 6 | 48 | 22 |

So relocating the existing block alone buys **three** violations and is not
worth a restructure on its own. Interleaving the whole strand — all
twenty-six pieces landing across chapters 1 through 6, one letter every two or
three content lessons, the pace HL11 §4's `minLessonsBetweenScriptSegments`
already asks for — buys **seventeen**, and is the shape rule B describes.

Two obstacles are real and must be planned around rather than discovered:

- **Chapter 6 must not move.** `language-ladder/tests/bookhashes.test.ts` pins
  `["bengali", 6, 1]` — the browser-loaded Chapter 6 AST across one lesson.
  Renumbering chapters 1–15 breaks a test in another package.
- **Payoff representativeness.** Dropping script lessons into an existing
  content chapter adds atoms its payoff lesson does not assess, and
  `payoffRepresentativeness` is a 0.5 floor. Bengali is currently clean on it;
  chapters 1–5 would need their payoffs re-scoped, or the script lessons need
  their own interleaved chapters, which reintroduces the renumbering problem.

The remaining twenty-two never-taught glyphs, ordered by how many Bengali
lessons show each one untaught, are: **য** 25, **়** 23, **ও** 13, **গ** 9,
**ছ** 7, **ৃ** 7, **ড** 7, **ঞ** 5, **ষ** 4, **শ** 4, **ঁ** 3, **ূ** 3,
**থ** 3, **ঝ** 3, **ফ** 3, **ট** 3, **ঙ** 2, **ঃ** 2, **অ** 1, **ং** 1,
**ঠ** 1, **উ** 1.

The head of that list is where a fifth tranche should start, and each of the
five has a word already glossed in chapters 1–15 waiting to pay it off:
**য** and **ঁ** for হ্যাঁ, **ড** and **়** for কাপড়, **ও** for হওয়া, **গ**
for লাগা, **ছ** for আছি. Teaching those seven would take `neverTaughtGlyphs`
to fifteen without inventing a single new word.
