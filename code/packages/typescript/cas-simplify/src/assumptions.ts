import {
  EQUAL,
  GREATER,
  GREATER_EQUAL,
  IRNode,
  LESS,
  LESS_EQUAL,
  NOT_EQUAL,
  equals,
  headName,
  int,
  structuralKey,
} from "@coding-adventures/symbolic-ir";

const POSITIVE = "positive";
const NEGATIVE = "negative";
const ZERO = "zero";
const NONZERO = "nonzero";
const NONNEG = "nonneg";
const NONPOS = "nonpos";
const INTEGER = "integer";

const PROPERTY_MAP = new Map<string, string>([
  ["positive", POSITIVE],
  ["pos", POSITIVE],
  ["negative", NEGATIVE],
  ["neg", NEGATIVE],
  ["zero", ZERO],
  ["nonzero", NONZERO],
  ["nonneg", NONNEG],
  ["nonnegative", NONNEG],
  ["nonpos", NONPOS],
  ["nonpositive", NONPOS],
  ["integer", INTEGER],
  ["integerp", INTEGER],
]);

const ZERO_IR = int(0);

// Short operator strings used to canonicalise compound relations.  The
// six legal values mirror the Python ``_RELATION_HEAD_TO_OP`` map and
// give us a stable, hashable middle component for the
// ``(lhs, op, rhs)`` triple stored in ``_generalRelations``.
type RelationOp = ">" | "<" | ">=" | "<=" | "=" | "!=";

const RELATION_HEAD_TO_OP: ReadonlyMap<string, RelationOp> = new Map([
  [GREATER.name, ">"],
  [LESS.name, "<"],
  [GREATER_EQUAL.name, ">="],
  [LESS_EQUAL.name, "<="],
  [EQUAL.name, "="],
  [NOT_EQUAL.name, "!="],
]);

export class AssumptionContext {
  private readonly facts = new Map<string, Set<string>>();

  // Track G1 (TS port) — store compound relations as canonicalised
  // ``(lhs, op, rhs)`` triples keyed by a structural string.  IR nodes
  // are not natively hashable in JavaScript, but ``structuralKey``
  // produces a deterministic string that respects structural equality
  // (the same key Add / Mul canonicalisation already relies on).
  private readonly generalRelations = new Set<string>();

  assumeRelation(expr: IRNode): void {
    const parsed = parseRelation(expr);
    if (parsed === undefined) return;
    const { lhs, op, rhs } = parsed;
    const sym = symbolNameOf(lhs);
    // Plain-symbol-vs-zero path: fold into the per-symbol fact table.
    if (sym !== undefined && equals(rhs, ZERO_IR)) {
      if (op === ">") this.add(sym, POSITIVE);
      else if (op === "<") this.add(sym, NEGATIVE);
      else if (op === ">=") this.add(sym, NONNEG);
      else if (op === "<=") this.add(sym, NONPOS);
      else if (op === "=") this.add(sym, ZERO);
      else if (op === "!=") this.add(sym, NONZERO);
      return;
    }
    // Compound-relation path: canonicalise and stash.
    this.generalRelations.add(canonKey(lhs, op, rhs));
  }

  assumeProperty(symbol: IRNode, property: IRNode): void {
    const symbolName = symbolNameOf(symbol);
    const propertyName = symbolNameOf(property);
    if (symbolName === undefined || propertyName === undefined) return;
    const fact = PROPERTY_MAP.get(propertyName.toLowerCase());
    if (fact !== undefined) this.add(symbolName, fact);
  }

  forgetRelation(expr: IRNode): void {
    const parsed = parseRelation(expr);
    if (parsed === undefined) return;
    const { lhs, op, rhs } = parsed;
    const sym = symbolNameOf(lhs);
    if (sym !== undefined && equals(rhs, ZERO_IR)) {
      if (op === ">") this.remove(sym, POSITIVE);
      else if (op === "<") this.remove(sym, NEGATIVE);
      else if (op === ">=") this.remove(sym, NONNEG);
      else if (op === "<=") this.remove(sym, NONPOS);
      else if (op === "=") this.remove(sym, ZERO);
      else if (op === "!=") this.remove(sym, NONZERO);
      return;
    }
    this.generalRelations.delete(canonKey(lhs, op, rhs));
  }

  forgetAll(): void {
    this.facts.clear();
    this.generalRelations.clear();
  }

  isPositive(symbolName: string): boolean | undefined {
    const facts = this.facts.get(symbolName);
    if (facts?.has(POSITIVE)) return true;
    if (facts?.has(NEGATIVE) || facts?.has(ZERO)) return false;
    return undefined;
  }

  isNegative(symbolName: string): boolean | undefined {
    const facts = this.facts.get(symbolName);
    if (facts?.has(NEGATIVE)) return true;
    if (facts?.has(POSITIVE) || facts?.has(ZERO) || facts?.has(NONNEG)) return false;
    return undefined;
  }

  isNonneg(symbolName: string): boolean | undefined {
    const facts = this.facts.get(symbolName);
    if (facts?.has(NONNEG) || facts?.has(POSITIVE) || facts?.has(ZERO)) return true;
    if (facts?.has(NEGATIVE)) return false;
    return undefined;
  }

  isInteger(symbolName: string): boolean {
    return this.facts.get(symbolName)?.has(INTEGER) ?? false;
  }

  signOf(symbolName: string): 1 | -1 | 0 | undefined {
    const facts = this.facts.get(symbolName);
    if (facts?.has(POSITIVE)) return 1;
    if (facts?.has(NEGATIVE)) return -1;
    if (facts?.has(ZERO)) return 0;
    return undefined;
  }

  /**
   * Evaluate a relational IR node to ``true`` / ``false`` / ``undefined``.
   *
   * Two paths, tried in order:
   *
   *   1. Plain-symbol-vs-zero — folds against the per-symbol fact
   *      table (Phase 21 behaviour).  May return ``true`` or ``false``
   *      depending on what the user asserted; falls through on
   *      ``undefined`` so a verbatim compound assertion may still
   *      match.
   *   2. Compound-relation lookup — Track G1.  Structurally looks up
   *      ``_generalRelations`` for the canonicalised query.  Returns
   *      ``true`` on hit, otherwise ``undefined``.  No
   *      negative-knowledge inference: asserting ``a^2 > b^2`` does
   *      NOT make ``a^2 < b^2`` return ``false``.
   */
  isTrueRelation(expr: IRNode): boolean | undefined {
    const parsed = parseRelation(expr);
    if (parsed === undefined) return undefined;
    const { lhs, op, rhs } = parsed;
    const sym = symbolNameOf(lhs);

    if (sym !== undefined && equals(rhs, ZERO_IR)) {
      const plain = this.isTruePlain(sym, op);
      if (plain !== undefined) return plain;
      // Fall through to compound-relation lookup just in case the user
      // asserted the comparison verbatim with a non-symbol LHS that
      // canonicalises to the same triple.
    }

    return this.generalRelations.has(canonKey(lhs, op, rhs)) ? true : undefined;
  }

  private isTruePlain(symbolName: string, op: RelationOp): boolean | undefined {
    const facts = this.facts.get(symbolName);
    if (op === ">") return this.isPositive(symbolName);
    if (op === "<") return this.isNegative(symbolName);
    if (op === ">=") {
      if (facts?.has(POSITIVE) || facts?.has(ZERO) || facts?.has(NONNEG)) return true;
      if (facts?.has(NEGATIVE)) return false;
      return undefined;
    }
    if (op === "<=") {
      if (facts?.has(NEGATIVE) || facts?.has(ZERO) || facts?.has(NONPOS)) return true;
      if (facts?.has(POSITIVE)) return false;
      return undefined;
    }
    if (op === "=") {
      if (facts?.has(ZERO)) return true;
      if (facts?.has(POSITIVE) || facts?.has(NEGATIVE) || facts?.has(NONZERO)) return false;
      return undefined;
    }
    if (op === "!=") {
      if (facts?.has(NONZERO) || facts?.has(POSITIVE) || facts?.has(NEGATIVE)) return true;
      if (facts?.has(ZERO)) return false;
      return undefined;
    }
    return undefined;
  }

  hasAnyFacts(symbolName: string): boolean {
    return (this.facts.get(symbolName)?.size ?? 0) > 0;
  }

  factsFor(symbolName: string): readonly string[] {
    return [...(this.facts.get(symbolName) ?? [])].sort();
  }

  symbolsWithFacts(): readonly string[] {
    return [...this.facts.entries()]
      .filter(([, facts]) => facts.size > 0)
      .map(([name]) => name)
      .sort();
  }

  private add(symbolName: string, fact: string): void {
    const existing = this.facts.get(symbolName);
    if (existing === undefined) {
      this.facts.set(symbolName, new Set([fact]));
    } else {
      existing.add(fact);
    }
  }

  private remove(symbolName: string, fact: string): void {
    const existing = this.facts.get(symbolName);
    if (existing === undefined) return;
    existing.delete(fact);
    if (existing.size === 0) this.facts.delete(symbolName);
  }
}

interface ParsedRelation {
  readonly lhs: IRNode;
  readonly op: RelationOp;
  readonly rhs: IRNode;
}

function parseRelation(expr: IRNode): ParsedRelation | undefined {
  if (expr.kind !== "apply" || expr.args.length !== 2) return undefined;
  const op = RELATION_HEAD_TO_OP.get(headName(expr.head));
  if (op === undefined) return undefined;
  return { lhs: expr.args[0], op, rhs: expr.args[1] };
}

/**
 * Canonicalise a ``(lhs, op, rhs)`` relation and return a string key
 * suitable for set membership.  Rules mirror the Python ``_canon_relation``:
 *
 *   - ``a < b`` is stored as ``(b, >, a)`` — every strict inequality
 *     becomes ``>``.
 *   - ``a <= b`` becomes ``(b, >=, a)``.
 *   - ``a = b`` / ``a != b`` are commutative — order by structural key.
 *   - ``a > b`` and ``a >= b`` pass through verbatim.
 */
function canonKey(lhs: IRNode, op: RelationOp, rhs: IRNode): string {
  if (op === "<") return tripleKey(rhs, ">", lhs);
  if (op === "<=") return tripleKey(rhs, ">=", lhs);
  if (op === "=" || op === "!=") {
    const a = structuralKey(lhs);
    const b = structuralKey(rhs);
    return a <= b ? `${a}|${op}|${b}` : `${b}|${op}|${a}`;
  }
  return tripleKey(lhs, op, rhs);
}

function tripleKey(lhs: IRNode, op: RelationOp, rhs: IRNode): string {
  return `${structuralKey(lhs)}|${op}|${structuralKey(rhs)}`;
}

function symbolNameOf(node: IRNode): string | undefined {
  return node.kind === "symbol" ? node.name : undefined;
}
