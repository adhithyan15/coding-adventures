## HL-C317 — Chinese A1: pinyin is a pronunciation claim, and not one particle is taught

`core/exam-inventory-chinese-a1.json` enumerates **191** A1 points and the corpus
covers **66** of them, 35%. Two things in this file are worth more than the
number: a decision about what pinyin coverage means, and a measurement that says
the track has taught vocabulary and script and has not yet taught grammar.

### The Spanish orthography column does not transfer, so it was restated

`A1-O1-01` asks for the alphabet: the closed set of units a reader learns once
and reuses forever. There are three ways to handle that for a logographic script
and only one of them is honest.

* **Drop it.** Then the file says the proxy has nothing to offer about the
  Chinese writing system, which is false — the question it asks is real.
* **Read it as "the characters".** Then the denominator is tens of thousands and
  every Mandarin course that has ever existed reports as failing, which measures
  nothing.
* **Read it as the STROKE**, which is what the question was about. The corpus
  teaches exactly this and opens on `yi` — a single horizontal that is both one
  stroke and one whole character.

The third. Likewise `A1-O1-04` and `A1-O1-05`, the case points: hanzi is
unicameral, so those have **no character analogue at all**, and this is written
into `ZH-A1-H-06` as a statement rather than a drop, because a Spanish or
English learner's instinct to capitalise needs somewhere to be corrected. The
demand survives in pinyin and is measured there. Only `A1-O1-06` — superscript
letters in abbreviations — returns nothing on either side and is the file's one
`notTransferred`, and its reason names the track that *derived* the same point
(Russian, `RU-A1-L-09`, the hyphenated ordinal `1-y`) so that "nothing to
superscript" cannot be read as "superscripts looked foreign".

### The pinyin decision, and why it is in the file rather than in a commit

The corpus teaches many words in pinyin before it teaches their characters. So
an inventory has to answer: **when a word is known in pinyin, is that a script
claim or a pronunciation claim?** The answer changes the character column
directly — count pinyin as script and the column roughly doubles.

**Decided: pronunciation.** Three pieces of evidence, all internal and all
re-checkable:

1. The corpus files pinyin in the `romanization` frontmatter field, and
   `script-closure.ts` treats a declared romanization as **the promise that the
   reader can use the word without reading it**. Within this repository's own
   machinery, pinyin is by construction the thing that removes a decoding demand
   rather than one that meets it.
2. Every one of the 45 `ZH-SCRIPT` atoms names a character, a component or a
   stroke method. **Not one names a pinyin letter, initial, final or tone mark.**
   The tone marks are `ZH-TONE` atoms; the segments are `sounds:` tags. The
   corpus's own script column contains no pinyin.
3. The track already splits the two claims itself. A word arrives by ear as a
   `ZH-LEX` atom introduced by a `listening-speaking` lesson with a pinyin
   headword, and becomes readable and writable **later**, as a separate
   `ZH-ORTHO` atom. Eleven `ZH-ORTHO` atoms exist for exactly this purpose.

And the corpus says it out loud in one lesson: the chapter-12 hearing lesson for
`nu'er` explains that the apostrophe "only keeps the vowel boundary visible in
pinyin. It is not a pause and not part of the Chinese writing."

**The consequence is recorded, not hidden.** The character column counts
characters, so 53 words known by ear cannot inflate it; and pinyin's own
orthography — initials, finals, tone-mark placement, capitals, word division —
is untaught, and sits as four uncovered points in a column of its own. A reader
who disagrees with the decision can refile those four as script debt without
re-deriving anything. A test asserts that no point in the pinyin column is ever
probed with a `ZH-SCRIPT` atom, so the decision cannot rot into the opposite one
by accident.

**Generalisable rule:** where a track teaches a romanization, decide once
whether it is script or pronunciation, write the decision and its consequence
into the inventory as a null point, and enforce the split mechanically. The next
two tracks that need this are Japanese (romaji) and Urdu.

### Not one particle is taught, which is a different kind of gap

    de    (possessive/relative)  0 occurrences in 175 files
    le    (aspect/change)        0
    ma    (polar question)       0
    ne    (follow-up question)   0
    ba    (suggestion)           0
    guo   (experiential)         0
    zhe   (durative)             0

Checked in characters and in tone-marked pinyin. Mandarin carries almost all of
its grammar in this handful of toneless syllables, so **this is not a vocabulary
gap in the way the other empty columns are** — it is the reason the possessive,
the relative clause, the polar question, the past and the sentence-final change
of state are all null at once. `ma` alone turns every statement in the book into
a question. `de` alone opens possession and the relative clause.

### The joining column is 0 of 8, seventh in a row

`he`, `gen`, `huozhe`, `haishi`, `keshi`, `danshi`, `yinwei`, `suoyi`: zero
across 175 files. Every raw pinyin match for `he` is inside `heng` (a stroke
name) or `shenme`. This is the second non-South-Asian track to report the column
empty — Russian was the first, in the same sitting — which puts the finding well
past the point where it could be a fact about Indo-Aryan or Dravidian grammar.
**It is a fact about how this corpus is authored:** chapters are built around a
word family, and a conjunction belongs to no family, so nothing ever schedules
one.

### One repair move, and the chapter that knows it

Chapter 9 stages an exchange that goes wrong and gets repaired, and explains
itself: "until now every exchange in this book has gone perfectly, which is not
what conversations do." That instinct is right and rare. What it delivers is one
move — `shenme?`, asking for a word back.

    duibuqi / bu hao yisi  (sorry)         0 occurrences
    dong / ting bu dong    (understand)    0
    zai shuo yi bian       (say it again)  0
    man                    (slowly)        0

So there is no apology and no way to tell a speaker to change what they are
doing. Japanese, measured in the same sitting, spends **two whole chapters** on
exactly these moves. The comparison is the useful part: the difference is not
difficulty, it is that one track scheduled repair as a topic and the other did
not.

### A font blocker recorded inside a lesson, which is the right place for it

Chapter 9 states that "what is your name?" cannot be completed because the
missing half needs a character the book's font cannot print, and chapter 10 says
the commonest reply to `xiexie` is unavailable for the same reason. Both are
honest and both are real debt: **the single most-asked question of any A1
speaking paper is blocked on font coverage**, not on curriculum design. That is
a `main-font-charset.json` / book-preamble task rather than an authoring one,
and it should be scheduled as such.

### What to author next, off this inventory

1. `ma` — closes the polar question, the tag question and "how are you", and
   costs one character at the end of sentences the reader already says.
2. `de` — closes possession and the relative clause, four Spanish points.
3. `zhe` and `na` — closes three demonstrative points and gives `shi`, already
   taught, the commonest subject on a beginner's paper.
4. `you` — three Spanish points (existence, presence, having) on one verb.
5. The numerals six to ten — unblocks the decade rule, the calendar, age and
   price, and the track already teaches one to five and shows `shi` under `zao`.
6. `duibuqi`, `dong`, `man` — closes the repair column and needs no new grammar.
7. Resolve the font blocker so chapter 9's question can be finished.
