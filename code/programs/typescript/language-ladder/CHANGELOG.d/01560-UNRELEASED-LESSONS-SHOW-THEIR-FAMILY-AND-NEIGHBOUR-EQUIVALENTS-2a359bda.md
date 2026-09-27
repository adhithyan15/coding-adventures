## Unreleased — lessons show their family and neighbour equivalents (HL41)

Opening a Tamil vocabulary lesson now shows the panel the book prints: "In the
family, and next door". Each row is one language:

- Kannada, Telugu and Malayalam;
- Hindi, marked as the neighbour;
- English, last.

Each word is in its own script with its romanization, and "same root" appears
only where the author judged it. The table goes under the lesson's first
teaching section.

- `src/equivalents.ts` is the pure half. It turns an owner file into rows. A
  malformed file produces no panel, never half of one.
- `src/equivalents-sources.ts` is the lazy half. It holds the loader map for
  the `equivalents.d` owner files, the comparison sets, and the builder that
  makes the table from text nodes only.
- The owner files are grouped into one lazy chunk per track in
  `vite.config.ts`, so opening lessons costs one request per track, not one per
  word.
- `scripts/check-bundle.mjs` now fails if the source-map chunk or any track's
  data chunk becomes eager.

The largest eager chunk is unchanged at 495,771 bytes.
