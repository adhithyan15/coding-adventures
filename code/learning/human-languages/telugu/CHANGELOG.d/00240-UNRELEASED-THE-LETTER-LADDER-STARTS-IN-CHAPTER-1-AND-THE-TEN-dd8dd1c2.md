## Unreleased — the letter ladder starts in chapter 1, and the ten numerals are taught

Telugu's script-closure violations fall **22 → 6** and its never-taught glyphs
**13 → 0**. Both numbers come from `report-cli`'s `script closure` line;
corpus-wide the violation count moves 576 → 560, and every glyph of that
movement is Telugu's. Load-bearing headwords stay at **0**.

### The ladder now opens on chapter 1, lesson 2

HL-C217 measured the ceiling on the previous approach: every surviving
violation sat at or before chapter 19 while the first script lesson sat in
chapter 6, and a script lesson can only teach the lessons that follow it. So
this tranche moved the ladder rather than extending it.

**క is now taught immediately after నమస్కారం**, the book's first word, which
contains it. **ా** follows two lessons later and **ే** closes the chapter. The
chain was reordered so the marks that unlock reading come first — ా, న, ు, ్,
ి, ం before chapter 3 — instead of following the alphabet's own order:

| chapter | script lessons now taught there |
|---|---|
| 1 | క · ా · ే |
| 2 | న · ు · ౧ · ్ · ౨ · ి · ం |
| 3 | ౩ · ఉ · ౪ · ర · ౫ |
| 4 | డ · ౬ · ప · ౭ |
| 5 | త · ౮ · స · హ · ౯ |
| 6 | ౦ · య |

Nothing was cut. Every letter lesson that used to sit in chapters 6–20 is still
in the book; sixteen of them moved earlier, and each had its example words
pruned back to words the reader has actually met at the lesson's new position.
`TE-S123` (డ) lost four example words and kept one; `TE-S125` (హ) lost సహాయం,
which now arrives on the page after it, and lost a sentence pointing forward at
మధ్యాహ్నం nine chapters ahead.

### The ten numerals ౦ ౧ ౨ ౩ ౪ ౫ ౬ ౭ ౮ ౯, taught two per chapter

Ten new lessons, `TE-S140` … `TE-S149`, spread across chapters 2–6 so they never
bunch. They carry **no Telugu word**, on purpose: a numeral is read as a
quantity, so its shape can be learned months before the spoken name arrives in
chapter 7 — and holding it back until then is exactly what left ten characters
untaught for the whole book. ౦ comes last and pays the set off: it is the ring
the reader already knows as the sunna ం, and with it ౧౦ and ౨౦ and ౧౦౦ become
readable, because Telugu numerals use place value the same way the reader's own
digits do.

Both number lessons in chapter 7 are now clean.

### Six new letters, four of them a family

| lesson | letter | placed at | first met in |
|---|---|---|---|
| `TE-S134-vowel-sign-uu` | ◌ూ | ch. 7 | మూడు |
| `TE-S135-letter-i` | ఇ | ch. 9 | ‑ఇంచు |
| `TE-S136-vowel-sign-ai` | ◌ై | ch. 16 | చైత్రం, వైశాఖం |
| `TE-S137-letter-kha` | ఖ | ch. 16 | వైశాఖం |
| `TE-S138-letter-gha` | ఘ | ch. 17 | మాఘం |
| `TE-S139-letter-ddha` | ఢ | ch. 18 | ఆషాఢం |

ఖ, ఘ and ఢ were the last three never-taught glyphs, and teaching them together
turns what used to be an isolated pair (బ/భ) into a rule the reader can
predict: a plain consonant and the same consonant with a puff of breath. `థ`
moved from chapter 34 to chapter 33 — after అర్థం, the word that needs it, and
before `TE-C33-raayu`, which used to ask for it untaught — and now names the
whole family instead of claiming త was the first letter the book ever taught,
which it no longer is. ఇ is taught as the pair to the sign ◌ి the reader
already reads, which is the rule that makes an abugida legible: sign after a
consonant, letter when the vowel starts.

### Script atoms now return three times instead of once

A `TE-SCRIPT-RECOG-*` atom used to be revisited exactly once, by the next letter
in the chain, and then never again — against an owner directive of "reviewed
constantly," and against ~14 appearances for a lexical atom. Each script lesson
now recalls its predecessor and the letter **four back** in its warm-up, and the
letter **twelve back** in its wrap-up, which lands inside the corpus's own R1,
R2 and R3 windows at both the dense and the sparse ends of the ladder.

Telugu's script-atom reinforcement misses fall **103 → 68** while the atom count
rises 33 → 49 — 3.12 → 1.39 misses per atom — with R2 misses 32 → 7 and R3
misses 31 → 11. Mean lessons per script atom rises **1.94 → 3.65**. R4 is still
unserved and is recorded in HL-C218.

### What else moved

Script-ramp glyph spikes fall **5 → 3**. Chapter atom-budget violations stay at
0 and forward references stay at 10, under the pinned ceiling of 12. The
handwritten `.tex` for chapters 1–5 gained twenty-four sections, declared in
`core/book-generation.d/handwritten.d/telugu-000{1..5}.json` as
`embeddedLessonIds` so the book cannot silently drop a lesson the curriculum
teaches. Twenty-four inherited uses of the banned word *just* left the script-lesson
template.

Six violations remain. All six are lessons that are themselves the corpus's
first sighting of the glyph they fail on, so no honest script lesson can precede
them; the remedy is vocabulary sequencing, and HL-C218 names each one.

