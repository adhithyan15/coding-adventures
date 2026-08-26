import { defineConfig } from "vite";
import path from "node:path";
// How lesson batches are grouped lives in ONE module, imported by both this
// config and scripts/check-bundle.mjs. The gate used to recover the band width
// by regex-ing this file, which a comment mentioning the constant could shadow.
import { bandChunkNameForModuleId } from "./lesson-bands.mjs";

// The per-letter script data (glyph, components, stroke order) is the SAME data
// the HL01 layer publishes, at code/learning/human-languages/data/scripts/*.json.
// We import those canonical files directly rather than copying them, so the app
// can never drift from the curriculum. They live outside this package's folder,
// so the dev server must be told the repo root is a legal place to read from.
const repoRoot = path.resolve(__dirname, "../../../..");

export default defineConfig({
  // Relative base → the built index.html works when opened from any path
  // (local file, GitHub Pages sub-path, etc.) without a deploy-specific prefix.
  base: "./",
  server: {
    fs: { allow: [repoRoot] },
  },
  build: {
    // Keep the interactive shell below Vite's 500 kB warning threshold. These
    // canonical datasets change at different cadences and are independently
    // cacheable; lesson Markdown is already split lazily by `lessons.ts`.
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            {
              // A frontier import should stay small, but corpus-wide modes must
              // not fan out to one request per lesson.
              //
              // Batches are grouped by a CHAPTER RANGE -- five chapters of one
              // track's lesson series -- with a size cap kept only as a
              // backstop. That is issue #12918, and it replaces a grouping that
              // was track-then-size.
              //
              // WHY THE OLD SHAPE KEPT FAILING. Grouping by track and splitting
              // by size makes the batch count a function of corpus BYTES, so it
              // walked into the request ceiling every few content tranches. It
              // was answered twice by raising this cap, 32 kB -> 49 kB and
              // 49 kB -> 56 kB, and each bump looked sufficient because the
              // report showed a large unused fraction of the aggregate cap --
              // 32% after the second one, read at the time as headroom the next
              // tranches could grow into.
              //
              // It was not headroom. Rolldown groups by track and THEN splits
              // that track greedily by size, so every other track's tail batch
              // is sealed and never revisited. A Spanish tranche can only ever
              // extend Spanish's tail. Aggregate slack is stranded by
              // construction, and the measurement that settles it is that
              // adding 35 lessons weighing 145,711 B -- about 2.6 batches at the
              // 56 kB cap, and LIGHTER than the 35 that landed in the previous
              // tranche -- added SIX batches while the unused fraction sat
              // unchanged at 32%. A number that does not move when the thing it
              // supposedly measures does is not a measurement.
              //
              // WHAT THE CHAPTER RANGE BUYS. A band gains a batch only when a
              // track passes a chapter multiple, not when it gains lessons, so
              // the count follows chapters instead of bytes. Measured on the
              // corpus at the time of the change:
              //
              //     grouping              batches  largest   p90      median
              //     track + 56 kB cap         353   54,688   52,598   40,529
              //     5-chapter + 256 kB cap    281  200,124   90,225   40,731
              //     10-chapter + 256 kB cap   166  221,470  156,358   75,875
              //
              // FIVE and not ten, deliberately. Ten halves the count again, but
              // the number that costs a reader anything is the payload of the
              // batch their next lesson lands in, not how many batches exist.
              // A five-chapter band holds the median batch at 40,731 B -- within
              // 500 bytes of what the size-capped grouping already delivered --
              // while a ten-chapter band nearly doubles it. The count is amply
              // solved at 281; buying a further halving with double the
              // per-open payload is the wrong direction.
              //
              // THE CAP IS A BACKSTOP, NOT THE SPLITTER. At 256 kB it binds on
              // exactly one band in the whole corpus, which is what keeps the
              // count equal to the band count plus one rather than drifting back
              // toward byte-linear. Lowering it re-introduces the old shape a
              // little at a time; check the batches-vs-bands gap in
              // scripts/check-bundle.mjs before touching it.
              //
              // The pattern itself, the series letter it must capture, the track-name
              // whitelist and the digit bound all live in lesson-bands.mjs, next
              // to the band arithmetic that consumes them, so the gate cannot
              // drift from the bundler about which files exist.
              name: bandChunkNameForModuleId,
              // Backstop only -- see the note above. Mirrored by
              // scripts/check-bundle.mjs, which fails a batch the bundler did
              // not intend to emit.
              maxSize: 262_144,
            },
            {
              name: "script-data",
              test: /learning[\\/]human-languages[\\/]data[\\/]scripts[\\/]/,
              // Source-verified stroke metadata grows with every HL-C09
              // tranche. Keep the eager canonical corpus in a few cacheable
              // batches instead of allowing one chunk to cross the 500 kB
              // budget as soon as the next citation lands.
              maxSize: 250_000,
            },
            {
              // The track registry and the shared spine: ~18 kB between them,
              // read synchronously by the shell to name languages and resolve
              // spine nodes. Small, and it grows by a line per new TRACK rather
              // than per lesson, so it is safe to keep on the eager path.
              name: "curriculum-core",
              test: /core[\\/](?:languages|spine)\.json$/,
            },
            {
              // One chunk per track's authored plan, all of them lazy (see the
              // note on CURRICULUM_LOADERS in src/curriculum.ts).
              //
              // This SUPERSEDES the `maxSize: 250_000` briefly put on the old
              // single `curriculum-plans` group. That cap did satisfy the gate
              // -- it measures the LARGEST eager chunk -- but the browser went
              // on downloading the same ~500 kB before first paint, now as four
              // chunks instead of one, and the total kept growing every day.
              // HL-C110 wrote that trade down in advance as gaming the metric:
              // the fix has to remove bytes from the preload set. These are
              // gone from it entirely.
              //
              // Per-track is the other half: a tranche of Telugu words
              // re-downloads Telugu's plan alone, instead of invalidating one
              // shared half-megabyte blob on every corpus commit.
              name(moduleId) {
                const normalized = moduleId.replaceAll("\\", "/");
                const match = /human-languages\/([^/]+)\/curriculum\.json$/.exec(normalized);
                return match?.[1] ? `curriculum-${match[1]}` : null;
              },
            },
            {
              name: "book-ledgers",
              test: /(?:chapters\.json|generated-book-hashes[\\/][^/\\]+\.json)$/,
            },
            {
              // Handwriting grows one cited path at a time. Keep its model,
              // renderer, and font parser out of the interactive shell so
              // later source-backed letters do not consume shell headroom.
              name: "handwriting-tools",
              // The three modules moved into @coding-adventures/script-ductus,
              // so the path they are matched by changed with them. `scriptdata`
              // is NOT in this chunk: the app's shell needs SCRIPTS on first
              // paint, while the pen paths and the font parser are only needed
              // once a learner opens a letter's handwriting view.
              test:
                /script-ductus[\\/]src[\\/](?:strokes|ductusview|truetype)\.ts$/,
            },
          ],
        },
      },
    },
  },
});
