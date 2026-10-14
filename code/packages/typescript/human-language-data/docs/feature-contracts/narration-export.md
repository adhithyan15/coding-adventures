# The narration export (HL08)

The audio-script output [`HL04`](../../../../../specs/HL04-shared-spine-and-content-pipeline.md)'s
pipeline diagram has always named. It turns the lesson AST into something an AI voice
assistant can read aloud while the learner drives — HL08's stated purpose, in the
project owner's own words: *"I want to be able to have one of the AI chatbots with
voice capabilities read through and teach me while I am driving."*

```bash
npm run build
npm run generate:narration   # writes <language>/narration/chNN.{txt,json}
npm run check:narration      # byte-for-byte; exits 1 on drift
```

Two views per chapter:

- **`chNN.txt`** — one continuous script. Hand it to any voice assistant with "read
  this to me". Directives appear as bracketed stage directions (`[pause 2 seconds]`,
  `[your turn — say: …]`, `[question — say your answer, then pause 9 seconds]`),
  because that is a form every model already reads as an *instruction to the reader*
  rather than words to pronounce.
- **`chNN.json`** — the same script with its joints intact: typed segments a voice
  agent can act on, so it pauses where the lesson says pause, waits where it says
  `[YOU SAY: …]`, and **scores a spoken answer against the compiled activity contract
  from `activity.ts`** — never against prose.

That last point is the module's governing rule. Lessons contain two similar-looking
things and this export never conflates them:

| In the lesson | Becomes | Scored? |
|---|---|---|
| `[YOU SAY: "hola" — OH-la]` | a `prompt` segment | ❌ — a rehearsal, no answer key |
| `<!-- hl-activity: {…} -->` | an `activity` segment | ✅ — `acceptedResponses` from `compileLessonActivities` |

**Tables are linearised, never dropped.** A two-column word→gloss table becomes
*"नमस्ते means hello"*; a three-column table becomes labelled facts, *"Language:
Telugu. Hello: namaskāram. Source: Sanskrit."* Three columns is where a table stops
being a list of facts a listener can hold and starts being a grid whose meaning lives
in the comparison *across* rows — the corpus's own four-column tables prove it, with an
unlabelled first column that only means something because of where it sits on the
page. At that width the lineariser reads **371 of the corpus's 442 tables (84%)**,
covering 272 of the 340 table-bearing lesson files.

A table it refuses is **spoken, not skipped**: the learner hears its size, its column
headings, and why it needs eyes, and the lesson is marked `sight` so they are warned
before they start. `sight` and `pen` lessons still export in full, opening with a
notice naming what they will need and which sections to leave until they have stopped.

Target-script text carries its `romanization` alongside — *"خداحافظ (khodâ hâfez)"* —
using the whole chapter's headwords, so a lesson can pair a word a neighbouring lesson
introduced. Pairing is whole-word only: the Arabic track teaches ا (*alif*) as its own
lesson, and a substring replace once turned سلام into `سلا (alif)م`.

The export is hash-gated exactly like the generated `.tex`:
`core/generated-narration-hashes/<language>.d/NNNN.json` records an FNV-1a
fingerprint of one chapter's lesson AST and of the two files generated from it,
so a lesson edited without re-running the exporter fails `--check` instead of
leaving a voice assistant confidently teaching a lesson that no longer exists.
`_meta.json` owns the ledger-wide version, language, algorithm, and table-width
setting.
There is no tracked per-language aggregate: regenerating chapter 7 updates
`0007.json` without colliding with an agent working on chapter 8.

`narrationLessonIdentityIndex(root, expectedLanguages)` is the shared strict
corpus projection for gates that need narration-owned lesson identities without
inventing another validator. It proves exact language-directory equality before
opening owner bytes, then returns stable per-language lesson ids while rejecting
unsafe or reserved identities, duplicate or case-fold-colliding ownership across
chapters and languages, aggregates, nesting, symlinks, and non-regular owners.

**Out of scope, per HL04 and HL08:** no audio. No TTS, no voice selection, no
recordings. This is a script *for* a voice agent.

remaining 1,038, 308 carry a Markdown table — the single largest obstacle to a
hands-free course, and a far more tractable one than the script. **708 lessons
(65%) are drivable exactly as authored.** No track has yet authored an interspersed
writing segment, so every lesson's core equals its full modality and the two-scale
derivation currently moves no number; the regression test pins that.
