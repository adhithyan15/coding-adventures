# HL04 — Shared Spine, Lesson Contract, and Content Pipeline

## Status and purpose

This spec turns the Human Languages curriculum into one progressively disclosed
course system that can produce books, app lessons, audio scripts, and mixed-language
practice without maintaining four copies of the teaching content.

It extends [HL00](./HL00-human-language-curriculum-framework.md),
[HL01](./HL01-concept-taxonomy-and-data-layer.md), and
[HL03](./HL03-unified-language-learning-app.md). Where those specs describe an
aspiration that the current implementation does not satisfy, this spec defines the
migration target and the executable checks that decide when it is satisfied.

The design requirements are:

1. Most languages follow a shared, ordered **can-do spine**.
2. A language may insert, replace, delay, or omit spine material when its grammar,
   script, register system, or culture requires it.
3. Every lesson is strictly shorter than five minutes and assumes only knowledge
   established by earlier lessons in that language.
4. The canonical lesson content feeds the book, app, audio delivery, and practice
   compiler.
5. Learners choose which languages to mix. Cross-language practice never bypasses
   either language's own prerequisites.
6. Persian and Urdu are first-class tracks, not variants of Arabic or Hindi.
7. Etymology remains the signature memory aid, but it is paired with communicative
   use, graded input, output, fluency work, and cultural pragmatics.

## Evidence boundary

This is a benchmark of **methods and publicly documented features**, not a claim to
have enumerated every language-learning book and not a reconstruction of copyrighted
text. The project adopts transferable pedagogical patterns and writes original
lessons, examples, stories, explanations, and exercises. Publisher descriptions,
open courses, proficiency frameworks, and research establish the design rationale;
they are not source text for the corpus.

## Representative method benchmark

No single course tradition covers the whole target. The useful design is a synthesis.

| Tradition or framework | Transferable strength | What HL adopts |
|---|---|---|
| McGraw-Hill *Easy Spanish Step-by-Step* | Grammar is ordered so later structures build on earlier ones. | An explicit dependency graph and cumulative practice, but grammar is one strand rather than the entire syllabus. |
| Assimil | Short, graded dialogues; notes at the point of need; an input-heavy phase followed by active production; periodic consolidation. | Every new form appears first in understandable input, then in guided output. Reviews revisit a small run of prior lessons. |
| Pimsleur | Anticipation, spoken production, core vocabulary, and graduated recall. | Audio-ready prompts require an answer before reveal; successful items move to expanding intervals. Long sessions are compiled from independent sub-five-minute lessons. |
| *Lingua Latina per se illustrata* | Carefully controlled input grows from transparent sentences into sustained reading. | Recurring micro-stories and dialogues use only known material plus the lesson's declared introduction budget. Complexity grows through text, not only explanation. |
| Routledge Colloquial courses | Contemporary dialogues, jargon-free grammar, all four skills, native audio, answer keys, and interval reviews. | Present-day usage, explicit self-study answers, periodic mastery checks, and a four-skill coverage report. |
| *Genki* | Everyday dialogue, plain grammar notes, varied practice, and a coordinated reading/writing progression. | Script and grammar extensions attach to the communicative lesson that needs them; neither becomes a disconnected front-loaded gate. |
| *Integrated Chinese* | Coordinated listening, speaking, reading, and writing; real-life situations; culture; interpretive, interpersonal, and presentational modes. | Every spine cluster declares communicative outcomes and mode coverage, not just vocabulary inventory. |
| *Alif Baa* / Al-Kitaab | Sounds, connected script, production from the beginning, and explicit spoken/formal variety choices. | Script is learned through usable language; variety and register are structured metadata rather than silently mixed forms. |
| Modern Persian beginner courses | Spoken Persian from the outset, a clear script ramp, concise grammar, audio dialogues, and four-skill exercises. | Persian distinguishes colloquial and formal forms, introduces connected letters in real words, and progressively fades transliteration. |
| Modern Urdu and open Urdu-script courses | Nastaliq taught through shape families, contextual forms, real-world examples, and cumulative reading/writing practice. | Urdu receives Nastaliq-aware typography and script lessons, not generic Arabic glyph presentation. Hindi–Urdu connections are enrichment and practice bridges, never assumed prerequisites. |
| CEFR Companion Volume | Action-oriented can-do goals; reception, production, interaction, mediation; plurilingual and pluricultural competence. | The shared spine is made of communicative abilities. Mixed-language tasks include comparison and mediation without pretending languages are structurally identical. |
| ACTFL Proficiency Guidelines 2024 | Interpretive, interpersonal, and presentational communication across modalities. | Coverage is reported by mode and skill. Book knowledge alone cannot mark a stage complete. |
| Nation's Four Strands | A balanced course includes meaning-focused input, meaning-focused output, language-focused learning, and fluency development. | Rolling curriculum windows report all four strands; etymology and grammar cannot crowd out input, output, or fluency. |
| Retrieval and spacing research | Retrieval improves delayed retention; distributed practice beats massed restudy. | Practice records attempts, gives corrective feedback, and schedules later retrieval rather than counting exposure as mastery. |
| Recent L2 interleaving research | Interleaving supports discrimination, but immediate interleaving can overload initial acquisition; a focused-to-mixed progression is safer. | New material receives a short focused acquisition phase. It joins mixed-language review only after an initial success criterion. |

## Current coverage and gaps

The 2026-08 audit of `origin/main` found 18 tracks and 963 lesson files. The
foundation is real, but the governing promises are not yet enforceable.

| Requirement | Current coverage | Migration gap |
|---|---|---|
| One atomic lesson | Strong: most content lessons teach one word or phrase. | Some lessons are long enough to contain several independent teaching steps. |
| Strictly less than five minutes | Not enforced: 364 lessons declare 5 minutes and 104 declare 6. | Add a computed duration budget and split every lesson at or above 300 seconds. |
| Etymology and morphology | Strong prose tradition and root metadata. | Normalize etymons, record sources/confidence, and deliver the full explanation to the app. |
| Inline grammar | Present in Markdown prose. | Grammar has almost no structured metadata, cannot be ordered or practised by the app, and cannot be closure-checked. |
| Inline script | Writing lessons and script JSON exist. | Writing lessons do not participate in the concept sweep; the app gives a one-time script signature rather than a cumulative reading path. |
| Shared cross-language concepts | 42 concepts are shared by two or more languages. | The shared layer is mostly greetings and A0 vocabulary; 385 concepts remain language-namespaced. |
| Safe progression | Lesson-id prerequisites have no cycles or unknown ids. | Sixty lessons have no prerequisites, including 42 later-chapter lessons; assumed vocabulary, grammar, sounds, and glyphs are not represented. |
| Single content source | Claimed by HL00. | The Markdown body is discarded by the parser, LaTeX chapters are handwritten copies, and the app consumes frontmatter summaries only. |
| Four skills and communicative use | Audio-script cues and guided prompts exist. | There is no structured listening, interaction, writing, or can-do coverage model, and no audio asset contract. |
| Graded input and sustained reading | Isolated example sentences occur. | There is no controlled micro-story/dialogue corpus or known-token validation. |
| Mixed-language practice | Concept sweeps, cumulative review, and mixed-script drills exist. | Learn mode hard-codes 10 languages, learners cannot choose a set, and a concept can expose a late lesson early in another language. |
| Register, dialect, and culture | Often explained in lesson prose. | These distinctions are not typed, so the app can silently compare non-equivalent registers or varieties. |
| Proficiency target | Roadmaps generally point toward B1. | No CEFR/ACTFL-aligned outcome matrix shows which reception, production, interaction, or mediation abilities are covered. |

### First implementation slice

The first implementation slice establishes, without replacing existing books or
tracks:

- `core/languages.json`, containing all 20 current tracks in one registry;
- `core/spine.json`, giving all 45 current canonical concepts an explicit first
  shared-spine order;
- Persian and Urdu starter tracks, each with five dependency-ordered, under-five-
  minute lessons and language-specific script metadata;
- lossless Markdown bodies and lossless discovery of all 17 existing LaTeX books
  in `human-language-data`, with chapter-to-lesson alignment validation;
- a registry-driven app language picker, the structured spine in Learn mode,
  rendered authored lesson bodies, and fail-closed declared prerequisites.

This slice does not claim the full acceptance criteria below. Per-track spine
maps, typed body blocks, computed duration, book generation, independently gated
focused-to-mixed retrieval, a true Nastaliq font, and B1 corpus expansion remain
explicit migration work.

## The curriculum model

The curriculum has four different kinds of object. Keeping them separate prevents
the flat `concept_tag` vocabulary from being asked to do incompatible jobs.

1. **Spine node** — a language-independent communicative ability, such as
   “exchange names politely” or “describe where a familiar object is.”
2. **Knowledge atom** — a lexeme, construction, morpheme, sound contrast,
   grapheme, pragmatic convention, cultural fact, or etymon.
3. **Language realization** — how one language teaches a spine node, including
   its ordered lessons and required extensions.
4. **Activity** — an authored or compiled act of reception, production,
   interaction, mediation, language-focused study, or fluency practice.

`concept_tag` remains the join key for comparable meanings. It is not the spine
order and it is not a grammar syllabus.

### Shared spine

The shared spine is an ordered directed acyclic graph. Nodes use stable semantic
ids, never ordinal ids:

```jsonc
{
  "id": "SPINE-INTRO-EXCHANGE-NAMES",
  "stage": "pre-A1",
  "canDo": "I can ask someone's name and give my own in a neutral setting.",
  "prerequisites": ["SPINE-GREETING-MEET"],
  "modes": ["interpretive", "interpersonal", "presentational"],
  "strands": ["meaning-input", "meaning-output", "language-focus", "fluency"],
  "core": true,
  "extensionPoints": ["script", "register", "copula", "pronouns"]
}
```

The canonical stages are `pre-A1`, `A1`, `A2`, and `B1`. They are curriculum
planning labels, not claims that lesson completion alone proves external exam
proficiency.

The first shared-spine families are:

1. meet, greet, thank, apologize, and take leave;
2. exchange identity and basic personal information;
3. understand and make simple requests;
4. locate, possess, and describe familiar people and objects;
5. talk about daily actions, preferences, ability, obligation, and plans;
6. narrate simple past events and contrast background with completed actions;
7. compare, explain a reason, give an opinion, and connect clauses;
8. manage common transactions, travel, health, time, and social situations;
9. understand and produce short connected texts;
10. mediate a simple meaning between English and one or more learned languages.

This order moves from immediately useful formulae through generative sentence
building into connected discourse. Individual languages may realize the functions
with radically different structures.

### Per-language plan and extensions

Each track gains a `curriculum.json` that maps spine nodes to an ordered local
path:

```jsonc
{
  "language": "urdu",
  "spine": {
    "SPINE-INTRO-EXCHANGE-NAMES": {
      "lessons": ["UR-C02-aap", "UR-C02-naam", "UR-C02-mera-naam"],
      "before": ["UR-SCRIPT-BE-FAMILY", "UR-GRAMMAR-POSSESSIVE-AGREEMENT"],
      "after": ["UR-REGISTER-TUM-AAP"],
      "omits": []
    }
  }
}
```

An extension may be **required**, **supporting**, **reference**, or explicitly
**not applicable**. No tool infers order from the minimum chapter number found in
another language. The shared node orders abilities; each `curriculum.json` orders
the realization.

## Canonical lesson contract

Markdown remains the authoring format, but both frontmatter and body become typed.
The parser must preserve the body as a lesson AST instead of discarding it.

```yaml
---
schema_version: 2
id: ES-C01-gracias
spine_node: SPINE-COURTESY-THANK
sequence: 40
type: word
headword: gracias
gloss: thank you
concept_tag: COURTESY-THANKS
duration:
  max_seconds: 240
requires:
  knowledge: [ES-SOUND-G, ES-LEX-GRACIAS-RECEPTIVE]
introduces:
  knowledge: [ES-LEX-GRACIAS-PRODUCTIVE, ETYMON-LATIN-GRATIA]
practises:
  knowledge: [SPINE-GREETING-MEET]
skills: [listening, speaking, reading]
modes: [interpretive, interpersonal]
strands: [meaning-input, meaning-output, language-focus]
register: neutral
variety: general
sources: [rae-gracias, dle-gracia, de-vries-gratia]
---
```

Each level-two body block begins with a compact metadata comment. The comment is
part of the canonical AST and source hash, but renderers omit it from learner
copy:

```markdown
## The word, taken apart
<!-- hl-knowledge: introduces=[ES-LEX-GRACIAS-PRODUCTIVE]; assesses=[] -->

**gracias** is the everyday way to say "thank you."

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[ES-LEX-GRACIAS-PRODUCTIVE] -->

- [YOU SAY: "gracias"]
```

The body is parsed into stable, named blocks: `warmup`, `input`, `notice`,
`pronunciation`, `script`, `etymology`, `grammar`, `culture-pragmatics`,
`guided-production`, `comprehension`, `fluency`, and `recall`. A block may declare
prompts, accepted answers, feedback, and response time. The app and book render the
same block; medium-specific presentation belongs in renderers, not a second copy.
Every schema-v2 block authors both lists, including explicit empty lists. The
union of block introductions must equal `introduces.knowledge`; every assessed
atom must be declared in `practises.knowledge`; and guided-production and recall
blocks must name at least one assessed atom.

### Knowledge closure

Before a lesson begins, every atom in `requires.knowledge` must have been
introduced by a transitive prerequisite. During the lesson, assessed material may
use only previously introduced atoms, atoms introduced earlier in this lesson, and
transparent English instructions.

The validator computes this set at every block boundary. It checks assessments
before adding that block's introductions, so a form must be established by a
prerequisite or by an earlier block rather than introduced and tested in one
opaque step. Unknown atoms, cycles, forward references, missing or malformed
directives, undeclared assessments, and assessed unavailable forms are errors.
Runtime scheduling fails closed and reports a curriculum defect; it never unlocks
everything as a fallback.

### Five-minute contract

`max_seconds` must be `1..299`. CI independently estimates delivery time from
spoken word count, explicit pauses, repeat count and audio duration, a learner-
response budget for every prompt, and a safety margin. The greater of the declared
and computed values is used. A lesson at 300 seconds or more fails validation.

One session may chain several lessons, but each lesson is independently resumable.
A lesson has one primary outcome. The default introduction budget is one lexical
item or fixed phrase, one grammatical contrast, or one script pattern. A track may
raise the atom count only for transparent decompositions and must still satisfy
knowledge closure and duration.

## How grammar, vocabulary, and script grow

### Grammar

Grammar is introduced function-first: understand a meaningful example, notice the
contrast, explain it in plain English, optionally name the conventional term,
manipulate one variable, use it in a tiny communicative task, and retrieve it later
in a different context. Full paradigms are reference views assembled from already
encountered cells, not lesson-sized information dumps.

### Vocabulary and graded input

Vocabulary selection records frequency band, communicative utility, register,
variety, concreteness, and root connections. Frequency informs the choice; it does
not override a high-value communicative need.

Every spine cluster includes a recurring dialogue or micro-story. Each installment
uses known language plus the current lesson's explicit introduction budget. This
creates the graded reading/listening ramp missing from isolated word lessons.

### Script and transliteration

The target script is visible from first contact. Transliteration is temporary:

- show target script and audio together;
- teach letters by shape family, sound, joining behavior, and position in real words;
- record contextual forms, ligatures, marks, and non-joining rules;
- fade transliteration per learner after successful script retrieval;
- retain an on-demand pronunciation aid for accessibility;
- test directionality, Unicode normalization, font shaping, and cursor behavior;
- never present a generic script font as culturally equivalent when a language's
  conventional typography differs.

## Etymology contract

Etymology is an elaborative memory hook, not a substitute for current meaning and
usage. Every claim records a canonical etymon id, source language and script,
transliteration, gloss, change chain, relation type, confidence, traceable sources,
semantic change, and false-cognate boundaries.

The app joins canonical etymon ids, not raw root strings. A lesson first establishes
what an expression means and how it is used, then uses the root web to strengthen
memory.

## Balanced skill and proficiency coverage

Over each rolling spine cluster, the coverage report expects meaningful activity
in all four of Nation's strands: meaning-focused input, meaning-focused output,
language-focused learning, and fluency development with known material. The target
is approximate balance, reported as a range rather than a false exact 25 percent.

The same report tracks listening, speaking, reading, writing; interpretive,
interpersonal, and presentational communication; CEFR reception, production,
interaction, and mediation; cultural/pragmatic competence; and controlled versus
fluent use.

Stage completion requires original performance tasks with transparent rubrics. A
multiple-choice recognition score alone cannot complete a can-do node.

## Mixed-language learning and practice

The learner selects any active language subset and may reorder it. Progress and
knowledge closure remain per language. Each new `(knowledge atom, language)` moves
through:

1. **encounter** — understandable input;
2. **focused acquisition** — a small block of same-language supported practice;
3. **initial retrieval** — correct production or comprehension without reveal;
4. **eligible for interleaving** — mixed with established items;
5. **fluency and transfer** — faster or less scaffolded use in new contexts.

Supported directions include meaning → target language, target language → meaning,
audio → meaning or form, dictation, language A → language B through a shared
concept, same-etymon comparison, contrastive grammar/pragmatics, and simple
mediation.

Language A → language B becomes eligible only when both realizations independently
pass initial retrieval. The scheduler respects each realization's local spine
position. A cross-language answer never grants mastery to an unseen local lesson.
Wrong answers record the chosen language/form so contrastive feedback can distinguish
a memory lapse from cross-language confusion.

## Persian and Urdu onboarding

Persian and Urdu share ancestry and many shapes with the Arabic script, but they
must not reuse an undifferentiated `arabic` course identity. Script data gains
inheritance or composition:

```text
arabic-script-core
├── arabic-orthography
└── perso-arabic-core
    ├── persian-orthography
    └── urdu-nastaliq-orthography
```

### Persian

The first extension path covers right-to-left flow, joining and non-joining letters,
Persian additions, omitted short vowels, contextual forms, common orthographic
ambiguities, and transliteration fading. Formal written Persian and colloquial
Iranian Persian are typed varieties; Dari and Tajik are future related varieties,
not silent synonyms. Arabic loans preserve historically informative spellings while
receiving Persian pronunciation and usage notes.

### Urdu

Urdu renders and tests with a suitable Nastaliq font and layout. Its extension path
covers shape families, positional forms, non-joiners, short-vowel omission, Urdu
additions for Indic sounds, aspiration and retroflexion, nasalization, and common
ligatures. Naskh may appear as comparison or accessibility fallback, but not as the
normal printed Urdu experience.

Hindi and Urdu may share communicative spine nodes and selected Hindustani grammar
atoms. Their scripts, register distributions, literary vocabulary, and cultural
pragmatics remain distinct. Cross-language practice starts only after both sides
are learned.

## One-source content pipeline

```text
spine.json + per-track curriculum.json + lessons/*.md + source registry
                              │
                       validated lesson AST
              ┌───────────────┼───────────────┬──────────────┐
              ▼               ▼               ▼              ▼
         LaTeX/HTML book   app teaching UI   exercise bank   audio script
```

Required changes:

1. `human-language-data` preserves and parses every Markdown body block.
2. Book chapters are rendered from the lesson AST. Handwritten LaTeX supplies
   templates and front/back matter, not duplicate explanations.
3. The app renders the same blocks and compiles exercises from their prompt/answer
   contracts.
4. Generated outputs embed a source hash. CI rebuilds when lessons, spine, taxonomy,
   source registry, renderer, or templates change and rejects drift.
5. Audio uses the same blocks plus pronunciation metadata; recordings and synthesized
   audio carry provenance and variety labels.
6. Linguistic and etymological citations use stable ids in a source registry.

Book and app may arrange the same blocks differently; they may not silently teach
different facts or prerequisite order.

## Validation gates

Tracks migrate independently. `schema_version: 1` produces migration warnings;
once a track declares version 2, these are hard errors:

1. shared and local curriculum graphs contain no cycles or unknown ids;
2. every lesson and body block is reachable in one authored order;
3. every required or assessed knowledge atom is known at its point of use;
4. computed lesson duration is below 300 seconds;
5. exercises have resolvable prompts, answers, accepted variants, and feedback;
6. script glyphs, forms, marks, direction, normalization, and font shaping resolve;
7. variety and register are explicit where they affect appropriateness;
8. etymological claims have canonical relations, confidence, and sources;
9. every spine cluster reports strand, skill, mode, and can-do coverage;
10. book and app lesson hashes match the canonical AST;
11. mixed practice includes only independently unlocked realizations;
12. stage assessment tests production or interaction as well as recognition.

The validator emits human-readable and JSON gap reports so corpus expansion is
measurable.

## Migration order

1. **Contract and parser** — add spine, knowledge-atom, body-AST, source, and
   duration types while preserving version-1 compatibility.
2. **Spanish vertical slice** — migrate Chapters 1–3, render book and app, and
   prove strict duration plus closed-world progression.
3. **Mixed-practice correction** — add learner-selected languages, per-language
   cursors, focused-before-interleaved eligibility, and fail-closed scheduling.
4. **Persian and Urdu pilots** — build script composition and the first three spine
   clusters in each language with appropriate typography and variety metadata.
5. **Existing-track migration** — map every track onto the spine, insert extensions,
   normalize etymons, and eliminate handwritten book copies.
6. **Corpus expansion** — grow through B1 while coverage reports select the next
   missing can-do, skill, mode, register, or language realization.

## First milestone acceptance criteria

The first vertical slice is complete when every migrated lesson computes to 299
seconds or less; no assessed word, construction, sound, or glyph appears early;
book and app display the same lesson facts from the same AST; a learner can select
two or more languages without exposing a later local lesson; new items receive
focused retrieval before mixed review; progress is per language, atom, skill, and
prompt direction; Persian and Urdu can extend the spine without pretending their
scripts or registers are Arabic or Hindi; and CI reports gaps across the full corpus.

## Sources

These sources support the benchmark and design rationale. They are not licenses to
copy lesson content.

- McGraw Hill, [*Easy Spanish Step-by-Step*](https://www.mheducation.com/highered/mhp/product/easy-spanish-step-step.html)
- Assimil, [The Assimil method](https://www.assimil.com/en/articles/5-the-assimil-method)
- Pimsleur, [The Pimsleur Method](https://www.pimsleur.com/the-pimsleur-method/)
- Schola Latina, [*Lingua Latina per se illustrata*: method](https://scholalatina.it/en/latine-discere-aude-our-method/)
- Routledge, [Colloquial Series](https://www.routledge.com/Colloquial-Series/book-series/SE0069)
- Japan Times Publishing, [What is *Genki*?](https://genki3.japantimes.co.jp/en/intro/index.html)
- Cheng & Tsui, [*Integrated Chinese*](https://www.cheng-tsui.com/browse/integrated-chinese-3rd-edition)
- Georgetown University Press, [*Alif Baa*](https://press.georgetown.edu/Book/Alif-Baa-with-Website)
- Routledge, [*Colloquial Persian*](https://www.routledge.com/Colloquial-Persian-The-Complete-Course-for-Beginners/Rafiee/p/book/9781032356730)
- Routledge, [*Colloquial Urdu*](https://www.routledge.com/Colloquial-Urdu-The-Complete-Course-for-Beginners-2nd-Edition/Bhatia-Koul/p/book/9781315649672)
- University of Texas at Austin, [Persian Online](https://sites.la.utexas.edu/persian_online_resources/)
- Northwestern University, [*Zer o Zabar*](https://openbooks.library.northwestern.edu/zerozabar/)
- Council of Europe, [CEFR Companion Volume](https://book.coe.int/en/education-and-modern-languages/8150-common-european-framework-of-reference-for-languages-learning-teaching-assessment-companion-volume.html)
- ACTFL, [Proficiency Guidelines 2024](https://www.actfl.org/uploads/files/general/Resources-Publications/ACTFL_Proficiency_Guidelines_2024.pdf)
- Nation and Yamamoto, [Applying the Four Strands](https://www.wgtn.ac.nz/lals/resources/paul-nations-resources/paul-nations-publications/publications/documents/yamamoto-four-strands.pdf)
- Roediger and Karpicke, [Test-enhanced learning](https://doi.org/10.1111/j.1467-9280.2006.01693.x)
- Cepeda et al., [Distributed practice](https://pubmed.ncbi.nlm.nih.gov/16719566/)
- Hwang, [Initial blocked practice and interleaving in L2 vocabulary](https://onlinelibrary.wiley.com/doi/full/10.1111/lang.12659)
