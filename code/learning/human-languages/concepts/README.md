# Concepts — the cross-language join layer

This directory holds the **canonical concept taxonomy** for the Human Languages
curriculum. Full rationale is in
[`HL01`](../../../specs/HL01-concept-taxonomy-and-data-layer.md); this is the
operational note.

## What a "concept" is

A **concept** is a language-independent unit of meaning or function the curriculum
teaches — "the greeting you say on meeting someone" is a concept; Spanish *hola*,
German *hallo*, Hindi *नमस्ते*, and Telugu *నమస్కారం* are its **realizations**.
The concept id is the key that lets tooling (the companion app, `HL02`) line the
same idea up across every language — which is what powers "learn it in Spanish,
then the same thing in French, then review both."

## `taxonomy.json`

The authoritative list of **universal** concept ids (the join keys), each with a
family, an English gloss, a `core` flag, and — where it replaced older tags
during normalization — a `retires` list for traceability.

- **Universal** ids (e.g. `GREETING-HELLO`, `TIME-DAY`, `INTRO-MY-NAME-IS`) live
  here and are shared across tracks.
- **Language-local** lexical/structural ids are *namespaced* (`<LANG>-<NAME>`,
  e.g. `ES-VERB-LLAMAR`) and live only in lesson frontmatter, never here.
- **Practice/review** lessons (`type: practice | practice-mix`) carry a session
  label, not a concept, and are exempt from the join.

## How a lesson joins in

Each lesson's `concept_tag` frontmatter field is either a universal id from
`taxonomy.json` or a namespaced id. `type: word` and `type: phrase` lessons are
**content** lessons and must carry a real concept tag; `practice`/`practice-mix`
lessons are exempt. The `HL01` data-layer validator enforces this in CI.

## Normalization (2026-07)

The tags were originally authored per-track and had drifted apart (`GREETING-NIGHT`
vs `GREETING-GOODNIGHT`; `DIA`/`JOUR`/`TAG`/`DAY` all meaning "day"; three spellings
of "thank you"; a few tracks tagging two different words `GREETING-HELLO`). They
were reconciled onto this taxonomy in one mechanical pass over 121 lessons
(frontmatter only — no prose changed). Every `retires` entry in `taxonomy.json`
records an old tag that was folded in. Notable decisions:

- **Synonyms unified**: `GREETING-NIGHT`→`GREETING-GOODNIGHT`, `GREETING-GOODBYE`→
  `FAREWELL`, `THANKS`/`GRATITUDE-THANKS`/`THANKS-FORMAL`→`COURTESY-THANKS`.
- **Scattered lexical tags unified**: the day/night/morning/evening nouns collapse
  into the `TIME-*` family; the "good/well" adjectives into `WORD-GOOD`/`WORD-WELL`;
  the article systems into `GRAMMAR-THE`.
- **Within-track duplicates split**: where a track taught two "hello" words, the
  more formal one (Latin *avē*, Sanskrit *namaskāraḥ*, Punjabi *sat srī akāl*)
  moved to `GREETING-FORMAL`; Punjabi's two "thanks" split into `COURTESY-THANKS`
  (Sanskritic *dhannavād*) and `COURTESY-THANKS-CASUAL` (Perso-Arabic *shukrīā*).
- **Combined yes/no lessons** keep `RESPONSE-YESNO` until the parity pass splits
  them into `RESPONSE-YES` + `RESPONSE-NO`.

The taxonomy grows one concept at a time as the curriculum does — it is never
front-loaded with concepts no lesson yet realizes.
