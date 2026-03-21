/**
 * Tests for the DFA (Deterministic Finite Automaton) implementation.
 *
 * These tests cover:
 * 1. Construction and validation
 * 2. Processing single events and sequences
 * 3. Acceptance checking
 * 4. Introspection (reachability, completeness, validation)
 * 5. Visualization (DOT and ASCII output)
 * 6. Classic examples (turnstile, binary div-by-3, branch predictor)
 * 7. Error cases
 */

import { describe, expect, it } from "vitest";
import { DFA } from "../src/dfa.js";
import { transitionKey } from "../src/types.js";
import type { TransitionRecord } from "../src/types.js";

// ============================================================
// Helper — reusable DFA definitions
// ============================================================

/** The classic turnstile: insert coin to unlock, push to lock. */
function makeTurnstile(): DFA {
  return new DFA(
    new Set(["locked", "unlocked"]),
    new Set(["coin", "push"]),
    new Map([
      [transitionKey("locked", "coin"), "unlocked"],
      [transitionKey("locked", "push"), "locked"],
      [transitionKey("unlocked", "coin"), "unlocked"],
      [transitionKey("unlocked", "push"), "locked"],
    ]),
    "locked",
    new Set(["unlocked"]),
  );
}

/**
 * DFA that accepts binary strings representing numbers divisible by 3.
 *
 * States represent the remainder when divided by 3:
 *   r0 = remainder 0 (divisible by 3) — accepting
 *   r1 = remainder 1
 *   r2 = remainder 2
 *
 * Transition logic: new_remainder = (old_remainder * 2 + bit) mod 3
 */
function makeDivBy3(): DFA {
  return new DFA(
    new Set(["r0", "r1", "r2"]),
    new Set(["0", "1"]),
    new Map([
      [transitionKey("r0", "0"), "r0"], // (0*2+0) mod 3 = 0
      [transitionKey("r0", "1"), "r1"], // (0*2+1) mod 3 = 1
      [transitionKey("r1", "0"), "r2"], // (1*2+0) mod 3 = 2
      [transitionKey("r1", "1"), "r0"], // (1*2+1) mod 3 = 0
      [transitionKey("r2", "0"), "r1"], // (2*2+0) mod 3 = 1
      [transitionKey("r2", "1"), "r2"], // (2*2+1) mod 3 = 2
    ]),
    "r0",
    new Set(["r0"]),
  );
}

/**
 * 2-bit saturating counter branch predictor as a DFA.
 *
 * States: SNT (strongly not-taken), WNT (weakly not-taken),
 *         WT (weakly taken), ST (strongly taken)
 */
function makeBranchPredictor(): DFA {
  return new DFA(
    new Set(["SNT", "WNT", "WT", "ST"]),
    new Set(["taken", "not_taken"]),
    new Map([
      [transitionKey("SNT", "taken"), "WNT"],
      [transitionKey("SNT", "not_taken"), "SNT"],
      [transitionKey("WNT", "taken"), "WT"],
      [transitionKey("WNT", "not_taken"), "SNT"],
      [transitionKey("WT", "taken"), "ST"],
      [transitionKey("WT", "not_taken"), "WNT"],
      [transitionKey("ST", "taken"), "ST"],
      [transitionKey("ST", "not_taken"), "WT"],
    ]),
    "WNT",
    new Set(["WT", "ST"]), // states that predict "taken"
  );
}

// ============================================================
// Construction and Validation Tests
// ============================================================

describe("DFA Construction", () => {
  it("should construct a valid DFA without errors", () => {
    const t = makeTurnstile();
    expect(t.currentState).toBe("locked");
    expect(t.initial).toBe("locked");
    expect(t.states).toEqual(new Set(["locked", "unlocked"]));
    expect(t.alphabet).toEqual(new Set(["coin", "push"]));
    expect(t.accepting).toEqual(new Set(["unlocked"]));
  });

  it("should reject empty states set", () => {
    expect(() =>
      new DFA(
        new Set(),
        new Set(["a"]),
        new Map(),
        "q0",
        new Set(),
      ),
    ).toThrow(/non-empty/);
  });

  it("should reject initial state not in states", () => {
    expect(() =>
      new DFA(
        new Set(["q0", "q1"]),
        new Set(["a"]),
        new Map([[transitionKey("q0", "a"), "q1"]]),
        "q_missing",
        new Set(),
      ),
    ).toThrow(/Initial state/);
  });

  it("should reject accepting states not subset of states", () => {
    expect(() =>
      new DFA(
        new Set(["q0", "q1"]),
        new Set(["a"]),
        new Map([[transitionKey("q0", "a"), "q1"]]),
        "q0",
        new Set(["q_missing"]),
      ),
    ).toThrow(/Accepting state/);
  });

  it("should reject transition source not in states", () => {
    expect(() =>
      new DFA(
        new Set(["q0"]),
        new Set(["a"]),
        new Map([[transitionKey("q_bad", "a"), "q0"]]),
        "q0",
        new Set(),
      ),
    ).toThrow(/source/);
  });

  it("should reject transition event not in alphabet", () => {
    expect(() =>
      new DFA(
        new Set(["q0"]),
        new Set(["a"]),
        new Map([[transitionKey("q0", "b"), "q0"]]),
        "q0",
        new Set(),
      ),
    ).toThrow(/alphabet/);
  });

  it("should reject transition target not in states", () => {
    expect(() =>
      new DFA(
        new Set(["q0"]),
        new Set(["a"]),
        new Map([[transitionKey("q0", "a"), "q_bad"]]),
        "q0",
        new Set(),
      ),
    ).toThrow(/target/);
  });

  it("should reject action without transition", () => {
    expect(() =>
      new DFA(
        new Set(["q0"]),
        new Set(["a"]),
        new Map([[transitionKey("q0", "a"), "q0"]]),
        "q0",
        new Set(),
        new Map([[transitionKey("q0", "b"), (_s, _e, _t) => {}]]),
      ),
    ).toThrow(/no transition/);
  });

  it("should allow empty accepting set", () => {
    const dfa = new DFA(
      new Set(["q0"]),
      new Set(["a"]),
      new Map([[transitionKey("q0", "a"), "q0"]]),
      "q0",
      new Set(),
    );
    expect(dfa.accepting.size).toBe(0);
  });

  it("should return copy of transitions", () => {
    const t = makeTurnstile();
    const t1 = t.transitions;
    const t2 = t.transitions;
    expect(t1).toEqual(t2);
    expect(t1).not.toBe(t2);
  });
});

// ============================================================
// Processing Tests
// ============================================================

describe("DFA Processing", () => {
  it("should process single event", () => {
    const t = makeTurnstile();
    const result = t.process("coin");
    expect(result).toBe("unlocked");
    expect(t.currentState).toBe("unlocked");
  });

  it("should process multiple events sequentially", () => {
    const t = makeTurnstile();
    t.process("coin");
    expect(t.currentState).toBe("unlocked");
    t.process("push");
    expect(t.currentState).toBe("locked");
    t.process("coin");
    expect(t.currentState).toBe("unlocked");
    t.process("coin");
    expect(t.currentState).toBe("unlocked");
  });

  it("should build trace from process calls", () => {
    const t = makeTurnstile();
    t.process("coin");
    t.process("push");

    const trace = t.trace;
    expect(trace.length).toBe(2);
    expect(trace[0]).toEqual({
      source: "locked",
      event: "coin",
      target: "unlocked",
      actionName: null,
    });
    expect(trace[1]).toEqual({
      source: "unlocked",
      event: "push",
      target: "locked",
      actionName: null,
    });
  });

  it("should process sequence and return trace", () => {
    const t = makeTurnstile();
    const trace = t.processSequence(["coin", "push", "coin"]);
    expect(trace.length).toBe(3);
    expect(trace[0].source).toBe("locked");
    expect(trace[0].target).toBe("unlocked");
    expect(trace[1].source).toBe("unlocked");
    expect(trace[1].target).toBe("locked");
    expect(trace[2].source).toBe("locked");
    expect(trace[2].target).toBe("unlocked");
  });

  it("should return empty trace for empty sequence", () => {
    const t = makeTurnstile();
    const trace = t.processSequence([]);
    expect(trace).toEqual([]);
    expect(t.currentState).toBe("locked");
  });

  it("should throw on invalid event", () => {
    const t = makeTurnstile();
    expect(() => t.process("kick")).toThrow(/not in the alphabet/);
  });

  it("should throw on undefined transition", () => {
    const dfa = new DFA(
      new Set(["q0", "q1"]),
      new Set(["a", "b"]),
      new Map([[transitionKey("q0", "a"), "q1"]]),
      "q0",
      new Set(),
    );
    expect(() => dfa.process("b")).toThrow(/No transition/);
  });

  it("should handle self-loops", () => {
    const dfa = new DFA(
      new Set(["q0"]),
      new Set(["a"]),
      new Map([[transitionKey("q0", "a"), "q0"]]),
      "q0",
      new Set(["q0"]),
    );
    dfa.process("a");
    expect(dfa.currentState).toBe("q0");
    dfa.process("a");
    expect(dfa.currentState).toBe("q0");
  });

  it("should fire actions with correct arguments", () => {
    const log: Array<[string, string, string]> = [];

    function logger(source: string, event: string, target: string): void {
      log.push([source, event, target]);
    }

    const dfa = new DFA(
      new Set(["a", "b"]),
      new Set(["x"]),
      new Map([
        [transitionKey("a", "x"), "b"],
        [transitionKey("b", "x"), "a"],
      ]),
      "a",
      new Set(),
      new Map([[transitionKey("a", "x"), logger]]),
    );
    dfa.process("x");
    expect(log).toEqual([["a", "x", "b"]]);
    dfa.process("x");
    expect(log.length).toBe(1); // action only on (a, x), not (b, x)
  });

  it("should record action name in trace", () => {
    function myAction(_source: string, _event: string, _target: string): void {}

    const dfa = new DFA(
      new Set(["a", "b"]),
      new Set(["x"]),
      new Map([
        [transitionKey("a", "x"), "b"],
        [transitionKey("b", "x"), "a"],
      ]),
      "a",
      new Set(),
      new Map([[transitionKey("a", "x"), myAction]]),
    );
    dfa.process("x");
    expect(dfa.trace[0].actionName).toBe("myAction");
  });
});

// ============================================================
// Acceptance Tests
// ============================================================

describe("DFA Acceptance", () => {
  it("should accept sequences ending in accepting state", () => {
    const t = makeTurnstile();
    expect(t.accepts(["coin"])).toBe(true);
    expect(t.accepts(["coin", "push"])).toBe(false);
    expect(t.accepts(["coin", "push", "coin"])).toBe(true);
  });

  it("should handle empty input based on initial state", () => {
    const t = makeTurnstile();
    expect(t.accepts([])).toBe(false); // locked is not accepting

    // DFA where initial IS accepting
    const dfa = new DFA(
      new Set(["q0"]),
      new Set(["a"]),
      new Map([[transitionKey("q0", "a"), "q0"]]),
      "q0",
      new Set(["q0"]),
    );
    expect(dfa.accepts([])).toBe(true);
  });

  it("should not modify state when checking acceptance", () => {
    const t = makeTurnstile();
    t.process("coin");
    expect(t.currentState).toBe("unlocked");

    t.accepts(["push", "push", "push"]);
    expect(t.currentState).toBe("unlocked"); // unchanged
  });

  it("should not modify trace when checking acceptance", () => {
    const t = makeTurnstile();
    t.process("coin");
    const traceLen = t.trace.length;

    t.accepts(["push", "coin"]);
    expect(t.trace.length).toBe(traceLen); // unchanged
  });

  it("should return false on undefined transition (no crash)", () => {
    const dfa = new DFA(
      new Set(["q0", "q1"]),
      new Set(["a", "b"]),
      new Map([[transitionKey("q0", "a"), "q1"]]),
      "q0",
      new Set(["q1"]),
    );
    expect(dfa.accepts(["a"])).toBe(true);
    expect(dfa.accepts(["b"])).toBe(false); // no transition, graceful reject
  });

  it("should throw on invalid event in accepts", () => {
    const t = makeTurnstile();
    expect(() => t.accepts(["kick"])).toThrow(/not in the alphabet/);
  });

  it("should correctly check binary divisibility by 3", () => {
    const d = makeDivBy3();
    // 0 = 0 (div by 3) — empty string starts in r0 which is accepting
    expect(d.accepts([])).toBe(true);
    // 1 = 1 (not div by 3)
    expect(d.accepts(["1"])).toBe(false);
    // 10 = 2
    expect(d.accepts(["1", "0"])).toBe(false);
    // 11 = 3
    expect(d.accepts(["1", "1"])).toBe(true);
    // 100 = 4
    expect(d.accepts(["1", "0", "0"])).toBe(false);
    // 110 = 6
    expect(d.accepts(["1", "1", "0"])).toBe(true);
    // 1001 = 9
    expect(d.accepts(["1", "0", "0", "1"])).toBe(true);
    // 1100 = 12
    expect(d.accepts(["1", "1", "0", "0"])).toBe(true);
    // 1111 = 15
    expect(d.accepts(["1", "1", "1", "1"])).toBe(true);
    // 10000 = 16
    expect(d.accepts(["1", "0", "0", "0", "0"])).toBe(false);
  });
});

// ============================================================
// Branch Predictor as DFA Tests
// ============================================================

describe("Branch Predictor DFA", () => {
  it("should start in WNT", () => {
    const bp = makeBranchPredictor();
    expect(bp.currentState).toBe("WNT");
  });

  it("should warm up to strongly taken", () => {
    const bp = makeBranchPredictor();
    bp.process("taken");
    expect(bp.currentState).toBe("WT");
    bp.process("taken");
    expect(bp.currentState).toBe("ST");
  });

  it("should saturate at ST", () => {
    const bp = makeBranchPredictor();
    bp.processSequence(["taken", "taken", "taken", "taken"]);
    expect(bp.currentState).toBe("ST");
  });

  it("should saturate at SNT", () => {
    const bp = makeBranchPredictor();
    bp.processSequence(["not_taken", "not_taken", "not_taken"]);
    expect(bp.currentState).toBe("SNT");
  });

  it("should exhibit hysteresis", () => {
    const bp = makeBranchPredictor();
    bp.processSequence(["taken", "taken"]);
    expect(bp.currentState).toBe("ST");

    bp.process("not_taken");
    expect(bp.currentState).toBe("WT");
    expect(bp.accepting.has("WT")).toBe(true); // still predicts taken
  });

  it("should handle loop pattern", () => {
    const bp = makeBranchPredictor();
    const pattern = Array(9).fill("taken").concat(["not_taken"]);
    bp.processSequence(pattern);
    expect(bp.currentState).toBe("WT");
    expect(bp.accepting.has(bp.currentState)).toBe(true);
  });

  it("should predict via accepting states", () => {
    const bp = makeBranchPredictor();
    // WNT is not accepting (predicts not-taken)
    expect(bp.accepting.has(bp.currentState)).toBe(false);

    // After one 'taken': WT is accepting (predicts taken)
    bp.process("taken");
    expect(bp.accepting.has(bp.currentState)).toBe(true);
  });
});

// ============================================================
// Reset Tests
// ============================================================

describe("DFA Reset", () => {
  it("should return to initial state", () => {
    const t = makeTurnstile();
    t.process("coin");
    expect(t.currentState).toBe("unlocked");

    t.reset();
    expect(t.currentState).toBe("locked");
  });

  it("should clear trace", () => {
    const t = makeTurnstile();
    t.processSequence(["coin", "push", "coin"]);
    expect(t.trace.length).toBe(3);

    t.reset();
    expect(t.trace).toEqual([]);
  });
});

// ============================================================
// Introspection Tests
// ============================================================

describe("DFA Introspection", () => {
  it("should find all reachable states", () => {
    const t = makeTurnstile();
    expect(t.reachableStates()).toEqual(new Set(["locked", "unlocked"]));
  });

  it("should exclude unreachable states", () => {
    const dfa = new DFA(
      new Set(["q0", "q1", "q_dead"]),
      new Set(["a"]),
      new Map([
        [transitionKey("q0", "a"), "q1"],
        [transitionKey("q1", "a"), "q0"],
      ]),
      "q0",
      new Set(),
    );
    expect(dfa.reachableStates()).toEqual(new Set(["q0", "q1"]));
  });

  it("should detect complete DFA", () => {
    const t = makeTurnstile();
    expect(t.isComplete()).toBe(true);
  });

  it("should detect incomplete DFA", () => {
    const dfa = new DFA(
      new Set(["q0", "q1"]),
      new Set(["a", "b"]),
      new Map([[transitionKey("q0", "a"), "q1"]]),
      "q0",
      new Set(),
    );
    expect(dfa.isComplete()).toBe(false);
  });

  it("should validate clean DFA with no warnings", () => {
    const t = makeTurnstile();
    expect(t.validate()).toEqual([]);
  });

  it("should report unreachable states", () => {
    const dfa = new DFA(
      new Set(["q0", "q1", "q_dead"]),
      new Set(["a"]),
      new Map([
        [transitionKey("q0", "a"), "q1"],
        [transitionKey("q1", "a"), "q0"],
        [transitionKey("q_dead", "a"), "q_dead"],
      ]),
      "q0",
      new Set(),
    );
    const warnings = dfa.validate();
    expect(warnings.some((w) => w.includes("Unreachable"))).toBe(true);
    expect(warnings.some((w) => w.includes("q_dead"))).toBe(true);
  });

  it("should report unreachable accepting states", () => {
    const dfa = new DFA(
      new Set(["q0", "q_dead"]),
      new Set(["a"]),
      new Map([
        [transitionKey("q0", "a"), "q0"],
        [transitionKey("q_dead", "a"), "q_dead"],
      ]),
      "q0",
      new Set(["q_dead"]),
    );
    const warnings = dfa.validate();
    expect(warnings.some((w) => w.includes("Unreachable accepting"))).toBe(
      true,
    );
  });

  it("should report missing transitions", () => {
    const dfa = new DFA(
      new Set(["q0", "q1"]),
      new Set(["a", "b"]),
      new Map([[transitionKey("q0", "a"), "q1"]]),
      "q0",
      new Set(),
    );
    const warnings = dfa.validate();
    expect(warnings.some((w) => w.includes("Missing transitions"))).toBe(true);
  });
});

// ============================================================
// Visualization Tests
// ============================================================

describe("DFA Visualization", () => {
  it("should generate DOT with expected structure", () => {
    const t = makeTurnstile();
    const dot = t.toDot();
    expect(dot).toContain("digraph DFA");
    expect(dot).toContain("__start");
    expect(dot).toContain("doublecircle");
    expect(dot).toContain("locked");
    expect(dot).toContain("unlocked");
    expect(dot).toContain("coin");
    expect(dot).toContain("push");
    expect(dot.endsWith("}")).toBe(true);
  });

  it("should have initial arrow in DOT", () => {
    const t = makeTurnstile();
    const dot = t.toDot();
    expect(dot).toContain('__start -> "locked"');
  });

  it("should mark accepting states with doublecircle in DOT", () => {
    const t = makeTurnstile();
    const dot = t.toDot();
    expect(dot).toContain('"unlocked" [shape=doublecircle]');
    expect(dot).toContain('"locked" [shape=circle]');
  });

  it("should include all states and events in ASCII table", () => {
    const t = makeTurnstile();
    const ascii = t.toAscii();
    expect(ascii).toContain("locked");
    expect(ascii).toContain("unlocked");
    expect(ascii).toContain("coin");
    expect(ascii).toContain("push");
  });

  it("should mark initial state with > in ASCII table", () => {
    const t = makeTurnstile();
    const ascii = t.toAscii();
    expect(ascii).toContain(">");
  });

  it("should mark accepting states with * in ASCII table", () => {
    const t = makeTurnstile();
    const ascii = t.toAscii();
    expect(ascii).toContain("*");
  });

  it("should produce correct table header", () => {
    const t = makeTurnstile();
    const table = t.toTable();
    expect(table[0][0]).toBe("State");
    expect(table[0]).toContain("coin");
    expect(table[0]).toContain("push");
  });

  it("should produce correct table data", () => {
    const t = makeTurnstile();
    const table = t.toTable();
    const lockedRow = table.find((row) => row[0] === "locked")!;
    const events = table[0].slice(1);
    const coinIdx = events.indexOf("coin") + 1;
    const pushIdx = events.indexOf("push") + 1;
    expect(lockedRow[coinIdx]).toBe("unlocked");
    expect(lockedRow[pushIdx]).toBe("locked");
  });

  it("should show missing transitions as dash in table", () => {
    const dfa = new DFA(
      new Set(["q0", "q1"]),
      new Set(["a", "b"]),
      new Map([[transitionKey("q0", "a"), "q1"]]),
      "q0",
      new Set(),
    );
    const table = dfa.toTable();
    const q0Row = table.find((row) => row[0] === "q0")!;
    expect(q0Row).toContain("\u2014");
  });
});

// ============================================================
// Edge Cases
// ============================================================

describe("DFA Edge Cases", () => {
  it("should handle single state self-loop", () => {
    const dfa = new DFA(
      new Set(["q0"]),
      new Set(["a"]),
      new Map([[transitionKey("q0", "a"), "q0"]]),
      "q0",
      new Set(["q0"]),
    );
    expect(dfa.accepts(["a", "a", "a"])).toBe(true);
    expect(dfa.accepts([])).toBe(true);
  });

  it("should handle large alphabet", () => {
    const alphabet = new Set<string>();
    const transitions = new Map<string, string>();
    for (let i = 97; i <= 122; i++) {
      const c = String.fromCharCode(i);
      alphabet.add(c);
      transitions.set(transitionKey("q0", c), "q1");
      transitions.set(transitionKey("q1", c), "q0");
    }
    const dfa = new DFA(
      new Set(["q0", "q1"]),
      alphabet,
      transitions,
      "q0",
      new Set(["q1"]),
    );
    expect(dfa.accepts(["a"])).toBe(true);
    expect(dfa.accepts(["a", "b"])).toBe(false);
    expect(dfa.accepts(["x", "y", "z"])).toBe(true);
  });

  it("should return copy of trace", () => {
    const t = makeTurnstile();
    t.process("coin");
    const t1 = t.trace;
    const t2 = t.trace;
    expect(t1).toEqual(t2);
    expect(t1).not.toBe(t2);
  });

  it("should verify div-by-3 for all numbers 0-31", () => {
    const d = makeDivBy3();
    for (let n = 0; n < 32; n++) {
      const binary = n.toString(2);
      const bits = binary.split("");
      const expected = n % 3 === 0;
      if (n === 0) {
        expect(d.accepts([])).toBe(true);
      } else {
        expect(d.accepts(bits)).toBe(expected);
      }
    }
  });
});
