import { describe, it } from "node:test";
import assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import {
  tokenizeWarrant,
  parseWarrant,
  evaluateWarrantExpr,
  executeWarrantModule,
  areWarrantValuesEqual,
} from "../dist/index.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

function expect<T>(actual: T) {
  return {
    toBe(expected: T): void {
      assert.strictEqual(actual, expected);
    },
    toEqual(expected: any): void {
      assert.deepStrictEqual(actual, expected);
    },
    toBeDefined(): void {
      assert.notStrictEqual(actual, undefined);
    },
    toThrow(): void {
      assert.throws(actual as any);
    },
  };
}

// Real TypeScript implementation of ct-compare operations
const ctCompareHost = {
  eq: (a: Uint8Array, b: Uint8Array): boolean => {
    if (a.length !== b.length) return false;
    let diff = 0;
    for (let i = 0; i < a.length; i++) {
      diff |= a[i] ^ b[i];
    }
    return diff === 0;
  },

  eq_fixed: (_n: number, a: Uint8Array, b: Uint8Array): boolean => {
    if (a.length !== b.length) return false;
    let diff = 0;
    for (let i = 0; i < a.length; i++) {
      diff |= a[i] ^ b[i];
    }
    return diff === 0;
  },

  select_bytes: (
    a: Uint8Array,
    b: Uint8Array,
    choice: boolean
  ): { ok: Uint8Array } | { err: string } => {
    if (a.length !== b.length) {
      return { err: "SelectError.CT_SELECT_LENGTH_MISMATCH" };
    }
    // Return a fresh cloned buffer
    const out = new Uint8Array(a.length);
    const mask = choice ? 0xff : 0x00;
    for (let i = 0; i < a.length; i++) {
      out[i] = (a[i] & mask) | (b[i] & ~mask);
    }
    return { ok: out };
  },

  eq_u64: (a: bigint, b: bigint): boolean => {
    return a === b;
  },
};

describe("Warrant Lexer & Parser (VER04c / warrant-syntax-v1)", () => {
  const fixturePath = path.resolve(
    __dirname,
    "../../../../specs/fixtures/warrant-syntax-v1/cases.json"
  );
  const fixtureContent = JSON.parse(fs.readFileSync(fixturePath, "utf-8"));

  it("should validate all 57 cases in the canonical syntax corpus", () => {
    expect(fixtureContent.cases.length).toBe(57);
    for (const testCase of fixtureContent.cases) {
      if (testCase.accept) {
        const mod = parseWarrant(testCase.source);
        expect(mod).toBeDefined();
      } else {
        expect(() => parseWarrant(testCase.source)).toThrow();
      }
    }
  });

  it("handles comments and indentation tokens correctly", () => {
    const src = `warrant test
  -- This is a canonical comment
  model M = U64
`;
    const tokens = tokenizeWarrant(src);
    expect(tokens.some((t) => t.type === "INDENT")).toBe(true);
    expect(tokens.some((t) => t.text === "test")).toBe(true);
  });
});

describe("Canonical ct-compare.warrant Execution (VER04e, VER04f)", () => {
  const warrantPath = path.resolve(__dirname, "../../../../specs/ct-compare.warrant");
  const warrantSource = fs.readFileSync(warrantPath, "utf-8");
  const module = parseWarrant(warrantSource);

  it("parses ct-compare.warrant module structure", () => {
    expect(module.name).toBe("ct_compare");
    expect(module.errors.has("SelectError")).toBe(true);
    expect(module.models.has("same")).toBe(true);
    expect(module.operations.has("eq")).toBe(true);
    expect(module.operations.has("select_bytes")).toBe(true);
    expect(module.examples.length).toBe(12);
    expect(module.claims.length).toBe(1);
    expect(module.inputContracts.length).toBe(9);
  });

  it("executes all 12 examples and 1 claim successfully against host implementation", () => {
    const results = executeWarrantModule(module, ctCompareHost, {
      claimSamples: 25,
      seed: 42n,
    });

    expect(results.length).toBe(13);
    for (const r of results) {
      expect(r.passed).toBe(true);
      expect(r.verdict.status).toBe("pass");
    }
  });

  it("falsifies when host implementation is broken (sabotage test)", () => {
    const brokenHostFalse = {
      ...ctCompareHost,
      eq: () => false, // Broken: always returns false
    };

    const resultsFalse = executeWarrantModule(module, brokenHostFalse);
    const eqEmptyResult = resultsFalse.find((r) => r.name === "eq_empty");
    expect(eqEmptyResult?.passed).toBe(false);

    const brokenHostTrue = {
      ...ctCompareHost,
      eq: () => true, // Broken: always returns true
    };

    const resultsTrue = executeWarrantModule(module, brokenHostTrue);
    const claimResult = resultsTrue.find((r) => r.name === "eq_agrees");
    expect(claimResult?.passed).toBe(false);
  });

  it("detects fresh output reference leak without emitting duplicate verdicts", () => {
    const leakingHost = {
      ...ctCompareHost,
      select_bytes: (a: Uint8Array, b: Uint8Array, choice: boolean) => {
        if (a.length !== b.length) {
          return { err: "SelectError.CT_SELECT_LENGTH_MISMATCH" };
        }
        // Leaking input reference directly instead of returning fresh allocation
        return { ok: choice ? a : b };
      },
    };

    const results = executeWarrantModule(module, leakingHost);
    const leakResults = results.filter((r) => r.name === "select_left_fresh");
    expect(leakResults.length).toBe(1);
    expect(leakResults[0].passed).toBe(false);
    expect(leakResults[0].verdict.status).toBe("fail");
  });
});

describe("Warrant Evaluator, Pattern Matching, and Resource Limits", () => {
  it("evaluates boolean, arithmetic, and comparison operators", () => {
    const mod = parseWarrant(`warrant math
  example bool_ops:
    (not (true and false) or false) == true
  example arith_ops:
    (10 + 20 * 2 - 5) == 45
  example cmp_ops:
    (5 < 10 and 10 <= 10 and 20 > 15 and 20 >= 20) == true
`);
    const results = executeWarrantModule(mod, {});
    expect(results.length).toBe(3);
    for (const r of results) {
      expect(r.passed).toBe(true);
    }
  });

  it("evaluates conditional and pattern matching expressions", () => {
    const mod = parseWarrant(`warrant exprs
  example if_true:
    (if true then 1 else 2) == 1
  example if_false:
    (if false then 1 else 2) == 2
  example match_none:
    (match None with None -> 10 | Some(x) -> x) == 10
  example match_some:
    (match Some(42) with None -> 0 | Some(x) -> x) == 42
  example match_ok:
    (match Ok(99) with Ok(x) -> x | Err(e) -> 0) == 99
`);
    const results = executeWarrantModule(mod, {});
    expect(results.length).toBe(5);
    for (const r of results) {
      expect(r.passed).toBe(true);
    }
  });

  it("handles recursion depth and step budget limits safely (W0703)", () => {
    const deepExpr = {
      kind: "binary" as const,
      op: "==",
      left: { kind: "bool" as const, value: true },
      right: { kind: "bool" as const, value: true },
    };

    // Depth limit exceeded
    expect(() =>
      evaluateWarrantExpr(deepExpr, {
        models: new Map(),
        hostOps: {},
        env: new Map(),
        depth: 65,
        budget: { count: 0, limit: 10_000 },
      })
    ).toThrow();

    // Step budget limit exceeded
    expect(() =>
      evaluateWarrantExpr(deepExpr, {
        models: new Map(),
        hostOps: {},
        env: new Map(),
        depth: 0,
        budget: { count: 10_001, limit: 10_000 },
      })
    ).toThrow();
  });

  it("rejects non-constant-time operators like division", () => {
    expect(() =>
      parseWarrant(`warrant divzero
  example bad:
    (10 / 2) == 5
`)
    ).toThrow();
  });

  it("filters claim candidates with refinement and fails closed on exhaustion", () => {
    const mod = parseWarrant(`warrant claims
  claim vacuous:
    forall n: U64 | n < 0. n == n
`);
    // Unsatisfiable condition (n < 0 for unsigned integer): must fail closed due to retry exhaustion
    const results = executeWarrantModule(mod, {});
    expect(results[0].passed).toBe(false);
    expect(results[0].verdict.status).toBe("fail");
  });

  it("compares warrant values with areWarrantValuesEqual", () => {
    expect(areWarrantValuesEqual(new Uint8Array([1, 2]), new Uint8Array([1, 2]))).toBe(true);
    expect(areWarrantValuesEqual(new Uint8Array([1, 2]), new Uint8Array([1, 3]))).toBe(false);
    expect(areWarrantValuesEqual(new Uint8Array([1]), new Uint8Array([1, 2]))).toBe(false);
    expect(areWarrantValuesEqual([1, 2, 3], [1, 2, 3])).toBe(true);
    expect(areWarrantValuesEqual([1, 2], [1, 3])).toBe(false);
    expect(areWarrantValuesEqual({ ok: 42 }, { ok: 42 })).toBe(true);
    expect(areWarrantValuesEqual({ some: 42 }, { some: 42 })).toBe(true);
    expect(areWarrantValuesEqual({ some: 42 }, { some: 43 })).toBe(false);
    expect(areWarrantValuesEqual({ err: "E.A" }, { err: "E.A" })).toBe(true);
    expect(areWarrantValuesEqual({ err: "E.A" }, { err: "E.B" })).toBe(false);
  });

  it("covers parser guards, errors, unary minus, and evaluator branches", () => {
    // Unterminated bytes literal
    expect(() => parseWarrant('warrant t\n example e:\n   b"12\n')).toThrow();

    // Parser recursion depth limit
    const deepParens = "(".repeat(70) + "true" + ")".repeat(70);
    expect(() => parseWarrant(`warrant t\n example e:\n   ${deepParens}\n`)).toThrow();

    // Invalid pattern in match arm
    expect(() => parseWarrant("warrant t\n model m(x: Bool) -> Int = match x with Bad -> 1\n")).toThrow();

    // Unary minus expression
    const modNeg = parseWarrant(`warrant t
  example neg:
    (-5 + 10) == 5
`);
    const resNeg = executeWarrantModule(modNeg, {});
    expect(resNeg[0].passed).toBe(true);

    // Unbound identifier in model evaluation
    const modUnbound = parseWarrant(`warrant t
  example unbound:
    missing_var == 5
`);
    const resUnbound = executeWarrantModule(modUnbound, {});
    expect(resUnbound[0].passed).toBe(false);

    // len() expects bytes or list
    const modLenErr = parseWarrant(`warrant t
  example badlen:
    len(true) == 1
`);
    const resLenErr = executeWarrantModule(modLenErr, {});
    expect(resLenErr[0].passed).toBe(false);

    // Claims with Bool and U8 binders
    const modBinders = parseWarrant(`warrant t
  claim bool_claim:
    forall b: Bool. (b or not b) == true
  claim u8_claim:
    forall x: U8. (x + 0u8) == x
`);
    const resBinders = executeWarrantModule(modBinders, {}, { claimSamples: 5 });
    expect(resBinders[0].passed).toBe(true);
    expect(resBinders[1].passed).toBe(true);

    // Generic op call evaluation
    const modGen = parseWarrant(`warrant t
  op test_gen<N: Nat>(x: Bytes<N>) -> Bool
  example call_gen:
    test_gen<16>(b"00") == true
`);
    const resGen = executeWarrantModule(modGen, {
      test_gen: (n: number, b: Uint8Array) => n === 16 && b.length === 1,
    });
    expect(resGen[0].passed).toBe(true);
  });
});

describe("Security, Bounds, and Host Operation Hardening", () => {
  it("guards against prototype pollution on host operations", () => {
    const mod = parseWarrant(`warrant proto
  example call_tostring:
    toString() == true
  example call_constructor:
    constructor() == true
`);
    const results = executeWarrantModule(mod, {});
    expect(results.length).toBe(2);
    expect(results[0].passed).toBe(false);
    expect(results[0].verdict.status).toBe("fail");
    expect(results[1].passed).toBe(false);
    expect(results[1].verdict.status).toBe("fail");
  });

  it("extracts and serializes counterexamples containing BigInts without TypeError", () => {
    const mod = parseWarrant(`warrant bigint_claim
  claim false_claim:
    forall n: U64. n == 0u64
`);
    const results = executeWarrantModule(mod, {});
    expect(results.length).toBe(1);
    expect(results[0].passed).toBe(false);
    expect(results[0].verdict.status).toBe("fail");
    expect(results[0].verdict.detail?.includes("TypeError")).toBe(false);
    expect(results[0].verdict.detail?.includes("Property violated")).toBe(true);
  });

  it("conjoins multiple binder refinements correctly", () => {
    const mod = parseWarrant(`warrant multi_refine
  claim refine_both:
    forall a: U8 | a > 50, b: U8 | b > 50. (a + b) > 100
`);
    const results = executeWarrantModule(mod, {}, { claimSamples: 20 });
    expect(results.length).toBe(1);
    expect(results[0].passed).toBe(true);
    expect(results[0].verdict.status).toBe("pass");
  });

  it("enforces recursion depth limits on unary expressions and nested types", () => {
    const deepNot = "not ".repeat(100) + "true";
    expect(() => parseWarrant(`warrant d\n  example e:\n    ${deepNot} == true\n`)).toThrow();

    const deepType = "Option<".repeat(100) + "Int" + ">".repeat(100);
    expect(() => parseWarrant(`warrant dt\n  model M = ${deepType}\n`)).toThrow();
  });

  it("detects direct and subarray buffer aliasing for fresh operations", () => {
    const mod = parseWarrant(`warrant fresh_dup
  op dup(x: Bytes) -> Bytes fresh
  example test_dup:
    dup(b"0102") == b"0102"
`);
    // Leaking identical buffer
    const leakingDirectHost = {
      dup: (x: Uint8Array) => x,
    };
    const resDirect = executeWarrantModule(mod, leakingDirectHost);
    expect(resDirect[0].passed).toBe(false);
    expect(resDirect[0].verdict.code).toBe("fresh-aliasing");

    // Leaking subarray buffer with overlapping memory
    const leakingSubarrayHost = {
      dup: (x: Uint8Array) => x.subarray(0, x.length),
    };
    const resSub = executeWarrantModule(mod, leakingSubarrayHost);
    expect(resSub[0].passed).toBe(false);
    expect(resSub[0].verdict.code).toBe("fresh-aliasing");

    // Proper fresh cloned buffer passes
    const honestHost = {
      dup: (x: Uint8Array) => new Uint8Array(x),
    };
    const resHonest = executeWarrantModule(mod, honestHost);
    expect(resHonest[0].passed).toBe(true);
    expect(resHonest[0].verdict.status).toBe("pass");
  });
});
