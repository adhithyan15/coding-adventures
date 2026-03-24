/**
 * ==========================================================================
 * useARM1 — React Hook for ARM1 Simulator State
 * ==========================================================================
 *
 * This hook owns the CPU instance and manages all simulation state.
 * Components call step/runN/runToEnd/reset and receive a SimulatorState
 * snapshot on each update.
 *
 * # Why useRef for the CPU?
 *
 * The ARM1 instance is mutable and expensive to recreate. We store it in
 * a ref so React never tries to diff or re-create it. All CPU reads happen
 * in callbacks; the SimulatorState snapshot in useState drives rendering.
 *
 * # Memory layout
 *
 * The simulator allocates 4 KB of memory (0x000–0xFFF). Programs load
 * at address 0. The Array Max program places its data at 0x200.
 */

import { useState, useCallback, useRef } from "react";
import {
  ARM1,
  decode,
  disassemble,
  barrelShift,
  decodeImmediate,
  INST_DATA_PROCESSING,
  SHIFT_LSL,
  SHIFT_LSR,
  SHIFT_ASR,
  SHIFT_ROR,
  FLAG_I,
  FLAG_F,
} from "@coding-adventures/arm1-simulator";
import type { DecodedInstruction } from "@coding-adventures/arm1-simulator";
import type {
  ExtendedTrace,
  PipelineState,
  PipelineStage,
  ShiftDetail,
  ShiftTypeName,
  SimulatorState,
} from "../simulator/types.js";
import { PROGRAMS } from "../simulator/programs.js";
import type { Program } from "../simulator/programs.js";

const MAX_TRACES = 100;
const MEMORY_SIZE = 4 * 1024; // 4 KiB — enough for all demo programs + data
const STALLED: PipelineStage = { pc: 0, raw: 0, mnemonic: "—", valid: false };

// ==========================================================================
// Barrel Shift Analysis
// ==========================================================================
//
// After a step completes, we look back at the instruction that just ran
// and reconstruct the barrel shifter inputs/outputs from the saved
// register-before snapshot. This gives the Barrel Shifter tab its data.

function computeShiftDetail(
  decoded: DecodedInstruction,
  regsBefore: number[],
): ShiftDetail | undefined {
  // Only data processing instructions use the barrel shifter.
  if (decoded.type !== INST_DATA_PROCESSING) return undefined;

  const r15before = regsBefore[15] ?? 0;
  const carryIn = (r15before >>> 29) & 1;
  const cin = carryIn !== 0;

  if (decoded.immediate) {
    // Immediate form: the 8-bit constant is rotated right by (rotate_field × 2).
    // Visualise this as a ROR to show the concept, but only when rotate ≠ 0.
    const rotateAmt = decoded.rotate * 2;
    if (rotateAmt === 0) return undefined;
    const [output, carryOut] = decodeImmediate(decoded.imm8, decoded.rotate);
    return {
      input: decoded.imm8,
      shiftType: "ROR",
      amount: rotateAmt,
      output,
      carryOut,
      isNop: false,
    };
  }

  // Register form: Rm is shifted by either an immediate amount or Rs.
  const input = regsBefore[decoded.rm] ?? 0;
  let amount: number;
  if (decoded.shiftByReg) {
    amount = (regsBefore[decoded.rs] ?? 0) & 0xFF;
  } else {
    amount = decoded.shiftImm;
  }

  // Special case: ROR #0 (immediate) encodes RRX — rotate right by 1 through carry.
  if (decoded.shiftType === SHIFT_ROR && amount === 0 && !decoded.shiftByReg) {
    const [output, carryOut] = barrelShift(input, SHIFT_ROR, 0, cin, false);
    return { input, shiftType: "RRX", amount: 0, output, carryOut, isNop: false };
  }

  const [output, carryOut] = barrelShift(input, decoded.shiftType, amount, cin, decoded.shiftByReg);

  const typeNames: Record<number, ShiftTypeName> = {
    [SHIFT_LSL]: "LSL",
    [SHIFT_LSR]: "LSR",
    [SHIFT_ASR]: "ASR",
    [SHIFT_ROR]: "ROR",
  };
  const shiftType = typeNames[decoded.shiftType] ?? "none";
  const isNop = shiftType === "LSL" && amount === 0;

  return { input, shiftType, amount, output, carryOut, isNop };
}

// ==========================================================================
// Pipeline Snapshot
// ==========================================================================
//
// After each step, the pipeline has advanced by one stage. We build a
// snapshot for the Pipeline tab based on where PC is now:
//   Execute  = instruction that just ran (pcBefore)
//   Decode   = instruction at the current PC (was being decoded)
//   Fetch    = instruction at PC+4 (was being fetched)

function buildPipeline(cpu: ARM1, executedPc: number, executedMnemonic: string): PipelineState {
  const decodePc = cpu.pc;
  const fetchPc = (cpu.pc + 4) & 0x03ffffff;

  const decodeRaw = cpu.readWord(decodePc);
  const fetchRaw = cpu.readWord(fetchPc);

  let decodeMnemonic = "—";
  try {
    decodeMnemonic = disassemble(decode(decodeRaw));
  } catch (_) { /* ignore */ }

  let fetchMnemonic = "—";
  try {
    fetchMnemonic = disassemble(decode(fetchRaw));
  } catch (_) { /* ignore */ }

  return {
    execute: { pc: executedPc, raw: cpu.readWord(executedPc), mnemonic: executedMnemonic, valid: true },
    decode:  { pc: decodePc,   raw: decodeRaw,                mnemonic: decodeMnemonic,  valid: true },
    fetch:   { pc: fetchPc,    raw: fetchRaw,                 mnemonic: fetchMnemonic,   valid: true },
  };
}

// ==========================================================================
// State Snapshot
// ==========================================================================

function buildState(
  cpu: ARM1,
  traces: ExtendedTrace[],
  pipeline: PipelineState,
  totalCycles: number,
  programName: string,
): SimulatorState {
  const registers: number[] = [];
  for (let i = 0; i < 16; i++) registers.push(cpu.readRegister(i));

  const r15 = cpu.readRegister(15);
  return {
    registers,
    flags: cpu.flags,
    irqDisabled: (r15 & FLAG_I) !== 0,
    fiqDisabled: (r15 & FLAG_F) !== 0,
    mode: cpu.mode,
    pc: cpu.pc,
    r15,
    halted: cpu.halted,
    traces,
    pipeline,
    totalCycles,
    programName,
  };
}

// ==========================================================================
// Hook
// ==========================================================================

export interface UseARM1Return {
  state: SimulatorState;
  step: () => void;
  runN: (n: number) => void;
  runToEnd: () => void;
  reset: () => void;
  loadProgram: (index: number) => void;
  programIndex: number;
  programs: typeof PROGRAMS;
  readMemory: (addr: number, count: number) => number[];
}

export function useARM1(): UseARM1Return {
  // CPU lives in a ref so it is never re-created between renders.
  const cpuRef = useRef<ARM1>(new ARM1(MEMORY_SIZE));
  const [programIndex, setProgramIndex] = useState(0);
  const cycleRef = useRef(0);
  const pipelineRef = useRef<PipelineState>({
    fetch:   { ...STALLED },
    decode:  { ...STALLED },
    execute: { ...STALLED },
  });

  // ------------------------------------------------------------------
  // Program loading
  // ------------------------------------------------------------------

  const loadProgramIntoMemory = useCallback((prog: Program, cpu: ARM1) => {
    // Wipe memory, then write code + optional data.
    const blank = new Uint8Array(MEMORY_SIZE);
    for (let i = 0; i < MEMORY_SIZE; i++) cpu.writeByte(i, blank[i]!);
    cpu.loadProgram(prog.code, 0);
    if (prog.data && prog.dataAddr !== undefined) {
      cpu.loadProgram(prog.data, prog.dataAddr);
    }
    cpu.reset();  // PC ← 0, flags ← SVC+I+F, registers ← 0
  }, []);

  const initState = useCallback((prog: Program): SimulatorState => {
    loadProgramIntoMemory(prog, cpuRef.current);
    cycleRef.current = 0;
    const emptyPipeline: PipelineState = {
      fetch:   { ...STALLED },
      decode:  { ...STALLED },
      execute: { ...STALLED },
    };
    pipelineRef.current = emptyPipeline;
    return buildState(cpuRef.current, [], emptyPipeline, 0, prog.name);
  }, [loadProgramIntoMemory]);

  const [state, setState] = useState<SimulatorState>(() =>
    initState(PROGRAMS[0]!),
  );

  // ------------------------------------------------------------------
  // Single step
  // ------------------------------------------------------------------

  const doStep = useCallback((): ExtendedTrace | null => {
    const cpu = cpuRef.current;
    if (cpu.halted) return null;

    // Capture PC and registers BEFORE the step so we can compute shift detail.
    const pcBefore = cpu.pc;
    const rawBefore = cpu.readWord(pcBefore);
    const decoded = decode(rawBefore);
    const regsBefore: number[] = [];
    for (let i = 0; i < 16; i++) regsBefore.push(cpu.readRegister(i));

    // Execute one instruction.
    const trace = cpu.step();
    cycleRef.current++;

    const shift = computeShiftDetail(decoded, regsBefore);
    const extended: ExtendedTrace = {
      ...trace,
      decoded,
      shift,
      cycle: cycleRef.current,
    };

    pipelineRef.current = buildPipeline(cpu, pcBefore, trace.mnemonic);

    return extended;
  }, []);

  // ------------------------------------------------------------------
  // Public controls
  // ------------------------------------------------------------------

  const step = useCallback(() => {
    const extended = doStep();
    if (extended === null) return;
    setState(prev => {
      const traces = [...prev.traces.slice(-(MAX_TRACES - 1)), extended];
      return buildState(cpuRef.current, traces, pipelineRef.current, cycleRef.current, prev.programName);
    });
  }, [doStep]);

  const runN = useCallback((n: number) => {
    const batch: ExtendedTrace[] = [];
    for (let i = 0; i < n; i++) {
      const t = doStep();
      if (t === null) break;
      batch.push(t);
    }
    if (batch.length === 0) return;
    setState(prev => {
      const traces = [...prev.traces, ...batch].slice(-MAX_TRACES);
      return buildState(cpuRef.current, traces, pipelineRef.current, cycleRef.current, prev.programName);
    });
  }, [doStep]);

  const runToEnd = useCallback(() => {
    const batch: ExtendedTrace[] = [];
    for (let i = 0; i < 100_000; i++) {
      const t = doStep();
      if (t === null) break;
      batch.push(t);
      if (cpuRef.current.halted) break;
    }
    setState(prev => {
      const traces = [...prev.traces, ...batch].slice(-MAX_TRACES);
      return buildState(cpuRef.current, traces, pipelineRef.current, cycleRef.current, prev.programName);
    });
  }, [doStep]);

  const reset = useCallback(() => {
    const prog = PROGRAMS[programIndex]!;
    setState(initState(prog));
  }, [programIndex, initState]);

  const loadProgram = useCallback((index: number) => {
    const prog = PROGRAMS[index];
    if (!prog) return;
    setProgramIndex(index);
    setState(initState(prog));
  }, [initState]);

  const readMemory = useCallback((addr: number, count: number): number[] => {
    const bytes: number[] = [];
    for (let i = 0; i < count; i++) bytes.push(cpuRef.current.readByte(addr + i));
    return bytes;
  }, []);

  return {
    state,
    step,
    runN,
    runToEnd,
    reset,
    loadProgram,
    programIndex,
    programs: PROGRAMS,
    readMemory,
  };
}
