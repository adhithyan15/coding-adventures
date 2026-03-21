/**
 * Tests for the NFA (Non-deterministic Finite Automaton) implementation.
 *
 * These tests cover:
 * 1. Construction and validation
 * 2. Epsilon closure computation
 * 3. Processing events (non-deterministic branching)
 * 4. Acceptance checking
 * 5. Subset construction (NFA -> DFA conversion)
 * 6. Visualization
 * 7. Classic examples
 */

import { describe, expect, it } from "vitest";
import { EPSILON, NFA } from "../src/nfa.js";
import { transitionKey } from "../src/types.js";

// ============================================================
// Helpers — reusable NFA definitions
// ============================================================

/** NFA that accepts strings containing 'ab' as a substring. */
function makeContainsAb(): NFA {
  return new NFA(
    new Set(["q0", "q1", "q2"]),
    new Set(["a", "b"]),
    new Map([
      [transitionKey("q0", "a"), new Set(["q0", "q1"])],
      [transitionKey("q0", "b"), new Set(["q0"])],
      [transitionKey("q1", "b"), new Set(["q2"])],
      [transitionKey("q2", "a"), new Set(["q2"])],
      [transitionKey("q2", "b"), new Set(["q2"])],
    ]),
    "q0",
    new Set(["q2"]),
  );
}

/** NFA with a chain of epsilon transitions: q0 --e--> q1 --e--> q2. */
function makeEpsilonChain(): NFA {
  return new NFA(
    new Set(["q0", "q1", "q2", "q3"]),
    new Set(["a"]),
    new Map([
      [transitionKey("q0", EPSILON), new Set(["q1"])],
      [transitionKey("q1", EPSILON), new Set(["q2"])],
      [transitionKey("q2", "a"), new Set(["q3"])],
    ]),
    "q0",
    new Set(["q3"]),
  );
}

/** NFA that accepts "a" or "ab" using epsilon transitions. */
function makeAOrAb(): NFA {
  return new NFA(
    new Set(["q0", "q1", "q2", "q3", "q4", "q5"]),
    new Set(["a", "b"]),
    new Map([
      [transitionKey("q0", EPSILON), new Set(["q1", "q3"])],
      [transitionKey("q1", "a"), new Set(["q2"])],
      [transitionKey("q3", "a"), new Set(["q4"])],
      [transitionKey("q4", "b"), new Set(["q5"])],
    ]),
    "q0",
    new Set(["q2", "q5"]),
  );
}

// ============================================================
// Construction Tests
// ============================================================

describe("NFA Construction", () => {
  it("should construct a valid NFA", () => {
    const nfa = makeContainsAb();
    expect(nfa.states).toEqual(new Set(["q0", "q1", "q2"]));
    expect(nfa.alphabet).toEqual(new Set(["a", "b"]));
    expect(nfa.initial).toBe("q0");
    expect(nfa.accepting).toEqual(new Set(["q2"]));
  });

  it("should reject empty states", () => {
    expect(
      () =>
        new NFA(new Set(), new Set(["a"]), new Map(), "q0", new Set()),
    ).toThrow(/non-empty/);
  });

  it("should reject epsilon in alphabet", () => {
    expect(
      () =>
        new NFA(
          new Set(["q0"]),
          new Set(["a", ""]),
          new Map(),
          "q0",
          new Set(),
        ),
    ).toThrow(/epsilon/);
  });

  it("should reject initial not in states", () => {
    expect(
      () =>
        new NFA(
          new Set(["q0"]),
          new Set(["a"]),
          new Map(),
          "q_bad",
          new Set(),
        ),
    ).toThrow(/Initial/);
  });

  it("should reject accepting not subset", () => {
    expect(
      () =>
        new NFA(
          new Set(["q0"]),
          new Set(["a"]),
          new Map(),
          "q0",
          new Set(["q_bad"]),
        ),
    ).toThrow(/Accepting/);
  });

  it("should reject transition source not in states", () => {
    expect(
      () =>
        new NFA(
          new Set(["q0"]),
          new Set(["a"]),
          new Map([[transitionKey("q_bad", "a"), new Set(["q0"])]]),
          "q0",
          new Set(),
        ),
    ).toThrow(/source/);
  });

  it("should reject transition event not in alphabet and not epsilon", () => {
    expect(
      () =>
        new NFA(
          new Set(["q0"]),
          new Set(["a"]),
          new Map([[transitionKey("q0", "z"), new Set(["q0"])]]),
          "q0",
          new Set(),
        ),
    ).toThrow(/alphabet/);
  });

  it("should reject transition target not in states", () => {
    expect(
      () =>
        new NFA(
          new Set(["q0"]),
          new Set(["a"]),
          new Map([[transitionKey("q0", "a"), new Set(["q_bad"])]]),
          "q0",
          new Set(),
        ),
    ).toThrow(/target/);
  });
});

// ============================================================
// Epsilon Closure Tests
// ============================================================

describe("Epsilon Closure", () => {
  it("should return input set when no epsilon transitions", () => {
    const nfa = makeContainsAb();
    expect(nfa.epsilonClosure(new Set(["q0"]))).toEqual(new Set(["q0"]));
  });

  it("should follow single epsilon transition", () => {
    const nfa = new NFA(
      new Set(["q0", "q1"]),
      new Set(["a"]),
      new Map([[transitionKey("q0", EPSILON), new Set(["q1"])]]),
      "q0",
      new Set(),
    );
    expect(nfa.epsilonClosure(new Set(["q0"]))).toEqual(
      new Set(["q0", "q1"]),
    );
  });

  it("should follow chained epsilons", () => {
    const nfa = makeEpsilonChain();
    expect(nfa.epsilonClosure(new Set(["q0"]))).toEqual(
      new Set(["q0", "q1", "q2"]),
    );
  });

  it("should handle epsilon cycles without infinite loop", () => {
    const nfa = new NFA(
      new Set(["q0", "q1"]),
      new Set(["a"]),
      new Map([
        [transitionKey("q0", EPSILON), new Set(["q1"])],
        [transitionKey("q1", EPSILON), new Set(["q0"])],
      ]),
      "q0",
      new Set(),
    );
    expect(nfa.epsilonClosure(new Set(["q0"]))).toEqual(
      new Set(["q0", "q1"]),
    );
  });

  it("should follow branching epsilons", () => {
    const nfa = makeAOrAb();
    expect(nfa.epsilonClosure(new Set(["q0"]))).toEqual(
      new Set(["q0", "q1", "q3"]),
    );
  });

  it("should compute closure for multiple states", () => {
    const nfa = makeEpsilonChain();
    const result = nfa.epsilonClosure(new Set(["q0", "q3"]));
    expect(result).toEqual(new Set(["q0", "q1", "q2", "q3"]));
  });

  it("should return empty set for empty input", () => {
    const nfa = makeEpsilonChain();
    expect(nfa.epsilonClosure(new Set())).toEqual(new Set());
  });
});

// ============================================================
// Processing Tests
// ============================================================

describe("NFA Processing", () => {
  it("should start in epsilon closure of initial state", () => {
    const nfa = makeEpsilonChain();
    expect(nfa.currentStates).toEqual(new Set(["q0", "q1", "q2"]));
  });

  it("should process deterministic case", () => {
    const nfa = makeContainsAb();
    nfa.process("b");
    expect(nfa.currentStates).toEqual(new Set(["q0"]));
  });

  it("should handle non-deterministic branching", () => {
    const nfa = makeContainsAb();
    nfa.process("a");
    expect(nfa.currentStates).toEqual(new Set(["q0", "q1"]));
  });

  it("should let dead paths vanish", () => {
    const nfa = makeContainsAb();
    nfa.process("a"); // {q0, q1}
    nfa.process("a"); // q0->{q0,q1}, q1 has no 'a' -> dies
    expect(nfa.currentStates).toEqual(new Set(["q0", "q1"]));
  });

  it("should reach accepting state", () => {
    const nfa = makeContainsAb();
    nfa.process("a");
    nfa.process("b");
    expect(nfa.currentStates.has("q2")).toBe(true);
  });

  it("should process through epsilon chain", () => {
    const nfa = makeEpsilonChain();
    nfa.process("a");
    expect(nfa.currentStates).toEqual(new Set(["q3"]));
  });

  it("should throw on invalid event", () => {
    const nfa = makeContainsAb();
    expect(() => nfa.process("c")).toThrow(/not in the alphabet/);
  });

  it("should return trace from processSequence", () => {
    const nfa = makeContainsAb();
    const trace = nfa.processSequence(["a", "b"]);
    expect(trace.length).toBe(2);
    const [before, event, after] = trace[0];
    expect(event).toBe("a");
    expect(before.has("q0")).toBe(true);
    expect(after.has("q1")).toBe(true);
    const [, event2, after2] = trace[1];
    expect(event2).toBe("b");
    expect(after2.has("q2")).toBe(true);
  });
});

// ============================================================
// Acceptance Tests
// ============================================================

describe("NFA Acceptance", () => {
  it("should accept strings containing ab", () => {
    const nfa = makeContainsAb();
    expect(nfa.accepts(["a", "b"])).toBe(true);
    expect(nfa.accepts(["b", "a", "b"])).toBe(true);
    expect(nfa.accepts(["a", "a", "b"])).toBe(true);
    expect(nfa.accepts(["a", "b", "a", "b"])).toBe(true);
  });

  it("should reject strings not containing ab", () => {
    const nfa = makeContainsAb();
    expect(nfa.accepts(["a"])).toBe(false);
    expect(nfa.accepts(["b"])).toBe(false);
    expect(nfa.accepts(["b", "a"])).toBe(false);
    expect(nfa.accepts(["b", "b", "b"])).toBe(false);
    expect(nfa.accepts([])).toBe(false);
  });

  it("should accept 'a' and 'ab' with epsilon NFA", () => {
    const nfa = makeAOrAb();
    expect(nfa.accepts(["a"])).toBe(true);
    expect(nfa.accepts(["a", "b"])).toBe(true);
  });

  it("should reject invalid strings for a-or-ab NFA", () => {
    const nfa = makeAOrAb();
    expect(nfa.accepts([])).toBe(false);
    expect(nfa.accepts(["b"])).toBe(false);
    expect(nfa.accepts(["a", "a"])).toBe(false);
    expect(nfa.accepts(["a", "b", "a"])).toBe(false);
  });

  it("should accept single 'a' via epsilon chain", () => {
    const nfa = makeEpsilonChain();
    expect(nfa.accepts(["a"])).toBe(true);
  });

  it("should reject empty and multi-char for epsilon chain", () => {
    const nfa = makeEpsilonChain();
    expect(nfa.accepts([])).toBe(false);
    expect(nfa.accepts(["a", "a"])).toBe(false);
  });

  it("should not modify state when checking acceptance", () => {
    const nfa = makeContainsAb();
    const original = new Set(nfa.currentStates);
    nfa.accepts(["a", "b", "a"]);
    expect(nfa.currentStates).toEqual(original);
  });

  it("should throw on invalid event in accepts", () => {
    const nfa = makeContainsAb();
    expect(() => nfa.accepts(["c"])).toThrow(/not in the alphabet/);
  });

  it("should reject early when NFA reaches empty state set", () => {
    const nfa = new NFA(
      new Set(["q0", "q1"]),
      new Set(["a", "b"]),
      new Map([[transitionKey("q0", "a"), new Set(["q1"])]]),
      "q0",
      new Set(["q1"]),
    );
    expect(nfa.accepts(["b"])).toBe(false);
    expect(nfa.accepts(["b", "a"])).toBe(false);
  });
});

// ============================================================
// Subset Construction Tests (NFA -> DFA)
// ============================================================

describe("Subset Construction", () => {
  it("should convert deterministic NFA cleanly", () => {
    const nfa = new NFA(
      new Set(["q0", "q1"]),
      new Set(["a", "b"]),
      new Map([
        [transitionKey("q0", "a"), new Set(["q1"])],
        [transitionKey("q0", "b"), new Set(["q0"])],
        [transitionKey("q1", "a"), new Set(["q0"])],
        [transitionKey("q1", "b"), new Set(["q1"])],
      ]),
      "q0",
      new Set(["q1"]),
    );
    const dfa = nfa.toDfa();
    expect(dfa.states.size).toBe(2);
    expect(dfa.accepts(["a"])).toBe(true);
    expect(dfa.accepts(["a", "a"])).toBe(false);
    expect(dfa.accepts(["a", "b"])).toBe(true);
  });

  it("should convert contains-ab NFA to equivalent DFA", () => {
    const nfa = makeContainsAb();
    const dfa = nfa.toDfa();

    const testCases: [string[], boolean][] = [
      [["a", "b"], true],
      [["b", "a", "b"], true],
      [["a", "a", "b"], true],
      [["a"], false],
      [["b"], false],
      [["b", "a"], false],
      [[], false],
    ];
    for (const [events, expected] of testCases) {
      expect(dfa.accepts(events)).toBe(expected);
    }
  });

  it("should convert epsilon NFA correctly", () => {
    const nfa = makeAOrAb();
    const dfa = nfa.toDfa();

    expect(dfa.accepts(["a"])).toBe(true);
    expect(dfa.accepts(["a", "b"])).toBe(true);
    expect(dfa.accepts([])).toBe(false);
    expect(dfa.accepts(["b"])).toBe(false);
    expect(dfa.accepts(["a", "a"])).toBe(false);
  });

  it("should convert epsilon chain NFA correctly", () => {
    const nfa = makeEpsilonChain();
    const dfa = nfa.toDfa();

    expect(dfa.accepts(["a"])).toBe(true);
    expect(dfa.accepts([])).toBe(false);
    expect(dfa.accepts(["a", "a"])).toBe(false);
  });

  it("should produce valid DFA from conversion", () => {
    const nfa = makeContainsAb();
    const dfa = nfa.toDfa();
    const warnings = dfa.validate();
    for (const w of warnings) {
      expect(w).not.toContain("Unreachable");
    }
  });

  it("should be language-equivalent for all strings up to length 4", () => {
    // NFA for "ends with 'ab'"
    const nfa = new NFA(
      new Set(["q0", "q1", "q2"]),
      new Set(["a", "b"]),
      new Map([
        [transitionKey("q0", "a"), new Set(["q0", "q1"])],
        [transitionKey("q0", "b"), new Set(["q0"])],
        [transitionKey("q1", "b"), new Set(["q2"])],
      ]),
      "q0",
      new Set(["q2"]),
    );
    const dfa = nfa.toDfa();

    // Generate all strings of a,b up to length 4
    function genStrings(
      alpha: string[],
      maxLen: number,
    ): string[][] {
      let result: string[][] = [[]];
      for (let len = 1; len <= maxLen; len++) {
        const newStrs: string[][] = [];
        for (const s of result.filter((r) => r.length === len - 1)) {
          for (const c of alpha) {
            newStrs.push([...s, c]);
          }
        }
        result = result.concat(newStrs);
      }
      return result;
    }

    for (const s of genStrings(["a", "b"], 4)) {
      expect(nfa.accepts(s)).toBe(dfa.accepts(s));
    }
  });
});

// ============================================================
// Reset Tests
// ============================================================

describe("NFA Reset", () => {
  it("should return to epsilon closure of initial", () => {
    const nfa = makeContainsAb();
    nfa.process("a");
    expect(nfa.currentStates.has("q1")).toBe(true);

    nfa.reset();
    expect(nfa.currentStates).toEqual(new Set(["q0"]));
  });

  it("should re-compute epsilon closure on reset", () => {
    const nfa = makeEpsilonChain();
    nfa.process("a");
    expect(nfa.currentStates).toEqual(new Set(["q3"]));

    nfa.reset();
    expect(nfa.currentStates).toEqual(new Set(["q0", "q1", "q2"]));
  });
});

// ============================================================
// Visualization Tests
// ============================================================

describe("NFA Visualization", () => {
  it("should generate DOT with expected structure", () => {
    const nfa = makeContainsAb();
    const dot = nfa.toDot();
    expect(dot).toContain("digraph NFA");
    expect(dot).toContain("__start");
    expect(dot).toContain("doublecircle");
    expect(dot).toContain("q0");
    expect(dot).toContain("q1");
    expect(dot).toContain("q2");
  });

  it("should label epsilon transitions with epsilon symbol in DOT", () => {
    const nfa = makeEpsilonChain();
    const dot = nfa.toDot();
    expect(dot).toContain("\u03B5");
  });
});
