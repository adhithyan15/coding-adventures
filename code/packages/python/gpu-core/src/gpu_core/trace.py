"""Execution traces — making every instruction's journey visible.

=== Why Traces? ===

A key principle of this project is educational transparency: every operation
should be observable. When a GPU core executes an instruction, the trace
records exactly what happened:

    Cycle 3 | PC=2 | FFMA R3, R0, R1, R2
    → R3 = R0 * R1 + R2 = 2.0 * 3.0 + 1.0 = 7.0
    → Registers changed: {R3: 7.0}
    → Next PC: 3

This lets a student (or debugger) follow the execution step by step,
understanding not just *what* the GPU did but *why* — which registers were
read, what computation was performed, and what state changed.

=== Trace vs Log ===

A trace is more structured than a log message. Each field is typed and
accessible programmatically, which enables:
- Automated testing (assert trace.registers_changed == {"R3": 7.0})
- Visualization tools (render execution as a timeline)
- Performance analysis (count cycles, track register usage)
"""

from __future__ import annotations

from dataclasses import dataclass, field

from gpu_core.opcodes import Instruction


@dataclass(frozen=True)
class GPUCoreTrace:
    """A record of one instruction's execution on a GPU core.

    Every call to GPUCore.step() returns one of these, providing full
    visibility into what the instruction did.

    Fields:
        cycle:             The clock cycle number (1-indexed).
        pc:                The program counter BEFORE this instruction executed.
        instruction:       The instruction that was executed.
        description:       Human-readable description of what happened.
                          Example: "R3 = R0 * R1 + R2 = 2.0 * 3.0 + 1.0 = 7.0"
        registers_changed: Which registers changed and their new values.
                          Example: {"R3": 7.0}
        memory_changed:    Which memory addresses changed and their new values.
                          Example: {0: 3.14, 4: 2.71}
        next_pc:           The program counter AFTER this instruction.
        halted:            True if this instruction stopped execution.
    """

    cycle: int
    pc: int
    instruction: Instruction
    description: str
    next_pc: int
    halted: bool = False
    registers_changed: dict[str, float] = field(default_factory=dict)
    memory_changed: dict[int, float] = field(default_factory=dict)

    def format(self) -> str:
        """Pretty-print this trace record for educational display.

        Returns a multi-line string like:

            [Cycle 3] PC=2: FFMA R3, R0, R1, R2
              → R3 = R0 * R1 + R2 = 2.0 * 3.0 + 1.0 = 7.0
              → Registers: {R3: 7.0}
              → Next PC: 3
        """
        lines = [f"[Cycle {self.cycle}] PC={self.pc}: {self.instruction!r}"]
        lines.append(f"  -> {self.description}")

        if self.registers_changed:
            regs = ", ".join(
                f"{k}={v}" for k, v in self.registers_changed.items()
            )
            lines.append(f"  -> Registers: {{{regs}}}")

        if self.memory_changed:
            mems = ", ".join(
                f"0x{k:04X}={v}" for k, v in self.memory_changed.items()
            )
            lines.append(f"  -> Memory: {{{mems}}}")

        if self.halted:
            lines.append("  -> HALTED")
        else:
            lines.append(f"  -> Next PC: {self.next_pc}")

        return "\n".join(lines)
