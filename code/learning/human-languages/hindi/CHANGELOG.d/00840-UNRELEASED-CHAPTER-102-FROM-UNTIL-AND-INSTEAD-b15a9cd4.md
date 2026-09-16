## Unreleased — chapter 102: from, until, and instead

Hindi A1 exam coverage **228/282 (81%) → 230/282 (82%)**, closing two points for
**one new noun**.

| point | what was missing |
|---|---|
| `HI-A1-POST-09` | *se … tak* for a span of days or hours |
| `HI-A1-NEG-03` | contrasting two alternatives negatively — *X se nahīṁ, Y se* |

### Both notes were exactly right

The first two accurate notes in six chapters. `POST-09` said *"The weekday nouns
are taught; the frame that spans them is not"* — true. `NEG-03` said *"Needs
`se`"* — true, and **से** was taught in chapter 98.

So the whole chapter costs **one word**. *bas se* came with chapter 98's
postposition lesson, the weekdays with chapter 10, *subah* and *shām* with
chapters 28 and 29, *baje* with chapter 96. Mock 2's `main bas se nahīṁ, ṭren se
jāūṁgā` was one noun away from readable.

### से now does five jobs and has never bent

| job | example |
|---|---|
| from a place | स्टेशन से |
| by a means | ट्रेन से |
| measured against | बच्चे से बड़ा |
| the start of a span | सोमवार से |
| one option in a correction | बस से नहीं |

**से never changes form**, whatever it is doing — unlike **का**, which had to
become **के** the moment another word followed. The reason is worth keeping and
the review lesson states it: **का** is a possessive and possessives agree;
**से** is a plain postposition with nothing to agree with.

### The span is two packages, not one phrase

**[ सोमवार से ] [ शुक्रवार तक ]**

English builds a span out of *from* and *to* standing **in front of** their
days. Hindi marks each end **from behind**. Reading it as two packages rather
than one long string is what makes a written account of somebody's week come
apart at the right seams the first time.

**तक does not require से.** *pāṁc baje tak* is complete on its own, and that is
how the word most often appears on a sign or in a notice — so `HI-C94-tak`
teaches it alone before anything pairs it. A reader who has only ever seen the
pair will not recognise the half.

The frame does not care about its units: days, parts of a day and clock hours
all fill it unchanged. The clock row stacks two endings on one side —
**दस** → **दस बजे** → **दस बजे से** — each step adding one word to the right,
which is the direction this language always builds in.

### The correction frame, and the thing English drops

**बस से नहीं, ट्रेन से।**

| beat | contents |
|---|---|
| 1 | the wrong option, with its own **से** |
| 2 | **नहीं** |
| 3 | the right option, with its own **से** |

**Each option keeps its marker.** English drops the second one — *not by bus, by
train* has one *by* — and Hindi has two **से**. Counting them is a usable check:
two means a swap is being offered.

There is also no word in the middle beyond the negative itself. The English
speaker's instinct is to reach for something like *but*; the comma carries the
pause.

### The position of नहीं says how much is denied

| what is denied | example |
|---|---|
| the whole sentence | मैं नहीं जाता |
| one option in it | बस से नहीं, ट्रेन से |

In every sentence before this one, **नहीं** stood in front of the verb and
denied the whole thing. Here it stands after a noun-plus-postposition and denies
**only that phrase**. Getting it wrong turns a correction into a refusal.

That is also why a reading item is answerable: the phrase **after** the negative
is the true one, and the commonest wrong answer is the phrase before it — which
is exactly why the paper prints the rejected option first.

The lesson's headword is **नहीं की जगह**, not the sentence. Printing both sides
of a contrast claims to introduce both, and here *both* were already taught —
*bas se* nine chapters ago and *ṭren se* one lesson earlier. The metalinguistic
headword is what keeps `forward-language` flat; the sentence version cost one
entry and was caught before commit by the `lessons.d` procedure.

### ट्रेन, and why the first letter is the curled-back one

Both vehicles in the track are English loans, and **ट्रेन** takes the retroflex
**ट** rather than the dental **त** because English *t* sounds, to a Hindi ear,
much more like the retroflex. That is the regular habit for English loanwords.

### नौ is why the au-mātrā matters

The first draft wrote the clock span as *nau baje se pāṁc baje tak* — *from nine
to five*, the natural English working day. **नौ** contains **ौ**, which is
untaught, and it put two lessons into script debt. Visible **only** in the
snapshot diff: every gate passed.

Changed to **दस**. It is worth recording what the accident showed: **ौ is the
largest single glyph debt left in the track at four lessons**, its earliest use
is chapter 16, and *nau* is one of the words it blocks.

### Numbers

| metric | before → after |
|---|---|
| atoms taught | 504 → 509 |
| measurable lessons | 447 → 453 |
| `measurement-blind` | 15 → 16 |
| **atoms never revisited** | **34 → 33** |
| `forward-language` | **unchanged** (21) |
| `payoff-surprise` | **unchanged** |
| script-closure violations | **unchanged** (30) |
| never-taught glyphs | **unchanged** (6) |

`atoms never revisited` **falls**, which is unusual for a chapter that adds
atoms: the review and synthesis lessons between them practise all five new
atoms, and the synthesis reaches back to chapter 96's future and chapter 100's
comparison.

`measurement-blind` rises by one because the payoff lesson is `type: synthesis`,
which `isExplicitRetrievalOnlyLesson` does not accept. Reported rather than
relabelled.

Reinforcement misses 1175 → 1184, in step with seven new lessons.

### Lessons

| id | headword |
|---|---|
| `HI-C94-tak` | तक |
| `HI-C94-se-tak-din` | सोमवार से शुक्रवार तक |
| `HI-C94-se-tak-samay` | सुबह से शाम तक |
| `HI-C94-train` | ट्रेन |
| `HI-C94-se-nahin` | नहीं की जगह |
| `HI-C94-repaso-safar` | से की तालिका |
| `HI-C94-sintesis-safar` | मैं ट्रेन से जाऊँगा |
