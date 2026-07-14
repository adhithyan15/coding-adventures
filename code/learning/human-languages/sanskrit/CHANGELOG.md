# Changelog

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
