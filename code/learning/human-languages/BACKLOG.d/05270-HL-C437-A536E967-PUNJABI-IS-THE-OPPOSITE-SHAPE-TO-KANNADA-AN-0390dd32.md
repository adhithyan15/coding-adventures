## HL-C437-a536e967 — Punjabi is the opposite shape to Kannada and a terminal lesson's windows are uncountable until something follows it

**Status: CLOSED (2026-09-23) — implemented.** The fourth of the large
reinforcement debts.

```
punjabi: reinforcement 40 -> 0   (six lessons, one new chapter)
```

**Nineteen of twenty-three tracks** now carry no pre-A1 reinforcement debt.
Punjabi goes three ladder blockers to two.

### I HAD THIS TRACK'S CONSTRAINT WRONG IN MY OWN BRIEF

The HL-C436 check-in said punjabi was "the one remaining track with real index
pins (6 test files)". **It has none.** That number came from a grep matching
`toHaveLength(`, which I then wrote down as fact and carried forward two
tranches. The six files it matched are property assertions over named lesson
sets — durations, `introduces.knowledge` empty, skills present — none of which
an appended lesson can disturb, plus one `toEqual([])` that is monotone in the
work's favour.

The real constraint is a **272-row session map**: one row per lesson, sessions
numbered contiguously, with `lessonId` *and* `chapter` both asserted equal to
canonical sequence order. Appending at the end made it a six-row addition rather
than the renumbering HL-C432 needed for gujarati.

The lesson is the one HL-C432 already recorded, applied to my own notes rather
than to someone else's test: **a count from a grep is not a finding.** Open the
files.

### THE OPPOSITE SHAPE TO KANNADA

Kannada's 41 atoms spanned 24 chapters but **one** path segment, so a single new
chapter reached all of them cheaply. Punjabi's 40 span **seventeen** segments,
ranks 110 to 470, sequences 230 to 2160 — no concentration anywhere:

```
kannada  41 atoms / 24 chapters /  1 dominant segment  -> cheap
punjabi  40 atoms / 18 chapters / 17 segments          -> expensive
```

Same headline numbers, opposite cost. **Group by segment before designing**, and
do not infer the shape from the chapter count in either direction.

Because nothing could be placed in among the material, chapter 48 is appended at
the end in three strands — the form-field ladder, the Gurmukhi pieces, the
courtesy and parting words. Three segments rather than one because a lesson's
`spine_node` must equal its segment's, and the three strands serve three
different nodes.

### A TERMINAL LESSON'S WINDOWS ARE UNCOUNTABLE, NOT CLOSED

The whole-track R-window summary moved `{55, 132, 217, 107}` →
`{55, 135, 215, 90}`, and diffing the `(atom, window)` pairs decomposes it
exactly:

```
CLOSED  8 R3 + 22 R4 = 30   the six lessons retrieving forty atoms from
                            chapters 4-36 at the far end of the book
OPENED  3 R2 + 6 R3 + 5 R4 = 14   and NOT ONE belongs to a lesson this tranche
                            added: phone fields, pronouns, question words,
                            connected reading -- late-chapter atoms whose
                            windows did not EXIST at 272 lessons and do at 278
```

That second half is this pin's own earlier comment running in reverse. It
recorded that the timed writing paper, being last, "practises five form atoms,
opening a window for each that nothing after it can close", and predicted that
**"the next lesson added after it will pay part of it back."** It did — the 22
closed R4 windows — while simultaneously making fourteen previously uncountable
windows countable and therefore missed.

**Appending to a track always does both**, and the net is not the story. A bare
count would have shown R4 falling seventeen and hidden that fourteen windows
opened on material this tranche never touched.

### THE FOUR-PART CHAPTER REGISTRATION WENT RIGHT FIRST TIME

HL-C436's `lessons.d` entry paid for itself immediately: chapters.d record,
book-generation target, then regenerate books and narration, in that order, and
`check:shards` was green on the first run. The only track-specific difference is
the target's script fields — punjabi uses `unicodeScript` / `scriptCommand`
where kannada uses `scriptSet`, so copy the neighbouring target rather than the
last track's.

### Remaining

```
hindi 44   malayalam 44   tamil 73   arabic 78
```

239 atoms across four tracks. **Arabic still wants its own entry**: 146 slots,
68 of 78 with no later revisit at all, a missing review layer rather than debt.
