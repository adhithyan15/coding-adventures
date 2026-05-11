import { ADD, DIV, IRNode, LOG, MUL, POW, SUB, app, headName } from "@coding-adventures/symbolic-ir";
import { AssumptionContext } from "./assumptions";

export function logcontract(expr: IRNode): IRNode {
  if (expr.kind !== "apply") return expr;
  const args = expr.args.map(logcontract);
  const node = app(expr.head, args);
  const name = headName(node.head);
  if (name === ADD.name) return contractAdd(node);
  if (name === SUB.name) return contractSub(node);
  if (name === MUL.name) return contractMul(node);
  return node;
}

export function logexpand(expr: IRNode, _ctx?: AssumptionContext): IRNode {
  if (expr.kind !== "apply") return expr;
  const args = expr.args.map((arg) => logexpand(arg, _ctx));
  const node = app(expr.head, args);
  return headName(node.head) === LOG.name ? expandLog(node) : node;
}

function contractAdd(expr: Extract<IRNode, { kind: "apply" }>): IRNode {
  const logArgs: IRNode[] = [];
  const other: IRNode[] = [];
  for (const arg of expr.args) {
    if (isLog(arg)) logArgs.push(arg.args[0]);
    else other.push(arg);
  }
  if (logArgs.length < 2) return expr;
  const merged = app(LOG, [app(MUL, logArgs)]);
  return other.length === 0 ? merged : app(ADD, [...other, merged]);
}

function contractSub(expr: Extract<IRNode, { kind: "apply" }>): IRNode {
  if (expr.args.length !== 2) return expr;
  const [lhs, rhs] = expr.args;
  if (isLog(lhs) && isLog(rhs)) return app(LOG, [app(DIV, [lhs.args[0], rhs.args[0]])]);
  return expr;
}

function contractMul(expr: Extract<IRNode, { kind: "apply" }>): IRNode {
  const logs: Extract<IRNode, { kind: "apply" }>[] = [];
  const numerics: IRNode[] = [];
  const other: IRNode[] = [];
  for (const arg of expr.args) {
    if (isLog(arg)) logs.push(arg);
    else if (arg.kind === "integer" || arg.kind === "rational") numerics.push(arg);
    else other.push(arg);
  }
  if (logs.length !== 1 || numerics.length === 0 || other.length > 0) return expr;
  const coefficient = numerics.length === 1 ? numerics[0] : app(MUL, numerics);
  return app(LOG, [app(POW, [logs[0].args[0], coefficient])]);
}

function expandLog(expr: Extract<IRNode, { kind: "apply" }>): IRNode {
  if (expr.args.length !== 1) return expr;
  const arg = expr.args[0];

  if (isApplyHead(arg, POW.name) && arg.args.length === 2) {
    const [base, exp] = arg.args;
    if (exp.kind === "integer" || exp.kind === "rational") return app(MUL, [exp, app(LOG, [base])]);
  }

  if (isApplyHead(arg, MUL.name) && arg.args.length >= 2) {
    const terms = arg.args.map((factor) => app(LOG, [factor]));
    return terms.slice(1).reduce<IRNode>((acc, term) => app(ADD, [acc, term]), terms[0]);
  }

  if (isApplyHead(arg, DIV.name) && arg.args.length === 2) {
    return app(SUB, [app(LOG, [arg.args[0]]), app(LOG, [arg.args[1]])]);
  }

  return expr;
}

function isLog(node: IRNode): node is Extract<IRNode, { kind: "apply" }> {
  return isApplyHead(node, LOG.name) && node.args.length === 1;
}

function isApplyHead(node: IRNode, name: string): node is Extract<IRNode, { kind: "apply" }> {
  return node.kind === "apply" && headName(node.head) === name;
}
