# Hindi Roadmap

Same shape as the other tracks: deep one-word lessons in themed chapters.
Slug-identified; order lives in the book and
[`session-map.md`](./session-map.md). See
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md).

Grounding: English + Arabic, and above all Hindi's **double inheritance** —
Sanskrit *tatsama* words beside a Perso-Arabic layer — traced word by word.
Devanagari is taught **inline**, inside the word lessons, never as a gated
reading course.

## Authored

- **Ch. 1 — Greetings**: namaste → namaskār → dhanyavād → shukriyā → alvidā →
  practice. Devanagari introduced through the words (inherent *a*, mātrā vowel
  signs, halant + conjuncts, independent vowels), and the Sanskrit /
  Perso-Arabic split introduced through the greetings themselves.
- **Ch. 2 — Introducing Yourself**: nām → merā → hai → **merā nām … hai** ("my
  name is") → āp/tum → kyā → **āpkā nām kyā hai?** → khushī (pleased to meet) →
  practice. Every atom traced (nām ← *nāman* → *name*; hai ← *asti* → *is*;
  merā ← *ma-* → *my*; kyā ← *ka-* → *what*); SOV word order; the three-level
  "you."

- **Ch. 3 — How Are You**: kaise (how) → āp kaise haiṁ (how are you) → maiṁ (I)
  → hūṁ (am) → ṭhīk (fine) → āpkā svāgat hai (you're welcome) → practice. The
  copula trio *hūṁ/hai/haiṁ* (← *asmi/asti*, English *am/is*); respect-as-plural.
- **Ch. 4 — Farewells**: phir (again) → milenge (we'll meet) → phir milenge → kal
  milte haiṁ (see you tomorrow; *kal* = tomorrow *and* yesterday ← *kāla*) →
  chaltā/chaltī hūṁ (I'll be off, gendered) → practice. Deepens Ch.1's *alvidā*.
- **Ch. 5 — First Verbs**: bolnā (to speak) → maiṁ hindī boltā hūṁ (I speak
  Hindi; *hindī* ← *sindhu*) → rahnā (to live; postposition *meṁ*) → karnā (to
  do; ← √kṛ, the root of *karma/namaskār/Sanskrit*) → practice. The present
  habitual (stem + *-tā/-tī/-te* + *honā*), verb-last, gender agreement.
- **Writing track W01–W05 — hand-writing Devanagari.** A parallel spine, the
  Hindi counterpart of Arabic AR-W01–12 and Russian RU-W01–05, ending on the
  first word of the course. **W01** the **shirorekhā** (शिरोरेखा = "head-line";
  *śiras* ← PIE \**ḱerh₂-*, the root of Latin *cornu*/*cerebrum* and English
  **horn**) — drawn **last**, and as **one** bar across a whole word, so Hindi
  hangs *from* a line where Latin sits *on* one (flagged as the **common
  convention, not a rule**); plus न and म and the commonest letter frame
  (**spine right, shape left, bar on top**), with the spineless minority named
  rather than implied away. **W02** the **inherent vowel** — क is *ka*, not *k* —
  so the script is an **abugida**, a coinage from Ge'ez that names four
  **consonants** of the old Semitic order *and* four **vowel series** at once
  (*ʾä-bu-gi-da*), so the term performs its own definition; plus क, त, and the
  five **stop families** ordered by **place of articulation** (soft palate →
  lips), a phonetics chart memorised as an alphabet. **W03**
  **mātrās** (मात्रा = "a **measure**", ← *mā-* "to measure", PIE \**meh₁-*,
  whence *meter*/*measure*/*month*), which **replace** the inherent vowel rather
  than adding to it — ा and े, building **नाम**, with **ि** as the punchline: the
  one Hindi mātrā written **before** the consonant and pronounced **after** it (an
  inherited Brahmi quirk, not something to rationalise). **W04** र — the **first
  spineless letter** the learner writes, with **द** named so it doesn't read as
  unique — and स, plus the **probable** long-lost cousinhood of **स · Σ · S**:
  certainly *šin* → Σ → S going west, while Brahmi's Semitic descent via Aramaic
  is the **leading view, not settled**. Builds **मेरा नाम**, where two words means
  **two bars** — Hindi uses an ordinary **space**, and the bar breaks *because* of
  it. **W05** the **virama** ् / **halant** (विराम = "a **stopping**", the word
  that grades Hindi punctuation: *pūrṇ virām* = full stop, *alp virām* = comma),
  which kills the inherent vowel and in practice **fuses** the consonant into a
  **conjunct** (स् + त → स्त, a **spine-bearing** first consonant surrendering
  its spine — with र's repha/ra-kāra and क्ष/त्र/ज्ञ named as the exceptions) —
  assembling **नमस्ते**, and
  revealing that it is *namas* ("a **bow**", ← *nam-* "to bend") + *te* ("to
  you"): the greeting *is* the bow. **Authored.**
  - Data source: `data/scripts/devanagari.json` (28 letters, 12 marks), which is
    marked `complete: false` and covers the greeting/self-introduction vocabulary.
    **Every letter the learner is asked to WRITE has a real entry** with real
    `components`/`strokeOrder`; nothing was invented. Six letters appear in the
    prose without entries (**ख ज ञ ट ण ष**), none of them as something to draw;
    where one is cited *as a letter* (**ट** in W02's articulation chart, **ख** in
    the word *shirorekhā*) the lesson says so outright — "read it now, draw it
    when its entry is written."
  - **Known data defects found while writing this, not fixed here.**
    `devanagari.json`'s **ध** entry ("like द with an extra inner loop") omits the
    vertical spine that ध actually has, and its **ह** entry asserts a right spine
    although ह is traditionally counted in the *no-pāī* class. Both are
    load-bearing for conjunct behaviour. The lessons therefore cite only **द** and
    **र** as spineless — the two the data and the tradition agree on — and a
    separate data pass should settle ध and ह against a typography source rather
    than a guess.
  - **The same file serves Marathi and Sanskrit**, which share the script. Their
    parallel W-tracks are the obvious follow-on and can mirror this arc directly —
    नमस्ते is identical in all three.

## Planned

| Chapter | Theme |
|---|---|
| 6 | Postpositions (*ko, se, meṁ, par*), the ergative *ne*; more -nā verbs |
| 7+ | Numbers, family, food, negation — always with the two-vocabularies thread |

Note: Hindi splits "you" by **register** (*āp* formal / *tum* familiar / *tū*
intimate) — like Spanish/French/German, and unlike Arabic's gender split.
Worth teaching beside those.
