import { describe, expect, it } from "vitest";
import {
  ACOS,
  ACOSH,
  ADD,
  ASIN,
  ASINH,
  ASSIGN,
  ATAN,
  ATANH,
  COS,
  COSH,
  COTH,
  CSCH,
  D,
  DEFINE,
  DIV,
  EQUAL,
  EXP,
  FACTOR,
  FALSE,
  IF,
  INTEGRATE,
  LIST,
  LOG,
  MUL,
  NEG,
  POW,
  SECH,
  SIN,
  SINH,
  SQRT,
  SUB,
  TAN,
  TANH,
  TRUE,
  app,
  equals,
  headName,
  int,
  numberNode,
  rational,
  sym,
} from "@coding-adventures/symbolic-ir";
import { ArityError, StrictBackend, StrictEvaluationError, SymbolicBackend, VM } from "../src/index.js";

describe("symbolic-vm", () => {
  it("strict backend folds numeric arithmetic exactly", () => {
    const vm = new VM(new StrictBackend());
    const expr = app(ADD, [rational(1, 2), rational(1, 3)]);
    expect(vm.eval(expr)).toEqual(rational(5, 6));
  });

  it("strict backend rejects unbound symbols", () => {
    const vm = new VM(new StrictBackend());
    expect(() => vm.eval(sym("x"))).toThrow(StrictEvaluationError);
  });

  it("symbolic backend leaves free symbols unresolved", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(sym("x"))).toEqual(sym("x"));
  });

  it("symbolic backend folds identity and zero laws", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(ADD, [sym("x"), int(0)]))).toEqual(sym("x"));
    expect(vm.eval(app(MUL, [sym("x"), int(0)]))).toEqual(int(0));
    expect(vm.eval(app(POW, [sym("x"), int(1)]))).toEqual(sym("x"));
  });

  it("leaves unknown symbolic heads unevaluated", () => {
    const vm = new VM(new SymbolicBackend());
    const expr = app(sym("Mystery"), [app(ADD, [int(1), int(2)])]);
    const result = vm.eval(expr);
    expect(result).toEqual(app(sym("Mystery"), [int(3)]));
  });

  it("factors univariate integer polynomials in the symbolic backend", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const expr = app(FACTOR, [app(SUB, [app(POW, [x, int(2)]), int(1)])]);

    expect(vm.eval(expr)).toEqual(app(MUL, [
      app(ADD, [int(1), x]),
      app(ADD, [int(-1), x]),
    ]));
  });

  it("extracts common multivariate factors before univariate factoring", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const y = sym("y");
    const expr = app(FACTOR, [
      app(SUB, [
        app(MUL, [app(POW, [x, int(2)]), y]),
        y,
      ]),
    ]);

    expect(vm.eval(expr)).toEqual(app(MUL, [
      y,
      app(MUL, [
        app(ADD, [int(1), x]),
        app(ADD, [int(-1), x]),
      ]),
    ]));
  });

  it("extracts common integer content from multivariate factors", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const y = sym("y");
    const z = sym("z");
    const expr = app(FACTOR, [
      app(ADD, [
        app(MUL, [int(2), app(MUL, [x, y])]),
        app(MUL, [int(2), app(MUL, [x, z])]),
      ]),
    ]);

    expect(vm.eval(expr)).toEqual(app(MUL, [
      app(MUL, [int(2), x]),
      app(ADD, [y, z]),
    ]));
  });

  it("extracts negative common integer content from multivariate factors", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const y = sym("y");
    const z = sym("z");
    const expr = app(FACTOR, [
      app(ADD, [
        app(MUL, [int(-2), app(MUL, [x, y])]),
        app(MUL, [int(-2), app(MUL, [x, z])]),
      ]),
    ]);

    expect(vm.eval(expr)).toEqual(app(MUL, [
      app(MUL, [int(-2), x]),
      app(ADD, [y, z]),
    ]));
  });

  it("factors bivariate perfect squares", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const y = sym("y");
    const expr = app(FACTOR, [
      app(ADD, [
        app(ADD, [
          app(POW, [x, int(2)]),
          app(MUL, [int(2), app(MUL, [x, y])]),
        ]),
        app(POW, [y, int(2)]),
      ]),
    ]);

    expect(vm.eval(expr)).toEqual(app(POW, [
      app(ADD, [x, y]),
      int(2),
    ]));
  });

  it("factors bivariate differences of squares", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const y = sym("y");
    const expr = app(FACTOR, [
      app(SUB, [
        app(POW, [x, int(2)]),
        app(POW, [y, int(2)]),
      ]),
    ]);

    expect(vm.eval(expr)).toEqual(app(MUL, [
      app(SUB, [x, y]),
      app(ADD, [x, y]),
    ]));
  });

  it("factors bivariate differences of cubes", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const y = sym("y");
    const expr = app(FACTOR, [
      app(SUB, [
        app(POW, [x, int(3)]),
        app(POW, [y, int(3)]),
      ]),
    ]);

    expect(vm.eval(expr)).toEqual(app(MUL, [
      app(SUB, [x, y]),
      app(ADD, [
        app(ADD, [
          app(POW, [x, int(2)]),
          app(MUL, [x, y]),
        ]),
        app(POW, [y, int(2)]),
      ]),
    ]));
  });

  it("factors bivariate sums of cubes", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const y = sym("y");
    const expr = app(FACTOR, [
      app(ADD, [
        app(POW, [x, int(3)]),
        app(POW, [y, int(3)]),
      ]),
    ]);

    expect(vm.eval(expr)).toEqual(app(MUL, [
      app(ADD, [x, y]),
      app(ADD, [
        app(ADD, [
          app(POW, [x, int(2)]),
          app(MUL, [int(-1), app(MUL, [x, y])]),
        ]),
        app(POW, [y, int(2)]),
      ]),
    ]));
  });

  it("factors grouped multivariate terms", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const y = sym("y");
    const z = sym("z");
    const expr = app(FACTOR, [
      app(ADD, [
        app(ADD, [app(MUL, [x, y]), app(MUL, [x, z])]),
        app(ADD, [y, z]),
      ]),
    ]);

    expect(vm.eval(expr)).toEqual(app(MUL, [
      app(ADD, [x, int(1)]),
      app(ADD, [y, z]),
    ]));
  });

  it("factors grouped multivariate terms with signed residuals", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const y = sym("y");
    const z = sym("z");
    const expr = app(FACTOR, [
      app(ADD, [
        app(SUB, [app(MUL, [x, y]), app(MUL, [x, z])]),
        app(SUB, [y, z]),
      ]),
    ]);

    expect(vm.eval(expr)).toEqual(app(MUL, [
      app(ADD, [x, int(1)]),
      app(ADD, [y, app(MUL, [int(-1), z])]),
    ]));
  });

  it("factors bivariate perfect cube sums", () => {
    // (a + b)^3 = a^3 + 3a^2b + 3ab^2 + b^3
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const y = sym("y");
    // Build x^3 + 3*x^2*y + 3*x*y^2 + y^3 as nested ADD nodes
    const expr = app(FACTOR, [
      app(ADD, [
        app(ADD, [
          app(ADD, [
            app(POW, [x, int(3)]),
            app(MUL, [int(3), app(MUL, [app(POW, [x, int(2)]), y])]),
          ]),
          app(MUL, [int(3), app(MUL, [x, app(POW, [y, int(2)])])]),
        ]),
        app(POW, [y, int(3)]),
      ]),
    ]);

    expect(vm.eval(expr)).toEqual(app(POW, [app(ADD, [x, y]), int(3)]));
  });

  it("factors bivariate perfect cube differences", () => {
    // (a - b)^3 = a^3 - 3a^2b + 3ab^2 - b^3
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const y = sym("y");
    // Build x^3 - 3*x^2*y + 3*x*y^2 - y^3 using SUB to negate the right terms
    const expr = app(FACTOR, [
      app(ADD, [
        app(SUB, [
          app(ADD, [
            app(POW, [x, int(3)]),
            app(MUL, [int(3), app(MUL, [x, app(POW, [y, int(2)])])]),
          ]),
          app(MUL, [int(3), app(MUL, [app(POW, [x, int(2)]), y])]),
        ]),
        app(MUL, [int(-1), app(POW, [y, int(3)])]),
      ]),
    ]);

    expect(vm.eval(expr)).toEqual(app(POW, [app(SUB, [x, y]), int(3)]));
  });

  it("supports assignment and later lookup", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(ASSIGN, [sym("x"), app(ADD, [int(2), int(3)])]))).toEqual(int(5));
    expect(vm.eval(app(MUL, [sym("x"), int(2)]))).toEqual(int(10));
  });

  it("stores delayed function definitions and applies user functions", () => {
    const vm = new VM(new SymbolicBackend());
    const body = app(POW, [sym("x"), int(2)]);
    expect(vm.eval(app(DEFINE, [sym("square"), app(LIST, [sym("x")]), body]))).toEqual(sym("square"));
    expect(vm.eval(app(sym("square"), [int(5)]))).toEqual(int(25));
  });

  it("supports exact division and negative powers", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(DIV, [int(3), int(2)]))).toEqual(rational(3, 2));
    expect(vm.eval(app(POW, [int(2), int(-3)]))).toEqual(rational(1, 8));
  });

  it("evaluates elementary numeric functions and exact identities", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(SIN, [int(0)]))).toEqual(int(0));
  });

  it("evaluates reciprocal hyperbolic numeric functions and exact identities", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(COTH, [numberNode(1.2)]))).toEqual(numberNode(Math.cosh(1.2) / Math.sinh(1.2)));
    expect(vm.eval(app(SECH, [numberNode(0.7)]))).toEqual(numberNode(1 / Math.cosh(0.7)));
    expect(vm.eval(app(CSCH, [numberNode(0.5)]))).toEqual(numberNode(1 / Math.sinh(0.5)));
    expect(vm.eval(app(SECH, [int(0)]))).toEqual(int(1));
  });

  it("leaves reciprocal hyperbolic symbolic applications held", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(COTH, [sym("x")]))).toEqual(app(COTH, [sym("x")]));
    expect(vm.eval(app(SECH, [sym("x")]))).toEqual(app(SECH, [sym("x")]));
    expect(vm.eval(app(CSCH, [sym("x")]))).toEqual(app(CSCH, [sym("x")]));
  });

  it("evaluates comparisons and held if branches", () => {
    const vm = new VM(new SymbolicBackend());
    const expr = app(IF, [
      app(EQUAL, [int(1), int(1)]),
      app(ASSIGN, [sym("x"), int(7)]),
      app(ASSIGN, [sym("x"), int(9)]),
    ]);
    expect(vm.eval(expr)).toEqual(int(7));
    expect(vm.eval(sym("x"))).toEqual(int(7));
  });

  it("evaluates boolean equality as symbols", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(EQUAL, [TRUE, TRUE]))).toEqual(TRUE);
    expect(vm.eval(app(EQUAL, [TRUE, FALSE]))).toEqual(FALSE);
  });

  it("checks structural equality helper remains compatible with results", () => {
    const vm = new VM(new SymbolicBackend());
    const result = vm.eval(app(ADD, [int(1), int(2)]));
    expect(equals(result, int(3))).toBe(true);
  });

  it("keeps D symbolic-backend-only", () => {
    const vm = new VM(new StrictBackend());
    expect(() => vm.eval(app(D, [int(1), int(1)]))).toThrow(StrictEvaluationError);
  });

  it("differentiates constants and variable identity", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(D, [int(5), sym("x")]))).toEqual(int(0));
    expect(vm.eval(app(D, [sym("y"), sym("x")]))).toEqual(int(0));
    expect(vm.eval(app(D, [sym("x"), sym("x")]))).toEqual(int(1));
  });

  it("differentiates Add, Sub, and Neg", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(D, [app(ADD, [app(POW, [sym("x"), int(2)]), sym("x")]), sym("x")]))).toEqual(
      app(ADD, [app(MUL, [int(2), sym("x")]), int(1)]),
    );
    expect(vm.eval(app(D, [app(SUB, [sym("x"), sym("y")]), sym("x")]))).toEqual(int(1));
    expect(vm.eval(app(D, [app(NEG, [sym("x")]), sym("x")]))).toEqual(int(-1));
  });

  it("differentiates product and quotient rules", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(D, [app(MUL, [sym("x"), sym("y")]), sym("x")]))).toEqual(sym("y"));
    expect(vm.eval(app(D, [app(DIV, [sym("x"), sym("y")]), sym("x")]))).toEqual(
      app(DIV, [sym("y"), app(POW, [sym("y"), int(2)])]),
    );
  });

  it("differentiates constant, exponential, and general powers", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(D, [app(POW, [sym("x"), int(3)]), sym("x")]))).toEqual(
      app(MUL, [int(3), app(POW, [sym("x"), int(2)])]),
    );
    expect(vm.eval(app(D, [app(POW, [int(2), sym("x")]), sym("x")]))).toEqual(
      app(MUL, [app(POW, [int(2), sym("x")]), numberNode(Math.log(2))]),
    );
    // Phase 30: exp(x·log(x)) now simplifies to x^x, so D(x^x, x) = x^x·(log(x)+1).
    expect(vm.eval(app(D, [app(POW, [sym("x"), sym("x")]), sym("x")]))).toEqual(
      app(MUL, [
        app(POW, [sym("x"), sym("x")]),
        app(ADD, [app(LOG, [sym("x")]), app(MUL, [sym("x"), app(DIV, [int(1), sym("x")])])]),
      ]),
    );
  });

  it("applies elementary chain rules", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(D, [app(SIN, [app(POW, [sym("x"), int(2)])]), sym("x")]))).toEqual(
      app(MUL, [app(COS, [app(POW, [sym("x"), int(2)])]), app(MUL, [int(2), sym("x")])]),
    );
    expect(vm.eval(app(D, [app(COS, [sym("x")]), sym("x")]))).toEqual(app(NEG, [app(SIN, [sym("x")])]));
    expect(vm.eval(app(D, [app(TAN, [sym("x")]), sym("x")]))).toEqual(
      app(DIV, [int(1), app(POW, [app(COS, [sym("x")]), int(2)])]),
    );
    expect(vm.eval(app(D, [app(EXP, [sym("x")]), sym("x")]))).toEqual(app(EXP, [sym("x")]));
    expect(vm.eval(app(D, [app(LOG, [sym("x")]), sym("x")]))).toEqual(app(DIV, [int(1), sym("x")]));
    expect(vm.eval(app(D, [app(SQRT, [sym("x")]), sym("x")]))).toEqual(
      app(DIV, [int(1), app(MUL, [int(2), app(SQRT, [sym("x")])])]),
    );
    expect(vm.eval(app(D, [app(ASIN, [sym("x")]), sym("x")]))).toEqual(
      app(DIV, [int(1), app(SQRT, [app(SUB, [int(1), app(POW, [sym("x"), int(2)])])])]),
    );
    expect(vm.eval(app(D, [app(ACOS, [sym("x")]), sym("x")]))).toEqual(
      app(NEG, [app(DIV, [int(1), app(SQRT, [app(SUB, [int(1), app(POW, [sym("x"), int(2)])])])])]),
    );
  });

  it("applies hyperbolic and inverse hyperbolic chain rules", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(D, [app(SINH, [sym("x")]), sym("x")]))).toEqual(app(COSH, [sym("x")]));
    expect(vm.eval(app(D, [app(COSH, [sym("x")]), sym("x")]))).toEqual(app(SINH, [sym("x")]));
    expect(vm.eval(app(D, [app(TANH, [sym("x")]), sym("x")]))).toEqual(
      app(DIV, [int(1), app(POW, [app(COSH, [sym("x")]), int(2)])]),
    );
    expect(vm.eval(app(D, [app(ASINH, [sym("x")]), sym("x")]))).toEqual(
      app(DIV, [int(1), app(SQRT, [app(ADD, [app(POW, [sym("x"), int(2)]), int(1)])])]),
    );
    expect(vm.eval(app(D, [app(ACOSH, [sym("x")]), sym("x")]))).toEqual(
      app(DIV, [int(1), app(SQRT, [app(SUB, [app(POW, [sym("x"), int(2)]), int(1)])])]),
    );
    expect(vm.eval(app(D, [app(ATANH, [sym("x")]), sym("x")]))).toEqual(
      app(DIV, [int(1), app(SUB, [int(1), app(POW, [sym("x"), int(2)])])]),
    );
  });

  it("applies reciprocal hyperbolic chain rules through Sinh and Cosh", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const square = app(POW, [x, int(2)]);
    expect(vm.eval(app(D, [app(COTH, [x]), x]))).toEqual(
      app(NEG, [app(DIV, [int(1), app(POW, [app(SINH, [x]), int(2)])])]),
    );
    expect(vm.eval(app(D, [app(SECH, [x]), x]))).toEqual(
      app(NEG, [app(DIV, [app(SINH, [x]), app(POW, [app(COSH, [x]), int(2)])])]),
    );
    expect(vm.eval(app(D, [app(CSCH, [x]), x]))).toEqual(
      app(NEG, [app(DIV, [app(COSH, [x]), app(POW, [app(SINH, [x]), int(2)])])]),
    );
    expect(vm.eval(app(D, [app(COTH, [square]), x]))).toEqual(
      app(NEG, [app(DIV, [app(MUL, [int(2), x]), app(POW, [app(SINH, [square]), int(2)])])]),
    );
  });

  it("leaves unknown dependent derivatives unevaluated", () => {
    const vm = new VM(new SymbolicBackend());
    const expr = app(D, [app(sym("F"), [sym("x")]), sym("x")]);
    expect(vm.eval(expr)).toEqual(expr);
  });

  it("keeps Integrate symbolic-backend-only", () => {
    const vm = new VM(new StrictBackend());
    expect(() => vm.eval(app(INTEGRATE, [int(1), sym("x")]))).toThrow(StrictEvaluationError);
  });

  it("integrates constants, free symbols, and variable identity", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(INTEGRATE, [int(5), sym("x")]))).toEqual(app(MUL, [int(5), sym("x")]));
    expect(vm.eval(app(INTEGRATE, [sym("y"), sym("x")]))).toEqual(app(MUL, [sym("y"), sym("x")]));
    expect(vm.eval(app(INTEGRATE, [sym("x"), sym("x")]))).toEqual(
      app(MUL, [rational(1, 2), app(POW, [sym("x"), int(2)])]),
    );
    expect(vm.eval(app(INTEGRATE, [int(0), sym("x")]))).toEqual(int(0));
  });

  it("integrates powers and reciprocals", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(INTEGRATE, [app(POW, [sym("x"), int(2)]), sym("x")]))).toEqual(
      app(MUL, [rational(1, 3), app(POW, [sym("x"), int(3)])]),
    );
    expect(vm.eval(app(INTEGRATE, [app(POW, [sym("x"), rational(1, 2)]), sym("x")]))).toEqual(
      app(MUL, [rational(2, 3), app(POW, [sym("x"), rational(3, 2)])]),
    );
    expect(vm.eval(app(INTEGRATE, [app(POW, [sym("x"), int(-1)]), sym("x")]))).toEqual(
      app(LOG, [sym("x")]),
    );
    expect(vm.eval(app(INTEGRATE, [app(DIV, [int(3), sym("x")]), sym("x")]))).toEqual(
      app(MUL, [int(3), app(LOG, [sym("x")])]),
    );
  });

  it("integrates Add, Sub, Neg, and constant factors", () => {
    const vm = new VM(new SymbolicBackend());
    const halfXSquared = app(MUL, [rational(1, 2), app(POW, [sym("x"), int(2)])]);
    expect(vm.eval(app(INTEGRATE, [app(ADD, [sym("x"), int(3)]), sym("x")]))).toEqual(
      app(ADD, [halfXSquared, app(MUL, [int(3), sym("x")])]),
    );
    expect(vm.eval(app(INTEGRATE, [app(SUB, [sym("x"), int(1)]), sym("x")]))).toEqual(
      app(SUB, [halfXSquared, sym("x")]),
    );
    expect(vm.eval(app(INTEGRATE, [app(NEG, [sym("x")]), sym("x")]))).toEqual(app(NEG, [halfXSquared]));
    expect(vm.eval(app(INTEGRATE, [app(MUL, [sym("y"), app(POW, [sym("x"), int(2)])]), sym("x")]))).toEqual(
      app(MUL, [sym("y"), app(MUL, [rational(1, 3), app(POW, [sym("x"), int(3)])])]),
    );
    expect(vm.eval(app(INTEGRATE, [app(MUL, [sym("x"), int(7)]), sym("x")]))).toEqual(
      app(MUL, [int(7), halfXSquared]),
    );
  });

  it("integrates direct elementary functions at x", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(INTEGRATE, [app(SIN, [sym("x")]), sym("x")]))).toEqual(
      app(NEG, [app(COS, [sym("x")])]),
    );
    expect(vm.eval(app(INTEGRATE, [app(COS, [sym("x")]), sym("x")]))).toEqual(app(SIN, [sym("x")]));
    expect(vm.eval(app(INTEGRATE, [app(EXP, [sym("x")]), sym("x")]))).toEqual(app(EXP, [sym("x")]));
    expect(vm.eval(app(INTEGRATE, [app(LOG, [sym("x")]), sym("x")]))).toEqual(
      app(SUB, [app(MUL, [sym("x"), app(LOG, [sym("x")])]), sym("x")]),
    );
    expect(vm.eval(app(INTEGRATE, [app(SQRT, [sym("x")]), sym("x")]))).toEqual(
      app(MUL, [rational(2, 3), app(POW, [sym("x"), rational(3, 2)])]),
    );
  });

  it("integrates constant-base exponentials", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(INTEGRATE, [app(POW, [int(2), sym("x")]), sym("x")]))).toEqual(
      app(DIV, [app(POW, [int(2), sym("x")]), numberNode(Math.log(2))]),
    );
  });

  it("closes Phase 23 erf/erfi quadratic exponential integrals", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const xsq = app(POW, [x, int(2)]);

    const erfOut = vm.eval(app(INTEGRATE, [app(EXP, [app(NEG, [xsq])]), x]));
    expect(containsHeadName(erfOut as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    expect(containsHeadName(erfOut as unknown as ReturnType<typeof sym>, "Erf")).toBe(true);

    const erfiOut = vm.eval(app(INTEGRATE, [app(EXP, [xsq]), x]));
    expect(containsHeadName(erfiOut as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    expect(containsHeadName(erfiOut as unknown as ReturnType<typeof sym>, "Erfi")).toBe(true);

    const scaled = vm.eval(app(INTEGRATE, [app(EXP, [app(MUL, [int(-4), xsq])]), x]));
    expect(containsHeadName(scaled as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    expect(containsHeadName(scaled as unknown as ReturnType<typeof sym>, "Erf")).toBe(true);
  });

  it("recognizes elliptic first-kind integrals", () => {
    const vm = new VM(new SymbolicBackend());
    const theta = sym("theta");
    const k = sym("k");
    const integrand = app(DIV, [
      int(1),
      app(SQRT, [
        app(SUB, [
          int(1),
          app(MUL, [
            app(POW, [k, int(2)]),
            app(POW, [app(SIN, [theta]), int(2)]),
          ]),
        ]),
      ]),
    ]);

    expect(vm.eval(app(INTEGRATE, [integrand, theta]))).toEqual(app(sym("EllipticF"), [theta, k]));
    expect(vm.eval(app(INTEGRATE, [integrand, theta, int(0), app(DIV, [sym("%pi"), int(2)])]))).toEqual(
      app(sym("EllipticK"), [k]),
    );
  });

  it("recognizes complete and incomplete elliptic second-kind integrals", () => {
    // EllipticE — integrand is Sqrt(1 - k²·sin²(θ)) (NOT 1/Sqrt like EllipticF).
    //
    // Complete form:  ∫₀^(π/2) sqrt(1-k²sin²θ) dθ  → EllipticE(k)
    // Incomplete form: ∫ sqrt(1-k²sin²θ) dθ        → EllipticE(θ, k)
    const vm = new VM(new SymbolicBackend());
    const theta = sym("theta");
    const k = sym("k");
    const piOver2 = app(DIV, [sym("%pi"), int(2)]);

    // ── symbolic modulus ──────────────────────────────────────────────────────
    const eIntegrand = app(SQRT, [
      app(SUB, [
        int(1),
        app(MUL, [
          app(POW, [k, int(2)]),
          app(POW, [app(SIN, [theta]), int(2)]),
        ]),
      ]),
    ]);

    // Complete EllipticE: ∫₀^(π/2) sqrt(1-k²sin²θ) dθ → EllipticE(k)
    expect(vm.eval(app(INTEGRATE, [eIntegrand, theta, int(0), piOver2]))).toEqual(
      app(sym("EllipticE"), [k]),
    );
    // Incomplete EllipticE: ∫ sqrt(1-k²sin²θ) dθ → EllipticE(θ, k)
    expect(vm.eval(app(INTEGRATE, [eIntegrand, theta]))).toEqual(
      app(sym("EllipticE"), [theta, k]),
    );

    // ── pre-evaluated numeric modulus: (1/2)^2 = IRRational(1,4) ─────────────
    // The compiler folds (1/2)^2 to rational(1,4); we must recover k = 1/2.
    const eIntegrandNumeric = app(SQRT, [
      app(SUB, [
        int(1),
        app(MUL, [
          rational(1, 4),  // (1/2)^2 pre-evaluated
          app(POW, [app(SIN, [theta]), int(2)]),
        ]),
      ]),
    ]);

    // Complete with numeric k²=1/4 → EllipticE(1/2)
    expect(vm.eval(app(INTEGRATE, [eIntegrandNumeric, theta, int(0), piOver2]))).toEqual(
      app(sym("EllipticE"), [rational(1, 2)]),
    );
    // Incomplete with numeric k²=1/4 → EllipticE(θ, 1/2)
    expect(vm.eval(app(INTEGRATE, [eIntegrandNumeric, theta]))).toEqual(
      app(sym("EllipticE"), [theta, rational(1, 2)]),
    );

    // ── pre-evaluated float modulus: 0.5^2 = IRFloat(0.25) ───────────────────
    const eIntegrandFloat = app(SQRT, [
      app(SUB, [
        int(1),
        app(MUL, [
          numberNode(0.25),  // 0.5^2 pre-evaluated
          app(POW, [app(SIN, [theta]), int(2)]),
        ]),
      ]),
    ]);

    // Complete with float k²=0.25 → EllipticE(0.5)
    expect(vm.eval(app(INTEGRATE, [eIntegrandFloat, theta, int(0), piOver2]))).toEqual(
      app(sym("EllipticE"), [numberNode(0.5)]),
    );
  });

  it("recognizes complete elliptic third-kind integrals (EllipticPi)", () => {
    // EllipticPi — integrand is 1/((1+n·sin²θ)·sqrt(1-k²sin²θ)).
    //
    // Complete form: ∫₀^(π/2) 1/((1+n·sin²θ)·sqrt(1-k²sin²θ)) dθ → EllipticPi(n, k)
    const vm = new VM(new SymbolicBackend());
    const theta = sym("theta");
    const k = sym("k");
    const n = sym("n");
    const piOver2 = app(DIV, [sym("%pi"), int(2)]);

    // ── symbolic n and k ──────────────────────────────────────────────────────
    const piIntegrand = app(DIV, [
      int(1),
      app(MUL, [
        app(ADD, [int(1), app(MUL, [n, app(POW, [app(SIN, [theta]), int(2)])])]),
        app(SQRT, [
          app(SUB, [
            int(1),
            app(MUL, [
              app(POW, [k, int(2)]),
              app(POW, [app(SIN, [theta]), int(2)]),
            ]),
          ]),
        ]),
      ]),
    ]);

    expect(vm.eval(app(INTEGRATE, [piIntegrand, theta, int(0), piOver2]))).toEqual(
      app(sym("EllipticPi"), [n, k]),
    );

    // ── numeric k² = rational(1,4) ────────────────────────────────────────────
    const piIntegrandNumericK = app(DIV, [
      int(1),
      app(MUL, [
        app(ADD, [int(1), app(MUL, [int(2), app(POW, [app(SIN, [theta]), int(2)])])]),
        app(SQRT, [
          app(SUB, [
            int(1),
            app(MUL, [
              rational(1, 4),  // (1/2)^2 pre-evaluated
              app(POW, [app(SIN, [theta]), int(2)]),
            ]),
          ]),
        ]),
      ]),
    ]);

    // EllipticPi(2, 1/2)
    expect(vm.eval(app(INTEGRATE, [piIntegrandNumericK, theta, int(0), piOver2]))).toEqual(
      app(sym("EllipticPi"), [int(2), rational(1, 2)]),
    );
  });

  it("regression: EllipticK and EllipticF still work after adding EllipticE and EllipticPi", () => {
    // Guard that the second-kind and third-kind recognisers do not shadow the
    // first-kind recognisers accidentally.
    const vm = new VM(new SymbolicBackend());
    const theta = sym("theta");
    const k = sym("k");
    const piOver2 = app(DIV, [sym("%pi"), int(2)]);
    const kIntegrand = app(DIV, [
      int(1),
      app(SQRT, [
        app(SUB, [
          int(1),
          app(MUL, [
            app(POW, [k, int(2)]),
            app(POW, [app(SIN, [theta]), int(2)]),
          ]),
        ]),
      ]),
    ]);
    expect(vm.eval(app(INTEGRATE, [kIntegrand, theta, int(0), piOver2]))).toEqual(
      app(sym("EllipticK"), [k]),
    );
    expect(vm.eval(app(INTEGRATE, [kIntegrand, theta]))).toEqual(
      app(sym("EllipticF"), [theta, k]),
    );
  });

  it("leaves unknown dependent integrals unevaluated", () => {
    const vm = new VM(new SymbolicBackend());
    const expr = app(INTEGRATE, [app(sym("F"), [sym("x")]), sym("x")]);
    expect(vm.eval(expr)).toEqual(expr);
  });

  it("leaves non-symbol integration variables unevaluated and rejects wrong arity", () => {
    const vm = new VM(new SymbolicBackend());
    const expr = app(INTEGRATE, [sym("x"), int(1)]);
    expect(vm.eval(expr)).toEqual(expr);
    expect(() => vm.eval(app(INTEGRATE, [sym("x")]))).toThrow(ArityError);
  });

  // Recursively check whether any sub-node has the given head symbol name.
  function containsHeadName(node: ReturnType<typeof sym>, name: string): boolean {
    const n = node as unknown as { kind: string; name?: string; head?: unknown; args?: unknown[] };
    if (n.kind === "apply") {
      const h = n.head as { name?: string } | null;
      if (h?.name === name) return true;
      return (n.args ?? []).some((a) => containsHeadName(a as ReturnType<typeof sym>, name));
    }
    return false;
  }

  // Phase 26 — log-power IBP reduction
  // ∫ log(x)^2 dx = x·log²x − 2x·log x + 2x
  it("Phase 26: ∫ log(x)^2 dx returns a closed form", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const integrand = app(POW, [app(LOG, [x]), int(2)]);
    const result = vm.eval(app(INTEGRATE, [integrand, x]));
    // Must not contain INTEGRATE as its head
    expect(result).not.toMatchObject({ head: INTEGRATE });
    // Must contain LOG (antiderivative involves log(x))
    expect(containsHeadName(result as unknown as ReturnType<typeof sym>, "Log")).toBe(true);
  });

  it("Phase 26: ∫ x·log(x)^2 dx numerical correctness", () => {
    // Closed form: x²/4·log²x − x²/4·log x + x²/8
    // ∫₁^2 x·log(x)^2 dx
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const integrand = app(MUL, [x, app(POW, [app(LOG, [x]), int(2)])]);
    const result = vm.eval(app(INTEGRATE, [integrand, x]));

    function evalIR(node: ReturnType<typeof sym>, xVal: number): number {
      const n = node as unknown as { kind: string; value?: number | bigint; numer?: bigint; denom?: bigint; head?: unknown; args?: unknown[] };
      if (n.kind === "integer") return Number(n.value);
      if (n.kind === "rational") return Number(n.numer!) / Number(n.denom!);
      if (n.kind === "float") return n.value as number;
      if (n.kind === "symbol") return xVal;
      const h = (n.head as { name?: string })?.name ?? "";
      const args = n.args as typeof node[];
      if (h === "Add") return evalIR(args[0], xVal) + evalIR(args[1], xVal);
      if (h === "Sub") return evalIR(args[0], xVal) - evalIR(args[1], xVal);
      if (h === "Mul") return evalIR(args[0], xVal) * evalIR(args[1], xVal);
      if (h === "Div") return evalIR(args[0], xVal) / evalIR(args[1], xVal);
      if (h === "Neg") return -evalIR(args[0], xVal);
      if (h === "Pow") return Math.pow(evalIR(args[0], xVal), evalIR(args[1], xVal));
      if (h === "Log") return Math.log(evalIR(args[0], xVal));
      if (h === "Sin") return Math.sin(evalIR(args[0], xVal));
      if (h === "Cos") return Math.cos(evalIR(args[0], xVal));
      throw new Error(`unsupported head: ${h}`);
    }

    function trapezoid(fn: (t: number) => number, a: number, b: number, n = 10000): number {
      const h = (b - a) / n;
      let total = 0.5 * (fn(a) + fn(b));
      for (let i = 1; i < n; i++) total += fn(a + i * h);
      return total * h;
    }

    const antiderivDiff = evalIR(result as unknown as ReturnType<typeof sym>, 2) -
                          evalIR(result as unknown as ReturnType<typeof sym>, 1);
    const numerical = trapezoid(t => t * Math.log(t) ** 2, 1, 2);
    expect(Math.abs(antiderivDiff - numerical)).toBeLessThan(1e-5);
  });

  // Phase 27 — trig-of-log integration
  it("Phase 27: ∫ sin(log(x)) dx returns a closed form with SIN and COS", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const integrand = app(SIN, [app(LOG, [x])]);
    const result = vm.eval(app(INTEGRATE, [integrand, x]));
    expect(result).not.toMatchObject({ head: INTEGRATE });
    expect(containsHeadName(result as unknown as ReturnType<typeof sym>, "Sin")).toBe(true);
    expect(containsHeadName(result as unknown as ReturnType<typeof sym>, "Cos")).toBe(true);
  });

  it("Phase 27: ∫ sin(log(x)) dx numerical correctness", () => {
    // ∫₁^3 sin(log x) dx
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const integrand = app(SIN, [app(LOG, [x])]);
    const result = vm.eval(app(INTEGRATE, [integrand, x]));

    function evalIR(node: ReturnType<typeof sym>, xVal: number): number {
      const n = node as unknown as { kind: string; value?: number | bigint; numer?: bigint; denom?: bigint; head?: unknown; args?: unknown[] };
      if (n.kind === "integer") return Number(n.value);
      if (n.kind === "rational") return Number(n.numer!) / Number(n.denom!);
      if (n.kind === "float") return n.value as number;
      if (n.kind === "symbol") return xVal;
      const h = (n.head as { name?: string })?.name ?? "";
      const args = n.args as typeof node[];
      if (h === "Add") return evalIR(args[0], xVal) + evalIR(args[1], xVal);
      if (h === "Sub") return evalIR(args[0], xVal) - evalIR(args[1], xVal);
      if (h === "Mul") return evalIR(args[0], xVal) * evalIR(args[1], xVal);
      if (h === "Div") return evalIR(args[0], xVal) / evalIR(args[1], xVal);
      if (h === "Neg") return -evalIR(args[0], xVal);
      if (h === "Pow") return Math.pow(evalIR(args[0], xVal), evalIR(args[1], xVal));
      if (h === "Log") return Math.log(evalIR(args[0], xVal));
      if (h === "Sin") return Math.sin(evalIR(args[0], xVal));
      if (h === "Cos") return Math.cos(evalIR(args[0], xVal));
      throw new Error(`unsupported head: ${h}`);
    }

    function trapezoid(fn: (t: number) => number, a: number, b: number, n = 10000): number {
      const h = (b - a) / n;
      let total = 0.5 * (fn(a) + fn(b));
      for (let i = 1; i < n; i++) total += fn(a + i * h);
      return total * h;
    }

    const antiderivDiff = evalIR(result as unknown as ReturnType<typeof sym>, 3) -
                          evalIR(result as unknown as ReturnType<typeof sym>, 1);
    const numerical = trapezoid(t => Math.sin(Math.log(t)), 1, 3);
    expect(Math.abs(antiderivDiff - numerical)).toBeLessThan(1e-5);
  });

  it("Phase 27: ∫ cos(log(x)) dx numerical correctness", () => {
    // ∫₁^3 cos(log x) dx
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const integrand = app(COS, [app(LOG, [x])]);
    const result = vm.eval(app(INTEGRATE, [integrand, x]));

    function evalIR(node: ReturnType<typeof sym>, xVal: number): number {
      const n = node as unknown as { kind: string; value?: number | bigint; numer?: bigint; denom?: bigint; head?: unknown; args?: unknown[] };
      if (n.kind === "integer") return Number(n.value);
      if (n.kind === "rational") return Number(n.numer!) / Number(n.denom!);
      if (n.kind === "float") return n.value as number;
      if (n.kind === "symbol") return xVal;
      const h = (n.head as { name?: string })?.name ?? "";
      const args = n.args as typeof node[];
      if (h === "Add") return evalIR(args[0], xVal) + evalIR(args[1], xVal);
      if (h === "Sub") return evalIR(args[0], xVal) - evalIR(args[1], xVal);
      if (h === "Mul") return evalIR(args[0], xVal) * evalIR(args[1], xVal);
      if (h === "Div") return evalIR(args[0], xVal) / evalIR(args[1], xVal);
      if (h === "Neg") return -evalIR(args[0], xVal);
      if (h === "Pow") return Math.pow(evalIR(args[0], xVal), evalIR(args[1], xVal));
      if (h === "Log") return Math.log(evalIR(args[0], xVal));
      if (h === "Sin") return Math.sin(evalIR(args[0], xVal));
      if (h === "Cos") return Math.cos(evalIR(args[0], xVal));
      throw new Error(`unsupported head: ${h}`);
    }

    function trapezoid(fn: (t: number) => number, a: number, b: number, n = 10000): number {
      const h = (b - a) / n;
      let total = 0.5 * (fn(a) + fn(b));
      for (let i = 1; i < n; i++) total += fn(a + i * h);
      return total * h;
    }

    const antiderivDiff = evalIR(result as unknown as ReturnType<typeof sym>, 3) -
                          evalIR(result as unknown as ReturnType<typeof sym>, 1);
    const numerical = trapezoid(t => Math.cos(Math.log(t)), 1, 3);
    expect(Math.abs(antiderivDiff - numerical)).toBeLessThan(1e-5);
  });

  it("Phase 27: ∫ x·sin(log(x)) dx numerical correctness", () => {
    // ∫₁^2 x·sin(log x) dx
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const integrand = app(MUL, [x, app(SIN, [app(LOG, [x])])]);
    const result = vm.eval(app(INTEGRATE, [integrand, x]));
    expect(result).not.toMatchObject({ head: INTEGRATE });

    function evalIR(node: ReturnType<typeof sym>, xVal: number): number {
      const n = node as unknown as { kind: string; value?: number | bigint; numer?: bigint; denom?: bigint; head?: unknown; args?: unknown[] };
      if (n.kind === "integer") return Number(n.value);
      if (n.kind === "rational") return Number(n.numer!) / Number(n.denom!);
      if (n.kind === "float") return n.value as number;
      if (n.kind === "symbol") return xVal;
      const h = (n.head as { name?: string })?.name ?? "";
      const args = n.args as typeof node[];
      if (h === "Add") return evalIR(args[0], xVal) + evalIR(args[1], xVal);
      if (h === "Sub") return evalIR(args[0], xVal) - evalIR(args[1], xVal);
      if (h === "Mul") return evalIR(args[0], xVal) * evalIR(args[1], xVal);
      if (h === "Div") return evalIR(args[0], xVal) / evalIR(args[1], xVal);
      if (h === "Neg") return -evalIR(args[0], xVal);
      if (h === "Pow") return Math.pow(evalIR(args[0], xVal), evalIR(args[1], xVal));
      if (h === "Log") return Math.log(evalIR(args[0], xVal));
      if (h === "Sin") return Math.sin(evalIR(args[0], xVal));
      if (h === "Cos") return Math.cos(evalIR(args[0], xVal));
      throw new Error(`unsupported head: ${h}`);
    }

    function trapezoid(fn: (t: number) => number, a: number, b: number, n = 10000): number {
      const h = (b - a) / n;
      let total = 0.5 * (fn(a) + fn(b));
      for (let i = 1; i < n; i++) total += fn(a + i * h);
      return total * h;
    }

    const antiderivDiff = evalIR(result as unknown as ReturnType<typeof sym>, 2) -
                          evalIR(result as unknown as ReturnType<typeof sym>, 1);
    const numerical = trapezoid(t => t * Math.sin(Math.log(t)), 1, 2);
    expect(Math.abs(antiderivDiff - numerical)).toBeLessThan(1e-5);
  });

  it("Phase 27: regression — ∫ sin(x) dx still works", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const result = vm.eval(app(INTEGRATE, [app(SIN, [x]), x]));
    // Should be -cos(x)
    expect(result).toEqual(app(NEG, [app(COS, [x])]));
  });

  it("Phase 27: regression — ∫ cos(x) dx still works", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const result = vm.eval(app(INTEGRATE, [app(COS, [x]), x]));
    // Should be sin(x)
    expect(result).toEqual(app(SIN, [x]));
  });

  // Phase 28 — general IBP: poly × log(Q) and poly × atan(Q) for non-linear Q
  // ─────────────────────────────────────────────────────────────────────────────
  // Helper: numerically evaluate a simple IR tree at x = xVal.
  // Includes Atan for Phase 28 atan residuals.
  function evalIR28(node: ReturnType<typeof sym>, xVal: number): number {
    const n = node as unknown as {
      kind: string; value?: number | bigint; numer?: bigint; denom?: bigint;
      head?: unknown; args?: unknown[];
    };
    if (n.kind === "integer") return Number(n.value);
    if (n.kind === "rational") return Number(n.numer!) / Number(n.denom!);
    if (n.kind === "float") return n.value as number;
    if (n.kind === "symbol") return xVal;
    const h = (n.head as { name?: string })?.name ?? "";
    const args = n.args as typeof node[];
    if (h === "Add") return evalIR28(args[0], xVal) + evalIR28(args[1], xVal);
    if (h === "Sub") return evalIR28(args[0], xVal) - evalIR28(args[1], xVal);
    if (h === "Mul") return evalIR28(args[0], xVal) * evalIR28(args[1], xVal);
    if (h === "Div") return evalIR28(args[0], xVal) / evalIR28(args[1], xVal);
    if (h === "Neg") return -evalIR28(args[0], xVal);
    if (h === "Pow") return Math.pow(evalIR28(args[0], xVal), evalIR28(args[1], xVal));
    if (h === "Log") return Math.log(evalIR28(args[0], xVal));
    if (h === "Sqrt") return Math.sqrt(evalIR28(args[0], xVal));
    if (h === "Sin") return Math.sin(evalIR28(args[0], xVal));
    if (h === "Cos") return Math.cos(evalIR28(args[0], xVal));
    if (h === "Asin") return Math.asin(evalIR28(args[0], xVal));
    if (h === "Acos") return Math.acos(evalIR28(args[0], xVal));
    if (h === "Atan") return Math.atan(evalIR28(args[0], xVal));
    if (h === "Sinh") return Math.sinh(evalIR28(args[0], xVal));
    if (h === "Cosh") return Math.cosh(evalIR28(args[0], xVal));
    if (h === "Tanh") return Math.tanh(evalIR28(args[0], xVal));
    if (h === "Coth") {
      const v = evalIR28(args[0], xVal);
      return Math.cosh(v) / Math.sinh(v);
    }
    if (h === "Sech") return 1 / Math.cosh(evalIR28(args[0], xVal));
    if (h === "Csch") return 1 / Math.sinh(evalIR28(args[0], xVal));
    if (h === "Asinh") return Math.asinh(evalIR28(args[0], xVal));
    if (h === "Acosh") return Math.acosh(evalIR28(args[0], xVal));
    if (h === "Atanh") return Math.atanh(evalIR28(args[0], xVal));
    throw new Error(`evalIR28: unsupported head: ${h}`);
  }

  function trapezoid28(fn: (t: number) => number, a: number, b: number, n = 10_000): number {
    const h = (b - a) / n;
    let total = 0.5 * (fn(a) + fn(b));
    for (let i = 1; i < n; i++) total += fn(a + i * h);
    return total * h;
  }

  function numericalDerivative28(node: ReturnType<typeof sym>, xVal: number): number {
    const h = 1e-6;
    return (evalIR28(node, xVal + h) - evalIR28(node, xVal - h)) / (2 * h);
  }

  it("Phase 28: ∫ log(x²+1) dx returns a closed form with LOG and ATAN", () => {
    // IBP: R=x, Q′=2x, residual 2x²/(x²+1) = 2 − 2/(x²+1)
    // Result: x·log(x²+1) − 2x + 2·atan(x)
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const xsq = app(POW, [x, int(2)]);
    const integrand = app(LOG, [app(ADD, [xsq, int(1)])]);
    const result = vm.eval(app(INTEGRATE, [integrand, x]));
    // Must be closed (no INTEGRATE head)
    expect(result).not.toMatchObject({ head: INTEGRATE });
    // Must contain both LOG and ATAN (from the partial-fraction residual)
    expect(containsHeadName(result as unknown as ReturnType<typeof sym>, "Log")).toBe(true);
    expect(containsHeadName(result as unknown as ReturnType<typeof sym>, "Atan")).toBe(true);
  });

  it("Phase 28: ∫ log(x²+1) dx numerical correctness", () => {
    // ∫₀¹ log(x²+1) dx  ≈ 0.26338...
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const xsq = app(POW, [x, int(2)]);
    const integrand = app(LOG, [app(ADD, [xsq, int(1)])]);
    const antideriv = vm.eval(app(INTEGRATE, [integrand, x]));

    const diff = evalIR28(antideriv as unknown as ReturnType<typeof sym>, 1)
               - evalIR28(antideriv as unknown as ReturnType<typeof sym>, 0);
    const numerical = trapezoid28(t => Math.log(t ** 2 + 1), 0, 1);
    expect(Math.abs(diff - numerical)).toBeLessThan(1e-5);
  });

  it("Phase 28: ∫ x·log(x²+1) dx returns a closed form with LOG", () => {
    // IBP: R=x²/2, Q′=2x, residual x³/(x²+1) = x − x/(x²+1)
    // Result: (x²/2)·log(x²+1) − x²/2 + ½·log(x²+1)
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const xsq = app(POW, [x, int(2)]);
    const integrand = app(MUL, [x, app(LOG, [app(ADD, [xsq, int(1)])])]);
    const result = vm.eval(app(INTEGRATE, [integrand, x]));
    expect(result).not.toMatchObject({ head: INTEGRATE });
    expect(containsHeadName(result as unknown as ReturnType<typeof sym>, "Log")).toBe(true);
  });

  it("Phase 28: ∫ x·log(x²+1) dx numerical correctness", () => {
    // ∫₁² x·log(x²+1) dx
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const xsq = app(POW, [x, int(2)]);
    const integrand = app(MUL, [x, app(LOG, [app(ADD, [xsq, int(1)])])]);
    const antideriv = vm.eval(app(INTEGRATE, [integrand, x]));

    const diff = evalIR28(antideriv as unknown as ReturnType<typeof sym>, 2)
               - evalIR28(antideriv as unknown as ReturnType<typeof sym>, 1);
    const numerical = trapezoid28(t => t * Math.log(t ** 2 + 1), 1, 2);
    expect(Math.abs(diff - numerical)).toBeLessThan(1e-5);
  });

  it("Phase 28: ∫ x²·log(x²+1) dx numerical correctness", () => {
    // ∫₁² x²·log(x²+1) dx
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const xsq = app(POW, [x, int(2)]);
    const integrand = app(MUL, [xsq, app(LOG, [app(ADD, [xsq, int(1)])])]);
    const antideriv = vm.eval(app(INTEGRATE, [integrand, x]));

    const diff = evalIR28(antideriv as unknown as ReturnType<typeof sym>, 2)
               - evalIR28(antideriv as unknown as ReturnType<typeof sym>, 1);
    const numerical = trapezoid28(t => t ** 2 * Math.log(t ** 2 + 1), 1, 2);
    expect(Math.abs(diff - numerical)).toBeLessThan(1e-5);
  });

  it("Phase 28: ∫ atan(x²) dx stays unevaluated (fallthrough)", () => {
    // The residual 2x²/(1+x⁴) needs irrational partial fractions.
    // The engine must return the unevaluated Integrate, not a wrong answer.
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const xsq = app(POW, [x, int(2)]);
    const integrand = app(ATAN, [xsq]);
    const result = vm.eval(app(INTEGRATE, [integrand, x]));
    // Must still contain INTEGRATE (unevaluated fallthrough).
    expect(containsHeadName(result as unknown as ReturnType<typeof sym>, "Integrate")).toBe(true);
  });

  it("Phase 28: ∫ x·atan(x²) dx returns a closed form with ATAN and LOG", () => {
    // IBP: R=x²/2, Q=x², Q′=2x, denom=1+x⁴
    // residual x³/(1+x⁴) = (1/4)·log(1+x⁴) via Case A (x³ = (1/4)·(4x³))
    // Result: (x²/2)·atan(x²) − (1/4)·log(1+x⁴)
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const xsq = app(POW, [x, int(2)]);
    const integrand = app(MUL, [x, app(ATAN, [xsq])]);
    const result = vm.eval(app(INTEGRATE, [integrand, x]));
    expect(result).not.toMatchObject({ head: INTEGRATE });
    expect(containsHeadName(result as unknown as ReturnType<typeof sym>, "Atan")).toBe(true);
    expect(containsHeadName(result as unknown as ReturnType<typeof sym>, "Log")).toBe(true);
  });

  it("Phase 28: ∫ x·atan(x²) dx numerical correctness", () => {
    // ∫₀¹ x·atan(x²) dx
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const xsq = app(POW, [x, int(2)]);
    const integrand = app(MUL, [x, app(ATAN, [xsq])]);
    const antideriv = vm.eval(app(INTEGRATE, [integrand, x]));

    const diff = evalIR28(antideriv as unknown as ReturnType<typeof sym>, 1)
               - evalIR28(antideriv as unknown as ReturnType<typeof sym>, 0);
    const numerical = trapezoid28(t => t * Math.atan(t ** 2), 0, 1);
    expect(Math.abs(diff - numerical)).toBeLessThan(1e-5);
  });

  it("Phase 28: regression — ∫ log(x) dx still handled by Phase 3", () => {
    // Phase 3 gives x·log(x) − x; Phase 28 must not intercept linear Q.
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const result = vm.eval(app(INTEGRATE, [app(LOG, [x]), x]));
    expect(result).not.toMatchObject({ head: INTEGRATE });
    expect(containsHeadName(result as unknown as ReturnType<typeof sym>, "Log")).toBe(true);
  });

  it("Phase 11: ∫ atan(x) dx returns a closed form with ATAN and LOG", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const result = vm.eval(app(INTEGRATE, [app(ATAN, [x]), x]));
    expect(containsHeadName(result as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    expect(containsHeadName(result as unknown as ReturnType<typeof sym>, "Atan")).toBe(true);
    expect(containsHeadName(result as unknown as ReturnType<typeof sym>, "Log")).toBe(true);
  });

  it("Phase 11: ∫ atan(x+1) dx numerical correctness", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const integrand = app(ATAN, [app(ADD, [x, int(1)])]);
    const antideriv = vm.eval(app(INTEGRATE, [integrand, x]));

    const diff = evalIR28(antideriv as unknown as ReturnType<typeof sym>, 1)
      - evalIR28(antideriv as unknown as ReturnType<typeof sym>, 0);
    const numerical = trapezoid28((t) => Math.atan(t + 1), 0, 1);
    expect(Math.abs(diff - numerical)).toBeLessThan(1e-5);
  });

  it("Phase 11: ∫ x·atan(x) dx numerical correctness", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const integrand = app(MUL, [x, app(ATAN, [x])]);
    const antideriv = vm.eval(app(INTEGRATE, [integrand, x]));

    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    const diff = evalIR28(antideriv as unknown as ReturnType<typeof sym>, 1)
      - evalIR28(antideriv as unknown as ReturnType<typeof sym>, 0);
    const numerical = trapezoid28((t) => t * Math.atan(t), 0, 1);
    expect(Math.abs(diff - numerical)).toBeLessThan(1e-5);
  });

  it("Phase 12: ∫ asin(x) dx closes and matches numerically", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const antideriv = vm.eval(app(INTEGRATE, [app(ASIN, [x]), x]));
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    const diff = evalIR28(antideriv as unknown as ReturnType<typeof sym>, 0.5)
               - evalIR28(antideriv as unknown as ReturnType<typeof sym>, 0);
    const numerical = trapezoid28((t) => Math.asin(t), 0, 0.5);
    expect(Math.abs(diff - numerical)).toBeLessThan(1e-5);
  });

  it("Phase 12: ∫ x·asin(x) dx closes and matches numerically", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const integrand = app(MUL, [x, app(ASIN, [x])]);
    const antideriv = vm.eval(app(INTEGRATE, [integrand, x]));
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    const diff = evalIR28(antideriv as unknown as ReturnType<typeof sym>, 0.5)
               - evalIR28(antideriv as unknown as ReturnType<typeof sym>, 0);
    const numerical = trapezoid28((t) => t * Math.asin(t), 0, 0.5);
    expect(Math.abs(diff - numerical)).toBeLessThan(1e-5);
  });

  it("Phase 12: ∫ x·acos(x) dx closes and includes the asin residual", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const integrand = app(MUL, [x, app(ACOS, [x])]);
    const antideriv = vm.eval(app(INTEGRATE, [integrand, x]));
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Asin")).toBe(true);
    const diff = evalIR28(antideriv as unknown as ReturnType<typeof sym>, 0.5)
               - evalIR28(antideriv as unknown as ReturnType<typeof sym>, 0);
    const numerical = trapezoid28((t) => t * Math.acos(t), 0, 0.5);
    expect(Math.abs(diff - numerical)).toBeLessThan(1e-5);
  });

  it("Phase 12: ∫ asin(2x+1) dx handles linear arguments", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const arg = app(ADD, [app(MUL, [int(2), x]), int(1)]);
    const antideriv = vm.eval(app(INTEGRATE, [app(ASIN, [arg]), x]));
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    const diff = evalIR28(antideriv as unknown as ReturnType<typeof sym>, -0.1)
               - evalIR28(antideriv as unknown as ReturnType<typeof sym>, -0.4);
    const numerical = trapezoid28((t) => Math.asin(2 * t + 1), -0.4, -0.1);
    expect(Math.abs(diff - numerical)).toBeLessThan(1e-5);
  });

  it("Phase 12: ∫ asin(x²) dx stays unevaluated", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const result = vm.eval(app(INTEGRATE, [app(ASIN, [app(POW, [x, int(2)])]), x]));
    expect(containsHeadName(result as unknown as ReturnType<typeof sym>, "Integrate")).toBe(true);
  });

  it("Phase 13: integrates x*sinh(x) and differentiates back", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const integrand = app(MUL, [x, app(SINH, [x])]);
    const antideriv = vm.eval(app(INTEGRATE, [integrand, x]));
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Cosh")).toBe(true);
    for (const xVal of [-0.4, 0.2, 0.8]) {
      const got = numericalDerivative28(antideriv as unknown as ReturnType<typeof sym>, xVal);
      expect(Math.abs(got - xVal * Math.sinh(xVal))).toBeLessThan(1e-5);
    }
  });

  it("Phase 13: integrates x*cosh(x) and differentiates back", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const integrand = app(MUL, [x, app(COSH, [x])]);
    const antideriv = vm.eval(app(INTEGRATE, [integrand, x]));
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Sinh")).toBe(true);
    for (const xVal of [-0.4, 0.2, 0.8]) {
      const got = numericalDerivative28(antideriv as unknown as ReturnType<typeof sym>, xVal);
      expect(Math.abs(got - xVal * Math.cosh(xVal))).toBeLessThan(1e-5);
    }
  });

  it("Phase 13: integrates asinh(x) and differentiates back", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const antideriv = vm.eval(app(INTEGRATE, [app(ASINH, [x]), x]));
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Sqrt")).toBe(true);
    for (const xVal of [-0.4, 0.2, 0.8]) {
      const got = numericalDerivative28(antideriv as unknown as ReturnType<typeof sym>, xVal);
      expect(Math.abs(got - Math.asinh(xVal))).toBeLessThan(1e-5);
    }
  });

  it("Phase 13: integrates x*acosh(x) on the real domain", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const integrand = app(MUL, [x, app(ACOSH, [x])]);
    const antideriv = vm.eval(app(INTEGRATE, [integrand, x]));
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Sqrt")).toBe(true);
    for (const xVal of [1.3, 1.7, 2.2]) {
      const got = numericalDerivative28(antideriv as unknown as ReturnType<typeof sym>, xVal);
      expect(Math.abs(got - xVal * Math.acosh(xVal))).toBeLessThan(1e-5);
    }
  });

  it("Phase 13: integrates tanh(2x+1) as log(cosh)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const arg = app(ADD, [app(MUL, [int(2), x]), int(1)]);
    const antideriv = vm.eval(app(INTEGRATE, [app(TANH, [arg]), x]));
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Log")).toBe(true);
    for (const xVal of [-0.4, 0.1, 0.5]) {
      const got = numericalDerivative28(antideriv as unknown as ReturnType<typeof sym>, xVal);
      expect(Math.abs(got - Math.tanh(2 * xVal + 1))).toBeLessThan(1e-5);
    }
  });

  it("Phase 13: integrates atanh(x/2) and differentiates back", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const arg = app(MUL, [rational(1, 2), x]);
    const antideriv = vm.eval(app(INTEGRATE, [app(ATANH, [arg]), x]));
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Log")).toBe(true);
    for (const xVal of [-0.5, 0.2, 0.7]) {
      const got = numericalDerivative28(antideriv as unknown as ReturnType<typeof sym>, xVal);
      expect(Math.abs(got - Math.atanh(xVal / 2))).toBeLessThan(1e-5);
    }
  });

  it("Phase 13: leaves x*tanh(x) deferred", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const result = vm.eval(app(INTEGRATE, [app(MUL, [x, app(TANH, [x])]), x]));
    expect(containsHeadName(result as unknown as ReturnType<typeof sym>, "Integrate")).toBe(true);
  });

  it("Phase 15: integrates coth(2x+1) as log(sinh)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const arg = app(ADD, [app(MUL, [int(2), x]), int(1)]);
    const antideriv = vm.eval(app(INTEGRATE, [app(COTH, [arg]), x]));
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Log")).toBe(true);
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Sinh")).toBe(true);
    for (const xVal of [0.2, 0.6, 1.0]) {
      const u = 2 * xVal + 1;
      const got = numericalDerivative28(antideriv as unknown as ReturnType<typeof sym>, xVal);
      expect(Math.abs(got - Math.cosh(u) / Math.sinh(u))).toBeLessThan(1e-5);
    }
  });

  it("Phase 15: integrates sech(3x) as atan(sinh)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const arg = app(MUL, [int(3), x]);
    const antideriv = vm.eval(app(INTEGRATE, [app(SECH, [arg]), x]));
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Atan")).toBe(true);
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Sinh")).toBe(true);
    for (const xVal of [-0.4, 0.2, 0.8]) {
      const got = numericalDerivative28(antideriv as unknown as ReturnType<typeof sym>, xVal);
      expect(Math.abs(got - 1 / Math.cosh(3 * xVal))).toBeLessThan(1e-5);
    }
  });

  it("Phase 15: integrates csch(x/2) as log(tanh half-argument)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const arg = app(MUL, [rational(1, 2), x]);
    const antideriv = vm.eval(app(INTEGRATE, [app(CSCH, [arg]), x]));
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Log")).toBe(true);
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Tanh")).toBe(true);
    for (const xVal of [0.6, 1.2, 2.0]) {
      const got = numericalDerivative28(antideriv as unknown as ReturnType<typeof sym>, xVal);
      expect(Math.abs(got - 1 / Math.sinh(xVal / 2))).toBeLessThan(1e-5);
    }
  });

  it("Phase 15: leaves x*coth(x) deferred", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const result = vm.eval(app(INTEGRATE, [app(MUL, [x, app(COTH, [x])]), x]));
    expect(containsHeadName(result as unknown as ReturnType<typeof sym>, "Integrate")).toBe(true);
  });

  it("Phase 16: integrates sech^2(3x) as tanh(3x)/3", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const arg = app(MUL, [int(3), x]);
    const integrand = app(POW, [app(SECH, [arg]), int(2)]);
    const antideriv = vm.eval(app(INTEGRATE, [integrand, x]));
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Tanh")).toBe(true);
    for (const xVal of [-0.4, 0.2, 0.8]) {
      const got = numericalDerivative28(antideriv as unknown as ReturnType<typeof sym>, xVal);
      expect(Math.abs(got - 1 / Math.cosh(3 * xVal) ** 2)).toBeLessThan(1e-5);
    }
  });

  it("Phase 16: integrates sech^3(x) via the odd-power recurrence", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const integrand = app(POW, [app(SECH, [x]), int(3)]);
    const antideriv = vm.eval(app(INTEGRATE, [integrand, x]));
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Atan")).toBe(true);
    for (const xVal of [-0.4, 0.2, 0.8]) {
      const got = numericalDerivative28(antideriv as unknown as ReturnType<typeof sym>, xVal);
      expect(Math.abs(got - 1 / Math.cosh(xVal) ** 3)).toBeLessThan(1e-5);
    }
  });

  it("Phase 16: integrates csch^2(x/2) as -2*coth(x/2)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const arg = app(MUL, [rational(1, 2), x]);
    const integrand = app(POW, [app(CSCH, [arg]), int(2)]);
    const antideriv = vm.eval(app(INTEGRATE, [integrand, x]));
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Coth")).toBe(true);
    for (const xVal of [0.6, 1.2, 2.0]) {
      const got = numericalDerivative28(antideriv as unknown as ReturnType<typeof sym>, xVal);
      expect(Math.abs(got - 1 / Math.sinh(xVal / 2) ** 2)).toBeLessThan(1e-5);
    }
  });

  it("Phase 16: integrates csch^3(x) via the odd-power recurrence", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const integrand = app(POW, [app(CSCH, [x]), int(3)]);
    const antideriv = vm.eval(app(INTEGRATE, [integrand, x]));
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Log")).toBe(true);
    for (const xVal of [0.6, 1.2, 2.0]) {
      const got = numericalDerivative28(antideriv as unknown as ReturnType<typeof sym>, xVal);
      expect(Math.abs(got - 1 / Math.sinh(xVal) ** 3)).toBeLessThan(1e-5);
    }
  });

  it("Phase 16: integrates coth powers through identity reduction", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    for (const power of [2, 3]) {
      const integrand = app(POW, [app(COTH, [x]), int(power)]);
      const antideriv = vm.eval(app(INTEGRATE, [integrand, x]));
      expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
      for (const xVal of [0.6, 1.2, 2.0]) {
        const u = Math.cosh(xVal) / Math.sinh(xVal);
        const got = numericalDerivative28(antideriv as unknown as ReturnType<typeof sym>, xVal);
        expect(Math.abs(got - u ** power)).toBeLessThan(1e-5);
      }
    }
  });

  it("Phase 16: leaves sech^2(x^2) deferred", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const integrand = app(POW, [app(SECH, [app(POW, [x, int(2)])]), int(2)]);
    const result = vm.eval(app(INTEGRATE, [integrand, x]));
    expect(containsHeadName(result as unknown as ReturnType<typeof sym>, "Integrate")).toBe(true);
  });

  it("Phase 17: integrates tanh powers through identity reduction", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    for (const power of [2, 3, 4]) {
      const integrand = app(POW, [app(TANH, [x]), int(power)]);
      const antideriv = vm.eval(app(INTEGRATE, [integrand, x]));
      expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
      for (const xVal of [-0.4, 0.2, 0.8]) {
        const got = numericalDerivative28(antideriv as unknown as ReturnType<typeof sym>, xVal);
        expect(Math.abs(got - Math.tanh(xVal) ** power)).toBeLessThan(1e-5);
      }
    }
  });

  it("Phase 17: integrates tanh^2(2x+1) with the chain-rule factor", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const arg = app(ADD, [app(MUL, [int(2), x]), int(1)]);
    const integrand = app(POW, [app(TANH, [arg]), int(2)]);
    const antideriv = vm.eval(app(INTEGRATE, [integrand, x]));
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Integrate")).toBe(false);
    expect(containsHeadName(antideriv as unknown as ReturnType<typeof sym>, "Tanh")).toBe(true);
    for (const xVal of [-0.4, 0.1, 0.5]) {
      const got = numericalDerivative28(antideriv as unknown as ReturnType<typeof sym>, xVal);
      expect(Math.abs(got - Math.tanh(2 * xVal + 1) ** 2)).toBeLessThan(1e-5);
    }
  });

  it("Phase 17: leaves tanh^2(x^2) deferred", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const integrand = app(POW, [app(TANH, [app(POW, [x, int(2)])]), int(2)]);
    const result = vm.eval(app(INTEGRATE, [integrand, x]));
    expect(containsHeadName(result as unknown as ReturnType<typeof sym>, "Integrate")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Phase 29: Abs and Sqrt algebraic rules
// ---------------------------------------------------------------------------
describe("Phase 29: Abs and Sqrt algebraic rules", () => {
  it("Abs: numeric fold and exact values", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(sym("Abs"), [int(-3)]))).toEqual(int(3));
    expect(vm.eval(app(sym("Abs"), [int(5)]))).toEqual(int(5));
    expect(vm.eval(app(sym("Abs"), [rational(-3n, 4n)]))).toEqual(rational(3n, 4n));
  });

  it("Abs: idempotency — Abs(Abs(x)) = Abs(x)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const absX = app(sym("Abs"), [x]);
    expect(vm.eval(app(sym("Abs"), [absX]))).toEqual(absX);
  });

  it("Abs: strips Neg — Abs(-x) = Abs(x)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(sym("Abs"), [app(NEG, [x])]))).toEqual(app(sym("Abs"), [x]));
  });

  it("Abs: even power — Abs(x^2) = x^2, Abs(x^4) = x^4", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const x2 = app(POW, [x, int(2)]);
    const x4 = app(POW, [x, int(4)]);
    expect(vm.eval(app(sym("Abs"), [x2]))).toEqual(x2);
    expect(vm.eval(app(sym("Abs"), [x4]))).toEqual(x4);
  });

  it("Sqrt: numeric fold with perfect-square detection", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(SQRT, [int(0)]))).toEqual(int(0));
    expect(vm.eval(app(SQRT, [int(1)]))).toEqual(int(1));
    expect(vm.eval(app(SQRT, [int(4)]))).toEqual(int(2));
    expect(vm.eval(app(SQRT, [int(9)]))).toEqual(int(3));
    expect(vm.eval(app(SQRT, [int(16)]))).toEqual(int(4));
    expect(vm.eval(app(SQRT, [int(25)]))).toEqual(int(5));
  });

  it("Sqrt: sqrt(x^2) = Abs(x)  — k=1 odd", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(SQRT, [app(POW, [x, int(2)])]))).toEqual(app(sym("Abs"), [x]));
  });

  it("Sqrt: sqrt(x^4) = x^2  — k=2 even", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(SQRT, [app(POW, [x, int(4)])]))).toEqual(app(POW, [x, int(2)]));
  });

  it("Sqrt: sqrt(x^6) = Abs(x^3)  — k=3 odd", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(SQRT, [app(POW, [x, int(6)])]))).toEqual(
      app(sym("Abs"), [app(POW, [x, int(3)])]),
    );
  });

  it("Sqrt: sqrt(x^8) = x^4  — k=4 even", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(SQRT, [app(POW, [x, int(8)])]))).toEqual(app(POW, [x, int(4)]));
  });
});

// ---------------------------------------------------------------------------
// Phase 30: Log and Exp cancellation rules
// ---------------------------------------------------------------------------
describe("Phase 30: Log and Exp cancellation rules", () => {
  it("Log: special value log(1) = 0", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(LOG, [int(1)]))).toEqual(int(0));
  });

  it("Log: log(exp(x)) = x  (cancellation)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(LOG, [app(EXP, [x])]))).toEqual(x);
  });

  it("Log: log(exp(2·x)) = 2·x  (cancellation through product)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    const twoX = app(MUL, [int(2), x]);
    expect(vm.eval(app(LOG, [app(EXP, [twoX])]))).toEqual(twoX);
  });

  it("Exp: special value exp(0) = 1", () => {
    const vm = new VM(new SymbolicBackend());
    expect(vm.eval(app(EXP, [int(0)]))).toEqual(int(1));
  });

  it("Exp: exp(log(x)) = x  (cancellation)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(EXP, [app(LOG, [x])]))).toEqual(x);
  });

  it("Exp: exp(2·log(x)) = x^2  (power form, Mul(n, log(x)))", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(EXP, [app(MUL, [int(2), app(LOG, [x])])]))).toEqual(
      app(POW, [x, int(2)]),
    );
  });

  it("Exp: exp(log(x)·3) = x^3  (power form, Mul(log(x), n))", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(EXP, [app(MUL, [app(LOG, [x]), int(3)])]))).toEqual(
      app(POW, [x, int(3)]),
    );
  });
});

// ---------------------------------------------------------------------------
// Phase 31: Trig and hyperbolic symmetry + arc-cancellation
// ---------------------------------------------------------------------------
describe("Phase 31: Trig symmetry and arc-cancellation", () => {
  it("Sin: odd symmetry — sin(-x) = -sin(x)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(SIN, [app(NEG, [x])]))).toEqual(app(NEG, [app(SIN, [x])]));
  });

  it("Sin: arc-cancellation — sin(asin(x)) = x", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(SIN, [app(ASIN, [x])]))).toEqual(x);
  });

  it("Cos: even symmetry — cos(-x) = cos(x)  (NEG stripped)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(COS, [app(NEG, [x])]))).toEqual(app(COS, [x]));
  });

  it("Cos: arc-cancellation — cos(acos(x)) = x", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(COS, [app(ACOS, [x])]))).toEqual(x);
  });

  it("Tan: odd symmetry — tan(-x) = -tan(x)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(TAN, [app(NEG, [x])]))).toEqual(app(NEG, [app(TAN, [x])]));
  });

  it("Tan: arc-cancellation — tan(atan(x)) = x", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(TAN, [app(ATAN, [x])]))).toEqual(x);
  });

  it("Sinh: odd symmetry — sinh(-x) = -sinh(x)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(SINH, [app(NEG, [x])]))).toEqual(app(NEG, [app(SINH, [x])]));
  });

  it("Sinh: arc-cancellation — sinh(asinh(x)) = x", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(SINH, [app(ASINH, [x])]))).toEqual(x);
  });

  it("Cosh: even symmetry — cosh(-x) = cosh(x)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(COSH, [app(NEG, [x])]))).toEqual(app(COSH, [x]));
  });

  it("Cosh: arc-cancellation — cosh(acosh(x)) = x", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(COSH, [app(ACOSH, [x])]))).toEqual(x);
  });

  it("Tanh: odd symmetry — tanh(-x) = -tanh(x)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(TANH, [app(NEG, [x])]))).toEqual(app(NEG, [app(TANH, [x])]));
  });

  it("Tanh: arc-cancellation — tanh(atanh(x)) = x", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(TANH, [app(ATANH, [x])]))).toEqual(x);
  });
});

// ---------------------------------------------------------------------------
// Phase 32: Inverse trig/hyperbolic odd symmetry and acos reflection
// ---------------------------------------------------------------------------
describe("Phase 32: Inverse trig symmetry", () => {
  it("Asin: odd symmetry — asin(-x) = -asin(x)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(ASIN, [app(NEG, [x])]))).toEqual(app(NEG, [app(ASIN, [x])]));
  });

  it("Acos: reflection — acos(-x) = %pi - acos(x)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(ACOS, [app(NEG, [x])]))).toEqual(
      app(SUB, [sym("%pi"), app(ACOS, [x])]),
    );
  });

  it("Atan: odd symmetry — atan(-x) = -atan(x)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(ATAN, [app(NEG, [x])]))).toEqual(app(NEG, [app(ATAN, [x])]));
  });

  it("Asinh: odd symmetry — asinh(-x) = -asinh(x)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(ASINH, [app(NEG, [x])]))).toEqual(app(NEG, [app(ASINH, [x])]));
  });

  it("Atanh: odd symmetry — atanh(-x) = -atanh(x)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    expect(vm.eval(app(ATANH, [app(NEG, [x])]))).toEqual(app(NEG, [app(ATANH, [x])]));
  });

  it("Double-neg collapses: asin(-(-x)) = asin(x)", () => {
    const vm = new VM(new SymbolicBackend());
    const x = sym("x");
    // asin(NEG(NEG(x))): NEG(x) evaluated → since NEG(NEG(x)) collapses to x,
    // asin arg becomes x.
    expect(vm.eval(app(ASIN, [app(NEG, [app(NEG, [x])])]))).toEqual(app(ASIN, [x]));
  });
});

// ---------------------------------------------------------------------------
// Phase 33: Trig exact values at rational multiples of π
// ---------------------------------------------------------------------------
describe("Phase 33: Trig π-multiple exact values", () => {
  const vm = new VM(new SymbolicBackend());
  const pi = sym("%pi");

  it("sin(%pi) = 0", () => {
    expect(vm.eval(app(SIN, [pi]))).toEqual(int(0));
  });
  it("sin(%pi/6) = 1/2", () => {
    expect(vm.eval(app(SIN, [app(DIV, [pi, int(6)])]))).toEqual(rational(1n, 2n));
  });
  it("sin(%pi/4) = √2/2", () => {
    expect(vm.eval(app(SIN, [app(DIV, [pi, int(4)])]))).toEqual(
      app(DIV, [app(SQRT, [int(2)]), int(2)]),
    );
  });
  it("sin(%pi/3) = √3/2", () => {
    expect(vm.eval(app(SIN, [app(DIV, [pi, int(3)])]))).toEqual(
      app(DIV, [app(SQRT, [int(3)]), int(2)]),
    );
  });
  it("sin(%pi/2) = 1", () => {
    expect(vm.eval(app(SIN, [app(DIV, [pi, int(2)])]))).toEqual(int(1));
  });
  it("sin(2·%pi) = 0", () => {
    expect(vm.eval(app(SIN, [app(MUL, [int(2), pi])]))).toEqual(int(0));
  });
  it("sin(3·%pi/2) = -1", () => {
    expect(vm.eval(app(SIN, [app(DIV, [app(MUL, [int(3), pi]), int(2)])]))).toEqual(int(-1));
  });

  it("cos(%pi) = -1", () => {
    expect(vm.eval(app(COS, [pi]))).toEqual(int(-1));
  });
  it("cos(%pi/2) = 0", () => {
    expect(vm.eval(app(COS, [app(DIV, [pi, int(2)])]))).toEqual(int(0));
  });
  it("cos(%pi/3) = 1/2", () => {
    expect(vm.eval(app(COS, [app(DIV, [pi, int(3)])]))).toEqual(rational(1n, 2n));
  });
  it("cos(%pi/4) = √2/2", () => {
    expect(vm.eval(app(COS, [app(DIV, [pi, int(4)])]))).toEqual(
      app(DIV, [app(SQRT, [int(2)]), int(2)]),
    );
  });
  it("cos(2·%pi) = 1", () => {
    expect(vm.eval(app(COS, [app(MUL, [int(2), pi])]))).toEqual(int(1));
  });

  it("tan(%pi) = 0", () => {
    expect(vm.eval(app(TAN, [pi]))).toEqual(int(0));
  });
  it("tan(%pi/4) = 1", () => {
    expect(vm.eval(app(TAN, [app(DIV, [pi, int(4)])]))).toEqual(int(1));
  });
  it("tan(%pi/3) = √3", () => {
    expect(vm.eval(app(TAN, [app(DIV, [pi, int(3)])]))).toEqual(app(SQRT, [int(3)]));
  });
  it("tan(3·%pi/4) = -1", () => {
    expect(vm.eval(app(TAN, [app(DIV, [app(MUL, [int(3), pi]), int(4)])]))).toEqual(int(-1));
  });
  it("tan(%pi/2) stays unevaluated (undefined)", () => {
    // tan(π/2) is undefined; the handler should leave it unevaluated.
    const result = vm.eval(app(TAN, [app(DIV, [pi, int(2)])]));
    expect(result.kind).toBe("apply");
    if (result.kind === "apply") {
      expect(headName(result.head)).toBe("Tan");
    }
  });

  it("sin(-(%pi/6)) = -1/2  (odd symmetry via negative q)", () => {
    // sin(-(π/6)) — tryPiMultiple detects NEG(Div(%pi, 6)) → q = -1/6
    // (-1/6) mod 2 = 11/6 → sin(11π/6) = -1/2
    expect(vm.eval(app(SIN, [app(NEG, [app(DIV, [pi, int(6)])])]))).toEqual(rational(-1n, 2n));
  });

  it("cos(-(%pi/3)) = 1/2  (even symmetry via negative q mod 2)", () => {
    // cos(-(π/3)) — q = -1/3, (-1/3) mod 2 = 5/3 → cos(5π/3) = 1/2
    expect(vm.eval(app(COS, [app(NEG, [app(DIV, [pi, int(3)])])]))).toEqual(rational(1n, 2n));
  });

  it("regression: numeric sin/cos/tan still work", () => {
    expect(vm.eval(app(SIN, [int(0)]))).toEqual(int(0));
    expect(vm.eval(app(COS, [int(0)]))).toEqual(int(1));
    expect(vm.eval(app(TAN, [int(0)]))).toEqual(int(0));
  });
});

// ---------------------------------------------------------------------------
// Phase 47 (TypeScript port): Nested-Add flattening.
//
// Ports the Python Phase 47 fix.  When either binary Add operand is itself
// an Add(...) apply, the handler flattens the tree, sums numeric literals
// once, and rebuilds a left-associated chain.  Makes Add canonical for
// any consumer comparing trees structurally.
// ---------------------------------------------------------------------------

describe("Phase 47: nested-Add flattening", () => {
  const vm = new VM(new SymbolicBackend());
  const k = sym("k");

  it("Add(Add(k, 1), 1) flattens to Add(k, 2)", () => {
    const nested = app(ADD, [app(ADD, [k, int(1)]), int(1)]);
    const out = vm.eval(nested);
    expect(out).toEqual(app(ADD, [k, int(2)]));
  });

  it("triply-nested Add(Add(Add(k, 1), 1), 1) → Add(k, 3)", () => {
    const nested = app(ADD, [
      app(ADD, [app(ADD, [k, int(1)]), int(1)]),
      int(1),
    ]);
    const out = vm.eval(nested);
    expect(out).toEqual(app(ADD, [k, int(3)]));
  });

  it("Add(Add(k, 2), 3) folds the constants", () => {
    const nested = app(ADD, [app(ADD, [k, int(2)]), int(3)]);
    const out = vm.eval(nested);
    expect(out).toEqual(app(ADD, [k, int(5)]));
  });

  it("Add(Add(k, 1), -1) cancels the constants → bare k", () => {
    const nested = app(ADD, [app(ADD, [k, int(1)]), int(-1)]);
    const out = vm.eval(nested);
    expect(out).toEqual(k);
  });

  it("Add(Add(x, y), z) leaves order alone when no literals present", () => {
    const x = sym("x");
    const y = sym("y");
    const z = sym("z");
    const nested = app(ADD, [app(ADD, [x, y]), z]);
    const out = vm.eval(nested);
    // Should rebuild as left-associated Add(Add(x, y), z) with same args.
    expect(out).toEqual(app(ADD, [app(ADD, [x, y]), z]));
  });

  it("non-nested Add(k, 1) is untouched (no rebuild)", () => {
    const flat = app(ADD, [k, int(1)]);
    const out = vm.eval(flat);
    expect(out).toEqual(flat);
  });

  it("regression: Add(0, x) still simplifies to x", () => {
    const x = sym("x");
    expect(vm.eval(app(ADD, [int(0), x]))).toEqual(x);
  });
});
