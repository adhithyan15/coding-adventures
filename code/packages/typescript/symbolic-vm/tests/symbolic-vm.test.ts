import { describe, expect, it } from "vitest";
import {
  ADD,
  ASSIGN,
  DEFINE,
  DIV,
  EQUAL,
  FALSE,
  IF,
  LIST,
  MUL,
  POW,
  SIN,
  TRUE,
  app,
  equals,
  int,
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
});
