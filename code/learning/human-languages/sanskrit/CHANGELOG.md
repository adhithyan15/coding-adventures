# Changelog

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
