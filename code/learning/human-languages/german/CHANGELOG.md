# Changelog

## Chapter 4 — Farewells (completes the ES/FR/DE farewell trilogy)

- **Chapter 4 authored** (`GE-C04-auf-wiedersehen`, `-tschuss`, `-bis-bald`,
  `-bis-morgen`, `-practice`): closing a conversation, atom-first, reviewing
  Chapter 3. Reuses the shared `FAREWELL` / `FAREWELL-SOON` / `FAREWELL-TOMORROW`
  concepts and adds `FAREWELL-CASUAL`.
- **auf Wiedersehen** = "on the seeing-again" (*sehen* = English *see*) — the
  exact twin of French *au revoir*, both against Spanish *adiós* "to God".
- **tschüss**, the best etymology in the chapter: *tschüss* ← Low German
  *atschüs* ← Walloon *adjûs* ← French *adieu* — so the breeziest German bye is
  secretly **"to God"**, a far-travelled cousin of *adiós* and *adieu*.
- **The "bis …" family** mirrors Spanish *hasta* / French *à*: *bis bald* (soon —
  *bald* ← Old High German "bold/quick", = English *bold*), *bis später* (later),
  *bis morgen* (tomorrow — *Morgen* = English *morning/morrow*, the same
  morning→tomorrow move as *mañana* / *demain*).
- Taxonomy: `FAREWELL-CASUAL` added (canonical, `core:false`).

## Chapter 3 — "Wie geht's?" (completes the how-are-you trilogy)

- **Chapter 3 authored** (`GE-C03-danke`, `-bitte`, `-gehen`, `-wie-geht-es`,
  `-es-geht`, `-practice`): the "how are you?" exchange, atom-first, reviewing
  Chapter 2. Third of a deliberate cross-language trilogy in this PR (Spanish
  Ch.4 / French Ch.3 / German Ch.3), all sharing the canonical concepts
  `STATE-HOW-ARE-YOU`, `COURTESY-YOUREWELCOME`, `WORD-SOSO`.
- **The etymologies English speakers already own**:
  - *danke* ← *denken* "to think" — and English *thank* IS *think* (both from
    Old English *þancian*/*þencan*), set against *merci* (reward) and *gracias*
    (grace).
  - *bitte* ← *bitten* "to ask/pray" — cognate of English *bid* and *bead* (a
    bead was a prayer); the one word doing please / you're-welcome / here-you-go
    / pardon.
  - *gehen* IS English *go* (straight Germanic cognate); *es geht mir gut* = "it
    goes well *to me*" — gently introduces the **dative** (*mir/dir/Ihnen*).
  - *es geht* ("it goes," nothing added) as the understated shrug for "so-so."
- **The trilogy's payoff**, stated in-lesson: German and French say wellbeing as
  motion ("how does it **go**?"), Spanish as posture ("how are you
  **standing**?" — *estar*).
- Taxonomy: namespaced `DE-VERB-GEHEN` documented.

## Chapter 2 — Introducing Yourself

- New chapter built around the introduction dialogue (*Ich heiße Susanne. / Wie
  heißen Sie? / Ich heiße David. / Freut mich.*), atom-first, one word per
  lesson (`lessons/GE-C02-*`, `book/chapters/ch02-introductions.tex`):
  - **ich** ("I" ← *\*ik* / PIE *\*eǵ*; cousin of Latin *ego*, English *I*).
  - **heißen** ("to be called" ← *\*haitaną*; English archaic *hight*, *behest*)
    — German names with a plain verb, no reflexive "myself."
  - **ich heiße…** — **"my name is…"** ("I am called"), with literal *mein Name
    ist* (*Name* ← *\*namô*, English *name* / Latin *nōmen*) as the alternative.
  - **du / Sie** (familiar / formal "you") — *Sie* is the capitalized 3rd-person
    plural "they" used as polite "you"; the third route to politeness beside
    Spanish *usted* and French *vous*.
  - **wie** ("how" ← *\*hwī* / PIE *\*kʷo-*; English *how/what/who*).
  - **wie heißen Sie?** — **"what's your name?"** ("how are you called?");
    verb-second word order; informal *wie heißt du?*.
  - **freut mich** ("pleased to meet you" = "it gladdens me"; ← *froh*, "glad").
    Its object pronoun **mich** ("me") is traced too — ← *\*mek* / PIE *\*me-*,
    cousin of English *me/my/mine* and French *me* (every atom rooted, not
    glossed).
  - **practice** — the whole dialogue.
- Book compiles clean with XeLaTeX.

## Beginner-audience + parity pass

Brought the German book fully to the Hindi/Spanish standard. Two things:

**Stop assuming prior Spanish/French (HL00 Audience rule).** The books are for a
true beginner whose only shared language is English; German leaned on the other
tracks as knowledge already owned.
- Preface: dropped "exactly as the Spanish book used the *-ct-→-ch-* rules" and
  "Because the reader also knows Spanish (and is meeting French)"; states the
  true-beginner framing and that every Spanish/French form is supplied in full.
- `ch01-greetings.tex`: "German's version of the Spanish *-ct-→-ch-* rule" →
  self-contained sound-law framing; "the same job *bueno/buena* and *bon/bonne*
  did" → "the same job Romance adjectives do."
- Practice lessons `GE-C01-gut` ("the rules you met in Spanish") and
  `GE-C01-der-die-das` ("You've met gender in Spanish and French") de-assumed.

**Filled the parity gaps the audit flagged.**
- Added per-word **`sounds` boxes** (the book previously gave pronunciation only
  inline): *hallo*, *gut*, *der/die/das*, *Tag*, *Morgen*, *Abend*, *Nacht* ---
  including German final-devoicing (*Tag* → *tahk*, *Abend* → *AH-bent*) and the
  *ach*-laut in *Nacht*.
- Added noun **plurals**: *die Tage*, *die Morgen*, *die Abende*, *die Nächte*.
- Book still compiles clean with XeLaTeX (14 pages).

## Chapter 1 — Greetings (track bootstrapped)

- New German track on the HL00 framework: one word per lesson, slug ids,
  gender-before-nouns, atom-first, derivations shown, LaTeX book (CI
  auto-discovers `german/book/`).
- Chapter 1 (`lessons/GE-C01-*`), atom-first, with German's Germanic-roots
  flavor:
  - **hallo** (a *real* cousin of English "hello," unlike Spanish *hola*)
  - **gut** ("good" *and* "well" ← Germanic *\*gōdaz* = English *good*;
    introduces the **High German Consonant Shift** d→t as a recurring decoder)
  - **der / die / das** ("the"; **three** genders — German kept the neuter;
    ← Germanic *\*sa/\*sō/\*þat*, cousins of English *the/that*)
  - **Tag** (← *\*dagaz* = English *day*; ≠ Latin *dies* behind *día/jour*)
  - **Guten Tag** (assembled; the *-en* accusative ending)
  - **Morgen** (← *\*murganaz* = *morning/tomorrow*) · **Guten Morgen**
  - **Abend** (← *\*ābanþs* = English *eve*; contrast with Romance "late"
    words) · **Guten Abend**
  - **Nacht** (← PIE *\*nókʷts* — the four-way *Nacht/night/noche/nuit*
    reunion; feminine) · **Gute Nacht** (feminine agreement, *-e* not *-en*)
  - **practice**
- Grounds each word against English (direct Germanic cousin), with Spanish and
  French alongside for contrast. Book compiles clean with XeLaTeX (13 pages).
