### Fixed — four of VisiCalc's five Compose degradations were false positives (#14843)

`table-cell-role` was reported for every backend except React, on the claim that
"authored table-cell roles and wrapper geometry are currently implemented only
by React". Compose emits `collectionItemInfo` for every header, leading and body
cell of a table it recognises, so for those cells the claim is simply untrue.

VisiCalc, Compose: **5 degradations to 1**.

| code | before | after |
| --- | --- | --- |
| `accessibility.authored-table-cell-unimplemented` | 4 | **0** |
| `interaction.table-wheel-shift-unimplemented` | 1 | 1 |

The remaining one is **real and stays**: `grep onViewportShift` in
`mosaic-emit-compose` returns zero, so Compose does not implement wheel routing
at all. Measuring that first is what kept it from being "fixed" as well.

#### The question is about the table, not the cell

Cell semantics come from `compose_semantic_table_shape`, computed on the
enclosing `HostTable` — so a cell cannot be judged from the cell alone. The
degradation walker recursed over children without carrying it, which is why the
arm had to answer with a blanket backend check.

`collect_native_degradations` now threads the nearest enclosing `HostTable`, and
the `table-cell-role` arm asks the same kind of per-backend question the
`HostTable focusable` arm beside it already asked. A cell with no enclosing
table, or in a table Compose does not recognise, is still reported.

Grouping the walk's four invariant arguments into a `NativeScan` struct kept the
recursive function inside clippy's argument limit; the enclosing table was the
eighth.

#### Measured, not assumed

VisiCalc's own `.mll` contains **zero** `HostTable` nodes — it reaches its table
through `pkg::mosaic-pkg-grid::`, so a probe that skips the package resolver
sees nothing. Resolved, there is one table, `host_table_has_native_semantics`
is true for it, and it contains exactly the 4 `table-cell-role` props that were
being reported. The new test builds its fixture through the real resolver for
the same reason.

VisiCalc's render script pinned the count at 5; it now pins 1. Trestle and
Engram remain native-complete with 0.

