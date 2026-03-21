/**
 * OS Kernel -- minimal kernel with process management, scheduler, syscalls.
 */

import { DisplayDriver } from "@coding-adventures/display";
import { InterruptController, type InterruptFrame } from "@coding-adventures/interrupt-handler";
import { encodeAddi, encodeEcall, encodeJal, encodeLui, assemble } from "@coding-adventures/riscv-simulator";

// === Process states ===
export enum ProcessState { Ready, Running, Blocked, Terminated }

// === Well-known addresses ===
export const DEFAULT_KERNEL_BASE = 0x00020000;
export const DEFAULT_IDLE_PROCESS_BASE = 0x00030000;
export const DEFAULT_USER_PROCESS_BASE = 0x00040000;
export const DEFAULT_KERNEL_STACK_TOP = 0x0006fff0;

// === Syscall numbers ===
export const SYS_EXIT = 0;
export const SYS_WRITE = 1;
export const SYS_READ = 2;
export const SYS_YIELD = 3;

// Register indices
export const REG_A0 = 10;
export const REG_A1 = 11;
export const REG_A2 = 12;
export const REG_A7 = 17;
export const REG_SP = 2;

// === Memory Permission ===
export const PERM_READ = 1;
export const PERM_WRITE = 2;
export const PERM_EXECUTE = 4;

export interface MemoryRegion {
  base: number; size: number; permissions: number; owner: number; name: string;
}

// === Memory Manager ===
export class MemoryManager {
  regions: MemoryRegion[];
  constructor(regions: MemoryRegion[]) { this.regions = regions.map(r => ({ ...r })); }
  findRegion(address: number): MemoryRegion | null {
    for (const r of this.regions) { if (address >= r.base && address < r.base + r.size) return r; }
    return null;
  }
  checkAccess(pid: number, address: number, perm: number): boolean {
    const r = this.findRegion(address);
    if (!r) return false;
    if (r.owner !== -1 && r.owner !== pid) return false;
    return (r.permissions & perm) === perm;
  }
  regionCount(): number { return this.regions.length; }
}

// === Process Control Block ===
export interface ProcessControlBlock {
  pid: number; state: ProcessState; savedRegisters: number[];
  savedPC: number; stackPointer: number; memoryBase: number; memorySize: number;
  name: string; exitCode: number;
}

export interface ProcessInfo { pid: number; name: string; state: ProcessState; pc: number; }

// === Scheduler ===
export class Scheduler {
  processTable: ProcessControlBlock[];
  current: number;
  constructor(pt: ProcessControlBlock[]) { this.processTable = pt; this.current = 0; }
  schedule(): number {
    const n = this.processTable.length;
    if (n === 0) return 0;
    for (let i = 1; i <= n; i++) {
      const idx = (this.current + i) % n;
      if (this.processTable[idx].state === ProcessState.Ready) return idx;
    }
    if (this.current < n && this.processTable[this.current].state === ProcessState.Ready) return this.current;
    return 0;
  }
  contextSwitch(from: number, to: number): void {
    if (from >= 0 && from < this.processTable.length && this.processTable[from].state === ProcessState.Running) {
      this.processTable[from].state = ProcessState.Ready;
    }
    if (to >= 0 && to < this.processTable.length) this.processTable[to].state = ProcessState.Running;
    this.current = to;
  }
}

// === Register / Memory access interfaces ===
export interface RegisterAccess { readRegister(index: number): number; writeRegister(index: number, value: number): void; }
export interface MemoryAccess { readMemoryByte(address: number): number; }

// === Syscall Handler type ===
type SyscallHandler = (k: Kernel, regs: RegisterAccess, mem: MemoryAccess) => boolean;

// === Kernel ===
export interface KernelConfig {
  timerInterval: number; maxProcesses: number; memoryLayout: MemoryRegion[];
}

export function defaultKernelConfig(): KernelConfig {
  return {
    timerInterval: 100, maxProcesses: 16,
    memoryLayout: [
      { base: 0, size: 0x1000, permissions: PERM_READ, owner: -1, name: "IDT" },
      { base: 0x1000, size: 0x1000, permissions: PERM_READ | PERM_WRITE, owner: -1, name: "Boot Protocol" },
      { base: DEFAULT_KERNEL_BASE, size: 0x10000, permissions: 7, owner: -1, name: "Kernel" },
      { base: DEFAULT_IDLE_PROCESS_BASE, size: 0x10000, permissions: 7, owner: 0, name: "Idle" },
      { base: DEFAULT_USER_PROCESS_BASE, size: 0x10000, permissions: 7, owner: 1, name: "User" },
      { base: 0x60000, size: 0x10000, permissions: PERM_READ | PERM_WRITE, owner: -1, name: "Kernel Stack" },
    ],
  };
}

export class Kernel {
  config: KernelConfig;
  processTable: ProcessControlBlock[] = [];
  currentProcess = 0;
  scheduler: Scheduler | null = null;
  memoryManager: MemoryManager | null = null;
  interruptCtrl: InterruptController | null;
  display: DisplayDriver | null;
  keyboardBuffer: number[] = [];
  syscallTable: Map<number, SyscallHandler>;
  booted = false;
  private nextPID = 0;

  constructor(config: KernelConfig, interruptCtrl: InterruptController | null, display: DisplayDriver | null) {
    this.config = config;
    this.interruptCtrl = interruptCtrl;
    this.display = display;
    this.syscallTable = new Map<number, SyscallHandler>([
      [SYS_EXIT, handleSysExit],
      [SYS_WRITE, handleSysWrite],
      [SYS_READ, handleSysRead],
      [SYS_YIELD, handleSysYield],
    ]);
  }

  boot(): void {
    this.memoryManager = new MemoryManager(this.config.memoryLayout);
    const idleBin = generateIdleProgram();
    this.createProcess("idle", idleBin, DEFAULT_IDLE_PROCESS_BASE, 0x10000);
    const hwBin = generateHelloWorldProgram(DEFAULT_USER_PROCESS_BASE);
    this.createProcess("hello-world", hwBin, DEFAULT_USER_PROCESS_BASE, 0x10000);
    this.scheduler = new Scheduler(this.processTable);
    if (this.processTable.length > 1) {
      this.processTable[1].state = ProcessState.Running;
      this.currentProcess = 1;
      this.scheduler.current = 1;
    }
    this.booted = true;
  }

  createProcess(name: string, _binary: number[], memBase: number, memSize: number): number {
    if (this.processTable.length >= this.config.maxProcesses) return -1;
    const pid = this.nextPID++;
    const regs = new Array(32).fill(0);
    regs[REG_SP] = memBase + memSize - 16;
    this.processTable.push({
      pid, state: ProcessState.Ready, savedRegisters: regs,
      savedPC: memBase, stackPointer: memBase + memSize - 16,
      memoryBase: memBase, memorySize: memSize, name, exitCode: 0,
    });
    return pid;
  }

  handleSyscall(syscallNum: number, regs: RegisterAccess, mem: MemoryAccess): boolean {
    const handler = this.syscallTable.get(syscallNum);
    if (!handler) {
      const pid = this.currentProcess;
      if (pid >= 0 && pid < this.processTable.length) {
        this.processTable[pid].state = ProcessState.Terminated;
        this.processTable[pid].exitCode = -1;
      }
      return false;
    }
    return handler(this, regs, mem);
  }

  isIdle(): boolean {
    for (const pcb of this.processTable) {
      if (pcb.pid === 0) continue;
      if (pcb.state !== ProcessState.Terminated) return false;
    }
    return true;
  }

  processCount(): number { return this.processTable.length; }
  getCurrentPCB(): ProcessControlBlock | null {
    const pid = this.currentProcess;
    return pid >= 0 && pid < this.processTable.length ? this.processTable[pid] : null;
  }
  addKeystroke(ch: number): void { this.keyboardBuffer.push(ch); }
}

// === Syscall handlers ===
function handleSysExit(k: Kernel, regs: RegisterAccess, _mem: MemoryAccess): boolean {
  const exitCode = regs.readRegister(REG_A0);
  const pid = k.currentProcess;
  if (pid >= 0 && pid < k.processTable.length) {
    k.processTable[pid].state = ProcessState.Terminated;
    k.processTable[pid].exitCode = exitCode;
  }
  if (k.scheduler) {
    const next = k.scheduler.schedule();
    k.scheduler.contextSwitch(pid, next);
    k.currentProcess = next;
  }
  return true;
}

function handleSysWrite(k: Kernel, regs: RegisterAccess, mem: MemoryAccess): boolean {
  const fd = regs.readRegister(REG_A0);
  const bufAddr = regs.readRegister(REG_A1);
  const length = regs.readRegister(REG_A2);
  if (fd !== 1 || !k.display) { regs.writeRegister(REG_A0, 0); return true; }
  let written = 0;
  for (let i = 0; i < length; i++) {
    k.display.putChar(mem.readMemoryByte(bufAddr + i));
    written++;
  }
  regs.writeRegister(REG_A0, written);
  return true;
}

function handleSysRead(k: Kernel, regs: RegisterAccess, _mem: MemoryAccess): boolean {
  const fd = regs.readRegister(REG_A0);
  const length = regs.readRegister(REG_A2);
  if (fd !== 0) { regs.writeRegister(REG_A0, 0); return true; }
  const toRead = Math.min(length, k.keyboardBuffer.length);
  regs.writeRegister(REG_A0, toRead);
  if (toRead > 0) k.keyboardBuffer.splice(0, toRead);
  return true;
}

function handleSysYield(k: Kernel, _regs: RegisterAccess, _mem: MemoryAccess): boolean {
  const pid = k.currentProcess;
  if (pid >= 0 && pid < k.processTable.length && k.processTable[pid].state === ProcessState.Running) {
    k.processTable[pid].state = ProcessState.Ready;
  }
  if (k.scheduler) {
    const next = k.scheduler.schedule();
    k.scheduler.contextSwitch(pid, next);
    k.currentProcess = next;
  }
  return true;
}

// === Program generators ===
export function generateIdleProgram(): number[] {
  return assemble([encodeAddi(REG_A7, 0, SYS_YIELD), encodeEcall(), encodeJal(0, -8)]);
}

export function generateHelloWorldProgram(memBase: number): number[] {
  const dataOffset = 0x100;
  const dataAddr = memBase + dataOffset;
  const message = "Hello World\n";
  const instructions: number[] = [];
  let upper = dataAddr >>> 12;
  const lower = dataAddr & 0xfff;
  if (lower >= 0x800) upper++;
  instructions.push(encodeLui(REG_A1, upper));
  if (lower !== 0) {
    let sl = lower;
    if (sl >= 0x800) sl -= 0x1000;
    instructions.push(encodeAddi(REG_A1, REG_A1, sl));
  }
  instructions.push(encodeAddi(REG_A0, 0, 1)); // fd = 1
  instructions.push(encodeAddi(REG_A2, 0, message.length)); // len
  instructions.push(encodeAddi(REG_A7, 0, SYS_WRITE));
  instructions.push(encodeEcall());
  instructions.push(encodeAddi(REG_A0, 0, 0)); // exit code
  instructions.push(encodeAddi(REG_A7, 0, SYS_EXIT));
  instructions.push(encodeEcall());

  const code = assemble(instructions);
  const binary = new Array(dataOffset + message.length).fill(0);
  for (let i = 0; i < code.length; i++) binary[i] = code[i];
  for (let i = 0; i < message.length; i++) binary[dataOffset + i] = message.charCodeAt(i);
  return binary;
}
