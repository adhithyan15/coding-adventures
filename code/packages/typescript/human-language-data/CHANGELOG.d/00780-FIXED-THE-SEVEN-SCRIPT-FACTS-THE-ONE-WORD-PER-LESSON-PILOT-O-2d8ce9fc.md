### Fixed — the seven script facts the one-word-per-lesson pilot orphaned

Splitting Tamil chapter 1 moved letter-by-letter reading out of the word lessons and
into the writing track, on the grounds that the writing track already taught those
letters. For **seven of them it did not**: ஆ, இ, ச, ர, ல, the ை sign, and the rule
that *a word opening on a vowel opens with a full vowel letter, not a sign*. The
chapter recap still tested all of it.

Three writing lessons close the gap, each teaching its letters and then assembling the
word they spell:

| lesson | teaches | writes |
|---|---|---|
| `TA-W05-write-aam` (230s) | **ஆ**, and the word-initial vowel rule | ஆம் |
| `TA-W06-write-illai` (233s) | **இ**, **ல**, the **ை** sign, three *l*'s | இல்லை |
| `TA-W07-write-sari` (241s) | **ச** as s/ch/j, **ர**, two *r*'s | சரி |

They sit at sequences 182, 192 and 222 — **after** the word each one spells, so the
learner meets a word by ear and then learns to write it. That is the opposite of the
pre-existing `TA-W04` / நன்றி inversion, where the writing lesson taught how to write a
word 30 sequence-steps before the word itself was introduced.

Each computes to 230–241 seconds under `estimateLessonDuration`, which puts them
with the writing lessons they sit among (TA-W01–W04 compute 176–297s) rather than
with the word lessons, which run 88–136s. That split is the point: a word lesson
is short because it holds one word, and a writing lesson is longer because the
hand is slower than the ear. All three declare `max_seconds: 260`, above their
computed cost, so the effective figure is the declared one and nothing is
silently absorbed by `max(declared, computed)`.

Two things the review caught that are worth recording. **ை was described backwards** —
it is written to the *left* of its consonant and pronounced after it, which the
project's own `data/scripts/tamil.json` and `TA-W04` both already said. The wrong
version had reached the narration script. And **ச and ர are taught for reading only**:
this project gives a stroke order only where it has a sourced one, and those two have
none, so the lesson says so rather than inventing strokes.

