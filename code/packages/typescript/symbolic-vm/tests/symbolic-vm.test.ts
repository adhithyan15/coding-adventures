import { describe, expect, it } from "vitest";
import {
  ACOSH,
  ADD,
  ASINH,
  ASSIGN,
  ATANH,
  COS,
  COSH,
  D,
  DEFINE,
  DIV,
  EQUAL,
  EXP,
  FALSE,
  IF,
  LIST,
  LOG,
  MUL,
  NEG,
  POW,
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
import { StrictBackend, StrictEvaluationError, SymbolicBackend, VM } from "../src/index.js";

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

  it("leaves unknown dependent derivatives unevaluated", () => {
    const vm = new VM(new SymbolicBackend());
    const expr = app(D, [app(sym("F"), [sym("x")]), sym("x")]);
    expect(vm.eval(expr)).toEqual(expr);
  });
});
