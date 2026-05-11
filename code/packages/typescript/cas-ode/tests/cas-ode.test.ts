import { describe, expect, it } from "vitest";
import {
  ADD,
  COS,
  D,
  DIV,
  EQUAL,
  EXP,
  INTEGRATE,
  LOG,
  MUL,
  POW,
  SIN,
  SUB,
  app,
  equals,
  headName,
  int,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";
import { C, C1, C2, ODE2, buildOdeHandlerTable, ode2, solveOde } from "../src/index";

const x = sym("x");
const y = sym("y");
const yp = app(D, [y, x]);
const ypp = app(D, [yp, x]);

function expectEqual(actual: IRNode, expected: IRNode): void {
  expect(equals(actual, expected), `${display(actual)} !== ${display(expected)}`).toBe(true);
}

function display(node: IRNode): string {
  return JSON.stringify(node, (_key, value) => (typeof value === "bigint" ? value.toString() : value));
}

function hasHead(node: IRNode, name: string): boolean {
  return node.kind === "apply" && (headName(node.head) === name || node.args.some((arg) => hasHead(arg, name)));
}

function rhsOfEqual(node: IRNode): IRNode {
  expect(node.kind).toBe("apply");
  expect(node.kind === "apply" && equals(node.head, EQUAL)).toBe(true);
  return (node as Extract<IRNode, { kind: "apply" }>).args[1];
}

describe("cas-ode package shape", () => {
  it("exports an ODE2 handler table", () => {
    const table = buildOdeHandlerTable();
    expect(table.get("ODE2")).toBeTypeOf("function");
  });

  it("falls through as an unevaluated ODE2 node", () => {
    const unsupported = app(ADD, [yp, app(SIN, [app(MUL, [x, y])])]);
    expectEqual(ode2(unsupported, y, x), app(ODE2, [unsupported, y, x]));
  });
});

describe("first-order ODEs", () => {
  it("solves first-order linear equations with symbolic integrating factors", () => {
    const equation = app(SUB, [yp, app(MUL, [int(2), y])]);
    const result = solveOde(equation, y, x);
    expect(result).not.toBeNull();
    const rhs = rhsOfEqual(result!);
    expect(hasHead(rhs, EXP.name)).toBe(true);
    expect(hasHead(rhs, DIV.name)).toBe(true);
    expect(display(rhs)).toContain("%c");
  });

  it("returns implicit integrals for separable nonlinear equations", () => {
    const equation = app(SUB, [yp, app(MUL, [x, app(POW, [y, int(2)])])]);
    const result = solveOde(equation, y, x);
    expect(result).not.toBeNull();
    expect(result?.kind === "apply" && equals(result.head, EQUAL)).toBe(true);
    expect(display(result!)).toContain("%c");
  });

  it("solves Bernoulli equations through the linearized substitution", () => {
    const equation = app(ADD, [app(SUB, [yp, y]), app(POW, [y, int(2)])]);
    const result = solveOde(equation, y, x);
    expect(result).not.toBeNull();
    const rhs = rhsOfEqual(result!);
    expect(hasHead(rhs, POW.name)).toBe(true);
    expect(display(rhs)).toContain("%c");
  });

  it("solves exact equations as implicit potentials", () => {
    const M = app(ADD, [app(MUL, [int(2), x, y]), int(1)]);
    const N = app(POW, [x, int(2)]);
    const equation = app(ADD, [M, app(MUL, [N, yp])]);
    const result = solveOde(equation, y, x);
    const expectedPotential = app(ADD, [x, app(MUL, [y, app(POW, [x, int(2)])])]);
    expectEqual(result!, app(EQUAL, [expectedPotential, C]));
  });
});

describe("second-order ODEs", () => {
  it("solves constant-coefficient homogeneous equations", () => {
    const result = solveOde(app(ADD, [ypp, y]), y, x);
    const expected = app(EQUAL, [
      y,
      app(ADD, [
        app(MUL, [C1, app(COS, [x])]),
        app(MUL, [C2, app(SIN, [x])]),
      ]),
    ]);
    expectEqual(result!, expected);
  });

  it("solves polynomial-forced constant-coefficient equations", () => {
    const equation = app(SUB, [app(ADD, [ypp, y]), x]);
    const result = solveOde(equation, y, x);
    const rhs = rhsOfEqual(result!);
    expect(display(rhs)).toContain("%c1");
    expect(display(rhs)).toContain("%c2");
    expect(display(rhs)).toContain("\"name\":\"x\"");
  });

  it("uses variation of parameters with symbolic Integrate fallback", () => {
    const equation = app(SUB, [app(ADD, [ypp, y]), app(LOG, [x])]);
    const result = solveOde(equation, y, x);
    expect(result).not.toBeNull();
    expect(hasHead(result!, INTEGRATE.name)).toBe(true);
  });

  it("solves Euler-Cauchy equations", () => {
    const equation = app(SUB, [app(MUL, [app(POW, [x, int(2)]), ypp]), app(MUL, [int(2), y])]);
    const result = solveOde(equation, y, x);
    const rhs = rhsOfEqual(result!);
    expect(display(rhs)).toContain("%c1");
    expect(display(rhs)).toContain("%c2");
    expect(hasHead(rhs, POW.name)).toBe(true);
  });
});
