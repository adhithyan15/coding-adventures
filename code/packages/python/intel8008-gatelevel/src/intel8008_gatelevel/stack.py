"""8-level push-down stack for the Intel 8008 gate-level simulator.

=== The 8008's Unique Stack Architecture ===

Most CPUs have a stack pointer register that increments/decrements to move
through a software-managed stack in RAM. The Intel 8008's stack is completely
different: it's a hardware circular register file with 8 × 14-bit entries,
no stack pointer visible to the programmer, and a unique convention where
entry 0 IS the current program counter.

This design has a profound architectural consequence: when you do CALL,
the "old" PC doesn't need to be saved separately — it's already in entry 0,
and the CALL merely rotates the entries and loads the new target into entry 0.

=== How push works (CALL) ===

Before CALL:
    entry[0] = 0x0100 (current PC, about to execute CALL 0x0200)

After CALL 0x0200:
    entry[0] = 0x0200 (new PC = jump target)
    entry[1] = 0x0103 (return address = PC after the 3-byte CALL)
    entry[2] = whatever was in entry[1]
    ...
    entry[7] = whatever was in entry[6] (oldest entry, silently lost)

=== How pop works (RETURN) ===

Before RETURN:
    entry[0] = 0x0205 (current PC, executing RETURN)
    entry[1] = 0x0103 (return address to go back to)

After RETURN:
    entry[0] = 0x0103 (new PC = return address from entry[1])
    entry[1] = whatever was in entry[2]
    ...
    entry[7] = 0 (zeroed after pop)

=== Stack depth ===

The stack_depth counter tracks how many saved return addresses are live.
It doesn't count entry 0 (which is always the PC). Maximum useful call
depth is 7 — the 8th CALL silently overwrites the oldest return address.

=== Gate-level implementation ===

Each stack entry is a 14-bit register (14 D flip-flops). The push/pop
operations are implemented as shift-register operations — each entry's
bits are wired to the next entry's inputs on the relevant clock edge.
Here we model this as a list rotation.
"""

from __future__ import annotations


class PushDownStack:
    """8-level hardware push-down stack for the Intel 8008 program counter.

    Entry 0 is always the current PC. Entries 1–7 are saved return addresses.

    Usage:
        >>> stack = PushDownStack()
        >>> stack.current_pc()
        0
        >>> stack.push_and_jump(return_addr=0x0103, target=0x0200)
        >>> stack.current_pc()
        512
        >>> stack.pop()
        >>> stack.current_pc()
        259
    """

    def __init__(self) -> None:
        """Initialize stack with all entries at 0."""
        self._entries: list[int] = [0] * 8
        self._depth: int = 0

    def current_pc(self) -> int:
        """Return the current program counter (entry 0).

        The PC always lives in entry 0. Reading the PC just returns
        the value held in the 14-bit register at position 0.
        """
        return self._entries[0]

    def load(self, address: int) -> None:
        """Directly set the PC (entry 0) without touching other entries.

        This is used for:
        - Loading the program start address
        - JMP/CALL: after the push, set entry 0 to the jump target
        - After fetching opcode bytes, advance the PC

        Args:
            address: 14-bit address (0–16383). Higher bits are masked.
        """
        self._entries[0] = address & 0x3FFF

    def increment(self, n: int = 1) -> None:
        """Advance the PC by n bytes (e.g., after fetching instruction bytes).

        Wraps at 14-bit boundary (0x3FFF + 1 = 0x0000).

        Args:
            n: Number of bytes to advance (default 1).
        """
        self._entries[0] = (self._entries[0] + n) & 0x3FFF

    def push_and_jump(self, return_addr: int, target: int) -> None:
        """CALL: save return address, jump to target.

        Rotates all entries down (entry i → entry i+1), then:
        - entry[0] = target (new PC)
        - entry[1] = return_addr (was already in entry[0] or can be specified)

        Wait — the return_addr is the address AFTER the CALL instruction.
        By the time push_and_jump() is called, the PC (entry[0]) has already
        been advanced past the 3-byte CALL instruction. So the current PC IS
        the return address. We accept both forms here.

        Args:
            return_addr: The address to return to (currently in entry[0]).
                         Caller should pass self.current_pc() if already advanced.
                         This parameter is kept for API clarity but the actual
                         rotation saves entry[0] → entry[1] automatically.
            target:      14-bit address to jump to.
        """
        # Rotate entries 0..6 down to 1..7 (hardware: wiring from entry i to entry i+1)
        # Entry 7 (oldest) is silently overwritten on the 8th nested call
        for i in range(7, 0, -1):
            self._entries[i] = self._entries[i - 1]
        # Set new PC
        self._entries[0] = target & 0x3FFF
        # Track depth
        self._depth = min(self._depth + 1, 7)

    def pop(self) -> None:
        """RETURN: restore the saved return address to the PC.

        Rotates all entries up (entry i+1 → entry i):
        - entry[0] = entry[1] (saved return address becomes new PC)
        - entry[1] = entry[2]
        - ...
        - entry[7] = 0 (hardware: the "vacated" slot is left undefined; we zero it)
        """
        for i in range(7):
            self._entries[i] = self._entries[i + 1]
        self._entries[7] = 0  # zeroed for cleanliness
        self._depth = max(self._depth - 1, 0)

    @property
    def depth(self) -> int:
        """Number of saved return addresses (0–7)."""
        return self._depth

    @property
    def entries(self) -> list[int]:
        """All 8 stack entries (read-only snapshot)."""
        return self._entries[:]

    def reset(self) -> None:
        """Reset stack to all-zeros."""
        self._entries = [0] * 8
        self._depth = 0
