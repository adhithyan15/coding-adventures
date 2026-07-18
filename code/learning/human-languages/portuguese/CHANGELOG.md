# Changelog

## Chapter 2 — "Tudo bem?" (the how-are-you chapter)

- **Chapter 2 authored** (`PT-C02-de-nada`, `-como`, `-tudo`, `-tudo-bem`,
  `-mais-ou-menos`, `-practice`): the "how are you?" exchange, atom-first,
  reviewing Chapter 1. Fifth and final track in the PR's cross-language
  how-are-you set, reusing `STATE-HOW-ARE-YOU`, `COURTESY-YOUREWELCOME`,
  `WORD-SOSO`. Reordered ahead of introductions to widen the set (register
  você / o senhor introduced inline).
- **Portuguese's distinctive move — the verb-free greeting**: *Tudo bem?*
  ("everything well?") uses **no pronoun and no verb**, dodging the
  você/tu/senhor tangle; built on **tudo** ← Latin *tōtus* "whole" (→ total/
  teetotal) + **bem** ← *bene*. It also does *Como vai?* (*ir*, "to go" — with
  French/German) **and** *Como está?* (*estar*, "to stand" — with Spanish/
  Italian), so Portuguese spans all three patterns.
- **Etymology hooks**: *de nada* ← *nāta* "born thing" (exact twin of Spanish);
  *como* ← *quōmodo*; *mais ou menos* ← *magis* + *minus* (twin of Spanish *más
  o menos*); and *você* ← *vossa mercê* "your mercy" — the same "your grace"
  origin as Spanish *usted*.
- Taxonomy: namespaced `PT-WORD-TUDO` documented.

## Chapter 1 — Greetings (track bootstrapped)

- New Portuguese track on the HL00 framework: one word per lesson, slug ids,
  gender-before-nouns, atom-first, derivations shown, LaTeX book (Latin Modern;
  CI auto-discovers `portuguese/book/`).
- Chapter 1 (`lessons/PT-C01-*`), atom-first:
  - **olá** ("hi"; a rootless interjection, twin of Spanish *hola*).
  - **bom / boa** ("good" ← *bonus*; nasal *bõ*; adjective agreement).
  - **o / a** ("the"; gender ← *ille/illa*, eroded to a single vowel — further
    than Italian *il* / Spanish *el*).
  - **dia** ("day" ← *dies*; the gender trap — masculine despite *-a*).
  - **bom dia** (assembled; singular vs. Spanish plural *buenos días*).
  - **tarde / boa tarde** ("afternoon" ← *tardus*, "late"; English *tardy*).
  - **noite / boa noite** ("night" ← *noctem*; Latin *-ct-* → *-it-*, like
    French *nuit*).
  - **obrigado / obrigada** ("thanks" ← *obligātus*, "obliged"; English
    *obligated*) — agrees with the **speaker**, not the noun.
  - **practice**.
- Grounds each word against English + Latin, with Spanish/French/Italian
  supplied for contrast (beginner-audience). Book compiles clean with XeLaTeX.
