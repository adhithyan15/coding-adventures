### Add the F# Document AST HTML renderer

- Added the F# `document-ast-to-html` package with complete TE00 node rendering,
  raw-HTML suppression, HTML escaping, executable-URL rejection, and explicit
  depth, node-count, output-size, aggregate URL-work, and aggregate table
  alignment-metadata bounds.
- Added native F# conformance, security, determinism, and resource-limit tests
  with 95% enforced line coverage, cross-platform repository build fronts,
  NuGet metadata, documentation, and an empty capability declaration.
- Made the coverage fronts clean-checkout safe by building the referenced
  project graph before the no-build Coverlet test pass.
