/**
 * Calculator hook — manages the CPU instance, key input, and execution.
 *
 * === How calculator I/O works ===
 *
 * The Busicom calculator communicates with the Intel 4004 through two
 * I/O mechanisms:
 *
 *   INPUT (keyboard → CPU):
 *     1. User clicks a calculator key → pendingKey is set
 *     2. CPU runs its keyboard scan loop, executing RDR instructions
 *     3. Before RDR executes, the wrapper sets romPort = pendingKey
 *     4. After RDR, pendingKey is cleared (key consumed)
 *     5. CPU branches based on key value, runs the appropriate routine
 *
 *   OUTPUT (CPU → display):
 *     1. ROM program writes display digits via WMP (RAM output port)
 *     2. React component reads the display buffer from RAM
 *     3. CSS 7-segment display renders each digit
 *
 * === Execution model ===
 *
 * When a key is pressed:
 *   1. Set pendingKey and romPort on the CPU
 *   2. Run the CPU until it executes RDR and gets 0 (no more keys)
 *   3. Read the display buffer from RAM
 *   4. Update React state with new display values
 */

import { useState, useCallback, useRef, useMemo } from "react";
import { TracingCPU } from "../cpu/tracing-cpu.js";
import { getBusicomROM } from "../rom/busicom-rom.js";
import type { DetailedTrace } from "../cpu/types.js";

/**
 * Key codes that map to ROM port values.
 *
 * These match the encoding in the ROM program:
 *   0x1-0x9: digits 1-9
 *   0xA: digit 0
 *   0xB: clear (C)
 *   0xC: add (+)
 *   0xD: subtract (-)
 *   0xE: multiply (×)
 *   0xF: equals (=)
 */
export const KEY_CODES = {
  "1": 0x1,
  "2": 0x2,
  "3": 0x3,
  "4": 0x4,
  "5": 0x5,
  "6": 0x6,
  "7": 0x7,
  "8": 0x8,
  "9": 0x9,
  "0": 0xa,
  C: 0xb,
  "+": 0xc,
  "-": 0xd,
  "×": 0xe,
  "÷": 0xe,
  "=": 0xf,
} as const;

export type KeyName = keyof typeof KEY_CODES;

/**
 * Return type of the useCalculator hook.
 *
 * This is the primary interface between the CPU simulation and the
 * React component tree. All five visualization layers consume this.
 */
export interface CalculatorState {
  /** Current display digits (13 BCD digits, LSB first). */
  displayDigits: number[];

  /** Press a calculator key. Runs the CPU until idle. */
  pressKey: (key: KeyName) => void;

  /** Execute a single CPU instruction (for step-by-step mode). */
  stepOne: () => DetailedTrace | undefined;

  /** Reset the CPU and display. */
  reset: () => void;

  /** Most recent instruction trace (for ALU/gate/transistor views). */
  lastTrace: DetailedTrace | undefined;

  /** Full trace history (for instruction trace log). */
  traceHistory: readonly DetailedTrace[];

  /** Current CPU state accessors. */
  accumulator: number;
  registers: number[];
  carry: boolean;
  pc: number;
  hwStack: number[];
  ramData: number[][][];
  ramOutput: number[];
  halted: boolean;
}

/**
 * React hook that manages the Busicom calculator simulation.
 *
 * Creates and owns the TracingCPU instance, loads the ROM, and provides
 * methods for key input and stepping. All state updates trigger re-renders
 * so the visualization layers stay in sync.
 */
export function useCalculator(): CalculatorState {
  const cpuRef = useRef<TracingCPU | null>(null);
  const [updateCount, setUpdateCount] = useState(0);

  // Initialize CPU on first access
  if (!cpuRef.current) {
    const cpu = new TracingCPU();
    cpu.loadProgram(getBusicomROM());
    // Run initialization (MAIN → CLEAR → KEY_SCAN)
    cpu.runUntilIdle(5000);
    cpuRef.current = cpu;
  }

  const cpu = cpuRef.current;

  /** Force a React re-render after CPU state changes. */
  const triggerUpdate = useCallback(() => {
    setUpdateCount((n) => n + 1);
  }, []);

  /**
   * Press a calculator key.
   *
   * Sets the key code on the ROM port and runs the CPU until it returns
   * to the idle scan loop (RDR returns 0).
   */
  const pressKey = useCallback(
    (key: KeyName) => {
      const code = KEY_CODES[key];
      cpu.romPort = code;
      cpu.runUntilIdle(10000);
      triggerUpdate();
    },
    [cpu, triggerUpdate],
  );

  /** Execute one instruction. */
  const stepOne = useCallback(() => {
    if (cpu.halted) return undefined;
    const trace = cpu.step();
    triggerUpdate();
    return trace;
  }, [cpu, triggerUpdate]);

  /** Reset everything. */
  const reset = useCallback(() => {
    cpu.reset();
    cpu.loadProgram(getBusicomROM());
    cpu.runUntilIdle(5000);
    triggerUpdate();
  }, [cpu, triggerUpdate]);

  /**
   * Read the 13-digit display buffer from RAM.
   *
   * The display buffer is in Bank 0, Register 0, Characters 0-12.
   */
  const displayDigits = useMemo(() => {
    // Suppress unused variable warning — updateCount is used to trigger
    // re-computation of this memo when the CPU state changes.
    void updateCount;
    const ram = cpu.ramData;
    const reg0 = ram[0]?.[0];
    if (!reg0) return new Array(13).fill(0) as number[];
    return reg0.slice(0, 13);
  }, [cpu, updateCount]);

  return {
    displayDigits,
    pressKey,
    stepOne,
    reset,
    lastTrace: cpu.lastTrace,
    traceHistory: cpu.traceHistory,
    accumulator: cpu.accumulator,
    registers: [...cpu.registers],
    carry: cpu.carry,
    pc: cpu.pc,
    hwStack: [...cpu.hwStack],
    ramData: cpu.ramData,
    ramOutput: [...cpu.ramOutput],
    halted: cpu.halted,
  };
}
