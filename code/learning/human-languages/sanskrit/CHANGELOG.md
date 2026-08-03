# Changelog

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
