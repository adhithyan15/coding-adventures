import { describe, expect, it } from "vitest";
import {
  ADD,
  DIV,
  EXP,
  MUL,
  NEG,
  POW,
  PRODUCT,
  SUB,
  SUM,
  app,
  int,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";
import {
  GAMMA_FUNC,
  evaluateProduct,
  evaluateProductExpr,
  evaluateSum,
  faulhaberIr,
  geometricSumIr,
  polySumIr,
  rationalValue,
  trySpecialInfinite,
  type RationalValue,
} from "../src/index";

function evalNode(node: IRNode): IRNode {
  if (node.kind !== "apply") return node;
  const head = node.head;
  const args = node.args.map(evalNode);
  if (head.kind !== "symbol") return app(head, args);
  const out = (() => {
    switch (head.name) {
      case ADD.name:
        return fold(args, { numer: 0n, denom: 1n }, addR);
      case SUB.name:
        return args.length === 2 ? binary(args[0], args[1], subR) : undefined;
      case MUL.name:
        return fold(args, { numer: 1n, denom: 1n }, mulR);
      case DIV.name:
        return args.length === 2 ? binary(args[0], args[1], divR) : undefined;
      case POW.name:
        return powNode(args[0], args[1]);
      case NEG.name: {
        const value = rationalValue(args[0]);
        return value === undefined ? undefined : rationalToIr({ numer: -value.numer, denom: value.denom });
      }
      default:
        return undefined;
    }
  })();
  return out ?? app(head, args);
}

function fold(args: readonly IRNode[], init: RationalValue, op: (a: RationalValue, b: RationalValue) => RationalValue): IRNode | undefined {
  let acc = init;
  for (const arg of args) {
    const value = rationalValue(arg);
    if (value === undefined) return undefined;
    acc = op(acc, value);
  }
  return rationalToIr(acc);
}

function binary(a: IRNode, b: IRNode, op: (a: RationalValue, b: RationalValue) => RationalValue): IRNode | undefined {
  const av = rationalValue(a);
  const bv = rationalValue(b);
  return av === undefined || bv === undefined ? undefined : rationalToIr(op(av, bv));
}

function powNode(a: IRNode, b: IRNode): IRNode | undefined {
  const base = rationalValue(a);
  if (base === undefined || b.kind !== "integer" || b.value < 0n) return undefined;
  let out = { numer: 1n, denom: 1n };
  for (let i = 0n; i < b.value; i += 1n) out = mulR(out, base);
  return rationalToIr(out);
}

function rationalToIr(value: RationalValue): IRNode {
  return value.denom === 1n ? int(value.numer) : rational(value.numer, value.denom);
}

function addR(a: RationalValue, b: RationalValue): RationalValue {
  return reduce({ numer: a.numer * b.denom + b.numer * a.denom, denom: a.denom * b.denom });
}

function subR(a: RationalValue, b: RationalValue): RationalValue {
  return reduce({ numer: a.numer * b.denom - b.numer * a.denom, denom: a.denom * b.denom });
}

function mulR(a: RationalValue, b: RationalValue): RationalValue {
  return reduce({ numer: a.numer * b.numer, denom: a.denom * b.denom });
}

function divR(a: RationalValue, b: RationalValue): RationalValue {
  return reduce({ numer: a.numer * b.denom, denom: a.denom * b.numer });
}

function reduce(value: RationalValue): RationalValue {
  let n = value.numer;
  let d = value.denom;
  if (d < 0n) {
    n = -n;
    d = -d;
  }
  const g = gcd(n < 0n ? -n : n, d);
  return { numer: n / g, denom: d / g };
}

function gcd(a: bigint, b: bigint): bigint {
  let x = a;
  let y = b;
  while (y !== 0n) {
    const t = y;
    y = x % y;
    x = t;
  }
  return x === 0n ? 1n : x;
}

describe("summation", () => {
  it("evaluates constant and geometric sums", () => {
    const k = sym("k");
    expect(evaluateSum(int(5), k, int(1), int(10), evalNode)).toEqual(int(50));
    expect(evaluateSum(app(POW, [rational(1, 2), k]), k, int(0), sym("%inf"), evalNode)).toEqual(int(2));
  });

  it("builds finite and infinite geometric sums", () => {
    expect(evalNode(geometricSumIr(int(1), int(3), int(0), int(3), false))).toEqual(int(40));
    expect(evalNode(geometricSumIr(int(1), rational(1, 4), int(2), undefined, true))).toEqual(rational(1, 12));
  });

  it("covers Faulhaber and polynomial power sums", () => {
    const expected = [4, 10, 30, 100, 354, 1300];
    for (const [m, value] of expected.entries()) {
      expect(evalNode(faulhaberIr(m, int(4)) ?? sym("bad"))).toEqual(int(value));
    }
    expect(faulhaberIr(6, int(4))).toBeUndefined();
    expect(evalNode(polySumIr(2, { numer: 1n, denom: 1n }, 1n, int(4)) ?? sym("bad"))).toEqual(int(30));
    expect(evalNode(polySumIr(0, { numer: 1n, denom: 1n }, 0n, int(4)) ?? sym("bad"))).toEqual(int(5));
    expect(evaluateSum(app(MUL, [int(3), sym("k")]), sym("k"), int(1), int(4), evalNode)).toEqual(int(30));
  });

  it("recognises classic infinite series", () => {
    const k = sym("k");
    const x = sym("x");
    expect(trySpecialInfinite(app(DIV, [int(1), app(POW, [k, int(2)])]), k, int(1))).toEqual(
      app(DIV, [app(POW, [sym("%pi"), int(2)]), int(6)]),
    );
    const gamma = app(GAMMA_FUNC, [app(ADD, [k, int(1)])]);
    expect(trySpecialInfinite(app(DIV, [int(1), gamma]), k, int(0))).toEqual(sym("%e"));
    expect(trySpecialInfinite(app(DIV, [app(POW, [x, k]), gamma]), k, int(0))).toEqual(app(EXP, [x]));
  });

  it("evaluates products and preserves fallback nodes", () => {
    const k = sym("k");
    const n = sym("n");
    expect(evaluateProduct(k, k, int(1), n, evalNode)).toEqual(app(GAMMA_FUNC, [app(ADD, [n, int(1)])]));
    expect(evaluateProduct(int(2), k, int(0), int(4), evalNode)).toEqual(int(32));
    expect(evaluateProduct(app(MUL, [int(2), k]), k, int(1), n, evalNode)).toEqual(
      app(MUL, [app(POW, [int(2), n]), app(GAMMA_FUNC, [app(ADD, [n, int(1)])])]),
    );
    expect(evaluateProductExpr(k, k, int(0), n)).toBeUndefined();
    const fallback = evaluateProduct(app(POW, [k, int(3)]), k, int(1), n, evalNode);
    expect(fallback.kind).toBe("apply");
    expect(fallback.kind === "apply" ? fallback.head : undefined).toEqual(PRODUCT);
  });

  it("falls back for unknown sums", () => {
    const k = sym("k");
    const out = evaluateSum(app(sym("Sin"), [k]), k, int(1), sym("n"), evalNode);
    expect(out.kind).toBe("apply");
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });
});
