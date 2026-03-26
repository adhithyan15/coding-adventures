/**
 * Bootloader -- generates RISC-V machine code that loads the kernel from
 * disk to RAM and transfers control.
 */

import { encodeAddi, encodeLui, encodeLw, encodeSw, encodeBeq, encodeBne, encodeJalr, encodeJal, assemble } from "@coding-adventures/riscv-simulator";

// === Well-known addresses ===
export const DEFAULT_ENTRY_ADDRESS = 0x00010000;
export const DEFAULT_KERNEL_DISK_OFFSET = 0x00080000;
export const DEFAULT_KERNEL_LOAD_ADDRESS = 0x00020000;
export const DEFAULT_STACK_BASE = 0x0006fff0;
export const DISK_MEMORY_MAP_BASE = 0x10000000;
export const BOOT_PROTOCOL_ADDRESS = 0x00001000;
export const BOOT_PROTOCOL_MAGIC = 0xb007cafe;

// === Config ===
export interface BootloaderConfig {
  entryAddress: number;
  kernelDiskOffset: number;
  kernelLoadAddress: number;
  kernelSize: number;
  stackBase: number;
}

export function defaultBootloaderConfig(): BootloaderConfig {
  return {
    entryAddress: DEFAULT_ENTRY_ADDRESS,
    kernelDiskOffset: DEFAULT_KERNEL_DISK_OFFSET,
    kernelLoadAddress: DEFAULT_KERNEL_LOAD_ADDRESS,
    kernelSize: 0,
    stackBase: DEFAULT_STACK_BASE,
  };
}

// === Bootloader generator ===
export interface AnnotatedInstruction {
  address: number;
  machineCode: number;
  assembly: string;
  comment: string;
}

function signExtend12(val: number): number {
  val = val & 0xfff;
  return val >= 0x800 ? val - 0x1000 : val;
}

function emitLi(instructions: AnnotatedInstruction[], addr: { value: number }, rd: number, value: number, comment: string): void {
  let upper = value >>> 12;
  const lower = value & 0xfff;
  if (lower >= 0x800) upper++;

  if (upper !== 0) {
    instructions.push({ address: addr.value, machineCode: encodeLui(rd, upper), assembly: `lui x${rd}, 0x${upper.toString(16)}`, comment });
    addr.value += 4;
    if (lower !== 0) {
      const sl = signExtend12(lower);
      instructions.push({ address: addr.value, machineCode: encodeAddi(rd, rd, sl), assembly: `addi x${rd}, x${rd}, ${sl}`, comment });
      addr.value += 4;
    }
  } else if (lower !== 0) {
    const sl = signExtend12(lower);
    instructions.push({ address: addr.value, machineCode: encodeAddi(rd, 0, sl), assembly: `addi x${rd}, x0, ${sl}`, comment });
    addr.value += 4;
  } else {
    instructions.push({ address: addr.value, machineCode: encodeAddi(rd, 0, 0), assembly: `addi x${rd}, x0, 0`, comment: comment + " (0)" });
    addr.value += 4;
  }
}

export class Bootloader {
  constructor(public readonly config: BootloaderConfig) {}

  generate(): number[] {
    const annotated = this.generateWithComments();
    return assemble(annotated.map(a => a.machineCode));
  }

  generateWithComments(): AnnotatedInstruction[] {
    const instructions: AnnotatedInstruction[] = [];
    const addr = { value: this.config.entryAddress };
    const emit = (code: number, asm: string, comment: string) => {
      instructions.push({ address: addr.value, machineCode: code, assembly: asm, comment });
      addr.value += 4;
    };

    // Phase 1: validate magic
    emit(encodeLui(5, 1), "lui t0, 1", "Phase 1: t0 = 0x1000");
    emit(encodeLw(6, 5, 0), "lw t1, 0(t0)", "Phase 1: read magic");
    // Load expected magic 0xB007CAFE
    let magicUpper = 0xb007cafe >>> 12;
    const magicLower = 0xb007cafe & 0xfff;
    if (magicLower >= 0x800) magicUpper++;
    emit(encodeLui(7, magicUpper), `lui t2, 0x${magicUpper.toString(16)}`, "Phase 1: expected magic upper");
    if (magicLower !== 0) emit(encodeAddi(7, 7, signExtend12(magicLower)), `addi t2, t2, ${signExtend12(magicLower)}`, "Phase 1: expected magic lower");

    const haltBranchIdx = instructions.length;
    emit(encodeBne(6, 7, 0), "bne t1, t2, halt", "Phase 1: halt if magic wrong");

    // Phase 2: load addresses
    const source = (DISK_MEMORY_MAP_BASE + this.config.kernelDiskOffset) >>> 0;
    emitLi(instructions, addr, 5, source, "Phase 2: source");
    emitLi(instructions, addr, 6, this.config.kernelLoadAddress, "Phase 2: dest");
    emitLi(instructions, addr, 7, this.config.kernelSize, "Phase 2: size");

    // Phase 3: copy loop
    emit(encodeBeq(7, 0, 24), "beq t2, x0, +24", "Phase 3: skip if size=0");
    const loopAddr = addr.value;
    emit(encodeLw(28, 5, 0), "lw t3, 0(t0)", "Phase 3: load word");
    emit(encodeSw(28, 6, 0), "sw t3, 0(t1)", "Phase 3: store word");
    emit(encodeAddi(5, 5, 4), "addi t0, t0, 4", "Phase 3: src += 4");
    emit(encodeAddi(6, 6, 4), "addi t1, t1, 4", "Phase 3: dst += 4");
    emit(encodeAddi(7, 7, -4), "addi t2, t2, -4", "Phase 3: remaining -= 4");
    emit(encodeBne(7, 0, loopAddr - addr.value), `bne t2, x0, ${loopAddr - addr.value}`, "Phase 3: loop");

    // Phase 4: set stack, jump to kernel
    emitLi(instructions, addr, 2, this.config.stackBase, "Phase 4: sp");
    emitLi(instructions, addr, 5, this.config.kernelLoadAddress, "Phase 4: kernel entry");
    emit(encodeJalr(0, 5, 0), "jalr x0, t0, 0", "Phase 4: jump to kernel");

    // halt
    const haltAddr = addr.value;
    emit(encodeJal(0, 0), "jal x0, 0", "Halt: infinite loop");

    // Patch halt branch
    const branchPC = instructions[haltBranchIdx].address;
    instructions[haltBranchIdx].machineCode = encodeBne(6, 7, haltAddr - branchPC);

    return instructions;
  }

  instructionCount(): number { return this.generateWithComments().length; }
  estimateCycles(): number { return Math.floor(this.config.kernelSize / 4) * 6 + 20; }
}

// === Disk Image ===
export const DISK_KERNEL_OFFSET = 0x00080000;
export const DEFAULT_DISK_SIZE = 2 * 1024 * 1024;

export class DiskImage {
  readonly data: Uint8Array;

  constructor(sizeBytes: number = DEFAULT_DISK_SIZE) {
    this.data = new Uint8Array(sizeBytes);
  }

  loadKernel(kernelBinary: Uint8Array | number[]): void { this.loadAt(DISK_KERNEL_OFFSET, kernelBinary); }

  loadAt(offset: number, data: Uint8Array | number[]): void {
    if (offset + data.length > this.data.length) throw new Error("data exceeds disk size");
    for (let i = 0; i < data.length; i++) this.data[offset + i] = data[i];
  }

  readWord(offset: number): number {
    if (offset < 0 || offset + 4 > this.data.length) return 0;
    return (this.data[offset] | (this.data[offset + 1] << 8) | (this.data[offset + 2] << 16) | (this.data[offset + 3] << 24)) >>> 0;
  }

  size(): number { return this.data.length; }
}
