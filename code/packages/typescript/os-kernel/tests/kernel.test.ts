import { describe, it, expect } from "vitest";
import {
  Kernel, defaultKernelConfig, Scheduler, MemoryManager, ProcessState,
  generateIdleProgram, generateHelloWorldProgram,
  SYS_EXIT, SYS_WRITE, SYS_YIELD,
  REG_A0, REG_A1, REG_A2, REG_A7,
  DEFAULT_USER_PROCESS_BASE,
} from "../src/index.js";

describe("MemoryManager", () => {
  it("finds region by address", () => {
    const mm = new MemoryManager([
      { base: 0x1000, size: 0x1000, permissions: 7, owner: -1, name: "test" },
    ]);
    expect(mm.findRegion(0x1500)).not.toBeNull();
    expect(mm.findRegion(0x3000)).toBeNull();
  });

  it("checkAccess respects owner and permissions", () => {
    const mm = new MemoryManager([
      { base: 0, size: 0x1000, permissions: 1, owner: 1, name: "proc1" },
    ]);
    expect(mm.checkAccess(1, 0x500, 1)).toBe(true); // owner matches
    expect(mm.checkAccess(2, 0x500, 1)).toBe(false); // wrong owner
    expect(mm.checkAccess(1, 0x500, 2)).toBe(false); // no write perm
  });
});

describe("Scheduler", () => {
  function makePCB(pid: number, state: ProcessState) {
    return {
      pid, state, savedRegisters: Array(32).fill(0), savedPC: 0,
      stackPointer: 0, memoryBase: 0, memorySize: 0, name: `p${pid}`, exitCode: 0,
    };
  }

  it("round-robin selects next ready process", () => {
    const procs = [makePCB(0, ProcessState.Ready), makePCB(1, ProcessState.Ready)];
    const sched = new Scheduler(procs);
    sched.current = 0;
    expect(sched.schedule()).toBe(1);
  });

  it("falls back to idle when no others ready", () => {
    const procs = [makePCB(0, ProcessState.Ready), makePCB(1, ProcessState.Terminated)];
    const sched = new Scheduler(procs);
    sched.current = 1;
    expect(sched.schedule()).toBe(0);
  });

  it("contextSwitch updates states", () => {
    const procs = [makePCB(0, ProcessState.Ready), makePCB(1, ProcessState.Running)];
    const sched = new Scheduler(procs);
    sched.contextSwitch(1, 0);
    expect(procs[1].state).toBe(ProcessState.Ready);
    expect(procs[0].state).toBe(ProcessState.Running);
  });
});

describe("Kernel", () => {
  it("boot creates idle and hello-world processes", () => {
    const kernel = new Kernel(defaultKernelConfig(), null, null);
    kernel.boot();
    expect(kernel.processCount()).toBe(2);
    expect(kernel.processTable[0].name).toBe("idle");
    expect(kernel.processTable[1].name).toBe("hello-world");
    expect(kernel.booted).toBe(true);
  });

  it("isIdle returns true when all non-idle terminated", () => {
    const kernel = new Kernel(defaultKernelConfig(), null, null);
    kernel.boot();
    kernel.processTable[1].state = ProcessState.Terminated;
    expect(kernel.isIdle()).toBe(true);
  });

  it("isIdle returns false when user process still running", () => {
    const kernel = new Kernel(defaultKernelConfig(), null, null);
    kernel.boot();
    expect(kernel.isIdle()).toBe(false);
  });

  it("handleSyscall dispatches sys_exit", () => {
    const kernel = new Kernel(defaultKernelConfig(), null, null);
    kernel.boot();
    const regs = { readRegister: (i: number) => i === REG_A0 ? 0 : 0, writeRegister: () => {} };
    const mem = { readMemoryByte: () => 0 };
    kernel.handleSyscall(SYS_EXIT, regs, mem);
    expect(kernel.processTable[1].state).toBe(ProcessState.Terminated);
  });
});

describe("Program generators", () => {
  it("idle program generates valid machine code", () => {
    const idle = generateIdleProgram();
    expect(idle.length).toBe(12); // 3 instructions * 4 bytes
  });

  it("hello-world program includes data section", () => {
    const hw = generateHelloWorldProgram(DEFAULT_USER_PROCESS_BASE);
    expect(hw.length).toBeGreaterThan(0x100); // code + data at 0x100
    // Check "Hello World\n" is in the data section
    const str = String.fromCharCode(...hw.slice(0x100, 0x100 + 12));
    expect(str).toBe("Hello World\n");
  });
});
