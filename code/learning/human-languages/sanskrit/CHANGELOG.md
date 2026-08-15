# Changelog

## Unreleased — Chapters 31–37: 35 more words on pre-A1 nodes

Sanskrit's pre-A1 vocabulary criterion moves 69/300 → 104/300. The shortfall
falls 231 → 196, a drop of exactly 35 — one per lesson, which is the proof that
every lesson landed on a pre-A1 spine node rather than merely on a good word.

Seven chapters appended after chapter 30, chained from `SA-C30-anjali`, one
pre-A1 node per chapter and all seven nodes reused a second time. Node reuse is
the mechanism here, not a workaround: there are only seven pre-A1 nodes in the
whole spine and the vocabulary criterion, not node coverage, is what binds.

- **Ch. 31 — The Sky and Its Lights** (`SPINE-MEET-GREET`): आकाशः, चन्द्रः,
  तारा, वायुः, हिमम्. Closes on the snow word so the reader can take
  *Himālaya* apart for themselves.
- **Ch. 32 — The Tree and What Grows** (`SPINE-EXCHANGE-NAMES`): तरुः, पत्रम्,
  तृणम्, लता, दारु. The tree lesson deliberately says nothing about दारु,
  because explaining the shared root there would make chapter 32's first page
  point forward to its last.
- **Ch. 33 — The River and the Road** (`SPINE-TAKE-LEAVE`): नदी, सेतुः,
  पर्वतः, पुरम्, समुद्रः — with पुरम् beside the Greek *polis* behind *police*.
- **Ch. 34 — More of the Body** (`SPINE-CHECK-WELLBEING`): बाहुः, ग्रीवा,
  अस्थि, रक्तम्, देहः, taking the body count to ten across two chapters.
- **Ch. 35 — In the House** (`SPINE-POLITE-REQUEST-REPAIR`): दुग्धम्, मधु,
  लवणम्, पुस्तकम्, शय्या. पुस्तकम् is flagged as the one borrowed word of the
  chapter, in from Persian *post* and back out to six modern languages.
- **Ch. 36 — More Short Replies** (`SPINE-RESPOND-BASIC`): अपि, सर्वम्, तथा,
  मन्दम्, सम्यक्. मन्दम् is a repair word, not a filler: it is what you say
  when somebody is talking faster than you can follow.
- **Ch. 37 — More Courtesy** (`SPINE-COURTESY-THANK`): शान्तिः, सादरम्,
  प्रसन्नः, अभिनन्दनम्, आशीर्वादः, closing on the *-vāda* that आशीर्वादः
  shares with धन्यवादः from the first page.

One new headword per lesson (HL14); reuse of already-taught words is unlimited
and is what makes the ramp gentler — R1 improves 0.2922 → 0.2895 with its
numerator held at 1106 while the denominator grows.

Every candidate was grepped against all 161 existing lessons with the
forward-reference detector's own word-boundary regex, and separately checked for
sitting **inside** an existing multi-word headword. Two words were caught only
by that second check and by nothing else: *atha*, which sits inside
`SA-C03-katham`'s romanization, and *aśvaḥ*, which sits inside
`SA-C28-day-after`'s *paraśvaḥ*. Neither appears in any lesson body. A third,
प्रीतिः, was dropped because `SA-C09-snihyati` already explains प्रिय at
length; teaching it later would have made chapter 9 point forward.

Every headword is spelled entirely from the 35 Devanagari glyphs the track's own
writing lessons teach, so the tranche adds **zero** script-closure violations
(sanskrit holds at 56) and zero exposure-exempted glyphs. `forwardReferences`
holds at its 500 ceiling and `ruleStatements` at 30. Book builds clean with
XeLaTeX: exit 0, 281 pages, 0 missing characters, 0 overfull, 0 underfull.

## Unreleased — Chapter 14: Pointing, and Asking

Six words and the pattern behind them: एतत् तत् अत्र तत्र कः कुत्र

Until now the reader could NAME things and not point at them. With these they
can: *this one*, *that one*, *here*, *there*, and the two questions that matter
first — *who?* and *where?* Everything already in the book becomes a sentence
they can use.

The seventh lesson is the reason these six are one chapter. They are not six
words, they are **a- / t- / k-** — three beginnings on the same ending, and changing
the front walks the meaning from near to far to a question. A reader who sees
that once does not have to be taught the third member of the next family they
meet; they will work it out.

The whole chapter is **voice**: nothing in it needs eyes, so it is learnable end
to end at the wheel.

## Unreleased — अ and आ gain their real pen path

The two segments for अ and आ shipped asking the reader to trace. Both letters
have a **cited stroke order** in `devanagari.json`, which the first pass did not
look for, so they now carry the numbered pen path, the pen-lift count and the
source. The other four still trace, because their letters are not cited.

The vowel-sign segment also shows its worked example — **न + ◌ा = ना** *nā* — and
every letter its component breakdown, both read from the script file. Devanagari
records these and the Dravidian script files do not, which is why the Sanskrit
segments are longer than their Telugu counterparts.

## Unreleased — the first 6 characters this book actually teaches

6 recognition segments, one character each, in chapters 8-13: म न अ आ ◌् ◌ा

Until now this track taught **no letters at all**. Every word was printed in its
own script and the reader had no way in — HL12's measurement put the track at A2
by aspiration and pre-A1 by attainment, with the script strand simply missing.

Each segment names one character, says what it carries, and shows it inside four
words the reader **already says** — so nothing new has to be learned in order to
do the recognising. That is HL12 §2.1's rule made concrete: a lesson may sit at
the frontier of decoding or of meaning, never both, because a reader who fails
one that is new in both cannot tell which one they failed.

They teach recognition and not writing, and that is a sourcing fact rather than a
pedagogical preference. This script has **no cited stroke order** in the corpus —
its own script file says *"Recognition only"* — and HL11 §5 forbids a pen path
without one, because a learner cannot tell an invented stroke order from an
attested one and will drill it for years. So the reader is asked to trace the
printed shape, which needs no source, and the book says plainly that where to
start the character and which way to travel are not written down yet.

Sanskrit takes six rather than eight because chapters 6 and 7 are already at or
over HL08's per-chapter atom budget -- 15 and 12 against a budget of 12 -- and one
more atom would have pushed them further. A chapter that cannot afford a letter
does not get one.

Each segment sits **last** in its chapter, after every word in that chapter that
contains its character — so it consolidates rather than pre-teaches, and it costs
the driving edition nothing: `drivablePrefixTotal` is unchanged corpus-wide.

## Unreleased — 22 words a reader can now say

Added `romanization` to 22 lessons that had none, so their headwords become
HL11 *exposure* — something the reader is shown and can use — rather than script
they are stuck on. Each is recovered from the pronunciation the lesson already
gives in its own prose, then checked against the headword's script so a wrong
grab cannot pass. Nothing is transliterated: a mechanical romanization of this
script disagrees with its own authors often enough to teach mispronunciations.

## Chapters 10–13 — the pre-A1 noun tranche (HL-C41 continuation) — 2026-08-08

Twelve everyday-noun lessons across four new chapters (10–13), continuing the
cross-track pre-A1 vocabulary program and confirming the same measured
mechanism a fifteenth time: `vocabularyOf()` counts distinct `headword:`
strings 1:1 with lessons, so twelve new word lessons move Sanskrit's pre-A1
vocabulary by exactly twelve (22 → 34 distinct headwords at or below pre-A1;
track-wide 41 → 53). The remaining level-gate blocker narrows from
`spine-nodes + vocabulary` to `vocabulary` alone.

- **Ch. 10 — What You'd Ask For** (`SPINE-POLITE-REQUEST-REPAIR`, previously
  unrealized — this is the resolution): पानीयम्, क्षीरम्, अन्नम्. Chose
  पानीयम् over the more textbook जल/उदक specifically because it is the
  attested ancestor of the Hindi/Gujarati/Bengali "water" words this program
  has already taught. Names the three grammatical genders every Sanskrit noun
  carries, tying अन्नम्'s neuter -म् back to Chapter 6's neuter numerals.
- **Ch. 11 — The People You'd Introduce** (`SPINE-EXCHANGE-NAMES`): मित्रम्,
  कुटुम्बम्, भ्राता, भगिनी. Chose भगिनी over स्वसृ because भगिनी, not the more
  "correct"-looking स्वसृ, is the verified ancestor of the daughter-language
  sister-words already taught. Flags कुटुम्बम् as a Dravidian loan **into**
  Sanskrit, later re-lent back to Kannada/Telugu/Tamil — the taproot running
  backwards.
- **Ch. 12 — The Face, Part One** (`SPINE-CHECK-WELLBEING`): अक्षि, कर्णः,
  मुखम्. Says why eyes take the dual number, and reports both scholarly
  accounts of मुखम्'s origin without choosing between them.
- **Ch. 13 — The Face, Part Two**: नासिका, हृदयम्. Names the PIE root हृदयम्
  shares with *heart*, *cor* and *kardia*, and closes the tranche's twelve
  nouns against their daughter-language descendants by group and by gender.
- Book compiles clean with XeLaTeX; syllable-break dots kept in roman font;
  all four new chapters wired into `book-generation.json`.

## Chapters 8 and 9 — the eight shared verbs — 2026-08-07

- Authored eight schema-v2 word lessons across **two** chapters of four,
  realizing the eight canonical verb concepts that every one of the other
  fifteen verb-bearing tracks already teaches: `SA-C08-cintayati` (`VERB-THINK`),
  `SA-C08-avagacchati` (`VERB-UNDERSTAND`), `SA-C08-pathati` (`VERB-READ`),
  `SA-C08-likhati` (`VERB-WRITE`), `SA-C09-grhnati` (`VERB-TAKE`),
  `SA-C09-prcchati` (`VERB-ASK`), `SA-C09-sahayyam-karoti` (`VERB-HELP`),
  `SA-C09-snihyati` (`VERB-LIKE-LOVE`). Sanskrit's core-verb coverage moves from
  **6/40 to 14/40**, and each of the eight widens a fifteen-way cross-language
  join to sixteen.
- **Two chapters, not one of eight.** Sixteen new atoms, eight per chapter,
  against a `maxNewAtomsPerChapter` of 12 — and each chapter closes on its own
  four with its own capability sentence and payoff.
- **Cited in the third person singular present, and by class.** Every lesson
  says which of the ten **गण** builds the stem, extending Chapter 7's three
  classes with class 10 (**चिन्तयति**, planting **-अय-**), class 6
  (**लिखति**, **पृच्छति**) and class 8 (**करोति**); class 9 returns on
  **गृह्णाति**, whose **-ना-** retroflexes to **-णा-** after the vocalic ऋ.
- **The taproot claim, made checkable.** पठति is the verb Hindi and Urdu
  *paṛhnā*, Bengali *poṛa* and Marathi *paḍhṇe* descend from; लिखति is behind
  *likhnā*, *lekhā* and *lihiṇe*; बुध्यते is behind Hindi *samajhnā* through
  Prakrit *saṁbujjhaï*; गृह्णाति wore down to Marathi *gheṇe*; पृच्छति to Hindi
  *pūchnā*; and Bengali still helps with साहाय्य itself.
- **Named the mechanism rather than gesturing at it.** A new atom,
  `SA-HISTORY-TATSAMA-TADBHAVA`, separates a word worn down by centuries of
  speech (*तद्भव*, *paṛhnā*) from one lifted back out of Sanskrit whole
  (*तत्सम*, *cintā*). Six later lesson blocks retrieve it. प्रिय is the case
  that does both at once: unchanged as *priya*, worn down as Hindi *piyā*.
- **Westward, and only where the derivation is real.** \**ghrebh-* is *grab*,
  *grip*, *grasp*; \**prek-* is Latin *precārī* → *pray*, *prayer*,
  *precarious*, and German *fragen* by the same *p*→*f* that made *pañca* into
  *five*; \**bheudh-* is **बुद्ध** and English *forbid*, *bode*; \**priHos* is
  *friend*, *free*, *Friday*; सहाय "one who goes with" is matched by Latin
  *comes*, *com-* + *īre*.
- **Said plainly where a link is absent or contested.** चिन्त् ← \**kweyt-*
  left English nothing (its living kin are Russian *čitat* and Lithuanian
  *skaityti*, both "to read"); लिख् has no secure English descendant either;
  पठति's own ancestry is a common account and not a settled one; and स्निह्'s
  tie to the *snow* root is argued over. Each carries the label rather than the
  claim — the discipline Chapter 6 set on *punch*.
- **Reinforcement at both cadences.** Every lesson practises atoms from the one
  to three lessons before it, across the chapter seam; the two payoffs reach
  several chapters back. Track-wide, atoms never revisited after introduction
  fall from **13 of 27 (48%) to 5 of 43 (12%)**, and three of those five are
  Chapter 6 numeral-grammar atoms no verb lesson could honestly claim; the other
  two are the final lesson's own, with nothing after them yet.
- Both payoffs assess **8 of 8** of their chapter's introduced atoms
  (representativeness 1.00).
- All eight lessons are `voice`: the track's drivable prefix runs 40 lessons
  deep, chapters 7, 8 and 9 are fully drivable end to end, and the letters each
  word needs are taught inline under *Sounds you'll need*.
- Registered `SA-PATH-011` and `SA-PATH-012` in `curriculum.json` and dropped
  the eight now-realized concepts from the `SPINE-SAY-WHAT-I-DO` omission
  ledger. The book compiles warning-free at nine chapters: no missing
  characters, no overfull or underfull boxes, no LaTeX warnings.

## Chapter 7 — The Core Verbs — 2026-08-07

- Authored six schema-v2 word lessons realizing `SPINE-SAY-WHAT-I-DO` with
  **canonical** verb concepts, where the track previously had only namespaced
  ones: `SA-C07-asti` (`VERB-BE`), `SA-C07-gacchati` (`VERB-GO`),
  `SA-C07-agacchati` (`VERB-COME`), `SA-C07-khadati` (`VERB-EAT`),
  `SA-C07-pashyati` (`VERB-SEE`), `SA-C07-janati` (`VERB-KNOW`). Sanskrit's
  core-verb coverage moves from 0/40 to 6/40 (15%); its three older verbs
  (`SA-VERB-KR`, `SA-VERB-VAD`, `SA-VERB-VAS`) stay counted as extras.
- Taught the **dhātu and gaṇa** system rather than a conjugation table: a verb is
  named by its root, and the class is what turns a root into a present stem. The
  chapter walks three of the ten classes in order — nothing inserted (*as-ti*), a
  vowel appended (*bhava-ti*, *gaccha-ti*, *khāda-ti*, *paśya-ti*), and a syllable
  planted inside the word (*jā-**nā**-ti*, class 9).
- Spent one whole lesson on **आगच्छति = आ- + गच्छति**, the *upasarga* prefix
  system in miniature: Sanskrit has no separate word for "come," and the same
  device already built **संस्कृत** back in Chapter 5.
- Followed the headwaters westward on every lesson, since Sanskrit is where the
  other tracks' etymologies come from: \**es-* and \**bheu-* are the two roots
  English melted into one ragged *am/is/are/be/been*, while Sanskrit keeps
  **अस्ति** and **भवति** apart; \**gwem-* is *come*, *venue*, *advent*; \**spek-*
  is *inspect*, *species*, *telescope*; \**gno-* is *know*, *notice*, *diagnosis*.
- Named the honest gap in the *eat* lesson: **खादति** is the everyday verb but
  is **not** the cognate of English *eat* — that is **अद्** / **अत्ति**, kept
  alongside it. Cognate and everyday word are different questions.
- Recalled Chapter 4's **पुनर्दर्शनाय** as the other half of the *see* verb:
  the present comes from **पश्**, everything else from **दृश्**, the root behind
  *darśana*. One verb, two roots, split by tense.
- Made `SA-C07-janati` the chapter payoff, assessing all 12 of the chapter's
  introduced atoms (representativeness 1.00).
- All six lessons are `voice`: the chapter's drivable prefix is 6 of 6, the
  track's first fully drivable chapter, and the letters each word needs are
  taught inline under *Sounds you'll need* rather than in a gated reading course.
- Registered `SA-PATH-010` in `curriculum.json` and dropped the six now-realized
  concepts from the `SPINE-SAY-WHAT-I-DO` omission ledger.

## Chapter capability ledger — 2026-08-06

- Added `chapters.json`, the HL05 chapter capability ledger, covering Chapter 6:
  the reader can say *eka, dva, tri, catur, pañca* with their dual and gendered
  forms and follow **पञ्च** outward into other languages.
- Made `SA-C06-pancha-travels` the chapter payoff — the chapter's last schema-v2
  lesson by sequence (360). It tracks *pañca* into Persian *panj-āb*, Greek
  *pente* in *pentagon* and *pentathlon*, and the qualified five-ingredients
  account of *punch*.
- Recorded `SPINE-COUNT-ONE-TO-FIVE` as the chapter's spine node, matching
  `SA-PATH-009` in `curriculum.json`.
- Omitted Chapters 1–5 rather than stubbing them: all 30 of their lessons are
  schema v1 and declare no `practises.knowledge`, so no payoff there could name
  atoms a lesson actually exercises. Their absence is the debt the HL05 gap
  report exists to measure.
- Measured payoff representativeness for Chapter 6 at 7/15 introduced atoms
  (0.47), just below the 0.5 policy floor. Sanskrit's chapter is the widest of
  the six — three lessons, fifteen atoms — and the terminal lesson exercises the
  *pañca* thread rather than the dual, the gendered paradigm, or the Grimm's-law
  material. Recorded, not padded.

## Book warning cleanup — 2026-08-03

- Replaced five duplicate recap anchors with stable chapter-qualified labels.
- Preserved Devanagari in PDF bookmarks while suppressing the font-only command
  there, and mapped the vendored static font to every requested shape.
- Let short lesson pages end naturally and shortened three running titles so a
  forced six-chapter build has no layout, bookmark, label, font, or glyph
  warnings.

## Canonical Chapter 6 publication — 2026-08-03

- Migrated all three number lessons to schema v2 with the shared
  `SPINE-COUNT-ONE-TO-FIVE` can-do node, explicit sub-five-minute budgets, and
  block-level knowledge closure.
- Generated the downloadable Chapter 6 from the same ordered lesson AST and
  source hash that Language Ladder loads, rather than maintaining another copy.
- Preserved Devanagari forms, gender/dual tables, sound-law comparisons, and
  romanized section bookmarks in the generated chapter.

## Sub-five-minute remediation — 2026-08-02

- Corrected nine declared five-minute estimates whose computed durations were
  already between 107 and 186 seconds.
- Split the 513-second numbers lesson into a 232-second forms/grammar lesson, a
  240-second cognate-and-sound-law lesson, and a 180-second *pañca* travel lesson.
- Preserved the masculine/neuter table, dual, east-west cognate map, PIE *kʷ*
  outcomes, Grimm's law, analogical *four*, and the qualified *punch* etymology.
  The shared report now measures zero Sanskrit duration violations.
- Updated the roadmap and session map to expose all three Chapter 6 lesson
  boundaries. Chapter 6's missing one-source book publication remains explicit
  in the shared backlog.

## Chapter 6 — Numbers 1–5, the anchor for the whole Indo-Aryan group

- **Chapter 6 authored** (`SA-C06-numbers-1-5`) — the first chapter past 5 in any
  Indo-Aryan track, and written deliberately as the **anchor** the other five
  hang from.
- ***eka, dva, tri, catur, pañca***, given as stems **and in both masculine and
  neuter** — *ekam, dve, **trī́ṇi**, **catvā́ri*** — because the modern
  languages mostly descend from the **neuter** forms, not the masculine ones. The
  lesson says so outright and tells the learner to keep that column in view,
  since all five daughter chapters depend on it. "Two" is *dváu* because Sanskrit
  has a **dual**.
- **The double payoff that makes this the anchor chapter.** These five are at
  once the **source** of every modern Indic number and the **cousins** of the
  English ones — not borrowings in either direction, but three branches of one
  family, and numerals are among the most stubbornly preserved words a language
  has. (With one honest caveat: *éka-* and *ūnus*/*one* are the same **root** with
  **different suffixes**, \**oy-ko-* against \**óynos* — relatives rather than the
  same word. The other four rows are the same word.)
- **The PIE \**kʷ* goes three ways**, and the lesson gives it a table:
  - **Latin** kept it as *qu-* (*quattuor*)
  - **Indo-Iranian** merged \**kʷ* into *k*, then **palatalised** it before front
    vowels (*catvā́ri*) — which is also where *pañca*'s *ñc* comes from
  - **Germanic** turned it into *hw-* (*what*, *who*)
- **A warning the first draft got wrong**, now taught explicitly: the **f- of
  *five* has nothing to do with the \**kʷ***. It is the initial \**p*, shifted by
  **Grimm's law** (*pater* → *father*, *pēs* → *foot*). And English **four** is
  irregular — by rule it should begin *hw-* like *what*, and has *f-* only
  because it was pulled into line with its neighbour *five*. Numbers influence
  their neighbours, which becomes load-bearing in the Marathi chapter.
- Closes on how far *pañca* travelled: **Punjab** (Persian *panj-āb*, "five
  waters" — Persian *panj* being the **Iranian** cousin of the Indic *pañca*),
  *pentagon*/*pentathlon* (Greek *pente*), and **punch** the drink for its five
  ingredients — the last flagged as the usual story with the rival *puncheon*
  derivation named, rather than asserted. The place-name is described as spanning
  **India and Pakistan** rather than as "an Indian state."

## Chapters 2–5 — Introductions, How-are-you, Farewells, First Verbs

- Four new chapters carry Sanskrit from Chapter 1 to Chapter 5, matching the
  leading tracks' arc. One word per lesson, atom-first, Devanagari inline; every
  root traced (`lessons/SA-C0{2,3,4,5}-*`, `book/chapters/ch0{2,3,4,5}-*.tex`).
  Concept tags reuse the universal `HL01` taxonomy; verbs namespaced (`SA-VERB-*`).
  As the **taproot**, each atom is presented as a *source* — pointing west
  (*aham*→*ego/I*, *asmi*→*am*, *vas*→*was*, *kim*→*what*) and east (into the
  Indo-Aryan daughters).
- **Ch. 2 — Introducing Yourself**: *nāma* (→ *name*) → *mama* (→ *me/my*) →
  *asti* (→ *is/est*; Sanskrit **keeps** the copula its Dravidian neighbours drop)
  → *mama nāma … asti* → *bhavān/tvam* (respect by 3rd-person honorific) → *kim*
  (→ *what/quis*) → *tava nāma kim?* → *ānandaḥ* ("joy," pleased to meet) →
  practice.
- **Ch. 3 — How Are You**: *katham* → *bhavān katham asti?* → *aham* (→ Latin
  *ego*, English **I**) → *kuśalam* (well; ← *kuśa* grass → "skilled" → "well")
  → *na cintā* ("no worry" = you're welcome) → practice. The copula trio
  *asmi/asi/asti*.
- **Ch. 4 — Farewells**: *gacchāmi* ("I go"; ← *gam* → *come*) → *punaḥ* →
  *punar-darśanāya* ("for seeing again"; the dative; *darśana* = a beholding) →
  *śvaḥ* ("tomorrow," kept distinct from *hyaḥ* "yesterday," unlike Hindi *kal*)
  → practice.
- **Ch. 5 — First Verbs**: *vadāmi* (← *vad*; featuring the **dual** *vadāvaḥ*
  "we two speak") → *ahaṁ saṁskṛtaṁ vadāmi* (*saṁskṛta* = "perfected"; sandhi) →
  *vasāmi* (← *vas* → English **was**; the locative *-e*) → *karomi* (← √kṛ, the
  root of *namaskāra/karma/Sanskrit*; *kāryaṁ karomi* "I work") → practice. Book
  compiles clean with XeLaTeX (0 missing chars, 0 undefined refs).

## Chapter 1 — Greetings (Devanagari taught inline)

- New Sanskrit track on the HL00 framework — a senior Indo-European branch, the
  taproot of the Indo-Aryan tracks and a sister of Latin/Greek/English. Written
  in Devanagari (vendored Noto Sans Devanagari font, shared with Hindi/Marathi).
  One word per lesson, slug ids, atom-first, derivations shown, LaTeX book. No
  reading course: the script is taught inside each word lesson, with the extra
  Sanskrit features (visarga, vocalic ṛ, sandhi) flagged where they occur.
- Chapter 1 (`lessons/SA-C01-*`):
  - **नमस्ते** namaste ("a bow to you") — *namas* (√nam "to bend") + *te*; the
    source of every Indo-Aryan greeting; *te* ↔ Latin *tē* ↔ English *thee*.
  - **नमस्कारः** namaskāraḥ ("the making of a bow") — *namas* + *kāra* (√kṛ "to
    do"); introduces the visarga and the masculine-singular ending.
  - **धन्यवादः** dhanyavādaḥ ("thank you") — *dhanya* + *vāda* (√vad "to speak");
    the full form behind Hindi/Marathi/Punjabi/Bengali thanks.
  - **स्वागतम्** svāgatam ("welcome," lit. "well come") — *su* + *āgata* (√gam);
    the deep-IE payload: *su-* ↔ Greek *eu-*, √gam ↔ English *come*; teaches
    sandhi (*su*+*āgata* → *svāgata*).
  - **आम् / न** ām / na ("yes / no") — *na* ← PIE *ne, cousin of Latin *nōn*
    (previous track), English *no/not/none*, German *nein*.
  - **practice**.
- The recurring thread: Sanskrit as the taproot pointing **both ways** — east into
  the Indo-Aryan tracks, west into Latin/Greek/English — culminating in Sir
  William Jones's 1786 observation of the family's kinship. Devanagari + Sanskrit
  sounds documented in the appendix. Book compiles clean with XeLaTeX;
  syllable-break dots kept in roman font to avoid tofu in the Devanagari span.
