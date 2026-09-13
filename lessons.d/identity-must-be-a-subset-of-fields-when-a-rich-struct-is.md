# Identity must be a SUBSET of fields when a rich struct is a map/graph key (spreadsheet-core fill PR)

`CellAddress` carries `{row, col, absolute_row, absolute_col}` and derives
`Eq`/`Hash` over **all four** fields. But a cell's *identity* is its position
(`row, col`) only — the `$` markers just steer copy/fill shifting. The engine
stored cells (and dependency-graph nodes) keyed by the bare `CellAddress`, yet
the evaluator looked them up with the address taken **straight from the formula's
`Ref`**, flags and all. So `=$A$1` built a lookup key `{1,1,true,true}` that never
matched the relatively-stored `{1,1,false,false}` cell → it read as empty (**0**),
and editing `A1` never recomputed a dependent that referenced it absolutely.

Nobody had ever written a test that *evaluated* an absolute reference, so the bug
sat latent until the fill feature (whose whole point is "`$A$1` stays pinned")
surfaced it. Fix: a `without_absolute()` normaliser applied at the two key
boundaries (the evaluator's `lookup` closure and `collect_refs`).

Lesson: when a struct with "decorative" fields is used as a `HashMap` key or graph
node, either (a) don't derive `Hash`/`Eq` over the decorative fields, or (b)
normalise to the identity subset at **every** key boundary — and add a test that
exercises the decorated form through the real lookup path, not just the AST. A
derive that silently widens identity is a landmine.
