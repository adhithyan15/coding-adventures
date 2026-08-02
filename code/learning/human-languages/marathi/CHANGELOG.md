# Changelog

## Sub-five-minute remediation — 2026-08-02

- Corrected seven declared five-minute estimates whose computed durations were
  already between 126 and 171 seconds.
- Split the genuinely long numbers lesson into a 163-second counting lesson and
  a prerequisite-ordered 240-second etymology lesson.
- Preserved the complete *don / tīn* analogy, *pāch / pāṁch* retention contrast,
  English *four / five* analogy, and Marathi *chār / tsār* sound shift. The
  shared report now measures zero Marathi duration violations.
- Updated the roadmap and session map to expose both Chapter 6 lesson boundaries.
  Chapter 6's missing one-source book publication remains explicit in the shared
  backlog.

## Chapter 6 — Numbers 1–5, and what "conservative" actually means

- **Chapter 6 authored** (`MR-C06-numbers-1-5`): *ek, don, tīn, chār, pāch* —
  with *chār* noted as pronounced nearer ***tsār***; see the last bullet.
- Marathi and Hindi share a script and an ancestor, so most of these are near
  identical — and the lesson is built on the differences, which turn out to be
  **two different kinds of thing**:
  - **दोन *don*** — an **innovation**, not a retention. The obvious guess is that
    Marathi kept something Hindi lost; it didn't. Sanskrit neuter *trī́ṇi* gave
    Prakrit ***tiṇṇi***, where the **ṇ genuinely belongs to the word for
    three** — the *doubling* being a Prakrit trade for the lost *r*, exactly as
    the Hindi chapter spells out, so the lesson is careful **not** to call the
    whole *-ṇṇi* ancient. The word for *two* was then **reshaped by analogy** to
    match its neighbour in the counting sequence, giving ***doṇṇi***. So
    Marathi's *-n* is a **borrowed
    rhyme taken from the word for "three"**, and Hindi's *do* is simply the form
    that never picked it up. (An earlier draft called it "the worn-down remains
    of an old inflectional ending Marathi held onto" — exactly backwards.) The
    lesson ties it to English *four* getting its *f-* from *five*, which the
    Sanskrit anchor chapter has just taught.
  - **पाच *pāch*** — here it *is* a plain retention difference: Hindi keeps
    Sanskrit *pañca*'s nasal in the chandrabindu, Marathi's spelling drops it.
- **So "neither language is simply older" survives, but sharpened**: the two
  cases aren't even the same kind of event — one is an innovation Marathi
  adopted, the other a retention Hindi made.
- Adds a third difference that is **invisible in the spelling**: Marathi's **च**
  before *ā* is nearer **ts** than English "ch", so चार is *tsār*. The earlier
  "only two differ" was true only of the written forms.

## Chapters 2–5 — introductions, how-are-you, farewells, first verbs

Brings Marathi to Chapter 5 parity with the leading tracks (~26 new deep
lessons + four book chapters), mirroring the Indic template. Every atom traced to
its root; the script stays inline. Book recompiles clean with XeLaTeX (0 missing
characters, 0 undefined references), rasterized and visually QA'd.

- **Chapter 2 — Introducing Yourself** (`lessons/MR-C02-*`): nāv (Sanskrit
  *nāman* → English *name*; the Marathi *m→v* softening), mājhaṁ (three-gender
  agreement), **āhe** (the copula, from *ásti*/√as, and it goes **last**),
  "mājhaṁ nāv … āhe", tū/tumhī (courtesy-by-plural), kāy ("what," the *k-*
  family), "tumchaṁ nāv kāy āhe?", ānand ("joy"), practice.
- **Chapter 3 — How Are You** (`lessons/MR-C03-*`): kasā/kaśī/kasaṁ (gendered
  "how"), "tumhī kase āhāt?", mī (*aham* → Latin *ego*, English *I*), "mī barā
  āhe" (Ch1 *baraṁ* now gendered), **kāhī harkat nāhī** (*harkat* ← Arabic — the
  Deccan Perso-Arabic layer), practice.
- **Chapter 4 — Farewells** (`lessons/MR-C04-*`): punhā (Sanskrit *punar*), bheṭū
  (the "we" in the *-ū* ending), "punhā bheṭū", "udyā bheṭū" (Marathi keeps
  *udyā* tomorrow ≠ *kāl* yesterday), kāḷjī ghyā (the retroflex **ळ**; *ghyā*
  resp. / *ghe* fam.), practice.
- **Chapter 5 — The First Verbs** (`lessons/MR-C05-*`): bolṇe (the *-ṇe*
  infinitive; the **gendered present** *bolto/bolte* — Marathi's signature),
  "mī marāṭhī bolto" (*marāṭhī* ← *Mahārāṣṭra* "great realm"), rāhṇe (postposition
  *-āt* "in"), kām karṇe (√*kṛ* — root of *namaskār*, *karma*; *kām* ← *karma*),
  practice.

Concept tags reuse the universal HL01 ids (WORD-NAME, PRONOUN-MY/I/YOU, WORD-IS,
QUESTION-WHAT/HOW, INTRO-*, STATE-HOW-ARE-YOU, WORD-WELL, COURTESY-YOUREWELCOME,
FAREWELL-LATER/-TOMORROW); verbs and lexemes namespaced (MR-VERB-*, MR-WORD-*,
MR-PHRASE-*). The thread throughout: gender on the **verb** in the present, the
**three** genders, and the extra retroflex letter **ळ**.

## Chapter 1 — Greetings (Devanagari taught inline)

- New Marathi track on the HL00 framework — Indo-Aryan, written in Devanagari
  (reuses the vendored Noto Sans Devanagari font). One word per lesson, slug
  ids, atom-first, derivations shown, LaTeX book. No reading course: the script
  is taught *inside* each word lesson.
- Chapter 1 (`lessons/MR-C01-*`):
  - **नमस्कार** namaskār ("hello/goodbye," Sanskrit *namaḥ* + *kāra*) — Marathi's
    default greeting, where Hindi leans on *namaste*; teaches the halant + स्का
    conjunct.
  - **धन्यवाद** dhanyavād ("thanks," Sanskrit) — the न्य conjunct; warmer
    *ābhārī āhe*.
  - **हो** ho ("yes," distinct from Hindi *hāṁ*).
  - **नाही** nāhī ("no / is not") — on PIE *\*ne* (English *no/not/none*).
  - **बरं** baraṃ ("okay/fine," a native Marathi word) — the anusvāra nasal.
  - **येतो / येते** yeto/yete ("I'll be going," lit. "I come [again]") — the
    Dravidian-style "promise of return" farewell, **gendered on the verb**
    (m./f.).
  - **practice**.
- The recurring thread: what makes Marathi its own language despite sharing
  Devanagari with Hindi — *namaskār*, **three** genders, gender on the verb, and
  the extra letter **ळ** (documented in the appendix). Grounded against English
  + Sanskrit. Book compiles clean with XeLaTeX.
