# Changelog — math-layout

## Unreleased

### Added — TEX-1: MathExpr to a TeX math list

- `AtomClass` (the eight TeX classes), `Atom`, `MathList`, and `lower()` from
  `math_frontend::MathExpr`.
- TeX's two `Bin` demotion rules, which are what make the minus in `-x` a
  different atom from the minus in `a-x`. Applied to the list rather than
  during construction, so a sub-list can be spliced into a larger one without
  re-deriving it.
- The inter-atom spacing table, per style.
- A nesting cap. `MathExpr` is parsed from user-supplied LaTeX, so its depth is
  attacker-controlled, and lowering walks it recursively: 100k nested groups
  aborted the process with a stack overflow. In Engram that means a crafted
  flashcard crashes the app. `lower` now returns `Result` and refuses past
  `MAX_NESTING_DEPTH` — an error rather than a truncation, since quietly
  dropping the over-deep part would render a formula that is subtly not the one
  written.

### The table is TeX's, not ours

`code/scripts/extract_tex_spacing_table.py` asks a real `tex` to typeset all
256 class pairs in all four styles and reads back the glue it inserted. TeX
names the parameter it used (`\glue(\medmuskip)`), so nothing is derived from a
measured width or a threshold deciding what counts as "thin".

That caught a real error immediately. Script-style suppression is **per cell**,
not per size of space: `Op` then `Ord` keeps its thin space while `Inner` then
`Ord` loses the identical one. The obvious summary — "thin survives, medium and
thick do not" — is wrong in **30 of the 256** combinations, and every one
failed.

The fixture is generated from the arrangement `Ord L R Ord`, so TeX applies its
own demotion before choosing glue and the test reproduces that arrangement.
The demotion rules are therefore under test too, including the cells the
TeXbook marks impossible — those are reachable only through a demotion, and the
book gives no value there to copy.
