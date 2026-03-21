import { describe, it, expect } from "vitest";
import {
  InterruptDescriptorTable, InterruptController, ISRRegistry,
  saveContext, restoreContext,
  INT_TIMER, INT_KEYBOARD, INT_SYSCALL, IDT_ENTRY_SIZE,
} from "../src/index.js";

describe("InterruptDescriptorTable", () => {
  it("sets and gets entries", () => {
    const idt = new InterruptDescriptorTable();
    idt.setEntry(32, { isrAddress: 0x800, present: true, privilegeLevel: 0 });
    const e = idt.getEntry(32);
    expect(e.isrAddress).toBe(0x800);
    expect(e.present).toBe(true);
  });

  it("write/load memory round-trip", () => {
    const idt = new InterruptDescriptorTable();
    idt.setEntry(1, { isrAddress: 0xdeadbeef, present: true, privilegeLevel: 0 });
    const mem = new Uint8Array(256 * IDT_ENTRY_SIZE);
    idt.writeToMemory(mem, 0);
    const idt2 = new InterruptDescriptorTable();
    idt2.loadFromMemory(mem, 0);
    expect(idt2.getEntry(1).isrAddress).toBe(0xdeadbeef >>> 0);
    expect(idt2.getEntry(1).present).toBe(true);
  });
});

describe("ISRRegistry", () => {
  it("registers and dispatches handlers", () => {
    const reg = new ISRRegistry();
    let called = false;
    reg.register(INT_TIMER, () => { called = true; });
    const frame = { pc: 0, registers: [], mstatus: 0, mcause: INT_TIMER };
    reg.dispatch(INT_TIMER, frame, null);
    expect(called).toBe(true);
  });

  it("throws if no handler registered", () => {
    const reg = new ISRRegistry();
    expect(() => reg.dispatch(99, { pc: 0, registers: [], mstatus: 0, mcause: 0 }, null))
      .toThrow("no ISR handler");
  });
});

describe("InterruptController", () => {
  it("raises and acknowledges interrupts", () => {
    const ic = new InterruptController();
    ic.raiseInterrupt(INT_TIMER);
    expect(ic.pendingCount()).toBe(1);
    expect(ic.hasPending()).toBe(true);
    expect(ic.nextPending()).toBe(INT_TIMER);
    ic.acknowledge(INT_TIMER);
    expect(ic.pendingCount()).toBe(0);
  });

  it("no duplicates in pending", () => {
    const ic = new InterruptController();
    ic.raiseInterrupt(INT_TIMER);
    ic.raiseInterrupt(INT_TIMER);
    expect(ic.pendingCount()).toBe(1);
  });

  it("pending sorted by priority", () => {
    const ic = new InterruptController();
    ic.raiseInterrupt(INT_SYSCALL);
    ic.raiseInterrupt(INT_TIMER);
    expect(ic.nextPending()).toBe(INT_TIMER); // lower number = higher priority
  });

  it("masking blocks interrupts", () => {
    const ic = new InterruptController();
    ic.raiseInterrupt(5);
    ic.setMask(5, true);
    expect(ic.hasPending()).toBe(false);
    ic.setMask(5, false);
    expect(ic.hasPending()).toBe(true);
  });

  it("global disable blocks all", () => {
    const ic = new InterruptController();
    ic.raiseInterrupt(INT_TIMER);
    ic.disable();
    expect(ic.hasPending()).toBe(false);
    ic.enable();
    expect(ic.hasPending()).toBe(true);
  });

  it("clearAll removes all pending", () => {
    const ic = new InterruptController();
    ic.raiseInterrupt(INT_TIMER);
    ic.raiseInterrupt(INT_KEYBOARD);
    ic.clearAll();
    expect(ic.pendingCount()).toBe(0);
  });
});

describe("InterruptFrame", () => {
  it("save/restore context round-trips", () => {
    const regs = Array.from({ length: 32 }, (_, i) => i * 10);
    const frame = saveContext(regs, 0x1000, 0x08, 11);
    const restored = restoreContext(frame);
    expect(restored.pc).toBe(0x1000);
    expect(restored.mstatus).toBe(0x08);
    expect(restored.registers[5]).toBe(50);
  });
});
