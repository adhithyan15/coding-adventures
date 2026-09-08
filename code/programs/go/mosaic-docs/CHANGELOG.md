# Changelog — mosaic-docs

## [Unreleased]

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
