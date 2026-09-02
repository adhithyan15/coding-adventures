## HL-C218 — Bengali after the eight shapes, and the etymon lever measured for the other five tracks

HL-C201 is closed. Teaching the eight shapes that already had a glossed word
waiting for them, plus giving Sanskrit citation forms in IAST rather than in
Bengali script, moved Bengali's script-closure violations **41 → 21** and
`neverTaughtGlyphs` **22 → 11**, and the new retrieval chapter moved R4 misses
**26 → 14** and never-revisited atoms **9 → 5**.

### The etymon measurement, for the tracks that asked for it

The coordinator's survey found 46 Bengali lines citing Sanskrit with the track's
own script on the same line, and asked whether that was inflating the closure
number the way Gurmukhi citation forms inflated Punjabi's (40 → 0). It is not
the same size of lever, and the difference is worth writing down.

**Of Bengali's 41 violations, exactly 6 were etymon-only** — lessons whose entire
untaught set came from a Sanskrit/Prakrit/Persian citation form:

| lesson | glyph it was spending | citation |
|---|---|---|
| `BN-C03-ami` | অ | *asmi* |
| `BN-C05-ami-bangla-boli` | গ ঙ | *bôngo* |
| `BN-C09-neowa` | ী | *√nī* |
| `BN-C11-bhai` | ৃ | *bhrātṛ* |
| `BN-C11-poribar` | ৃ | *√vṛ* |
| `BN-C15-kapor` | ট | *karpaṭa* |

Three glyphs — **অ**, **ঠ**, **উ** — left the corpus entirely, since a citation
was the only place they were ever shown.

The other 35 were **not** etymon-inflated. Bengali's untaught glyphs come
overwhelmingly from **Bengali words in prose** — খাওয়া, হওয়া, পড়া, জিজ্ঞাসা,
সাহায্য, হৃদয়, কলকাতায়, আপনি, তুই — which is why Punjabi's result does not
transfer. Teaching the eight shapes alone cleared **9**, and a further **5**
cleared only when both levers were pulled, because those lessons had one glyph
from each.

**So: 6 from orthography, 9 from teaching, 5 jointly, 41 → 21.** Telugu (9),
Kannada (7), Gujarati (9), Urdu (4), Tamil (2) and Malayalam (1) have the same
pattern at smaller scale; the number they should expect is "roughly one lesson
cleared per two citation lines", not Punjabi's clean sweep.

The editorial rule Bengali now follows, for anyone copying it: **a form cited as
belonging to another language is given in IAST; a Bengali word stays in Bengali
script even where its Sanskrit ancestor is spelled identically.** হৃদয়, মুখ, দয়া,
নীল, ভগিনী, ক্ষমা and শ্বেত all stayed. This is a claim about pedagogy, not about
Bengali scholarly practice, and `bengali/README.md` says so in those words.

### What is left, and what it would cost

The eleven shapes Bengali still never teaches, by how many lessons show each:

**ঞ** 4 · **ষ** 3 · **ঝ** 2 · **ফ** 2 · **থ** 2 · **ূ** 1 · **ট** 1 · **ঃ** 1 ·
**ঙ** 1 · **শ** 1 · (**ং**, exempt everywhere it appears)

Six of the twenty-one remaining violations want **ঞ** or **ষ**, and both have a
Bengali word waiting rather than a citation — **জিজ্ঞাসা** (chapter 15),
**ওষুধ** and **ক্ষমা**. Those two are the next tranche and are worth about six
lessons between them. **থ** (কথা, থাকা) and **ফ** (মাফ) are worth two each.
**ঝ** (বুঝি, বোঝে) is worth two but both consumers sit in chapter 14, ahead of
any chapter that could teach it after বোঝা is said.

### ী has no cheap earlier anchor — this was checked, not assumed

The previous note asked whether **ী** could move up from chapter 24 to sit beside
its short twin in chapter 4. It cannot, and after the etymon cleanup the reason
is sharper than before: **নীল is now the *only* word in the entire corpus that
carries it.** The four other ী forms — *√nī*, *bhaginī*, *śāṭī* and সাড়ী — were
all citation forms or older spellings, and three of them are gone. নীল is a
chapter-23 colour, so the sign cannot precede it, and the one remaining
ী violation (`BN-C14-shobuj`) is *intra-chapter*: শোবুজ recalls নীল inside the
colour chapter itself. Moving ী would require adding a pre-A1 headword carrying
a long *i* — নদী is the obvious candidate — which is new vocabulary, and this
tranche was told not to invent any. That is a real option for a later tranche;
it is not a cheap one.

### The curriculum path is script-first, and that blocks cross-strand review

`BN-PATH-001` places all sixty script lessons at positions 0–60, ahead of every
content lesson, so a script lesson that DECLARES a content atom needs a content
prerequisite the curriculum order cannot satisfy. That is why the fifteen new
script lessons carry their cross-strand links in `reviews_of` rather than in
`practises`, and why the retrieval chapter needed its own trailing segment
(`BN-PATH-022`) to be allowed to practise concept atoms at all.

The path ordering is a legacy artefact: it predates the interleaving of the
strand and no longer matches reading order. Rebuilding it so segments follow the
book would let every reading lesson credit the word it is reading, which is the
single largest remaining source of R2/R3 misses in this track (85 and 88). It
touches `curriculum.d/path/`, `curriculum.d/spine/` and the extension
attachments, so it wants its own tranche.

### Bengali is not on the writing-stage ladder

The retrieval chapter's `Writing — from sound` blocks are genuinely
dictation-transcription, but declaring that stage requires earlier evidence for
`observe-trace`, `guided-copy` and `delayed-copy` in the same track, and Bengali
declares none. The directives were removed rather than declared falsely.
Bengali's sixty script lessons already *do* observe-trace, guided-copy and
delayed-copy work — they are simply not annotated — so this is annotation, not
authoring, and it would move Bengali onto the 15-of-23 list.

### One shared test was re-pinned, and it needed to be

`tests/script-closure.test.ts` asserted `report.summary.violations > 500`. This
tranche took the corpus to 498 and the assertion failed — a **floor on debt
forbids exactly the work the module exists to prompt**. The same file's next
assertion already carries a note making this argument about
`tracksTeachingNothing`, which was converted from a floor to a ceiling for the
same reason. `violations` is now a ceiling on the same footing: `<= 498`, may
fall, never grow, and whoever raises it writes down why.
