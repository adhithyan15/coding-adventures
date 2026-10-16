# XAML native table authored row names

For #14278 and epic #14267, generated native table cell automation names must
use the authored row-header Text content when the canonical table has row headers.
A viewport whose first visible header is 16 must name its first A cell
`A, row 16: value`, while its local GridItem row remains zero. Tables without
row headers retain the one-based local row fallback.

Lower literal, slot and expression labels through the existing compiler binding
machinery into the cell projection; retained projections must recompute names
when owner slots or captured row indices change. Keep explicit accessibility
names authoritative, and avoid visual-tree searches on each name request.
No VisiCalc-specific offsets or generated-output edits are permitted.

Validation: emitter regressions for literal labels, slot labels, offset/index
expressions and unnumbered fallback; package tests and generated WinUI build.
Native scroll/name inspection and keyboard acceptance remain separately pending
until computer control is available. This change does not establish release
readiness or resolve all of #14278.
