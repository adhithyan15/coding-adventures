# Changelog

## 0.1.0 — 2026-04-19

Initial release.

- Dispatch-loop VM `execute(program, backend)` returning a `QueryResult`
- Stack machine with separate row_buffer, cursors, and agg_table state
- Full arithmetic, logic, and comparison semantics with SQL three-valued
  NULL logic (AND/OR truth tables, NULL propagation through arithmetic
  and comparisons)
- Scan, AdvanceCursor, CloseScan — paired with label-driven loop exit
- BeginRow / EmitColumn / EmitRow for result assembly
- InitAgg / UpdateAgg / FinalizeAgg / SaveGroupKey / LoadGroupKey for
  GROUP BY and HAVING
- SortResult / LimitResult / DistinctResult post-processing
- DML: InsertRow, UpdateRows, DeleteRows
- DDL: CreateTable, DropTable
- Typed error hierarchy rooted at `VmError`
