## Unreleased — chapter 100: degree, and two notes that went stale while nobody was looking

Hindi A1 exam coverage **221/282 (78%) → 225/282 (80%)**, closing four points.

| point | what was missing |
|---|---|
| `HI-A1-ADJ-08` | *thoṛā* as a hedging quantifier |
| `HI-A1-ADV-06` | doing something well, and doing it badly |
| `HI-A1-ADJ-09` | comparison — X is bigger than Y |
| `HI-A1-PRON-10` | the exclamative *kitnā acchā!* |

### Two of the four were waiting on work already done

This is a new variety of stale note and worth naming, because the cause is
different from the previous six. These were not wrong when written. **They went
stale because a later chapter removed the blocker and nothing re-read them.**

| point | what the note said | what had happened |
|---|---|---|
| `ADJ-09` | *"Hindi compares with the postposition `se`, which is untaught"* | *se* was taught in **chapter 98** |
| `ADV-06` | *"No word for 'badly' — neither `bura` nor `kharab` — is introduced"* | *burā* was introduced by `HI-C83-buraa` in **chapter 91** |

`ADJ-09` even pointed at the point that would unblock it — *"See HI-A1-POST-04"*
— and `POST-04` closed two chapters ago. The dependency was recorded and nobody
followed the arrow back.

**A note that names its own blocker should be re-read the moment that blocker
closes.** Eight stale or partly-stale notes so far, and this is the first pair
that a search of the corpus alone would not have caught — you have to re-read
the notes of points that *depend* on what you just landed.

### The thread: five words, and every one of them bends

| what you want to do | what you say |
|---|---|
| hedge an answer | थोड़ा / थोड़ी |
| describe how an action went | अच्छा / बुरा |
| measure one thing against another | X से बड़ा |
| ask a quantity | कितना / कितनी |
| admire something | कितना अच्छा! |

**All five end in -आ.** None is a new kind of word — they are adjectives doing
five different jobs, which is why the chapter adds five words and **no new
machinery**.

### Three places English carries machinery and Hindi carries none

| English has | Hindi has |
|---|---|
| good **and** well | one word |
| how much **and** how many | one word |
| bigger, more interesting | the adjective, unchanged |

Hindi has **no comparative form at all**. English runs two machines — *-er* on
short adjectives, *more* in front of long ones — and Hindi runs neither: the
adjective stands still and the thing measured against takes **से**. That is the
same idea as the postposition's other uses, the comparison **proceeding from**
the thing measured against.

The one thing to drill is order. English puts *than X* **after** the adjective;
Hindi puts it **before**, which follows from the verb-final rule the track has
taught since its first sentence.

**से now does three jobs** — *from*, *by way of*, and *measured against* — and
the review lesson sets them side by side.

### The ending is the item

`ADJ-08`'s note names mock 1 listening item 12, which keys on **हाँ, थोड़ी**.
The lesson makes the **ending** the subject rather than the word, because that
is what the item tests: the answer is **थोड़ी** and not **थोड़ा** because
*hindī* is feminine, and the hedge agrees with what it hedges rather than with
the person speaking.

English speakers are caught by this because *a little* is a fixed lump there and
an adjective here. The same trap sits under **अच्छा** meaning *well*: it is
still an adjective, so it **still bends**, where an English adverb never does.

### छ, at last

The largest single glyph debt in the track — **sixteen lessons** — and the
reason chapter 99 had to write its agreement examples with **लंबा** instead of
**अच्छा**.

`HI-S130-letter-chha` teaches it as **the breathy partner of च**, exactly as
**थ** is of **त**:

| character | sound | puff of breath |
|---|---|---|
| च | ca | no |
| छ | chha | yes |
| त | ta | no |
| थ | tha | yes |

Two pairs, one distinction, and the second pair costs less than the first —
which is the point worth making about this script. Unlike *ta* and *tha*, the
two shapes look nothing alike: **the sound is paired, the drawing is not.**

Placed at chapter 21, immediately before **छह** at sequence 690, which is the
earliest **छ** word in the track. Never-taught glyphs **7 → 6**, closure
violations **31 → 30**, and `payoff-surprise` held at **13** — chapter 21 had
the headroom the placement rule asks for.

### Numbers

| metric | before → after |
|---|---|
| atoms taught | 491 → 497 |
| measurable lessons | 432 → 439 |
| writing-practice lessons | 103 → 104 |
| never-taught glyphs | 7 → 6 |
| script-closure violations | 31 → 30 |
| `measurement-blind` | 13 → 14 |
| `forward-language` | **unchanged** |
| `payoff-surprise` | **unchanged** |
| atoms never revisited | **unchanged** |

Atoms never revisited held flat because `HI-C92-achchha-buraa` requires and
practises `HI-SCRIPT-RECOG-130`, asking the reader to find **छ** inside
**अच्छा** — the fourth chapter running where a letter lesson pays off instead of
becoming terminal debt.

Reinforcement misses rose 1143 → 1164, in step with eight new lessons.

### Verification

`human-language-data` full suite; all twelve `check:*` gates;
`check-book-compile.sh hindi` under XeLaTeX.

### What this does not claim

Coverage means the teaching exists, not that a reader scores. No track has had a
single exam item graded against it.
