/**
 * Tests for the Busicom calculator ROM program.
 *
 * These tests verify that the ROM program correctly implements BCD
 * arithmetic on the Intel 4004 gate-level simulator. We load the ROM,
 * inject key presses via the ROM port, run the CPU until idle, and
 * check the display buffer in RAM.
 */

import { describe, it, expect } from "vitest";
import { Intel4004GateLevel } from "@coding-adventures/intel4004-gatelevel";
import { getBusicomROM, buildBusicomROM, ROM_ADDRESSES } from "./busicom-rom.js";

describe("Busicom ROM", () => {
  // ==========================================================================
  // ROM structure tests
  // ==========================================================================

  describe("ROM structure", () => {
    it("should build a 4096-byte ROM", () => {
      const rom = buildBusicomROM();
      expect(rom).toBeInstanceOf(Uint8Array);
      expect(rom.length).toBe(4096);
    });

    it("should cache the ROM on repeated calls", () => {
      const rom1 = getBusicomROM();
      const rom2 = getBusicomROM();
      expect(rom1).toBe(rom2);
    });

    it("should have non-zero bytes at MAIN entry point", () => {
      const rom = getBusicomROM();
      // MAIN starts with JMS CLEAR (0x51, 0x40) or similar
      expect(rom[ROM_ADDRESSES.MAIN]).not.toBe(0);
    });

    it("should have non-zero bytes at KEY_SCAN entry point", () => {
      const rom = getBusicomROM();
      expect(rom[ROM_ADDRESSES.KEY_SCAN]).not.toBe(0);
    });

    it("should have non-zero bytes at ADD_BCD entry point", () => {
      const rom = getBusicomROM();
      expect(rom[ROM_ADDRESSES.ADD_BCD]).not.toBe(0);
    });

    it("should have non-zero bytes at CLEAR entry point", () => {
      const rom = getBusicomROM();
      expect(rom[ROM_ADDRESSES.CLEAR]).not.toBe(0);
    });

    it("should export ROM_ADDRESSES with correct values", () => {
      expect(ROM_ADDRESSES.MAIN).toBe(0x000);
      expect(ROM_ADDRESSES.KEY_SCAN).toBe(0x010);
      expect(ROM_ADDRESSES.DIGIT_ENTRY).toBe(0x060);
      expect(ROM_ADDRESSES.OP_PRESSED).toBe(0x080);
      expect(ROM_ADDRESSES.EQUALS).toBe(0x0b0);
      expect(ROM_ADDRESSES.ADD_BCD).toBe(0x0f0);
      expect(ROM_ADDRESSES.SUB_BCD).toBe(0x130);
      expect(ROM_ADDRESSES.MUL_BCD).toBe(0x160);
      expect(ROM_ADDRESSES.DIV_BCD).toBe(0x1a0);
      expect(ROM_ADDRESSES.DISPLAY).toBe(0x1e0);
      expect(ROM_ADDRESSES.CLEAR).toBe(0x220);
    });
  });

  // ==========================================================================
  // CPU execution tests
  // ==========================================================================

  describe("CPU execution", () => {
    /**
     * Helper: create a CPU with the Busicom ROM loaded and initialized.
     * Runs the MAIN routine (which calls CLEAR and enters KEY_SCAN).
     */
    function createCalculator(): Intel4004GateLevel {
      const cpu = new Intel4004GateLevel();
      cpu.loadProgram(getBusicomROM());
      // Run initialization — MAIN calls CLEAR then enters KEY_SCAN loop
      runUntilScan(cpu, 5000);
      return cpu;
    }

    /**
     * Run the CPU until it executes RDR with no key (idle in scan loop).
     */
    function runUntilScan(cpu: Intel4004GateLevel, maxSteps: number = 10000): void {
      for (let i = 0; i < maxSteps; i++) {
        const trace = cpu.step();
        // RDR = 0xEA
        if (trace.raw === 0xea && cpu.accumulator === 0) {
          return;
        }
      }
    }

    /**
     * Press a key: set ROM port, run until idle.
     *
     * After the first RDR reads the key, we clear the port so the CPU
     * can return to idle in the scan loop (RDR returns 0 = no key).
     */
    function pressKey(cpu: Intel4004GateLevel, keyCode: number): void {
      cpu.romPort = keyCode;
      let keyConsumed = false;
      for (let i = 0; i < 10000; i++) {
        const trace = cpu.step();
        // RDR = 0xEA
        if (trace.raw === 0xea) {
          if (keyConsumed) {
            // Second RDR — if acc is 0, we're idle
            if (cpu.accumulator === 0) return;
          } else {
            // First RDR consumed the key — clear port
            keyConsumed = true;
            cpu.romPort = 0;
          }
        }
      }
    }

    /**
     * Read the display digit at position 0 (LSB) from RAM.
     */
    function readDisplay(cpu: Intel4004GateLevel): number[] {
      const ram = cpu.ramData;
      return ram[0][0].slice(0, 13);
    }

    it("should initialize with display showing 0", () => {
      const cpu = createCalculator();
      const display = readDisplay(cpu);
      // All digits should be 0 after CLEAR
      expect(display[0]).toBe(0);
    });

    it("should enter a single digit", () => {
      const cpu = createCalculator();
      pressKey(cpu, 0x5); // Press '5'
      const display = readDisplay(cpu);
      expect(display[0]).toBe(5);
    });

    it("should enter digit 0 (key code 0xA)", () => {
      const cpu = createCalculator();
      pressKey(cpu, 0xa); // Press '0'
      const display = readDisplay(cpu);
      expect(display[0]).toBe(0);
    });

    it("should enter digits 1-9", () => {
      for (let digit = 1; digit <= 9; digit++) {
        const cpu = createCalculator();
        pressKey(cpu, digit);
        const display = readDisplay(cpu);
        expect(display[0]).toBe(digit);
      }
    });

    it("should perform 2 + 3 = 5", () => {
      const cpu = createCalculator();
      pressKey(cpu, 0x2); // Press '2'
      pressKey(cpu, 0xc); // Press '+' (op code 1 = add injected by KEY_SCAN)
      pressKey(cpu, 0x3); // Press '3'
      pressKey(cpu, 0xf); // Press '='
      const display = readDisplay(cpu);
      expect(display[0]).toBe(5);
    });

    it("should perform 5 + 4 = 9", () => {
      const cpu = createCalculator();
      pressKey(cpu, 0x5);
      pressKey(cpu, 0xc); // '+'
      pressKey(cpu, 0x4);
      pressKey(cpu, 0xf); // '='
      const display = readDisplay(cpu);
      expect(display[0]).toBe(9);
    });

    it("should perform 7 + 8 = 15 (two digits via BCD)", () => {
      const cpu = createCalculator();
      pressKey(cpu, 0x7);
      pressKey(cpu, 0xc); // '+'
      pressKey(cpu, 0x8);
      pressKey(cpu, 0xf); // '='
      const display = readDisplay(cpu);
      // 15 in BCD: digit 0 = 5, digit 1 = 1
      expect(display[0]).toBe(5);
      expect(display[1]).toBe(1);
    });
  });
});
