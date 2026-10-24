### Added C# document AST HTML renderer

- Added the missing pure C# `document-ast-to-html` package with complete TE00
  node rendering, raw HTML controls, dangerous URL rejection, bounded
  traversal/output, native coverage, and Unix/Windows build fronts.
- Split the larger C# CommonMark prerequisite into renderer, measured parser
  reconciliation, and facade owners after all 652 official examples exposed a
  233-pass/419-gap parser baseline. No facade or CommonMark conformance claim is
  included in this renderer tranche.
