/**
 * End-to-end tests -- real Brainfuck programs translated and executed.
 *
 * These tests exercise the full pipeline: source code -> translator ->
 * GenericVM -> BrainfuckResult. They use ``executeBrainfuck()`` which
 * wraps the entire process in one call.
 *
 * The test cases range from trivial (empty program, single increment)
 * to the canonical Hello World program. If Hello World works, the
 * entire system is functioning correctly.
 */

import { describe, it, expect } from "vitest";

import { executeBrainfuck } from "../src/vm.js";

// =========================================================================
// Simple Programs
// =========================================================================

describe("SimplePrograms", () => {
  /** Small programs that test fundamental behavior. */

  it("runs empty program", () => {
    const result = executeBrainfuck("");
    expect(result.output).toBe("");
    expect(result.tape[0]).toBe(0);
  });

  it("runs single increment", () => {
    const result = executeBrainfuck("+");
    expect(result.tape[0]).toBe(1);
  });

  it("computes 2 + 5 = 7", () => {
    /**
     * Classic BF addition pattern.
     *
     * Put 2 in cell 0, 5 in cell 1.
     * Loop: decrement cell 1, increment cell 0.
     * Result: 7 in cell 0, 0 in cell 1.
     */
    const result = executeBrainfuck("++>+++++[<+>-]");
    expect(result.tape[0]).toBe(7);
    expect(result.tape[1]).toBe(0);
  });

  it("moves value from cell 0 to cell 1", () => {
    /**
     * Set cell 0 to 5, then [>+<-] moves it to cell 1.
     */
    const result = executeBrainfuck("+++++[>+<-]");
    expect(result.tape[0]).toBe(0);
    expect(result.tape[1]).toBe(5);
  });

  it("wraps overflow: 256 increments = 0", () => {
    /** 255 + 1 = 0. */
    const source = "+".repeat(256);
    const result = executeBrainfuck(source);
    expect(result.tape[0]).toBe(0);
  });

  it("wraps underflow: decrement from 0 = 255", () => {
    /** 0 - 1 = 255. */
    const result = executeBrainfuck("-");
    expect(result.tape[0]).toBe(255);
  });

  it("skips empty loop when cell is 0", () => {
    /** [] is skipped when cell is 0 (which it starts as). */
    const result = executeBrainfuck("[]+++");
    expect(result.tape[0]).toBe(3);
  });
});

// =========================================================================
// Output
// =========================================================================

describe("Output", () => {
  /** Programs that produce output. */

  it("outputs 'H' (ASCII 72)", () => {
    /**
     * 9 * 8 = 72 -> +++++++++[>++++++++<-]>.
     */
    const result = executeBrainfuck("+++++++++[>++++++++<-]>.");
    expect(result.output).toBe("H");
  });

  it("outputs 'AB' (two characters)", () => {
    /**
     * Cell 0 = 65 ('A'), output, inc, output ('B').
     */
    const source = "+".repeat(65) + ".+.";
    const result = executeBrainfuck(source);
    expect(result.output).toBe("AB");
  });

  it("outputs Hello World!", () => {
    /**
     * The classic Brainfuck Hello World program.
     *
     * This is the canonical test -- if this works, everything works.
     * Source: https://esolangs.org/wiki/Brainfuck
     */
    const helloWorld =
      "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]" +
      ">>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.";
    const result = executeBrainfuck(helloWorld);
    expect(result.output).toBe("Hello World!\n");
  });
});

// =========================================================================
// Input
// =========================================================================

describe("Input", () => {
  /** Programs that read input. */

  it("echoes a single character", () => {
    /** , reads one byte, . outputs it. */
    const result = executeBrainfuck(",.", "X");
    expect(result.output).toBe("X");
  });

  it("runs cat program (echo input until EOF)", () => {
    /**
     * ,[.,] -- echo input until EOF.
     *
     * Reads a character. If nonzero, output it and read the next.
     * On EOF (0), the loop exits.
     */
    const result = executeBrainfuck(",[.,]", "Hi");
    expect(result.output).toBe("Hi");
  });

  it("stores input byte value in cell", () => {
    /** Verify the cell holds the input byte value. */
    const result = executeBrainfuck(",", "A");
    expect(result.tape[0]).toBe(65); // ord('A')
  });

  it("reads 0 for EOF (no input)", () => {
    /** Reading with no input gives 0. */
    const result = executeBrainfuck(",");
    expect(result.tape[0]).toBe(0);
  });
});

// =========================================================================
// Nested Loops
// =========================================================================

describe("NestedLoops", () => {
  /** Programs with nested loop structures. */

  it("computes 2 * 3 = 6 using nested loops", () => {
    /**
     * Cell 0 = 2, Cell 1 = 3.
     * Outer loop (cell 0): for each unit, add cell 1 to cell 2.
     * Result: cell 2 = 6.
     *
     *     ++           cell[0] = 2
     *     >+++         cell[1] = 3
     *     <            back to cell[0]
     *     [            while cell[0] != 0:
     *       >          move to cell[1]
     *       [>+>+<<-]  copy cell[1] to cell[2] and cell[3]
     *       >>         move to cell[3]
     *       [<<+>>-]   move cell[3] back to cell[1] (restore)
     *       <<<        back to cell[0]
     *       -          dec cell[0]
     *     ]
     */
    const source = "++>+++<[>[>+>+<<-]>>[<<+>>-]<<<-]";
    const result = executeBrainfuck(source);
    expect(result.tape[2]).toBe(6);
  });

  it("handles deeply nested loops", () => {
    /** ++[>++[>+<-]<-] -- nested decrement loops. */
    const result = executeBrainfuck("++[>++[>+<-]<-]");
    // Outer loop runs 2 times.
    // Each time: cell[1] = 2, inner loop moves cell[1] to cell[2].
    // After 2 outer iterations: cell[2] = 2 + 2 = 4.
    expect(result.tape[2]).toBe(4);
    expect(result.tape[1]).toBe(0);
    expect(result.tape[0]).toBe(0);
  });
});

// =========================================================================
// BrainfuckResult Fields
// =========================================================================

describe("BrainfuckResult", () => {
  /** Verify the BrainfuckResult structure. */

  it("has correct field types", () => {
    const result = executeBrainfuck("+++.");
    expect(typeof result.output).toBe("string");
    expect(Array.isArray(result.tape)).toBe(true);
    expect(typeof result.dp).toBe("number");
    expect(Array.isArray(result.traces)).toBe(true);
    expect(typeof result.steps).toBe("number");
  });

  it("counts steps correctly", () => {
    const result = executeBrainfuck("+++");
    // 3 INCs + 1 HALT = 4 steps
    expect(result.steps).toBe(4);
  });

  it("reports final data pointer position", () => {
    const result = executeBrainfuck(">>>");
    expect(result.dp).toBe(3);
  });

  it("populates traces", () => {
    const result = executeBrainfuck("+");
    expect(result.traces.length).toBe(2); // INC + HALT
  });
});

// =========================================================================
// Comments
// =========================================================================

describe("Comments", () => {
  /** Non-BF characters are comments and should not affect execution. */

  it("ignores arbitrary text around BF commands", () => {
    const result = executeBrainfuck("This is + a + program + .");
    expect(result.tape[0]).toBe(3);
  });

  it("ignores numbers in source", () => {
    const result = executeBrainfuck("123+456");
    expect(result.tape[0]).toBe(1);
  });

  it("ignores newlines in source", () => {
    const result = executeBrainfuck("+\n+\n+");
    expect(result.tape[0]).toBe(3);
  });
});
