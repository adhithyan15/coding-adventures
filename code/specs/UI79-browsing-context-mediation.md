# UI79 - Browsing Context Mediation

## Status

Accepted for Venture cross-host convergence.

## Goal

Link and form navigation policy belongs to the shared browser pipeline. Native
and web shells receive bounded effects; they do not reinterpret HTML target,
download, opener, or referrer policy.

## Shared Contract

The parser carries authored and effective link targets through the content and
render trees. The layout extension and paint hit regions retain that metadata
with `download`, `noopener`, and `noreferrer` state. Form navigation plans use
the parser's form policy plus the control model's authored submitter override
to carry the effective target alongside the complete GET or POST request.

Targets are normalized case-insensitively into `_self`, `_blank`, `_parent`,
`_top`, or a case-preserving named context. Venture currently has one retained
top-level context, so `_self`, `_parent`, and `_top` use the existing
transactional navigation path. `_blank` and named targets emit an
`open-browsing-context` effect without changing current history. Link downloads
emit a `download` effect and likewise leave the retained session untouched.

The open-context effect includes the normalized target, complete bounded
request metadata, and opener/referrer restrictions. `_blank` defaults to
`noopener` unless `rel=opener` is explicit; `noreferrer` also implies
`noopener`. Download effects include the
resolved request and optional authored filename. Hosts may choose windows,
tabs, or download UI, but may not change these shared decisions.

## Acceptance

- Base targets and submitter overrides survive parser, layout, paint, and form
  planning.
- Current-context navigation remains fetch-before-commit and rolls back on
  failure.
- Auxiliary and download activation do not fetch or mutate current history.
- Cairo, macOS, Windows, Qt, Flutter, Compose, SwiftUI, and XAML bridges expose
  the same JSON effects after page activation.
- Deterministic tests cover keyword normalization, inherited targets,
  `noreferrer`, suggested filenames, named form targets, and history isolation.
