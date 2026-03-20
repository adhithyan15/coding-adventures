/**
 * Tests for the Pushdown Automaton (PDA) implementation.
 */

import { describe, expect, it } from "vitest";
import { PushdownAutomaton } from "../src/pda.js";
import type { PDATransition } from "../src/pda.js";

// ============================================================
// Helpers — reusable PDA definitions
// ============================================================

/** PDA that accepts balanced parentheses: (), (()), ((())), etc. */
function makeBalancedParens(): PushdownAutomaton {
  return new PushdownAutomaton(
    new Set(["q0", "accept"]),
    new Set(["(", ")"]),
    new Set(["(", "$"]),
    [
      { source: "q0", event: "(", stackRead: "$", target: "q0", stackPush: ["$", "("] },
      { source: "q0", event: "(", stackRead: "(", target: "q0", stackPush: ["(", "("] },
      { source: "q0", event: ")", stackRead: "(", target: "q0", stackPush: [] },
      { source: "q0", event: null, stackRead: "$", target: "accept", stackPush: [] },
    ],
    "q0",
    "$",
    new Set(["accept"]),
  );
}

/**
 * PDA that accepts a^n b^n: ab, aabb, aaabbb, etc.
 *
 * Strategy: push 'a' for each 'a', pop 'a' for each 'b'.
 * Accept when stack is empty (only $ remains).
 */
function makeAnBn(): PushdownAutomaton {
  return new PushdownAutomaton(
    new Set(["pushing", "popping", "accept"]),
    new Set(["a", "b"]),
    new Set(["a", "$"]),
    [
      // Push phase: reading a's
      { source: "pushing", event: "a", stackRead: "$", target: "pushing", stackPush: ["$", "a"] },
      { source: "pushing", event: "a", stackRead: "a", target: "pushing", stackPush: ["a", "a"] },
      // Switch to popping on first 'b'
      { source: "pushing", event: "b", stackRead: "a", target: "popping", stackPush: [] },
      // Pop phase: reading b's
      { source: "popping", event: "b", stackRead: "a", target: "popping", stackPush: [] },
      // Accept when stack is empty
      { source: "popping", event: null, stackRead: "$", target: "accept", stackPush: [] },
    ],
    "pushing",
    "$",
    new Set(["accept"]),
  );
}

// ============================================================
// Construction Tests
// ============================================================

describe("PDA Construction", () => {
  it("should construct a valid PDA", () => {
    const pda = makeBalancedParens();
    expect(pda.currentState).toBe("q0");
    expect(pda.stack).toEqual(["$"]);
  });

  it("should reject empty states", () => {
    expect(
      () =>
        new PushdownAutomaton(
          new Set(),
          new Set(),
          new Set(["$"]),
          [],
          "q0",
          "$",
          new Set(),
        ),
    ).toThrow(/non-empty/);
  });

  it("should reject initial not in states", () => {
    expect(
      () =>
        new PushdownAutomaton(
          new Set(["q0"]),
          new Set(),
          new Set(["$"]),
          [],
          "q_bad",
          "$",
          new Set(),
        ),
    ).toThrow(/Initial/);
  });

  it("should reject initial stack symbol not in stack alphabet", () => {
    expect(
      () =>
        new PushdownAutomaton(
          new Set(["q0"]),
          new Set(),
          new Set(["$"]),
          [],
          "q0",
          "X",
          new Set(),
        ),
    ).toThrow(/stack symbol/);
  });

  it("should reject duplicate transitions (non-deterministic)", () => {
    expect(
      () =>
        new PushdownAutomaton(
          new Set(["q0", "q1"]),
          new Set(["a"]),
          new Set(["$"]),
          [
            { source: "q0", event: "a", stackRead: "$", target: "q0", stackPush: ["$"] },
            { source: "q0", event: "a", stackRead: "$", target: "q1", stackPush: ["$"] },
          ],
          "q0",
          "$",
          new Set(),
        ),
    ).toThrow(/Duplicate/);
  });
});

// ============================================================
// Balanced Parentheses Tests
// ============================================================

describe("Balanced Parentheses PDA", () => {
  it("should accept ()", () => {
    expect(makeBalancedParens().accepts(["(", ")"])).toBe(true);
  });

  it("should accept (())", () => {
    expect(makeBalancedParens().accepts(["(", "(", ")", ")"])).toBe(true);
  });

  it("should accept ((()))", () => {
    expect(
      makeBalancedParens().accepts(["(", "(", "(", ")", ")", ")"]),
    ).toBe(true);
  });

  it("should accept ()()", () => {
    expect(makeBalancedParens().accepts(["(", ")", "(", ")"])).toBe(true);
  });

  it("should accept empty string (zero pairs)", () => {
    expect(makeBalancedParens().accepts([])).toBe(true);
  });

  it("should reject ((( (unmatched opens)", () => {
    expect(makeBalancedParens().accepts(["(", "(", "("])).toBe(false);
  });

  it("should reject ) (close without open)", () => {
    expect(makeBalancedParens().accepts([")"])).toBe(false);
  });

  it("should reject )( (wrong order)", () => {
    expect(makeBalancedParens().accepts([")", "("])).toBe(false);
  });

  it("should reject (() (partial match)", () => {
    expect(makeBalancedParens().accepts(["(", "(", ")"])).toBe(false);
  });

  it("should reject ()) (extra close)", () => {
    expect(makeBalancedParens().accepts(["(", ")", ")"])).toBe(false);
  });
});

// ============================================================
// a^n b^n Tests
// ============================================================

describe("a^n b^n PDA", () => {
  it("should accept ab (n=1)", () => {
    expect(makeAnBn().accepts(["a", "b"])).toBe(true);
  });

  it("should accept aabb (n=2)", () => {
    expect(makeAnBn().accepts(["a", "a", "b", "b"])).toBe(true);
  });

  it("should accept aaabbb (n=3)", () => {
    expect(makeAnBn().accepts(["a", "a", "a", "b", "b", "b"])).toBe(true);
  });

  it("should reject empty string", () => {
    expect(makeAnBn().accepts([])).toBe(false);
  });

  it("should reject aaa (no b's)", () => {
    expect(makeAnBn().accepts(["a", "a", "a"])).toBe(false);
  });

  it("should reject bbb (no a's)", () => {
    expect(makeAnBn().accepts(["b", "b", "b"])).toBe(false);
  });

  it("should reject aab (more a's)", () => {
    expect(makeAnBn().accepts(["a", "a", "b"])).toBe(false);
  });

  it("should reject abb (more b's)", () => {
    expect(makeAnBn().accepts(["a", "b", "b"])).toBe(false);
  });

  it("should reject abab (interleaved)", () => {
    expect(makeAnBn().accepts(["a", "b", "a", "b"])).toBe(false);
  });

  it("should reject ba (wrong order)", () => {
    expect(makeAnBn().accepts(["b", "a"])).toBe(false);
  });
});

// ============================================================
// Processing and Trace Tests
// ============================================================

describe("PDA Processing", () => {
  it("should push onto stack when processing (", () => {
    const pda = makeBalancedParens();
    pda.process("(");
    expect(pda.currentState).toBe("q0");
    expect(pda.stackTop).toBe("(");
  });

  it("should return trace from processSequence", () => {
    const pda = makeBalancedParens();
    const trace = pda.processSequence(["(", ")"]);
    // Should have at least 2 entries (push, pop) + epsilon for accept
    expect(trace.length).toBeGreaterThanOrEqual(2);
    expect(trace[0].event).toBe("(");
    expect(trace[0].source).toBe("q0");
    expect(trace[1].event).toBe(")");
  });

  it("should throw when no transition matches", () => {
    const pda = new PushdownAutomaton(
      new Set(["q0"]),
      new Set(["a"]),
      new Set(["$"]),
      [],
      "q0",
      "$",
      new Set(),
    );
    expect(() => pda.process("a")).toThrow(/No transition/);
  });

  it("should allow stack inspection after each step", () => {
    const pda = makeBalancedParens();
    pda.process("(");
    expect(pda.stack).toEqual(["$", "("]);
    expect(pda.stackTop).toBe("(");

    pda.process("(");
    expect(pda.stack).toEqual(["$", "(", "("]);
    expect(pda.stackTop).toBe("(");

    pda.process(")");
    expect(pda.stack).toEqual(["$", "("]);

    pda.process(")");
    expect(pda.stack).toEqual(["$"]);
  });
});

// ============================================================
// Reset Tests
// ============================================================

describe("PDA Reset", () => {
  it("should restore initial state and stack", () => {
    const pda = makeBalancedParens();
    pda.process("(");
    pda.process("(");
    expect(pda.stackTop).toBe("(");

    pda.reset();
    expect(pda.currentState).toBe("q0");
    expect(pda.stack).toEqual(["$"]);
    expect(pda.trace).toEqual([]);
  });
});

// ============================================================
// Accepts Non-Mutating Tests
// ============================================================

describe("PDA Accepts Non-Mutating", () => {
  it("should not modify state or stack", () => {
    const pda = makeBalancedParens();
    pda.process("(");
    const originalState = pda.currentState;
    const originalStack = [...pda.stack];

    pda.accepts([")", "(", ")"]);

    expect(pda.currentState).toBe(originalState);
    expect(pda.stack).toEqual(originalStack);
  });
});
