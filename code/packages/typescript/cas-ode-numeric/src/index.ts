import type { IRNode } from "@coding-adventures/symbolic-ir";

export const RK4_SOLVE = "RK4Solve";

export interface Binding {
  readonly name: string;
  readonly value: number;
}

export interface Rk4Point {
  readonly t: number;
  readonly state: readonly number[];
}

export interface Rk4Options {
  readonly stateNames?: readonly string[];
  readonly tName?: string;
}

export type Rk4EvalFn = (node: IRNode, bindings: readonly Binding[]) => number;

export class Rk4ArgumentError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "Rk4ArgumentError";
  }
}

export class Rk4EvaluationError extends Error {
  constructor(readonly index: number, message: string, options?: ErrorOptions) {
    super(`RK4: failed to evaluate RHS component ${index}: ${message}`, options);
    this.name = "Rk4EvaluationError";
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

export function defaultStateNames(n: number): string[] {
  return Array.from({ length: n }, (_, index) => `y${index}`);
}

export function rk4Solve(
  fIr: readonly IRNode[],
  y0: readonly number[],
  tSpan: readonly [number, number],
  dt: number,
  evalFn: Rk4EvalFn,
  options: Rk4Options = {},
): Rk4Point[] {
  const n = fIr.length;
  if (y0.length !== n) {
    throw new Rk4ArgumentError(`f_ir has ${n} components but y0 has ${y0.length} entries.`);
  }
  if (dt <= 0) {
    throw new Rk4ArgumentError(`dt must be positive, got ${String(dt)}.`);
  }

  const stateNames = options.stateNames === undefined ? defaultStateNames(n) : [...options.stateNames];
  if (stateNames.length !== n) {
    throw new Rk4ArgumentError(`state_names has ${stateNames.length} entries but f_ir has ${n}.`);
  }
  const tName = options.tName ?? "t";

  const [tStart, tEnd] = tSpan;
  const trajectory: Rk4Point[] = [];
  let tCur = tStart;
  let yCur = [...y0];
  trajectory.push({ t: tCur, state: [...yCur] });

  while (tCur < tEnd - dt * 1e-10) {
    const h = Math.min(dt, tEnd - tCur);
    const k1 = evalRhs(fIr, tCur, yCur, stateNames, tName, evalFn);

    const yMid1 = yCur.map((value, index) => value + 0.5 * h * k1[index]);
    const k2 = evalRhs(fIr, tCur + 0.5 * h, yMid1, stateNames, tName, evalFn);

    const yMid2 = yCur.map((value, index) => value + 0.5 * h * k2[index]);
    const k3 = evalRhs(fIr, tCur + 0.5 * h, yMid2, stateNames, tName, evalFn);

    const yEndStage = yCur.map((value, index) => value + h * k3[index]);
    const k4 = evalRhs(fIr, tCur + h, yEndStage, stateNames, tName, evalFn);

    yCur = yCur.map(
      (value, index) => value + (h / 6) * (k1[index] + 2 * k2[index] + 2 * k3[index] + k4[index]),
    );
    tCur += h;
    trajectory.push({ t: tCur, state: [...yCur] });
  }

  return trajectory;
}

function evalRhs(
  fIr: readonly IRNode[],
  tValue: number,
  yValues: readonly number[],
  stateNames: readonly string[],
  tName: string,
  evalFn: Rk4EvalFn,
): number[] {
  const bindings: Binding[] = [
    { name: tName, value: tValue },
    ...stateNames.map((name, index) => ({ name, value: yValues[index] })),
  ];

  return fIr.map((node, index) => {
    try {
      const value = evalFn(node, bindings);
      if (typeof value !== "number") {
        throw new TypeError(`expected number, got ${typeof value}`);
      }
      return value;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      throw new Rk4EvaluationError(index, message, error instanceof Error ? { cause: error } : undefined);
    }
  });
}
