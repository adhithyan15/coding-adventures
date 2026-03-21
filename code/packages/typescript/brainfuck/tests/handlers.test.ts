/**
 * Tests for individual Brainfuck opcode handlers.
 *
 * Each handler is tested in isolation by constructing small CodeObjects
 * with specific instruction sequences, then verifying the resulting VM
 * state (tape contents, data pointer position, output, etc.).
 *
 * We use the ``execCode`` helper to create a BF VM, execute instructions,
 * and return the VM for inspection. This keeps each test focused on the
 * behavior of specific opcodes without needing to go through the translator.
 */

import { describe, it, expect } from "vitest";

import type { Instruction, CodeObject } from "@coding-adventures/virtual-machine";
import type { GenericVM } from "@coding-adventures/virtual-machine";

import { HANDLERS, TAPE_SIZE, BrainfuckError } from "../src/handlers.js";
import { Op } from "../src/opcodes.js";
import { createBrainfuckVm } from "../src/vm.js";

// =========================================================================
// Test Helper
// =========================================================================

/**
 * Helper: create a BF VM, execute instructions, return the VM.
 *
 * Automatically appends a HALT instruction so the VM stops cleanly.
 * The returned VM can be inspected for tape contents, data pointer
 * position, output, etc.
 */
function execCode(
  instructions: Instruction[],
  inputData: string = "",
): GenericVM {
  const code: CodeObject = {
    instructions: [...instructions, { opcode: Op.HALT }],
    constants: [],
    names: [],
  };
  const vm = createBrainfuckVm(inputData);
  vm.execute(code);
  return vm;
}

// =========================================================================
// Handler Registry
// =========================================================================

describe("HandlerRegistry", () => {
  /** All 9 handlers are registered. */

  it("has all 9 opcodes registered", () => {
    expect(HANDLERS.size).toBe(9);
  });

  it("covers all expected opcodes", () => {
    const expected = new Set([
      Op.RIGHT, Op.LEFT, Op.INC, Op.DEC,
      Op.OUTPUT, Op.INPUT, Op.LOOP_START, Op.LOOP_END, Op.HALT,
    ]);
    const actual = new Set(HANDLERS.keys());
    expect(actual).toEqual(expected);
  });

  it("registers all handlers on the VM", () => {
    const vm = createBrainfuckVm();
    for (const opcode of HANDLERS.keys()) {
      // The GenericVM stores handlers in a private Map, but we can verify
      // by trying to step through an instruction with each opcode.
      // Instead, we check via the _handlers property on GenericVM.
      // The GenericVM uses a private Map, so we access it via (vm as any).
      expect((vm as any).handlers.has(opcode)).toBe(true);
    }
  });
});

// =========================================================================
// Pointer Movement
// =========================================================================

describe("PointerMovement", () => {
  /** > and < handlers. */

  it("moves pointer right", () => {
    const vm = execCode([{ opcode: Op.RIGHT }]);
    expect((vm as any).dp).toBe(1);
  });

  it("moves pointer left after moving right", () => {
    const vm = execCode([
      { opcode: Op.RIGHT },
      { opcode: Op.LEFT },
    ]);
    expect((vm as any).dp).toBe(0);
  });

  it("moves pointer right multiple times", () => {
    const instrs: Instruction[] = Array.from({ length: 10 }, () => ({
      opcode: Op.RIGHT,
    }));
    const vm = execCode(instrs);
    expect((vm as any).dp).toBe(10);
  });

  it("throws BrainfuckError when moving left at position 0", () => {
    expect(() => execCode([{ opcode: Op.LEFT }])).toThrow(BrainfuckError);
    expect(() => execCode([{ opcode: Op.LEFT }])).toThrow(/before start/);
  });

  it("throws BrainfuckError when moving right past tape end", () => {
    const instrs: Instruction[] = Array.from({ length: TAPE_SIZE }, () => ({
      opcode: Op.RIGHT,
    }));
    expect(() => execCode(instrs)).toThrow(BrainfuckError);
    expect(() => execCode(instrs)).toThrow(/past end/);
  });
});

// =========================================================================
// Cell Modification
// =========================================================================

describe("CellModification", () => {
  /** + and - handlers. */

  it("increments cell", () => {
    const vm = execCode([{ opcode: Op.INC }]);
    expect((vm as any).tape[0]).toBe(1);
  });

  it("increments cell multiple times", () => {
    const instrs: Instruction[] = Array.from({ length: 5 }, () => ({
      opcode: Op.INC,
    }));
    const vm = execCode(instrs);
    expect((vm as any).tape[0]).toBe(5);
  });

  it("decrements cell after incrementing", () => {
    const vm = execCode([
      { opcode: Op.INC },
      { opcode: Op.INC },
      { opcode: Op.DEC },
    ]);
    expect((vm as any).tape[0]).toBe(1);
  });

  it("wraps increment at 255 -> 0", () => {
    /** 255 + 1 = 0 (byte wrapping). */
    const instrs: Instruction[] = Array.from({ length: 256 }, () => ({
      opcode: Op.INC,
    }));
    const vm = execCode(instrs);
    expect((vm as any).tape[0]).toBe(0);
  });

  it("wraps decrement at 0 -> 255", () => {
    /** 0 - 1 = 255 (byte wrapping). */
    const vm = execCode([{ opcode: Op.DEC }]);
    expect((vm as any).tape[0]).toBe(255);
  });

  it("modifies cells at different positions", () => {
    /** Increment a cell that isn't cell 0. */
    const vm = execCode([
      { opcode: Op.RIGHT },
      { opcode: Op.INC },
      { opcode: Op.INC },
    ]);
    expect((vm as any).tape[0]).toBe(0);
    expect((vm as any).tape[1]).toBe(2);
  });
});

// =========================================================================
// Output
// =========================================================================

describe("Output", () => {
  /** . handler. */

  it("outputs cell value as ASCII character", () => {
    /** Set cell to 65 ('A'). */
    const instrs: Instruction[] = [
      ...Array.from({ length: 65 }, (): Instruction => ({ opcode: Op.INC })),
      { opcode: Op.OUTPUT },
    ];
    const vm = execCode(instrs);
    expect(vm.output).toEqual(["A"]);
  });

  it("outputs cell 0 as null character", () => {
    const vm = execCode([{ opcode: Op.OUTPUT }]);
    expect(vm.output).toEqual(["\x00"]);
  });

  it("outputs multiple characters", () => {
    const vm = execCode([
      { opcode: Op.INC },   // cell = 1
      { opcode: Op.OUTPUT },
      { opcode: Op.INC },   // cell = 2
      { opcode: Op.OUTPUT },
    ]);
    expect(vm.output.length).toBe(2);
  });
});

// =========================================================================
// Input
// =========================================================================

describe("Input", () => {
  /** , handler. */

  it("reads one byte of input", () => {
    const vm = execCode(
      [{ opcode: Op.INPUT }],
      "A",
    );
    expect((vm as any).tape[0]).toBe(65); // ord('A')
  });

  it("reads multiple bytes of input", () => {
    const vm = execCode(
      [
        { opcode: Op.INPUT },
        { opcode: Op.RIGHT },
        { opcode: Op.INPUT },
      ],
      "AB",
    );
    expect((vm as any).tape[0]).toBe(65);
    expect((vm as any).tape[1]).toBe(66);
  });

  it("returns 0 for EOF (empty input)", () => {
    /** Reading past end of input gives 0. */
    const vm = execCode(
      [{ opcode: Op.INPUT }],
      "",
    );
    expect((vm as any).tape[0]).toBe(0);
  });

  it("returns 0 for EOF after consuming all input", () => {
    const vm = execCode(
      [
        { opcode: Op.INPUT },
        { opcode: Op.RIGHT },
        { opcode: Op.INPUT },
      ],
      "X",
    );
    expect((vm as any).tape[0]).toBe("X".charCodeAt(0));
    expect((vm as any).tape[1]).toBe(0); // EOF
  });
});

// =========================================================================
// Control Flow
// =========================================================================

describe("ControlFlow", () => {
  /** [ and ] handlers. */

  it("skips loop when cell is zero", () => {
    /** [..] is skipped entirely if cell is 0. */
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOOP_START, operand: 3 }, // skip to index 3
        { opcode: Op.INC },                     // should be skipped
        { opcode: Op.LOOP_END, operand: 0 },   // should be skipped
        { opcode: Op.HALT },
      ],
      constants: [],
      names: [],
    };
    const vm = createBrainfuckVm();
    vm.execute(code);
    expect((vm as any).tape[0]).toBe(0); // INC was skipped
  });

  it("enters loop when cell is nonzero", () => {
    /** Loop body executes when cell != 0. */
    const code: CodeObject = {
      instructions: [
        { opcode: Op.INC },                     // cell = 1
        { opcode: Op.LOOP_START, operand: 5 },  // cell != 0, enter loop
        { opcode: Op.DEC },                     // cell = 0
        { opcode: Op.RIGHT },                   // dp = 1
        { opcode: Op.LOOP_END, operand: 1 },   // cell[1] == 0, exit
        { opcode: Op.HALT },
      ],
      constants: [],
      names: [],
    };
    const vm = createBrainfuckVm();
    vm.execute(code);
    expect((vm as any).tape[0]).toBe(0);
    expect((vm as any).dp).toBe(1);
  });

  it("repeats loop until cell becomes zero", () => {
    /** Set cell to 3, then loop: [>+<-] (move value to cell 1). */
    const code: CodeObject = {
      instructions: [
        { opcode: Op.INC },                     // cell[0] = 1
        { opcode: Op.INC },                     // cell[0] = 2
        { opcode: Op.INC },                     // cell[0] = 3
        { opcode: Op.LOOP_START, operand: 8 },  // [
        { opcode: Op.RIGHT },                   // dp = 1
        { opcode: Op.INC },                     // cell[1]++
        { opcode: Op.LEFT },                    // dp = 0
        { opcode: Op.DEC },                     // cell[0]--
        { opcode: Op.LOOP_END, operand: 3 },   // ]
        { opcode: Op.HALT },
      ],
      constants: [],
      names: [],
    };
    const vm = createBrainfuckVm();
    vm.execute(code);
    expect((vm as any).tape[0]).toBe(0);
    expect((vm as any).tape[1]).toBe(3);
  });
});

// =========================================================================
// VM State
// =========================================================================

describe("VMState", () => {
  /** GenericVM state initialization. */

  it("initializes tape with correct size", () => {
    const vm = createBrainfuckVm();
    expect((vm as any).tape.length).toBe(TAPE_SIZE);
  });

  it("initializes all tape cells to zero", () => {
    const vm = createBrainfuckVm();
    const tape = (vm as any).tape as number[];
    expect(tape.every((c: number) => c === 0)).toBe(true);
  });

  it("initializes data pointer at zero", () => {
    const vm = createBrainfuckVm();
    expect((vm as any).dp).toBe(0);
  });

  it("sets input buffer from argument", () => {
    const vm = createBrainfuckVm("hello");
    expect((vm as any).inputBuffer).toBe("hello");
    expect((vm as any).inputPos).toBe(0);
  });
});
