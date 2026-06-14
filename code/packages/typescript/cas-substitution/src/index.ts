import { IRApply, IRNode, RULE, app, equals, headName, sym } from "@coding-adventures/symbolic-ir";

export const BLANK = sym("Blank");
export const PATTERN = sym("Pattern");

export interface Rule {
  readonly pattern: IRNode;
  readonly replacement: IRNode;
}

export function subst(value: IRNode, variable: IRNode, expr: IRNode): IRNode {
  if (equals(expr, variable)) return value;
  if (expr.kind === "apply") {
    return app(subst(value, variable, expr.head), expr.args.map((arg) => subst(value, variable, arg)));
  }
  return expr;
}

export function substMany(rules: Iterable<readonly [IRNode, IRNode]>, expr: IRNode): IRNode {
  let out = expr;
  for (const [variable, value] of rules) {
    out = subst(value, variable, out);
  }
  return out;
}

export function replaceAll(expr: IRNode, rule: IRApply | Rule): IRNode {
  const normalized = normalizeRule(rule);
  const bindings = matchPattern(normalized.pattern, expr);
  if (bindings !== undefined) {
    return instantiate(normalized.replacement, bindings);
  }
  if (expr.kind === "apply") {
    return app(replaceAll(expr.head, normalized), expr.args.map((arg) => replaceAll(arg, normalized)));
  }
  return expr;
}

export function replaceAllMany(expr: IRNode, rules: Iterable<IRApply | Rule>): IRNode {
  let out = expr;
  for (const rule of rules) {
    out = replaceAll(out, rule);
  }
  return out;
}

export function rule(pattern: IRNode, replacement: IRNode): IRApply {
  return app(RULE, [pattern, replacement]);
}

export function blank(): IRApply {
  return app(BLANK, []);
}

export function pattern(name: string, inner: IRNode = blank()): IRApply {
  return app(PATTERN, [sym(name), inner]);
}

function normalizeRule(ruleLike: IRApply | Rule): Rule {
  if ("pattern" in ruleLike) return ruleLike;
  if (headName(ruleLike.head) !== RULE.name || ruleLike.args.length !== 2) {
    throw new TypeError("replaceAll expects Rule(pattern, replacement)");
  }
  return { pattern: ruleLike.args[0], replacement: ruleLike.args[1] };
}

function matchPattern(patternNode: IRNode, value: IRNode, bindings = new Map<string, IRNode>()): Map<string, IRNode> | undefined {
  if (patternNode.kind === "apply" && headName(patternNode.head) === BLANK.name && patternNode.args.length === 0) {
    return bindings;
  }
  if (patternNode.kind === "apply" && headName(patternNode.head) === PATTERN.name && patternNode.args.length >= 1) {
    const nameNode = patternNode.args[0];
    if (nameNode.kind !== "symbol") throw new TypeError("Pattern name must be a symbol");
    const inner = patternNode.args[1] ?? blank();
    const next = matchPattern(inner, value, bindings);
    if (next === undefined) return undefined;
    const previous = next.get(nameNode.name);
    if (previous !== undefined && !equals(previous, value)) return undefined;
    next.set(nameNode.name, value);
    return next;
  }
  if (patternNode.kind === "apply" && value.kind === "apply") {
    const headBindings = matchPattern(patternNode.head, value.head, new Map(bindings));
    if (headBindings === undefined || patternNode.args.length !== value.args.length) return undefined;
    let current = headBindings;
    for (let i = 0; i < patternNode.args.length; i += 1) {
      const next = matchPattern(patternNode.args[i], value.args[i], current);
      if (next === undefined) return undefined;
      current = next;
    }
    return current;
  }
  return equals(patternNode, value) ? bindings : undefined;
}

function instantiate(node: IRNode, bindings: ReadonlyMap<string, IRNode>): IRNode {
  if (node.kind === "apply" && headName(node.head) === PATTERN.name && node.args.length >= 1) {
    const name = node.args[0];
    if (name.kind === "symbol") {
      return bindings.get(name.name) ?? node;
    }
  }
  if (node.kind === "symbol") {
    return bindings.get(node.name) ?? node;
  }
  if (node.kind === "apply") {
    return app(instantiate(node.head, bindings), node.args.map((arg) => instantiate(arg, bindings)));
  }
  return node;
}
