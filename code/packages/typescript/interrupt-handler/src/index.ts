/**
 * Interrupt Handler -- IDT, ISR registry, interrupt controller, and frame.
 */

// === Interrupt types and well-known numbers ===
export const INT_DIVISION_BY_ZERO = 0;
export const INT_TIMER = 32;
export const INT_KEYBOARD = 33;
export const INT_SYSCALL = 128;

// === IDT ===
export const IDT_ENTRY_SIZE = 8;
export const IDT_SIZE = 256 * IDT_ENTRY_SIZE;
export const IDT_BASE_ADDRESS = 0x00000000;

export interface IDTEntry {
  isrAddress: number;
  present: boolean;
  privilegeLevel: number;
}

export class InterruptDescriptorTable {
  readonly entries: IDTEntry[] = Array.from({ length: 256 }, () => ({
    isrAddress: 0, present: false, privilegeLevel: 0,
  }));

  setEntry(number: number, entry: IDTEntry): void {
    if (number < 0 || number > 255) throw new Error("IDT entry 0-255");
    this.entries[number] = { ...entry };
  }

  getEntry(number: number): IDTEntry {
    if (number < 0 || number > 255) throw new Error("IDT entry 0-255");
    return this.entries[number];
  }

  writeToMemory(memory: Uint8Array, baseAddress: number): void {
    for (let i = 0; i < 256; i++) {
      const offset = baseAddress + i * IDT_ENTRY_SIZE;
      const e = this.entries[i];
      memory[offset] = e.isrAddress & 0xff;
      memory[offset + 1] = (e.isrAddress >>> 8) & 0xff;
      memory[offset + 2] = (e.isrAddress >>> 16) & 0xff;
      memory[offset + 3] = (e.isrAddress >>> 24) & 0xff;
      memory[offset + 4] = e.present ? 1 : 0;
      memory[offset + 5] = e.privilegeLevel;
      memory[offset + 6] = 0;
      memory[offset + 7] = 0;
    }
  }

  loadFromMemory(memory: Uint8Array, baseAddress: number): void {
    for (let i = 0; i < 256; i++) {
      const offset = baseAddress + i * IDT_ENTRY_SIZE;
      this.entries[i] = {
        isrAddress: (memory[offset] | (memory[offset + 1] << 8) | (memory[offset + 2] << 16) | (memory[offset + 3] << 24)) >>> 0,
        present: memory[offset + 4] !== 0,
        privilegeLevel: memory[offset + 5],
      };
    }
  }
}

// === Interrupt Frame ===
export interface InterruptFrame {
  pc: number;
  registers: number[];
  mstatus: number;
  mcause: number;
}

export function saveContext(registers: number[], pc: number, mstatus: number, mcause: number): InterruptFrame {
  return { pc, registers: [...registers], mstatus, mcause };
}

export function restoreContext(frame: InterruptFrame): { registers: number[]; pc: number; mstatus: number } {
  return { registers: [...frame.registers], pc: frame.pc, mstatus: frame.mstatus };
}

// === ISR Registry ===
export type ISRHandler = (frame: InterruptFrame, kernel: unknown) => void;

export class ISRRegistry {
  private handlers = new Map<number, ISRHandler>();

  register(interruptNumber: number, handler: ISRHandler): void {
    this.handlers.set(interruptNumber, handler);
  }

  dispatch(interruptNumber: number, frame: InterruptFrame, kernel: unknown): void {
    const handler = this.handlers.get(interruptNumber);
    if (!handler) throw new Error("no ISR handler registered for interrupt");
    handler(frame, kernel);
  }

  hasHandler(interruptNumber: number): boolean {
    return this.handlers.has(interruptNumber);
  }
}

// === Interrupt Controller ===
export class InterruptController {
  readonly idt = new InterruptDescriptorTable();
  readonly registry = new ISRRegistry();
  pending: number[] = [];
  maskRegister = 0;
  enabled = true;

  raiseInterrupt(number: number): void {
    if (!this.pending.includes(number)) {
      this.pending.push(number);
      this.pending.sort((a, b) => a - b);
    }
  }

  hasPending(): boolean {
    if (!this.enabled) return false;
    return this.pending.some(n => !this.isMasked(n));
  }

  nextPending(): number {
    if (!this.enabled) return -1;
    for (const n of this.pending) {
      if (!this.isMasked(n)) return n;
    }
    return -1;
  }

  acknowledge(number: number): void {
    this.pending = this.pending.filter(n => n !== number);
  }

  setMask(number: number, masked: boolean): void {
    if (number < 0 || number > 31) return;
    if (masked) this.maskRegister |= 1 << number;
    else this.maskRegister &= ~(1 << number);
  }

  isMasked(number: number): boolean {
    if (number < 0 || number > 31) return false;
    return (this.maskRegister & (1 << number)) !== 0;
  }

  enable(): void { this.enabled = true; }
  disable(): void { this.enabled = false; }
  pendingCount(): number { return this.pending.length; }
  clearAll(): void { this.pending = []; }
}
