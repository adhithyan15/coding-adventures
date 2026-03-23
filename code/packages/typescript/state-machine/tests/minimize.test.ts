/**
 * Tests for DFA minimization (Hopcroft's algorithm).
 */

import { describe, expect, it } from "vitest";
import { DFA } from "../src/dfa.js";
import { minimize } from "../src/minimize.js";
import { NFA } from "../src/nfa.js";
import { transitionKey } from "../src/types.js";

describe("Minimize Basic", () => {
  it("should not reduce an already minimal DFA", () => {
    const dfa = new DFA(
      new Set(["q0", "q1"]),
      new Set(["a", "b"]),
      new Map([
        [transitionKey("q0", "a"), "q1"],
        [transitionKey("q0", "b"), "q0"],
        [transitionKey("q1", "a"), "q0"],
        [transitionKey("q1", "b"), "q1"],
      ]),
      "q0",
      new Set(["q1"]),
    );
    const minimized = minimize(dfa);
    expect(minimized.states.size).toBe(2);
  });

  it("should merge equivalent states", () => {
    // q1 and q2 are both accepting with identical transitions (self-loops)
    const dfa = new DFA(
      new Set(["q0", "q1", "q2"]),
      new Set(["a", "b"]),
      new Map([
        [transitionKey("q0", "a"), "q1"],
        [transitionKey("q0", "b"), "q2"],
        [transitionKey("q1", "a"), "q1"],
        [transitionKey("q1", "b"), "q1"],
        [transitionKey("q2", "a"), "q2"],
        [transitionKey("q2", "b"), "q2"],
      ]),
      "q0",
      new Set(["q1", "q2"]),
    );
    const minimized = minimize(dfa);
    expect(minimized.states.size).toBe(2);
  });

  it("should remove unreachable states", () => {
    const dfa = new DFA(
      new Set(["q0", "q1", "q_dead"]),
      new Set(["a"]),
      new Map([
        [transitionKey("q0", "a"), "q1"],
        [transitionKey("q1", "a"), "q0"],
        [transitionKey("q_dead", "a"), "q_dead"],
      ]),
      "q0",
      new Set(["q1"]),
    );
    const minimized = minimize(dfa);
    expect(minimized.states.size).toBe(2);
  });

  it("should preserve the language", () => {
    const dfa = new DFA(
      new Set(["q0", "q1", "q2", "q3"]),
      new Set(["a", "b"]),
      new Map([
        [transitionKey("q0", "a"), "q1"],
        [transitionKey("q0", "b"), "q2"],
        [transitionKey("q1", "a"), "q3"],
        [transitionKey("q1", "b"), "q3"],
        [transitionKey("q2", "a"), "q3"],
        [transitionKey("q2", "b"), "q3"],
        [transitionKey("q3", "a"), "q3"],
        [transitionKey("q3", "b"), "q3"],
      ]),
      "q0",
      new Set(["q1", "q2"]),
    );
    const minimized = minimize(dfa);

    const testInputs = [["a"], ["b"], ["a", "a"], ["a", "b"], ["b", "a"], []];
    for (const events of testInputs) {
      expect(dfa.accepts(events)).toBe(minimized.accepts(events));
    }
  });

  it("should handle single-state DFA", () => {
    const dfa = new DFA(
      new Set(["q0"]),
      new Set(["a"]),
      new Map([[transitionKey("q0", "a"), "q0"]]),
      "q0",
      new Set(["q0"]),
    );
    const minimized = minimize(dfa);
    expect(minimized.states.size).toBe(1);
    expect(minimized.accepts(["a"])).toBe(true);
    expect(minimized.accepts([])).toBe(true);
  });
});

describe("Minimize with NFA", () => {
  it("should minimize DFA from NFA conversion", () => {
    // NFA for "ends with 'a'"
    const nfa = new NFA(
      new Set(["q0", "q1"]),
      new Set(["a", "b"]),
      new Map([
        [transitionKey("q0", "a"), new Set(["q0", "q1"])],
        [transitionKey("q0", "b"), new Set(["q0"])],
      ]),
      "q0",
      new Set(["q1"]),
    );
    const dfa = nfa.toDfa();
    const minimized = minimize(dfa);

    // The minimal DFA for "ends with 'a'" has exactly 2 states
    expect(minimized.states.size).toBe(2);

    // Verify language
    expect(minimized.accepts(["a"])).toBe(true);
    expect(minimized.accepts(["b", "a"])).toBe(true);
    expect(minimized.accepts(["a", "b", "a"])).toBe(true);
    expect(minimized.accepts(["b"])).toBe(false);
    expect(minimized.accepts(["a", "b"])).toBe(false);
    expect(minimized.accepts([])).toBe(false);
  });

  it("should preserve language exhaustively for strings up to length 3", () => {
    // NFA for "contains 'aa'"
    const nfa = new NFA(
      new Set(["q0", "q1", "q2"]),
      new Set(["a", "b"]),
      new Map([
        [transitionKey("q0", "a"), new Set(["q0", "q1"])],
        [transitionKey("q0", "b"), new Set(["q0"])],
        [transitionKey("q1", "a"), new Set(["q2"])],
        [transitionKey("q2", "a"), new Set(["q2"])],
        [transitionKey("q2", "b"), new Set(["q2"])],
      ]),
      "q0",
      new Set(["q2"]),
    );
    const dfa = nfa.toDfa();
    const minimized = minimize(dfa);

    // Generate all strings up to length 3
    function gen(maxLen: number): string[][] {
      let result: string[][] = [[]];
      for (let i = 0; i < maxLen; i++) {
        const newStrs: string[][] = [];
        for (const s of result) {
          for (const c of ["a", "b"]) {
            newStrs.push([...s, c]);
          }
        }
        result = result.concat(newStrs);
      }
      return result;
    }

    for (const s of gen(3)) {
      expect(nfa.accepts(s)).toBe(minimized.accepts(s));
    }
  });
});
