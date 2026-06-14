import { describe, expect, it } from "vitest";
import {
  ADD,
  MUL,
  NEG,
  POW,
  SUB,
  app,
  int,
  numberNode,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";
import {
  Rk4ArgumentError,
  Rk4EvaluationError,
  defaultStateNames,
  irToFloat,
  rk4Solve,
  type Binding,
  type Rk4Point,
} from "../src/index";

function evalNumeric(node: IRNode, bindings: readonly Binding[]): number {
  const out = evalNode(node, bindings);
  const value = irToFloat(out);
  if (value === undefined) throw new TypeError(`expected numeric IR node, got ${JSON.stringify(out)}`);
  return value;
}

function evalNode(node: IRNode, bindings: readonly Binding[]): IRNode {
  if (node.kind === "symbol") {
    const binding = bindings.find((entry) => entry.name === node.name);
    return binding === undefined ? node : numberNode(binding.value);
  }
  if (node.kind !== "apply") return node;

  const head = node.head;
  const args = node.args.map((arg) => evalNode(arg, bindings));
  if (head.kind !== "symbol") return app(head, args);

  const numericBinary = (op: (a: number, b: number) => number): IRNode | undefined => {
    if (args.length !== 2) return undefined;
    const a = irToFloat(args[0]);
    const b = irToFloat(args[1]);
    return a === undefined || b === undefined ? undefined : numberNode(op(a, b));
  };

  const out = (() => {
    switch (head.name) {
      case ADD.name:
        return numericBinary((a, b) => a + b);
      case SUB.name:
        return numericBinary((a, b) => a - b);
      case MUL.name:
        return numericBinary((a, b) => a * b);
      case POW.name:
        return numericBinary((a, b) => a ** b);
      case NEG.name: {
        if (args.length !== 1) return undefined;
        const value = irToFloat(args[0]);
        return value === undefined ? undefined : numberNode(-value);
      }
      default:
        return undefined;
    }
  })();

  return out ?? app(head, args);
}

function solve(
  fIr: readonly IRNode[],
  y0: readonly number[],
  tSpan: readonly [number, number],
  dt: number,
  stateNames: readonly string[],
): Rk4Point[] {
  return rk4Solve(fIr, y0, tSpan, dt, evalNumeric, { stateNames });
}

describe("irToFloat", () => {
  it("accepts numeric literals", () => {
    expect(irToFloat(int(3))).toBe(3);
    expect(irToFloat(numberNode(1.5))).toBe(1.5);
    expect(irToFloat(rational(1, 2))).toBe(0.5);
    expect(irToFloat(sym("x"))).toBeUndefined();
  });
});

describe("defaultStateNames", () => {
  it("matches the Python package defaults", () => {
    expect(defaultStateNames(4)).toEqual(["y0", "y1", "y2", "y3"]);
  });
});

describe("rk4Solve", () => {
  it("integrates scalar decay", () => {
    const y = sym("y");
    const f = app(MUL, [int(-2), y]);
    const traj = solve([f], [1], [0, 1], 0.001, ["y"]);
    const end = traj.at(-1);
    expect(end?.t).toBeCloseTo(1, 10);
    expect(end?.state[0]).toBeCloseTo(Math.exp(-2), 4);
  });

  it("records the initial condition and expected trajectory length", () => {
    const y = sym("y");
    const f = app(MUL, [int(-1), y]);
    const traj = solve([f], [2.5], [0, 1], 0.1, ["y"]);
    expect(traj).toHaveLength(11);
    expect(traj[0].t).toBe(0);
    expect(traj[0].state[0]).toBe(2.5);
  });

  it("keeps zero RHS constant", () => {
    const traj = solve([int(0)], [3.7], [0, 2], 0.5, ["y"]);
    expect(traj.every((point) => Math.abs(point.state[0] - 3.7) < 1e-12)).toBe(true);
  });

  it("integrates a coupled oscillator", () => {
    const y = sym("y");
    const v = sym("v");
    const traj = solve([v, app(NEG, [y])], [1, 0], [0, 2 * Math.PI], 0.001, ["y", "v"]);
    const state = traj.at(-1)?.state ?? [];
    expect(Math.abs((state[0] ?? Number.NaN) - 1)).toBeLessThan(0.01);
    expect(Math.abs(state[1] ?? Number.NaN)).toBeLessThan(0.01);
  });

  it("uses the time binding and clamps the last step", () => {
    const traj = rk4Solve([sym("time")], [0], [0, 1], 0.3, evalNumeric, {
      stateNames: ["y"],
      tName: "time",
    });
    const end = traj.at(-1);
    expect(traj).toHaveLength(5);
    expect(end?.t).toBeCloseTo(1, 12);
    expect(end?.state[0]).toBeCloseTo(0.5, 8);
  });

  it("gets more accurate with a smaller step", () => {
    const y = sym("y");
    const f = app(MUL, [int(-1), y]);
    const coarse = solve([f], [1], [0, 1], 0.1, ["y"]);
    const fine = solve([f], [1], [0, 1], 0.05, ["y"]);
    const exact = Math.exp(-1);
    const coarseErr = Math.abs((coarse.at(-1)?.state[0] ?? Number.NaN) - exact);
    const fineErr = Math.abs((fine.at(-1)?.state[0] ?? Number.NaN) - exact);
    expect(coarseErr).toBeGreaterThan(fineErr);
  });

  it("reports argument errors", () => {
    expect(() => rk4Solve([int(0)], [1], [0, 1], 0, evalNumeric)).toThrow(Rk4ArgumentError);
    expect(() => rk4Solve([int(0)], [1, 2], [0, 1], 0.1, evalNumeric)).toThrow("y0 has 2 entries");
    expect(() =>
      rk4Solve([int(0)], [1], [0, 1], 0.1, evalNumeric, { stateNames: ["a", "b"] }),
    ).toThrow("state_names has 2 entries");
  });

  it("reports non-numeric RHS evaluation", () => {
    expect(() => rk4Solve([sym("unbound")], [1], [0, 0.1], 0.05, evalNumeric, { stateNames: ["y"] }))
      .toThrow(Rk4EvaluationError);
  });

  it("simulates an underdamped RLC transient", () => {
    const q = sym("q");
    const i = sym("i");
    const halfI = app(MUL, [numberNode(0.5), i]);
    const diDt = app(SUB, [app(SUB, [int(1), halfI]), q]);
    const traj = solve([i, diDt], [0, 0], [0, 20], 0.01, ["q", "i"]);
    const qValues = traj.map((point) => point.state[0]);
    expect(Math.max(...qValues)).toBeLessThan(3);
    expect(Math.min(...qValues)).toBeGreaterThan(-1);
    expect(traj.at(-1)?.state[0]).toBeCloseTo(1, 1);
  });
});
