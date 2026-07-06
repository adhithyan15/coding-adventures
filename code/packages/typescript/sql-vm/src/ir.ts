// Re-export the IR types from sql-codegen so sql-vm doesn't need its own copy.
// sql-vm depends on sql-codegen, so this re-export keeps the type contract in one place.
export type { Instruction, Program, SortSpec, ColumnSpec, SqlValue } from "@coding-adventures/sql-codegen";
export { buildLabelIndex } from "@coding-adventures/sql-codegen";
