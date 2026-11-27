import { Bus, type Permissions } from "./bus.ts";
import { Cpu } from "./cpu.ts";
import { ConfigurationRom, ConsoleDevice, Ram } from "./devices.ts";

export interface BoardConfig {
  id: string;
  ramEnd: number;
  consoleAddress: number;
}

export interface Board {
  readonly config: BoardConfig;
  readonly bus: Bus;
  readonly ram: Ram;
  readonly console: ConsoleDevice;
  readonly cpu: Cpu;
}

export const CONSOLE_PROGRAM_ID = "ascii-console-demo-v1";

/** Same bytecode on both boards: resolve a console pointer, write OK and newline, then halt. */
export const CONSOLE_PROGRAM = new Uint8Array([
  0x10, 0x00, 0x00, // LDX_ABS 0x0000: load the board console address from configuration ROM.
  0x20, 0x4f, 0x30, // LDA_IMM 'O'; STAX.
  0x20, 0x4b, 0x30, // LDA_IMM 'K'; STAX.
  0x20, 0x0a, 0x30, // LDA_IMM newline; STAX.
  0xff,             // HALT.
]);

const RWX: Permissions = { read: true, write: true, execute: true };
const READ_ONLY: Permissions = { read: true, write: false, execute: false };
const WRITE_ONLY: Permissions = { read: false, write: true, execute: false };

/** A board is only a checked memory map and component bundle, not an ISA claim. */
export function createBoard(config: BoardConfig, program: Uint8Array = CONSOLE_PROGRAM): Board {
  if (!config.id || !Number.isSafeInteger(config.ramEnd) || config.ramEnd <= 0x0100 || config.ramEnd > Bus.addressLimit) throw new Error("invalid board configuration");
  if (!Number.isSafeInteger(config.consoleAddress) || config.consoleAddress < 0 || config.consoleAddress >= Bus.addressLimit) throw new Error("invalid console address");

  const bus = new Bus();
  const rom = new ConfigurationRom(config.consoleAddress);
  const ram = new Ram(config.ramEnd - 0x0100);
  const consoleDevice = new ConsoleDevice();
  ram.load(0, program);
  bus.addRegion("configuration-rom", 0x0000, 0x0002, READ_ONLY, rom);
  bus.addRegion("ram", 0x0100, config.ramEnd, RWX, ram);
  bus.addRegion("console", config.consoleAddress, config.consoleAddress + 1, WRITE_ONLY, consoleDevice);
  const cpu = new Cpu(0x0100);
  return { config: Object.freeze({ ...config }), bus, ram, console: consoleDevice, cpu };
}

export const BOARD_CONFIGS: BoardConfig[] = [
  { id: "museum-board-16k", ramEnd: 0x4100, consoleAddress: 0xff00 },
  { id: "compact-board-8k", ramEnd: 0x2100, consoleAddress: 0xe000 },
];
