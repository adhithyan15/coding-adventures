# Changelog

## Opening chapter reading order (#12260)

- Add sequence 10 through 40 to the four legacy opening lessons in the book's
  explicit greet, thank, respectful yes, no dependency order.
- Remove Urdu's missing-sequence and false forward-prerequisite debt without
  changing the short opening lessons.

## 0.11.0 — 2026-08-12

Thirteen everyday-noun lessons across three new chapters (13–15), the
track's second pre-A1 vocabulary tranche (wave 6), continuing the
cross-track program: `vocabularyOf()` moves Urdu's pre-A1 vocabulary from
30 to 43 distinct headwords (43 → 56 overall). All seven pre-A1 spine
nodes were already realized after the first tranche; this wave adds depth,
not new spine coverage.

- **Ch. 13 — Four Colors** (`SPINE-POLITE-REQUEST-REPAIR`): lāl, safed,
  kālā, nīlā. Sorts the set into two adjectives that never change ending
  (lāl, safed) and two that agree with the noun's gender the way *merā*
  does (kālā/kālī, nīlā/nīlī) — the chapter's first new grammar atom,
  `UR-GRAMMAR-ADJECTIVE-AGREEMENT`. safed reuses the دل/hṛdaya two-roads
  shape, this time with English **white** itself as the third cousin
  (PIE \*ḱweyt-). kālā is inherited from Sanskrit *kāla* but that root is
  usually traced to Proto-Dravidian \*kār- rather than to
  Proto-Indo-European — the first inherited word in this book whose own
  source sits outside the Indo-European family. nīlā's own root is a dead
  end backward, but its cousin *nīlī* travelled forward through Persian,
  Arabic *an-nīl*, and Portuguese *anil* into English **aniline**.
- **Ch. 14 — Four Things You Wear** (`SPINE-POLITE-REQUEST-REPAIR`):
  qamīz, jūtā, ṭopī, koṭ. Names all three loan directions in one small
  set: Persian and Arabic into Urdu (qamīz, probably the same Late Latin
  *camisia* that gives English **chemise**, by two completely different
  roads); Urdu out into English (ṭopī, borrowed directly as **topee**,
  1825–35); and English straight into Urdu (koṭ, a homograph of the older,
  unrelated *koṭ* "fort"). jūtā traces to Sanskrit *yukta* "joined, yoked"
  from PIE \*yewg-, a cousin of English **yoke** and of Sanskrit's own
  word **yoga**. Two new letters: **ق** *qāf* and **ض** *zād* (qamīz), and
  **ت** *te*, contrasted by dot-count with already-known retroflex **ٹ**
  (jūtā).
- **Ch. 15 — Rain, Sun, Wind, Heat, Cold** (`SPINE-CHECK-WELLBEING`):
  bārish, dhūp, hawā, garmī, sardī. Sorts all five by how far their root
  reaches: two Persian dead ends (bārish, sardī), one inherited dead end
  (dhūp, the same honest wall as کان and روٹی), one loan of a loan (hawā,
  Persian's own borrowing from Arabic *hawāʾ* — a shape this book had not
  shown before), and garmī, whose root reaches Greek **thermós**
  (PIE \*gʷʰer-) — with English **warm** flagged honestly as only a
  disputed, not settled, cousin. One new letter, **گ** *gāf* (garmī, one
  extra stroke on already-known **ک**). sardī closes the tranche and
  reaches back several chapters to rescue orphaned atoms.
- Corrected two assumptions before writing: لال ("red") is a Persian loan
  with no further documented root, not a Sanskrit *lākṣā* ("lac dye")
  derivation as an initial search suggested; and English **warm**'s link
  to گرمی's PIE \*gʷʰer- root is only a proposed "distant doublet," not a
  settled cousin the way Greek *thermós* is — both lessons state the
  weaker or absent connection rather than the stronger claim that first
  seemed plausible.
- Reinforcement: every lesson's `practises.knowledge` reaches back to the
  preceding one to three lessons, and the tranche is threaded with
  rescue passes for atoms the corpus had never adequately revisited —
  `UR-CROSSLINGUAL-KHUDA-HAFIZ-SPELLING`, `UR-SCRIPT-MERA-NAAM-HAI`,
  `UR-CROSSLINGUAL-THIK`, `UR-DIALOGUE-TAKE-LEAVE`,
  `UR-GRAMMAR-KHUDA-HAFIZ-ELLIPSIS`, and sixteen more spanning chapters
  2–12. sardī, the tranche's final payoff, reaches back to دل's PIE
  \*ḱērd- argument and to `UR-REGISTER-TWO-ROADS-ONE-ROOT`. Pre-A1
  reinforcement debt (revisited fewer than twice) fell from 24 atoms to
  12; the remainder is this tranche's own tail lessons and a few
  pre-existing atoms whose thinness only became measurable once the
  longer track made the R3 window apply — debt left visible for the next
  wave, not papered over.
- Declared `chapters.json` chapters 13–15, each with a `canDo` and a
  payoff whose `assesses` list is the payoff lesson's own
  `practises.knowledge` verbatim. Wired all three into
  `core/book-generation.json` (chapter/output/script fields only,
  deriving title and label from `chapters.json`) and `book/book.tex`.
- No lesson exceeds three new atoms; no chapter exceeds twelve. The three
  pre-existing atom-budget violations (chapters 3–5) and the pre-existing
  chapter-4/3/6 budget overruns are untouched — outside this tranche's
  scope.
- Book compiles clean with XeLaTeX: 135 pages, zero errors, zero `Missing
  character` warnings. Two Devanagari citations and one bare IPA glottal
  stop (ʔ) were caught by the compile and replaced with plain
  transliteration, matching the track's transliteration-only convention
  for Sanskrit forms.

## 0.10.1 — 2026-08-08

- Replaced the explicitly temporary Naskh presentation with static Noto
  Nastaliq Urdu Regular and Bold faces in both the downloadable book and
  Language Ladder.
- Kept Naskh as a browser fallback rather than presenting its letter style as
  the Urdu course's normal typography.
- Pinned the official Noto distribution revision and file hashes in the shared
  font inventory, and verified Urdu coverage, OpenType shaping tables, the
  XeLaTeX book, and the production application bundle.

## 0.10.0 — 2026-08-08

Thirteen everyday-noun lessons across four new chapters (9–12), continuing the
cross-track pre-A1 vocabulary program and confirming the same measured
mechanism a further time: `vocabularyOf()` counts distinct `headword:` strings
1:1 with lessons, so thirteen new word lessons move Urdu's pre-A1 vocabulary
by exactly thirteen (17 → 30 distinct headwords at or below pre-A1). All seven
pre-A1 spine nodes are now realized.

- **Ch. 9 — People You'd Introduce** (`SPINE-EXCHANGE-NAMES`): bhāī, bahan,
  dost, khāndān. Chooses *merā*/*merī* correctly for each noun's grammatical
  gender, and sorts the four into inherited *bhāī*/*bahan* against Persian
  *dost*/*khāndān* — the same native-versus-borrowed split Chapter 8's verbs
  drew.
- **Ch. 10 — Four Words on the Face** (`SPINE-CHECK-WELLBEING`): kān, nāk,
  āṅkh, muṅh. Notices *kān* and *nāk* share the same three letters reversed;
  reads the plain nūn that nasalizes *āṅkh* and *muṅh* against the dedicated
  ں of *maiṅ*/*hūṅ*; counts three settled Indo-European cousins (*nāk*/nose,
  *āṅkh*/eye) against *kān*'s honest dead end.
- **Ch. 11 — The Heart**: dil. Traces *dil* through Middle Persian and
  Proto-Iranian back to the same PIE \*ḱērd- that gives Sanskrit *hṛdaya*,
  English *heart*, Latin *cor* and Greek *kardía* — *dil* and *hṛdaya* split
  at the Indo-Iranian fork, the same root by two roads, not opposite
  lineages, echoing *pūchhnā* and *porsīdan*.
- **Ch. 12 — Water, Tea, Milk, Bread** (`SPINE-POLITE-REQUEST-REPAIR`,
  previously unrealized — this is the resolution): pānī, chāy, dūdh, roṭī.
  Reuses Chapter 8's dative-experiencer frame on a plain noun (*mujhe roṭī
  pasand hai*) instead of an infinitive; sorts *pānī*/*dūdh*/*roṭī* as
  inherited against *chāy*'s Persian loan, and corrects a tempting false
  cognate between *dūdh* and English *dough*.
- Book compiles clean with XeLaTeX; all four new chapters wired into
  `book-generation.json`.

## 0.9.0 — 2026-08-07

- Added eight schema-v2 lessons in **two** chapters of four, never one of
  eight, so neither chapter exceeds the 12-atom `maxNewAtomsPerChapter` budget
  (both land at 11). Chapter 7, *Four Verbs of the Mind*: **سوچنا** *sochnā*
  (`VERB-THINK`), **سمجھنا** *samajhnā* (`VERB-UNDERSTAND`), **پڑھنا**
  *paṛhnā* (`VERB-READ`), **لکھنا** *likhnā* (`VERB-WRITE`). Chapter 8, *Four
  Verbs Between People*: **لینا** *lenā* (`VERB-TAKE`), **پوچھنا** *pūchhnā*
  (`VERB-ASK`), **مدد کرنا** *madad karnā* (`VERB-HELP`), **پسند** *pasand*
  (`VERB-LIKE-LOVE`). Eleven other tracks already taught these eight; Urdu is
  the twelfth, and `SPINE-SAY-WHAT-I-DO` now omits eight fewer concepts.
- Gave Urdu the argument only Urdu can make. **پوچھنا** *pūchhnā* is Sanskrit
  *pṛcchati* and Persian **پرسیدن** *porsīdan* is the **same** Indo-European
  root — one arriving by inheritance through India, the other borrowed through
  Iran — so Urdu's inherited verb and its loaned **پرسش** *pursish* are cousins
  that met again inside one language. **مدد** *madad* puts a second Arabic
  triliteral root, **م-د-د** "to stretch out, extend", beside Chapter 5's
  **ح-ف-ظ**, and the conjunct verb (noun + *karnā*) is named as the device that
  let Urdu absorb centuries of Persian and Arabic without ever conjugating a
  foreign word. *sochnā*'s inherited **سوچ** *soch* sits beside Arabic **فکر**
  *fikr*, both meaning a thought and, just as ordinarily, a worry.
- Taught **مجھے پڑھنا پسند ہے** — "to me, reading is pleasing" — as a
  dative-experiencer sentence with the liked thing as grammatical subject. The
  liked slot takes any **-nā** infinitive, because an Urdu infinitive doubles as
  a noun, so the payoff closes over the chapter's own verbs by substitution.
  Named as the sixth language in the course whose ordinary word for liking
  refuses to make you the subject, beside Spanish *gustar*, Italian *piacere*,
  Tamil *piḍikkum* and Bengali *bhālo lāge*.
- Kept the etymology honest, per lesson: *sochnā* ← Prakrit *soccadi* ←
  Sanskrit *śocati* "burns, grieves", with *śuc-* flagged as having **no secure
  English cousin**; *samajhnā* ← *sam-* + *budh-* "to wake", the root of
  **Buddha** and, through PIE \**bʰewdʰ-*, of English **bode** and **forbid**;
  *paṛhnā* ← Prakrit *paḍhaï* ← *paṭh-* "to recite aloud", flagged as having
  **no secure Indo-European ancestry**, with the original sense shown alive in
  **نماز پڑھنا**; *likhnā* ← *likhati* "scratches", beside three unrelated
  roots that made the same picture; *lenā* ← *labh-*, levelled by Prakrit
  against *de-* so *lenā* and *denā* rhyme; and *pasand*'s deeper root reported
  as **disputed** rather than asserted.
- Three new letters across eight lessons, one per lesson and none in the last
  four: **چ** (the **ج** body with three dots instead of one), **ھ**
  *do-chashmī he* (no sound of its own, aspirates its right-hand neighbour),
  and **ڑ** (**ر** wearing the same retroflex mark that rides on **ٹ**).
  **لینا** gives **ی** a third value, long *e*. Added `che-vs-jim-dots`,
  `do-chashmi-he`, `retroflex-flap-rre` and `majhul-e` to
  `pronunciation-reference.md`, with the shape-family notes to match.
- Reached back at two cadences. Every lesson names the preceding one to three
  lessons' atoms in `practises.knowledge`, across the chapter seam, so R1 and
  R2 close at zero new lessons; and both payoffs reach several chapters back.
  Rescued sixteen atoms that had never been revisited at any distance —
  `UR-SCRIPT-THIK`, `UR-SCRIPT-BE-LETTER`, `UR-SCRIPT-KYA`, `UR-SCRIPT-JANA`,
  `UR-SCRIPT-KAISE-KAISI`, `UR-SCRIPT-MAIN-HUN`, `UR-SCRIPT-JANNA-DOUBLE-NUN`,
  `UR-ETYMON-KYA-QUESTION-FAMILY`, `UR-ETYMON-KHUSH-PERSIAN`,
  `UR-ETYMON-KHUDA-PERSIAN`, `UR-ETYMON-HAFIZ-ARABIC`, `UR-ETYMON-ANA-TOWARD-GO`,
  `UR-ETYMON-JANNA-KNOW`, `UR-CHUNK-AAP-SE-MIL-KAR`, `UR-REGISTER-INDO-ARYAN-CORE`,
  and `UR-LEX-JANNA`. Track atoms never revisited fell from **24 of 59 (41%)**
  to **12 of 81 (15%)**.
- Declared `chapters.json` chapters 7 and 8. Chapter 7's payoff `UR-C07-likhna`
  assesses 11/11 of the chapter's atoms; chapter 8's payoff `UR-C08-pasand`
  assesses 7/11 = 0.64. Both are above the 0.5 representativeness floor and
  both are closed — no new `chapter-payoff-not-closed` or
  `chapter-payoff-not-representative` findings. Chapters 4 and 5 remain below
  the floor and chapter 1 remains without a capability; that debt is untouched.
- Every lesson routes its letter notes through the canonical
  `## The letters in this word` heading rather than hiding them in prose. That
  block is detachable, so all eight lessons keep a `voice` **core** modality and
  the HL-C41 report puts Urdu at **100% drivable** (nine lessons rescued by a
  detachable segment). The older HL08 manifest still derives `drivable` from the
  conservative whole-lesson modality, where the same honest labelling moves Urdu
  from 92% to 73%; that gap closes when `blockModality` flips true in
  `core/lesson-modality.json`.
- Generated `book/chapters/ch07-mind-verbs.tex` and `ch08-social-verbs.tex` and
  compiled the eight-chapter book with XeLaTeX: **53 pages, zero errors, zero
  `Missing character`, zero overfull or underfull boxes**. Urdu's
  `core/latex-warning-baseline.json` entry is still `null` (never seeded); the
  measured counts are zero across every tracked class. The PIE citation for
  *pūchhnā* uses \**prek-* rather than \**pr̥sḱéti* because U+0325 and U+1E31
  are both absent from Latin Modern Roman.
- No lesson exceeds three new atoms or three new glyphs; all eight sit under
  300 effective seconds. No table appears in any of the eight lessons.

## 0.8.0 — 2026-08-06

- Added five schema-v2 Chapter 6 micro-lessons, the track's first verbs:
  **ہونا** *honā* (`VERB-BE`), **جانا** *jānā* (`VERB-GO`), **آنا** *ānā*
  (`VERB-COME`), **بولنا** *bolnā* (`VERB-SPEAK`), and **جاننا** *jānnā*
  (`VERB-KNOW`). Urdu had taught zero verbs before this; `SPINE-SAY-WHAT-I-DO`
  is now realized rather than wholly omitted, and the track reaches A2.
- Taught the **-نا** *-nā* infinitive ending as a tool rather than a label:
  strip it and the stem falls out, so each new verb costs less than the last.
- Introduced the two simultaneous agreements the Urdu present makes — the
  participle for gender and number, the copula for person — on *jānā*, where a
  real stem exists to hang them on, and reused the frame unchanged on *bolnā*
  and *jānnā* to show the machine is already built.
- Kept the etymology honest per track: *honā* ← Sanskrit *bhavati* ← PIE
  \**bʰuH-* (English **be**, **build**, **future**, **physics**); *jānnā* ←
  *jñā-* ← \**ǵneh₃-* (English **know**, **notice**, **diagnosis**); *ānā* is
  the *ā-* "toward" preverb welded to the same *yā-* root that hardened into
  *jānā*'s **j-**. *bolnā* is flagged as a genuine dead end — the trail stops
  at Prakrit *bollaï* and the Sanskrit *brūte* link is proposed, not settled.
  *jānā* is flagged as having no English cousin at all rather than being given
  a decorative one.
- Placed the Persian and Arabic literary register beside the Indo-Aryan core on
  *bolnā* (homely *bolnā* against Persian-derived *guftagū*), which is also
  where *nastaʿlīq* is named as part of that same Persian inheritance.
- Cross-language comparison stays self-contained: Hindi is described as the
  other standard form of the same spoken language, never assumed as knowledge
  the reader already has.
- One new letter across the whole chapter: **ب**, taught against already-read
  **پ** as a dot-count contrast. **جاننا**'s doubled **ن** is taught as the
  only thing separating "know" from "go". Added `be-vs-pe-dots` and
  `geminate-nun` to `pronunciation-reference.md`.
- All five lessons derive `voice` modality, so Chapter 6 is fully drivable and
  the track's drivable share rises from 90% to 92%.
- Declared `chapters.json` chapter 6 with `UR-C06-janna` as payoff; measured
  representativeness 9/14 = 0.643, above the 0.5 policy threshold. Generated
  `book/chapters/ch06-core-verbs.tex` and compiled the six-chapter book with
  XeLaTeX: zero `Missing character` warnings, zero errors. Sanskrit forms are
  cited in transliteration only, because this book vendors no Devanagari face.

## 0.7.0 — 2026-08-06

- Added `chapters.json`, the HL05 chapter capability ledger, for Chapters 2–5:
  each declares a first-person `canDo`, the spine nodes it realises, and the
  payoff lesson that proves the claim.
- Every `payoff.assesses` list is the payoff lesson's own
  `practises.knowledge` set verbatim — nothing is claimed that the lesson does
  not already practise, and nothing is padded to clear a threshold.
- Chapter 1 is deliberately omitted. Its four lessons are still schema v1 and
  declare no knowledge atoms, so any payoff written for it would be invented
  rather than derived. The gap is left visible as debt.
- Measured payoff representativeness (assessed ÷ chapter-introduced atoms)
  against the 0.5 policy threshold: ch2 3/3 = 1.00, ch3 8/14 = 0.571,
  ch4 6/16 = 0.375, ch5 4/12 = 0.333. Chapters 4 and 5 sit below threshold
  because their word lessons introduce script, cross-lingual, and etymon atoms
  that the consolidating dialogue does not re-exercise; that is a content gap
  for a later revision, not something to paper over here.
- Chapter capability text describes what the shipped book actually renders. The
  Naskh fallback recorded in HL-U01 remains unfixed, so no chapter claims to
  teach Nastaliq letterforms.

## 0.6.0 — 2026-08-04

- Added four schema-v2 Chapter 5 micro-lessons for **خدا**, **حافظ**, spaced
  **خدا حافظ**, and a start-versus-end interaction.
- Secured the Urdu form before using its Persian and Arabic history as a bridge;
  mixed comparison preserves Urdu spacing and Persian joining.
- Extended the exact N+1/N+3/N+7/N+15 ledger through S35, with objective
  activities and a generated five-chapter book.

## 0.5.0 — 2026-08-04

- Added six schema-v2 Chapter 4 micro-lessons for *kaise/kaisī*, respectful
  **āp ... haiṅ**, the first-person **maiṅ ... hūṅ** frame, *ṭhīk*, the polite
  reply, and cumulative practice.
- Kept addressee agreement separate from honorific register, introduced the
  retroflex-aspiration sequence only inside **ٹھیک**, and made the Hindi bridge
  follow independent Urdu-script retrieval.
- Extended the sound-id reference and exact N+1/N+3/N+7/N+15 ledger through
  S31, with objective activities and a generated four-chapter book.

## 0.4.0 — 2026-08-03

- Added five schema-v2 Chapter 3 micro-lessons for **āp/tum/tū**, *kyā*, the
  full name question, the meeting response, and cumulative practice.
- Added objective activity contracts and prerequisite-closed knowledge atoms for
  the migrated Chapter 2 name frame and every new lesson.
- Generated Chapter 3 for the downloadable book from the same canonical lesson
  AST used by Language Ladder and extended the review ledger through S25.

## 0.3.0 — 2026-08-03

- Added the authoritative five-lesson session map with exact N+1, N+3, N+7,
  and N+15 review placements.
- Added an on-demand Urdu pronunciation and script reference that labels the
  current Naskh presentation fallback without weakening the Nastaliq goal.

## 0.2.0 — 2026-08-02

- Added the first downloadable LaTeX edition.
- Published Chapter 1 (greetings and responses) and Chapter 2 (giving your name)
  from the five dependency-ordered starter lessons.
- Added a B1-oriented track roadmap with Urdu-specific extension points.

## 0.1.0 — 2026-08-02

- Added the Urdu shared-spine pilot with five under-five-minute lessons.
