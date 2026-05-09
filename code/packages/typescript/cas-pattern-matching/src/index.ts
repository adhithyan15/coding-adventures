import { app, equals, headName, sym, type IRApply, type IRNode } from "@coding-adventures/symbolic-ir";

export const BLANK = "Blank";
export const PATTERN = "Pattern";
export const RULE = "Rule";
export const RULE_DELAYED = "RuleDelayed";

export class Bindings {
  private readonly data: ReadonlyMap<string, IRNode>;

  private constructor(data: ReadonlyMap<string, IRNode>) {
    this.data = data;
  }

  static empty(): Bindings {
    return new Bindings(new Map());
  }

  bind(name: string, value: IRNode): Bindings {
    const existing = this.data.get(name);
    if (existing !== undefined && equals(existing, value)) {
      return this;
    }
    const next = new Map(this.data);
    next.set(name, value);
    return new Bindings(next);
  }

  get(name: string): IRNode | undefined {
    return this.data.get(name);
  }

  contains(name: string): boolean {
    return this.data.has(name);
  }

  get size(): number {
    return this.data.size;
  }

  get isEmpty(): boolean {
    return this.data.size === 0;
  }

  entries(): Array<[string, IRNode]> {
    return [...this.data.entries()];
  }

  equals(other: Bindings): boolean {
    if (this.size !== other.size) return false;
    for (const [name, value] of this.data) {
      const rhs = other.get(name);
      if (rhs === undefined || !equals(value, rhs)) return false;
    }
    return true;
  }
}

export interface RewriteCycleError {
  readonly kind: "rewrite-cycle";
  readonly maxIterations: number;
}

export function blank(): IRNode {
  return app(sym(BLANK), []);
}

export function blankTyped(head: string): IRNode {
  return app(sym(BLANK), [sym(head)]);
}

export function named(name: string, inner: IRNode): IRNode {
  return app(sym(PATTERN), [sym(name), inner]);
}

export function rule(lhs: IRNode, rhs: IRNode): IRNode {
  return app(sym(RULE), [lhs, rhs]);
}

export function ruleDelayed(lhs: IRNode, rhs: IRNode): IRNode {
  return app(sym(RULE_DELAYED), [lhs, rhs]);
}

export function isBlank(node: IRNode): boolean {
  return isHead(node, BLANK);
}

export function isPattern(node: IRNode): boolean {
  return isHead(node, PATTERN);
}

export function isRule(node: IRNode): boolean {
  return node.kind === "apply"
    && node.head.kind === "symbol"
    && (node.head.name === RULE || node.head.name === RULE_DELAYED)
    && node.args.length === 2;
}

export function matchPattern(pattern: IRNode, target: IRNode, bindings = Bindings.empty()): Bindings | null {
  if (isBlank(pattern)) {
    if (pattern.kind !== "apply") return null;
    const constraint = blankHeadConstraint(pattern);
    if (constraint === null) return bindings;
    return effectiveHeadName(target) === constraint ? bindings : null;
  }

  if (isPattern(pattern)) {
    if (pattern.kind !== "apply") return null;
    const name = patternName(pattern);
    const inner = patternInner(pattern);
    const matched = matchPattern(inner, target, bindings);
    if (matched === null) return null;
    const existing = matched.get(name);
    if (existing !== undefined) return equals(existing, target) ? matched : null;
    return matched.bind(name, target);
  }

  if (pattern.kind === "apply") {
    if (target.kind !== "apply") return null;
    return matchApply(pattern, target, bindings);
  }

  return equals(pattern, target) ? bindings : null;
}

export function applyRule(rewriteRule: IRNode, expr: IRNode): IRNode | null {
  if (!isRule(rewriteRule) || rewriteRule.kind !== "apply") {
    throw new TypeError(`applyRule expected Rule/RuleDelayed, got ${JSON.stringify(rewriteRule)}`);
  }
  const [lhs, rhs] = rewriteRule.args;
  const bindings = matchPattern(lhs, expr, Bindings.empty());
  return bindings === null ? null : substitute(rhs, bindings);
}

export function substitute(template: IRNode, bindings: Bindings): IRNode {
  if (isPattern(template) && template.kind === "apply") {
    const captured = bindings.get(patternName(template));
    return captured ?? template;
  }

  if (template.kind === "apply") {
    return app(
      substitute(template.head, bindings),
      template.args.map((arg) => substitute(arg, bindings)),
    );
  }

  return template;
}

export function rewrite(
  expr: IRNode,
  rules: readonly IRNode[],
  maxIterations = 100,
): IRNode | RewriteCycleError {
  let counter = 0;

  const walk = (node: IRNode): IRNode | RewriteCycleError => {
    let current = node;
    if (node.kind === "apply") {
      const newHead = walk(node.head);
      if (isRewriteCycleError(newHead)) return newHead;
      const newArgs: IRNode[] = [];
      for (const arg of node.args) {
        const nextArg = walk(arg);
        if (isRewriteCycleError(nextArg)) return nextArg;
        newArgs.push(nextArg);
      }
      current = app(newHead, newArgs);
    }

    while (true) {
      let fired = false;
      for (const candidateRule of rules) {
        const replacement = applyRule(candidateRule, current);
        if (replacement !== null && !equals(replacement, current)) {
          counter += 1;
          if (counter > maxIterations) {
            return { kind: "rewrite-cycle", maxIterations };
          }
          const walkedReplacement = walk(replacement);
          if (isRewriteCycleError(walkedReplacement)) return walkedReplacement;
          current = walkedReplacement;
          fired = true;
          break;
        }
      }
      if (!fired) return current;
    }
  };

  return walk(expr);
}

export function isRewriteCycleError(value: IRNode | RewriteCycleError): value is RewriteCycleError {
  return typeof value === "object" && value !== null && "kind" in value && value.kind === "rewrite-cycle";
}

function matchApply(pattern: IRApply, target: IRApply, bindings: Bindings): Bindings | null {
  let current = matchPattern(pattern.head, target.head, bindings);
  if (current === null) return null;
  if (pattern.args.length !== target.args.length) return null;

  for (let i = 0; i < pattern.args.length; i += 1) {
    current = matchPattern(pattern.args[i], target.args[i], current);
    if (current === null) return null;
  }
  return current;
}

function blankHeadConstraint(node: IRApply): string | null {
  if (node.args.length === 0) return null;
  const first = node.args[0];
  return first.kind === "symbol" ? first.name : null;
}

function patternName(node: IRApply): string {
  const first = node.args[0];
  if (first?.kind !== "symbol") {
    throw new TypeError("Pattern name must be a Symbol");
  }
  return first.name;
}

function patternInner(node: IRApply): IRNode {
  if (node.args.length < 2) {
    throw new TypeError("Pattern requires an inner expression");
  }
  return node.args[1];
}

function effectiveHeadName(node: IRNode): string {
  if (node.kind === "apply") return headName(node.head) || "Apply";
  if (node.kind === "integer") return "Integer";
  if (node.kind === "rational") return "Rational";
  if (node.kind === "float") return "Float";
  if (node.kind === "string") return "String";
  return "Symbol";
}

function isHead(node: IRNode, name: string): boolean {
  return node.kind === "apply" && node.head.kind === "symbol" && node.head.name === name;
}
