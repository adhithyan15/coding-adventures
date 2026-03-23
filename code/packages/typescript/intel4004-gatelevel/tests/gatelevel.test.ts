/**
 * Tests for the Intel 4004 gate-level simulator.
 *
 * These tests verify that every instruction works correctly when routed
 * through real logic gates. The test structure mirrors the behavioral
 * simulator's tests -- same programs, same expected results.
 */

import { describe, it, expect } from "vitest";
import {
  Intel4004GateLevel,
  GateALU,
  RegisterFile,
  ProgramCounter,
  HardwareStack,
  decode,
  intToBits,
  bitsToInt,
} from "../src/index.js";

// ===================================================================
// Basic instructions
// ===================================================================

describe("NOP", () => {
  it("does nothing", () => {
    const cpu = new Intel4004GateLevel();
    const traces = cpu.run(new Uint8Array([0x00, 0x01]));
    expect(cpu.accumulator).toBe(0);
    expect(traces[0].mnemonic).toBe("NOP");
  });

  it("handles multiple NOPs", () => {
    const cpu = new Intel4004GateLevel();
    const traces = cpu.run(new Uint8Array([0x00, 0x00, 0x00, 0x01]));
    expect(traces.length).toBe(4);
  });
});

describe("HLT", () => {
  it("stops execution", () => {
    const cpu = new Intel4004GateLevel();
    const traces = cpu.run(new Uint8Array([0x01]));
    expect(cpu.halted).toBe(true);
    expect(traces.length).toBe(1);
  });
});

describe("LDM", () => {
  it("loads all immediate values", () => {
    for (let n = 0; n < 16; n++) {
      const cpu = new Intel4004GateLevel();
      cpu.run(new Uint8Array([0xd0 | n, 0x01]));
      expect(cpu.accumulator).toBe(n);
    }
  });
});

describe("LD", () => {
  it("reads register into accumulator", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xd7, 0xb0, 0xa0, 0x01])); // LDM 7, XCH R0, LD R0
    expect(cpu.accumulator).toBe(7);
  });
});

describe("XCH", () => {
  it("swaps accumulator and register", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xd7, 0xb0, 0x01]));
    expect(cpu.registers[0]).toBe(7);
    expect(cpu.accumulator).toBe(0);
  });
});

describe("INC", () => {
  it("wraps at 15", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xdf, 0xb0, 0x60, 0x01])); // LDM 15, XCH R0, INC R0
    expect(cpu.registers[0]).toBe(0);
  });

  it("does not affect carry", () => {
    const cpu = new Intel4004GateLevel();
    // Set carry, then INC -- carry should stay
    cpu.run(new Uint8Array([0xdf, 0xb1, 0xdf, 0x81, 0x60, 0x01]));
    expect(cpu.carry).toBe(true);
  });
});

// ===================================================================
// Arithmetic
// ===================================================================

describe("ADD", () => {
  it("adds basic values", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xd3, 0xb0, 0xd2, 0x80, 0x01]));
    expect(cpu.accumulator).toBe(5);
    expect(cpu.carry).toBe(false);
  });

  it("sets carry on overflow", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xd1, 0xb0, 0xdf, 0x80, 0x01]));
    expect(cpu.accumulator).toBe(0);
    expect(cpu.carry).toBe(true);
  });

  it("includes carry in", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([
      0xdf, 0xb0, 0xdf, 0x80, // 15+15 -> carry=1
      0xd1, 0xb1, 0xd1, 0x81, // 1+1+carry = 3
      0x01,
    ]));
    expect(cpu.accumulator).toBe(3);
  });
});

describe("SUB", () => {
  it("subtracts basic values", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xd3, 0xb0, 0xd5, 0x90, 0x01]));
    expect(cpu.accumulator).toBe(2);
    expect(cpu.carry).toBe(true);
  });

  it("underflows correctly", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xd1, 0xb0, 0xd0, 0x90, 0x01]));
    expect(cpu.accumulator).toBe(15);
    expect(cpu.carry).toBe(false);
  });
});

// ===================================================================
// Accumulator operations
// ===================================================================

describe("Accumulator operations", () => {
  it("CLB clears accumulator and carry", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xdf, 0xb0, 0xdf, 0x80, 0xf0, 0x01]));
    expect(cpu.accumulator).toBe(0);
    expect(cpu.carry).toBe(false);
  });

  it("CLC clears carry only", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xdf, 0xb0, 0xdf, 0x80, 0xf1, 0x01]));
    expect(cpu.carry).toBe(false);
  });

  it("IAC increments accumulator", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xd5, 0xf2, 0x01]));
    expect(cpu.accumulator).toBe(6);
  });

  it("IAC sets carry on overflow", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xdf, 0xf2, 0x01]));
    expect(cpu.accumulator).toBe(0);
    expect(cpu.carry).toBe(true);
  });

  it("CMC complements carry", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xf3, 0x01]));
    expect(cpu.carry).toBe(true);
  });

  it("CMA complements accumulator", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xd5, 0xf4, 0x01]));
    expect(cpu.accumulator).toBe(10);
  });

  it("RAL rotates left through carry", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xd5, 0xf5, 0x01])); // 0101 -> 1010
    expect(cpu.accumulator).toBe(0b1010);
  });

  it("RAR rotates right through carry", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xd4, 0xf6, 0x01])); // 0100 -> 0010
    expect(cpu.accumulator).toBe(2);
  });

  it("TCC transfers carry to accumulator", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xfa, 0xf7, 0x01]));
    expect(cpu.accumulator).toBe(1);
    expect(cpu.carry).toBe(false);
  });

  it("DAC decrements accumulator", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xd5, 0xf8, 0x01]));
    expect(cpu.accumulator).toBe(4);
    expect(cpu.carry).toBe(true);
  });

  it("DAC wraps at zero", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xd0, 0xf8, 0x01]));
    expect(cpu.accumulator).toBe(15);
    expect(cpu.carry).toBe(false);
  });

  it("TCS transfers carry to accumulator as 9 or 10", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xfa, 0xf9, 0x01]));
    expect(cpu.accumulator).toBe(10);
  });

  it("STC sets carry", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xfa, 0x01]));
    expect(cpu.carry).toBe(true);
  });

  it("DAA decimal adjusts after add", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xdc, 0xfb, 0x01]));
    expect(cpu.accumulator).toBe(2);
    expect(cpu.carry).toBe(true);
  });

  it("KBP converts keyboard positions", () => {
    const expected: Record<number, number> = { 0: 0, 1: 1, 2: 2, 4: 3, 8: 4, 3: 15, 15: 15 };
    for (const [inp, out] of Object.entries(expected)) {
      const cpu = new Intel4004GateLevel();
      cpu.run(new Uint8Array([0xd0 | Number(inp), 0xfc, 0x01]));
      expect(cpu.accumulator).toBe(out);
    }
  });

  it("DCL sets RAM bank", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xd2, 0xfd, 0x01]));
    expect(cpu.ramBank).toBe(2);
  });
});

// ===================================================================
// Jump instructions
// ===================================================================

describe("Jumps", () => {
  it("JUN jumps unconditionally", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0x40, 0x04, 0xd5, 0x01, 0x01]));
    expect(cpu.accumulator).toBe(0); // LDM 5 skipped
  });

  it("JCN jumps when A==0", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0x14, 0x04, 0xd5, 0x01, 0x01]));
    expect(cpu.accumulator).toBe(0); // A==0 -> jump
  });

  it("JCN does not jump when A!=0", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xd3, 0x14, 0x06, 0xd5, 0x01, 0x01, 0x01]));
    expect(cpu.accumulator).toBe(5);
  });

  it("JCN with invert flag", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xd3, 0x1c, 0x06, 0xd5, 0x01, 0x01, 0x01]));
    expect(cpu.accumulator).toBe(3); // A!=0 -> jump (invert zero test)
  });

  it("ISZ loops until zero", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xde, 0xb0, 0x70, 0x02, 0x01]));
    expect(cpu.registers[0]).toBe(0);
  });
});

// ===================================================================
// Subroutines
// ===================================================================

describe("Subroutines", () => {
  it("JMS/BBL call and return", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([
      0x50, 0x04, // JMS 0x004
      0x01,       // HLT (returned here)
      0x00,       // padding
      0xc5,       // BBL 5
    ]));
    expect(cpu.accumulator).toBe(5);
  });

  it("handles nested calls", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([
      0x50, 0x06, // JMS sub1
      0xb0, 0x01, // XCH R0, HLT
      0x00, 0x00, // padding
      0x50, 0x0c, // sub1: JMS sub2
      0xb1,       // XCH R1
      0xd9, 0xc0, // LDM 9, BBL 0
      0x00,       // padding
      0xc3,       // sub2: BBL 3
    ]));
    expect(cpu.registers[1]).toBe(3);
  });
});

// ===================================================================
// Register pairs
// ===================================================================

describe("Register pairs", () => {
  it("FIM loads pair", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0x20, 0xab, 0x01]));
    expect(cpu.registers[0]).toBe(0xa);
    expect(cpu.registers[1]).toBe(0xb);
  });

  it("SRC/WRM/RDM round-trip", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([
      0x20, 0x00, 0x21, 0xd7, 0xe0, // SRC P0, LDM 7, WRM
      0xd0,                           // LDM 0
      0x20, 0x00, 0x21, 0xe9,         // SRC P0, RDM
      0x01,
    ]));
    expect(cpu.accumulator).toBe(7);
  });

  it("JIN jumps indirect", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0x22, 0x06, 0x33, 0xd5, 0x01, 0x00, 0x01]));
    expect(cpu.accumulator).toBe(0); // LDM 5 skipped
  });
});

// ===================================================================
// RAM I/O
// ===================================================================

describe("RAM I/O", () => {
  it("writes and reads status", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([
      0x20, 0x00, 0x21, // SRC P0
      0xd3, 0xe4,       // LDM 3, WR0
      0xd0,             // LDM 0
      0x20, 0x00, 0x21, // SRC P0
      0xec,             // RD0
      0x01,
    ]));
    expect(cpu.accumulator).toBe(3);
  });

  it("WRR/RDR round-trip", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xdb, 0xe2, 0xd0, 0xea, 0x01]));
    expect(cpu.accumulator).toBe(11);
  });

  it("RAM banking isolates data", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([
      0xd0, 0xfd,       // DCL bank 0
      0x20, 0x00, 0x21, // SRC P0
      0xd5, 0xe0,       // LDM 5, WRM
      0xd1, 0xfd,       // DCL bank 1
      0x20, 0x00, 0x21,
      0xd9, 0xe0,       // LDM 9, WRM
      0xd0, 0xfd,       // DCL bank 0
      0x20, 0x00, 0x21,
      0xe9,             // RDM
      0x01,
    ]));
    expect(cpu.accumulator).toBe(5);
  });
});

// ===================================================================
// End-to-end programs
// ===================================================================

describe("End-to-end programs", () => {
  it("computes x = 1 + 2", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xd1, 0xb0, 0xd2, 0x80, 0xb1, 0x01]));
    expect(cpu.registers[1]).toBe(3);
    expect(cpu.halted).toBe(true);
  });

  it("multiplies 3 x 4", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([
      0xd3, 0xb0, 0xdc, 0xb1,
      0xd0, 0x80, 0x71, 0x05,
      0xb2, 0x01,
    ]));
    expect(cpu.registers[2]).toBe(12);
  });

  it("BCD adds 7 + 8", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([
      0xd8, 0xb0, 0xd7, 0x80, 0xfb, 0x01,
    ]));
    expect(cpu.accumulator).toBe(5);
    expect(cpu.carry).toBe(true);
  });

  it("countdown to zero", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xd5, 0xf8, 0x1c, 0x01, 0x01]));
    expect(cpu.accumulator).toBe(0);
  });

  it("respects max steps", () => {
    const cpu = new Intel4004GateLevel();
    const traces = cpu.run(new Uint8Array([0x40, 0x00]), 10);
    expect(traces.length).toBe(10);
  });

  it("reports gate count", () => {
    const cpu = new Intel4004GateLevel();
    const count = cpu.gateCount();
    expect(count).toBeGreaterThan(500);
  });
});

// ===================================================================
// Component tests
// ===================================================================

describe("Components", () => {
  it("bits roundtrip 4-bit", () => {
    for (let val = 0; val < 16; val++) {
      expect(bitsToInt(intToBits(val, 4))).toBe(val);
    }
  });

  it("bits roundtrip 12-bit", () => {
    for (let val = 0; val < 4096; val++) {
      expect(bitsToInt(intToBits(val, 12))).toBe(val);
    }
  });

  it("ALU add", () => {
    const alu = new GateALU();
    const [result, carry] = alu.add(5, 3, 0);
    expect(result).toBe(8);
    expect(carry).toBe(false);
  });

  it("ALU subtract", () => {
    const alu = new GateALU();
    const [result, carry] = alu.subtract(5, 3, 1);
    expect(result).toBe(2);
    expect(carry).toBe(true); // no borrow
  });

  it("register file read/write", () => {
    const rf = new RegisterFile();
    rf.write(5, 11);
    expect(rf.read(5)).toBe(11);
    expect(rf.read(0)).toBe(0);
  });

  it("program counter increment", () => {
    const pc = new ProgramCounter();
    expect(pc.read()).toBe(0);
    pc.increment();
    expect(pc.read()).toBe(1);
    pc.increment();
    expect(pc.read()).toBe(2);
  });

  it("stack push/pop", () => {
    const stack = new HardwareStack();
    stack.push(0x100);
    stack.push(0x200);
    expect(stack.pop()).toBe(0x200);
    expect(stack.pop()).toBe(0x100);
  });

  it("decoder detects LDM", () => {
    const d = decode(0xd5);
    expect(d.isLdm).toBe(1);
    expect(d.immediate).toBe(5);
  });

  it("decoder detects ADD", () => {
    const d = decode(0x80);
    expect(d.isAdd).toBe(1);
    expect(d.regIndex).toBe(0);
  });
});

// ===================================================================
// Additional coverage: WMP, ADM, SBM, FIN
// ===================================================================

describe("Additional I/O", () => {
  it("WMP writes to output port", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([
      0x20, 0x00, 0x21, // FIM P0=0x00, SRC P0
      0xd7, 0xe1,       // LDM 7, WMP
      0x01,
    ]));
    expect(cpu.ramOutput[0]).toBe(7);
  });

  it("ADM adds RAM to accumulator", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([
      0x20, 0x00, 0x21, // FIM P0=0x00, SRC P0
      0xd5, 0xe0,       // LDM 5, WRM (write 5 to RAM)
      0xd3,             // LDM 3
      0x20, 0x00, 0x21, // FIM P0=0x00, SRC P0
      0xeb,             // ADM
      0x01,
    ]));
    expect(cpu.accumulator).toBe(8); // 3 + 5
  });

  it("SBM subtracts RAM from accumulator", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([
      0x20, 0x00, 0x21, // FIM P0=0x00, SRC P0
      0xd3, 0xe0,       // LDM 3, WRM (write 3 to RAM)
      0xd7,             // LDM 7
      0x20, 0x00, 0x21, // FIM P0=0x00, SRC P0
      0xe8,             // SBM
      0x01,
    ]));
    expect(cpu.accumulator).toBe(4); // 7 - 3
    expect(cpu.carry).toBe(true);    // no borrow
  });

  it("WPM is a no-op", () => {
    const cpu = new Intel4004GateLevel();
    const traces = cpu.run(new Uint8Array([0xe3, 0x01]));
    expect(traces[0].mnemonic).toBe("WPM");
  });

  it("WR1-WR3 write status chars", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([
      0x20, 0x00, 0x21, // FIM P0=0x00, SRC P0
      0xd9, 0xe5,       // LDM 9, WR1
      0xd0,             // LDM 0
      0x20, 0x00, 0x21, // SRC P0
      0xed,             // RD1
      0x01,
    ]));
    expect(cpu.accumulator).toBe(9);
  });

  it("RD2/RD3 read status chars", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([
      0x20, 0x00, 0x21, // FIM P0=0x00, SRC P0
      0xd7, 0xe6,       // LDM 7, WR2
      0xd0,             // LDM 0
      0x20, 0x00, 0x21, // SRC P0
      0xee,             // RD2
      0x01,
    ]));
    expect(cpu.accumulator).toBe(7);
  });
});

describe("JCN carry test", () => {
  it("jumps when carry is set", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([
      0xfa,             // STC (set carry)
      0x12, 0x06,       // JCN 2 (test carry), jump to 0x06
      0xd5,             // LDM 5 (should be skipped)
      0x01,
      0x00,
      0x01,             // HLT at 0x06
    ]));
    expect(cpu.accumulator).toBe(0); // LDM 5 was skipped
  });
});

describe("Step on halted CPU", () => {
  it("throws error", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0x01]));
    expect(() => cpu.step()).toThrow("CPU is halted");
  });
});

describe("Reset", () => {
  it("clears all state", () => {
    const cpu = new Intel4004GateLevel();
    cpu.run(new Uint8Array([0xdf, 0xfa, 0x01])); // LDM 15, STC, HLT
    expect(cpu.accumulator).toBe(15);
    expect(cpu.carry).toBe(true);
    cpu.reset();
    expect(cpu.accumulator).toBe(0);
    expect(cpu.carry).toBe(false);
    expect(cpu.pc).toBe(0);
    expect(cpu.halted).toBe(false);
  });
});
