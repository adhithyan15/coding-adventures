import { defineConfig } from "vite";
import path from "node:path";

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
              // not fan out to one request per lesson. Rolldown first groups by
              // track, then caps each batch by size.
              //
              // The cap was 32 kB, and the Spanish C2 chapters pushed the corpus
              // to exactly 400 batches -- one over the 399-request ceiling in
              // scripts/check-bundle.mjs. Raising that ceiling would have bought
              // one PR of room and made the app slower; raising the cap fixes the
              // shape. Keep the grouping cap aligned with the independently
              // enforced emitted-batch ceiling: that lets Rolldown fill the
              // final batch for each language instead of stranding usable bytes
              // in undersized tail chunks, without weakening either request or
              // response-size gates.
              //
              // 49 kB -> 56 kB. SECOND occurrence of that same recurrence, from
              // the Spanish A1 vocabulary tranche that took the corpus to 401
              // batches. Measured on that corpus:
              //
              //     cap 49 kB   401 batches   47,976 B largest
              //     cap 56 kB   353 batches   54,688 B largest
              //
              // Note which way the numbers move. This is a GROUPING parameter,
              // not a budget. Raising it takes the request count DOWN by 48, and
              // the request-count ceiling in check-bundle.mjs is lowered to the
              // measured 353 in the same commit -- a ceiling that may fall should
              // fall when it falls. Nothing was relaxed to squeeze past a gate:
              // the gate is met with more margin than before, and 54,688 B is
              // about 11% of the 500 kB chunk budget that is the constraint
              // actually protecting the browser.
              //
              // If you are here for a THIRD bump: stop, and do issue #12918
              // instead. Batches are grouped by track-then-size, so the count
              // tracks corpus bytes linearly and this recurs every few content
              // tranches. Grouping by a chapter range -- something a reader
              // actually navigates -- makes it grow sublinearly and ends this.
              name(moduleId) {
                const normalized = moduleId.replaceAll("\\", "/");
                const match = /human-languages\/([^/]+)\/lessons\//.exec(normalized);
                return match?.[1] ? `lessons-${match[1]}` : null;
              },
              maxSize: 56_000,
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
