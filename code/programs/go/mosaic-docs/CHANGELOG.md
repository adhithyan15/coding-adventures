# Changelog — mosaic-docs

## [Unreleased]

### Added — per-backend style coverage, measured on every build (#14709)

A `coverage.html` page, linked from the index, recording which mosstyle
properties each backend actually lowers. The answer is worse than anyone had
assumed: the three DOM backends lower 98-100% of the properties this repository
authors, and the five native ones lower 15-46%. Everything else is dropped in
silence, because mosstyle properties are freeform and nothing in the pipeline
knows what a backend can express.

Measured rather than written down, and re-measured on every build, so it cannot
drift from the emitters the way a hand-maintained table would.

The method is differential: emit the same probe component with and without a
property and compare the whole generated output. Text-matching the authored
value is wrong in both directions -- SwiftUI writes `#abcdef` as `Color(red:
0.671, ...)` so colours read as unsupported, and a short value like `3` matches
unrelated output so unsupported properties read as supported.

Two details the first draft got wrong, both caught by checking the result
against a hand-run measurement rather than trusting the tool:

- **Probe shape changes the answer.** `gap` on Qt and `align` on XAML lower only
  inside a Row, so each property is tried on both a Box and a Row probe.
- **One probe value is not enough.** A value equal to the backend's own default
  changes nothing even where support is complete. The most-authored
  `background` in this repository is `transparent`, which alone reported Qt and
  Flutter as unable to paint a background at all. Up to three authored values
  are now tried per property.

Probe values and the property list are censused from the repository's own `.msl`
files, so the report cannot test a value nobody writes, and rows are ordered by
real authored usage.

Opt out with `-coverage=false` for a fast local run.

### Added — per-story variant galleries (#14026, #14459)

A component with a sibling `<Component>.stories.json` now renders one frame per
story instead of a single sample preview. Badge shows eight variants side by
side; Button shows eight variants, three sizes, and disabled.

That view is the point: eight *identical* badges in a row is unmissable, and is
exactly what nobody could see while stories were impossible (#14031) and
fixtures were dropped (#14459). It is the same `.stories.json` MosaicBook reads,
so the dev server and this site cannot disagree.

Story names become directory slugs, so an authored name containing spaces or
slashes cannot escape its output directory.

### Added — the component documentation site generator (#14026)

Walks `code/packages/mosaic/*/src`, asks `mosaic-compile --describe` for each
component's declared surface, emits each as a runnable html project, and writes
a landing page plus a page per component.

55 components across 21 packages, 40 of them with a live preview. The 15 that
cannot be emitted carry the compiler's own error on their page instead of
vanishing from the catalog.

Three flags had to be passed for components to render at all, each of which
otherwise reads as a broken component rather than a missing argument:
`--style` (with a dark fallback, matching MosaicBook), `--package-manifest`
(design tokens — without it `$foundation-color-text-light` is "not found in
token map"), and `--package-search-path` (components composing other packages).
