/**
 * System Board -- the complete simulated computer.
 *
 * Composes ROM/BIOS, Bootloader, Interrupt Handler, OS Kernel, Display,
 * and RISC-V CPU into a complete system that boots to Hello World.
 */

import {
  RiscVSimulator, CSR_MTVEC, CSR_MEPC, CSR_MSTATUS, MIE,
} from "@coding-adventures/riscv-simulator";
import {
  DisplayDriver, defaultDisplayConfig, BYTES_PER_CELL, type DisplayConfig, DisplaySnapshot,
} from "@coding-adventures/display";
import { InterruptController } from "@coding-adventures/interrupt-handler";
import {
  Bootloader, defaultBootloaderConfig, DiskImage, BOOT_PROTOCOL_MAGIC,
} from "@coding-adventures/bootloader";
import { defaultBIOSConfig, type BIOSConfig, type HardwareInfo } from "@coding-adventures/rom-bios";
import {
  Kernel, defaultKernelConfig, type KernelConfig,
  generateIdleProgram, generateHelloWorldProgram,
  REG_A7, REG_SP, ProcessState,
} from "@coding-adventures/os-kernel";

// === Address space constants ===
export const ROM_BASE = 0xffff0000;
export const BOOT_PROTOCOL_ADDR = 0x00001000;
export const BOOTLOADER_BASE = 0x00010000;
export const KERNEL_BASE = 0x00020000;
export const IDLE_PROCESS_BASE = 0x00030000;
export const USER_PROCESS_BASE = 0x00040000;
export const KERNEL_STACK_TOP = 0x0006fff0;
export const DISK_MAPPED_BASE = 0x10000000;

// === Boot phases ===
export enum BootPhase { PowerOn, BIOS, Bootloader, KernelInit, UserProgram, Idle }

export interface BootEvent { phase: BootPhase; cycle: number; description: string; }

export class BootTrace {
  events: BootEvent[] = [];
  addEvent(phase: BootPhase, cycle: number, description: string): void {
    this.events.push({ phase, cycle, description });
  }
  phases(): BootPhase[] {
    const seen = new Set<BootPhase>();
    const result: BootPhase[] = [];
    for (const e of this.events) { if (!seen.has(e.phase)) { seen.add(e.phase); result.push(e.phase); } }
    return result;
  }
  totalCycles(): number { return this.events.length === 0 ? 0 : this.events[this.events.length - 1].cycle; }
}

// === System Config ===
export interface SystemConfig {
  memorySize: number;
  displayConfig: DisplayConfig;
  biosConfig: BIOSConfig;
  bootloaderConfig: ReturnType<typeof defaultBootloaderConfig>;
  kernelConfig: KernelConfig;
  userProgram: number[] | null;
}

export function defaultSystemConfig(): SystemConfig {
  const biosConfig = defaultBIOSConfig();
  biosConfig.memorySize = 1024 * 1024;
  return {
    memorySize: 1024 * 1024,
    displayConfig: defaultDisplayConfig(),
    biosConfig,
    bootloaderConfig: defaultBootloaderConfig(),
    kernelConfig: defaultKernelConfig(),
    userProgram: null,
  };
}

// === SystemBoard ===
export class SystemBoard {
  config: SystemConfig;
  cpu: RiscVSimulator | null = null;
  display: DisplayDriver | null = null;
  interruptCtrl: InterruptController | null = null;
  kernel: Kernel | null = null;
  trace: BootTrace;
  powered = false;
  cycle = 0;
  currentPhase = BootPhase.PowerOn;
  private kernelBooted = false;
  private previousPC = 0;

  constructor(config: SystemConfig) {
    this.config = config;
    this.trace = new BootTrace();
  }

  powerOn(): void {
    if (this.powered) return;
    const config = this.config;

    // Create CPU with enough memory for disk-mapped region
    const memSize = 0x10200000;
    this.cpu = new RiscVSimulator(memSize);
    this.interruptCtrl = new InterruptController();

    // Create display
    const displayMem = new Uint8Array(config.displayConfig.columns * config.displayConfig.rows * BYTES_PER_CELL);
    this.display = new DisplayDriver(config.displayConfig, displayMem);

    // Create kernel
    this.kernel = new Kernel(config.kernelConfig, this.interruptCtrl, this.display);

    // Generate binaries
    const userProgram = config.userProgram ?? generateHelloWorldProgram(USER_PROCESS_BASE);
    const idleBinary = generateIdleProgram();
    const blConfig = { ...config.bootloaderConfig };
    const kernelStubSize = 16;
    let totalSize = kernelStubSize + idleBinary.length + userProgram.length;
    if (totalSize % 4 !== 0) totalSize += 4 - (totalSize % 4);
    blConfig.kernelSize = totalSize;

    const bl = new Bootloader(blConfig);
    const bootloaderCode = bl.generate();

    // Write boot protocol magic
    writeWord(this.cpu, BOOT_PROTOCOL_ADDR, BOOT_PROTOCOL_MAGIC);
    writeWord(this.cpu, BOOT_PROTOCOL_ADDR + 4, config.memorySize);

    // Load bootloader
    for (let i = 0; i < bootloaderCode.length; i++) {
      this.cpu.cpu.memory.writeByte(BOOTLOADER_BASE + i, bootloaderCode[i]);
    }

    // Create disk image and load kernel
    const disk = new DiskImage();
    const kernelDiskData = new Array(totalSize).fill(0);
    for (let i = 0; i < idleBinary.length; i++) kernelDiskData[kernelStubSize + i] = idleBinary[i];
    for (let i = 0; i < userProgram.length; i++) kernelDiskData[kernelStubSize + idleBinary.length + i] = userProgram[i];
    disk.loadKernel(kernelDiskData);

    // Memory-map disk
    for (let i = 0; i < disk.data.length && DISK_MAPPED_BASE + i < memSize; i++) {
      this.cpu.cpu.memory.writeByte(DISK_MAPPED_BASE + i, disk.data[i]);
    }

    // Pre-load binaries at final locations
    for (let i = 0; i < idleBinary.length; i++) this.cpu.cpu.memory.writeByte(IDLE_PROCESS_BASE + i, idleBinary[i]);
    for (let i = 0; i < userProgram.length; i++) this.cpu.cpu.memory.writeByte(USER_PROCESS_BASE + i, userProgram[i]);

    // Set PC to bootloader
    this.cpu.cpu.pc = BOOTLOADER_BASE;
    this.cpu.csr.write(CSR_MTVEC, 0xdead0000);

    this.powered = true;
    this.currentPhase = BootPhase.PowerOn;
    this.trace.addEvent(BootPhase.PowerOn, 0, "System powered on");
    this.trace.addEvent(BootPhase.BIOS, 0, "BIOS simulated");
    this.currentPhase = BootPhase.BIOS;
  }

  step(): void {
    if (!this.powered || !this.cpu) return;
    this.previousPC = this.cpu.cpu.pc;
    this.cycle++;
    this.cpu.step();
    this.detectPhaseTransition();
    this.handleTrap();
  }

  run(maxCycles: number): BootTrace {
    if (!this.powered) return this.trace;
    for (let i = 0; i < maxCycles; i++) {
      this.step();
      if (this.kernelBooted && this.kernel?.isIdle()) {
        if (this.currentPhase !== BootPhase.Idle) {
          this.currentPhase = BootPhase.Idle;
          this.trace.addEvent(BootPhase.Idle, this.cycle, "System idle");
        }
        break;
      }
      if (this.cpu!.cpu.halted) break;
    }
    return this.trace;
  }

  displaySnapshot(): DisplaySnapshot | null {
    return this.display?.snapshot() ?? null;
  }

  isIdle(): boolean { return this.kernelBooted && !!this.kernel?.isIdle(); }
  getCycleCount(): number { return this.cycle; }
  getCurrentPhase(): BootPhase { return this.currentPhase; }

  private detectPhaseTransition(): void {
    if (!this.cpu) return;
    const pc = this.cpu.cpu.pc;
    switch (this.currentPhase) {
      case BootPhase.BIOS:
        if (pc >= BOOTLOADER_BASE && pc < BOOTLOADER_BASE + 0x10000) {
          this.currentPhase = BootPhase.Bootloader;
          this.trace.addEvent(BootPhase.Bootloader, this.cycle, "Bootloader executing");
        }
        break;
      case BootPhase.Bootloader:
        if (pc >= KERNEL_BASE && pc < KERNEL_BASE + 0x10000) {
          this.currentPhase = BootPhase.KernelInit;
          this.trace.addEvent(BootPhase.KernelInit, this.cycle, "Kernel entry reached");
          this.initializeKernel();
        }
        break;
      case BootPhase.KernelInit:
        if (pc >= USER_PROCESS_BASE && pc < USER_PROCESS_BASE + 0x10000) {
          this.currentPhase = BootPhase.UserProgram;
          this.trace.addEvent(BootPhase.UserProgram, this.cycle, "User program executing");
        }
        break;
    }
  }

  private initializeKernel(): void {
    if (this.kernelBooted || !this.kernel || !this.cpu) return;
    this.kernel.boot();
    this.kernelBooted = true;
    this.trace.addEvent(BootPhase.KernelInit, this.cycle, `Kernel booted: ${this.kernel.processCount()} processes`);
    if (this.kernel.processTable.length > 1) {
      const pcb = this.kernel.processTable[1];
      this.cpu.cpu.pc = pcb.savedPC;
      this.cpu.cpu.registers.write(REG_SP, pcb.stackPointer);
    }
  }

  private handleTrap(): void {
    if (!this.cpu || !this.kernel) return;
    const pc = this.cpu.cpu.pc;
    if (pc !== 0xdead0000) return;
    if (!this.kernelBooted) {
      const mepc = this.cpu.csr.read(CSR_MEPC);
      this.cpu.cpu.pc = mepc + 4;
      this.cpu.csr.write(CSR_MSTATUS, (this.cpu.csr.read(CSR_MSTATUS) | MIE) >>> 0);
      return;
    }
    const syscallNum = this.cpu.cpu.registers.read(REG_A7);
    const mepc = this.cpu.csr.read(CSR_MEPC);
    const regAccess = {
      readRegister: (i: number) => this.cpu!.cpu.registers.read(i),
      writeRegister: (i: number, v: number) => this.cpu!.cpu.registers.write(i, v),
    };
    const memAccess = { readMemoryByte: (a: number) => this.cpu!.cpu.memory.readByte(a) };
    this.kernel.handleSyscall(syscallNum, regAccess, memAccess);

    const currentPCB = this.kernel.getCurrentPCB();
    if (currentPCB && currentPCB.state === ProcessState.Running) {
      this.cpu.cpu.pc = mepc + 4;
    } else {
      const nextPCB = this.kernel.getCurrentPCB();
      if (nextPCB && nextPCB.state === ProcessState.Running) {
        this.cpu.cpu.pc = nextPCB.savedPC;
        this.cpu.cpu.registers.write(REG_SP, nextPCB.stackPointer);
      } else if (this.kernel.processTable.length > 0) {
        this.cpu.cpu.pc = this.kernel.processTable[0].savedPC;
      }
    }
    this.cpu.csr.write(CSR_MSTATUS, (this.cpu.csr.read(CSR_MSTATUS) | MIE) >>> 0);
  }
}

function writeWord(cpu: RiscVSimulator, address: number, value: number): void {
  cpu.cpu.memory.writeByte(address, value & 0xff);
  cpu.cpu.memory.writeByte(address + 1, (value >>> 8) & 0xff);
  cpu.cpu.memory.writeByte(address + 2, (value >>> 16) & 0xff);
  cpu.cpu.memory.writeByte(address + 3, (value >>> 24) & 0xff);
}
