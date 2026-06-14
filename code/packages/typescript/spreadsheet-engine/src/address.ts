/**
 * # Cell addresses and ranges — naming a place in the grid
 *
 * A spreadsheet is a grid, and every cell needs a name. The two universal
 * naming schemes are **A1 notation** (column letters + row number, e.g. `B7`)
 * and R1C1 (numeric both ways). We store everything internally as a pair of
 * zero-based integers and parse/print A1 on the boundary.
 *
 * ```text
 *        col 0   col 1   col 2          A1 string ↔ {col, row}
 *      ┌───────┬───────┬───────┐         A1   ↔ {col:0, row:0}
 * row0 │  A1   │  B1   │  C1   │         B1   ↔ {col:1, row:0}
 *      ├───────┼───────┼───────┤         A2   ↔ {col:0, row:1}
 * row1 │  A2   │  B2   │  C2   │         AA1  ↔ {col:26,row:0}
 *      └───────┴───────┴───────┘
 * ```
 *
 * ## The column-letter ↔ number bijection
 *
 * Column letters form a slightly unusual **bijective base-26** numbering
 * (sometimes called "spreadsheet base 26"). It is *not* plain base-26 because
 * there is no zero digit: the sequence is A, B, …, Z, AA, AB, …, AZ, BA, …
 * Note that after `Z` (25) comes `AA` (26), so `A` behaves like a leading 1,
 * never a leading 0. The two helpers below implement that bijection and are
 * exact inverses, so addresses round-trip through `parseA1`/`printA1` unchanged.
 */

/** A single cell location, plus optional `$` absolute-reference flags.
 *  The flags are display/fill-down metadata only — the recalc engine reads the
 *  resolved `col`/`row` and ignores absoluteness entirely (spec §3). */
export interface CellAddress {
  /** Zero-based column index. A=0, B=1, …, Z=25, AA=26, … */
  readonly col: number;
  /** Zero-based row index. Row "1" in A1 notation is index 0. */
  readonly row: number;
  /** `$` was present on the column (e.g. `$A1`). Optional; defaults false. */
  readonly absoluteCol?: boolean;
  /** `$` was present on the row (e.g. `A$1`). Optional; defaults false. */
  readonly absoluteRow?: boolean;
}

/** A rectangular block of cells, inclusive of both corners. */
export interface CellRange {
  readonly start: CellAddress;
  readonly end: CellAddress;
}

// ---------------------------------------------------------------------------
// Column letters ↔ column index
// ---------------------------------------------------------------------------

const A_CHARCODE = "A".charCodeAt(0);

/** Convert a zero-based column index to its letter string. 0→"A", 26→"AA". */
export function columnToLetters(col: number): string {
  if (col < 0 || !Number.isInteger(col)) {
    throw new RangeError(`column index must be a non-negative integer, got ${col}`);
  }
  // Bijective base-26: repeatedly take (n mod 26) as a digit, but subtract 1
  // first because there is no zero digit. `n = Math.floor((n-1)/26)` carries.
  let n = col + 1; // shift to 1-based for the bijective arithmetic
  let out = "";
  while (n > 0) {
    const rem = (n - 1) % 26;
    out = String.fromCharCode(A_CHARCODE + rem) + out;
    n = Math.floor((n - 1) / 26);
  }
  return out;
}

/** Convert a column letter string (case-insensitive) to a zero-based index.
 *  "A"→0, "Z"→25, "AA"→26. Throws on non-letters. */
export function lettersToColumn(letters: string): number {
  if (letters.length === 0) {
    throw new SyntaxError("empty column letters");
  }
  let n = 0;
  for (const ch of letters.toUpperCase()) {
    const code = ch.charCodeAt(0) - A_CHARCODE;
    if (code < 0 || code > 25) {
      throw new SyntaxError(`invalid column letter: ${ch}`);
    }
    n = n * 26 + (code + 1); // +1 keeps the bijection (A counts as 1, not 0)
  }
  return n - 1; // back to zero-based
}

// ---------------------------------------------------------------------------
// A1 string ↔ CellAddress
// ---------------------------------------------------------------------------

// Matches an optional $, run of letters, optional $, run of digits.
// Capturing groups: 1 = "$"?, 2 = letters, 3 = "$"?, 4 = digits.
const A1_RE = /^(\$?)([A-Za-z]+)(\$?)([0-9]+)$/;

/** Parse an A1-notation string (e.g. `"B7"`, `"$A$1"`) into a `CellAddress`. */
export function parseA1(a1: string): CellAddress {
  const m = A1_RE.exec(a1.trim());
  if (!m) {
    throw new SyntaxError(`not a valid A1 cell address: ${JSON.stringify(a1)}`);
  }
  const [, dollarCol, letters, dollarRow, digits] = m;
  const row = Number.parseInt(digits, 10) - 1; // A1's "1" is index 0
  if (row < 0) {
    throw new SyntaxError(`row number must be >= 1 in ${JSON.stringify(a1)}`);
  }
  return {
    col: lettersToColumn(letters),
    row,
    absoluteCol: dollarCol === "$",
    absoluteRow: dollarRow === "$",
  };
}

/** Print a `CellAddress` back to canonical A1 notation, including `$` flags. */
export function printA1(addr: CellAddress): string {
  const c = (addr.absoluteCol ? "$" : "") + columnToLetters(addr.col);
  const r = (addr.absoluteRow ? "$" : "") + String(addr.row + 1);
  return c + r;
}

/** Stable key for use in maps/graphs. We deliberately drop the `$` flags so
 *  that `A1` and `$A$1` map to the *same* cell — absoluteness never changes
 *  which physical cell is referenced, only how a formula fills down. */
export function addressKey(addr: CellAddress): string {
  return `${addr.col},${addr.row}`;
}

// ---------------------------------------------------------------------------
// Ranges
// ---------------------------------------------------------------------------

/** Parse a range string like `"A1:B3"` into a `CellRange`. A bare cell like
 *  `"A1"` yields a 1×1 range whose start and end are equal. */
export function parseRange(s: string): CellRange {
  const colon = s.indexOf(":");
  if (colon === -1) {
    const a = parseA1(s);
    return { start: a, end: a };
  }
  const start = parseA1(s.slice(0, colon));
  const end = parseA1(s.slice(colon + 1));
  return normalizeRange({ start, end });
}

/** Make sure `start` is the top-left and `end` the bottom-right, so callers can
 *  iterate without worrying about the order the user typed the corners in. */
export function normalizeRange(range: CellRange): CellRange {
  const minCol = Math.min(range.start.col, range.end.col);
  const maxCol = Math.max(range.start.col, range.end.col);
  const minRow = Math.min(range.start.row, range.end.row);
  const maxRow = Math.max(range.start.row, range.end.row);
  return {
    start: { col: minCol, row: minRow },
    end: { col: maxCol, row: maxRow },
  };
}

/**
 * The largest range we will ever *materialize* into individual cell objects.
 *
 * ## Why a cap exists at all
 *
 * Cell content is **untrusted host input**: a formula like `=SUM(A1:ZZ1000000)`
 * names a perfectly legal rectangle, but it covers ~702 × 1 000 000 ≈ 700
 * million cells. `expandRange` allocates one object per covered cell, so an
 * un-capped expansion would try to build a 700-million-element array and run the
 * host out of memory (an availability / denial-of-service hole) — all *before*
 * the formula is even evaluated, because `dependencies()` expands ranges on
 * `setCell`.
 *
 * The ceiling here is 2²⁰ = 1 048 576, the row count of a single Excel column.
 * That is comfortably larger than any *useful* hand-authored range yet small
 * enough that materializing it can never threaten the process. A range bigger
 * than this is treated as a structural error, not a thing to allocate.
 */
export const MAX_RANGE_CELLS = 1_048_576;

/**
 * Thrown by `expandRange` when a range is larger than {@link MAX_RANGE_CELLS}.
 *
 * It is a *typed, bounded* error: callers that touch untrusted ranges (the
 * dependency scan and the range-aggregation paths in the Excel adapter) catch it
 * and translate it into a `#REF!` cell value, so an oversized range degrades to
 * a normal spreadsheet error instead of either allocating the giant array or
 * letting an exception escape the engine.
 */
export class RangeTooLargeError extends Error {
  constructor(public readonly cellCount: number) {
    super(
      `range covers ${cellCount} cells, which exceeds the ${MAX_RANGE_CELLS}-cell ` +
        `safety cap (MAX_RANGE_CELLS)`,
    );
    this.name = "RangeTooLargeError";
  }
}

/** How many cells a (normalized) range covers, without materializing any of
 *  them. Used to enforce the cap *before* allocation. */
export function rangeCellCount(range: CellRange): number {
  const { start, end } = normalizeRange(range);
  const cols = end.col - start.col + 1;
  const rows = end.row - start.row + 1;
  return cols * rows;
}

/** Expand a range to the flat list of every `CellAddress` it covers, in
 *  row-major order. `A1:B2` → [A1, B1, A2, B2]. This is what the dependency
 *  graph consumes: a range reference becomes one edge per contained cell
 *  (spec §5, "range references stored as N separate edges").
 *
 *  Guarded by {@link MAX_RANGE_CELLS}: a range bigger than the cap throws
 *  {@link RangeTooLargeError} *before* allocating anything, rather than trying to
 *  build an array of hundreds of millions of objects and OOMing the host. The
 *  count is computed from the corners (cheap, O(1)); we never partially fill the
 *  array first. */
export function expandRange(range: CellRange): CellAddress[] {
  const { start, end } = normalizeRange(range);
  const count = (end.col - start.col + 1) * (end.row - start.row + 1);
  if (count > MAX_RANGE_CELLS) {
    throw new RangeTooLargeError(count);
  }
  const out: CellAddress[] = [];
  for (let row = start.row; row <= end.row; row++) {
    for (let col = start.col; col <= end.col; col++) {
      out.push({ col, row });
    }
  }
  return out;
}
