## Pre-A1 vocabulary tranche — twelve everyday nouns, three chapters

The level gate (`src/level-gate.ts`) reports every track blocked on
**vocabulary**: 300 distinct headwords at or below pre-A1. This tranche
authors twelve concrete nouns across three new chapters, continuing the
Hindi/Arabic/Tamil/German program and confirming the same mechanism:
`vocabularyOf()` counts distinct `headword:` strings, so twelve one-headword
lessons move Gujarati's pre-A1 vocabulary by exactly twelve — **22 → 34**
distinct headwords at or below pre-A1 (against the 300 target, shortfall
278 → 266), and 41 → 53 distinct headwords track-wide. No bulk credit;
measured, not assumed.

| Lesson | Concept | Word |
|---|---|---|
| `GU-C10-paani` | `GU-FOOD-WATER` | પાણી |
| `GU-C10-chaa` | `GU-FOOD-TEA` | ચા |
| `GU-C10-dudh` | `GU-FOOD-MILK` | દૂધ |
| `GU-C10-rotli` | `GU-FOOD-BREAD` | રોટલી |
| `GU-C11-mitra` | `GU-PEOPLE-FRIEND` | મિત્ર |
| `GU-C11-kutumb` | `GU-FAMILY-WHOLE` | કુટુંબ |
| `GU-C11-bhai` | `GU-FAMILY-BROTHER` | ભાઈ |
| `GU-C11-bahen` | `GU-FAMILY-SISTER` | બહેન |
| `GU-C12-aankh` | `GU-BODY-EYE` | આંખ |
| `GU-C12-kaan` | `GU-BODY-EAR` | કાન |
| `GU-C12-modhu` | `GU-BODY-MOUTH` | મોઢું |
| `GU-C12-naak` | `GU-BODY-NOSE` | નાક |

**Atom-first, gender taught with every noun.** Each lesson introduces 2–3
knowledge atoms (`GU-LEX-*`, usually `GU-ETYMON-*` and sometimes a
`GU-GRAMMAR-*` or `GU-HISTORY-*`), at or under `maxNewAtomsPerLesson: 3`.
Chapter 10 introduces 11 atoms, chapter 11 introduces 9, chapter 12 introduces
10 — all at or under `maxNewAtomsPerChapter: 12`. Every noun's gender is
stated with the noun.

**Chapter 10 — Water, Tea, Milk, and Bread** (`SPINE-POLITE-REQUEST-REPAIR`,
previously the one pre-A1 spine node with zero lessons on it): the chapter
builds Gujarati's first polite-request pattern from scratch — **[item],
મહેરબાની કરીને** ("[item], please," literally "having done a kindness"),
using the `કરીને`/`કરવી` family already established by `મદદ કરવી`. **પાણી**
and **દૂધ** are inherited straight from Sanskrit with confirmed PIE roots
(*peh₃(i)-* "to drink" → English *potion*, *poison*, *symposium*; *dʰewgʰ-*
"to be fit, useful" → English **doughty**, not milk-related at all in
English). **ચા** is a loan that took the **overland** route out of Mandarin
Chinese by way of Persian — the same route as Hindi/Russian *chai* and
Turkish *çay* — as opposed to English *tea*'s **sea** route via Hokkien
Chinese and Dutch traders; ચા is also one of Gujarati's few nouns whose
gender is not fixed (masculine or feminine). **રોટલી** is inherited too, but
Turner's and Mayrhofer's comparative dictionaries leave its own root an open
question — an honest dead end, not an invented one. The payoff also uses
**પાણી**'s neuter gender (despite its **-ī** ending) to teach that
**રોટલી**'s own feminine **-ī** is a hint, not a rule.

**Chapter 11 — Friend and Family** (`SPINE-EXCHANGE-NAMES`): **મિત્ર**
("friend") is Sanskrit, from Proto-Indo-Iranian *\*mitras*, "(that which)
causes binding" — the same word as Avestan Mithra, whose Middle Persian
descendant *mihr* is the very root **મહેરબાની** was built on last chapter, so
the two words turn out to be cousins three chapters apart. **કુટુંબ**
("family") is a **learned borrowing** (*tatsama*) from Sanskrit rather than a
word worn down through sound change (*tadbhava*) like the other three —
and Sanskrit's own *kuṭumba* is itself thought to be a **Dravidian** loan,
compared to Tamil *kuṭimai*. **ભાઈ** ("brother") is English *brother*'s
unbroken cousin, down to the shared PIE root *bʰréh₂tēr*. **બહેન**
("sister") is deliberately **not** *bhāī*'s parallel: it is not from the PIE
root behind English *sister* (*swésōr*) at all, but from Sanskrit *bhaginī*,
disputedly built on *bhaga*, "a share, good fortune" — named as disputed
because scholarly opinion is genuinely split, not settled for convenience.

**Chapter 12 — Eye, Ear, Mouth, and Nose** (`SPINE-CHECK-WELLBEING`):
**આંખ**/*eye* and **નાક**/*nose* both trace to confirmed PIE roots shared
unbroken with their English counterparts (*h₃ekʷ-*, *nehas-*). **કાન**/*ear*
is inherited from Sanskrit *karṇa* with no agreed root beyond it — scholars
have proposed "defect," "point," "handle," and a link to "to hear" without
settling — and its gender shifted from Sanskrit masculine to Gujarati
feminine, while **મિત્ર** kept its Sanskrit masculine, making the point that
inheritance does not guarantee gender survives unchanged. **મોઢું**/*mouth*
is this track's clearest example of the nasalized **-ũ** neuter tell — the
same ending every **-વું** verb infinitive wears, now shown doing the same
job on a noun — while **નાક** is neuter without it, and **આંખ**'s medial
nasal (echoing વાંચવું's) is shown NOT to be a gender marker at all, so the
chapter keeps "nasal as sound" and "nasal as the neuter tell" explicitly
apart rather than letting a learner conflate them.

**Reach back at two cadences (HL09 §7).** Every lesson names atoms from the
one to three lessons immediately before it. Each chapter's payoff also
reaches back at least one chapter further: chapter 10's payoff rescues
chapter 6's `GU-SCRIPT-HEADLESS-CLUE` (never revisited since it was
introduced); chapter 11's rescues chapter 6's `GU-HISTORY-LEARNED-RESTORATION`;
chapter 12's rescues chapter 8's `GU-SCRIPT-ANUSVARA-MEDIAL` and chapter 7's
`GU-FORM-JAVU-JOVU-CONTRAST`, both previously orphaned. All three payoffs
close over their own chapter's atoms at **1.00 representativeness** (11/11,
9/9, 10/10) against the 0.5 floor.

The level gate's **reinforcement** criterion, previously vacuous at pre-A1
because chapters 1–5 are schema v1 and declare no atoms, now runs for real —
this tranche is the first pre-A1 content the track has ever had. It reports 4
non-etymology atoms at or below pre-A1 revisited fewer than twice
(`GU-HISTORY-TEA-CHA-ISOGLOSS`, `GU-LEX-BHAI`, `GU-LEX-DUDH`,
`GU-LEX-ROTLI`) after 8 etymology hooks are waived per the project owner's
standing decision. One atom that started at zero revisits,
`GU-GRAMMAR-GENDER-NOT-BY-ENDING` (rotli's payoff atom), was deliberately
rescued to two by genuine callbacks in `GU-C12-kaan` and `GU-C12-naak`,
both of which independently restate its point about gender not following
from a noun's ending. The remaining four are left for a future tranche
rather than padded with unearned reach-back — the standing instruction is to
report observed numbers, not re-pin them.

**Font check, done before authoring and again before commit.** A forced
XeLaTeX compile caught three real mistakes: a literal Chinese character
(茶) in Chapter 10's `ચા` prose, the Greek letter θ inside "Miθra," and
seven Devanagari-titled Wiktionary source links (Chapter 10 through 12) —
all fell through to Latin Modern Roman, which has none of the three, and all
were fixed (romanized to *chá*, *Mithra*, and English-titled Wiktionary
links) before this commit. The nasalized-vowel check (`ũ`, `ĩ`, `ã`,
combining U+0303) needed no new declarations: all three were already
handled by `book/preamble.tex`. The precise PIE notation this tranche uses
(`h₃ekʷ-`, `bʰréh₂tēr`, `swésōr`, `dʰewgʰ-`, `peh₃(i)-`, `nehas-`) — richer
than the plain-ASCII Pokorny style Chapters 7–9 used — compiles clean under
the corpus-wide font mapping now in place.

**Wiring**: `GU-PATH-013`–`GU-PATH-015` are three new path segments — one on
`SPINE-POLITE-REQUEST-REPAIR` (previously realized by zero segments), one on
`SPINE-EXCHANGE-NAMES`, one on `SPINE-CHECK-WELLBEING` — each with a matching
`GU-EXT-0{13,14,15}-LANGUAGE-SPECIFIC` extension. Both steps are required,
since `lessonSpineNodes` only walks `curriculum.path[].lessons`.

**Verification.** A forced XeLaTeX build of the 76-page book has zero
missing characters, zero new overfull/underfull boxes (the corpus's one
pre-existing underfull box, in an earlier chapter, predates this tranche),
and zero duplicate labels. All three new chapters generate as `voice`,
drivable end-to-end via the detachable `## The letters in this word`
section. Narration for all three chapters was read back and confirmed to
narrate correctly aloud — no markdown tables are used anywhere in this
tranche, so the narration generator's column-header quirk does not apply.
`npx vitest run tests/integration.test.ts tests/cli.test.ts` passes (19/19);
`check:modality`, `check:books` and `check:narration` all pass with no diff
beyond the new chapters. The six corpus-wide pinned-number tests
(`chapters`, `continuity`, `levels`, `modality-manifest`, `narration`,
`ramp`) shift with any authored content and are left failing, per standing
instruction — their numbers are reported here, not re-pinned.

