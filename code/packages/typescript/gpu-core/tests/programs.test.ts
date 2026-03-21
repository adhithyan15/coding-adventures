/**
 * Integration tests -- multi-instruction GPU programs.
 *
 * These tests verify that the GPU core correctly executes complete programs,
 * not just individual instructions. They serve as both tests and examples
 * of what GPU programs look like at the core level.
 */

import { describe, it, expect } from "vitest";
import { FP16, BF16 } from "@coding-adventures/fp-arithmetic";
import { GPUCore } from "../src/core.js";
import {
  beq,
  blt,
  bne,
  fadd,
  ffma,
  fmul,
  fsub,
  halt,
  jmp,
  limm,
  load,
  mov,
  nop,
  store,
} from "../src/opcodes.js";

describe("SAXPY", () => {
  /**
   * SAXPY: y = a * x + y -- the "hello world" of GPU programming.
   *
   * In real GPU code, SAXPY runs across thousands of threads, each computing
   * one element. Here we simulate what a single thread does: one FMA.
   */

  it("y = 2.0 * 3.0 + 1.0 = 7.0", () => {
    const core = new GPUCore();
    core.loadProgram([
      limm(0, 2.0),      // R0 = a = 2.0
      limm(1, 3.0),      // R1 = x = 3.0
      limm(2, 1.0),      // R2 = y = 1.0
      ffma(3, 0, 1, 2),  // R3 = a * x + y = 7.0
      halt(),
    ]);
    const traces = core.run();
    expect(core.registers.readFloat(3)).toBe(7.0);
    expect(traces.length).toBe(5);
  });

  it("zero alpha: y = 0 * x + y = y", () => {
    const core = new GPUCore();
    core.loadProgram([
      limm(0, 0.0),
      limm(1, 99.0),
      limm(2, 5.0),
      ffma(3, 0, 1, 2),
      halt(),
    ]);
    core.run();
    expect(core.registers.readFloat(3)).toBe(5.0);
  });
});

describe("DotProduct", () => {
  /**
   * Dot product: sum of element-wise products.
   *
   * dot(A, B) = A[0]*B[0] + A[1]*B[1] + A[2]*B[2]
   *
   * This is the fundamental operation in neural networks.
   */

  it("dot([1,2,3], [4,5,6]) = 32", () => {
    const core = new GPUCore();
    core.loadProgram([
      // Load vector A
      limm(0, 1.0),
      limm(1, 2.0),
      limm(2, 3.0),
      // Load vector B
      limm(3, 4.0),
      limm(4, 5.0),
      limm(5, 6.0),
      // Accumulate with FMA
      limm(6, 0.0),
      ffma(6, 0, 3, 6),   // acc = 1*4 + 0 = 4
      ffma(6, 1, 4, 6),   // acc = 2*5 + 4 = 14
      ffma(6, 2, 5, 6),   // acc = 3*6 + 14 = 32
      halt(),
    ]);
    core.run();
    expect(core.registers.readFloat(6)).toBe(32.0);
  });
});

describe("Loop", () => {
  it("sum of 1+2+3+4 = 10", () => {
    const core = new GPUCore();
    core.loadProgram([
      limm(0, 0.0),       // R0 = sum
      limm(1, 1.0),       // R1 = i
      limm(2, 1.0),       // R2 = 1 (increment)
      limm(3, 5.0),       // R3 = limit
      fadd(0, 0, 1),      // sum += i        (PC=4)
      fadd(1, 1, 2),      // i += 1          (PC=5)
      blt(1, 3, -2),      // if i < 5: back  (PC=6)
      halt(),             //                  (PC=7)
    ]);
    core.run();
    expect(core.registers.readFloat(0)).toBe(10.0);
  });

  it("countdown from 3 to 0", () => {
    const core = new GPUCore();
    core.loadProgram([
      limm(0, 3.0),       // counter
      limm(1, 1.0),       // decrement
      limm(2, 0.0),       // zero
      fsub(0, 0, 1),      // counter -= 1   (PC=3)
      bne(0, 2, -1),      // if counter != 0: back (PC=4)
      halt(),             //                (PC=5)
    ]);
    core.run();
    expect(core.registers.readFloat(0)).toBe(0.0);
  });
});

describe("MemoryPrograms", () => {
  it("store and load array", () => {
    const core = new GPUCore();
    // Pre-store some values in memory
    core.memory.storePythonFloat(0, 10.0);
    core.memory.storePythonFloat(4, 20.0);
    core.memory.storePythonFloat(8, 30.0);

    core.loadProgram([
      limm(10, 0.0),       // R10 = base address
      load(0, 10, 0.0),    // R0 = Mem[0] = 10.0
      load(1, 10, 4.0),    // R1 = Mem[4] = 20.0
      load(2, 10, 8.0),    // R2 = Mem[8] = 30.0
      fadd(3, 0, 1),       // R3 = 10 + 20 = 30
      fadd(3, 3, 2),       // R3 = 30 + 30 = 60
      store(10, 3, 12.0),  // Mem[12] = 60.0
      halt(),
    ]);
    core.run();
    expect(core.registers.readFloat(3)).toBe(60.0);
    expect(core.memory.loadFloatAsPython(12)).toBe(60.0);
  });
});

describe("Conditional", () => {
  it("max(3, 7) = 7", () => {
    const core = new GPUCore();
    core.loadProgram([
      limm(0, 3.0),
      limm(1, 7.0),
      blt(0, 1, 2),       // if a < b: skip to "result = b"
      mov(2, 0),          // result = a
      jmp(5),             // skip "result = b"
      mov(2, 1),          // result = b
      halt(),
    ]);
    core.run();
    expect(core.registers.readFloat(2)).toBe(7.0);
  });

  it("max(7, 3) = 7 (else branch)", () => {
    const core = new GPUCore();
    core.loadProgram([
      limm(0, 7.0),
      limm(1, 3.0),
      blt(0, 1, 2),       // 7 < 3? No
      mov(2, 0),          // result = a = 7
      jmp(6),
      mov(2, 1),          // skipped
      halt(),
    ]);
    core.run();
    expect(core.registers.readFloat(2)).toBe(7.0);
  });
});

describe("PrecisionModes", () => {
  it("FP16 execution", () => {
    const core = new GPUCore({ fmt: FP16 });
    core.loadProgram([
      limm(0, 1.0),
      limm(1, 2.0),
      fadd(2, 0, 1),
      halt(),
    ]);
    core.run();
    expect(core.registers.readFloat(2)).toBe(3.0);
  });

  it("BF16 execution", () => {
    const core = new GPUCore({ fmt: BF16 });
    core.loadProgram([
      limm(0, 4.0),
      limm(1, 5.0),
      fmul(2, 0, 1),
      halt(),
    ]);
    core.run();
    expect(core.registers.readFloat(2)).toBe(20.0);
  });
});

describe("EdgeCases", () => {
  it("NOP-only program", () => {
    const core = new GPUCore();
    core.loadProgram([nop(), nop(), nop(), halt()]);
    const traces = core.run();
    expect(traces.length).toBe(4);
    expect(core.halted).toBe(true);
  });

  it("self-modifying register (R0 = R0 + R0)", () => {
    const core = new GPUCore();
    core.loadProgram([
      limm(0, 5.0),
      fadd(0, 0, 0),
      halt(),
    ]);
    core.run();
    expect(core.registers.readFloat(0)).toBe(10.0);
  });

  it("large register index (NVIDIA-scale)", () => {
    const core = new GPUCore({ numRegisters: 256 });
    core.loadProgram([
      limm(200, 42.0),
      limm(255, 1.0),
      fadd(254, 200, 255),
      halt(),
    ]);
    core.run();
    expect(core.registers.readFloat(254)).toBe(43.0);
  });

  it("BEQ with offset 0 creates infinite loop", () => {
    const core = new GPUCore();
    core.loadProgram([
      limm(0, 1.0),
      limm(1, 1.0),
      beq(0, 1, 0),
      halt(),
    ]);
    expect(() => core.run(50)).toThrow("Execution limit");
  });
});
