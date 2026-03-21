/**
 * Control and Status Register (CSR) file for M-mode.
 *
 * CSRs control CPU behavior at a level above normal computation:
 *   - mstatus:  whether interrupts are enabled
 *   - mtvec:    where to jump when a trap occurs
 *   - mepc:     where to return after handling a trap
 *   - mcause:   what caused the most recent trap
 *   - mscratch: temp storage for trap handlers
 */

// CSR address constants from the RISC-V privileged spec
export const CSR_MSTATUS = 0x300;
export const CSR_MTVEC = 0x305;
export const CSR_MSCRATCH = 0x340;
export const CSR_MEPC = 0x341;
export const CSR_MCAUSE = 0x342;

/** Machine Interrupt Enable bit within mstatus (bit 3). */
export const MIE = 1 << 3;

/** Trap cause: environment call from Machine mode. */
export const CAUSE_ECALL_MMODE = 11;

/**
 * CSRFile holds machine-mode Control and Status Registers.
 * Uses a Map from CSR address to value. Uninitialized CSRs read as 0.
 */
export class CSRFile {
  private regs = new Map<number, number>();

  read(addr: number): number {
    return (this.regs.get(addr) ?? 0) >>> 0;
  }

  write(addr: number, value: number): void {
    this.regs.set(addr, value >>> 0);
  }

  /** Atomically read old value and write new value (CSRRW semantic). */
  readWrite(addr: number, newValue: number): number {
    const old = this.read(addr);
    this.write(addr, newValue);
    return old;
  }

  /** Atomically read old value and set bits (CSRRS semantic). */
  readSet(addr: number, mask: number): number {
    const old = this.read(addr);
    this.write(addr, (old | mask) >>> 0);
    return old;
  }

  /** Atomically read old value and clear bits (CSRRC semantic). */
  readClear(addr: number, mask: number): number {
    const old = this.read(addr);
    this.write(addr, (old & ~mask) >>> 0);
    return old;
  }
}
