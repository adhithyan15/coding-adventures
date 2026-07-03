/**
 * # Cell Values — the data that lives in a spreadsheet cell
 *
 * A spreadsheet cell ultimately holds *one* of a small, fixed set of value
 * shapes. Excel calls these "value types"; we model them as a TypeScript
 * **discriminated union**. Every value carries a `kind` tag, and switching on
 * that tag tells the compiler (and the reader) exactly which other fields are
 * present.
 *
 * ```text
 *   empty     →  the cell is blank          (distinct from the text "")
 *   number    →  a numeric value            42, 3.14, -7
 *   text      →  a string                   "hello"
 *   boolean   →  TRUE / FALSE
 *   error     →  a propagating error code   #DIV/0!, #REF!, …
 * ```
 *
 * The `empty` case is load-bearing and easy to overlook: a blank cell is **not**
 * the same as a cell containing the empty string `""`. Excel keeps them
 * separate, and so do we. The difference shows up in coercion (see below) and in
 * functions like `COUNTBLANK`.
 *
 * Port note: the authoritative spec (`code/specs/spreadsheet-core.md` §1) is
 * written in Rust and lists nine Excel error variants plus `Array`/`Reference`.
 * For this v1 TypeScript core we ship the five value shapes and the six error
 * codes the task asks for; arrays and intermediate reference values are deferred
 * to a later pass (they are a recalc/spilling concern, not a model gap).
 */

/** The error codes this engine propagates. These are the on-the-wire strings
 *  Excel itself shows, so they round-trip into a UI unchanged. */
export type CellErrorCode =
  | "#DIV/0!" // division by zero
  | "#REF!" // a reference points at something that no longer exists
  | "#NAME?" // an unknown function or identifier was used
  | "#VALUE!" // a type mismatch (text where a number was needed, etc.)
  | "#CIRC!" // this cell takes part in a circular reference
  | "#NA"; // an explicit "not available" / unmatched lookup

/** The discriminated union of everything a cell can evaluate to. */
export type CellValue =
  | { kind: "empty" }
  | { kind: "number"; value: number }
  | { kind: "text"; value: string }
  | { kind: "boolean"; value: boolean }
  | { kind: "error"; code: CellErrorCode };

// ---------------------------------------------------------------------------
// Constructors — terse helpers so call sites read like the spec, not like JSON.
// ---------------------------------------------------------------------------

export const EMPTY: CellValue = { kind: "empty" };

export function num(value: number): CellValue {
  return { kind: "number", value };
}

export function text(value: string): CellValue {
  return { kind: "text", value };
}

export function bool(value: boolean): CellValue {
  return { kind: "boolean", value };
}

export function err(code: CellErrorCode): CellValue {
  return { kind: "error", code };
}

/** True when the value is one of the error codes — used to short-circuit
 *  arithmetic and to power `ISERROR`-style predicates in adapters. */
export function isError(v: CellValue): v is { kind: "error"; code: CellErrorCode } {
  return v.kind === "error";
}

// ---------------------------------------------------------------------------
// Coercions (spec §2)
//
// Excel quietly converts between types at the boundary of an operation. The
// table the spec gives us:
//
//   | context                       | empty cell becomes |
//   |-------------------------------|--------------------|
//   | arithmetic   (A1 + 5)         | 0                  |
//   | text concat  (A1 & "x")       | ""                 |
//   | logical                       | false              |
//
// These three functions are the single source of truth for that behaviour. An
// adapter that wants Excel-compatible semantics should route every coercion
// through them rather than re-deriving the rules.
// ---------------------------------------------------------------------------

/**
 * Coerce a value to a number for arithmetic.
 *
 *  - empty   → 0            (the famous "blank cell behaves as zero")
 *  - number  → itself
 *  - boolean → 1 / 0
 *  - text    → the parsed number if the whole string is numeric, else `#VALUE!`
 *  - error   → propagates unchanged (errors are sticky)
 *
 * Returns either a plain `number` (success) or a `CellValue` error to bubble up.
 */
export function toNumber(v: CellValue): number | { kind: "error"; code: CellErrorCode } {
  switch (v.kind) {
    case "empty":
      return 0;
    case "number":
      return v.value;
    case "boolean":
      return v.value ? 1 : 0;
    case "text": {
      // Excel only auto-coerces a string to a number when it is *entirely*
      // numeric (modulo surrounding whitespace). "12abc" is a #VALUE!, not 12.
      const trimmed = v.value.trim();
      if (trimmed === "") return { kind: "error", code: "#VALUE!" };
      const n = Number(trimmed);
      return Number.isNaN(n) ? { kind: "error", code: "#VALUE!" } : n;
    }
    case "error":
      return v;
  }
}

/** Coerce a value to a display/concatenation string.
 *  empty → "", boolean → "TRUE"/"FALSE", error → its code. */
export function toText(v: CellValue): string {
  switch (v.kind) {
    case "empty":
      return "";
    case "number":
      return String(v.value);
    case "text":
      return v.value;
    case "boolean":
      return v.value ? "TRUE" : "FALSE";
    case "error":
      return v.code;
  }
}

/** Coerce a value to a boolean for logical contexts.
 *  empty → false, number → (n !== 0), text → only "TRUE"/"FALSE" (else #VALUE!). */
export function toBoolean(v: CellValue): boolean | { kind: "error"; code: CellErrorCode } {
  switch (v.kind) {
    case "empty":
      return false;
    case "boolean":
      return v.value;
    case "number":
      return v.value !== 0;
    case "text": {
      const u = v.value.trim().toUpperCase();
      if (u === "TRUE") return true;
      if (u === "FALSE") return false;
      return { kind: "error", code: "#VALUE!" };
    }
    case "error":
      return v;
  }
}

/** Pretty-print a value for debugging / smoke tests. */
export function formatValue(v: CellValue): string {
  switch (v.kind) {
    case "empty":
      return "<empty>";
    case "number":
      return String(v.value);
    case "text":
      return JSON.stringify(v.value);
    case "boolean":
      return v.value ? "TRUE" : "FALSE";
    case "error":
      return v.code;
  }
}
