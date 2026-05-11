import {
  ADD,
  DIV,
  EXP,
  INV,
  IRNode,
  MUL,
  NEG,
  POW,
  SUB,
  app,
  equals,
  headName,
  int,
  isOne,
  isZero,
  rational,
  structuralKey,
} from "@coding-adventures/symbolic-ir";

export { AssumptionContext } from "./assumptions";
export { IMAGINARY_UNIT, demoivre, exponentialize } from "./exponentialize";
export * from "./heads";
export { logcontract, logexpand } from "./logcontract";
export { radcan } from "./radcan";

export function canonical(node: IRNode): IRNode {
  if (node.kind !== "apply") return node;
  const head = canonical(node.head);
  let args = node.args.map(canonical);
  const name = headName(head);
  if (name === ADD.name || name === MUL.name) {
    args = flattenSameHead(name, args).sort((a, b) => sortKey(a).localeCompare(sortKey(b)));
    if (args.length === 0) return int(name === ADD.name ? 0 : 1);
    if (args.length === 1) return args[0];
  }
  return app(head, args);
}

export function numericFold(node: IRNode): IRNode {
  if (node.kind !== "apply") return node;
  const head = numericFold(node.head);
  const args = node.args.map(numericFold);
  const name = headName(head);
  if (name === NEG.name && args.length === 1 && args[0].kind === "integer") return int(-args[0].value);
  if (name === INV.name && args.length === 1 && args[0].kind === "integer") return rational(1, args[0].value);
  if (args.length === 2 && args[0].kind === "integer" && args[1].kind === "integer") {
    const [a, b] = [args[0].value, args[1].value];
    if (name === ADD.name) return int(a + b);
    if (name === SUB.name) return int(a - b);
    if (name === MUL.name) return int(a * b);
    if (name === DIV.name) return rational(a, b);
    if (name === POW.name && b >= 0n) return int(a ** b);
  }
  return app(head, args);
}

export function simplify(node: IRNode, maxIterations = 50): IRNode {
  let current = node;
  for (let i = 0; i < maxIterations; i += 1) {
    const next = simplifyOnce(numericFold(canonical(current)));
    if (equals(next, current)) return next;
    current = next;
  }
  return current;
}

function simplifyOnce(node: IRNode): IRNode {
  if (node.kind !== "apply") return node;
  const head = simplifyOnce(node.head);
  const args = node.args.map(simplifyOnce);
  const name = headName(head);

  if (name === ADD.name) return simplifyAdd(args);
  if (name === MUL.name) return simplifyMul(args);
  if (name === SUB.name && args.length === 2 && isZero(args[1])) return args[0];
  if (name === DIV.name && args.length === 2) {
    if (isZero(args[0])) return int(0);
    if (isOne(args[1])) return args[0];
  }
  if (name === POW.name && args.length === 2) {
    if (isZero(args[1])) return int(1);
    if (isOne(args[1])) return args[0];
    if (isOne(args[0])) return int(1);
  }
  if (name === NEG.name && args.length === 1) {
    const arg = args[0];
    if (arg.kind === "apply" && headName(arg.head) === NEG.name && arg.args.length === 1) return arg.args[0];
  }
  if (name === EXP.name && args.length === 1 && isZero(args[0])) return int(1);
  return app(head, args);
}

function simplifyAdd(args: readonly IRNode[]): IRNode {
  const kept = args.filter((arg) => !isZero(arg));
  if (kept.length === 0) return int(0);
  if (kept.length === 1) return kept[0];
  return app(ADD, kept);
}

function simplifyMul(args: readonly IRNode[]): IRNode {
  if (args.some(isZero)) return int(0);
  const kept = args.filter((arg) => !isOne(arg));
  if (kept.length === 0) return int(1);
  if (kept.length === 1) return kept[0];
  return app(MUL, kept);
}

function flattenSameHead(name: string, args: readonly IRNode[]): IRNode[] {
  const out: IRNode[] = [];
  for (const arg of args) {
    if (arg.kind === "apply" && headName(arg.head) === name) {
      out.push(...arg.args);
    } else {
      out.push(arg);
    }
  }
  return out;
}

function sortKey(node: IRNode): string {
  return `${rank(node)}:${structuralKey(node)}`;
}

function rank(node: IRNode): number {
  if (node.kind === "integer") return 0;
  if (node.kind === "rational") return 1;
  if (node.kind === "float") return 2;
  if (node.kind === "symbol") return 3;
  if (node.kind === "apply") return 4;
  return 5;
}
