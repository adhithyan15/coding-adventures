"""IrValidator — Intel 4004 hardware-constraint validation for IrPrograms.

Why Validation?
---------------

The Intel 4004 is a 4-bit microprocessor from 1971 — the world's first
commercially available single-chip CPU.  It has severe hardware limits that
a modern language's type checker can't know about:

  - 4 chips × 40 bytes RAM = 160 bytes total addressable RAM
  - A 3-level hardware call stack (push/pop only — no heap of frames)
  - 16 physical registers (R0–R15), grouped into 8 pairs
  - 4-bit arithmetic — immediates > 15 need special pair-load instructions
  - No 16-bit memory operations — LOAD_WORD and STORE_WORD are impossible

The validator answers the question: "Can this IR program run on real 4004
hardware?"  It collects *all* violations and returns them as a list, so the
programmer sees every problem at once rather than fixing them one by one.

Validation Rules
----------------

+------------------+-----------------------------------------------------------+
| Rule             | Constraint                                                |
+==================+===========================================================+
| no_word_ops      | LOAD_WORD and STORE_WORD opcodes are forbidden            |
+------------------+-----------------------------------------------------------+
| static_ram       | Sum of all IrDataDecl sizes ≤ 160 bytes                   |
+------------------+-----------------------------------------------------------+
| call_depth       | Static call-graph DFS depth ≤ 2                           |
|                  | (3-level stack; _start occupies level 1)                  |
+------------------+-----------------------------------------------------------+
| register_count   | Distinct virtual register indices ≤ 12                    |
+------------------+-----------------------------------------------------------+
| operand_range    | Every LOAD_IMM immediate fits in u8 (0–255)               |
+------------------+-----------------------------------------------------------+

These are *accumulated* — we don't stop at the first error.

Call Depth Calculation
----------------------

The 4004 hardware stack has 3 levels.  The main function already occupies
level 1 (pushed when the CPU starts executing ``_start``).  That leaves 2
more levels for explicit CALL instructions.  A call graph like::

    main → foo → bar → baz  (depth 3 from main, 4 total)

would overflow the hardware stack.  The validator does a DFS from each
``LABEL`` entry point and measures the longest call chain.

Register Count Limit
--------------------

Physical registers R0–R15 total 16, but several are reserved:

  - R0  = zero constant (kept 0 by convention)
  - R1  = scratch (not addressable via IR)
  - R12:R13 (P6) = RAM address register (dedicated for SRC)
  - R14:R15 (P7) = scratch pair

That leaves R2–R11 = 10 user-addressable registers, plus we allow up to 12
virtual registers to be safe — the backend maps them to available physical
registers.
"""

from __future__ import annotations

from compiler_ir import (  # noqa: I001
    IrImmediate,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)

# ---------------------------------------------------------------------------
# IrValidationError — describes a single validation failure
# ---------------------------------------------------------------------------
#
# IrValidationError inherits from Exception so it can be both:
#   1. Collected into a list by the validator (list[IrValidationError])
#   2. Raised directly by the backend (raise IrValidationError(...))
#
# This dual role avoids needing two separate error types.  The validator
# returns a list; the backend raises if the list is non-empty.


class IrValidationError(Exception):
    """A single hardware-constraint violation detected by IrValidator.

    Inherits from ``Exception`` so it can be raised by ``Intel4004Backend``
    as well as collected into a list by ``IrValidator``.

    Attributes:
        rule:    Short identifier for the rule that was violated.
                 One of: ``"no_word_ops"``, ``"static_ram"``,
                 ``"call_depth"``, ``"register_count"``, ``"operand_range"``.
        message: Human-readable description of the problem and how to fix it.

    Example::

        e = IrValidationError(rule="static_ram", message="RAM usage 200 > 160 bytes")
        print(e.rule)     # "static_ram"
        print(e.message)  # "RAM usage 200 > 160 bytes"
        raise e           # also valid — it's an Exception
    """

    def __init__(self, rule: str, message: str) -> None:
        """Create an IrValidationError.

        Args:
            rule:    Short rule identifier (e.g., ``"static_ram"``).
            message: Human-readable description of the violation.
        """
        super().__init__(message)
        self.rule = rule
        self.message = message

    def __str__(self) -> str:
        """Return a printable one-line representation."""
        return f"[{self.rule}] {self.message}"

    def __eq__(self, other: object) -> bool:
        """Two IrValidationError objects are equal if rule and message match."""
        if not isinstance(other, IrValidationError):
            return NotImplemented
        return self.rule == other.rule and self.message == other.message

    def __hash__(self) -> int:
        """Hash based on rule and message."""
        return hash((self.rule, self.message))


# ---------------------------------------------------------------------------
# IrValidator — runs all checks, accumulates errors
# ---------------------------------------------------------------------------

# Maximum bytes of static RAM the 4004 can address.
# 4 Intel 4002 chips × 40 bytes each = 160 bytes.
_MAX_RAM_BYTES: int = 160

# Maximum depth of the user-visible call stack.
# The hardware has 3 levels; _start already uses level 1, leaving 2 for
# user calls.
_MAX_CALL_DEPTH: int = 2

# Maximum number of distinct virtual registers allowed.
# Physical registers R2–R11 are user-addressable; we allow 12 vRegs.
_MAX_VIRTUAL_REGISTERS: int = 12

# Maximum value for a LOAD_IMM immediate (u8 range).
_MAX_LOAD_IMM: int = 255
_MIN_LOAD_IMM: int = 0


class IrValidator:
    """Validates an IrProgram against Intel 4004 hardware constraints.

    The validator checks five rules in a single pass (plus a DFS for call
    depth).  All violations are accumulated and returned together so the
    programmer can fix everything at once.

    Usage::

        validator = IrValidator()
        errors = validator.validate(prog)
        if errors:
            for e in errors:
                print(e)
        else:
            print("Program is feasible on Intel 4004 hardware.")
    """

    def validate(self, program: IrProgram) -> list[IrValidationError]:
        """Run all hardware-constraint checks on a program.

        Checks are independent — even if one fails we continue checking the
        others.  This gives the programmer a complete picture.

        Args:
            program: The ``IrProgram`` to validate.

        Returns:
            A list of ``IrValidationError`` objects.  An empty list means
            the program passes all checks and can proceed to code generation.
        """
        errors: list[IrValidationError] = []

        errors.extend(self._check_no_word_ops(program))
        errors.extend(self._check_static_ram(program))
        errors.extend(self._check_call_depth(program))
        errors.extend(self._check_register_count(program))
        errors.extend(self._check_operand_range(program))

        return errors

    # ------------------------------------------------------------------
    # Rule 1: No 16-bit memory operations
    # ------------------------------------------------------------------
    #
    # The 4004 is a 4-bit CPU — it has no native 16-bit memory bus.
    # LOAD_WORD and STORE_WORD would require software emulation that
    # doesn't fit in the 640-byte ROM of a typical 4004 application.
    # If you need 16-bit data, split it into two byte-width operations.

    def _check_no_word_ops(self, program: IrProgram) -> list[IrValidationError]:
        """Ensure no LOAD_WORD or STORE_WORD instructions are present.

        These opcodes require a 16-bit memory bus the 4004 doesn't have.
        Split 16-bit accesses into two 8-bit LOAD_BYTE/STORE_BYTE pairs.

        Returns:
            A list containing at most one error (we report the first
            occurrence of each forbidden opcode).
        """
        errors: list[IrValidationError] = []
        seen_load_word = False
        seen_store_word = False

        for instr in program.instructions:
            if instr.opcode == IrOp.LOAD_WORD and not seen_load_word:
                errors.append(
                    IrValidationError(
                        rule="no_word_ops",
                        message=(
                            "LOAD_WORD is not supported on Intel 4004 — the CPU has "
                            "no 16-bit memory bus.  Replace with two LOAD_BYTE "
                            "instructions (low nibble + high nibble)."
                        ),
                    )
                )
                seen_load_word = True
            elif instr.opcode == IrOp.STORE_WORD and not seen_store_word:
                errors.append(
                    IrValidationError(
                        rule="no_word_ops",
                        message=(
                            "STORE_WORD is not supported on Intel 4004 — the CPU has "
                            "no 16-bit memory bus.  Replace with two STORE_BYTE "
                            "instructions (low nibble + high nibble)."
                        ),
                    )
                )
                seen_store_word = True

        return errors

    # ------------------------------------------------------------------
    # Rule 2: Static RAM usage ≤ 160 bytes
    # ------------------------------------------------------------------
    #
    # The 4004 system used up to four Intel 4002 RAM chips.  Each chip
    # holds 40 bytes (4 banks × 10 characters × 4 bits/char, packed
    # into bytes).  Total: 4 × 40 = 160 bytes.
    #
    # Think of it like a tiny apartment: you only have 160 square feet.
    # If the compiler asks for 200, there's nowhere to put the furniture.

    def _check_static_ram(self, program: IrProgram) -> list[IrValidationError]:
        """Ensure total static data does not exceed 160 bytes.

        Sums all IrDataDecl.size values and compares against the 4×40=160
        byte limit imposed by the 4002 RAM chips.

        Returns:
            A list containing at most one error with the actual usage.
        """
        total = sum(decl.size for decl in program.data)
        if total > _MAX_RAM_BYTES:
            return [
                IrValidationError(
                    rule="static_ram",
                    message=(
                        f"Static RAM usage {total} bytes exceeds the Intel 4004 "
                        f"limit of {_MAX_RAM_BYTES} bytes "
                        f"(4 × Intel 4002 chips × 40 bytes each).  "
                        f"Reduce data declarations by at least "
                        f"{total - _MAX_RAM_BYTES} bytes."
                    ),
                )
            ]
        return []

    # ------------------------------------------------------------------
    # Rule 3: Call graph depth ≤ 2
    # ------------------------------------------------------------------
    #
    # The 4004 hardware call stack has exactly 3 levels.  You can think
    # of it as a stack with 3 slots:
    #
    #   [_start return address]   ← slot 0, always occupied
    #   [first  CALL return addr] ← slot 1 (depth 1)
    #   [second CALL return addr] ← slot 2 (depth 2)  ← maximum
    #
    # A third nested CALL would push a fourth address, overwriting slot 0
    # (the stack wraps!), corrupting the return chain.
    #
    # The check builds a static call graph from CALL instructions and
    # runs a DFS to find the longest call chain.

    def _check_call_depth(self, program: IrProgram) -> list[IrValidationError]:
        """Ensure the static call graph depth does not exceed 2.

        Builds a call graph from LABEL and CALL instructions, then does
        a DFS from every label to find the maximum nesting depth.

        Returns:
            A list containing at most one error with the actual depth.
        """
        # Build a map: label_name → list of callee label names
        call_graph: dict[str, list[str]] = {}
        current_label: str | None = None

        for instr in program.instructions:
            if instr.opcode == IrOp.LABEL:
                # IrLabel operand gives us the label name
                lbl = instr.operands[0]
                if isinstance(lbl, IrLabel):
                    current_label = lbl.name
                    if current_label not in call_graph:
                        call_graph[current_label] = []
            elif instr.opcode == IrOp.CALL:
                # We're inside current_label and we call another function
                callee = instr.operands[0]
                if isinstance(callee, IrLabel) and current_label is not None:
                    call_graph.setdefault(current_label, []).append(callee.name)
                    # Ensure callee appears in the graph even if not yet defined
                    if callee.name not in call_graph:
                        call_graph[callee.name] = []

        def find_cycle() -> list[str] | None:
            """Return one recursive cycle, or None if the graph is acyclic."""
            visiting: set[str] = set()
            visited: set[str] = set()
            path: list[str] = []

            def dfs(node: str) -> list[str] | None:
                if node in visiting:
                    if node in path:
                        start = path.index(node)
                        return path[start:] + [node]
                    return [node, node]
                if node in visited:
                    return None

                visiting.add(node)
                visited.add(node)
                path.append(node)
                for child in call_graph.get(node, []):
                    cycle = dfs(child)
                    if cycle is not None:
                        return cycle
                path.pop()
                visiting.remove(node)
                return None

            for label in call_graph:
                cycle = dfs(label)
                if cycle is not None:
                    return cycle
            return None

        cycle = find_cycle()
        if cycle is not None:
            cycle_str = " -> ".join(cycle)
            return [
                IrValidationError(
                    rule="call_depth",
                    message=(
                        "Recursive call graphs are not supported on the Intel 4004. "
                        f"Found cycle: {cycle_str}. "
                        "Refactor the recursion into an iterative loop or inline helper calls."
                    ),
                )
            ]

        # DFS: find maximum depth reachable from any label in the graph.
        # "depth" here means the number of CALL edges traversed (not
        # counting the implicit _start frame).
        max_depth = 0

        def dfs(node: str, depth: int, visited: set[str]) -> int:
            """Return maximum call depth reachable from node."""
            if node in visited:
                return depth
            visited = visited | {node}
            children = call_graph.get(node, [])
            if not children:
                return depth
            return max(dfs(child, depth + 1, visited) for child in children)

        for label in call_graph:
            d = dfs(label, 0, set())
            if d > max_depth:
                max_depth = d

        if max_depth > _MAX_CALL_DEPTH:
            return [
                IrValidationError(
                    rule="call_depth",
                    message=(
                        f"Call graph depth {max_depth} exceeds the Intel 4004 "
                        f"hardware stack limit of {_MAX_CALL_DEPTH} nested calls "
                        f"(the 3-level stack; _start occupies level 1).  "
                        f"Reduce nesting or inline functions."
                    ),
                )
            ]
        return []

    # ------------------------------------------------------------------
    # Rule 4: Virtual register count ≤ 12
    # ------------------------------------------------------------------
    #
    # The 4004 has 16 physical registers (R0–R15).  Of these:
    #
    #   R0       = zero constant (always 0)
    #   R1       = scratch (used internally by codegen)
    #   R12:R13  = RAM address register (SRC operand)
    #   R14:R15  = scratch pair
    #
    # That leaves R2–R11 = 10 registers, plus a margin to 12 vRegs for
    # the virtual register allocator.  More than 12 distinct vReg indices
    # cannot be mapped without spilling — and the 4004 has no stack for
    # register spilling.

    def _check_register_count(self, program: IrProgram) -> list[IrValidationError]:
        """Ensure no more than 12 distinct virtual register indices are used.

        Collects all IrRegister.index values from all operands across all
        instructions and counts unique values.

        Returns:
            A list containing at most one error with the actual count.
        """
        seen: set[int] = set()
        for instr in program.instructions:
            for operand in instr.operands:
                if isinstance(operand, IrRegister):
                    seen.add(operand.index)

        count = len(seen)
        if count > _MAX_VIRTUAL_REGISTERS:
            return [
                IrValidationError(
                    rule="register_count",
                    message=(
                        f"Program uses {count} distinct virtual registers but "
                        f"Intel 4004 supports at most {_MAX_VIRTUAL_REGISTERS} "
                        f"(R2–R13 are user-addressable; R0, R1, R14, R15 reserved). "
                        f"Reduce virtual register usage by reusing registers."
                    ),
                )
            ]
        return []

    # ------------------------------------------------------------------
    # Rule 5: LOAD_IMM operand fits in u8 (0–255)
    # ------------------------------------------------------------------
    #
    # The 4004 can load a literal value in two ways:
    #
    #   LDM k     — 4-bit immediate (k in 0..15), loads into accumulator
    #   FIM Pn, k — 8-bit immediate (k in 0..255), loads into a register pair
    #
    # The code generator uses FIM for values > 15 and LDM for values ≤ 15.
    # A value > 255 cannot be expressed in a single instruction — it would
    # require multiple instructions and a different IR opcode sequence.

    def _check_operand_range(self, program: IrProgram) -> list[IrValidationError]:
        """Ensure every LOAD_IMM immediate fits in a u8 (0–255).

        Checks all LOAD_IMM instructions for an IrImmediate second operand
        and verifies it is in [0, 255].

        Returns:
            A list of errors, one per out-of-range immediate found.
        """
        errors: list[IrValidationError] = []

        for instr in program.instructions:
            if instr.opcode != IrOp.LOAD_IMM:
                continue
            # LOAD_IMM operands: [dst_reg, immediate_value]
            if len(instr.operands) < 2:
                continue
            operand = instr.operands[1]
            if not isinstance(operand, IrImmediate):
                continue
            val = operand.value
            if val < _MIN_LOAD_IMM or val > _MAX_LOAD_IMM:
                errors.append(
                    IrValidationError(
                        rule="operand_range",
                        message=(
                            f"LOAD_IMM immediate {val} is out of range for "
                            f"Intel 4004.  Valid range is "
                            f"[{_MIN_LOAD_IMM}, {_MAX_LOAD_IMM}] (u8).  "
                            f"Values 0–15 use LDM; 16–255 use FIM pair load.  "
                            f"Values > 255 cannot be loaded in a single instruction."
                        ),
                    )
                )

        return errors
