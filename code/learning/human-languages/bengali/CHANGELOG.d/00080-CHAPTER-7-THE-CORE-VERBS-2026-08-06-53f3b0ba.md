## Chapter 7 — The Core Verbs — 2026-08-06

- Authored six schema-v2 lessons realizing the canonical `SPINE-SAY-WHAT-I-DO`
  concepts `VERB-BE`, `VERB-GO`, `VERB-COME`, `VERB-EAT`, `VERB-SEE` and
  `VERB-KNOW`. Before this the track realized **no** canonical verb concept: its
  only four verbs (*bôlā*, *thākā*, *kôrā*, *dækhā hôbe*) were all namespaced
  `BN-VERB-*` and none of them was on the shared spine.
- One idea per lesson, each one a thing Bengali does that its neighbours do not:
  - **হওয়া** *hôwā* — Bengali has **two** be-verbs, and আছ- is unfinished: it
    has a present and a past and nothing else, so the future falls to *hôbe* or
    to Chapter 5's থাকা. Root: Sanskrit √bhū, PIE *\*bʰuH-* → English **be**,
    **been**, **future**, **physics**.
  - **যাওয়া** *jāwā* — the honorific level lives in the **verb ending**:
    *jāsh* / *jāo* / *jān* for তুই / তুমি / আপনি, and *se jāy* against *tini
    jān* in the third person. Drop the pronoun and the register still stands.
  - **আসা** *āsā* — **no grammatical gender, anywhere**, set against Hindi
    *ātā/ātī*, Marathi *yeto/yete* and Gujarati's *āvyo/āvī* past. Not a
    beginner's simplification the grammar takes back later.
  - **খাওয়া** *khāwā* — Bengali **eats its drinks**: *jôl khāwā*, *chā khāwā*,
    where Hindi keeps *pīnā*. The formal পান করা carries √pā → English
    **potion**, **potable**.
  - **দেখা** *dækhā* — **vowel harmony**: *dekhi* closes where *dækhe* and
    *dækho* stay open, and the spelling দে never moves. Root: Sanskrit √dṛś, PIE
    *\*derḱ-* → Greek *drákōn* → English **dragon**.
  - **জানা** *jānā* — জানা for facts against চেনা for people, the *savoir* /
    *connaître* line English lost. Root: √jñā, PIE *\*ǵneh₃-* → **know**,
    **notice**, **diagnosis**.
- Flagged two dead ends honestly rather than inventing cousins: যাওয়া's PIE
  *\*yeh₂-* has no living English descendant, and খাওয়া's *khād-* has no secure
  Indo-European pedigree outside Indo-Aryan.
- All six derive as **`voice`** — the chapter's `drivablePrefix` is 6, every
  table is two columns, and no lesson leans on a sight cue. Computed durations
  257–281 s, all inside the 300 s ceiling.
- Wired the chapter into `curriculum.json` (`BN-PATH-011`,
  `BN-EXT-011-CORE-VERBS`, and six concepts struck from the
  `SPINE-SAY-WHAT-I-DO` omission ledger), `chapters.json` (payoff
  `BN-C07-jana`, 8/12 introduced atoms = 0.67, above the 0.5 floor),
  `core/book-generation.json`, and `book/book.tex`.
- Gave the book preamble an optional `grammarlens` title — the generator passes
  each lesson's own "Grammar Lens: …" heading through, which the old
  no-argument box could not accept — plus composed glyphs for the PIE palatals
  `ǵ` and `ḱ`. The seven-chapter build is still warning-free, with no missing
  characters and no over/underfull boxes.

