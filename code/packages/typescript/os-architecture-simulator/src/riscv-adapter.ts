import { Memory } from "@coding-adventures/cpu-simulator";
import {
  RiscVSimulator,
  encodeAdd,
  encodeAddi,
  encodeEcall,
  encodeLbu,
  encodeSb,
  encodeSlli,
} from "@coding-adventures/riscv-simulator";
import { Bus, type DeviceEffect } from "./bus.ts";
import { BOARD_CONFIGS, type BoardConfig, createBoard } from "./board.ts";
import type { CpuEvent } from "./cpu.ts";

export const RISCV_PROGRAM_ID = "rv32i-console-demo-v1";
export const RISCV_PROGRAM_WORDS: number[] = [
  encodeLbu(1, 0, 0),     // x1 = board console pointer low byte.
  encodeLbu(2, 0, 1),     // x2 = board console pointer high byte.
  encodeSlli(2, 2, 8),    // Move the high byte into its address position.
  encodeAdd(1, 1, 2),     // x1 = full MMIO console address.
  encodeAddi(3, 0, 79),   // x3 = 'O'.
  encodeSb(3, 1, 0),      // Write O through the memory bus.
  encodeAddi(3, 0, 75),   // x3 = 'K'.
  encodeSb(3, 1, 0),
  encodeAddi(3, 0, 10),   // x3 = newline.
  encodeSb(3, 1, 0),
  encodeEcall(),           // Existing default zero mtvec halts the simulator.
];

export function wordsToBytes(words: number[]): Uint8Array {
  const bytes = new Uint8Array(words.length * 4);
  words.forEach((word, index) => {
    const value = word >>> 0;
    bytes[index * 4] = value & 0xff;
    bytes[index * 4 + 1] = (value >>> 8) & 0xff;
    bytes[index * 4 + 2] = (value >>> 16) & 0xff;
    bytes[index * 4 + 3] = (value >>> 24) & 0xff;
  });
  return bytes;
}

/**
 * Adapter for the repository CPU's existing concrete Memory seam.
 * Fetch and data reads both call readWord; this probe treats readWord at the
 * current PC as an instruction fetch and other word reads as data.
 */
export class BusBackedMemory extends Memory {
  private effects: DeviceEffect[] = [];
  private readonly bus: Bus;
  private readonly programCounter: () => number;
  constructor(bus: Bus, programCounter: () => number) {
    super(Bus.addressLimit);
    this.bus = bus;
    this.programCounter = programCounter;
  }

  override readByte(address: number): number { return this.bus.readByte(address); }

  override writeByte(address: number, value: number): void {
    this.effects.push(...this.bus.writeByte(address, value));
  }

  override readWord(address: number): number {
    const fetch = address === this.programCounter();
    const read = (byteAddress: number): number => fetch ? this.bus.fetchByte(byteAddress) : this.bus.readByte(byteAddress);
    return (read(address) | (read(address + 1) << 8) | (read(address + 2) << 16) | (read(address + 3) << 24)) >>> 0;
  }

  override writeWord(address: number, value: number): void {
    const word = value >>> 0;
    this.writeByte(address, word & 0xff);
    this.writeByte(address + 1, (word >>> 8) & 0xff);
    this.writeByte(address + 2, (word >>> 16) & 0xff);
    this.writeByte(address + 3, (word >>> 24) & 0xff);
  }

  override loadBytes(address: number, data: number[] | Uint8Array): void {
    data.forEach((byte, index) => this.writeByte(address + index, byte));
  }

  override dump(start = 0, length = 16): number[] {
    return Array.from({ length }, (_, offset) => this.bus.readByte(start + offset));
  }

  drainEffects(): DeviceEffect[] {
    const captured = this.effects;
    this.effects = [];
    return captured.map(effect => structuredClone(effect));
  }
}

export interface RiscVBoardSnapshot {
  schemaVersion: 1;
  board: BoardConfig;
  program: string;
  programWords: number[];
  programBytes: number[];
  pc: number;
  halted: boolean;
  cycles: number;
  registers: Record<string, number>;
  consoleBytes: number[];
  consoleText: string;
  instructions: string[];
  deviceEffects: DeviceEffect[];
  budget: number;
}

/** Run real RV32I encodings through the repository decoder/executor and HOS bus. */
export function runRiscVBoard(config: BoardConfig, budget = 16, programWords = RISCV_PROGRAM_WORDS): RiscVBoardSnapshot {
  if (!Number.isSafeInteger(budget) || budget < 0) throw new Error("instruction budget must be a nonnegative safe integer");
  const programBytes = wordsToBytes(programWords);
  const board = createBoard(config, programBytes);
  const simulator = new RiscVSimulator(Bus.addressLimit);
  const memory = new BusBackedMemory(board.bus, () => simulator.cpu.pc);
  simulator.cpu.memory = memory;
  simulator.cpu.pc = 0x0100;

  const traces = [] as ReturnType<RiscVSimulator["step"]>[];
  const deviceEffects: DeviceEffect[] = [];
  for (let index = 0; index < budget && !simulator.cpu.halted; index++) {
    traces.push(simulator.step());
    deviceEffects.push(...memory.drainEffects());
  }
  const instructions: CpuEvent[] = traces.map(trace => ({
    step: trace.cycle + 1,
    kind: "riscv.instruction",
    data: { pc: String(trace.fetch.pc), mnemonic: trace.decode.mnemonic, description: trace.execute.description },
  }));
  return {
    schemaVersion: 1,
    board: { ...board.config },
    program: RISCV_PROGRAM_ID,
    programWords: [...programWords],
    programBytes: Array.from(programBytes),
    pc: simulator.cpu.pc,
    halted: simulator.cpu.halted,
    cycles: simulator.cpu.cycle,
    registers: simulator.cpu.registers.dump(),
    consoleBytes: board.console.outputBytes,
    consoleText: board.console.text,
    instructions: instructions.map(event => event.data.mnemonic),
    deviceEffects,
    budget,
  };
}

export function runRiscVExamples(budget = 16): RiscVBoardSnapshot[] {
  return BOARD_CONFIGS.map(config => runRiscVBoard(config, budget));
}
