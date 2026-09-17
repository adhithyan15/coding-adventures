## Unreleased — the anusvāra and the last two mātrās

**`HI-A1-SCR-14` closes, and `HI-A1-SCR-07` stops being half a claim.** After
this, **Devanagari debt is one glyph** — ञ, which appears only inside a conjunct
the corpus already teaches.

| metric | before → after |
|---|---|
| exam-point coverage | 235/282 → **236/282 (84%)** |
| atoms taught | 530 → 533 |
| measurable lessons | 475 → 478 |
| writing-practice lessons | 121 → 124 |
| `forward-language` | unchanged (22) |
| `atomsNeverRevisited` | unchanged (33) |
| `durationViolations` | unchanged (0) |

### The most-used glyph the corpus never taught

**The anusvāra is in 60 headwords**, from chapter 2 — more than व's 39 or श's
29. It is in **मैं**, **हैं**, **में**, **मिलेंगे**, **हिंदी**: not decorative
words, but how a reader says who they are and how things stand.

It was credited as taught because `HI-W05-virama-namaste` — a lesson about the
**virama** — has the headword **नमस्ते** (`HL-C386`).

### A point marked covered while half of it was a deferral

`HI-A1-SCR-07` names *"the nasalisation marks: candrabindu, and the anusvāra it
is confused with."* The candrabindu had a lesson. The anusvāra had none — **and
that lesson said so itself**:

> There is a plain dot without the crescent that does a related job … **For
> now**: crescent plus dot means *through the nose*.

A lesson recording a debt it has no way to notice being paid — the fifth variety
of stale claim in `lessons.d`, and the second time one has been found outside the
exam inventory. `HI-S148-sign-anusvara` draws the mark at chapter 16, and the
candrabindu's deferral is **rewritten to point back at it** rather than forward
at nothing.

### Three of `SCR-14`'s four were never really undrawn debt

| mātrā | was credited to | which is a lesson about |
|---|---|---|
| ◌ू | `HI-W06-name-sentence-stop` | the **danda** (headword *pūrṇ virām*) |
| ◌ो | `HI-W01-shirorekha-na-ma` | the **head-line** (headword *śirorekhā*) |
| ◌ौ | `HI-S134-vowel-sign-au` | itself — a real lesson |
| ◌ृ | — | drawn in the previous entry |

**◌ू** is taught off its short twin: same hook, longer tail, longer vowel — one
of the few places the writing matches the sound so directly, and the lesson says
that is worth noticing precisely because it is not always true.

**◌ो** is taught off **दो** — one consonant and one mark, the plainest example
in the language.

### A forward reference caught before commit

`HI-S150` first worked through **क** + **◌ो** = **को**, and **को** is taught
361 lessons later. `forward-language` went 22 → 23. Swapped to **द** + **◌ो** =
**दो**, taught in chapter 6, and the metric went back to 22.

### An atom that could not be wired backwards

The candrabindu lesson's prose now refers to the anusvāra as already known,
which is true in reading order — position 94 against 265. **Wiring its atom in
was rejected**, because atom availability is computed over curriculum order
rather than sequence, and `HI-EXT-100` sits after the extension holding the
chapter-59 run. The prose stands on its own; the atom does not need to.

### Payoffs

| atom | payoff |
|---|---|
| `HI-SCRIPT-RECOG-148` | `HI-C18-ghanta` — **घंटा** |
| `HI-SCRIPT-RECOG-149` | `HI-C37-dudh` — **दूध** |
| `HI-SCRIPT-RECOG-150` | `HI-C30-dopahar-widened` — **दोपहर** |

### What is left

**One glyph: ञ.** It appears three times in the corpus and never standalone —
always inside **ज्ञ**, which `HI-W05-conjuncts` teaches as one of its three
special shapes. **ङ** is absent from the corpus and from the script data
(`HL-C385`) and needs no Hindi lesson.

The Devanagari script track is, for practical purposes, finished.
