import { describe, expect, it } from "vitest";
import {
  ACOSH,
  ADD,
  ASINH,
  ASSIGN,
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
    expect(vm.eval(app(D, [app(POW, [sym("x"), sym("x")]), sym("x")]))).toEqual(
      app(MUL, [
        app(EXP, [app(MUL, [sym("x"), app(LOG, [sym("x")])])]),
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
});
