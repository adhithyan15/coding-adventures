import assert from "node:assert/strict";
import test from "node:test";
import { Bus, BusFault } from "../src/bus.ts";
import { BOARD_CONFIGS, CONSOLE_PROGRAM, CONSOLE_PROGRAM_ID, createBoard, type BoardConfig } from "../src/board.ts";
import { BoardMachine } from "../src/board-machine.ts";
import { Cpu } from "../src/cpu.ts";
import { ConfigurationRom, ConsoleDevice, Ram } from "../src/devices.ts";

test("machine snapshot restoration rejects inconsistent CPU or device state", () => {
  const machine = BoardMachine.replay(BOARD_CONFIGS[0]!, [1, 4, 2, 1], CONSOLE_PROGRAM, CONSOLE_PROGRAM_ID);
  const forged = JSON.parse(machine.serialize()) as { cpu: { halted: boolean } };
  forged.cpu.halted = false;
  assert.throws(() => BoardMachine.restore(JSON.stringify(forged)), /does not match replayed budgets/);
});

test("bus enforces the address width, mapped ranges, overlap and permissions", () => {
  const bus = new Bus();
  const rom = new ConfigurationRom(0x1234);
  bus.addRegion("rom", 0, 2, { read: true, write: false, execute: false }, rom);
  assert.deepEqual([bus.readByte(0), bus.readByte(1)], [0x34, 0x12]);
  assert.throws(() => bus.fetchByte(0), (error: unknown) => error instanceof BusFault && error.code === "EPERM");
  assert.throws(() => bus.writeByte(0, 1), (error: unknown) => error instanceof BusFault && error.code === "EPERM");
  assert.throws(() => bus.readByte(2), (error: unknown) => error instanceof BusFault && error.code === "EUNMAPPED");
  assert.throws(() => bus.readByte(0x10000), (error: unknown) => error instanceof BusFault && error.code === "EADDRESS");
  assert.throws(() => bus.writeByte(0x10000, 0x1ff), (error: unknown) => error instanceof BusFault && error.code === "EADDRESS");
  assert.throws(() => bus.addRegion("overlap", 1, 3, { read: true, write: true, execute: true }, new Ram(2)),
    (error: unknown) => error instanceof BusFault && error.code === "EOVERLAP");
  assert.throws(() => bus.addRegion("outside", 0xffff, 0x10001, { read: true, write: false, execute: false }, new Ram(2)),
    (error: unknown) => error instanceof BusFault && error.code === "ERANGE");
  assert.throws(() => createBoard({ id: "overlap", ramEnd: 0x4000, consoleAddress: 0x0100 }),
    (error: unknown) => error instanceof BusFault && error.code === "EOVERLAP");
  assert.throws(() => createBoard({ id: "outside", ramEnd: 0x100000, consoleAddress: 0xff00 }), /invalid board configuration/);
  const board = createBoard(BOARD_CONFIGS[0]!);
  assert.throws(() => board.bus.readByte(board.config.consoleAddress),
    (error: unknown) => error instanceof BusFault && error.code === "EPERM");
});

test("a rejected write is checked before a target is mutated", () => {
  const bus = new Bus();
  const ram = new Ram(1);
  bus.addRegion("ram", 0x0100, 0x0101, { read: true, write: true, execute: true }, ram);
  assert.throws(() => bus.writeByte(0x0100, 0x100),
    (error: unknown) => error instanceof BusFault && error.code === "EDATA");
  assert.deepEqual(ram.snapshot(), [0]);
});

test("built-in targets reject invalid bytes before changing their state", () => {
  const ram = new Ram(1);
  const consoleDevice = new ConsoleDevice();
  assert.throws(() => ram.write(0, 0x100), /RAM byte is out of range/);
  assert.throws(() => consoleDevice.write(0, 0x100), /console byte is out of range/);
  assert.deepEqual(ram.snapshot(), [0]);
  assert.deepEqual(consoleDevice.outputBytes, []);
  assert.throws(() => new ConfigurationRom(0x10000), /outside the 16-bit address width/);
  assert.throws(() => new ConfigurationRom(-1), /outside the 16-bit address width/);
});

test("CPU instruction budgets are bounded and deterministic at halt", () => {
  const machine = new BoardMachine(BOARD_CONFIGS[0]!);
  assert.deepEqual(machine.run(0), []);
  assert.equal(machine.snapshot().cpu.instructions, 0);
  assert.throws(() => machine.run(-1), /nonnegative safe integer/);
  assert.throws(() => machine.run(0.5), /nonnegative safe integer/);
  assert.equal(machine.run(1).length, 1);
  assert.equal(machine.snapshot().cpu.instructions, 1);
  machine.run(20);
  assert.equal(machine.snapshot().cpu.instructions, 8);
  assert.equal(machine.snapshot().cpu.halted, true);
  assert.deepEqual(machine.run(3), []);
  assert.equal(machine.snapshot().cpu.instructions, 8);
});

test("a failed bus operation leaves the instruction registers and device effects unchanged", () => {
  const board = createBoard(BOARD_CONFIGS[0]!, new Uint8Array([0x20, 0x41, 0x30]));
  board.cpu.step(board.bus); // LDA_IMM succeeds and leaves the address register at read-only ROM.
  const before = board.cpu.state;
  assert.throws(() => board.cpu.step(board.bus), (error: unknown) => error instanceof BusFault && error.code === "EPERM");
  assert.deepEqual(board.cpu.state, before);
  assert.deepEqual(board.console.outputBytes, []);
});

test("fetches outside mapped executable memory and invalid opcodes fail before state changes", () => {
  const cpu = new Cpu(0x4444);
  const emptyBus = new Bus();
  assert.throws(() => cpu.step(emptyBus), (error: unknown) => error instanceof BusFault && error.code === "EUNMAPPED");
  assert.deepEqual(cpu.state, { pc: 0x4444, accumulator: 0, addressRegister: 0, halted: false, instructions: 0 });

  const config: BoardConfig = { id: "bad-opcode", ramEnd: 0x0200, consoleAddress: 0xff00 };
  const board = createBoard(config, new Uint8Array([0x00]));
  assert.throws(() => board.cpu.step(board.bus), /EOPCODE/);
  assert.equal(board.cpu.state.instructions, 0);
  assert.deepEqual(board.console.outputBytes, []);
});
