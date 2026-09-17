## HL-C387 — the Devanagari digits have no data, no lesson, and nowhere to be paid off

`HI-A1-NUM-05` asks for **०१२३४५६७८९** as read on signs and forms. Three
separate things block it, and the third is the interesting one.

### 1. The corpus contains no Devanagari digit at all

A scan of every Hindi lesson returns **zero** occurrences of U+0966–U+096F. Not
one number anywhere in the track is written as a digit.

### 2. The script data has no digits either

`data/scripts/devanagari.json` has `letters` and `marks` and no third group.
There is no `digits` key, so there are no `components`, no `strokeOrder`, no
`penLifts` and no `strokeOrderSource` to write a lesson from. **The same class of
gap as `HL-C385`** (ङ missing from the same file), and worth fixing in the same
pass.

### 3. There is nowhere for the atom to be paid off

This is what stopped the lesson being written straight away. A glyph lesson's
atom is terminal debt unless a later lesson **requires, practises and assesses**
it, and no lesson in the corpus can honestly do that, because **no lesson ever
writes a number in digits**. `HI-C72-rupee` teaches the word **रुपया** and never
shows a price; the form lessons fill in a name field and never a number.

Wiring the atom into one of them anyway would be the overstatement this campaign
keeps correcting — claiming a lesson teaches something it does not exercise.

**NUM-05's own note already says this**, and it is right: *"Both mocks render
times and quantities in words, which conceals the gap rather than closing it."*

### What to do

Two lessons, not one, and in this order:

1. **The digit shapes**, placed after the counting words are complete — 1–10 by
   chapter 21 and 11–20 by chapter 22, so **chapter 23 is the first slot where a
   reader can say every number a digit stands for.** Copy-only unless the data
   gap in (2) is fixed first, following the mark lessons that have no sourced
   stroke order.
2. **A short reading lesson that meets digits in the wild** — a price, a house
   number, a platform number — which is the payoff for (1) and is also most of
   what `HI-A1-SCR-22` (abbreviations and symbols on signs) is asking for.

Any invented sign must carry no name that could belong to a real business,
school or person: a number and a common noun, nothing more.

**Do not write (1) without (2).** A digit lesson on its own adds a terminal atom
and teaches a shape the track then never uses again, which is the opposite of
what the ramp is for.
