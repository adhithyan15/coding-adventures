---
category: BUILD files & dependency management
---

# After adding a field to a generated struct, re-audit every committed literal after each main merge

PR #18158 added `guard` to grammar-tools' `TokenDefinition` and `conditions`
to `ModeTransition`, and mechanically inserted the default fields into every
committed `_grammar*.rs`. While it was open, other PRs merged regenerated
grammar files made with the *old* generator: #18132 (JS 6a, 60 new token
definitions and 240 transitions), then #18178 (JS 6d) and #18145 (Python 7c,
new Python 2 pairs). Each `git merge origin/main` either auto-merged those
field-less literals in silently, which would have failed compilation with E0063
on the merged tree, or conflicted in the generated file.

**Fix:** after every main merge, resolve any generated-file conflict by taking
main's side and **regenerating with the branch's own grammar-tools**, never by
hand-picking hunks. Then audit: in every `_grammar*.rs`, count
`TokenDefinition {` against `guard:` and `ModeTransition {` against
`conditions:`. Both counts must match before you push. Regenerate without
rustfmt when the committed file is raw generator output, and leave files that
gained no new literals (e.g. a parser `_grammar.rs`) untouched, to avoid a
huge formatting-only diff.

**Do differently:** a PR that changes a generated struct is racing every other
PR that regenerates grammars. Keep it short-lived, and re-audit on each merge,
not just at creation time.
