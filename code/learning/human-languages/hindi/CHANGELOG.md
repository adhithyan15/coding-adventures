# Changelog

## Unreleased — Chapters 45-51: Thirty-five everyday words, one per lesson

Hindi stood at 50 headwords against the 300 the pre-A1 vocabulary floor asks
for. These seven chapters answer with thirty-five, and with nothing else — **one
new word per lesson**, and reuse of everything already taught, unlimited and on
purpose.

  45 Things You Ask For     फल कपड़ा दीया नमक साबुन
  46 Hair, Ear, and Throat  बाल उँगली कान नाक गला
  47 Who Someone Is         शिक्षक छात्र किसान मेहमान पड़ोसी
  48 Short Replies          सच बस ज़रूर शायद बिलकुल
  49 Taking Your Leave      अभी परसों सफ़र निकलना मुलाक़ात
  50 Courtesy Words         आभार एहसान आदर आशीर्वाद प्रणाम
  51 Welcoming a Guest      फूल माला आँगन मेज़ मिठाई

Each chapter sits on one of the seven pre-A1 spine nodes, five lessons sharing
it, and all seven nodes are used — including SPINE-COURTESY-THANK, which no
Hindi chapter had realised since chapter 5. Every lesson chains to the one
before, and each chapter's second lesson still practises the previous chapter's
last word, so a word introduced by a payoff lesson is being retrieved two
lessons into the following chapter.

That chaining is why the ramp got *gentler* while the book got longer. The R1
reinforcement ratio falls 0.3005 to 0.2977, and the share of atoms never
revisited anywhere falls from 16% to 15%. Not one of the thirty-five new atoms
misses its R1 window, and the pre-A1 reinforcement blocker does not move at all.

Every headword was checked against all 157 existing lessons before it was
written, in both directions and with the forward-reference detector's own
word-boundary regex — zero hits, in either direction. So none of the thirty-five
re-teaches anything, and none of them makes an earlier page point forward at a
word it has not met: the corpus forward-reference figure holds at its ceiling of
500, and the rule-statement count holds at 30.

Two threads run the length of the seven chapters rather than sitting in one
lesson each. The first is the double inheritance the preface promises and the
book has mostly shown a word at a time: here it is a standing division of
labour. कपड़ा against *vastra*, नमक against *lavaṇa*, मेहमान against *atithi*,
आभार above धन्यवाद above शुक्रिया, सफ़र beside *yātrā* — the inherited word does
the daily work and the borrowed one does the ceremony, or the other way round,
again and again, until it stops being a fact about five words and becomes a fact
about the language.

The second is wearing-down as a mechanism rather than an anecdote. *karṇa* to
कान, *nāsikā* to नाक, *phulla* to फूल, *dīpa* to दीया, *satya* to सच: the same
collapse of a cluster and lengthening of the vowel that survives it, shown five
times in five different chapters so that the sixth time a reader meets it they
recognise the machine instead of memorising the word.

Chapter 48 collects on a debt: बिलकुल carries the Arabic article *al-*, which
the reader already has, unrecognised, inside अलविदा from chapter 1. Chapter 50
does the same for प्रणाम, whose *nam-* is the root standing inside both नमस्ते
and नमस्कार — one gesture the reader has been performing since the first page,
now with a third height of formality attached to it.

Every one of the thirty-five is **voice**: nothing in these chapters needs eyes,
so all seven chapters are learnable end to end at the wheel.

## Unreleased — Chapter 40: Pointing, and Asking

Six words and the pattern behind them: यह वह यहाँ वहाँ कौन कहाँ

Until now the reader could NAME things and not point at them. With these they
can: *this one*, *that one*, *here*, *there*, and the two questions that matter
first — *who?* and *where?* Everything already in the book becomes a sentence
they can use.

The seventh lesson is the reason these six are one chapter. They are not six
words, they are **y- / v- / k-** — three beginnings on the same ending, and changing
the front walks the meaning from near to far to a question. A reader who sees
that once does not have to be taught the third member of the next family they
meet; they will work it out.

The whole chapter is **voice**: nothing in it needs eyes, so it is learnable end
to end at the wheel.

## Unreleased — eight characters, two of them with a real pen path

Eight script segments, one character each, in chapters 6-13: म न अ आ ◌् ◌ा त स

Hindi already had eleven writing lessons and **not one of them reached the page**:
they sit in chapters 1-5, which are handwritten and protected from generation, so
they rendered only in the answer key. Meanwhile chapter 1's own prose promises
*"Each lesson introduces the letters its word needs."* These eight are the first
Hindi script lessons a reader of the book actually meets.

**अ and आ carry a cited stroke order** — the numbered pen path, the pen-lift
count, the source and its variation note, all read from `devanagari.json`. Nine
of Devanagari's twenty-eight letters are cited and these are the two that fall in
the letter ledger's first positions. The other six ask for tracing instead and
say plainly why, exactly as the Dravidian tracks do.

Every vowel sign shows its worked example from the script file — **न + ◌ा = ना**
*nā* — which is the whole idea of a sign that cannot stand alone, and every letter
shows its component breakdown.

Also: the eleven existing `HI-W*` writing lessons now declare `delivery: script`,
which they always were. Adopting the marker made the corpus check
*"the script strand is declared, not inferred"* apply to Hindi, and it
immediately found all eleven.

## Unreleased — 18 words a reader can now say

Added `romanization` to 18 lessons that had none, so their headwords become
HL11 *exposure* — something the reader is shown and can use — rather than script
they are stuck on. Each is recovered from the pronunciation the lesson already
gives in its own prose, then checked against the headword's script so a wrong
grab cannot pass. Nothing is transliterated: a mechanical romanization of this
script disagrees with its own authors often enough to teach mispronunciations.

## Chapters 36–39 — the pre-A1 noun tranche, and what it measured

Sixteen everyday nouns, four chapters of four lessons. This wave was authored as
a **measurement probe** as much as content: `src/level-gate.ts` reported that no
track had attained even pre-A1, and that Hindi's binding blockers were
`vocabulary` and `reinforcement`. The question was whether track-local lessons
attached to pre-A1 spine nodes through the `HI-EXT-*` mechanism actually move
those numbers.

**They do.** Measured with `buildCurriculumGapReport` before and after:

| `levelGate.tracks[hindi]` | before | after |
|---|---|---|
| `vocabulary` (whole track, any level) | 70 | 86 |
| headwords at or below pre-A1 | 34 | 50 |
| `vocabulary` shortfall (target 300) | 266 | 250 |
| `reinforcement` shortfall | 39 | **4** |
| `attained` / `inProgressAt` | null / pre-A1 | null / pre-A1 |

The vocabulary criterion counts **distinct headwords on `word`/`phrase` lessons
whose spine node sits at or below the level**. Sixteen new `word` lessons in
path segments pointing at pre-A1 nodes therefore moved it by exactly sixteen —
one per lesson, no more. That is the honest exchange rate: **closing pre-A1 on
vocabulary alone needs another 250 lessons of this shape**, and nothing about
the mechanism makes it cheaper. Lesson count, not authoring cleverness, is the
whole cost.

The reinforcement number is where the leverage was. All **39** pre-A1 atoms the
continuity ledger reported as revisited fewer than twice are now practised at
least twice — the yes/no words, कृपया and माफ़ कीजिए, the family and body
lessons, पानी/रोटी, शुभ रात्रि, and the entire W01–W05 writing series, rescued
inside "The letters in this word" sections that also carry their own word. The
remaining 4 are atoms introduced by these chapters' own final lessons, which no
later lesson exists to revisit.

**Where the four chapters hang, and what would not hang honestly.** The seven
pre-A1 spine nodes are all **speech acts** — greet, thank, respond, exchange
names, check wellbeing, request, take leave. Four of them take nouns without
strain: MEET-GREET holds what you are received into and offered (घर, दरवाज़ा,
कमरा, कुर्सी); POLITE-REQUEST-REPAIR holds what you ask for (चाय, दूध, खाना,
किताब); CHECK-WELLBEING already held सिर and हाथ, so body words extend it
(आँख, दाँत, पैर, पेट); EXCHANGE-NAMES holds the people you introduce (दोस्त,
बच्चा, आदमी, औरत). **There is no pre-A1 node for naming a thing in front of
you.** मेज़ (table), खिड़की (window), सड़क (road), गाड़ी (vehicle) and कागज़
(paper) were on the shortlist and were dropped rather than forced: no honest
reading of a greeting-or-request node covers them. If pre-A1 is to reach 300
headwords, the spine needs a node for plain naming. That is a finding about the
spine, not about the words.

Not duplicated: पानी and रोटी (Ch. 15), नाम (Ch. 2), पिता/माता and भाई/बहन
(Ch. 12) were already taught, so the brief's suggested list shrank by six before
authoring began. माँ was dropped for the same reason — Ch. 12 covers "mother".

**Gender is the atom, not a footnote.** Every lesson teaches its noun's gender
where the noun is taught, because Hindi gender is not recoverable from meaning
and only partly from shape. The chapters build and then break the rule of thumb
deliberately: Ch. 36 establishes long **-ā** masculine / long **-ī** feminine
and shows घर outside it; Ch. 37 breaks it with **चाय** and **किताब**, feminine
with no **-ī** to show for it; Ch. 38 sets **आँख** (f.) against **दाँत** (m.),
identical in shape; Ch. 39 breaks it the other way with **आदमी**, masculine
*with* an **-ī**, and shows **दोस्त** taking its possessive from the person
rather than the word. The thread is carried by **मेरा / मेरी**, and it finally
keeps the promise Ch. 2 made in writing: *"You'll meet merī at your first
feminine noun."*

**Corrections made during authoring, all against sources.** Several attractive
etymologies did not survive checking and are recorded here so they are not
reintroduced: गृह is **not** from √*grah* "to seize" (that is a grammarians'
root-assignment; the real root is \**gʰerdʰ-* "enclosure"); पैर is **not** from
*pāda* (पाँव is); पेट is **not** demonstrably from *peṭaka* "basket" (Turner
notes a resemblance and derives nothing; the mainstream account is a Dravidian
loan); Persian *bacca* is a **sister** of Sanskrit *vatsa*, not a descendant;
the Hebrew *ʾādām* / *ʾadāmāh* "ground" link is Genesis making a deliberate pun,
not the etymology; the Greek cognate of अक्षि is *ósse*, **not** *ophthalmós*;
English *dough* is unrelated to दूध (*doughty* is the real cousin); and the
tea *chá*/*tê* split is about which Chinese **port** a trader used, not overland
versus sea, since Portuguese *chá* came by sea. One usage correction too:
**कृपया** is written register — signage and announcements — and **एक चाय
दीजिए** is what is actually said, so the chapter says so.

**Fonts.** Latin Modern has no Greek, Hebrew or Han. The first draft quoted
Greek *odoús* and *ósse*, Hebrew *ʾādām* and *ʾadāmāh*, and the Chinese
character for tea in their own scripts; a XeLaTeX run reported **28** dropped
glyphs against a track baseline of **zero**. All are now romanization only. The
39-chapter book compiles to 164 pages with **zero** missing characters.

**Gates.** Zero chapter-gate findings for 36–39; each payoff assesses every atom
its own chapter introduces plus its reach-back list. Zero duration violations
(every lesson computes under 300s). Zero new atom-budget violations — the one
Hindi ramp violation, `HI-C22-gyarah-bees`, pre-dates this branch. Zero
script-ramp violations. `attained` is still `null` and honestly so: pre-A1 is
250 headwords away.

## Chapters 34–35 — the second verb tranche, and thirteen rescued atoms

Hindi sat at **4 of the shared spine's 40 core verbs**, the thinnest coverage of
any large track, while Spanish (21), Latin (16) and Portuguese (15) had already
taught the eight verbs below. These two chapters bring Hindi in, so each of the
eight becomes a **four-way** cross-language join — and, unlike the three tracks
already there, one that reaches across language families.

| Lesson | Concept | Word |
|---|---|---|
| `HI-C34-sochna` | `VERB-THINK` | सोचना |
| `HI-C34-samajhna` | `VERB-UNDERSTAND` | समझना |
| `HI-C34-padhna` | `VERB-READ` | पढ़ना |
| `HI-C34-likhna` | `VERB-WRITE` | लिखना |
| `HI-C35-lena` | `VERB-TAKE` | लेना |
| `HI-C35-puchna` | `VERB-ASK` | पूछना |
| `HI-C35-madad` | `VERB-HELP` | मदद करना |
| `HI-C35-pasand` | `VERB-LIKE-LOVE` | पसंद |

**Two chapters, not one.** Eight one-verb lessons introduce seventeen atoms
against `maxNewAtomsPerChapter: 12`; the last wave of tracks all broke that
ceiling. Splitting is the resolution rather than raising the budget, so chapter
34 introduces **8** atoms and chapter 35 introduces **9**, each with its own
capability and its own payoff. Hindi reports **zero** ramp chapter violations.

**Every payoff reaches back.** The corpus was measured at 51% of taught atoms
never revisited, median zero. A tranche that only reviews itself adds to that
pile, so each of the eight lessons re-practises at least one atom from an
earlier chapter — thirteen in all, and **all thirteen had never been practised
again anywhere in the corpus before this branch**: नहीं (7), माफ़ कीजिए ×2 (9),
पानी and रोटी (15), उम्र and कितने साल के हो (19), एक–पाँच (6), छह–दस (21),
कुत्ता (23), शाम (29), the preposed ि (W03) and मेरा नाम (W04). The reach-back
is teaching, not name-checking: *maiṁ nahīṁ samajhtā* is built on chapter 7's
negator, *paṛhnā*'s drill is reciting one to ten because that is literally what
the verb once meant, and *madad karnā* finally explains the join the learner
used unknowingly in *māf kījiye*.

**Hindi's own signature, three times over.**

- **पसंद is not a verb of liking — it is not a verb at all.** It is a Persian
  noun/adjective, and *mujhe roṭī pasand hai* means "to me, bread is pleasing":
  the liker sits in the dative **मुझे** and the thing liked is the grammatical
  subject that **है** agrees with. Spanish arrived at the identical shape in *me
  gusta el café* and Italian in *mi piace*, in another family, with no borrowing
  in either direction. Spanish is supplied as self-contained contrast, never as
  assumed knowledge, so the chapter stands alone in a single-language PDF.
- **मदद करना opens the conjunct verb**, not just one word. Arabic *madad* (root
  *m-d-d*, "to stretch out" — help as a hand extended) plus native **करना**, and
  only *karnā* ever conjugates. That is the mechanism by which Hindi absorbed
  centuries of Persian and Arabic vocabulary without ever bending a foreign verb:
  the loan stays a frozen noun and native grammar does the work.
- **पढ़ना means read *and* study**, because Sanskrit **पठति** *paṭhati* meant
  "to **recite aloud**" in a tradition whose oldest texts are *śruti*, "that
  which is heard." The retroflex flap **ढ़** — ढ under the same nuqtā met in
  *māf* — gets its own pronunciation note.

**Etymology carries the rest, and names its own gaps.** *sochnā* ← *śocati* "to
burn, to grieve," which is why **सोच** still means thought *and* worry;
*samajhnā* ← *sam-* + *budh-* "to wake," the root of **Buddha**, **bodhi** and
English **bode/forebode/forbid**; *lenā* ← *labhate* with the *-bh-* eroded and
the book-borrowed **लाभ** preserving it; *pūchnā* ← *pṛcchati* on \**prek-*, the
verb English kept as **pray**, **prayer** and **precarious** after its own
inherited *frignan* died out; *likhnā* ← *likhati* "to scratch," beside Latin
*scrībere*, Greek *gráphein* and Old English *wrītan* — **four separate roots**,
so the shared thing is the metaphor, not the word. Three dead ends are stated as
dead ends rather than papered over: *śuc-* has no secure English cousin,
*paṭh-* has no secure Indo-European ancestry at all, and English inherited
nothing from Arabic *m-d-d*.

**Wiring.** `HI-PATH-029` is a third `SPINE-SAY-WHAT-I-DO` segment carrying the
eight, and the eight concepts drop out of that node's `omits` (38 → 30). One
pre-existing hole had to be closed first: **`HI-C05-bolta-hun` — the lesson that
teaches the present habitual every one of these verbs runs on — was on no
realization path at all**, so naming it as a prerequisite produced a
`curriculum-prerequisite-omitted` error. It now sits in `HI-PATH-028` beside
*bolnā*, carried by the new `HI-EXT-028-LANGUAGE-SPECIFIC` extension. No lesson
moved relative to another. `chapters.json`, `core/book-generation.json`,
`book.tex`, the generated chapter TeX and the ch34/ch35 narration all follow.

All eight lessons are schema v2, computed at **258–297 s** against the 300 s
ceiling, and both chapters are **fully drivable** (`drivablePrefix` 4 of 4):
the four inline-letter sections derive as `sight` for the book and detach
cleanly, so `coreModality` stays `voice`. The book compiles under XeLaTeX at
124 pages with **zero** missing characters and zero errors; the single overfull
and single underfull box both pre-date these chapters.

Corpus snapshot tests are deliberately left failing rather than re-pinned:
book chapters 399 → 401, declared chapters 301 → 303, lessons 1249 → 1257,
A2 lessons 153 → 162, atoms taught 1519 → 1536, `universallyMissing` holds at
15 (all eight were already taught elsewhere), `meanCoveredPercent` 17 → 18,
`payoffsNotClosed` 0 and `payoffsNotRepresentative` 24 both unmoved. Hindi goes
**4 → 12 of 40** core verbs.

## Joining the cross-language verb corpus

Hindi already taught five verbs, but every one of them sat under a
**namespaced** `concept_tag` (`HI-VERB-BOLNA`, `HI-VERB-HONA`, …). Namespaced
ids are language-local, so Hindi's *bolnā* and Bengali's *bôlā* were unrelated
concepts and the track contributed **zero** verbs to the cross-language join
while eighteen other tracks were already realizing the canonical `VERB-*` set.
This retags four of them and rewires the realization path that follows.

- **Retagged**, metadata only — no lesson prose touched:
  `HI-C03-hun` (हूँ) `HI-VERB-HONA` → **`VERB-BE`**;
  `HI-C05-bolna` (बोलना) `HI-VERB-BOLNA` → **`VERB-SPEAK`**;
  `HI-C05-karna` (करना) `HI-VERB-KARNA` → **`VERB-DO-MAKE`**;
  `HI-C05-rahna` (रहना) `HI-VERB-RAHNA` → **`VERB-LIVE`**.
- **`HI-C04-milenge` keeps `HI-VERB-MILNA` deliberately.** `VERB-MEET` exists
  and the tag looks like an easy fifth, but the lesson does not teach "to
  meet" — it teaches the *future* **मिलेंगे** (*milenge*, "we will meet") as
  the second half of the farewell *phir milenge*. Filing it under `VERB-MEET`
  would have raised the coverage number by describing the lesson falsely, so
  it stays namespaced and the gap stays visible.
- **Chapter 5 now sits in the path at all.** `HI-C05-bolna`, `HI-C05-rahna`
  and `HI-C05-karna` were absent from `curriculum.json` entirely; they are now
  `HI-PATH-028` under `SPINE-SAY-WHAT-I-DO`, inserted between the Chapter 4
  segment and the Chapter 6 segment, in book order. No lesson moved relative
  to another and no chapter was reordered.
- **`HI-PATH-008` was split in place, not moved.** A canonical tag is owned by
  a spine node, so `VERB-BE` obliges `HI-C03-hun` to sit in a
  `SPINE-SAY-WHAT-I-DO` segment. Rather than relocate the lesson — which would
  have changed where a learner meets हूँ — the existing two-lesson segment was
  split at its seam: `HI-PATH-008` keeps `HI-C03-hun` (now
  `SPINE-SAY-WHAT-I-DO`, still carrying `HI-EXT-008-LANGUAGE-SPECIFIC`), and
  the new `HI-PATH-027` holds `HI-C03-thik` under `SPINE-CHECK-WELLBEING` at
  the very next position. The walked order of the path is byte-identical.
  This also matches every sibling track: all seventeen others that realize
  `VERB-BE` place that lesson directly in `SPINE-SAY-WHAT-I-DO`.
- **Ledgers rewired to match**: `SPINE-SAY-WHAT-I-DO.segments` `[]` →
  `["HI-PATH-008", "HI-PATH-028"]`; `VERB-BE`, `VERB-SPEAK`, `VERB-DO-MAKE`
  and `VERB-LIVE` dropped from that node's `omits` (42 → 38 concepts omitted);
  `SPINE-CHECK-WELLBEING.segments` retargeted `HI-PATH-008` → `HI-PATH-027`.
  `relocates` stays empty — no lesson carries a `spine_node` pin that
  disagrees with its placement.
- **Corpus effect**: Hindi covers 4 of the core 40 verbs (0% → 10%);
  `tracksWithNoCoreVerb` 4 → 3; `meanCoveredPercent` 13 → 14. Because
  `SPINE-SAY-WHAT-I-DO` is an A2 node, the Hindi track's `reach` becomes
  **A2** and four Hindi lessons are now levelled A2 (`pre-A1` 654 → 653
  corpus-wide, A2 122 → 126, ramp-to-A1 951 → 950). The corpus snapshot pins
  in `tests/levels.test.ts` and `tests/verbs.test.ts` are deliberately left
  failing rather than re-pinned here.

## Chapter capability ledger (HL05)

- Added `chapters.json`, the hand-authored capability ledger: one entry per
  chapter carrying a first-person `canDo`, the shared-spine nodes the chapter
  realises, and a `payoff` naming the lesson that proves the claim.
- Thirty of the thirty-three chapters are authored — Chapters 1, 2, and 6
  through 33. Titles and labels for Chapters 6–33 are copied exactly from
  `core/book-generation.json`; Chapters 1 and 2 have no generator target and
  take their names from the hand-authored `book/chapters/ch01`–`ch02` sources.
- Chapters 3, 4, and 5 are deliberately **absent**. Every lesson in them is
  still schema v1 with no `practises.knowledge`, so no payoff could name a real
  knowledge atom. The gap is recorded as debt in the file's own note rather
  than filled with a stub, because a stub would satisfy the gate while
  destroying the signal it exists to carry.
- Chapters 1 and 2 have no schema-v2 terminal consolidation lesson either:
  their `HI-C01-practice` and `HI-C02-practice` lessons are schema v1. Their
  payoffs therefore fall back to the chapter's last lesson by `sequence`, which
  in both cases is a Devanagari writing lesson — `HI-W02-ka-ta-mouth-order`
  and `HI-W05-write-namaste`. Both are recorded as `kind: task` and described
  as hand-writing work, not as spoken dialogue they are not.
- Every `payoff.assesses` list is exactly the payoff lesson's own declared
  `practises.knowledge`, so no chapter claims an atom its lesson never
  exercises.
- Four chapters will sit below the 0.5 representativeness threshold when the
  HL05 gates land: Chapter 1 (3/9), Chapter 2 (2/12), Chapter 6 (1/4), and
  Chapter 32 (1/3). Each is a chapter whose terminal lesson is narrower than
  the chapter as a whole; the fix is a real consolidation lesson, not a wider
  claim in this ledger.

## Warning-free 33-chapter book

- Devanagari, Arabic, and Cyrillic use explicit static bold and italic faces;
  script commands also degrade safely to Unicode text in PDF bookmarks.
- Four hand-authored practice sections have stable unique labels. Natural page
  bottoms, a modest line-break reserve, and one concise running title remove
  every horizontal and vertical layout warning without dropping content.
- The twelve open-right chapter versos and two front-matter versos are now
  truly empty instead of carrying orphaned running headers and page numbers.
- A forced 114-page XeLaTeX build has zero missing glyphs, overfull or
  underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings,
  or font warnings. Every rendered page was inspected again.

## Canonical book Chapters 6–33

- Fifty-one lessons now use schema v2 with explicit shared-spine placement,
  prerequisite-safe sequence numbers, honest sub-five-minute duration budgets,
  typed teaching blocks, and machine-checkable knowledge boundaries.
- Forty lessons across Chapters 6–33 generate twenty-eight LaTeX chapters from
  the same canonical AST loaded by Language Ladder. Per-chapter source hashes
  make app/book drift a test failure.
- Eleven writing companions in Chapters 1–2 also migrated to schema v2, but
  remain embedded in the hand-authored opening chapters so script appears only
  when the learner needs it rather than as a detached alphabet course.
- Devanagari, Arabic, and Cyrillic examples use vendored fonts. The shared
  renderer also handles stacked accents and historical-linguistics notation;
  the 114-page XeLaTeX build has zero missing glyphs.
- Canonical coverage now runs continuously through Chapter 33. The curriculum
  report remains at zero duration violations and zero unknown prerequisites,
  while lesson chapters missing from books fall from 104 to 76.

## Sub-five-minute canonical sequence

- All 40 Hindi duration violations are resolved without removing vocabulary,
  grammar, script, register, source qualification, or etymology content.
  Twenty-nine lessons already computed below the limit and now declare honest
  four-minute budgets.
- Five long writing lessons became eleven prerequisite-ordered steps through
  `HI-W01-na-ma`, `HI-W02-ka-ta-mouth-order`, `HI-W03-preposed-i`,
  `HI-W04-write-mera-naam`, `HI-W05-conjuncts`, and
  `HI-W05-write-namaste`. The progression now separates the head-line from its
  first letter bodies, inherent vowel from mouth-order writing, basic mātrās
  from preposed **ि**, letter history from phrase assembly, and virama from
  conjuncts and whole-word writing.
- Seven other long lessons gained focused supports:
  `HI-C06-tin-char-history`, `HI-C06-paanch-nasal`,
  `HI-C19-age-grammar`, `HI-C21-six-nine-ten-history`,
  `HI-C23-billi-history`, `HI-C24-pila-history`, and
  `HI-C32-evening-register`.
- Downstream prerequisites and reviews now pass through the immediately
  preceding skill. The 24 rewritten or new lessons compute between 114 and 293
  seconds, and the full corpus still has zero unknown prerequisite ids.

## Chapter 6 — Numbers 1–5, and Sanskrit wearing down

- **Chapter 6 authored** (`HI-C06-numbers-1-5`): *ek, do, tīn, chār, pāṁch*.
- Each number is set beside its ancestor **with the Prakrit stage shown in the
  middle**, since that is where most of the wearing-down actually happened rather
  than in a jump straight from Sanskrit.
- **Three and four descend from the NEUTER forms**, which the anchor chapter set
  up: *trī́ṇi* → Pkt *tiṇṇi* → **tīn**, and *catvā́ri* → *cattāri* → **chār** —
  not the masculine *tráyaḥ*/*catvā́raḥ*, which left no descendant here. The
  *tīn* chain is spelled out as a **two-step trade**: the *r* is lost and the *ṇ*
  **doubles** to compensate, then that double simplifies and the **vowel
  lengthens** instead. Each step swaps one kind of weight for another.
- **The cross-family payoff is the reason this chapter exists.** This is exactly
  the **Latin → Spanish** erosion the Romance tracks keep tracing, running on the
  other side of the world at the same time, and the lesson puts them in one
  table: *quattuor* → *cuatro* beside *catvā́ri* → *chār*. The **same PIE word**,
  two continents, two thousand years of wear each. (Verified that the Romance
  tracks really do trace *quattuor* → *cuatro* before citing them.)
- *pañca* → *pāṁch* gets its own section, because the nasal didn't vanish — it
  **migrated into the vowel**, which is precisely what the chandrabindu **ँ**
  records. The learner is asked to say both aloud and hear it move.
- **Writing-track claim corrected before shipping.** A first draft said the
  learner "can already read **एक**" from `HI-W01`–`W03` and named "the ए letter"
  as taught there. **ए is taught in no writing lesson** — W01 gives न/म, W02 क/त,
  W03 the mātrās, W04 र/स, W05 the virama — and **च** and **द** are untaught too.
  A **second** draft then cited the **ी** mātrā as Lesson 3's — also false; W03
  teaches ा, े and **short ि** only. The list was finally built by **grepping the
  W-lessons** rather than recalling them: taught here are **क**, **त**, **न**,
  **र** and the **ा** mātrā; untaught are **ए, च, द, प**, the **ी** and **ो**
  mātrās, and the **ँ** — flagged "read them now, draw them when the track
  reaches them," matching the convention `HI-W02` already uses for ट.

## Writing track W01–W05 — hand-writing Devanagari

The first writing lessons for **any Indic track**. Ten Indic/Dravidian tracks had
reached Chapter 5 with **zero** writing lessons; this opens the first of them,
modelled on Arabic `AR-W01–12` and Russian `RU-W01–05`.

The arc is built so every lesson assembles vocabulary the learner already has,
and the last one produces the **first word of the course**:

- **`HI-W01-shirorekha-na-ma`** — the **shirorekhā**, the line Devanagari hangs
  from. शिरोरेखा is literally "**head-line**" (*śiras* + *rekhā*), and *śiras*
  descends from PIE \**ḱerh₂-* "head, horn" — the root behind Latin *cornu*,
  Latin *cerebrum* and English **horn**. Two counter-intuitive habits taken
  straight from the data: it is drawn **last**, and across a word it is **one
  bar** — both flagged in the lesson as the **common convention, not a rule**,
  since plenty of writers cap each letter as they go. Plus न and म and the
  commonest frame — **spine on the right, character on the left, bar across the
  top** — stated as *commonest* rather than universal, with the spineless
  minority (**द**, **र**) named up front.
- **`HI-W02-abugida-ka-ta`** — the **inherent vowel**, met head-on: **क is "ka",
  not "k"**. Which retroactively upgrades W01: नम was never "nm", it was *nama*,
  the first half of *namaste*. Names the script type — an **abugida** — and gets
  the coinage right, which is more interesting than the version usually told:
  *ʾä-bu-gi-da* names the first four **consonants of the old Semitic order** *and*
  the first four **Ge'ez vowel series** simultaneously, so unlike *alphabet*
  (which merely recites *alpha-beta*) the term **demonstrates the system it
  names**. Noted too that Ge'ez's own recitation order is *hä-lä-ḥä-mä…*, so this
  is a scholar's coinage, not the Ethiopian schoolroom sequence. Plus क, त, and
  the **five stop families** sorted by place of articulation, soft palate forward
  to the lips (क → च → ट → त → प), analysed by Indian phoneticians — Pāṇini is
  roughly 4th c. BCE — millennia before Europe attempted the same.
- **`HI-W03-matras-naam`** — **mātrās**. मात्रा means "a **measure**", from *mā-*
  "to measure" (PIE \**meh₁-*, whence *meter*, *measure*, *immense*, and *month*
  — the moon as the original measuring instrument). The load-bearing point is
  that a mātrā **replaces** the inherent vowel rather than adding to it. Teaches
  ा and े, builds **नाम** (Ch. 2), and closes on **ि** — *the only Hindi mātrā*
  written **before** the consonant and pronounced **after**. The lesson declines
  to explain why: the placement is an inherited Brahmi quirk, and an invented
  rationalisation would be worse than none.
- **`HI-W04-ra-sa-mera-naam`** — र, the **first spineless letter the learner
  writes** (with **द** named so it doesn't read as unique — द has in fact already
  been *read*, in Ch. 1's *dhanyavād*), and स. Carries a long-range
  etymology told with its uncertainty intact: *šin* → **Σ** → **S** going west is
  certain; **Brahmi**'s descent from the same Semitic family via **Aramaic** is
  the **leading view, not consensus**, so the lesson says "probably cousins," not
  "cousins." Builds **मेरा नाम**, and gets the word-boundary story the right way
  round: modern Hindi uses an **ordinary space** like English, and the break in
  the bar is the **consequence** of that space, not the device that marks it.
  (Continuous bars across a whole line belong to spaceless Sanskrit manuscripts.)
- **`HI-W05-virama-namaste`** — the **virama** ्, which kills the inherent vowel.
  विराम is "a **stopping**" (*vi-* + *ram-*) — and rather than the loose claim
  that it *is* the Hindi full stop, the lesson shows the word **grading Hindi's
  punctuation**: *pūrṇ virām* (complete stop) = full stop, *alp virām* (slight
  stop) = comma, *ardh virām* (half stop) = semicolon. Also names the mark
  honestly: *virāma* is the Sanskrit/Unicode term, **halant** is what everyday
  Hindi calls it. Then what actually happens in handwriting: the bare consonant
  **fuses** with the next into a **conjunct** (स् + त → स्त), a **spine-bearing**
  first consonant surrendering its spine — scoped that way because र, taught one
  lesson earlier, has no spine to surrender and uses **repha**/**ra-kāra**
  instead (र् + क → र्क, क + ्र → क्र), while क्ष/त्र/ज्ञ are simply irregular.
  Assembles **नमस्ते**, then reveals
  what the learner has been saying since lesson one: *namas* ("a **bow**", ←
  *nam-* "to bend") + *te* ("to you"). The greeting **is** the bow. *Namaskār* is
  the same *namas* + *kāra*, "making".

### Data honesty

Everything the learner is asked to **hand-write** comes from
`data/scripts/devanagari.json` — 28 letters and 12 marks. Every such letter,
mātrā and mark has a real entry with real `components` and `strokeOrder`;
**nothing was invented**.

**Six** letters appear somewhere in the text without entries — **ख ज ञ ट ण ष** —
and none of them is ever presented as something to draw. Two are cited as letters
(**ट** in W02's articulation chart, **ख** inside the word *shirorekhā*) and each
carries an explicit "read it now, draw it when its entry is written" note. The
other four only occur *inside quoted Hindi words* — ण in *pūrṇ virām*, ष in क्ष,
ज and ञ in ज्ञ — where the word is the point and the glyph is incidental.

To be precise about the guarantee, since a vaguer version of it was wrong in an
earlier draft of this entry: **every letter this track asks you to hand-write has
a real entry with real `components` and `strokeOrder`.** It is not true, and is
not claimed, that every Devanagari character appearing anywhere in the prose has
one.

The file is marked `complete: false` (its inventory covers the
greeting/self-introduction vocabulary), and its stroke orders are flagged as the
common handwriting convention rather than a standard — so **W01 says the same in
the lesson text**, since it would otherwise state "draw the bar last, one bar per
word" as flat rules when many writers cap each letter as they go.

### Scope note

`devanagari.json` serves **three** tracks — Hindi, **Marathi** and **Sanskrit**.
This PR opens only the Hindi one; the other two can mirror this arc against the
same data, and **नमस्ते is identical in all three**.

**Still blocked, stated rather than skipped:** among Indic scripts only
`devanagari.json` and `gujarati.json` exist. **Tamil, Telugu, Kannada, Malayalam,
Bengali and Gurmukhi have no letter data at all**, so no writing track is
possible for them until that data is authored — a real piece of work, not an
oversight.

## Chapters 3–5 — How-are-you, Farewells, First Verbs

- Three new chapters carry Hindi to Chapter 5, matching the leading tracks'
  greet→introduce→how-are-you→farewell→verbs arc. One word per lesson, atom-first,
  Devanagari inline; every root traced (`lessons/HI-C0{3,4,5}-*`,
  `book/chapters/ch0{3,4,5}-*.tex`). Concept tags reuse the universal `HL01`
  taxonomy (`QUESTION-HOW`, `STATE-HOW-ARE-YOU`, `PRONOUN-I`, `WORD-WELL`,
  `COURTESY-YOUREWELCOME`, `FAREWELL-*`); verbs are namespaced (`HI-VERB-*`).
- **Ch. 3 — How Are You**: *kaise* (how; the *k-* question family) → *āp kaise
  haiṁ?* (respect-as-plural) → *maiṁ* (I ← *ma-*) → *hūṁ* (am ← Sanskrit *asmi* →
  English **am**; the copula trio hūṁ/hai/haiṁ) → *ṭhīk* (fine — native, no
  European cognate) → *āpkā svāgat hai* (you're welcome; *su* + *āgata* = "well
  come") → practice.
- **Ch. 4 — Farewells**: *phir* → *milenge* (the future as an ending) → *phir
  milenge* (warm/native vs. Ch.1's formal Perso-Arabic *alvidā*) → *kal milte
  haiṁ* (*kal* = both tomorrow and yesterday ← *kāla*, cousin of Punjabi *akāl*)
  → *chaltā/chaltī hūṁ* (gendered "I'll be off") → practice.
- **Ch. 5 — First Verbs**: *bolnā* (the *-nā* infinitive; stem + ending) → *maiṁ
  hindī boltā hūṁ* (present habitual; *hindī* ← *sindhu*, the Indus) → *rahnā*
  (to live; the postposition *meṁ*) → *karnā* (← √kṛ — the root of *karma*,
  *namaskār*, and the name *Sanskrit*) → practice. Book compiles clean with
  XeLaTeX (0 missing chars, 0 undefined refs).

## Chapter 2 — Introducing Yourself

- New chapter around the introduction dialogue (*merā nām … hai / āpkā nām kyā
  hai? / …khushī huī*), atom-first, one word per lesson (`lessons/HI-C02-*`,
  `book/chapters/ch02-introductions.tex`), Devanagari taught inline. Every atom
  traced with its cross-family cousins (no glossing):
  - **नाम** nām ("name") ← Sanskrit *nāman* → English **name**, Latin *nōmen* →
    **noun**.
  - **मेरा** merā ("my") ← root *ma-* → English **me/my/mine**; agrees with the
    noun.
  - **है** hai ("is") ← Sanskrit *asti* → English **is**, German *ist*, Latin
    *est*, Spanish *es*.
  - **मेरा नाम … है** — **"my name is…"**; subject–object–verb order.
  - **आप / तुम** āp/tum — the three-level "you" (āp/tum/tū); *tum* ← *tū* →
    archaic **thou**.
  - **क्या** kyā ("what") ← stem *ka-* → English **what/who**.
  - **आपका नाम क्या है?** — **"what's your name?"** (verb still last).
  - **ख़ुशी** khushī — "pleased to meet you"; *khushī* ← **Persian**, Hindi's
    second vocabulary.
  - **practice** — the whole dialogue.
- Book compiles clean with XeLaTeX.

## Chapter 1 — Greetings (Devanagari taught inline)

- New Hindi track on the HL00 framework: one word per lesson, slug ids,
  atom-first, derivations shown, LaTeX book. Uses the **vendored** Noto Sans
  Devanagari font (static instance, loaded by relative `Path=` so local and CI
  builds match).
- **No reading course.** Per `HL00`'s inline-letters rule, Devanagari is taught
  *inside* each word lesson: a *"The letters in this word"* section introduces
  exactly the letters that word needs, so you learn to read the word and learn
  its meaning together. A Devanagari reference page is included in the book as a
  lookup, explicitly not a gated pre-course.
- Chapter 1 (`lessons/HI-C01-*`), built around greetings and Hindi's double
  inheritance:
  - **नमस्ते** namaste (Sanskrit root *nam*, "to bow"; *namaḥ* + *te* = "I bow
    to you") — teaches inherent *a*, the *e*-mātrā े, halant, and the स्त
    conjunct.
  - **नमस्कार** namaskār (*namaḥ* + *kāra*, "the making of a bow"; root *kṛ*) —
    adds क र and the long-*ā* mātrā ा.
  - **धन्यवाद** dhanyavād (*dhanya* "worthy" + *vāda* "a saying"; root *vad*) —
    the formal, Sanskritic "thank you"; adds ध य व द and the न्य conjunct.
  - **शुक्रिया** shukriyā (Persian ← **Arabic** *shukr*, root **sh-k-r** — the
    same word as Arabic *shukran*) — the everyday "thanks"; introduces Hindi's
    two vocabularies, and the *i*-mātrā ि + क्र conjunct.
  - **अलविदा** alvidā (Persian ← **Arabic** *al-widāʿ*, "the farewell,"
    carrying the article **al-**) — the independent vowel अ and ल.
  - **practice** — recap; the two heritages held side by side.
- Grounds each word against English and Arabic; foregrounds the Sanskrit vs.
  Perso-Arabic split as the key to Hindi. Book compiles clean with XeLaTeX.
