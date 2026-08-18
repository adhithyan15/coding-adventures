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
              // shape. 48 kB gives 262 batches for the same 8.2 MB, a 35% cut in
              // requests, with the largest batch at ~47 kB -- still a small lazy
              // fetch, and now with headroom for a corpus that grows every day.
              name(moduleId) {
                const normalized = moduleId.replaceAll("\\", "/");
                const match = /human-languages\/([^/]+)\/lessons\//.exec(normalized);
                return match?.[1] ? `lessons-${match[1]}` : null;
              },
              maxSize: 48_000,
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
              name: "curriculum-plans",
              test: /(?:curriculum\.json|core[\\/](?:languages|spine)\.json)$/,
              // Same shape as script-data above: 22 tracks' worth of curriculum
              // JSON grows with every tranche, and this group had no cap until a
              // human-languages merge pushed the single chunk to 502 kB, over the
              // 500 kB eager-chunk budget. Capped rather than the budget raised,
              // for the same reason script-data was capped instead of the budget.
              maxSize: 250_000,
            },
            {
              name: "book-ledgers",
              test: /(?:chapters\.json|generated-book-hashes\.json)$/,
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
