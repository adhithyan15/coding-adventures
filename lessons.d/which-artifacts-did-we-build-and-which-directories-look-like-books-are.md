# "Which artifacts did we build?" and "which directories look like books?" are different questions

The same review caught a regression this PR introduced. Splitting compile from collection
left the collector re-deriving its work list with `find -type d -name book`, while the
compiler skipped any directory with no `book.tex`. Two enumerations, one of them
attacker-extensible: a PR adding only `code/learning/human-languages/anything/book/book.pdf`
gets that file uploaded as a build artifact and, on `main`, pushed to Pages and attached
to the Release — an attacker-authored PDF served as a book. Make it a symlink and `cp`
dereferences it, so the bytes of whatever it names go out instead.

**The rule: a producer that hands work downstream must say what it produced.** Add a
`--manifest` the compile appends to on success, and have the consumer read that file.
One writer, one reader, nothing to disagree about. Re-deriving a list is not a check on
the first derivation — it is a second, independent, silently-different answer.

Two corollaries, both bit here:
- Guard the OUTPUT path for a symlink, not only the input. `figures/*.svg` was vetted with
  `find -type f` and the derived `${svg%.svg}.pdf` was written unchecked; `book.pdf` had no
  guard at all even though XeLaTeX opens it for writing. `[ -L "$out" ]` before writing,
  and again before publishing, because the two steps should not have to trust each other.
- Deleting a duplicated step beats hardening it. The workflow's own SVG conversion existed
  only because the script's conversion was invisible from the YAML; the script's copy had
  the symlink guard and the workflow's did not.
