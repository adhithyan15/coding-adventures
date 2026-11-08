import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  Solver,
  CnfBuilder,
  makeLit,
  negateLit,
  varOf,
  signOf,
  litToString,
  parseDimacs,
  writeDimacs,
  LBool,
  type Lit,
} from "../dist/index.js";

// Clean standalone assertion helper without external harness dependencies
function expect<T>(actual: T) {
  return {
    toBe(expected: T): void {
      assert.strictEqual(actual, expected);
    },
    toEqual(expected: any): void {
      assert.deepStrictEqual(actual, expected);
    },
    toThrow(): void {
      assert.throws(actual as any);
    },
  };
}

// Standalone deterministic PRNG for differential fuzz/random tests
class SplitMix64 {
  private state: bigint;
  constructor(seed: bigint | number) {
    this.state = BigInt.asUintN(64, BigInt(seed));
  }
  nextU64(): bigint {
    this.state = BigInt.asUintN(64, this.state + 0x9e37_79b9_7f4a_7c15n);
    let z = this.state;
    z = BigInt.asUintN(64, (z ^ (z >> 30n)) * 0xbf58_476d_1ce4_e5b9n);
    z = BigInt.asUintN(64, (z ^ (z >> 27n)) * 0x94d0_49bb_1331_11ebn);
    return BigInt.asUintN(64, z ^ (z >> 31n));
  }
  bounded(n: number | bigint): number {
    const bn = BigInt(n);
    const threshold = BigInt.asUintN(64, -bn) % bn;
    while (true) {
      const x = this.nextU64();
      if (x >= threshold) {
        return Number(x % bn);
      }
    }
  }
}

// Helper: independent brute-force oracle for differential testing
function bruteForceSat(numVars: number, clauses: Lit[][], assumptions: Lit[] = []): boolean {
  const total = 1 << numVars;
  for (let mask = 0; mask < total; mask++) {
    // Check assumptions
    let sat = true;
    for (const a of assumptions) {
      const v = varOf(a);
      const isNeg = signOf(a) === 1;
      const bit = (mask >> v) & 1;
      const val = isNeg ? bit === 0 : bit === 1;
      if (!val) {
        sat = false;
        break;
      }
    }
    if (!sat) continue;

    // Check all clauses
    let allClausesSatisfied = true;
    for (const clause of clauses) {
      let clauseSat = false;
      for (const lit of clause) {
        const v = varOf(lit);
        const isNeg = signOf(lit) === 1;
        const bit = (mask >> v) & 1;
        const val = isNeg ? bit === 0 : bit === 1;
        if (val) {
          clauseSat = true;
          break;
        }
      }
      if (!clauseSat) {
        allClausesSatisfied = false;
        break;
      }
    }
    if (allClausesSatisfied) return true;
  }
  return false;
}

// Independent model validator
function isModelValid(clauses: Lit[][], model: boolean[]): boolean {
  for (const clause of clauses) {
    let clauseSat = false;
    for (const lit of clause) {
      const v = varOf(lit);
      const isNeg = signOf(lit) === 1;
      const val = model[v];
      if (isNeg ? !val : val) {
        clauseSat = true;
        break;
      }
    }
    if (!clauseSat) return false;
  }
  return true;
}

describe("CDCL SAT: Literals, Variables, and DIMACS Transport (VER09 §3.1, §3.6)", () => {
  it("encodes variables and literals correctly", () => {
    const l0 = makeLit(0, false);
    const l0_neg = makeLit(0, true);
    expect(varOf(l0)).toBe(0);
    expect(signOf(l0)).toBe(0);
    expect(varOf(l0_neg)).toBe(0);
    expect(signOf(l0_neg)).toBe(1);
    expect(negateLit(l0)).toBe(l0_neg);
    expect(negateLit(l0_neg)).toBe(l0);

    const l5 = makeLit(5, true);
    expect(varOf(l5)).toBe(5);
    expect(signOf(l5)).toBe(1);
    expect(litToString(l5)).toBe("-6");
    expect(litToString(makeLit(5, false))).toBe("6");
  });

  it("parses and writes DIMACS CNF files accurately", () => {
    const dimacsText = `c Simple test formula
p cnf 3 2
1 -2 3 0
-1 2 0
`;
    const cnf = parseDimacs(dimacsText);
    expect(cnf.numVars).toBe(3);
    expect(cnf.clauses.length).toBe(2);

    const serialized = writeDimacs(cnf);
    expect(serialized.includes("p cnf 3 2")).toBe(true);
    expect(serialized.includes("1 -2 3 0")).toBe(true);
    expect(serialized.includes("-1 2 0")).toBe(true);
  });

  it("rejects malformed DIMACS inputs", () => {
    expect(() => parseDimacs("invalid header")).toThrow();
    expect(() => parseDimacs("p cnf\n1 0\n")).toThrow();
    expect(() => parseDimacs("p cnf 2 1\n1 bad 0\n")).toThrow();
  });
});

describe("CDCL SAT: Solving, 1-UIP Learning, and Model Extraction (VER09 §3.3)", () => {
  it("solves simple satisfiable formulas and returns valid models", () => {
    const s = new Solver();
    const a = makeLit(s.newVar(), false);
    const b = makeLit(s.newVar(), false);
    const c = makeLit(s.newVar(), false);

    // (a or b) and (not a or c) and (not b or c)
    s.addClause([a, b]);
    s.addClause([negateLit(a), c]);
    s.addClause([negateLit(b), c]);

    const res = s.solve();
    expect(res).toBe("sat");

    const model = s.getModel();
    expect(model.length).toBe(3);
    // c must be true in any satisfying assignment
    expect(model[varOf(c)]).toBe(true);
  });

  it("proves direct contradiction UNSAT at level 0", () => {
    const s = new Solver();
    const x = makeLit(s.newVar(), false);
    expect(s.addClause([x])).toBe(true);
    expect(s.addClause([negateLit(x)])).toBe(false); // Immediate contradiction
  });

  it("proves Pigeonhole Principle (PHP 4 pigeons into 3 holes) is UNSAT", () => {
    const s = new Solver();
    const pigeons = 4;
    const holes = 3;

    // p[i][j]: pigeon i in hole j
    const p: Lit[][] = [];
    for (let i = 0; i < pigeons; i++) {
      p[i] = [];
      for (let j = 0; j < holes; j++) {
        p[i][j] = makeLit(s.newVar(), false);
      }
    }

    // 1. Each pigeon must be in at least one hole
    for (let i = 0; i < pigeons; i++) {
      s.addClause(p[i]);
    }

    // 2. No two pigeons in the same hole
    for (let j = 0; j < holes; j++) {
      for (let i1 = 0; i1 < pigeons; i1++) {
        for (let i2 = i1 + 1; i2 < pigeons; i2++) {
          s.addClause([negateLit(p[i1][j]), negateLit(p[i2][j])]);
        }
      }
    }

    const res = s.solve();
    expect(res).toBe("unsat");
  });

  it("solves 4-Queens problem and validates non-attacking queen placement", () => {
    const s = new Solver();
    const N = 4;
    const grid: Lit[][] = [];
    for (let r = 0; r < N; r++) {
      grid[r] = [];
      for (let c = 0; c < N; c++) {
        grid[r][c] = makeLit(s.newVar(), false);
      }
    }

    // Exactly one queen per row
    for (let r = 0; r < N; r++) {
      s.addClause(grid[r]); // At least one
      for (let c1 = 0; c1 < N; c1++) {
        for (let c2 = c1 + 1; c2 < N; c2++) {
          s.addClause([negateLit(grid[r][c1]), negateLit(grid[r][c2])]); // At most one
        }
      }
    }

    // At most one queen per column
    for (let c = 0; c < N; c++) {
      for (let r1 = 0; r1 < N; r1++) {
        for (let r2 = r1 + 1; r2 < N; r2++) {
          s.addClause([negateLit(grid[r1][c]), negateLit(grid[r2][c])]);
        }
      }
    }

    // At most one queen per diagonal
    for (let r1 = 0; r1 < N; r1++) {
      for (let c1 = 0; c1 < N; c1++) {
        for (let r2 = r1 + 1; r2 < N; r2++) {
          for (let c2 = 0; c2 < N; c2++) {
            if (Math.abs(r1 - r2) === Math.abs(c1 - c2)) {
              s.addClause([negateLit(grid[r1][c1]), negateLit(grid[r2][c2])]);
            }
          }
        }
      }
    }

    const res = s.solve();
    expect(res).toBe("sat");

    const model = s.getModel();
    let queenCount = 0;
    for (let r = 0; r < N; r++) {
      for (let c = 0; c < N; c++) {
        if (model[varOf(grid[r][c])]) {
          queenCount++;
        }
      }
    }
    expect(queenCount).toBe(4);
  });
});

describe("CDCL SAT: Incremental Solving, Assumptions, and Failed Core (VER09 §2)", () => {
  it("solves under assumptions and extracts sound failed assumption cores", () => {
    const s = new Solver();
    const a = makeLit(s.newVar(), false);
    const b = makeLit(s.newVar(), false);
    const c = makeLit(s.newVar(), false);

    // a -> b  (not a or b)
    s.addClause([negateLit(a), b]);

    // Test 1: Assume a = true and b = true -> SAT
    expect(s.solveWithAssumptions([a, b])).toBe("sat");

    // Test 2: Assume a = true and b = false -> UNSAT (violates a -> b)
    // Pass irrelevant assumption c as well; core must contain a and not b, but exclude c!
    expect(s.solveWithAssumptions([c, a, negateLit(b)])).toBe("unsat");

    // Check failed core
    const core = s.getFailedCore();
    expect(core.includes(a)).toBe(true);
    expect(core.includes(negateLit(b))).toBe(true);
    expect(core.includes(c)).toBe(false);

    // Test 3: Incremental clause addition
    // Add clause: not b
    s.addClause([negateLit(b)]);
    // Now assume a = true -> UNSAT
    expect(s.solveWithAssumptions([a])).toBe("unsat");
    expect(s.getFailedCore().includes(a)).toBe(true);
    // Assume not a -> SAT
    expect(s.solveWithAssumptions([negateLit(a)])).toBe("sat");
  });

  it("handles empty clause and level 0 contradiction via ok flag", () => {
    const s1 = new Solver();
    const x = makeLit(s1.newVar(), false);
    expect(s1.addClause([])).toBe(false);
    expect(s1.okay()).toBe(false);
    expect(s1.solve()).toBe("unsat");

    const s2 = new Solver();
    const y = makeLit(s2.newVar(), false);
    expect(s2.addClause([y])).toBe(true);
    expect(s2.addClause([negateLit(y)])).toBe(false);
    expect(s2.okay()).toBe(false);
    expect(s2.solve()).toBe("unsat");
  });

  it("prevents clause dropping and state leakage when adding clauses after solve", () => {
    const s = new Solver();
    const x = makeLit(s.newVar(), false);
    const y = makeLit(s.newVar(), false);

    // Initially satisfiable with x = true, y = false
    s.addClause([x]);
    expect(s.solve()).toBe("sat");
    expect(s.getModel()[varOf(x)]).toBe(true);

    // Now add constraint: not x
    // If addClause doesn't backjump to 0, this would evaluate x as true, think clause is satisfied,
    // and drop it!
    const added = s.addClause([negateLit(x)]);
    expect(added).toBe(false);
    expect(s.solve()).toBe("unsat");
  });

  it("isolates conflict and propagation budgets across successive solve calls", () => {
    const s = new Solver();
    // Build a Pigeonhole Principle instance PHP(4, 3) which requires multiple conflicts
    const pigeon = 4;
    const holes = 3;
    const vars: Lit[][] = [];
    for (let p = 0; p < pigeon; p++) {
      const row: Lit[] = [];
      for (let h = 0; h < holes; h++) {
        row.push(makeLit(s.newVar(), false));
      }
      vars.push(row);
      s.addClause(row);
    }
    for (let h = 0; h < holes; h++) {
      for (let p1 = 0; p1 < pigeon; p1++) {
        for (let p2 = p1 + 1; p2 < pigeon; p2++) {
          s.addClause([negateLit(vars[p1][h]), negateLit(vars[p2][h])]);
        }
      }
    }

    // First solve with small conflict budget: should return "unknown"
    const r1 = s.solve({ maxConflicts: 1 });
    expect(r1).toBe("unknown");
    const firstConflicts = s.conflicts;
    expect(firstConflicts >= 1).toBe(true);

    // Second solve with generous conflict budget: should solve and return "unsat"
    const r2 = s.solve({ maxConflicts: 100 });
    expect(r2).toBe("unsat");
  });
});

describe("CDCL SAT: Differential Testing vs Brute-Force Oracle (VER09 §2, VER00 §7.5)", () => {
  it("agrees with brute-force oracle across 100 random 3-SAT formulas", () => {
    const rng = new SplitMix64(0x9e3779b97f4a7c15n);
    let satCount = 0;
    let unsatCount = 0;

    for (let round = 0; round < 100; round++) {
      const vars = 4 + Number(rng.bounded(5)); // 4 to 8 vars
      const clausesCount = Math.floor(vars * 4.26); // Around 3-SAT phase transition

      const clauses: Lit[][] = [];
      const s = new Solver();
      for (let v = 0; v < vars; v++) {
        s.newVar();
      }

      for (let c = 0; c < clausesCount; c++) {
        const cLits: Lit[] = [];
        while (cLits.length < 3) {
          const v = Number(rng.bounded(vars));
          const sign = rng.bounded(2) === 1;
          const lit = makeLit(v, sign);
          if (!cLits.includes(lit) && !cLits.includes(negateLit(lit))) {
            cLits.push(lit);
          }
        }
        clauses.push(cLits);
        s.addClause(cLits);
      }

      const expected = bruteForceSat(vars, clauses);
      const res = s.solve();

      if (expected) {
        expect(res).toBe("sat");
        const model = s.getModel();
        expect(isModelValid(clauses, model)).toBe(true);
        satCount++;
      } else {
        expect(res).toBe("unsat");
        unsatCount++;
      }
    }

    // Both SAT and UNSAT should be exercised
    expect(satCount > 0).toBe(true);
    expect(unsatCount > 0).toBe(true);
  });
});

describe("Tseitin Bit-Blaster & Circuit Verification (VER09a §3)", () => {
  it("verifies logical gates (AND, OR, XOR, ITE, NOT, EQUIV)", () => {
    const builder = new CnfBuilder();
    const a = builder.newLit();
    const b = builder.newLit();

    // Verify XOR truth table
    const xorOut = builder.xor(a, b);
    const s = builder.getSolver();

    // 0 ^ 0 = 0
    expect(s.solveWithAssumptions([negateLit(a), negateLit(b), xorOut])).toBe("unsat");
    expect(s.solveWithAssumptions([negateLit(a), negateLit(b), negateLit(xorOut)])).toBe("sat");

    // 1 ^ 0 = 1
    expect(s.solveWithAssumptions([a, negateLit(b), xorOut])).toBe("sat");
    expect(s.solveWithAssumptions([a, negateLit(b), negateLit(xorOut)])).toBe("unsat");

    // 1 ^ 1 = 0
    expect(s.solveWithAssumptions([a, b, xorOut])).toBe("unsat");
    expect(s.solveWithAssumptions([a, b, negateLit(xorOut)])).toBe("sat");
  });

  it("verifies 1-bit full adder correctness across all 8 input combinations", () => {
    const builder = new CnfBuilder();
    const a = builder.newLit();
    const b = builder.newLit();
    const cin = builder.newLit();
    const { sum, cout } = builder.fullAdder(a, b, cin);
    const s = builder.getSolver();

    for (let va = 0; va <= 1; va++) {
      for (let vb = 0; vb <= 1; vb++) {
        for (let vc = 0; vc <= 1; vc++) {
          const expectedSum = (va + vb + vc) % 2 === 1;
          const expectedCout = va + vb + vc >= 2;

          const assumptions = [
            va === 1 ? a : negateLit(a),
            vb === 1 ? b : negateLit(b),
            vc === 1 ? cin : negateLit(cin),
            expectedSum ? sum : negateLit(sum),
            expectedCout ? cout : negateLit(cout),
          ];

          expect(s.solveWithAssumptions(assumptions)).toBe("sat");

          // Sabotage: assert opposite sum
          const badAssumptions = [
            va === 1 ? a : negateLit(a),
            vb === 1 ? b : negateLit(b),
            vc === 1 ? cin : negateLit(cin),
            expectedSum ? negateLit(sum) : sum,
          ];
          expect(s.solveWithAssumptions(badAssumptions)).toBe("unsat");
        }
      }
    }
  });

  it("verifies 4-bit ripple-carry adder correctness with bit-blasting", () => {
    const builder = new CnfBuilder();
    const a = [builder.newLit(), builder.newLit(), builder.newLit(), builder.newLit()];
    const b = [builder.newLit(), builder.newLit(), builder.newLit(), builder.newLit()];
    const sum = builder.bvAdd(a, b);
    const s = builder.getSolver();

    // Test a = 3 (0011), b = 5 (0101) -> sum must be 8 (1000)
    const assumptions = [
      // a = 3
      a[0],
      a[1],
      negateLit(a[2]),
      negateLit(a[3]),
      // b = 5
      b[0],
      negateLit(b[1]),
      b[2],
      negateLit(b[3]),
      // expected sum = 8 (0001 in little-endian bit order)
      negateLit(sum[0]),
      negateLit(sum[1]),
      negateLit(sum[2]),
      sum[3],
    ];

    expect(s.solveWithAssumptions(assumptions)).toBe("sat");

    // Sabotage: assert sum = 7
    const badAssumptions = [
      a[0],
      a[1],
      negateLit(a[2]),
      negateLit(a[3]),
      b[0],
      negateLit(b[1]),
      b[2],
      negateLit(b[3]),
      sum[0],
      sum[1],
      sum[2],
      negateLit(sum[3]),
    ];
    expect(s.solveWithAssumptions(badAssumptions)).toBe("unsat");
  });
});

describe("CDCL SAT: Resource Bounding and Hostile Input Protection (VER09 §2.9)", () => {
  it("enforces conflict budget returning unknown", () => {
    const s = new Solver();
    // Pigeonhole principle formula requiring multiple conflicts
    const pigeons = 6;
    const holes = 5;
    const p: Lit[][] = [];
    for (let i = 0; i < pigeons; i++) {
      p[i] = [];
      for (let j = 0; j < holes; j++) {
        p[i][j] = makeLit(s.newVar(), false);
      }
    }
    for (let i = 0; i < pigeons; i++) s.addClause(p[i]);
    for (let j = 0; j < holes; j++) {
      for (let i1 = 0; i1 < pigeons; i1++) {
        for (let i2 = i1 + 1; i2 < pigeons; i2++) {
          s.addClause([negateLit(p[i1][j]), negateLit(p[i2][j])]);
        }
      }
    }

    // Set tight budget: maxConflicts = 2
    const res = s.solve({ maxConflicts: 2 });
    expect(res).toBe("unknown");
  });

  it("enforces step budget returning unknown", () => {
    const s = new Solver();
    const a = makeLit(s.newVar(), false);
    const b = makeLit(s.newVar(), false);
    s.addClause([a, b]);
    s.addClause([negateLit(a), b]);

    const res = s.solve({ maxSteps: 0 });
    expect(res).toBe("unknown");
  });

  it("inspects solver variables, clauses, values, and resets cleanly", () => {
    const s = new Solver();
    const v1 = s.newVar();
    const v2 = s.newVar();
    expect(s.nVars()).toBe(2);
    s.addClause([makeLit(v1, false), makeLit(v2, false)]);
    expect(s.nClauses()).toBe(1);

    expect(s.varValue(v1)).toBe(LBool.Undef);
    expect(s.value(makeLit(v1, false))).toBe(LBool.Undef);

    // Out-of-bounds variable / literal queries return LBool.Undef (never undefined/NaN)
    expect(s.varValue(999)).toBe(LBool.Undef);
    expect(s.value(makeLit(999, false))).toBe(LBool.Undef);
    expect(s.value(makeLit(999, true))).toBe(LBool.Undef);

    s.solveWithAssumptions([makeLit(v1, false)]);
    expect(s.varValue(v1)).toBe(LBool.True);
    expect(s.value(makeLit(v1, false))).toBe(LBool.True);
    expect(s.value(makeLit(v1, true))).toBe(LBool.False);

    s.reset();
    expect(s.varValue(v1)).toBe(LBool.Undef);
  });

  it("prevents state corruption and budget leakage across successive addClause and solve calls", () => {
    const s = new Solver();
    const a = makeLit(s.newVar(), false);
    const b = makeLit(s.newVar(), false);
    s.addClause([a, b]);
    s.addClause([negateLit(a), b]);

    // First solve with maxSteps: 1 -> budget exhausted returns unknown
    const r1 = s.solve({ maxSteps: 1 });
    expect(r1).toBe("unknown");

    // Add unit clause after step budget exhaustion
    const c = makeLit(s.newVar(), false);
    const added = s.addClause([c]);
    expect(added).toBe(true);
    expect(s.okay()).toBe(true);

    // Second solve without limits must correctly solve without false unsat
    const r2 = s.solve();
    expect(r2).toBe("sat");
    expect(s.okay()).toBe(true);
  });

  it("rejects negative or malformed literals defensively", () => {
    const s = new Solver();
    expect(() => s.addClause([-1])).toThrow();
    expect(() => s.addClause([1.5])).toThrow();
    expect(() => s.solveWithAssumptions([-1])).toThrow();
  });

  it("tests Tseitin ITE, EQUIV, bvEq, and width mismatches", () => {
    const builder = new CnfBuilder();
    const cond = builder.newLit();
    const t = builder.newLit();
    const e = builder.newLit();

    // ITE(cond, t, e)
    const iteOut = builder.ite(cond, t, e);
    const s = builder.getSolver();

    // cond=1, t=1 -> iteOut must be 1
    expect(s.solveWithAssumptions([cond, t, negateLit(iteOut)])).toBe("unsat");
    expect(s.solveWithAssumptions([cond, t, iteOut])).toBe("sat");

    // cond=0, e=1 -> iteOut must be 1
    expect(s.solveWithAssumptions([negateLit(cond), e, negateLit(iteOut)])).toBe("unsat");

    // EQUIV
    const eqLit = builder.equiv(t, e);
    expect(s.solveWithAssumptions([t, e, eqLit])).toBe("sat");
    expect(s.solveWithAssumptions([t, negateLit(e), eqLit])).toBe("unsat");

    // bvEq
    const bv1 = [t, e];
    const bv2 = [t, e];
    const bveqLit = builder.bvEq(bv1, bv2);
    expect(s.solveWithAssumptions([bveqLit])).toBe("sat");

    // Empty bvEq
    const emptyEq = builder.bvEq([], []);
    expect(s.solveWithAssumptions([emptyEq])).toBe("sat");

    // Width mismatch
    expect(() => builder.bvEq([t], [t, e])).toThrow();
    expect(() => builder.bvAdd([t], [t, e])).toThrow();

    // trueLit and falseLit
    const tLit = builder.trueLit();
    const fLit = builder.falseLit();
    expect(s.solveWithAssumptions([tLit, negateLit(fLit)])).toBe("sat");
  });

  it("rejects malformed DIMACS inputs strictly", () => {
    // Unterminated clause
    const unclosed = `p cnf 2 1\n1 2\n`;
    expect(() => parseDimacs(unclosed)).toThrow();

    // Negative var count
    const negVars = `p cnf -2 1\n1 0\n`;
    expect(() => parseDimacs(negVars)).toThrow();

    // Literal out of declared range [1, numVars]
    const outOfRange = `p cnf 2 1\n3 0\n`;
    expect(() => parseDimacs(outOfRange)).toThrow();

    // Non-integer token
    const nonInt = `p cnf 2 1\n1 123abc 0\n`;
    expect(() => parseDimacs(nonInt)).toThrow();

    // Clause count mismatch
    const clauseMismatch = `p cnf 2 2\n1 2 0\n`;
    expect(() => parseDimacs(clauseMismatch)).toThrow();

    // Max vars cap exceeded
    const tooManyVars = `p cnf 100 1\n1 0\n`;
    expect(() => parseDimacs(tooManyVars, { maxVars: 50 })).toThrow();

    // SATLIB % terminator halts parsing cleanly before trailing garbage
    const satlibText = `p cnf 2 1\n1 2 0\n%\nmalformed non-dimacs garbage trailing lines`;
    const parsedSatlib = parseDimacs(satlibText);
    expect(parsedSatlib.numVars).toBe(2);
    expect(parsedSatlib.clauses.length).toBe(1);
  });
});
