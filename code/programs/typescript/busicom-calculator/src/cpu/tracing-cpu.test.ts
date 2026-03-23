/**
 * Tests for the TracingCPU wrapper.
 *
 * Verifies that the tracing CPU:
 *   - Produces DetailedTrace objects with decoded instructions
 *   - Captures CPU state snapshots
 *   - Reconstructs ALU detail for ADD/SUB/INC instructions
 *   - Detects memory access
 */

import { describe, it, expect } from "vitest";
import { TracingCPU } from "./tracing-cpu.js";
import { getBusicomROM } from "../rom/busicom-rom.js";

describe("TracingCPU", () => {
  // ==========================================================================
  // Basic functionality
  // ==========================================================================

  describe("basic functionality", () => {
    it("should step and return a DetailedTrace", () => {
      const cpu = new TracingCPU();
      // LDM 5 (load immediate 5 into accumulator)
      cpu.loadProgram(new Uint8Array([0xd5, 0x01]));
      const trace = cpu.step();

      expect(trace).toBeDefined();
      expect(trace.mnemonic).toContain("LDM");
      expect(trace.decoded).toBeDefined();
      expect(trace.decoded.isLdm).toBe(1);
      expect(trace.snapshot).toBeDefined();
      expect(trace.snapshot.accumulator).toBe(5);
    });

    it("should capture state snapshot after each step", () => {
      const cpu = new TracingCPU();
      // LDM 7, then LDM 3
      cpu.loadProgram(new Uint8Array([0xd7, 0xd3, 0x01]));

      const trace1 = cpu.step();
      expect(trace1.snapshot.accumulator).toBe(7);
      expect(trace1.snapshot.pc).toBe(1);

      const trace2 = cpu.step();
      expect(trace2.snapshot.accumulator).toBe(3);
      expect(trace2.snapshot.pc).toBe(2);
    });

    it("should maintain trace history", () => {
      const cpu = new TracingCPU();
      cpu.loadProgram(new Uint8Array([0xd1, 0xd2, 0xd3, 0x01]));

      cpu.step();
      cpu.step();
      cpu.step();

      expect(cpu.traceHistory).toHaveLength(3);
    });

    it("should cap trace history at maxHistory", () => {
      const cpu = new TracingCPU(5);
      // NOP × 10, then HLT
      const program = new Uint8Array(11);
      program[10] = 0x01; // HLT
      cpu.loadProgram(program);

      for (let i = 0; i < 10; i++) {
        cpu.step();
      }

      expect(cpu.traceHistory.length).toBeLessThanOrEqual(5);
    });

    it("should expose lastTrace", () => {
      const cpu = new TracingCPU();
      cpu.loadProgram(new Uint8Array([0xd5, 0x01]));

      expect(cpu.lastTrace).toBeUndefined();
      cpu.step();
      expect(cpu.lastTrace).toBeDefined();
      expect(cpu.lastTrace!.snapshot.accumulator).toBe(5);
    });

    it("should reset trace history on reset", () => {
      const cpu = new TracingCPU();
      cpu.loadProgram(new Uint8Array([0xd5, 0x01]));
      cpu.step();
      expect(cpu.traceHistory).toHaveLength(1);

      cpu.reset();
      expect(cpu.traceHistory).toHaveLength(0);
    });

    it("should reset trace history on loadProgram", () => {
      const cpu = new TracingCPU();
      cpu.loadProgram(new Uint8Array([0xd5, 0x01]));
      cpu.step();

      cpu.loadProgram(new Uint8Array([0xd3, 0x01]));
      expect(cpu.traceHistory).toHaveLength(0);
    });
  });

  // ==========================================================================
  // ALU detail reconstruction
  // ==========================================================================

  describe("ALU detail", () => {
    it("should capture ALU detail for ADD instruction", () => {
      const cpu = new TracingCPU();
      // LDM 3, XCH R0, LDM 5, ADD R0 → acc = 5 + 3 = 8
      cpu.loadProgram(
        new Uint8Array([0xd3, 0xb0, 0xd5, 0x80, 0x01]),
      );

      cpu.step(); // LDM 3
      cpu.step(); // XCH R0 (R0=3, acc=0)
      cpu.step(); // LDM 5

      const trace = cpu.step(); // ADD R0 (acc = 5 + 3 = 8)

      expect(trace.aluTrace).toBeDefined();
      expect(trace.aluTrace!.operation).toBe("add");
      expect(trace.aluTrace!.adders).toHaveLength(4);

      // Verify the result is correct (5 + 3 = 8)
      expect(trace.snapshot.accumulator).toBe(8);
    });

    it("should capture correct adder chain inputs for ADD", () => {
      const cpu = new TracingCPU();
      // LDM 1, XCH R0, LDM 1, ADD R0 → acc = 1 + 1 = 2
      cpu.loadProgram(
        new Uint8Array([0xd1, 0xb0, 0xd1, 0x80, 0x01]),
      );

      cpu.step(); // LDM 1
      cpu.step(); // XCH R0
      cpu.step(); // LDM 1

      const trace = cpu.step(); // ADD R0

      const alu = trace.aluTrace!;
      // 1 in binary LSB-first: [1, 0, 0, 0]
      expect(alu.inputA).toEqual([1, 0, 0, 0]);
      expect(alu.inputB).toEqual([1, 0, 0, 0]);
      expect(alu.carryIn).toBe(0);

      // Result: 2 in binary LSB-first: [0, 1, 0, 0]
      expect(alu.result).toEqual([0, 1, 0, 0]);
      expect(alu.carryOut).toBe(0);

      // Verify per-adder state
      // Adder 0: 1 + 1 + 0 = sum=0, carry=1
      expect(alu.adders[0].a).toBe(1);
      expect(alu.adders[0].b).toBe(1);
      expect(alu.adders[0].cIn).toBe(0);
      expect(alu.adders[0].sum).toBe(0);
      expect(alu.adders[0].cOut).toBe(1);

      // Adder 1: 0 + 0 + 1 = sum=1, carry=0
      expect(alu.adders[1].a).toBe(0);
      expect(alu.adders[1].b).toBe(0);
      expect(alu.adders[1].cIn).toBe(1);
      expect(alu.adders[1].sum).toBe(1);
      expect(alu.adders[1].cOut).toBe(0);
    });

    it("should capture ALU detail for SUB instruction", () => {
      const cpu = new TracingCPU();
      // LDM 2, XCH R0, LDM 5, SUB R0 → acc = 5 - 2 = 3
      cpu.loadProgram(
        new Uint8Array([0xd2, 0xb0, 0xd5, 0x90, 0x01]),
      );

      cpu.step(); // LDM 2
      cpu.step(); // XCH R0
      cpu.step(); // LDM 5

      const trace = cpu.step(); // SUB R0

      expect(trace.aluTrace).toBeDefined();
      expect(trace.aluTrace!.operation).toBe("sub");
      expect(trace.aluTrace!.adders).toHaveLength(4);
    });

    it("should capture ALU detail for INC instruction", () => {
      const cpu = new TracingCPU();
      // LDM 7, XCH R3, INC R3 → R3 = 8
      cpu.loadProgram(new Uint8Array([0xd7, 0xb3, 0x63, 0x01]));

      cpu.step(); // LDM 7
      cpu.step(); // XCH R3

      const trace = cpu.step(); // INC R3

      expect(trace.aluTrace).toBeDefined();
      expect(trace.aluTrace!.operation).toBe("inc");
      // 7 + 1 = 8
      expect(trace.aluTrace!.adders).toHaveLength(4);
    });

    it("should not capture ALU detail for non-ALU instructions", () => {
      const cpu = new TracingCPU();
      cpu.loadProgram(new Uint8Array([0xd5, 0x01])); // LDM 5
      const trace = cpu.step();
      expect(trace.aluTrace).toBeUndefined();
    });

    it("should capture complement operation for CMA", () => {
      const cpu = new TracingCPU();
      // LDM 5 (0101), CMA → acc = NOT(5) = 10 (1010)
      cpu.loadProgram(new Uint8Array([0xd5, 0xf4, 0x01]));

      cpu.step(); // LDM 5

      const trace = cpu.step(); // CMA

      expect(trace.aluTrace).toBeDefined();
      expect(trace.aluTrace!.operation).toBe("complement");
      // 5 in LSB-first: [1, 0, 1, 0]
      expect(trace.aluTrace!.inputA).toEqual([1, 0, 1, 0]);
      // Result: NOT(5) = 10 in LSB-first: [0, 1, 0, 1]
      expect(trace.aluTrace!.result).toEqual([0, 1, 0, 1]);
    });
  });

  // ==========================================================================
  // Memory access detection
  // ==========================================================================

  describe("memory access", () => {
    it("should detect LD (register read)", () => {
      const cpu = new TracingCPU();
      // LDM 9, XCH R5, LD R5
      cpu.loadProgram(new Uint8Array([0xd9, 0xb5, 0xa5, 0x01]));

      cpu.step(); // LDM 9
      cpu.step(); // XCH R5

      const trace = cpu.step(); // LD R5

      expect(trace.memoryAccess).toBeDefined();
      expect(trace.memoryAccess!.type).toBe("reg_read");
      expect(trace.memoryAccess!.address).toBe(5);
      expect(trace.memoryAccess!.value).toBe(9);
    });

    it("should detect XCH (register write)", () => {
      const cpu = new TracingCPU();
      // LDM 6, XCH R2
      cpu.loadProgram(new Uint8Array([0xd6, 0xb2, 0x01]));

      cpu.step(); // LDM 6

      const trace = cpu.step(); // XCH R2

      expect(trace.memoryAccess).toBeDefined();
      expect(trace.memoryAccess!.type).toBe("reg_write");
      expect(trace.memoryAccess!.address).toBe(2);
    });

    it("should detect INC (register write)", () => {
      const cpu = new TracingCPU();
      // LDM 3, XCH R0, INC R0
      cpu.loadProgram(new Uint8Array([0xd3, 0xb0, 0x60, 0x01]));

      cpu.step(); // LDM 3
      cpu.step(); // XCH R0

      const trace = cpu.step(); // INC R0

      expect(trace.memoryAccess).toBeDefined();
      expect(trace.memoryAccess!.type).toBe("reg_write");
      expect(trace.memoryAccess!.address).toBe(0);
      expect(trace.memoryAccess!.value).toBe(4); // 3 + 1 = 4
    });
  });

  // ==========================================================================
  // Delegated getters
  // ==========================================================================

  describe("delegated getters", () => {
    it("should expose accumulator", () => {
      const cpu = new TracingCPU();
      cpu.loadProgram(new Uint8Array([0xd7, 0x01]));
      cpu.step();
      expect(cpu.accumulator).toBe(7);
    });

    it("should expose registers", () => {
      const cpu = new TracingCPU();
      cpu.loadProgram(new Uint8Array([0xd5, 0xb3, 0x01]));
      cpu.step(); // LDM 5
      cpu.step(); // XCH R3
      expect(cpu.registers[3]).toBe(5);
    });

    it("should expose carry flag", () => {
      const cpu = new TracingCPU();
      // STC (set carry)
      cpu.loadProgram(new Uint8Array([0xfa, 0x01]));
      cpu.step();
      expect(cpu.carry).toBe(true);
    });

    it("should expose pc", () => {
      const cpu = new TracingCPU();
      cpu.loadProgram(new Uint8Array([0x00, 0x00, 0x01]));
      expect(cpu.pc).toBe(0);
      cpu.step(); // NOP
      expect(cpu.pc).toBe(1);
    });

    it("should expose halted", () => {
      const cpu = new TracingCPU();
      cpu.loadProgram(new Uint8Array([0x01])); // HLT
      expect(cpu.halted).toBe(false);
      cpu.step();
      expect(cpu.halted).toBe(true);
    });

    it("should allow setting romPort", () => {
      const cpu = new TracingCPU();
      cpu.romPort = 0x5;
      expect(cpu.romPort).toBe(0x5);
    });
  });

  // ==========================================================================
  // ROM integration
  // ==========================================================================

  describe("ROM integration", () => {
    it("should load and run the Busicom ROM", () => {
      const cpu = new TracingCPU();
      cpu.loadProgram(getBusicomROM());

      // Run a few instructions — should not crash
      for (let i = 0; i < 100; i++) {
        if (cpu.halted) break;
        cpu.step();
      }

      expect(cpu.traceHistory.length).toBeGreaterThan(0);
    });
  });
});
