/**
 * markov-chain.test.ts — Vitest tests for DT28 MarkovChain
 * ==========================================================
 *
 * These tests correspond directly to the 10 test cases specified in
 * code/specs/DT28-markov-chain.md. Each test is labelled with its
 * spec number so you can cross-reference easily.
 *
 * The test structure mirrors what we verify:
 *
 * 1.  Construction    — empty chain, zero states
 * 2.  Single pair     — train([A,B]), probability(A,B) == 1.0
 * 3.  Sequence        — train([A,B,A,C,A,B,B,A]), verify probabilities
 * 4.  Smoothing       — Laplace α=1, 3 states, probability(A,C) == 0.25
 * 5.  Generate length — generate(A,10) returns exactly 10 items
 * 6.  Generate string — generateString("th", 50) returns 50-char string
 * 7.  Stationary dist — sum of values ≈ 1.0
 * 8.  Order-2 chain   — generateString("ab",9) === "abcabcabc"
 * 9.  Unknown state   — nextState("UNKNOWN") throws
 * 10. Multi-train     — two train() calls accumulate counts
 */

import { describe, it, expect } from "vitest";
import { MarkovChain } from "../src/index.js";

// ---------------------------------------------------------------------------
// Test 1 — Construction: empty chain has zero states
// ---------------------------------------------------------------------------
describe("Test 1 — Construction", () => {
  it("creates a new chain with no states and no transitions", () => {
    const chain = new MarkovChain();
    expect(chain.states()).toEqual([]);
    expect(chain.transitionMatrix().size).toBe(0);
  });

  it("accepts order and smoothing parameters", () => {
    const chain = new MarkovChain(2, 0.5);
    expect(chain.states()).toHaveLength(0);
  });

  it("pre-registers states from the constructor", () => {
    const chain = new MarkovChain(1, 0.0, ["A", "B", "C"]);
    // All 3 states should be in the alphabet
    expect(chain.states().sort()).toEqual(["A", "B", "C"]);
  });

  it("throws if order < 1", () => {
    expect(() => new MarkovChain(0)).toThrow();
  });

  it("throws if smoothing < 0", () => {
    expect(() => new MarkovChain(1, -0.1)).toThrow();
  });
});

// ---------------------------------------------------------------------------
// Test 2 — Train single pair: probability(A, B) == 1.0
// ---------------------------------------------------------------------------
describe("Test 2 — Train single pair", () => {
  it("gives probability 1.0 when A→B is the only transition", () => {
    const chain = new MarkovChain();
    chain.train(["A", "B"]);
    expect(chain.probability("A", "B")).toBeCloseTo(1.0, 10);
  });

  it("adds both states to the alphabet", () => {
    const chain = new MarkovChain();
    chain.train(["A", "B"]);
    expect(chain.states().sort()).toEqual(["A", "B"]);
  });

  it("does nothing when sequence is too short to form a window", () => {
    const chain = new MarkovChain();
    chain.train(["A"]); // order=1 requires at least 2 elements
    expect(chain.states()).toHaveLength(0);
    expect(chain.probability("A", "B")).toBe(0.0);
  });
});

// ---------------------------------------------------------------------------
// Test 3 — Train sequence: counts and normalisation
// ---------------------------------------------------------------------------
describe("Test 3 — Train sequence [A,B,A,C,A,B,B,A]", () => {
  // Observations: A→B, B→A, A→C, C→A, A→B, B→B, B→A
  // Count(A→B)=2, Count(A→C)=1, Count(B→A)=2, Count(B→B)=1, Count(C→A)=1
  // Row A: total=3, prob(A→B)=2/3, prob(A→C)=1/3
  // Row B: total=3, prob(B→A)=2/3, prob(B→B)=1/3
  // Row C: total=1, prob(C→A)=1/1

  it("probability(A, B) ≈ 0.667", () => {
    const chain = new MarkovChain();
    chain.train(["A", "B", "A", "C", "A", "B", "B", "A"]);
    expect(chain.probability("A", "B")).toBeCloseTo(2 / 3, 8);
  });

  it("probability(A, C) ≈ 0.333", () => {
    const chain = new MarkovChain();
    chain.train(["A", "B", "A", "C", "A", "B", "B", "A"]);
    expect(chain.probability("A", "C")).toBeCloseTo(1 / 3, 8);
  });

  it("probability(B, A) ≈ 0.667", () => {
    const chain = new MarkovChain();
    chain.train(["A", "B", "A", "C", "A", "B", "B", "A"]);
    expect(chain.probability("B", "A")).toBeCloseTo(2 / 3, 8);
  });

  it("probability(B, B) ≈ 0.333 (self-loop)", () => {
    const chain = new MarkovChain();
    chain.train(["A", "B", "A", "C", "A", "B", "B", "A"]);
    expect(chain.probability("B", "B")).toBeCloseTo(1 / 3, 8);
  });

  it("probability(C, A) == 1.0", () => {
    const chain = new MarkovChain();
    chain.train(["A", "B", "A", "C", "A", "B", "B", "A"]);
    expect(chain.probability("C", "A")).toBeCloseTo(1.0, 10);
  });

  it("row A sums to 1.0", () => {
    const chain = new MarkovChain();
    chain.train(["A", "B", "A", "C", "A", "B", "B", "A"]);
    const matrix = chain.transitionMatrix();
    const rowA = matrix.get("A")!;
    let sum = 0;
    for (const p of rowA.values()) sum += p;
    expect(sum).toBeCloseTo(1.0, 10);
  });

  it("unseen transition probability(A, A) == 0.0 (no smoothing)", () => {
    const chain = new MarkovChain();
    chain.train(["A", "B", "A", "C", "A", "B", "B", "A"]);
    expect(chain.probability("A", "A")).toBe(0.0);
  });
});

// ---------------------------------------------------------------------------
// Test 4 — Laplace smoothing: probability(A, C) == 1/4
// ---------------------------------------------------------------------------
describe("Test 4 — Laplace smoothing", () => {
  // Chain: order=1, smoothing=1.0, states=[A,B,C] (pre-registered)
  // Train on [A, B]: Count(A→B) = 1
  // Row A with smoothing=1, |Σ|=3:
  //   prob(A→A) = (0+1) / (1+3) = 1/4
  //   prob(A→B) = (1+1) / (1+3) = 2/4 = 0.5
  //   prob(A→C) = (0+1) / (1+3) = 1/4

  it("probability(A, C) == 0.25 after training on [A,B] with 3 states", () => {
    const chain = new MarkovChain(1, 1.0, ["A", "B", "C"]);
    chain.train(["A", "B"]);
    expect(chain.probability("A", "C")).toBeCloseTo(0.25, 10);
  });

  it("probability(A, B) == 0.5 with Laplace smoothing", () => {
    const chain = new MarkovChain(1, 1.0, ["A", "B", "C"]);
    chain.train(["A", "B"]);
    expect(chain.probability("A", "B")).toBeCloseTo(0.5, 10);
  });

  it("row A sums to 1.0 under smoothing", () => {
    const chain = new MarkovChain(1, 1.0, ["A", "B", "C"]);
    chain.train(["A", "B"]);
    const matrix = chain.transitionMatrix();
    const rowA = matrix.get("A")!;
    let sum = 0;
    for (const p of rowA.values()) sum += p;
    expect(sum).toBeCloseTo(1.0, 10);
  });

  it("smoothing makes all probabilities non-zero", () => {
    const chain = new MarkovChain(1, 1.0, ["A", "B", "C"]);
    chain.train(["A", "B"]);
    // Even A→A (never observed) gets non-zero probability
    expect(chain.probability("A", "A")).toBeGreaterThan(0);
  });
});

// ---------------------------------------------------------------------------
// Test 5 — Generate length: exactly 10 items
// ---------------------------------------------------------------------------
describe("Test 5 — Generate length", () => {
  it("generate(A, 10) returns exactly 10 states", () => {
    const chain = new MarkovChain(1, 1.0);
    chain.train(["A", "B", "A", "C", "B", "A", "C", "B"]);
    const result = chain.generate("A", 10);
    expect(result).toHaveLength(10);
  });

  it("generate starts with the given state for order=1", () => {
    const chain = new MarkovChain(1, 1.0);
    chain.train(["A", "B", "A", "C", "B"]);
    const result = chain.generate("A", 5);
    expect(result[0]).toBe("A");
  });

  it("generate(A, 1) returns exactly 1 state", () => {
    const chain = new MarkovChain();
    chain.train(["A", "B"]);
    const result = chain.generate("A", 1);
    expect(result).toHaveLength(1);
    expect(result[0]).toBe("A");
  });

  it("generate(A, 0) returns empty array", () => {
    const chain = new MarkovChain();
    chain.train(["A", "B"]);
    expect(chain.generate("A", 0)).toHaveLength(0);
  });

  it("all generated states are known alphabet states", () => {
    const chain = new MarkovChain(1, 1.0);
    chain.train(["A", "B", "C", "A", "B", "C"]);
    const alphabet = new Set(chain.states());
    const result = chain.generate("A", 20);
    for (const s of result) {
      expect(alphabet.has(s)).toBe(true);
    }
  });
});

// ---------------------------------------------------------------------------
// Test 6 — Generate string: 50-char string from "th" seed
// ---------------------------------------------------------------------------
describe("Test 6 — Generate string from text corpus", () => {
  it("generateString produces exactly the requested length", () => {
    const chain = new MarkovChain(1, 0.1);
    chain.trainString(
      "the quick brown fox jumps over the lazy dog ".repeat(20)
    );
    const result = chain.generateString("t", 50);
    expect(result).toHaveLength(50);
  });

  it("generateString starts with the seed character", () => {
    const chain = new MarkovChain(1, 0.1);
    chain.trainString("abcabc".repeat(10));
    const result = chain.generateString("a", 10);
    expect(result[0]).toBe("a");
  });

  it("generateString(seed, 1) returns a single character", () => {
    const chain = new MarkovChain(1, 0.1);
    chain.trainString("abcabc");
    const result = chain.generateString("a", 1);
    expect(result).toHaveLength(1);
  });

  it("generateString(seed, 0) returns empty string", () => {
    const chain = new MarkovChain(1, 0.1);
    chain.trainString("abcabc");
    expect(chain.generateString("a", 0)).toBe("");
  });

  it("throws if seed is shorter than order", () => {
    const chain = new MarkovChain(2, 0.1);
    chain.trainString("abcabcabc");
    expect(() => chain.generateString("a", 10)).toThrow();
  });
});

// ---------------------------------------------------------------------------
// Test 7 — Stationary distribution sums to 1.0
// ---------------------------------------------------------------------------
describe("Test 7 — Stationary distribution", () => {
  it("sum of stationary distribution values ≈ 1.0", () => {
    // Build an ergodic 3-state chain (all states reachable from all others).
    const chain = new MarkovChain(1, 1.0);
    // Train on a repeating sequence that visits all states.
    chain.train(["A", "B", "C", "A", "B", "C", "A", "B", "C"]);
    const dist = chain.stationaryDistribution();
    let total = 0;
    for (const p of dist.values()) total += p;
    expect(total).toBeCloseTo(1.0, 6);
  });

  it("stationary distribution covers all alphabet states", () => {
    const chain = new MarkovChain(1, 1.0);
    chain.train(["A", "B", "C", "A", "B", "C"]);
    const dist = chain.stationaryDistribution();
    expect(dist.has("A")).toBe(true);
    expect(dist.has("B")).toBe(true);
    expect(dist.has("C")).toBe(true);
  });

  it("stationary distribution values are all non-negative", () => {
    const chain = new MarkovChain(1, 1.0);
    chain.train(["A", "B", "C", "A", "B", "C"]);
    const dist = chain.stationaryDistribution();
    for (const p of dist.values()) {
      expect(p).toBeGreaterThanOrEqual(0);
    }
  });

  it("throws when chain has no states", () => {
    const chain = new MarkovChain();
    expect(() => chain.stationaryDistribution()).toThrow();
  });
});

// ---------------------------------------------------------------------------
// Test 8 — Order-2 chain: generateString("ab", 9) === "abcabcabc"
// ---------------------------------------------------------------------------
describe("Test 8 — Order-2 chain", () => {
  it('context "ab" transitions to "c" with probability 1.0', () => {
    const chain = new MarkovChain(2, 0.0);
    chain.trainString("abcabcabc");
    // The order-2 context "a\x00b" should go to "c" with prob 1.0
    expect(chain.probability("a\x00b", "c")).toBeCloseTo(1.0, 10);
  });

  it('context "bc" transitions to "a" with probability 1.0', () => {
    const chain = new MarkovChain(2, 0.0);
    chain.trainString("abcabcabc");
    expect(chain.probability("b\x00c", "a")).toBeCloseTo(1.0, 10);
  });

  it('context "ca" transitions to "b" with probability 1.0', () => {
    const chain = new MarkovChain(2, 0.0);
    chain.trainString("abcabcabc");
    expect(chain.probability("c\x00a", "b")).toBeCloseTo(1.0, 10);
  });

  it('generateString("ab", 9) === "abcabcabc"', () => {
    const chain = new MarkovChain(2, 0.0);
    chain.trainString("abcabcabc");
    expect(chain.generateString("ab", 9)).toBe("abcabcabc");
  });

  it("order-2 generate returns exactly the requested length", () => {
    const chain = new MarkovChain(2, 0.0);
    chain.trainString("abcabcabcabc");
    const result = chain.generate("a\x00b", 6);
    expect(result).toHaveLength(6);
  });
});

// ---------------------------------------------------------------------------
// Test 9 — Unknown state raises an error
// ---------------------------------------------------------------------------
describe("Test 9 — Unknown state throws", () => {
  it("nextState on an unseen state throws an error", () => {
    const chain = new MarkovChain();
    chain.train(["A", "B"]);
    expect(() => chain.nextState("UNKNOWN")).toThrow();
  });

  it("nextState throws on an empty chain", () => {
    const chain = new MarkovChain();
    expect(() => chain.nextState("A")).toThrow();
  });

  it("error message mentions the unknown state", () => {
    const chain = new MarkovChain();
    chain.train(["A", "B"]);
    expect(() => chain.nextState("Z")).toThrowError(/Z/);
  });

  it("generate throws if start state is unknown", () => {
    const chain = new MarkovChain();
    chain.train(["A", "B"]);
    // For length > 1, nextState will be called and should throw.
    expect(() => chain.generate("UNKNOWN", 2)).toThrow();
  });
});

// ---------------------------------------------------------------------------
// Test 10 — Multi-train accumulation
// ---------------------------------------------------------------------------
describe("Test 10 — Multi-train accumulation", () => {
  it("calling train() twice accumulates counts correctly", () => {
    const chain = new MarkovChain();
    // First train: A→B once
    chain.train(["A", "B"]);
    expect(chain.probability("A", "B")).toBeCloseTo(1.0, 10);

    // Second train: A→C once
    // After both: A→B=1, A→C=1, total=2
    chain.train(["A", "C"]);
    expect(chain.probability("A", "B")).toBeCloseTo(0.5, 10);
    expect(chain.probability("A", "C")).toBeCloseTo(0.5, 10);
  });

  it("multi-train on same pair increases counts", () => {
    const chain = new MarkovChain();
    chain.train(["A", "B", "A", "C"]);
    // A→B: 1, A→C: 1 → both 0.5
    expect(chain.probability("A", "B")).toBeCloseTo(0.5, 8);

    // Train more A→B transitions.
    chain.train(["A", "B", "A", "B"]);
    // Total: A→B: 3, A→C: 1 → A→B: 0.75, A→C: 0.25
    expect(chain.probability("A", "B")).toBeCloseTo(0.75, 8);
    expect(chain.probability("A", "C")).toBeCloseTo(0.25, 8);
  });

  it("multi-train rows still sum to 1.0", () => {
    const chain = new MarkovChain();
    chain.train(["A", "B", "C"]);
    chain.train(["C", "A", "B"]);
    const matrix = chain.transitionMatrix();
    for (const [, row] of matrix) {
      let sum = 0;
      for (const p of row.values()) sum += p;
      expect(sum).toBeCloseTo(1.0, 10);
    }
  });

  it("train on empty sequence does not change existing probabilities", () => {
    const chain = new MarkovChain();
    chain.train(["A", "B"]);
    const before = chain.probability("A", "B");
    chain.train([]); // no-op — too short
    chain.train(["X"]); // also too short for order=1
    expect(chain.probability("A", "B")).toBe(before);
  });
});

// ---------------------------------------------------------------------------
// Additional edge-case and coverage tests
// ---------------------------------------------------------------------------
describe("Additional coverage tests", () => {
  it("transitionMatrix returns a deep copy (mutation does not affect chain)", () => {
    const chain = new MarkovChain();
    chain.train(["A", "B"]);
    const matrix = chain.transitionMatrix();
    // Mutate the copy
    matrix.get("A")!.set("B", 99);
    // Original should be unchanged
    expect(chain.probability("A", "B")).toBeCloseTo(1.0, 10);
  });

  it("trainString works the same as training individual characters", () => {
    const chain1 = new MarkovChain();
    const chain2 = new MarkovChain();
    chain1.trainString("abc");
    chain2.train(["a", "b", "c"]);
    expect(chain1.probability("a", "b")).toBeCloseTo(
      chain2.probability("a", "b"),
      10
    );
  });

  it("generates only states from the trained alphabet", () => {
    const chain = new MarkovChain(1, 0.5);
    chain.train(["X", "Y", "Z", "X", "Y", "Z"]);
    const alphabet = new Set(["X", "Y", "Z"]);
    for (let i = 0; i < 5; i++) {
      const result = chain.generate("X", 10);
      for (const s of result) {
        expect(alphabet.has(s)).toBe(true);
      }
    }
  });

  it("order-2 generate starts from last char of context key", () => {
    const chain = new MarkovChain(2, 0.0);
    chain.trainString("abcabcabc");
    // Context "a\x00b" — last char is "b", so result[0] should be "b"
    const result = chain.generate("a\x00b", 3);
    expect(result[0]).toBe("b");
  });

  it("probability returns 0.0 for completely unknown state", () => {
    const chain = new MarkovChain();
    chain.train(["A", "B"]);
    expect(chain.probability("UNKNOWN", "A")).toBe(0.0);
  });
});
