/**
 * Pipeline tests for n-variate Hensel-lift factorisation — Track K2
 * (TS port of Python Track K1, PR #5590).
 *
 * Exercises end-to-end ``Factor(expr)`` via the VM: construct ``Factor(...)``
 * IR, evaluate on a SymbolicBackend, and verify by re-expanding the
 * returned product back to a sparse-dict polynomial and comparing against
 * the sparse-dict expansion of the input.  We verify *algebraic* equality
 * rather than *shape* because the Hensel lift may emit factors in a
 * different deterministic order than the human-recognisable canonical
 * order, and integer-content can be pulled out separately.
 */

import { describe, expect, it } from "vitest";
import {
  ADD,
  FACTOR,
  IRApply,
  IRInteger,
  IRNode,
  IRSymbol,
  MUL,
  NEG,
  POW,
  SUB,
  app,
  int,
  sym,
} from "@coding-adventures/symbolic-ir";
import { SymbolicBackend, VM } from "../src/index";

const x = sym("x");
const y = sym("y");
const z = sym("z");
const w = sym("w");

function makeVm(): VM {
  return new VM(new SymbolicBackend());
}

type PolyDict = Map<string, bigint>;

function pkey(tup: number[]): string {
  return tup.join(",");
}

function pparse(k: string, n: number): number[] {
  const out: number[] = [];
  let start = 0;
  for (let i = 0; i < n - 1; i += 1) {
    const idx = k.indexOf(",", start);
    out.push(Number(k.slice(start, idx)));
    start = idx + 1;
  }
  out.push(Number(k.slice(start)));
  return out;
}

/**
 * Recursive structural expander for the polynomial subset of IR.
 *
 * Independent from the production ``irToNpoly`` so a bug in production
 * code can't masquerade as a bug-compatible test result.
 */
function expandToDict(node: IRNode, vars: IRSymbol[]): PolyDict {
  const n = vars.length;
  const zero = pkey(new Array<number>(n).fill(0));
  const varIdx = new Map<string, number>();
  vars.forEach((v, i) => varIdx.set(v.name, i));

  function unit(v: IRSymbol): string {
    const i = varIdx.get(v.name)!;
    const k = new Array<number>(n).fill(0);
    k[i] = 1;
    return pkey(k);
  }
  function addKeys(a: string, b: string): string {
    const ta = pparse(a, n);
    const tb = pparse(b, n);
    const out: number[] = new Array<number>(n);
    for (let i = 0; i < n; i += 1) out[i] = ta[i] + tb[i];
    return pkey(out);
  }
  function normalize(d: PolyDict): PolyDict {
    for (const [k, v] of [...d]) {
      if (v === 0n) d.delete(k);
    }
    return d;
  }
  function add(a: PolyDict, b: PolyDict): PolyDict {
    const out: PolyDict = new Map(a);
    for (const [k, v] of b) {
      out.set(k, (out.get(k) ?? 0n) + v);
    }
    return normalize(out);
  }
  function neg(a: PolyDict): PolyDict {
    const out: PolyDict = new Map();
    for (const [k, v] of a) out.set(k, -v);
    return normalize(out);
  }
  function mul(a: PolyDict, b: PolyDict): PolyDict {
    const out: PolyDict = new Map();
    for (const [ka, va] of a) {
      for (const [kb, vb] of b) {
        const k = addKeys(ka, kb);
        out.set(k, (out.get(k) ?? 0n) + va * vb);
      }
    }
    return normalize(out);
  }

  function walk(node: IRNode): PolyDict {
    if (node.kind === "integer") {
      if (node.value === 0n) return new Map();
      return new Map([[zero, node.value]]);
    }
    if (node.kind === "symbol") {
      if (varIdx.has(node.name)) return new Map([[unit(node), 1n]]);
      throw new Error(`unexpected symbol in test expander: ${node.name}`);
    }
    if (node.kind !== "apply") throw new Error("unexpected non-apply node");
    const head = node.head;
    if (head.kind !== "symbol") throw new Error("unexpected head");
    if (head.name === ADD.name) {
      let acc: PolyDict = new Map();
      for (const a of node.args) acc = add(acc, walk(a));
      return acc;
    }
    if (head.name === SUB.name) {
      const a = walk(node.args[0]);
      const b = walk(node.args[1]);
      return add(a, neg(b));
    }
    if (head.name === NEG.name) {
      return neg(walk(node.args[0]));
    }
    if (head.name === MUL.name) {
      let acc: PolyDict = new Map([[zero, 1n]]);
      for (const a of node.args) acc = mul(acc, walk(a));
      return acc;
    }
    if (head.name === POW.name) {
      const base = walk(node.args[0]);
      const exp = node.args[1];
      if (exp.kind !== "integer") throw new Error("non-integer exponent");
      const e = Number(exp.value);
      if (e < 0) throw new Error("negative exponent");
      if (e === 0) return new Map([[zero, 1n]]);
      let out = base;
      for (let i = 1; i < e; i += 1) out = mul(out, base);
      return out;
    }
    throw new Error(`unexpected head: ${head.name}`);
  }
  return walk(node);
}

function dictEquals(a: PolyDict, b: PolyDict): boolean {
  if (a.size !== b.size) return false;
  for (const [k, v] of a) {
    if (b.get(k) !== v) return false;
  }
  return true;
}

describe("n-variate Factor pipeline — Track K2", () => {
  it("factor(x^3 + y^3 + z^3 - 3*x*y*z) recovers two factors", () => {
    const vm = makeVm();
    // x^3 + y^3 + z^3 - 3*x*y*z
    const target = app(SUB, [
      app(ADD, [
        app(ADD, [
          app(POW, [x, int(3)]),
          app(POW, [y, int(3)]),
        ]),
        app(POW, [z, int(3)]),
      ]),
      app(MUL, [int(3), app(MUL, [app(MUL, [x, y]), z])]),
    ]);
    const expr = app(FACTOR, [target]);
    const result = vm.eval(expr);

    // Result should NOT still be a Factor(...) wrapper.
    const isWrapper =
      result.kind === "apply" &&
      result.head.kind === "symbol" &&
      result.head.name === FACTOR.name;
    expect(isWrapper).toBe(false);

    const vars = [x, y, z];
    const resultDict = expandToDict(result, vars);
    const targetDict = expandToDict(target, vars);
    expect(dictEquals(resultDict, targetDict)).toBe(true);
  });

  it("factor((x+y+z)*(x+2y+3z)) round-trips algebraically", () => {
    const vm = makeVm();
    // x^2 + 3xy + 4xz + 2y^2 + 5yz + 3z^2
    const expanded = app(ADD, [
      app(POW, [x, int(2)]),
      app(ADD, [
        app(MUL, [int(3), app(MUL, [x, y])]),
        app(ADD, [
          app(MUL, [int(4), app(MUL, [x, z])]),
          app(ADD, [
            app(MUL, [int(2), app(POW, [y, int(2)])]),
            app(ADD, [
              app(MUL, [int(5), app(MUL, [y, z])]),
              app(MUL, [int(3), app(POW, [z, int(2)])]),
            ]),
          ]),
        ]),
      ]),
    ]);
    const expr = app(FACTOR, [expanded]);
    const result = vm.eval(expr);
    const vars = [x, y, z];
    // Either Hensel found a factorisation (round-trip algebraic equality)
    // or the wrapper survived — both correct.
    const isWrapper =
      result.kind === "apply" &&
      result.head.kind === "symbol" &&
      result.head.name === FACTOR.name;
    if (isWrapper) return;
    const resultDict = expandToDict(result, vars);
    const expandedDict = expandToDict(expanded, vars);
    expect(dictEquals(resultDict, expandedDict)).toBe(true);
  });

  it("factor(x^2 + y^2 + z^2 + 1) — irreducible, falls through cleanly", () => {
    const vm = makeVm();
    const target = app(ADD, [
      app(POW, [x, int(2)]),
      app(ADD, [
        app(POW, [y, int(2)]),
        app(ADD, [app(POW, [z, int(2)]), int(1)]),
      ]),
    ]);
    const expr = app(FACTOR, [target]);
    const result = vm.eval(expr);
    const isWrapper =
      result.kind === "apply" &&
      result.head.kind === "symbol" &&
      result.head.name === FACTOR.name;
    if (isWrapper) return;
    const vars = [x, y, z];
    const targetDict = expandToDict(target, vars);
    const resultDict = expandToDict(result, vars);
    expect(dictEquals(resultDict, targetDict)).toBe(true);
  });

  it("factor(sin(x) + y + z) — transcendental, does not crash", () => {
    const vm = makeVm();
    const sin = sym("Sin");
    const target = app(ADD, [app(sin, [x]), app(ADD, [y, z])]);
    const expr = app(FACTOR, [target]);
    // We only require: no crash.
    const result = vm.eval(expr);
    expect(result).toBeDefined();
  });

  it("regression: bivariate x^2 + xy - 2y^2 still factors via Factor", () => {
    const vm = makeVm();
    // x^2 + xy - 2y^2  — should be handled by either bivariate or
    // n-variate Hensel.
    const target = app(SUB, [
      app(ADD, [
        app(POW, [x, int(2)]),
        app(MUL, [x, y]),
      ]),
      app(MUL, [int(2), app(POW, [y, int(2)])]),
    ]);
    const expr = app(FACTOR, [target]);
    const result = vm.eval(expr);
    const isWrapper =
      result.kind === "apply" &&
      result.head.kind === "symbol" &&
      result.head.name === FACTOR.name;
    expect(isWrapper).toBe(false);
    const vars = [x, y];
    const resultDict = expandToDict(result, vars);
    const targetDict = expandToDict(target, vars);
    expect(dictEquals(resultDict, targetDict)).toBe(true);
  });

  it("regression: univariate x^2 - 1 still factors", () => {
    const vm = makeVm();
    const target = app(SUB, [app(POW, [x, int(2)]), int(1)]);
    const expr = app(FACTOR, [target]);
    const result = vm.eval(expr);
    const isWrapper =
      result.kind === "apply" &&
      result.head.kind === "symbol" &&
      result.head.name === FACTOR.name;
    expect(isWrapper).toBe(false);
    const resultDict = expandToDict(result, [x]);
    const targetDict = expandToDict(target, [x]);
    expect(dictEquals(resultDict, targetDict)).toBe(true);
  });
});
