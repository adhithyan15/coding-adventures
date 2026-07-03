/**
 * # The default Excel/CAS adapter
 *
 * This is the `FormulaAdapter` the package ships with so the engine computes
 * real spreadsheet formulas out of the box. It is **separable**: the generic
 * engine core (`workbook.ts`) never imports it. You could delete this file and
 * the engine would still work with any other adapter.
 *
 * It composes four sibling packages:
 *
 *   - `@coding-adventures/excel-parser` — turns `"=A1+B2*3"` into a concrete
 *     syntax tree (CST). We walk that tree.
 *   - `@coding-adventures/symbolic-ir` — the symbolic-expression IR
 *     (`Add`/`Mul`/… nodes built with `app`, `int`, `numberNode`). We lower the
 *     arithmetic part of the CST into IR.
 *   - `@coding-adventures/cas-simplify` — `numericFold` collapses an exact
 *     integer/rational IR tree to a single number node.
 *   - (the engine's `resolve`) — supplies the current value of referenced cells.
 *
 * ## How the CST looks (verified against the real parser)
 *
 * The excel-parser produces a CST where rule nodes are `{ruleName, children}`
 * and tokens are `{type, value}`. Single-child "pass-through" rules collapse,
 * so the meaningful shapes are:
 *
 * ```text
 *   =42                → formula[ EQUALS, NUMBER("42") ]
 *   =A1                → formula[ EQUALS, CELL("A1") ]
 *   =$A$1              → formula[ EQUALS, CELL("$A$1") ]
 *   ="hi"              → formula[ EQUALS, STRING("hi") ]
 *   =TRUE              → formula[ EQUALS, KEYWORD("TRUE") ]
 *   =A1:A3             → formula[ EQUALS, range_reference[ CELL, COLON, CELL ] ]
 *   =A1+B2*3           → additive_expr[ CELL, PLUS, multiplicative_expr[ CELL, STAR, NUMBER ] ]
 *   =(A1+B2)*3         → multiplicative_expr[ parenthesized_expression[ LPAREN, …, RPAREN ], STAR, NUMBER ]
 *   =-A1               → unary_expr[ MINUS, CELL ]
 *   =A1%               → postfix_expr[ CELL, PERCENT ]
 *   =2^3               → power_expr[ NUMBER, CARET, NUMBER ]
 *   =A1&"x"            → concat_expr[ CELL, AMP, STRING ]
 *   =SUM(B1:B5)        → function_call[ FUNCTION_NAME("SUM"), LPAREN, range_reference, RPAREN ]
 *   =SUM(A1,B1,5)      → function_call[ FUNCTION_NAME, LPAREN, union_reference[ a, COMMA, b, … ], RPAREN ]
 * ```
 *
 * Binary-operator rules hold a **flat** child list: `[ lhs, OP, rhs, OP, rhs … ]`
 * which we fold left-associatively (right-associatively for `^`).
 *
 * Empty cells coerce to `0` in arithmetic — this is Excel's documented default
 * ("a blank cell behaves as zero"); see `cell-value.ts` `toNumber`.
 */

import { parseExcelFormula } from "@coding-adventures/excel-parser";
import type { ASTNode } from "@coding-adventures/parser";
import { isASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";
import { numericFold } from "@coding-adventures/cas-simplify";
import type { IRNode } from "@coding-adventures/symbolic-ir";
import {
  ADD,
  app,
  DIV,
  int,
  MUL,
  NEG,
  numberNode,
  POW,
  SUB,
} from "@coding-adventures/symbolic-ir";

import type { CellAddress } from "../address.js";
import { expandRange, parseA1, RangeTooLargeError } from "../address.js";
import type { CellResolver, FormulaAdapter } from "../adapter.js";
import type { CellValue } from "../cell-value.js";
import { bool, err, isError, num, text, toNumber, toText } from "../cell-value.js";

// A CST child is either a rule node or a raw token.
type CstChild = ASTNode | Token;

/** A thrown sentinel carrying a spreadsheet error code, used to unwind the
 *  recursive evaluator back to the top without littering every step with
 *  error-checking. Caught at the `evaluate` boundary and turned into a value. */
class FormulaError {
  constructor(public readonly value: CellValue) {}
}

function fail(code: Parameters<typeof err>[0]): never {
  throw new FormulaError(err(code));
}

// ---------------------------------------------------------------------------
// CST navigation helpers
// ---------------------------------------------------------------------------

function isToken(c: CstChild): c is Token {
  return !isASTNode(c);
}

/** A rule node's children with whitespace (`ws`) rules filtered out. */
function kids(node: ASTNode): CstChild[] {
  return node.children.filter((c) => !(isASTNode(c) && c.ruleName === "ws"));
}

/** Collapse pass-through rules: a rule node with exactly one meaningful child
 *  is "the same as" that child. Returns the deepest non-pass-through node. */
function unwrap(node: CstChild): CstChild {
  let cur = node;
  while (isASTNode(cur)) {
    const k = kids(cur);
    if (k.length === 1) {
      cur = k[0];
    } else {
      break;
    }
  }
  return cur;
}

// ---------------------------------------------------------------------------
// Reference extraction (for the dependency graph)
// ---------------------------------------------------------------------------

/** Pull every CELL token's address out of a CST, expanding ranges.
 *
 * The real parser wraps cell references in several layers
 * (`range_reference → reference_primary → a1_reference → CELL`). A
 * `range_reference` node containing a `COLON` is a true `A1:B3` range and is
 * expanded to its individual cells; a `range_reference` that is just a single
 * cell contributes that one cell. We descend the tree and treat any
 * `range_reference` as the unit of reference extraction. */
function collectRefs(node: CstChild, out: CellAddress[]): void {
  if (isToken(node)) return;

  if (node.ruleName === "range_reference") {
    pushRangeRefCells(node, out);
    return; // don't descend further; we've consumed the whole reference
  }

  for (const child of kids(node)) {
    if (isASTNode(child)) {
      collectRefs(child, out);
    } else if (child.type === "CELL") {
      // A bare CELL token not under a range_reference (defensive).
      out.push(parseCellToken(child.value));
    }
  }
}

/** Given a `range_reference` node, push every cell it covers into `out`.
 *  Handles both `A1` (single cell) and `A1:B3` (expanded).
 *
 *  An oversized range (`A1:ZZ1000000`) makes `expandRange` throw
 *  `RangeTooLargeError`; we let that propagate. The only caller in the
 *  dependency-scan path (`dependencies()`) catches it and registers no
 *  dependencies for the cell, and `evaluate` then surfaces a `#REF!` value — so
 *  we never materialize the giant array, here or anywhere. */
function pushRangeRefCells(rangeNode: ASTNode, out: CellAddress[]): void {
  const range = rangeReferenceToRange(rangeNode);
  if (range) for (const a of expandRange(range)) out.push(a);
}

/** Convert a `range_reference` CST node into a `CellRange`, or `undefined` if it
 *  doesn't contain cell endpoints (e.g. a stray numeric `row_reference`). */
function rangeReferenceToRange(rangeNode: ASTNode): { start: CellAddress; end: CellAddress } | undefined {
  const parts = kids(rangeNode);
  const colonIdx = parts.findIndex((c) => isToken(c) && c.type === "COLON");
  if (colonIdx === -1) {
    // A single cell wrapped in range_reference.
    const cell = firstCellToken(rangeNode);
    if (!cell) return undefined;
    const a = parseCellToken(cell);
    return { start: a, end: a };
  }
  // Two endpoints either side of the colon.
  const left = firstCellTokenIn(parts.slice(0, colonIdx));
  const right = firstCellTokenIn(parts.slice(colonIdx + 1));
  if (!left || !right) return undefined;
  return { start: parseCellToken(left), end: parseCellToken(right) };
}

/** Depth-first: the first CELL token's value found under `node`, or null. */
function firstCellToken(node: CstChild): string | null {
  if (isToken(node)) return node.type === "CELL" ? node.value : null;
  for (const c of kids(node)) {
    const found = firstCellToken(c);
    if (found) return found;
  }
  return null;
}

function firstCellTokenIn(nodes: CstChild[]): string | null {
  for (const n of nodes) {
    const found = firstCellToken(n);
    if (found) return found;
  }
  return null;
}

function parseCellToken(value: string): CellAddress {
  return parseA1(value);
}

// ---------------------------------------------------------------------------
// Evaluation: CST → CellValue
// ---------------------------------------------------------------------------

/** Evaluate a formula expression node to a CellValue. `node` is the expression
 *  that follows the leading `=`. */
function evalExpr(node: CstChild, resolve: CellResolver): CellValue {
  const n = unwrap(node);

  if (isToken(n)) {
    return evalToken(n, resolve);
  }

  switch (n.ruleName) {
    case "additive_expr":
    case "multiplicative_expr":
      return evalBinaryChain(n, resolve, /*rightAssoc*/ false);
    case "power_expr":
      return evalBinaryChain(n, resolve, /*rightAssoc*/ true);
    case "concat_expr":
      return evalConcat(n, resolve);
    case "comparison_expr":
      return evalComparison(n, resolve);
    case "unary_expr":
      return evalUnary(n, resolve);
    case "postfix_expr":
      return evalPostfix(n, resolve);
    case "parenthesized_expression": {
      // [ LPAREN, inner, RPAREN ] — evaluate the inner expression.
      const inner = kids(n).find((c) => isASTNode(c));
      return inner ? evalExpr(inner, resolve) : err("#VALUE!");
    }
    case "function_call":
      return evalFunctionCall(n, resolve);
    case "range_reference": {
      // In scalar context, a single-cell `range_reference` (e.g. just `A1`)
      // resolves to that cell. A true multi-cell range can't collapse to a
      // scalar — Excel would do implicit intersection; we surface #VALUE!.
      const range = rangeReferenceToRange(n);
      if (range && range.start.col === range.end.col && range.start.row === range.end.row) {
        return resolve(range.start);
      }
      // A multi-cell range can't collapse to a scalar. If it is also *oversized*
      // we still never expand it — we don't even reach `expandRange` here — so a
      // `#VALUE!` is the right answer and there is no allocation risk on this
      // path. (The allocation risk lives in the aggregation path below.)
      return err("#VALUE!");
    }
    default:
      // Unknown rule shape — try to recurse into a single child if present.
      const only = kids(n);
      if (only.length === 1) return evalExpr(only[0], resolve);
      return err("#VALUE!");
  }
}

/** A leaf token standing alone as an expression: a number, string, cell ref,
 *  or the TRUE/FALSE keyword. */
function evalToken(tok: Token, resolve: CellResolver): CellValue {
  switch (tok.type) {
    case "NUMBER":
      return num(Number(tok.value));
    case "STRING":
      return text(tok.value);
    case "CELL": {
      const v = resolve(parseCellToken(tok.value));
      return v;
    }
    case "KEYWORD": {
      const u = tok.value.toUpperCase();
      if (u === "TRUE") return bool(true);
      if (u === "FALSE") return bool(false);
      return err("#NAME?");
    }
    default:
      return err("#VALUE!");
  }
}

/** Evaluate `lhs OP rhs OP rhs …` for `+ - * /` (left-assoc) or `^`
 *  (right-assoc). We lower each operand to an IR node, assemble the IR tree,
 *  then fold it to a number. Division by zero is guarded *before* folding,
 *  because `numericFold` throws on a zero denominator. */
function evalBinaryChain(node: ASTNode, resolve: CellResolver, rightAssoc: boolean): CellValue {
  const parts = kids(node);
  // parts = [ operand, OP, operand, OP, operand, … ]
  const operands: IRNode[] = [];
  const ops: string[] = [];
  for (let i = 0; i < parts.length; i++) {
    const p = parts[i];
    if (isToken(p) && isBinaryOpToken(p.type)) {
      ops.push(p.type);
    } else {
      operands.push(toIR(evalExpr(p, resolve)));
    }
  }
  if (operands.length === 1) return irToValue(operands[0]);

  let ir: IRNode;
  if (rightAssoc) {
    // a ^ b ^ c = a ^ (b ^ c)
    ir = operands[operands.length - 1];
    for (let i = operands.length - 2; i >= 0; i--) {
      ir = applyOp(ops[i], operands[i], ir);
    }
  } else {
    ir = operands[0];
    for (let i = 0; i < ops.length; i++) {
      ir = applyOp(ops[i], ir, operands[i + 1]);
    }
  }
  return irToValue(ir);
}

function isBinaryOpToken(type: string): boolean {
  return (
    type === "PLUS" ||
    type === "MINUS" ||
    type === "STAR" ||
    type === "SLASH" ||
    type === "CARET"
  );
}

/** Build an IR apply-node for a binary operator, guarding division by zero. */
function applyOp(op: string, lhs: IRNode, rhs: IRNode): IRNode {
  switch (op) {
    case "PLUS":
      return app(ADD, [lhs, rhs]);
    case "MINUS":
      return app(SUB, [lhs, rhs]);
    case "STAR":
      return app(MUL, [lhs, rhs]);
    case "SLASH":
      if (isZeroIR(rhs)) fail("#DIV/0!");
      return app(DIV, [lhs, rhs]);
    case "CARET":
      return app(POW, [lhs, rhs]);
    default:
      return fail("#VALUE!");
  }
}

function isZeroIR(node: IRNode): boolean {
  if (node.kind === "integer") return node.value === 0n;
  if (node.kind === "float") return node.value === 0;
  if (node.kind === "rational") return node.numer === 0n;
  return false;
}

/** Coerce a CellValue to an IR numeric node for arithmetic. Errors unwind. */
function toIR(v: CellValue): IRNode {
  const n = toNumber(v);
  if (typeof n !== "number") throw new FormulaError(n);
  return numberToIR(n);
}

/** Represent a JS number as IR: exact integers become `int(…)` so cas-simplify
 *  can fold them exactly; everything else becomes a float node. */
function numberToIR(n: number): IRNode {
  if (Number.isInteger(n)) return int(BigInt(n));
  return numberNode(n);
}

/** Fold an IR tree to a CellValue number. `numericFold` collapses exact
 *  integer/rational arithmetic; for float-bearing trees it leaves an `apply`
 *  node, which we evaluate ourselves with `evalIRFloat`. */
function irToValue(ir: IRNode): CellValue {
  const folded = numericFold(ir);
  const direct = irToNumber(folded);
  if (direct !== undefined) return num(direct);
  // Float / mixed arithmetic that numericFold left symbolic: evaluate directly.
  const f = evalIRFloat(folded);
  return num(f);
}

/** Extract a JS number from a *concrete* IR numeric node (integer/rational/
 *  float), or `undefined` if it is still an `apply` (unevaluated) node. */
function irToNumber(node: IRNode): number | undefined {
  switch (node.kind) {
    case "integer":
      return Number(node.value);
    case "rational":
      return Number(node.numer) / Number(node.denom);
    case "float":
      return node.value;
    default:
      return undefined;
  }
}

/** Recursively evaluate any IR arithmetic tree to a JS float. Used for the
 *  float case `numericFold` declines to fold. Division by zero was already
 *  guarded at build time, so it cannot appear here. */
function evalIRFloat(node: IRNode): number {
  const concrete = irToNumber(node);
  if (concrete !== undefined) return concrete;
  if (node.kind !== "apply") fail("#VALUE!");
  const head = node.head.kind === "symbol" ? node.head.name : "";
  const a = node.args.map(evalIRFloat);
  switch (head) {
    case "Add":
      return a.reduce((x, y) => x + y, 0);
    case "Sub":
      return a.length === 1 ? -a[0] : a[0] - a[1];
    case "Mul":
      return a.reduce((x, y) => x * y, 1);
    case "Div":
      return a[0] / a[1];
    case "Pow":
      return Math.pow(a[0], a[1]);
    case "Neg":
      return -a[0];
    default:
      return fail("#VALUE!");
  }
}

/** `&` text concatenation: every operand coerced to text and joined. */
function evalConcat(node: ASTNode, resolve: CellResolver): CellValue {
  const parts = kids(node).filter((c) => !(isToken(c) && c.type === "AMP"));
  let out = "";
  for (const p of parts) {
    const v = evalExpr(p, resolve);
    if (isError(v)) return v;
    out += toText(v);
  }
  return text(out);
}

/** `= <> < > <= >=` comparison. Returns a boolean. Numbers compare numerically;
 *  otherwise we compare by text. */
function evalComparison(node: ASTNode, resolve: CellResolver): CellValue {
  const parts = kids(node);
  if (parts.length !== 3) {
    // chained comparisons aren't standard in Excel; evaluate the first child
    return parts.length ? evalExpr(parts[0], resolve) : err("#VALUE!");
  }
  const lhs = evalExpr(parts[0], resolve);
  const rhs = evalExpr(parts[2], resolve);
  if (isError(lhs)) return lhs;
  if (isError(rhs)) return rhs;
  // The operator sits in a `comparison_op` rule wrapping the actual token, so we
  // dig the token out rather than reading parts[1] directly.
  const opNode = parts[1];
  const op = isToken(opNode) ? opNode.type : (firstAnyToken(opNode)?.type ?? "");
  // Prefer numeric comparison when both sides are numbers; else textual.
  const ln = toNumber(lhs);
  const rn = toNumber(rhs);
  let cmp: number;
  if (typeof ln === "number" && typeof rn === "number") {
    cmp = ln < rn ? -1 : ln > rn ? 1 : 0;
  } else {
    const lt = toText(lhs);
    const rt = toText(rhs);
    cmp = lt < rt ? -1 : lt > rt ? 1 : 0;
  }
  // Operator token types verified against the real parser:
  //   EQUALS (=), NOT_EQUALS (<>), LESS_THAN (<), GREATER_THAN (>),
  //   LESS_EQUALS (<=), GREATER_EQUALS (>=).
  switch (op) {
    case "EQUALS":
      return bool(cmp === 0);
    case "NOT_EQUALS":
      return bool(cmp !== 0);
    case "LESS_THAN":
      return bool(cmp < 0);
    case "GREATER_THAN":
      return bool(cmp > 0);
    case "LESS_EQUALS":
      return bool(cmp <= 0);
    case "GREATER_EQUALS":
      return bool(cmp >= 0);
    default:
      return err("#VALUE!");
  }
}

/** Unary prefix `+`/`-`. The CST shape is `[ prefix_op*, operand ]` where each
 *  `prefix_op` is a rule wrapping a MINUS or PLUS token. We apply them
 *  inside-out (right to left). `+` is a no-op; `-` negates. */
function evalUnary(node: ASTNode, resolve: CellResolver): CellValue {
  const parts = kids(node);
  const operand = parts[parts.length - 1];
  let v = evalExpr(operand, resolve);
  for (let i = parts.length - 2; i >= 0; i--) {
    const opTok = firstTokenOfType(parts[i], "MINUS");
    if (opTok) {
      const ir = numericFold(app(NEG, [toIR(v)]));
      v = irToValue(ir);
    }
    // a leading PLUS is a no-op, so we don't need to handle it explicitly.
  }
  return v;
}

/** Depth-first search for the first token of `type` under `node` (or the node
 *  itself if it is that token). Returns the token or null. */
function firstTokenOfType(node: CstChild, type: string): Token | null {
  if (isToken(node)) return node.type === type ? node : null;
  for (const c of kids(node)) {
    const found = firstTokenOfType(c, type);
    if (found) return found;
  }
  return null;
}

/** Depth-first search for the first token of *any* type under `node`. */
function firstAnyToken(node: CstChild): Token | null {
  if (isToken(node)) return node;
  for (const c of kids(node)) {
    const found = firstAnyToken(c);
    if (found) return found;
  }
  return null;
}

/** Postfix `%` — divides by 100. */
function evalPostfix(node: ASTNode, resolve: CellResolver): CellValue {
  const parts = kids(node);
  let v = evalExpr(parts[0], resolve);
  for (let i = 1; i < parts.length; i++) {
    const t = parts[i];
    if (isToken(t) && t.type === "PERCENT") {
      const n = toNumber(v);
      if (typeof n !== "number") return n;
      v = num(n / 100);
    }
  }
  return v;
}

// ---------------------------------------------------------------------------
// Function calls
// ---------------------------------------------------------------------------

/** Evaluate `NAME(arg, arg, …)`. We resolve range/scalar arguments to a flat
 *  list of numbers and dispatch to the standard function library. */
function evalFunctionCall(node: ASTNode, resolve: CellResolver): CellValue {
  const children = kids(node);

  // The function name lives in a `function_name` rule wrapping a FUNCTION_NAME
  // token (verified against the real parser). Dig the token out.
  const nameNode = children.find(
    (c): c is ASTNode => isASTNode(c) && c.ruleName === "function_name",
  );
  const nameTok = nameNode
    ? (kids(nameNode).find((c) => isToken(c) && c.type === "FUNCTION_NAME") as Token | undefined)
    : children.find((c): c is Token => isToken(c) && c.type === "FUNCTION_NAME");
  if (!nameTok) return err("#NAME?");
  const name = nameTok.value.toUpperCase();

  const fn = FUNCTIONS[name];
  if (!fn) return err("#NAME?");

  // Arguments live in a `function_argument_list` rule between the parens.
  const argList = children.find(
    (c): c is ASTNode => isASTNode(c) && c.ruleName === "function_argument_list",
  );

  try {
    const numbers = collectArgNumbers(argList, resolve);
    return fn(numbers);
  } catch (e) {
    if (e instanceof FormulaError) return e.value;
    // An oversized range argument (`=SUM(A1:ZZ1000000)`) throws
    // `RangeTooLargeError` from `expandRange` *before* anything is allocated.
    // Translate it to a `#REF!` value so the formula degrades gracefully
    // instead of OOMing or letting the exception escape the engine.
    if (e instanceof RangeTooLargeError) return err("#REF!");
    throw e;
  }
}

/** Flatten a function's argument container into a list of numbers. Ranges
 *  contribute every cell's number (empty cells are skipped — Excel's SUM/AVERAGE
 *  ignore blanks). Scalar expressions contribute their single number. */
function collectArgNumbers(container: ASTNode | undefined, resolve: CellResolver): number[] {
  if (!container) return [];
  const out: number[] = [];
  collectArgNumbersInto(container, resolve, out);
  return out;
}

function collectArgNumbersInto(node: CstChild, resolve: CellResolver, out: number[]): void {
  const n = unwrap(node);
  if (isToken(n)) {
    pushScalar(evalToken(n, resolve), out, /*skipEmpty*/ false);
    return;
  }

  switch (n.ruleName) {
    // The list of `function_argument` nodes (commas, if present, sit between).
    case "function_argument_list":
    case "function_argument":
      for (const child of kids(n)) {
        if (isToken(child) && child.type === "COMMA") continue;
        collectArgNumbersInto(child, resolve, out);
      }
      return;

    // `SUM(A1,B1,5)` parses its comma-separated args as a single
    // `union_reference` holding COMMA-separated members. Each member is its own
    // argument.
    case "union_reference":
      for (const child of kids(n)) {
        if (isToken(child) && child.type === "COMMA") continue;
        collectArgNumbersInto(child, resolve, out);
      }
      return;

    // A `range_reference`: either a single cell or an expanded range. Empty
    // cells inside a range are skipped (blank ≠ zero for SUM/AVERAGE/COUNT).
    case "range_reference": {
      const range = rangeReferenceToRange(n);
      if (range) {
        const isSingle =
          range.start.col === range.end.col && range.start.row === range.end.row;
        for (const addr of expandRange(range)) {
          // For a single-cell argument we treat empty as 0 (it's an explicit
          // operand); for a multi-cell range, empty cells are skipped.
          pushScalar(resolve(addr), out, /*skipEmpty*/ !isSingle);
        }
        return;
      }
      break;
    }
  }

  // Otherwise it's a scalar expression argument (arithmetic, number, etc.).
  pushScalar(evalExpr(n, resolve), out, /*skipEmpty*/ false);
}

/** Push a single value's number into `out`. Empty cells inside ranges are
 *  skipped; an explicit non-numeric argument is a #VALUE!; errors unwind. */
function pushScalar(v: CellValue, out: number[], skipEmpty: boolean): void {
  if (v.kind === "empty") {
    if (!skipEmpty) out.push(0);
    return;
  }
  if (isError(v)) throw new FormulaError(v);
  const n = toNumber(v);
  if (typeof n !== "number") throw new FormulaError(n);
  out.push(n);
}

/** The standard function library: range/argument reducers. */
const FUNCTIONS: Record<string, (nums: number[]) => CellValue> = {
  SUM: (nums) => num(nums.reduce((a, b) => a + b, 0)),
  AVERAGE: (nums) =>
    nums.length === 0 ? err("#DIV/0!") : num(nums.reduce((a, b) => a + b, 0) / nums.length),
  MIN: (nums) => (nums.length === 0 ? num(0) : num(Math.min(...nums))),
  MAX: (nums) => (nums.length === 0 ? num(0) : num(Math.max(...nums))),
  COUNT: (nums) => num(nums.length),
  PRODUCT: (nums) => num(nums.reduce((a, b) => a * b, 1)),
};

// ---------------------------------------------------------------------------
// The adapter object
// ---------------------------------------------------------------------------

/** The shipped default adapter. Construct an engine with `{ adapter:
 *  excelCasAdapter }` (or use `createSpreadsheet()`). */
export const excelCasAdapter: FormulaAdapter = {
  isFormula(raw: string): boolean {
    return raw.startsWith("=");
  },

  dependencies(raw: string): CellAddress[] {
    try {
      const cst = parseExcelFormula(raw);
      const refs: CellAddress[] = [];
      collectRefs(cst, refs);
      return refs;
    } catch {
      // Two cases land here, both safe to treat as "no known dependencies":
      //   - the formula is unparseable, or
      //   - it names an oversized range (`A1:ZZ1000000`) and `expandRange` threw
      //     `RangeTooLargeError` *before* allocating the giant address array.
      // We register no edges and let `evaluate` report the actual error value.
      // Critically, this fires on `setCell` (before any eval), so a huge range
      // can never allocate a hundred-million-element array during dependency
      // scanning.
      return [];
    }
  },

  evaluate(raw: string, resolve: CellResolver): CellValue {
    let cst: ASTNode;
    try {
      cst = parseExcelFormula(raw);
    } catch {
      return err("#NAME?");
    }
    // The top node is `formula[ EQUALS, <expression> ]`. Find the expression.
    const top = kids(cst);
    const exprNode = top.find((c) => !(isToken(c) && c.type === "EQUALS"));
    if (!exprNode) return err("#VALUE!");
    try {
      return evalExpr(exprNode, resolve);
    } catch (e) {
      // The adapter contract (see `adapter.ts`) is that `evaluate` must *never*
      // throw for ordinary spreadsheet errors — it returns an error `CellValue`.
      // `FormulaError` is our own structured unwind and carries the right code.
      if (e instanceof FormulaError) return e.value;
      // Anything else is an *unexpected* throw on untrusted input. The most
      // important real case: a pathologically deep formula
      // (`=1+1+1+…` thousands of terms) overflows the recursive evaluator's
      // call stack with a `RangeError`. If we re-threw, that escapes `setCell`
      // and crashes the host. We instead degrade to `#VALUE!`, honouring the
      // "never throw" contract. (An oversized range surfaces as `#REF!` deeper
      // in, but if a `RangeTooLargeError` ever reached here it too becomes a
      // value rather than a throw.)
      if (e instanceof RangeTooLargeError) return err("#REF!");
      return err("#VALUE!");
    }
  },
};
