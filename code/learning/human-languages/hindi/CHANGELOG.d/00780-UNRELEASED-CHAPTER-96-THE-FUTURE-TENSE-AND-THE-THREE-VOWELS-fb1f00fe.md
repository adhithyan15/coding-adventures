## Unreleased — chapter 96: the future tense, and the three vowels that had never been taught

Hindi A1 exam coverage **211/282 (75%) → 213/282 (76%)**, closing two points and
making **four scored mock items** readable.

| point | what was missing |
|---|---|
| `HI-A1-V-11` | the future as a productive paradigm, with agreement |
| `HI-A1-T-06` | the adverbial *baje* — naming an hour to meet at |

### The alphabet blocked this one too, and the audit predicted it

Last chapter's finding — *when a track cannot say something, check the alphabet
before the curriculum* — was written up as `HL-C381`, which named **ऊ** as the
most expensive untaught glyph and said in as many words that it was what blocked
the future tense. That turned out to be exactly right, and the backlog entry was
the work queue it claimed to be.

The reason is worth stating precisely, because it is a fact about the **script
rather than the grammar**:

| stem ends in | how the long *ū* is written |
|---|---|
| a consonant (कर-) | a mark under the letter |
| a vowel (आ-, जा-) | the full letter **ऊ** |

**करूँगा** was writable all along. **आऊँगा** and **जाऊँगा** were not — and those
are the two the practice papers print. The same rule bites once more one lesson
later: **करोगे** carries its *o* as a mark, **आओगे** needs the full letter **ओ**.

So three letter lessons come first — `HI-S127-letter-u`, `HI-S128-letter-uu`,
`HI-S129-letter-o` — and each is an old shape with a piece added:

| letter | built from |
|---|---|
| उ | a bowl and a loop |
| ऊ | उ, plus one loop on the right |
| ओ | आ, plus an arc above the line |

They sit at **chapter 22**, immediately before the first word in the track that
needs one. Never-taught glyphs **11 → 7** across two chapters; closure violations
**35 → 31**.

### Half the point's note was too strong, and is corrected

`HI-A1-V-11`'s note said *"milenge is taught as one memorised farewell, not as a
tense."* Read against the corpus, that is not true. `HI-C04-milenge` takes the
word apart, names the root *mil-*, names **-एंगे** as the plural future, and says
outright that Hindi marks the future with an ending on the stem rather than a
helper word.

What chapter 4 gives is **one cell of a grid** — the plural, with no person and
no gender. That is why the point was genuinely uncovered, and it is a different
complaint from the one the note made. The new chapter reads the farewell back as
a stem plus this ending, which is the payoff chapter 4 set up and never
collected.

The grid it was missing:

| subject | with आ- |
|---|---|
| मैं | आऊँगा / आऊँगी |
| तुम | आओगे |
| वह | आएगा / आएगी |
| हम, वे, आप | आएँगे / आएँगी |

**-गा** is in every cell. What moves is the vowel in front of it, and that vowel
is the person — the same two-slot division the present and past already use. The
future is the odd tense out in one way: it has **no separate word for time at
all**, where the present and past both put the tense in the word after **-ता**.

*-गा* is also not arbitrary. It descends from Sanskrit **गत** (*gata*), "gone" —
so a Hindi future is built out of a word meaning *gone*, which is the instinct
behind English *I am going to*.

### baje: reading the hour against arranging to meet at it

`HI-A1-T-06`'s note is exactly right and needed no correction. Chapter 18 teaches
**बजना** as a **predicate** — *एक बज रहा है*, it **is** one o'clock — and never
the form that sits beside a number to fix a time to act at.

**पाँच बजे** is the same bell verb in a different shape, and there is **no word
for *at*** — the number plus **बजे** carries it. The time phrase takes the seat
the place phrase already took: after the person, before the verb.

The two belong in one chapter because that is how both papers print them: a
number, **बजे**, and a future verb.

### The items this makes readable

| paper | item |
|---|---|
| mock 1, reading 8 | मैं पाँच बजे आऊँगा। |
| mock 1, reading 11 | बस दस मिनट में आएगी। |
| mock 2, reading 8 | मैं आज देर से आऊँगी। |
| mock 2, reading 13 | मैं बस से नहीं, ट्रेन से जाऊँगा। |

The synthesis lesson reads the first of those word for word, and notes what the
item actually tests: the two wrong answers offered are past and present, so
**the only thing telling them apart is the ending on the last word.**

### A payoff dilution, caught and fixed by moving the lessons

First placement put the three letter lessons in **chapter 20**, beside the two
already there. That took the chapter to eight atoms against a payoff assessing
three — `payoff-surprise` **13 → 14**, a real regression.

The fix was placement rather than a pin: chapter 22 has four atoms and a payoff
assessing all four, so it absorbs three more at **0.57**, still above the 0.5
floor, and chapter 20 returns to **0.60**. Chapter 22 is also where the first
**ऊ** word lives, so the letters land immediately before the word that needs
them. `payoff-surprise` back to **13**.

**Worth carrying forward:** the script-recognition chain rides inside content
chapters, and a content chapter is small — three to five atoms. Three letter
lessons will sink most of them below the representativeness floor. Check the
host chapter's atom count before choosing where a letter run goes.

### Numbers

| metric | before → after |
|---|---|
| never-taught glyphs | 10 → 7 |
| script-closure violations | 35 → 31 |
| atoms taught | 464 → 473 |
| writing-practice lessons | 100 → 103 |
| atoms never revisited | 35 → **34** |
| `measurement-blind` | 9 → 10 |
| `forward-language` | **unchanged** |
| `payoff-surprise` | **unchanged** |

`measurement-blind` is the one new *síntesis*. HL-C377 again, and **not**
relabelled: Spanish avoids it by using `type: practice-mix`, and switching Hindi
to that purely to move a number would be letting the measurement pick the label.

**Atoms never revisited improved**, which is the second chapter running where a
letter lesson paid off instead of becoming terminal debt: `HI-C88-aaunga`
requires and practises `HI-SCRIPT-RECOG-128` and `HI-C88-aaoge` does the same for
`HI-SCRIPT-RECOG-129`, so the reader is asked to find the letter inside the word
it was taught for.

Reinforcement misses rose 1071 → 1097, in step with eleven new lessons.

### Verification

`human-language-data` full suite; all twelve `check:*` gates;
`check-book-compile.sh hindi` under XeLaTeX.

### What this does not claim

Coverage means the teaching exists, not that a reader scores. No track has had a
single exam item graded against it.
