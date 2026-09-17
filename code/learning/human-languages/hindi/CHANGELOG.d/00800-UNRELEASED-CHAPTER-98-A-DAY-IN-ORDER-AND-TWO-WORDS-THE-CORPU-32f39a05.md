## Unreleased — chapter 98: a day in order, and two words the corpus had been using without teaching

Hindi A1 exam coverage **214/282 (76%) → 217/282 (77%)**, closing three points.

| point | what was missing |
|---|---|
| `HI-A1-F-59` | describe a daily routine |
| `HI-A1-POST-04` | *se* for origin, instrument and means |
| `HI-A1-T-09` | *pahle* and *ke bād* — before and after |

### Another partly-stale note, and it is now the commonest finding here

`HI-A1-F-59`'s note said the habitual tense is taught but *"`jana`, `uthna`,
`baje`, `se` and `par` — everything a routine is made of besides the tense — are
not."*

**Three of those five had been taught by the time it was re-read:**

| exponent | where |
|---|---|
| *jānā* | `HI-C69-go`, a headword all along |
| *par* | chapter 95 |
| *baje* | chapter 96 |

The real gap was **उठना** and **से** — plus the sequencing words the note did not
mention at all, which turn out to be what a routine actually needs.

The other two notes were exactly right. `POST-04`'s says *se* is load-bearing in
both mocks and that the speaking interview's origin question cannot be answered
without it. `T-09`'s reads, in full, *"Nothing can be sequenced in time."*

### Four of the six new words are joints, not vocabulary

| word | what it adds to a day |
|---|---|
| उठना | the action it starts with |
| काम | the place it goes to |
| बस | what carries you there |
| से | from, and by |
| पहले | which of two came first |
| के बाद | which came second |

Before this chapter the track could state facts about a day and not put two of
them in an order, so a one-minute prepared description was four unrelated
statements. The synthesis lesson shows what the joints are doing by naming what
happens if you take them out.

**से** is taught as **one relation rather than two words**: the thing proceeds
from the noun, whether that noun is a city or a hand. Once that lands, a vehicle
taking the same postposition stops being a second fact.

The **पहले / के बाद** pair is asymmetric — *before* takes **से** and *after*
takes **के** — and the lesson says outright that there is no tidy reason to give,
which is more honest than inventing one. What it can show is that **के बाद** is
the possessive **का / के / की** already taught for *uskā* and *hamārā*, used as
glue: *the after **of** work*.

### Two things the corpus had been carrying without unpacking

**काम करना was never one word.** It is **काम** plus **करना** — *to do work* —
and once you see that, it stops being a memorised phrase and becomes the machine
Hindi builds an enormous number of verbs with: a noun, plus *to do*. The noun is
Sanskrit **कर्म** (*karma*), "a thing done," which English borrowed whole.

**बस is two different words spelled alike.** The Persian *bas* meaning *enough,
that's all* has been in the language for centuries; the English *bus* arrived
with the vehicle, shortened from *omnibus* — Latin for **"for everybody"**, which
is a fair description of a bus and a strange thing to find inside three
Devanagari letters. Nothing marks them apart on the page; **the sentence does**,
and the moment **से** follows it, it is a thing you can travel in. Declared as
`introduces_senses`, so the senses pin goes 24 → 25.

### forward-language 13 → 18, and this time the rise is correct

Five new entries, all naming this chapter's lessons as `taughtBy`: four for
**काम** and one for **से**.

One chapter ago an identical-sized rise was a **defect** — a headword falsely
claiming to introduce a form taught since chapter 5. This one is the opposite.
*Kām* had appeared in **nine** lessons since chapter 5, inside *kām karnā*, and
had never been introduced as a word; *se* had appeared in **fourteen**. The
detector could not see either hole, because `measureContinuity` reports a use as
early only relative to a lesson that claims to teach the word — **with nothing
claiming to teach it, an untaught word is invisible to this metric.**

So the chapter converted invisible debt into visible debt. Renaming the
headwords to dodge the rise would have put the holes back and hidden them again.

Recorded in `lessons.d`. The diagnosis is thirty seconds: group the new entries
by `taughtBy`, then grep for `^headword:.*<word>` in earlier lessons. If
something already introduces it, your headword is lying. If nothing does, the
entries are real. **The size of the jump tells you nothing** — both cases here
were +5, on consecutive chapters, from the same metric, and needed opposite
responses.

One small correction rides along: chapter 97's review lesson was headworded
*jārī kām kī tālikā*, which used **काम** one chapter before it is introduced.
Renamed to *abhī kī tālikā*.

### Numbers

| metric | before → after |
|---|---|
| atoms taught | 478 → 484 |
| measurable lessons | 417 → 424 |
| `forward-language` | 13 → 18 |
| `measurement-blind` | 11 → 12 |
| `payoff-surprise` | **unchanged** |
| script-closure violations | **unchanged** |
| atoms never revisited | **unchanged** |

No new glyphs were needed. That is now two chapters running, against two before
them that were blocked by the alphabet — the glyph check is what tells you which
kind of chapter you are about to write.

Reinforcement misses rose 1109 → 1126, in step with eight new lessons.

### Verification

`human-language-data` full suite; all twelve `check:*` gates;
`check-book-compile.sh hindi` under XeLaTeX.

### What this does not claim

Coverage means the teaching exists, not that a reader scores. No track has had a
single exam item graded against it.
