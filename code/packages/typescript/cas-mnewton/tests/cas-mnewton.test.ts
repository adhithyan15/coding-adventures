import { describe, expect, it } from "vitest";
import {
  ADD,
  COS,
  MUL,
  POW,
  SIN,
  SUB,
  app,
  equals,
  int,
  numberNode,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";
import {
  MNEWTON,
  MNewtonError,
  buildMNewtonHandlerTable,
  irToFloat,
  mnewtonHandler,
  mnewtonSolve,
} from "../src/index";

function evalNode(node: IRNode): IRNode {
  if (node.kind !== "apply") return node;
  const head = node.head;
  const args = node.args.map(evalNode);
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
      case SIN.name: {
        if (args.length !== 1) return undefined;
        const value = irToFloat(args[0]);
        return value === undefined ? undefined : numberNode(Math.sin(value));
      }
      case COS.name: {
        if (args.length !== 1) return undefined;
        const value = irToFloat(args[0]);
        return value === undefined ? undefined : numberNode(Math.cos(value));
      }
      default:
        return undefined;
    }
  })();

  return out ?? app(head, args);
}

function diff(node: IRNode, variable: IRNode): IRNode {
  if (equals(node, variable)) return int(1);
  switch (node.kind) {
    case "integer":
    case "rational":
    case "float":
    case "string":
    case "symbol":
      return int(0);
    case "apply": {
      if (node.head.kind !== "symbol") return int(0);
      const [a, b] = node.args;
      switch (node.head.name) {
        case ADD.name:
          return app(ADD, [diff(a, variable), diff(b, variable)]);
        case SUB.name:
          return app(SUB, [diff(a, variable), diff(b, variable)]);
        case MUL.name:
          return app(ADD, [
            app(MUL, [diff(a, variable), b]),
            app(MUL, [a, diff(b, variable)]),
          ]);
        case POW.name:
          if (b?.kind !== "integer" || b.value < 1n) return int(0);
          return app(MUL, [
            int(b.value),
            app(MUL, [
              b.value === 1n ? int(1) : app(POW, [a, int(b.value - 1n)]),
              diff(a, variable),
            ]),
          ]);
        case SIN.name:
          return app(MUL, [app(COS, [a]), diff(a, variable)]);
        default:
          return int(0);
      }
    }
  }
}

function solve(f: IRNode, x0: IRNode): IRNode {
  const x = sym("x");
  return mnewtonSolve(f, x, x0, evalNode, diff);
}

function expectClose(node: IRNode, expected: number, tol: number): void {
  const actual = irToFloat(node);
  expect(actual).toBeDefined();
  expect(Math.abs((actual ?? Number.NaN) - expected)).toBeLessThan(tol);
}

describe("irToFloat", () => {
  it("accepts numeric literals", () => {
    expect(irToFloat(int(3))).toBe(3);
    expect(irToFloat(numberNode(1.5))).toBe(1.5);
    expect(irToFloat(rational(1, 2))).toBe(0.5);
    expect(irToFloat(sym("x"))).toBeUndefined();
    expect(irToFloat(app(ADD, [int(1), int(2)]))).toBeUndefined();
  });
});

describe("mnewtonSolve", () => {
  it("solves linear functions", () => {
    const x = sym("x");
    expectClose(solve(app(SUB, [x, int(2)]), numberNode(0)), 2, 1e-9);
    expectClose(solve(app(SUB, [x, int(7)]), int(0)), 7, 1e-9);
  });

  it("solves quadratic roots from the starting side", () => {
    const x = sym("x");
    const f = app(SUB, [app(POW, [x, int(2)]), int(4)]);
    expectClose(solve(f, numberNode(3)), 2, 1e-9);
    expectClose(solve(f, numberNode(-3)), -2, 1e-9);
  });

  it("solves sqrt two and cubic roots", () => {
    const x = sym("x");
    expectClose(solve(app(SUB, [app(POW, [x, int(2)]), int(2)]), numberNode(1.5)), Math.sqrt(2), 1e-8);
    expectClose(solve(app(SUB, [app(POW, [x, int(3)]), int(8)]), numberNode(1)), 2, 1e-8);
  });

  it("accepts rational initial guesses", () => {
    const x = sym("x");
    expectClose(solve(app(SUB, [x, int(2)]), rational(3, 2)), 2, 1e-9);
  });

  it("returns the initial guess when already at a root", () => {
    const x = sym("x");
    expect(equals(solve(app(SUB, [x, int(5)]), numberNode(5)), numberNode(5))).toBe(true);
  });

  it("returns the original expression for symbolic input or non-numeric evaluation", () => {
    const x = sym("x");
    const y = sym("y");
    const f = app(SUB, [x, int(2)]);
    expect(equals(mnewtonSolve(f, x, sym("a"), evalNode, diff), f)).toBe(true);

    const extraSymbol = app(ADD, [x, y]);
    expect(equals(mnewtonSolve(extraSymbol, x, numberNode(1), evalNode, diff), extraSymbol)).toBe(true);
  });

  it("reports zero derivatives before taking a Newton step", () => {
    const x = sym("x");
    const f = app(SUB, [app(POW, [x, int(2)]), int(1)]);
    expect(() => mnewtonSolve(f, x, numberNode(0), evalNode, diff)).toThrow(MNewtonError);
  });

  it("honors custom tolerance and max iterations", () => {
    const x = sym("x");
    const f = app(SUB, [app(POW, [x, int(2)]), int(2)]);
    expectClose(mnewtonSolve(f, x, numberNode(1.5), evalNode, diff, { tol: 1e-4, maxIter: 50 }), Math.sqrt(2), 1e-3);
  });

  it("solves a sine root near pi", () => {
    const x = sym("x");
    expectClose(solve(app(SIN, [x]), numberNode(3)), Math.PI, 1e-8);
  });
});

describe("mnewtonHandler", () => {
  it("exports a handler table keyed by MNEWTON", () => {
    const table = buildMNewtonHandlerTable();
    expect(table.get(MNEWTON.name)).toBe(mnewtonHandler);
    expect([...table.keys()]).toEqual([MNEWTON.name]);
  });

  it("solves MNewton(f, x, x0) expressions", () => {
    const x = sym("x");
    const expr = app(MNEWTON, [app(SUB, [app(POW, [x, int(2)]), int(2)]), x, numberNode(1.5)]);
    expectClose(mnewtonHandler(expr, evalNode, diff), Math.sqrt(2), 1e-8);
  });

  it("accepts an optional numeric tolerance", () => {
    const x = sym("x");
    const expr = app(MNEWTON, [app(SUB, [app(POW, [x, int(2)]), int(2)]), x, numberNode(1.5), rational(1, 1000)]);
    expectClose(mnewtonHandler(expr, evalNode, diff), Math.sqrt(2), 1e-3);
  });

  it("returns the expression unchanged for malformed calls", () => {
    const x = sym("x");
    const f = app(SUB, [x, int(2)]);
    const wrongHead = app(SUB, [f, x, int(0)]);
    const wrongArity = app(MNEWTON, [f, x]);
    const nonSymbolVariable = app(MNEWTON, [f, app(ADD, [x, int(1)]), int(0)]);
    const symbolicX0 = app(MNEWTON, [f, x, sym("a")]);
    const symbolicTol = app(MNEWTON, [f, x, int(0), sym("tol")]);

    for (const expr of [wrongHead, wrongArity, nonSymbolVariable, symbolicX0, symbolicTol]) {
      expect(equals(mnewtonHandler(expr, evalNode, diff), expr)).toBe(true);
    }
  });

  it("returns the expression unchanged when Newton hits a zero derivative", () => {
    const x = sym("x");
    const expr = app(MNEWTON, [app(SUB, [app(POW, [x, int(2)]), int(1)]), x, int(0)]);
    expect(equals(mnewtonHandler(expr, evalNode, diff), expr)).toBe(true);
  });
});
