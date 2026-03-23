/**
 * # Process Manager Tests
 *
 * These tests verify every aspect of process management: PCB creation,
 * signal handling, fork/exec/wait lifecycle, process exit with reparenting,
 * and priority scheduling.
 *
 * The tests are organized by component, following the same structure as
 * the implementation:
 *
 * 1. PCB creation and defaults
 * 2. Signal delivery, masking, and handling
 * 3. Fork — cloning a process
 * 4. Exec — replacing a program
 * 5. Wait — reaping zombie children
 * 6. Exit — termination and reparenting
 * 7. Kill — sending signals via the ProcessManager
 * 8. Priority scheduler — scheduling and preemption
 */

import { describe, it, expect } from "vitest";
import {
  ProcessState,
  Signal,
  createPCB,
  SignalManager,
  ProcessManager,
  PriorityScheduler,
} from "../src/index.js";

// ============================================================================
// PCB Creation Tests
// ============================================================================

describe("ProcessControlBlock creation", () => {
  it("should create a PCB with correct defaults", () => {
    // Every new process starts in READY state with sensible defaults.
    const pcb = createPCB(1, "test-process");

    expect(pcb.pid).toBe(1);
    expect(pcb.name).toBe("test-process");
    expect(pcb.state).toBe(ProcessState.READY);
    expect(pcb.registers).toHaveLength(32);
    expect(pcb.registers.every((r) => r === 0)).toBe(true);
    expect(pcb.pc).toBe(0);
    expect(pcb.sp).toBe(0);
    expect(pcb.memory_base).toBe(0);
    expect(pcb.memory_size).toBe(0);
    expect(pcb.parent_pid).toBe(0);
    expect(pcb.children).toEqual([]);
    expect(pcb.pending_signals).toEqual([]);
    expect(pcb.signal_handlers.size).toBe(0);
    expect(pcb.signal_mask.size).toBe(0);
    expect(pcb.priority).toBe(20); // Default user priority
    expect(pcb.cpu_time).toBe(0);
    expect(pcb.exit_code).toBe(0);
  });

  it("should accept a custom parent_pid", () => {
    const pcb = createPCB(5, "child", 3);
    expect(pcb.parent_pid).toBe(3);
  });

  it("should have 32 registers initialized to zero", () => {
    // RISC-V has 32 general-purpose registers (x0-x31).
    const pcb = createPCB(0, "init");
    expect(pcb.registers.length).toBe(32);
    for (let i = 0; i < 32; i++) {
      expect(pcb.registers[i]).toBe(0);
    }
  });
});

// ============================================================================
// ProcessState Enum Tests
// ============================================================================

describe("ProcessState enum", () => {
  it("should have correct numeric values", () => {
    expect(ProcessState.READY).toBe(0);
    expect(ProcessState.RUNNING).toBe(1);
    expect(ProcessState.BLOCKED).toBe(2);
    expect(ProcessState.TERMINATED).toBe(3);
    expect(ProcessState.ZOMBIE).toBe(4);
  });
});

// ============================================================================
// Signal Enum Tests
// ============================================================================

describe("Signal enum", () => {
  it("should have correct POSIX signal numbers", () => {
    // These numbers are standardized by POSIX and must be exact.
    expect(Signal.SIGINT).toBe(2);
    expect(Signal.SIGKILL).toBe(9);
    expect(Signal.SIGTERM).toBe(15);
    expect(Signal.SIGCHLD).toBe(17);
    expect(Signal.SIGCONT).toBe(18);
    expect(Signal.SIGSTOP).toBe(19);
  });
});

// ============================================================================
// Signal Manager Tests
// ============================================================================

describe("SignalManager", () => {
  describe("send_signal", () => {
    it("should add catchable signals to pending list", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");

      const enqueued = sm.send_signal(pcb, Signal.SIGTERM);

      expect(enqueued).toBe(true);
      expect(pcb.pending_signals).toEqual([Signal.SIGTERM]);
    });

    it("should handle SIGKILL immediately (no pending)", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");

      const enqueued = sm.send_signal(pcb, Signal.SIGKILL);

      // SIGKILL takes effect immediately — not enqueued.
      expect(enqueued).toBe(false);
      expect(pcb.state).toBe(ProcessState.ZOMBIE);
      expect(pcb.exit_code).toBe(128 + 9); // 137
      expect(pcb.pending_signals).toEqual([]);
    });

    it("should handle SIGSTOP immediately", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");
      pcb.state = ProcessState.RUNNING;

      const enqueued = sm.send_signal(pcb, Signal.SIGSTOP);

      expect(enqueued).toBe(false);
      expect(pcb.state).toBe(ProcessState.BLOCKED);
    });

    it("should handle SIGCONT by resuming a blocked process", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");
      pcb.state = ProcessState.BLOCKED;

      const enqueued = sm.send_signal(pcb, Signal.SIGCONT);

      expect(enqueued).toBe(false);
      expect(pcb.state).toBe(ProcessState.READY);
    });

    it("should not change state for SIGCONT on non-blocked process", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");
      pcb.state = ProcessState.RUNNING;

      sm.send_signal(pcb, Signal.SIGCONT);

      // Process was not blocked, so SIGCONT has no effect on state.
      expect(pcb.state).toBe(ProcessState.RUNNING);
    });

    it("should enqueue SIGINT in pending list", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");

      sm.send_signal(pcb, Signal.SIGINT);

      expect(pcb.pending_signals).toContain(Signal.SIGINT);
    });

    it("should enqueue SIGCHLD in pending list", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");

      sm.send_signal(pcb, Signal.SIGCHLD);

      expect(pcb.pending_signals).toContain(Signal.SIGCHLD);
    });
  });

  describe("deliver_pending", () => {
    it("should deliver signals with default action (terminate)", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");
      pcb.pending_signals = [Signal.SIGTERM];

      const delivered = sm.deliver_pending(pcb);

      // SIGTERM with no handler: default action = terminate.
      expect(delivered).toEqual([{ signal: Signal.SIGTERM, action: "default" }]);
      expect(pcb.state).toBe(ProcessState.ZOMBIE);
      expect(pcb.exit_code).toBe(128 + 15); // 143
      expect(pcb.pending_signals).toEqual([]);
    });

    it("should deliver signals to custom handlers", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");
      pcb.signal_handlers.set(Signal.SIGTERM, 0x1000);
      pcb.pending_signals = [Signal.SIGTERM];

      const delivered = sm.deliver_pending(pcb);

      // Custom handler: process is NOT terminated.
      expect(delivered).toEqual([{ signal: Signal.SIGTERM, action: 0x1000 }]);
      expect(pcb.state).toBe(ProcessState.READY); // Still alive
      expect(pcb.pending_signals).toEqual([]);
    });

    it("should skip masked signals (keep them pending)", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");
      pcb.pending_signals = [Signal.SIGTERM];
      pcb.signal_mask.add(Signal.SIGTERM);

      const delivered = sm.deliver_pending(pcb);

      // Masked signal stays pending.
      expect(delivered).toEqual([]);
      expect(pcb.pending_signals).toEqual([Signal.SIGTERM]);
    });

    it("should deliver unmasked signals and keep masked ones", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");
      pcb.signal_handlers.set(Signal.SIGINT, 0x2000);
      pcb.pending_signals = [Signal.SIGINT, Signal.SIGTERM];
      pcb.signal_mask.add(Signal.SIGTERM);

      const delivered = sm.deliver_pending(pcb);

      // SIGINT is delivered (has handler), SIGTERM stays pending (masked).
      expect(delivered).toEqual([{ signal: Signal.SIGINT, action: 0x2000 }]);
      expect(pcb.pending_signals).toEqual([Signal.SIGTERM]);
    });

    it("should handle SIGCHLD with default action (ignore, not fatal)", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");
      pcb.pending_signals = [Signal.SIGCHLD];

      const delivered = sm.deliver_pending(pcb);

      // SIGCHLD default action is "ignore" — process should NOT be terminated.
      expect(delivered).toEqual([
        { signal: Signal.SIGCHLD, action: "default" },
      ]);
      expect(pcb.state).toBe(ProcessState.READY); // Still alive!
    });

    it("should handle multiple pending signals in order", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");
      pcb.signal_handlers.set(Signal.SIGINT, 0x1000);
      pcb.signal_handlers.set(Signal.SIGTERM, 0x2000);
      pcb.pending_signals = [Signal.SIGINT, Signal.SIGTERM];

      const delivered = sm.deliver_pending(pcb);

      expect(delivered).toHaveLength(2);
      expect(delivered[0]).toEqual({ signal: Signal.SIGINT, action: 0x1000 });
      expect(delivered[1]).toEqual({ signal: Signal.SIGTERM, action: 0x2000 });
      expect(pcb.pending_signals).toEqual([]);
    });
  });

  describe("register_handler", () => {
    it("should register a handler for catchable signals", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");

      const result = sm.register_handler(pcb, Signal.SIGTERM, 0x1000);

      expect(result).toBe(true);
      expect(pcb.signal_handlers.get(Signal.SIGTERM)).toBe(0x1000);
    });

    it("should refuse to register handler for SIGKILL", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");

      const result = sm.register_handler(pcb, Signal.SIGKILL, 0x1000);

      expect(result).toBe(false);
      expect(pcb.signal_handlers.has(Signal.SIGKILL)).toBe(false);
    });

    it("should refuse to register handler for SIGSTOP", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");

      const result = sm.register_handler(pcb, Signal.SIGSTOP, 0x1000);

      expect(result).toBe(false);
      expect(pcb.signal_handlers.has(Signal.SIGSTOP)).toBe(false);
    });

    it("should allow handler for SIGINT", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");

      const result = sm.register_handler(pcb, Signal.SIGINT, 0x3000);

      expect(result).toBe(true);
      expect(pcb.signal_handlers.get(Signal.SIGINT)).toBe(0x3000);
    });
  });

  describe("mask / unmask", () => {
    it("should mask a catchable signal", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");

      const result = sm.mask(pcb, Signal.SIGTERM);

      expect(result).toBe(true);
      expect(pcb.signal_mask.has(Signal.SIGTERM)).toBe(true);
    });

    it("should refuse to mask SIGKILL", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");

      const result = sm.mask(pcb, Signal.SIGKILL);

      expect(result).toBe(false);
      expect(pcb.signal_mask.has(Signal.SIGKILL)).toBe(false);
    });

    it("should refuse to mask SIGSTOP", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");

      const result = sm.mask(pcb, Signal.SIGSTOP);

      expect(result).toBe(false);
      expect(pcb.signal_mask.has(Signal.SIGSTOP)).toBe(false);
    });

    it("should unmask a signal", () => {
      const sm = new SignalManager();
      const pcb = createPCB(1, "test");
      pcb.signal_mask.add(Signal.SIGTERM);

      sm.unmask(pcb, Signal.SIGTERM);

      expect(pcb.signal_mask.has(Signal.SIGTERM)).toBe(false);
    });
  });

  describe("is_fatal", () => {
    it("should return true for SIGINT", () => {
      const sm = new SignalManager();
      expect(sm.is_fatal(Signal.SIGINT)).toBe(true);
    });

    it("should return true for SIGKILL", () => {
      const sm = new SignalManager();
      expect(sm.is_fatal(Signal.SIGKILL)).toBe(true);
    });

    it("should return true for SIGTERM", () => {
      const sm = new SignalManager();
      expect(sm.is_fatal(Signal.SIGTERM)).toBe(true);
    });

    it("should return false for SIGCHLD (default action is ignore)", () => {
      const sm = new SignalManager();
      expect(sm.is_fatal(Signal.SIGCHLD)).toBe(false);
    });

    it("should return false for SIGCONT", () => {
      const sm = new SignalManager();
      expect(sm.is_fatal(Signal.SIGCONT)).toBe(false);
    });

    it("should return true for SIGSTOP", () => {
      const sm = new SignalManager();
      expect(sm.is_fatal(Signal.SIGSTOP)).toBe(true);
    });
  });
});

// ============================================================================
// Process Manager Tests
// ============================================================================

describe("ProcessManager", () => {
  describe("create_process", () => {
    it("should create a process with sequential PIDs", () => {
      const pm = new ProcessManager();
      const p0 = pm.create_process("init");
      const p1 = pm.create_process("shell", 0);
      const p2 = pm.create_process("editor", 1);

      expect(p0.pid).toBe(0);
      expect(p1.pid).toBe(1);
      expect(p2.pid).toBe(2);
    });

    it("should add child to parent's children list", () => {
      const pm = new ProcessManager();
      const init = pm.create_process("init");
      const shell = pm.create_process("shell", 0);

      expect(init.children).toContain(shell.pid);
    });

    it("should set the parent_pid correctly", () => {
      const pm = new ProcessManager();
      pm.create_process("init");
      const shell = pm.create_process("shell", 0);

      expect(shell.parent_pid).toBe(0);
    });
  });

  describe("fork", () => {
    it("should create a child with a new PID", () => {
      const pm = new ProcessManager();
      const parent = pm.create_process("shell");

      const result = pm.fork(parent.pid);

      expect(result).not.toBeNull();
      expect(result!.child_pid).not.toBe(parent.pid);
    });

    it("should set child's parent_pid to parent's PID", () => {
      const pm = new ProcessManager();
      const parent = pm.create_process("shell");

      const result = pm.fork(parent.pid)!;
      const child = pm.get_process(result.child_pid)!;

      expect(child.parent_pid).toBe(parent.pid);
    });

    it("should return child PID to parent and 0 to child", () => {
      const pm = new ProcessManager();
      const parent = pm.create_process("shell");

      const result = pm.fork(parent.pid)!;
      const child = pm.get_process(result.child_pid)!;

      // Parent's register a0 (index 10) = child PID
      expect(parent.registers[10]).toBe(result.child_pid);
      // Child's register a0 = 0
      expect(child.registers[10]).toBe(0);
      // Result object has the parent's return value
      expect(result.parent_result).toBe(result.child_pid);
    });

    it("should add child to parent's children list", () => {
      const pm = new ProcessManager();
      const parent = pm.create_process("shell");

      const result = pm.fork(parent.pid)!;

      expect(parent.children).toContain(result.child_pid);
    });

    it("should set child's state to READY", () => {
      const pm = new ProcessManager();
      const parent = pm.create_process("shell");

      const result = pm.fork(parent.pid)!;
      const child = pm.get_process(result.child_pid)!;

      expect(child.state).toBe(ProcessState.READY);
    });

    it("should reset child's cpu_time to 0", () => {
      const pm = new ProcessManager();
      const parent = pm.create_process("shell");
      parent.cpu_time = 1000;

      const result = pm.fork(parent.pid)!;
      const child = pm.get_process(result.child_pid)!;

      expect(child.cpu_time).toBe(0);
    });

    it("should inherit parent's priority", () => {
      const pm = new ProcessManager();
      const parent = pm.create_process("shell");
      parent.priority = 10;

      const result = pm.fork(parent.pid)!;
      const child = pm.get_process(result.child_pid)!;

      expect(child.priority).toBe(10);
    });

    it("should inherit parent's signal handlers", () => {
      const pm = new ProcessManager();
      const parent = pm.create_process("shell");
      parent.signal_handlers.set(Signal.SIGTERM, 0x5000);

      const result = pm.fork(parent.pid)!;
      const child = pm.get_process(result.child_pid)!;

      expect(child.signal_handlers.get(Signal.SIGTERM)).toBe(0x5000);
    });

    it("should give child an empty children list", () => {
      const pm = new ProcessManager();
      const parent = pm.create_process("shell");

      const result = pm.fork(parent.pid)!;
      const child = pm.get_process(result.child_pid)!;

      expect(child.children).toEqual([]);
    });

    it("should give child an empty pending_signals list", () => {
      const pm = new ProcessManager();
      const parent = pm.create_process("shell");
      parent.pending_signals = [Signal.SIGCHLD];

      const result = pm.fork(parent.pid)!;
      const child = pm.get_process(result.child_pid)!;

      expect(child.pending_signals).toEqual([]);
    });

    it("should return null for non-existent parent", () => {
      const pm = new ProcessManager();
      const result = pm.fork(999);

      expect(result).toBeNull();
    });

    it("should copy parent's PC and SP", () => {
      const pm = new ProcessManager();
      const parent = pm.create_process("shell");
      parent.pc = 0x1000;
      parent.sp = 0x7FFF;

      const result = pm.fork(parent.pid)!;
      const child = pm.get_process(result.child_pid)!;

      expect(child.pc).toBe(0x1000);
      expect(child.sp).toBe(0x7FFF);
    });
  });

  describe("exec", () => {
    it("should reset registers to zero", () => {
      const pm = new ProcessManager();
      const proc = pm.create_process("shell");
      proc.registers[5] = 42;
      proc.registers[10] = 100;

      pm.exec(proc.pid, 0x10000, 0x7FFFF000);

      expect(proc.registers.every((r) => r === 0)).toBe(true);
    });

    it("should set PC to entry point", () => {
      const pm = new ProcessManager();
      const proc = pm.create_process("shell");

      pm.exec(proc.pid, 0x10000, 0x7FFFF000);

      expect(proc.pc).toBe(0x10000);
    });

    it("should set SP to stack top", () => {
      const pm = new ProcessManager();
      const proc = pm.create_process("shell");

      pm.exec(proc.pid, 0x10000, 0x7FFFF000);

      expect(proc.sp).toBe(0x7FFFF000);
    });

    it("should clear signal handlers", () => {
      const pm = new ProcessManager();
      const proc = pm.create_process("shell");
      proc.signal_handlers.set(Signal.SIGTERM, 0x1000);

      pm.exec(proc.pid, 0x10000, 0x7FFFF000);

      expect(proc.signal_handlers.size).toBe(0);
    });

    it("should clear pending signals", () => {
      const pm = new ProcessManager();
      const proc = pm.create_process("shell");
      proc.pending_signals = [Signal.SIGTERM, Signal.SIGCHLD];

      pm.exec(proc.pid, 0x10000, 0x7FFFF000);

      expect(proc.pending_signals).toEqual([]);
    });

    it("should preserve PID", () => {
      const pm = new ProcessManager();
      const proc = pm.create_process("shell");
      const original_pid = proc.pid;

      pm.exec(proc.pid, 0x10000, 0x7FFFF000);

      expect(proc.pid).toBe(original_pid);
    });

    it("should preserve parent_pid", () => {
      const pm = new ProcessManager();
      pm.create_process("init");
      const proc = pm.create_process("shell", 0);

      pm.exec(proc.pid, 0x10000, 0x7FFFF000);

      expect(proc.parent_pid).toBe(0);
    });

    it("should preserve children list", () => {
      const pm = new ProcessManager();
      const parent = pm.create_process("shell");
      pm.fork(parent.pid);

      const children_before = [...parent.children];
      pm.exec(parent.pid, 0x10000, 0x7FFFF000);

      expect(parent.children).toEqual(children_before);
    });

    it("should preserve priority", () => {
      const pm = new ProcessManager();
      const proc = pm.create_process("shell");
      proc.priority = 5;

      pm.exec(proc.pid, 0x10000, 0x7FFFF000);

      expect(proc.priority).toBe(5);
    });

    it("should preserve cpu_time", () => {
      const pm = new ProcessManager();
      const proc = pm.create_process("shell");
      proc.cpu_time = 500;

      pm.exec(proc.pid, 0x10000, 0x7FFFF000);

      expect(proc.cpu_time).toBe(500);
    });

    it("should return false for non-existent PID", () => {
      const pm = new ProcessManager();
      const result = pm.exec(999, 0x10000, 0x7FFFF000);

      expect(result).toBe(false);
    });
  });

  describe("wait", () => {
    it("should return zombie child's PID and exit code", () => {
      const pm = new ProcessManager();
      const parent = pm.create_process("shell");
      const result = pm.fork(parent.pid)!;
      const child = pm.get_process(result.child_pid)!;

      // Child exits with code 42.
      child.state = ProcessState.ZOMBIE;
      child.exit_code = 42;

      const wait_result = pm.wait(parent.pid);

      expect(wait_result).not.toBeNull();
      expect(wait_result!.child_pid).toBe(result.child_pid);
      expect(wait_result!.exit_code).toBe(42);
    });

    it("should remove zombie from process table after reaping", () => {
      const pm = new ProcessManager();
      const parent = pm.create_process("shell");
      const result = pm.fork(parent.pid)!;
      const child = pm.get_process(result.child_pid)!;

      child.state = ProcessState.ZOMBIE;
      child.exit_code = 0;

      pm.wait(parent.pid);

      // The zombie should be completely gone.
      expect(pm.get_process(result.child_pid)).toBeUndefined();
    });

    it("should remove child from parent's children list", () => {
      const pm = new ProcessManager();
      const parent = pm.create_process("shell");
      const result = pm.fork(parent.pid)!;
      const child = pm.get_process(result.child_pid)!;

      child.state = ProcessState.ZOMBIE;
      child.exit_code = 0;

      pm.wait(parent.pid);

      expect(parent.children).not.toContain(result.child_pid);
    });

    it("should return null when no zombie children exist", () => {
      const pm = new ProcessManager();
      const parent = pm.create_process("shell");
      pm.fork(parent.pid); // Child exists but is READY, not ZOMBIE.

      const result = pm.wait(parent.pid);

      expect(result).toBeNull();
    });

    it("should return null when parent has no children", () => {
      const pm = new ProcessManager();
      const parent = pm.create_process("shell");

      const result = pm.wait(parent.pid);

      expect(result).toBeNull();
    });

    it("should return null for non-existent parent", () => {
      const pm = new ProcessManager();
      const result = pm.wait(999);

      expect(result).toBeNull();
    });

    it("should reap only the first zombie when multiple exist", () => {
      const pm = new ProcessManager();
      const parent = pm.create_process("shell");
      const r1 = pm.fork(parent.pid)!;
      const r2 = pm.fork(parent.pid)!;

      const child1 = pm.get_process(r1.child_pid)!;
      const child2 = pm.get_process(r2.child_pid)!;
      child1.state = ProcessState.ZOMBIE;
      child1.exit_code = 1;
      child2.state = ProcessState.ZOMBIE;
      child2.exit_code = 2;

      // First wait reaps the first zombie.
      const result1 = pm.wait(parent.pid);
      expect(result1!.child_pid).toBe(r1.child_pid);
      expect(result1!.exit_code).toBe(1);

      // Second wait reaps the second zombie.
      const result2 = pm.wait(parent.pid);
      expect(result2!.child_pid).toBe(r2.child_pid);
      expect(result2!.exit_code).toBe(2);

      // No more zombies.
      expect(pm.wait(parent.pid)).toBeNull();
    });
  });

  describe("exit_process", () => {
    it("should set process state to ZOMBIE", () => {
      const pm = new ProcessManager();
      const proc = pm.create_process("shell");

      pm.exit_process(proc.pid, 0);

      expect(proc.state).toBe(ProcessState.ZOMBIE);
    });

    it("should set exit_code", () => {
      const pm = new ProcessManager();
      const proc = pm.create_process("shell");

      pm.exit_process(proc.pid, 42);

      expect(proc.exit_code).toBe(42);
    });

    it("should reparent children to init (PID 0)", () => {
      const pm = new ProcessManager();
      const init = pm.create_process("init"); // PID 0
      const parent = pm.create_process("shell", 0); // PID 1
      const fork_result = pm.fork(parent.pid)!;
      const grandchild = pm.get_process(fork_result.child_pid)!;

      pm.exit_process(parent.pid, 0);

      // Grandchild's parent should now be init.
      expect(grandchild.parent_pid).toBe(0);
      // Init should have the grandchild in its children list.
      expect(init.children).toContain(grandchild.pid);
    });

    it("should send SIGCHLD to parent", () => {
      const pm = new ProcessManager();
      const init = pm.create_process("init");
      const child = pm.create_process("shell", 0);

      pm.exit_process(child.pid, 0);

      // Init should have SIGCHLD pending.
      expect(init.pending_signals).toContain(Signal.SIGCHLD);
    });

    it("should clear the process's children list", () => {
      const pm = new ProcessManager();
      pm.create_process("init");
      const parent = pm.create_process("shell", 0);
      pm.fork(parent.pid);

      pm.exit_process(parent.pid, 0);

      expect(parent.children).toEqual([]);
    });

    it("should return false for non-existent process", () => {
      const pm = new ProcessManager();
      const result = pm.exit_process(999, 0);

      expect(result).toBe(false);
    });
  });

  describe("kill", () => {
    it("should send SIGTERM (adds to pending)", () => {
      const pm = new ProcessManager();
      const proc = pm.create_process("test");

      const result = pm.kill(proc.pid, Signal.SIGTERM);

      expect(result).toBe(true);
      expect(proc.pending_signals).toContain(Signal.SIGTERM);
    });

    it("should send SIGKILL (immediate termination)", () => {
      const pm = new ProcessManager();
      pm.create_process("init");
      const proc = pm.create_process("test", 0);

      pm.kill(proc.pid, Signal.SIGKILL);

      expect(proc.state).toBe(ProcessState.ZOMBIE);
    });

    it("should send SIGCHLD to parent when child is killed", () => {
      const pm = new ProcessManager();
      const parent = pm.create_process("shell");
      const fork_result = pm.fork(parent.pid)!;

      pm.kill(fork_result.child_pid, Signal.SIGKILL);

      expect(parent.pending_signals).toContain(Signal.SIGCHLD);
    });

    it("should return false for non-existent target", () => {
      const pm = new ProcessManager();
      const result = pm.kill(999, Signal.SIGTERM);

      expect(result).toBe(false);
    });

    it("should send SIGSTOP to block a process", () => {
      const pm = new ProcessManager();
      const proc = pm.create_process("test");
      proc.state = ProcessState.RUNNING;

      pm.kill(proc.pid, Signal.SIGSTOP);

      expect(proc.state).toBe(ProcessState.BLOCKED);
    });

    it("should send SIGCONT to resume a stopped process", () => {
      const pm = new ProcessManager();
      const proc = pm.create_process("test");
      proc.state = ProcessState.BLOCKED;

      pm.kill(proc.pid, Signal.SIGCONT);

      expect(proc.state).toBe(ProcessState.READY);
    });
  });

  describe("fork + exec + wait lifecycle", () => {
    it("should complete a full fork/exec/wait cycle", () => {
      // This simulates what a shell does when you type a command:
      // 1. Fork a child
      // 2. Child execs the command
      // 3. Parent waits for child to finish
      const pm = new ProcessManager();
      const shell = pm.create_process("shell");

      // Step 1: Fork
      const fork_result = pm.fork(shell.pid)!;
      const child = pm.get_process(fork_result.child_pid)!;

      // Step 2: Exec (child loads "ls" program)
      pm.exec(child.pid, 0x10000, 0x7FFFF000);
      expect(child.pc).toBe(0x10000);
      expect(child.registers.every((r) => r === 0)).toBe(true);

      // Step 3: Child runs and exits
      pm.exit_process(child.pid, 0);
      expect(child.state).toBe(ProcessState.ZOMBIE);

      // Step 4: Parent reaps the child
      const wait_result = pm.wait(shell.pid)!;
      expect(wait_result.child_pid).toBe(fork_result.child_pid);
      expect(wait_result.exit_code).toBe(0);

      // Child is completely gone
      expect(pm.get_process(fork_result.child_pid)).toBeUndefined();
    });
  });
});

// ============================================================================
// Priority Scheduler Tests
// ============================================================================

describe("PriorityScheduler", () => {
  describe("enqueue", () => {
    it("should place process in the correct priority queue", () => {
      const scheduler = new PriorityScheduler();
      const pcb = createPCB(1, "test");
      pcb.priority = 5;

      scheduler.enqueue(pcb);

      const queues = scheduler.get_ready_queues();
      expect(queues[5]).toContain(pcb);
    });

    it("should set process state to READY", () => {
      const scheduler = new PriorityScheduler();
      const pcb = createPCB(1, "test");
      pcb.state = ProcessState.RUNNING;

      scheduler.enqueue(pcb);

      expect(pcb.state).toBe(ProcessState.READY);
    });

    it("should throw for out-of-range priority", () => {
      const scheduler = new PriorityScheduler();
      const pcb = createPCB(1, "test");
      pcb.priority = 40;

      expect(() => scheduler.enqueue(pcb)).toThrow("out of range");
    });

    it("should throw for negative priority", () => {
      const scheduler = new PriorityScheduler();
      const pcb = createPCB(1, "test");
      pcb.priority = -1;

      expect(() => scheduler.enqueue(pcb)).toThrow("out of range");
    });
  });

  describe("schedule", () => {
    it("should pick highest priority (lowest number) process first", () => {
      const scheduler = new PriorityScheduler();
      const low = createPCB(1, "low");
      low.priority = 30;
      const high = createPCB(2, "high");
      high.priority = 5;
      const mid = createPCB(3, "mid");
      mid.priority = 20;

      scheduler.enqueue(low);
      scheduler.enqueue(high);
      scheduler.enqueue(mid);

      const next = scheduler.schedule();

      expect(next).toBe(high);
      expect(next!.state).toBe(ProcessState.RUNNING);
    });

    it("should round-robin within the same priority", () => {
      const scheduler = new PriorityScheduler();
      const a = createPCB(1, "A");
      a.priority = 20;
      const b = createPCB(2, "B");
      b.priority = 20;

      scheduler.enqueue(a);
      scheduler.enqueue(b);

      // First schedule: A (was enqueued first)
      const first = scheduler.schedule();
      expect(first).toBe(a);

      // Second schedule: B
      const second = scheduler.schedule();
      expect(second).toBe(b);
    });

    it("should return null when all queues are empty", () => {
      const scheduler = new PriorityScheduler();
      const result = scheduler.schedule();

      expect(result).toBeNull();
    });

    it("should set current_process", () => {
      const scheduler = new PriorityScheduler();
      const pcb = createPCB(1, "test");
      pcb.priority = 20;
      scheduler.enqueue(pcb);

      scheduler.schedule();

      expect(scheduler.get_current()).toBe(pcb);
    });

    it("should clear current_process when nothing to schedule", () => {
      const scheduler = new PriorityScheduler();

      scheduler.schedule();

      expect(scheduler.get_current()).toBeNull();
    });
  });

  describe("preempt", () => {
    it("should put the process back at the end of its queue", () => {
      const scheduler = new PriorityScheduler();
      const a = createPCB(1, "A");
      a.priority = 20;
      const b = createPCB(2, "B");
      b.priority = 20;

      scheduler.enqueue(a);
      scheduler.enqueue(b);

      // Schedule A.
      const running = scheduler.schedule()!;
      expect(running).toBe(a);

      // Preempt A — it goes to the end of the queue.
      scheduler.preempt(running);

      // Next schedule: B (A was put at the end).
      const next = scheduler.schedule();
      expect(next).toBe(b);

      // After B: A again.
      const after = scheduler.schedule();
      expect(after).toBe(a);
    });

    it("should set process state to READY", () => {
      const scheduler = new PriorityScheduler();
      const pcb = createPCB(1, "test");
      pcb.priority = 20;
      pcb.state = ProcessState.RUNNING;

      scheduler.preempt(pcb);

      expect(pcb.state).toBe(ProcessState.READY);
    });
  });

  describe("set_priority", () => {
    it("should move process to a different priority queue", () => {
      const scheduler = new PriorityScheduler();
      const pcb = createPCB(1, "test");
      pcb.priority = 20;
      scheduler.enqueue(pcb);

      scheduler.set_priority(pcb, 5);

      const queues = scheduler.get_ready_queues();
      expect(queues[20]).not.toContain(pcb);
      expect(queues[5]).toContain(pcb);
      expect(pcb.priority).toBe(5);
    });

    it("should update priority even if not in any queue", () => {
      const scheduler = new PriorityScheduler();
      const pcb = createPCB(1, "test");
      pcb.priority = 20;
      // Not enqueued — maybe it is running or blocked.

      scheduler.set_priority(pcb, 10);

      expect(pcb.priority).toBe(10);
    });

    it("should throw for out-of-range priority", () => {
      const scheduler = new PriorityScheduler();
      const pcb = createPCB(1, "test");

      expect(() => scheduler.set_priority(pcb, 40)).toThrow("out of range");
      expect(() => scheduler.set_priority(pcb, -1)).toThrow("out of range");
    });

    it("should do nothing when new priority equals old priority", () => {
      const scheduler = new PriorityScheduler();
      const pcb = createPCB(1, "test");
      pcb.priority = 20;
      scheduler.enqueue(pcb);

      scheduler.set_priority(pcb, 20);

      const queues = scheduler.get_ready_queues();
      expect(queues[20]).toContain(pcb);
      expect(pcb.priority).toBe(20);
    });
  });

  describe("get_time_quantum", () => {
    it("should return 200 for priority 0 (highest)", () => {
      const scheduler = new PriorityScheduler();
      expect(scheduler.get_time_quantum(0)).toBe(200);
    });

    it("should return 50 for priority 39 (lowest)", () => {
      const scheduler = new PriorityScheduler();
      expect(scheduler.get_time_quantum(39)).toBe(50);
    });

    it("should return intermediate values for middle priorities", () => {
      const scheduler = new PriorityScheduler();
      const q20 = scheduler.get_time_quantum(20);

      // Should be between 50 and 200.
      expect(q20).toBeGreaterThan(50);
      expect(q20).toBeLessThan(200);
    });

    it("should throw for out-of-range priority", () => {
      const scheduler = new PriorityScheduler();
      expect(() => scheduler.get_time_quantum(40)).toThrow("out of range");
      expect(() => scheduler.get_time_quantum(-1)).toThrow("out of range");
    });
  });
});
