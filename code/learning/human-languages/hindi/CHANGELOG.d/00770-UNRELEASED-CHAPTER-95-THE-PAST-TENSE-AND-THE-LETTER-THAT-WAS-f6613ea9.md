## Unreleased — chapter 95: the past tense, and the letter that was blocking it

Hindi A1 exam coverage **209/282 (74%) → 211/282 (75%)**, closing two points.

| point | what was missing |
|---|---|
| `HI-A1-V-12` | the past copula *thā / thī / the*, and the past habitual |
| `HI-A1-POST-05` | *par* for location at a point |

### The reason there was no past tense was one letter

`HI-A1-V-12`'s note reads *"Nothing in the corpus is past-tense."* Four hundred
lessons and ninety-four chapters, and no way to say where you had been. That
looks like something nobody got round to. It was not.

Hindi's past copula is **था / थी / थे**, and the letter **थ** had never been
taught anywhere in the track — not in any of the twenty-five letter lessons, not
in passing. `measureScriptClosure` counts a glyph as taught only where a script
lesson shows it, so every past-tense sentence would have asked the reader to
decode something they had never been given.

Nine lessons were already carrying **थ** words — **हाथ**, **माथा**, **थाली**,
**चौथा** — and four of them were live closure violations.

The legal shortcut was available: a headword with a declared `romanization` is
exposure rather than load-bearing, so the chapter could have romanized its way
round the problem. That would have produced a chapter whose central word the
reader could not read, and left the nine existing violations untouched.

**`HI-S126-letter-tha` teaches the letter instead.** It slots into the existing
recognition chain after `HI-S124-letter-dha`, at chapter 20, and its stroke
order was already sitting sourced and unused in `data/scripts/devanagari.json`.
Placement matters: closure is measured in reading order, so a letter taught at
chapter 20 clears every violation after it. Teaching it at chapter 95 would have
cleared one instead of four.

It also closes a row the reader had nearly filled. **त**, **द** and **ध** were
already taught; **थ** was the one empty seat in the dental four, and the lesson
teaches it as exactly that:

| character | sound | puff of breath |
|---|---|---|
| त | ta | no |
| थ | tha | yes |
| द | da | no |
| ध | dha | yes |

### The past tense costs one substitution

Every lesson in the chapter is the same move made once:

| now | earlier |
|---|---|
| मैं घर पर हूँ। | मैं घर पर था। |
| वह स्टेशन पर है। | वह स्टेशन पर था। |
| मैं हिंदी बोलता हूँ। | मैं हिंदी बोलता था। |

The third line is the past habitual, and it is **the present habitual the track
has taught since chapter 5 with its final word swapped**. **-ता** carries the
habit and the copula carries the time; they were never welded together, which is
why the past costs a substitution rather than a new conjugation.

The three endings are **-आ / -ई / -ए**, which is the alternation the track has
been bending adjectives with since the colours. What is new is *which* thing the
verb listens to:

| tense | listens to the person | listens to gender |
|---|---|---|
| present | yes (हूँ, है, हैं) | no |
| past | no | yes (था, थी) |

Neither tense asks the reader to track both at once. **आप** takes **थे** even
for one person, for the reason it takes **हैं** for one person — the polite
level is built out of plural forms — and the past is the one system where *तुम*
and *आप* do **not** split, which is one fewer decision in the place a reader
would expect another.

### par was a lump, and the paper prints it

`HI-A1-POST-05`'s note says mock 1 reading item 14 and mock 2 item 10 both turn
on it. Item 14 is **मैं घर पर हूँ।** — a sentence of four words, three of them
taught, and the untaught one was **पर**.

The word existed in the corpus only inside the chapter-94 phrase *हम स्टेशन पर
मिलेंगे*, carried as part of a lump. `HI-C87-par` gives it on its own and states
the ordering that makes the whole class learnable: **the small word trails its
noun**, which is what *post*position means. The synthesis lesson then reads item
14 back word for word and takes one step: *मैं घर पर था।*

### One forward reference, and it is HL-C379 again

`forward-language` **12 → 13**. The single entry is `HI-C68-lekin` showing
**पर**, 112 lessons before `HI-C87-par`.

It is a false positive of the exact shape `HL-C379` describes. The *lekin*
lesson prints Sanskrit **पर** meaning *other, beyond* — the ancestor of the
conjunction *par*, "but" — which is a **different word** that happens to be
spelled the same as the postposition. The detector has no word sense, so a
same-language homonym reads as an early use.

Not worked around. The alternative was to decline to introduce an exponent two
mock items turn on, because a known-broken detector produces noise.

### Numbers

| metric | before → after |
|---|---|
| script-closure violations | 38 → 35 |
| never-taught glyphs | 11 → 10 |
| `forward-language` | 12 → 13 |
| `measurement-blind` | 8 → 9 |
| writing-practice lessons | 99 → 100 |
| atoms taught | 457 → 464 |
| atoms never revisited | 35 → **35** |

`measurement-blind` is the one new *síntesis* lesson. HL-C377 again; not
relabelled.

**Atoms never revisited held flat on purpose.** The letter lesson's own atom
would have been terminal debt — taught at chapter 20 and retrieved nowhere — so
`HI-C87-thaa` requires and practises `HI-SCRIPT-RECOG-126`, asking the reader to
name the character carrying the first sound of **था**, the one they met in
**हाथ**. The letter lesson now pays off seventy-five chapters later, which is
what it was for.

Reinforcement misses rose **1039 → 1071**, in step with nine new lessons.

### The rest of the alphabet is now a work queue

Backlogged as `HL-C381`. Ten glyphs remain shown-but-untaught, and eight of the
ten already have sourced stroke data. The expensive one is **ऊ**: `HI-A1-V-11`
wants the future tense, both mock papers depend on it in the first scored part,
and the third-person **आएगा** is writable today while the first-person
**आऊँगा** is not. The same audit put **उ** and **ढ** in front of the daily
routine (`HI-A1-F-59`) and **ौ** in front of **और**.

### Verification

`human-language-data` full suite; all twelve `check:*` gates;
`check-book-compile.sh hindi` under XeLaTeX.

### What this does not claim

Coverage means the teaching exists, not that a reader scores. No track has had a
single exam item graded against it.
