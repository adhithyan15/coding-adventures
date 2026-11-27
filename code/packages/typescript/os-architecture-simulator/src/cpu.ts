import type { DeviceEffect } from "./bus.ts";
import { Bus, BusFault } from "./bus.ts";

export class CpuFault extends Error {
  readonly code: string;
  readonly pc: number;
  constructor(code: string, pc: number) {
    super(`${code} at program counter 0x${pc.toString(16).padStart(4, "0")}`);
    this.code = code;
    this.pc = pc;
  }
}

export interface CpuState {
  pc: number;
  accumulator: number;
  addressRegister: number;
  halted: boolean;
  instructions: number;
}

export interface CpuEvent { step: number; kind: string; data: Record<string, string> }

interface Decoded { mnemonic: string; effects: DeviceEffect[]; accumulator: number; addressRegister: number; halted: boolean }

/** Small synthetic bytecode CPU. Each call executes no more than one instruction. */
export class Cpu {
  private pcValue: number;
  private accumulatorValue = 0;
  private addressRegisterValue = 0;
  private haltedValue = false;
  private instructionCount = 0;

  constructor(startAddress: number) {
    if (!Number.isSafeInteger(startAddress) || startAddress < 0 || startAddress >= Bus.addressLimit) throw new Error("CPU start address is outside its address width");
    this.pcValue = startAddress;
  }

  get halted(): boolean { return this.haltedValue; }
  get state(): CpuState {
    return { pc: this.pcValue, accumulator: this.accumulatorValue, addressRegister: this.addressRegisterValue, halted: this.haltedValue, instructions: this.instructionCount };
  }

  step(bus: Bus): CpuEvent[] {
    if (this.haltedValue) throw new CpuFault("EHALTED", this.pcValue);
    const instructionPc = this.pcValue;
    const opcode = this.fetch(bus, instructionPc, 0);
    const length = this.instructionLength(opcode, instructionPc);
    const nextPc = instructionPc + length;
    if (nextPc >= Bus.addressLimit || !Number.isSafeInteger(this.instructionCount + 1)) throw new CpuFault("EADDRESS", instructionPc);
    const decoded = this.decode(bus, opcode, instructionPc);

    // All validation and bus accesses complete before CPU registers advance.
    this.pcValue = nextPc;
    this.accumulatorValue = decoded.accumulator;
    this.addressRegisterValue = decoded.addressRegister;
    this.haltedValue = decoded.halted;
    this.instructionCount++;
    const step = this.instructionCount;
    const events: CpuEvent[] = [{
      step,
      kind: "cpu.instruction",
      data: { pc: String(instructionPc), opcode: String(opcode), mnemonic: decoded.mnemonic },
    }];
    for (const effect of decoded.effects) events.push({ step, kind: effect.kind, data: effect.data });
    return events;
  }

  private fetch(bus: Bus, instructionPc: number, offset: number): number {
    const address = instructionPc + offset;
    if (!Number.isSafeInteger(address) || address >= Bus.addressLimit) throw new CpuFault("EADDRESS", instructionPc);
    try { return bus.fetchByte(address); }
    catch (error) {
      if (error instanceof BusFault) throw error;
      throw new CpuFault("EBUS", instructionPc);
    }
  }

  private decode(bus: Bus, opcode: number, pc: number): Decoded {
    let accumulator = this.accumulatorValue;
    let addressRegister = this.addressRegisterValue;
    let halted = false;
    let mnemonic = "";
    let effects: DeviceEffect[] = [];
    switch (opcode) {
      case 0x10: { // LDX_ABS: the immediate word names a word in guest address space.
        const low = this.fetch(bus, pc, 1);
        const high = this.fetch(bus, pc, 2);
        const source = low | (high << 8);
        const valueLow = bus.readByte(source);
        const valueHigh = bus.readByte(source + 1);
        addressRegister = valueLow | (valueHigh << 8);
        mnemonic = "LDX_ABS";
        break;
      }
      case 0x20: // LDA_IMM
        accumulator = this.fetch(bus, pc, 1);
        mnemonic = "LDA_IMM";
        break;
      case 0x30: // STAX
        effects = bus.writeByte(addressRegister, accumulator);
        mnemonic = "STAX";
        break;
      case 0xff: // HALT
        halted = true;
        mnemonic = "HALT";
        break;
      default:
        throw new CpuFault("EOPCODE", pc);
    }
    return { mnemonic, effects, accumulator, addressRegister, halted };
  }

  private instructionLength(opcode: number, pc: number): number {
    switch (opcode) {
      case 0x10: return 3;
      case 0x20: return 2;
      case 0x30:
      case 0xff: return 1;
      default: throw new CpuFault("EOPCODE", pc);
    }
  }
}
