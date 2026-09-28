### Fixed — nested `collapse: auto` navigation splits are bounded (#15851)

A `HostNavigationSplit` with `collapse: auto` (the default) emits its children
twice: once for the regular-width `Row` and once for the compact `Drawer`.
Nesting therefore multiplied the output as 2^depth. The layout parser's
`MAX_RULE_DEPTH = 100` bounds recursion, not output: 30 nested splits parse in
about 60 lines and would ask for 2^30 copies of the innermost pane.

`TableCtx` now counts enclosing duplicating splits (`duplicating_split_depth`).
Past `MAX_DUPLICATING_SPLIT_DEPTH` = 6, which is 64 copies, emission fails with
a clear error that suggests `collapse: never` for inner splits. `never` splits
emit once and are not counted. Real apps nest none or one.

