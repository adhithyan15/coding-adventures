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

export class AssumptionContext {
  private readonly facts = new Map<string, Set<string>>();

  assumeRelation(expr: IRNode): void {
    const parsed = relationAgainstZero(expr);
    if (parsed === undefined) return;
    const [symbolName, relation] = parsed;
    if (relation === GREATER.name) this.add(symbolName, POSITIVE);
    else if (relation === LESS.name) this.add(symbolName, NEGATIVE);
    else if (relation === GREATER_EQUAL.name) this.add(symbolName, NONNEG);
    else if (relation === LESS_EQUAL.name) this.add(symbolName, NONPOS);
    else if (relation === EQUAL.name) this.add(symbolName, ZERO);
    else if (relation === NOT_EQUAL.name) this.add(symbolName, NONZERO);
  }

  assumeProperty(symbol: IRNode, property: IRNode): void {
    const symbolName = symbolNameOf(symbol);
    const propertyName = symbolNameOf(property);
    if (symbolName === undefined || propertyName === undefined) return;
    const fact = PROPERTY_MAP.get(propertyName.toLowerCase());
    if (fact !== undefined) this.add(symbolName, fact);
  }

  forgetRelation(expr: IRNode): void {
    const parsed = relationAgainstZero(expr);
    if (parsed === undefined) return;
    const [symbolName, relation] = parsed;
    if (relation === GREATER.name) this.remove(symbolName, POSITIVE);
    else if (relation === LESS.name) this.remove(symbolName, NEGATIVE);
    else if (relation === GREATER_EQUAL.name) this.remove(symbolName, NONNEG);
    else if (relation === LESS_EQUAL.name) this.remove(symbolName, NONPOS);
    else if (relation === EQUAL.name) this.remove(symbolName, ZERO);
    else if (relation === NOT_EQUAL.name) this.remove(symbolName, NONZERO);
  }

  forgetAll(): void {
    this.facts.clear();
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

  isTrueRelation(expr: IRNode): boolean | undefined {
    const parsed = relationAgainstZero(expr);
    if (parsed === undefined) return undefined;
    const [symbolName, relation] = parsed;
    const facts = this.facts.get(symbolName);

    if (relation === GREATER.name) return this.isPositive(symbolName);
    if (relation === LESS.name) return this.isNegative(symbolName);
    if (relation === GREATER_EQUAL.name) {
      if (facts?.has(POSITIVE) || facts?.has(ZERO) || facts?.has(NONNEG)) return true;
      if (facts?.has(NEGATIVE)) return false;
      return undefined;
    }
    if (relation === LESS_EQUAL.name) {
      if (facts?.has(NEGATIVE) || facts?.has(ZERO) || facts?.has(NONPOS)) return true;
      if (facts?.has(POSITIVE)) return false;
      return undefined;
    }
    if (relation === EQUAL.name) {
      if (facts?.has(ZERO)) return true;
      if (facts?.has(POSITIVE) || facts?.has(NEGATIVE) || facts?.has(NONZERO)) return false;
      return undefined;
    }
    if (relation === NOT_EQUAL.name) {
      if (facts?.has(NONZERO) || facts?.has(POSITIVE) || facts?.has(NEGATIVE)) return true;
      if (facts?.has(ZERO)) return false;
      return undefined;
    }
    return undefined;
  }

  hasAnyFacts(symbolName: string): boolean {
    return (this.facts.get(symbolName)?.size ?? 0) > 0;
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

function relationAgainstZero(expr: IRNode): readonly [string, string] | undefined {
  if (expr.kind !== "apply" || expr.args.length !== 2 || !equals(expr.args[1], ZERO_IR)) {
    return undefined;
  }
  const symbolName = symbolNameOf(expr.args[0]);
  if (symbolName === undefined) return undefined;
  return [symbolName, headName(expr.head)];
}

function symbolNameOf(node: IRNode): string | undefined {
  return node.kind === "symbol" ? node.name : undefined;
}
