# HL01 — Cross-Language Concept Taxonomy & Structured Data Layer

## Overview

`HL00` defines the **content** of the Human Languages curriculum: per-language
Markdown lessons and a LaTeX book, each lesson excavating one word or phrase with
its etymology "cousin web." This spec, `HL01`, defines the **data layer** that
sits on top of that content — the machine-readable bridge that lets software (the
companion app in `HL02`, and anything later) reason about the curriculum
*across* languages.

The single idea here is the **concept**: a language-independent unit of meaning
or function that the curriculum teaches. "The greeting you say when you meet
someone" is a concept; Spanish *hola*, German *hallo*, Hindi *नमस्ते*, and Telugu
*నమస్కారం* are its **realizations** in four languages. Concepts are the join key
that makes cross-language study possible.

```
   lessons/*.md (frontmatter)              concepts/taxonomy.json
        │  concept_tag: GREETING-HELLO          │  canonical universal concepts
        │  headword: hola                        │  (the controlled vocabulary)
        ▼                                        ▼
   ┌────────────────────────────────────────────────────┐
   │  data layer: concept  ──►  { language: realization } │
   │  (derived by parsing frontmatter, validated in CI)   │
   └────────────────────────────────────────────────────┘
        │                                        │
        ▼                                        ▼
   languagesForConcept("GREETING-HELLO")   conceptsByLanguage("spanish")
        = [spanish, german, hindi, …]           = [GREETING-HELLO, WORD-DAY, …]
```

## Why this spec exists — the discovery that forced it

The lesson schema in `HL00` already carries a `concept_tag` field. In principle
that is the join key. In practice, an audit of the existing 16 tracks found the
tags are **not a shared vocabulary yet**:

- Spanish tags its good-night greeting `GREETING-NIGHT`; French and German tag
  the same function `GREETING-GOODNIGHT`.
- Spanish `DIA`, French `JOUR`, German has no "day" lexical tag — the same
  concept (the noun "day") wears three unrelated, language-local names.
- Telugu and Hindi both tag `WHATS-YOUR-NAME`; Spanish's equivalent lesson uses
  `ES-C03-como-se-llama` with tag `CH...`.

So a query like *"show me 'thank you' in Spanish, French, and German"* cannot be
answered reliably today. Before any cross-language tooling can exist, the tags
must be **reconciled onto a canonical taxonomy**. That reconciliation, plus the
schema and validator that keep it honest, is what `HL01` delivers.

## Two tiers of concept

Not every lesson teaches something that crosses languages cleanly, and forcing it
to would be dishonest. So concepts come in two tiers.

### 1. Universal concepts (the interleaving join keys)

A **universal concept** is a meaning or function that most languages realize with
*some* word — the thing a learner would ask "how do you say X in this language?"
about. These are the concepts the companion app interleaves across languages.

Their ids are **canonical, language-independent, and listed in `taxonomy.json`**.
Examples: `GREETING-HELLO`, `COURTESY-THANKS`, `RESPONSE-YES`, `RESPONSE-NO`,
`QUESTION-WHAT`, `INTRO-MY-NAME-IS`, `NUMBER-3`.

Id format: `FAMILY-NAME`, SCREAMING-KEBAB-CASE, family prefix from the fixed
family list below. Ids are **stable slugs, never renumbered** (same rule as
`HL00` lesson ids), so inserting a concept never cascades.

### 2. Lexical / structural concepts (language-local)

Some lessons excavate a **building block specific to one language** — a particular
root word (Spanish *día* ← Latin *dies*), or a structural feature (the Spanish
*el/la* article system, German's three-way *der/die/das*). These do not have a
clean one-to-one sibling in every other language, so forcing them into the
universal vocabulary would create false equivalences.

They keep a **namespaced tag**: `<LANG>-<NAME>`, e.g. `ES-WORD-DIA`,
`DE-ARTICLES-DER-DIE-DAS`. They are still first-class data — the app can drill
them *within* a language — but the cross-language interleaver does not expect
them to have realizations elsewhere.

A concept that starts life language-local can be *promoted* to universal later
(e.g. once "day/noche/día" is authored in enough tracks, a `WORD-DAY` universal
concept with per-language realizations becomes worthwhile). Promotion is a
taxonomy edit plus a retag; nothing renumbers.

## Concept Families (fixed prefixes)

| Family | Meaning | Example ids |
|---|---|---|
| `GREETING-` | greeting/parting formulas | `GREETING-HELLO`, `GREETING-MORNING`, `GREETING-AFTERNOON`, `GREETING-EVENING`, `GREETING-GOODNIGHT`, `FAREWELL` |
| `COURTESY-` | politeness formulas | `COURTESY-THANKS`, `COURTESY-THANKS-FORMAL`, `COURTESY-PLEASE`, `COURTESY-SORRY`, `COURTESY-WELCOME` |
| `RESPONSE-` | minimal answers | `RESPONSE-YES`, `RESPONSE-NO`, `RESPONSE-OKAY` |
| `QUESTION-` | interrogative words | `QUESTION-WHAT`, `QUESTION-WHERE`, `QUESTION-WHEN`, `QUESTION-WHO`, `QUESTION-WHY`, `QUESTION-HOW` |
| `INTRO-` | self-introduction moves | `INTRO-MY-NAME-IS`, `INTRO-WHATS-YOUR-NAME`, `INTRO-NICE-TO-MEET-YOU`, `INTRO-HOW-ARE-YOU`, `INTRO-IM-WELL` |
| `PRONOUN-` | personal pronouns | `PRONOUN-I`, `PRONOUN-YOU-INFORMAL`, `PRONOUN-YOU-FORMAL` |
| `NUMBER-` | cardinal numbers | `NUMBER-0` … `NUMBER-10` |
| `TIME-` | time-of-day / calendar nouns | `TIME-DAY`, `TIME-NIGHT`, `TIME-MORNING`, `TIME-EVENING` |
| `WORD-` | promoted cross-language lexical items | `WORD-GOOD`, `WORD-WELL` |
| *(namespaced)* | language-local lexical/structural | `ES-WORD-DIA`, `DE-ARTICLES-DER-DIE-DAS` |

The family list is closed — adding a family is a spec edit — but ids within a
family are open. `taxonomy.json` is the authoritative enumeration of every
universal id; namespaced ids live only in lesson frontmatter.

## `concepts/taxonomy.json`

Lives at the curriculum root (`code/learning/human-languages/concepts/taxonomy.json`).
One entry per universal concept:

```json
{
  "GREETING-HELLO": {
    "family": "GREETING",
    "gloss": "hello (neutral, said on meeting)",
    "core": true,
    "notes": "The default meeting greeting. A language may realize it with a
              formal/informal split (see PRONOUN-YOU-*); pick the neutral form."
  },
  "GREETING-GOODNIGHT": {
    "family": "GREETING",
    "gloss": "good night (said on parting at night / before sleep)",
    "core": true,
    "notes": "Distinct from GREETING-EVENING, which is a meeting greeting. This
              retires Spanish's older GREETING-NIGHT tag."
  }
}
```

- **`core: true`** marks concepts expected in *every* track (the greetings,
  yes/no, thanks, the numbers). The validator reports missing core realizations
  as a coverage gap (a warning, since some tracks are still being authored — it
  hard-fails only once a track is declared "parity-complete" in its README).
- **`gloss`** is the English handle the app shows.
- **`notes`** records reconciliation decisions (which old tags this id retires),
  so the history of the normalization is legible.

## Per-language realization schema

For each (concept, language) the data layer derives a **realization**. Most fields
come straight from lesson frontmatter; a few new optional frontmatter fields
(below) supply what the body prose currently holds implicitly.

```ts
interface Realization {
  concept: string;        // canonical or namespaced concept id
  language: string;       // track slug: "spanish", "telugu", …
  lessonId: string;       // ES-C01-hola
  chapter: number;
  headword: string;       // "hola" / "నమస్కారం"
  gloss: string;          // "hello"
  romanization: string;   // "OH-lah" / "namaskāram" — = headword for Latin script
  script: string;         // "latin" | "devanagari" | "telugu" | … (font key)
  gender: "masc" | "fem" | "neut" | null;
  sounds: string[];       // ids into pronunciation-reference.md
  roots: string[];        // etymological roots (for indexing)
  etymologyHook: string;  // ≤120-char memory anchor; falls back to gloss
}
```

### New optional frontmatter fields (added to the `HL00` schema)

To keep the data layer **derived from the lessons** (never a hand-maintained
parallel copy that drifts), two optional fields are added to lesson frontmatter:

```yaml
romanization: OH-lah        # required for non-Latin scripts; omit for Latin (defaults to headword)
etymology_hook: "gracia ← Latin gratia → grace, gratitude, gratis"   # optional ≤120 chars
```

Both are optional and back-compatible: a lesson without them still parses, and
the validator emits a *warning* (not an error) for a non-Latin lesson missing
`romanization`, or any lesson missing `etymology_hook`. Authoring them is folded
into the normalization pass and the ongoing parity work, not a blocking upfront
migration.

`script` is inferred from the track (each track declares its script once in its
`README.md` frontmatter / a `track.json`); `glyphs` used by a headword are
resolved against the script data below, not stored per lesson.

## Script & character-breakdown data — `data/scripts/<script>.json`

This is **new authored data**, separate from concepts, that powers the app's
"learn to write, piece by piece" study view (`HL02`). One file per script, sourced
from each track's `pronunciation-reference.md` plus standard handwriting convention.

**The goal is to teach *any* writing system, so the schema is general** — one shape
describes an alphabet, an abugida, or an abjad, and adding a new script (Gujarati,
Bengali, **Hebrew**, Greek, …) is a **data drop, never a code change**. The three
families and how the schema handles each:

| Family | Examples | Vowels | Direction | Schema features used |
|---|---|---|---|---|
| **alphabet** | Latin, **Cyrillic**, Greek | letters spell them | ltr | `letters` only |
| **abugida** | Devanagari, Bengali, Gujarati, Telugu, Kannada, Malayalam, Tamil | inherent vowel + `marks` | ltr | `letters` (+`inherentVowel`), `marks`, `combination` (conjuncts) |
| **abjad** | Arabic, **Hebrew** | optional diacritic `marks` | **rtl** | `letters` (+`forms`), `marks` (harakat/niqqud) |
| **logographic** | **Chinese** (Hanzi) | none — tone-marked pinyin in `sound` | ltr | `letters` (`role: logograph`/`radical`, `tone`, `components`=radicals/strokes) |

```jsonc
{
  "script": "devanagari",            // id, matches the filename (an OPEN string)
  "name": "Devanagari",
  "font": "_fonts/NotoSansDevanagari-Static.ttf",
  "direction": "ltr",                // "ltr" | "rtl"
  "system": "abugida",               // "alphabet" | "abugida" | "abjad" | …
  "complete": false,                 // true → validator ENFORCES glyph coverage
  "combination": "consonant + inherent 'a'; a mātrā changes it; virama stacks conjuncts",
  "letters": [
    { "glyph": "क", "sound": "ka", "role": "consonant", "inherentVowel": "a",
      "components": ["left loop", "right spine", "top bar (shirorekhā)"],
      "strokeOrder": ["left loop", "spine", "top bar"],
      "strokeOrderNote": "conventional" }
  ],
  "marks": [
    { "mark": "ी", "sound": "ī", "role": "vowel-sign", "attachesAs": "vertical to the right",
      "example": { "base": "न", "combined": "नी", "sound": "nī" } }
  ]
}
```

For a **cursive/abjad** script a `letter` adds contextual `forms`
(`isolated`/`initial`/`medial`/`final`) and the file sets `"direction": "rtl"`:

```jsonc
{ "glyph": "ب", "sound": "b", "role": "consonant",
  "forms": { "isolated": "ب", "initial": "بـ", "medial": "ـبـ", "final": "ـب" },
  "components": ["the boat bowl", "one dot below"], "strokeOrder": ["bowl", "dot"],
  "strokeOrderNote": "conventional" }
```

Design commitments:

- **General, not abugida-specific.** `script` is an open string
  (`Script = string`), letters carry a `role` and optional `forms`, and vowel
  signs / harakat / niqqud are all `marks`. So the same code teaches Devanagari,
  Arabic, or a future Hebrew with no type or logic changes. `data/scripts/README.md`
  documents the "add a script" checklist.
- **Compositional, not rote.** The data captures the *system* — the letter
  inventory, the vowel `marks`, and the `combination` rule (conjuncts, joining) —
  so the app teaches the generative pattern, not a flat list of thousands of
  syllables. (The neuroscience point in `HL02`.)
- **Stroke order is flagged conventional.** Freely-licensed authoritative
  stroke-order data does not exist for these scripts; every letter/mark carries
  `strokeOrderNote`, and the app renders it as "a typical way to write this."
- **Components are the "pieces."** The `components` array is the literal answer to
  the request — each glyph broken into named parts to practise one at a time.
- **Coverage hardens over time.** While a script is being authored it sets
  `"complete": false` and unknown headword characters are validator *warnings*;
  once the inventory is whole it flips to `true` and gaps become *errors*.

Tracks associate to a script through the built-in language→script map or a
per-track `track.json` (`{ "script": "hebrew" }`), so a brand-new-script language
needs no shared-map edit either.

## The data-layer package — `human-language-data`

`code/packages/typescript/human-language-data/` (src layout, publishable shell per
repo convention, though `private`). Responsibilities:

1. **Parse** every `lessons/*.md` frontmatter + `concepts/taxonomy.json` +
   `data/scripts/*.json` into an in-memory model.
2. **Expose typed accessors**:
   - `allConcepts(): Concept[]`
   - `conceptsByLanguage(lang): Concept[]`
   - `languagesForConcept(concept): Realization[]`  ← the interleaver's workhorse
   - `script(name): ScriptData`
3. **Validate** (see below), exposed both as a library function and a CLI
   (`human-language-data validate`) wired into CI.

The package ships the parsed dataset as a JSON artifact at build time so the app
can import it without re-parsing Markdown in the browser.

## The round-trip validator

Run as a `vitest` suite *and* a CI step (the repo favours round-trip checkers).
It asserts:

1. **Every `concept_tag` resolves** — it is either a key in `taxonomy.json`
   (universal) or matches the namespaced pattern `^[A-Z]{2}-[A-Z0-9-]+$` (lexical).
   An unknown bare tag is an **error**.
2. **One realization per (concept, language)** for `type: word` lessons — a
   universal concept realized twice in the same track is an error (ambiguous
   join). `review`/`practice-mix` lessons are exempt.
3. **Required realization fields present** — `headword`, `gloss`, `chapter`.
4. **Script references resolve** — every glyph a non-Latin headword uses appears
   in that script's `data/scripts/*.json` (warning while scripts are being
   authored; error once the script file declares itself complete).
5. **Core-concept coverage** — for each track whose `README.md` declares
   `parity: complete`, every `core: true` concept has a realization (error);
   otherwise reported as an informational coverage table.
6. **Field-shape checks** — `romanization` present for non-Latin `word` lessons
   (warning), `etymology_hook` ≤120 chars, `gender ∈ {masc,fem,neut,null}`.

Errors fail CI; warnings print a report. Coverage ≥95% for the package's own
logic (parsing, accessors, validation rules) per repo standard.

## Divergence & scope notes

- Like `HL00`, the *curriculum content* diverges from standard package process
  (no build/tests for Markdown). The **data-layer package is normal code** and
  follows full repo standards (tests, coverage, README, CHANGELOG, BUILD).
- This spec adds two optional frontmatter fields to the `HL00` schema
  (`romanization`, `etymology_hook`). `HL00`'s schema section should gain a
  one-line pointer to them; they are optional and back-compatible.
- Reconciling the existing tags onto the taxonomy is a content edit only — lesson
  *prose* refers to words by name, never by tag, so retagging never touches prose
  and never breaks a cross-link.
- Out of scope here: the app itself (`HL02`), audio/TTS, and any concept beyond
  what Chapters 1–3 across the tracks require (the taxonomy grows with the
  curriculum, one concept at a time, never front-loaded).
