# Changelog

## Chapter 4 — Farewells (parallel of Spanish Ch. 5)

- **Chapter 4 authored** (`FR-C04-au-revoir`, `-a-plus-tard`, `-a-bientot`,
  `-a-demain`, `-practice`): closing a conversation, atom-first, reviewing
  Chapter 3. Reuses the canonical `FAREWELL` + `FAREWELL-LATER/TOMORROW/SOON`
  concepts introduced with Spanish Ch. 5, mapping each French goodbye to its
  Spanish twin.
- **The "see you again" metaphor**: *au revoir* = "until the re-seeing" (*voir* ←
  *vidēre* → vision/video/revise) — explicitly paired with German *auf
  Wiedersehen* ("on the seeing-again"), against Spanish *adiós* ("to God").
- **Cross-language root callbacks**: *à plus tard* — *tard* ← Latin *tarde*, the
  same word as Spanish *tarde*; *à demain* — *demain* ← *dē māne* "from the
  morning", sharing *māne* with Spanish *mañana* (and English *matinée*).
- **A writing-nuance aside**: the circumflex on *bientôt* (← *tost*) as the ghost
  of a dropped *s* (*hôtel* ← *hostel*), tying back to the accent-mark thread.
- All soft goodbyes are **à** + a time, mirroring Spanish's **hasta**.

## Chapter 3 — "Comment ça va ?" (the parallel of Spanish Ch. 4)

- **Chapter 3 authored** (`FR-C03-merci`, `-de-rien`, `-aller`,
  `-comment-ca-va`, `-comme-ci-comme-ca`, `-practice`): the "how are you?"
  exchange, atom-first, reviewing Chapter 2 throughout. Built deliberately as the
  cross-language mirror of the Spanish Chapter 4 shipped in the same PR — same
  canonical concepts (`STATE-HOW-ARE-YOU`, `COURTESY-YOUREWELCOME`, `WORD-SOSO`),
  so the interleaving method has real parallel material.
- **Etymology contrasts made explicit** (the point of the curriculum):
  - *merci* ← *mercēs* "reward / wages" (→ mercy/merchant/commerce) — set against
    Spanish *gracias* ← *grātia* "grace" and Portuguese *obrigado* ← "obliged".
  - *de rien* ← *rem* "a thing" → "nothing" — the exact twin of Spanish *de nada*
    ← *nāta* "a born thing" (a callback the Spanish lesson already forward-references).
  - *aller* "to go" as the state-verb ("how does it *go*?") — contrasted with
    Spanish *estar* "to stand"; its suppletive paradigm traced to *ambulāre*
    (amble/ambulance), *vādere* (invade/evade), *īre* (exit/transit).
  - *comme ci, comme ça* — *comme* shares *quōmodo* with *comment*; the shrug set
    against Spanish *más o menos* and Italian *così così*.
- Taxonomy: namespaced `FR-VERB-ALLER` documented in the examples list.

## Chapter 2 — Introducing Yourself

- New chapter built around the introduction dialogue (*Je m'appelle Susanne. /
  Comment vous appelez-vous? / Je m'appelle David. / Enchanté.*), atom-first,
  one word per lesson (`lessons/FR-C02-*`, `book/chapters/ch02-introductions.tex`):
  - **je** ("I" ← *ego*; English *ego*)
  - **me** ("myself" ← Latin *mē*; English *me*, *my*, *mine*) — its own lesson,
    with the reflexive set *me / te / se* traced. (Every atom of *je m'appelle*
    is taught and rooted, not just glossed.)
  - **(s')appeler** ("to call [oneself]" ← *appellāre*; *appeal*, *appellation*)
    — introduces **reflexive verbs**.
  - **je m'appelle…** — assembled: **"my name is…"** ("I call myself"), with the
    literal *mon nom est* (← *nōmen*, English *noun*) as the stiffer alternative.
  - **tu / vous** (familiar / formal "you" ← *tū / vōs*) — politeness by using
    the plural on one person; contrasted with Spanish *usted*.
  - **comment** ("how" ← *quo modo*; same source as Spanish *cómo*).
  - **comment vous appelez-vous?** — **"what's your name?"** by inversion; the
    informal *comment tu t'appelles?*.
  - **enchanté(e)** ("pleased to meet you" ← *in-cantāre*; *enchant*,
    *incantation*, *chant*) — gender agreement with the speaker.
  - **practice** — the whole dialogue.
- Also fixed two leftover beginner-audience slips the earlier pass missed
  (`roadmap.md` "the learner's in-progress language"; `session-map.md` "the
  Spanish twin"). Book compiles clean with XeLaTeX.

## Beginner-audience pass — Spanish no longer assumed as prior knowledge

Corrected a systemic violation of HL00's Audience rule: the book and practice
lessons addressed a reader who was "also learning Spanish" and leaned on
Spanish as knowledge already owned. The books are for a true beginner whose
only shared language is English; Spanish comparisons are enrichment the text
must supply in full, not a baseline it may assume.

- Preface rewritten: drops "Because the reader is also learning Spanish…" and
  "exactly as in the Spanish book"; states the true-beginner framing and that
  every Spanish comparison is supplied by the text (a reader who knows Spanish
  "simply nods along").
- Chapter 1 (`book/chapters/ch01-greetings.tex`) and the matching practice
  lessons: recast every "Spanish twin," "the *bueno/buena* machine from
  Spanish," "One mercy over Spanish," and "you know this from Spanish" into
  self-contained "Spanish, another daughter of Latin, does X" enrichment.
  Section title "*bien* — and a Spanish twin" → "*bien* — 'well'."
- Filled the two missing noun plurals the standard wants: *les soirs*,
  *les nuits* (a new Grammar Lens on *soir*, extended on *nuit*).
- Book still compiles clean with XeLaTeX (13 pages).

## Chapter 1 — Greetings (track bootstrapped)

- New French track, built on the same HL00 framework as Spanish: one word per
  lesson, slug ids, gender-before-nouns, atom-first assembly, derivations
  shown (not just roots named), LaTeX book.
- Chapter 1 (`lessons/FR-C01-*`), atom-first:
  - **salut** (informal hi ← Latin *salus* "health") · **bien** ("well" ←
    *bene*; the Spanish twin) · **bon / bonne** ("good" ← *bonus*; agreement)
  - **le / la / les** ("the"; grammatical gender ← Latin *ille/illa/illos*,
    same as Spanish *el/la*, also the source of *il/elle*)
  - **jour** ("day" ← *diurnum* ← *dies*; the detour that gives English
    *journal*/*journey* and explains why French *jour* ≠ Spanish *día*)
  - **bonjour** (assembled; *singular*, contrasted with plural *buenos días*)
  - **soir** ("evening" ← *sērus* "late"; parallels Spanish *tarde* ←
    *tardus*) · **bonsoir**
  - **nuit** ("night" ← *noctem*; the *-ct-→-ch-* (Spanish) vs *-ct-→-it-*
    (French) sound-change table) · **bonne nuit** (feminine agreement)
  - **practice**
- Grounds each word against English **and Spanish** (the learner's in-progress
  language), foregrounding the Romance twins' differences.
- Book compiles clean with XeLaTeX (13 pages); the CI workflow auto-discovers
  `french/book/` and builds it as a PDF artifact.
