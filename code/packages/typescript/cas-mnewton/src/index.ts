import { subst } from "@coding-adventures/cas-substitution";
import { equals, numberNode, sym, type IRApply, type IRNode } from "@coding-adventures/symbolic-ir";

export const MNEWTON = sym("MNewton");

export interface MNewtonOptions {
  readonly tol?: number;
  readonly maxIter?: number;
}

export type EvalFn = (node: IRNode) => IRNode;
export type DiffFn = (node: IRNode, variable: IRNode) => IRNode;
export type MNewtonHandler = (expr: IRNode, evalFn: EvalFn, diffFn: DiffFn) => IRNode;

export class MNewtonError extends Error {
  constructor(readonly x: number) {
    super(`Newton's method: derivative is zero at x = ${String(x)}`);
    this.name = "MNewtonError";
  }
}

export function irToFloat(node: IRNode): number | undefined {
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

export function mnewtonSolve(
  fIr: IRNode,
  xSym: IRNode,
  x0Ir: IRNode,
  evalFn: EvalFn,
  diffFn: DiffFn,
  options: MNewtonOptions = {},
): IRNode {
  const tol = options.tol ?? 1e-10;
  const maxIter = options.maxIter ?? 50;
  const fPrimeIr = evalFn(diffFn(fIr, xSym));
  let xN = irToFloat(x0Ir);
  if (xN === undefined) return fIr;

  for (let iteration = 0; iteration < maxIter; iteration += 1) {
    const xNIr = numberNode(xN);
    const fXNIr = evalFn(subst(xNIr, xSym, fIr));
    const fXN = irToFloat(fXNIr);
    if (fXN === undefined) return fIr;

    if (Math.abs(fXN) < tol) return numberNode(xN);

    const fPrimeXNIr = evalFn(subst(xNIr, xSym, fPrimeIr));
    const fPrimeXN = irToFloat(fPrimeXNIr);
    if (fPrimeXN === undefined) return fIr;

    if (Math.abs(fPrimeXN) < 1e-300) {
      throw new MNewtonError(xN);
    }

    xN -= fXN / fPrimeXN;
  }

  return numberNode(xN);
}

export function mnewtonHandler(expr: IRNode, evalFn: EvalFn, diffFn: DiffFn): IRNode {
  const args = applyArgs(expr, MNEWTON);
  if (args === undefined || (args.length !== 3 && args.length !== 4)) return expr;

  const [fIr, xSym, x0Ir, tolIr] = args;
  if (xSym.kind !== "symbol" || irToFloat(x0Ir) === undefined) return expr;

  let options: MNewtonOptions = {};
  if (tolIr !== undefined) {
    const tol = irToFloat(tolIr);
    if (tol === undefined) return expr;
    options = { tol };
  }

  try {
    return mnewtonSolve(fIr, xSym, x0Ir, evalFn, diffFn, options);
  } catch (error) {
    if (error instanceof MNewtonError) return expr;
    throw error;
  }
}

export function buildMNewtonHandlerTable(): ReadonlyMap<string, MNewtonHandler> {
  return new Map([[MNEWTON.name, mnewtonHandler]]);
}

function applyArgs(node: IRNode, head: IRNode): readonly IRNode[] | undefined {
  return isApplyOf(node, head) ? node.args : undefined;
}

function isApplyOf(node: IRNode, head: IRNode): node is IRApply {
  return node.kind === "apply" && equals(node.head, head);
}
