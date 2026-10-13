# The modality manifest — two editions from one source (HL-C44)

The derivation above is only useful to a *program* if it is data.
`core/lesson-modality/<language>.d/` holds one independently mergeable owner per
lesson: `_meta.json` records stable `version`, language, hash algorithm, features,
and policy, while `<lesson-id>.json` records `{ lesson, findings }`. On the current
23-language corpus that is 4,485 owners for 4,462 lessons. The complete book, the
app, and the forthcoming dictation-friendly driving edition can each filter the
same canonical corpus instead of maintaining three copies of it.

```bash
npm run build
npm run generate:modality   # write direct owners under core/lesson-modality/*.d/
npm run check:modality      # fail (exit 1) if it drifted from the lessons
```

```ts
import { loadModalityManifest, modalityManifestById } from "@coding-adventures/human-language-data";

const manifest = loadModalityManifest();
manifest.summary.drivablePercent;        // 65
manifest.lessons.filter((l) => l.drivable);            // the driving edition's lessons
manifest.tracks[0].chapters[0].drivableLessonIds;      // the prefix, already in order
modalityManifestById(manifest).get("ES-C01-hola");     // a Map, never a plain object
```

Each reconstructed lesson row carries `id`, `language`, `chapter`, `sequence`,
`modality`, `derived`, `drivable`, `reasons`, and the lesson AST's `sourceHash`; the
three override fields (`authored`, `authoredReason`, `overridden`) appear only on the
handful of lessons that have them. Source hashes, chapter ordering and drivable
prefixes, track summaries, and the corpus summary are reconstructed from direct
owners. They are not additional tracked aggregates.

The loader first requires exact language-directory equality with
`core/languages.json`. Within every directory it requires exact owner-ID equality
with parsed lesson sources and independently with generated narration-hash lesson
IDs through the shared `narrationLessonIdentityIndex` boundary, before trusting
owner content. Owners must be canonical JSON, direct regular
files whose filename, embedded lesson id, and embedded language agree. Missing,
unexpected, nested, symlinked, case-colliding, unsafe, or noncanonical owners fail;
so does any resurrected `core/lesson-modality/<language>.json`. There is no aggregate
fallback. The generator builds and validates a complete replacement under a private
staging root, publishes it only after all three identity sets agree, and removes the
old aggregate last. See
[`HL32`](../../../../../specs/HL32-sharded-lesson-modality-ownership.md).

Two design decisions are worth knowing before consuming it:

- **`modality` is permanently the strongest channel the lesson needs anywhere.** It is
  the conservative filter. HL-C41 has now added *block-level* modality — a lesson core
  that is `voice` beside a short, separable `pen` segment — and it landed as a new
  optional `coreModality` key beside `modality`, never as a change to it. A consumer
  reads `entry.coreModality ?? entry.modality` and is correct both before and after;
  one that never learns about the new key keeps producing a merely *pessimistic*
  driving edition, which is the safe direction to be wrong in. `features.blockModality`
  in the header says whether a given build carries block data.
- **Nothing is authored.** The manifest is derived, exactly like
  `core/generated-book-hashes/<language>.d/NNNN.json`. HL08 deliberately refused to put
  `modality:` in
  1,096 frontmatter files, because 1,096 authored copies of a computed fact are 1,096
  places for it to go stale. `check:modality` runs in CI beside `check:books` so the
  manifest cannot drift from the lessons it describes.
