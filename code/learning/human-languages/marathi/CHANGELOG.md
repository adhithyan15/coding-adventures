# Changelog

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
