"""IrValidator — Intel 8008 hardware-constraint validation for IrPrograms.

Why Validation?
---------------

The Intel 8008 (1972) is an 8-bit microprocessor — the world's first
single-chip 8-bit CPU.  It targets real hardware that imposes constraints
the Oct type checker can't know about:

  - 8-level hardware call stack (only 7 levels available for user CAL)
  - Only 4 user-data registers: B, C, D, E (accumulator A is scratch)
  - 8-bit immediates only (MVI, ADI, etc. all take 8-bit literals)
  - Input ports 0–7 only (IN p; 3-bit field in the opcode)
  - Output ports 0–23 only (OUT p; 5-bit field in the opcode)
  - SYSCALL numbers restricted to a specific hardware-supported set
  - 8 KB RAM region: at most 8 191 static bytes (addresses 0x2000–0x3FFF)
  - No 16-bit memory bus (LOAD_WORD / STORE_WORD impossible)

The validator answers: "Can this IR program run on real 8008 hardware?"
It collects *all* violations in one pass so the programmer sees every
problem at once rather than fixing them one by one.

Validation Rules
----------------

+--------------------+-----------------------------------------------------------+
| Rule               | Constraint                                                |
+====================+===========================================================+
| no_word_ops        | LOAD_WORD and STORE_WORD opcodes are forbidden            |
+--------------------+-----------------------------------------------------------+
| static_ram         | Sum of all IrDataDecl sizes ≤ 8 191 bytes                 |
+--------------------+-----------------------------------------------------------+
| call_depth         | Static call-graph DFS depth ≤ 7                           |
|                    | (8-level stack; level 0 = current PC, leaving 7 usable)  |
+--------------------+-----------------------------------------------------------+
| register_count     | Distinct virtual register indices ≤ 6                     |
|                    | (v0=zero, v1=scratch, v2–v5=locals/params)                |
+--------------------+-----------------------------------------------------------+
| imm_range          | Every LOAD_IMM and ADD_IMM immediate fits in u8 (0–255)   |
+--------------------+-----------------------------------------------------------+
| syscall_whitelist  | SYSCALL numbers ∈ {3,4} ∪ {11–16} ∪ {20–27} ∪ {40–63}  |
|                    | (adc/sbb, rotations, carry/parity, in/out ports)          |
+--------------------+-----------------------------------------------------------+

All rules are accumulated — we don't stop at the first error.

Call Depth Calculation
----------------------

The 8008 hardware stack has 8 levels.  Level 0 is always the current PC,
which is always occupied.  That leaves 7 levels for CAL instructions.  The
validator does a DFS from every LABEL entry point and measures the longest
call chain (in edges, not nodes)::

    _start → main → foo → bar   (depth 3 from _start; 3 CAL edges)

With 7 available levels, chains up to depth 7 are allowed.  Recursive call
graphs are caught separately and always rejected — the 8008 push-down stack
wraps without any overflow detection.

SYSCALL Whitelist
-----------------

The 8008 intrinsic SYSCALL numbers and their hardware counterparts:

  SYSCALL  3  → adc(a, b)   — ADC r instruction
  SYSCALL  4  → sbb(a, b)   — SBB r instruction
  SYSCALL 11  → rlc(a)      — RLC (rotate A left circular)
  SYSCALL 12  → rrc(a)      — RRC (rotate A right circular)
  SYSCALL 13  → ral(a)      — RAL (rotate A left through carry)
  SYSCALL 14  → rar(a)      — RAR (rotate A right through carry)
  SYSCALL 15  → carry()     — materialise carry flag via ACI 0
  SYSCALL 16  → parity(a)   — materialise parity flag via ORA A + branch
  SYSCALL 20+p → in(p)      — IN p instruction (p ∈ 0–7; 8 input ports)
  SYSCALL 40+p → out(p, v)  — OUT p instruction (p ∈ 0–23; 24 output ports)

Any other SYSCALL number has no hardware instruction to lower it to and is
therefore invalid on the 8008.

Register Count Limit
--------------------

The 8008 has 7 named registers: A, B, C, D, E, H, L.

  A      = accumulator — scratch, managed by the code generator
  H, L   = memory address registers — scratch, used for LOAD_ADDR/LOAD_BYTE/
             STORE_BYTE sequences; not user-addressable
  B, C, D, E = 4 user data registers

The Oct calling convention maps virtual registers to physical registers:

  v0  → B   (constant zero, preloaded at _start)
  v1  → A   (scratch / return value; lives in accumulator)
  v2  → C   (1st local / 1st argument slot)
  v3  → D   (2nd local / 2nd argument slot)
  v4  → E   (3rd local / 3rd argument slot)
  v5  → ??? (4th local / 4th argument slot — see note below)

  Note: With only B, C, D, E available as persistent storage (A is scratch;
  H, L are reserved for addresses), 5 distinct persistent registers exist
  (including v0=B).  The code generator must handle v5 carefully.  The
  validator allows up to 6 distinct virtual register indices (v0–v5) to
  match the Oct calling convention, even though the physical mapping is
  tight.  Programs using v6 or higher cannot be expressed in 8008 assembly
  without register spilling to RAM — which the current code generator does
  not support.
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
# Inherits from Exception so it can be both collected into a list by the
# validator AND raised directly by downstream backend stages.  Two roles,
# one type — avoids a separate "ValidatorResult / error pair" dance.


class IrValidationError(Exception):
    """A single hardware-constraint violation detected by IrValidator.

    Inherits from ``Exception`` so it can be raised by the Intel 8008
    backend as well as collected into a list by ``IrValidator``.

    Attributes:
        rule:    Short identifier for the rule that was violated.
                 One of: ``"no_word_ops"``, ``"static_ram"``,
                 ``"call_depth"``, ``"register_count"``,
                 ``"imm_range"``, ``"syscall_whitelist"``.
        message: Human-readable description of the problem and how to fix it.

    Example::

        e = IrValidationError(rule="syscall_whitelist",
                              message="SYSCALL 99 is not a valid 8008 syscall")
        print(e.rule)     # "syscall_whitelist"
        print(e.message)  # "SYSCALL 99 is not a valid 8008 syscall"
        raise e           # also valid — it's an Exception
    """

    def __init__(self, rule: str, message: str) -> None:
        """Create an IrValidationError.

        Args:
            rule:    Short rule identifier (e.g., ``"syscall_whitelist"``).
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
# Hardware constants
# ---------------------------------------------------------------------------

# Maximum bytes of static RAM.  The 8008 RAM region spans 0x2000–0x3FFF,
# which is 8 192 bytes.  We reserve the last byte (0x3FFF) as a guard, so
# the practical limit is 8 191 static variables.
_MAX_RAM_BYTES: int = 8191

# The 8008 hardware push-down stack has 8 levels.  Level 0 is always the
# current program counter — it is never free.  That leaves 7 levels for
# CAL (call) instructions.  A call chain of depth > 7 would wrap the stack,
# silently corrupting the return address chain.
_MAX_CALL_DEPTH: int = 7

# Maximum number of distinct virtual register indices.
# v0 (zero), v1 (scratch/return), v2–v5 (locals/params) = 6 total.
# Index v6 and above have no physical home without RAM spilling.
_MAX_VIRTUAL_REGISTERS: int = 6

# Immediate value range for LOAD_IMM and ADD_IMM.
# The 8008 encodes immediates in 8-bit fields (MVI, ADI, etc.).
_MIN_IMM: int = 0
_MAX_IMM: int = 255

# SYSCALL numbers that have a valid 8008 hardware lowering.
#
#   3–4    : adc, sbb (ADC r / SBB r)
#   11–14  : rlc, rrc, ral, rar (rotation instructions)
#   15     : carry() (ACI 0 trick)
#   16     : parity() (ORA A + conditional branch)
#   20–27  : in(p) for ports 0–7 (IN p instruction)
#   40–63  : out(p, v) for ports 0–23 (OUT p instruction)
_VALID_SYSCALLS: frozenset[int] = frozenset(
    [3, 4]                        # adc, sbb
    + list(range(11, 17))         # 11=rlc, 12=rrc, 13=ral, 14=rar, 15=carry, 16=parity
    + list(range(20, 28))         # in(0)..in(7)
    + list(range(40, 64))         # out(0)..out(23)
)


# ---------------------------------------------------------------------------
# IrValidator — runs all checks, accumulates errors
# ---------------------------------------------------------------------------


class IrValidator:
    """Validates an IrProgram against Intel 8008 hardware constraints.

    The validator checks six rules in a single pass (plus a DFS for call
    depth).  All violations are accumulated and returned together so the
    programmer can fix everything at once rather than discovering failures
    one at a time.

    Usage::

        validator = IrValidator()
        errors = validator.validate(prog)
        if errors:
            for e in errors:
                print(e)
        else:
            print("Program is feasible on Intel 8008 hardware.")
    """

    def validate(self, program: IrProgram) -> list[IrValidationError]:
        """Run all hardware-constraint checks on a program.

        Checks are independent — a failure in one does not prevent the
        others from running.  This gives the programmer a complete picture
        of everything that must be fixed.

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
        errors.extend(self._check_imm_range(program))
        errors.extend(self._check_syscall_whitelist(program))

        return errors

    # ------------------------------------------------------------------
    # Rule 1: No 16-bit memory operations
    # ------------------------------------------------------------------
    #
    # The 8008 is an 8-bit CPU with an 8-bit data bus.  Every memory access
    # moves exactly one byte via the M pseudo-register (memory at H:L).
    # There is no 16-bit move instruction, no double-byte fetch.
    #
    # LOAD_WORD / STORE_WORD would require two separate byte-wide accesses
    # with manually managed H:L addressing — which is not what these opcodes
    # are designed to represent in the IR.  If 16-bit quantities are needed,
    # split them into two LOAD_BYTE/STORE_BYTE pairs with incremented
    # addresses.

    def _check_no_word_ops(self, program: IrProgram) -> list[IrValidationError]:
        """Ensure no LOAD_WORD or STORE_WORD instructions are present.

        These opcodes require a 16-bit memory bus the 8008 doesn't have.
        The 8008 only supports byte-wide M (memory at H:L) accesses.

        Returns:
            A list of errors, at most one per forbidden opcode type.
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
                            "LOAD_WORD is not supported on Intel 8008 — the CPU has "
                            "an 8-bit data bus and no 16-bit memory instruction.  "
                            "Replace with two LOAD_BYTE instructions at consecutive "
                            "addresses (low byte + high byte)."
                        ),
                    )
                )
                seen_load_word = True
            elif instr.opcode == IrOp.STORE_WORD and not seen_store_word:
                errors.append(
                    IrValidationError(
                        rule="no_word_ops",
                        message=(
                            "STORE_WORD is not supported on Intel 8008 — the CPU has "
                            "an 8-bit data bus and no 16-bit memory instruction.  "
                            "Replace with two STORE_BYTE instructions at consecutive "
                            "addresses (low byte + high byte)."
                        ),
                    )
                )
                seen_store_word = True

        return errors

    # ------------------------------------------------------------------
    # Rule 2: Static RAM usage ≤ 8 191 bytes
    # ------------------------------------------------------------------
    #
    # The 8008 backend maps static variables into the RAM region starting
    # at address 0x2000.  The RAM region runs to 0x3FFF, giving 8 192
    # bytes total.  We reserve the last byte as a guard, so the practical
    # limit is 8 191 bytes.
    #
    # Each Oct `static u8` variable occupies exactly 1 byte, so this limit
    # is equivalent to "at most 8 191 static variables".  Programs declaring
    # more static data than this cannot have their data segment fit in RAM.
    #
    # Analogy: think of the RAM as a 8 KB apartment.  Each `static` is a
    # piece of furniture.  If you try to fit 8 200 pieces in, some will end
    # up outside the apartment (in ROM space, which is read-only — crash).

    def _check_static_ram(self, program: IrProgram) -> list[IrValidationError]:
        """Ensure total static data does not exceed 8 191 bytes.

        Sums all IrDataDecl.size values and compares against the 8 KB
        RAM region limit (0x2000–0x3FFF minus one guard byte).

        Returns:
            A list containing at most one error with the actual usage.
        """
        total = sum(decl.size for decl in program.data)
        if total > _MAX_RAM_BYTES:
            return [
                IrValidationError(
                    rule="static_ram",
                    message=(
                        f"Static RAM usage {total} bytes exceeds the Intel 8008 "
                        f"limit of {_MAX_RAM_BYTES} bytes "
                        f"(RAM region 0x2000–0x3FFE).  "
                        f"Reduce data declarations by at least "
                        f"{total - _MAX_RAM_BYTES} bytes."
                    ),
                )
            ]
        return []

    # ------------------------------------------------------------------
    # Rule 3: Call graph depth ≤ 7
    # ------------------------------------------------------------------
    #
    # The 8008 hardware stack is an 8-slot circular push-down register.
    # Slot 0 always holds the current PC — it is never free.  That leaves
    # 7 slots available for nested CAL (call) instructions.
    #
    # Think of it as an 8-deep stack of plates.  One plate is permanently
    # on the bottom (current PC).  You can stack 7 more plates; an 8th CAL
    # would slide the bottom plate out and corrupt the return chain.
    #
    # The validator builds a static call graph from LABEL and CALL opcodes,
    # then runs a DFS to find the longest chain.  Recursive call graphs are
    # caught first and always rejected — the 8008 has no runtime check for
    # stack overflow.
    #
    # Example of a depth-3 chain (fine, within limit):
    #   _start → main → encrypt → xor_byte
    #
    # Example of a depth-8 chain (rejected):
    #   _start → a → b → c → d → e → f → g → h

    def _check_call_depth(self, program: IrProgram) -> list[IrValidationError]:
        """Ensure the static call graph depth does not exceed 7.

        Builds a call graph from LABEL and CALL instructions, checks for
        cycles (always rejected), then does a DFS to find the maximum
        nesting depth.

        Returns:
            A list containing at most one error describing the violation.
        """
        # ---- Build call graph ----
        # call_graph maps label_name → [callee_name, ...]
        call_graph: dict[str, list[str]] = {}
        current_label: str | None = None

        for instr in program.instructions:
            if instr.opcode == IrOp.LABEL:
                lbl = instr.operands[0]
                if isinstance(lbl, IrLabel):
                    current_label = lbl.name
                    if current_label not in call_graph:
                        call_graph[current_label] = []
            elif instr.opcode == IrOp.CALL:
                callee = instr.operands[0]
                if isinstance(callee, IrLabel) and current_label is not None:
                    call_graph.setdefault(current_label, []).append(callee.name)
                    if callee.name not in call_graph:
                        call_graph[callee.name] = []

        # ---- Cycle detection ----
        # A cycle means recursion, which is impossible on the 8008's
        # fixed-size hardware stack.  Find it before measuring depth,
        # because DFS depth is undefined (infinite) in a cyclic graph.

        def find_cycle() -> list[str] | None:
            """Return one cycle path, or None if the graph is acyclic."""
            visiting: set[str] = set()
            visited: set[str] = set()
            path: list[str] = []

            def dfs_cycle(node: str) -> list[str] | None:
                if node in visiting:
                    # Back-edge found: extract cycle from path
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
                    cycle = dfs_cycle(child)
                    if cycle is not None:
                        return cycle
                path.pop()
                visiting.remove(node)
                return None

            for label in call_graph:
                cycle = dfs_cycle(label)
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
                        "Recursive call graphs are not supported on the Intel 8008 "
                        "— the 8-level hardware stack wraps without overflow detection,"
                        " silently corrupting return addresses.  "
                        f"Found cycle: {cycle_str}.  "
                        "Refactor the recursion into an iterative loop."
                    ),
                )
            ]

        # ---- Maximum depth DFS ----
        # Count call edges (not nodes) from the deepest-reaching label.
        # Depth 0 = a label that makes no calls; depth N = N nested CALs.

        max_depth = 0

        def dfs_depth(node: str, depth: int, visited: set[str]) -> int:
            """Return maximum call depth reachable from node."""
            if node in visited:
                return depth
            visited = visited | {node}
            children = call_graph.get(node, [])
            if not children:
                return depth
            return max(dfs_depth(child, depth + 1, visited) for child in children)

        for label in call_graph:
            d = dfs_depth(label, 0, set())
            if d > max_depth:
                max_depth = d

        if max_depth > _MAX_CALL_DEPTH:
            return [
                IrValidationError(
                    rule="call_depth",
                    message=(
                        f"Call graph depth {max_depth} exceeds the Intel 8008 "
                        f"hardware stack limit of {_MAX_CALL_DEPTH} nested calls "
                        f"(8-level push-down stack; level 0 = current PC).  "
                        f"Reduce nesting by inlining functions or restructuring "
                        f"the call graph."
                    ),
                )
            ]
        return []

    # ------------------------------------------------------------------
    # Rule 4: Virtual register count ≤ 6
    # ------------------------------------------------------------------
    #
    # The 8008 has 7 named registers: A, B, C, D, E, H, L.
    # Of these:
    #
    #   A      = accumulator — scratch, used implicitly by all ALU ops
    #   H, L   = 14-bit memory address register — reserved for LOAD_ADDR /
    #             LOAD_BYTE / STORE_BYTE sequences
    #   B, C, D, E = 4 user data registers, persistent across instructions
    #
    # The Oct calling convention allocates virtual registers as:
    #
    #   v0  = constant zero (stored in a dedicated register)
    #   v1  = scratch / return value (lives in A)
    #   v2  = 1st local / 1st argument (→ C or similar)
    #   v3  = 2nd local / 2nd argument
    #   v4  = 3rd local / 3rd argument
    #   v5  = 4th local / 4th argument
    #
    # That is 6 distinct virtual registers total (v0–v5).  Any virtual
    # register index ≥ 6 cannot be assigned a physical home without
    # spilling to RAM, which the current code generator does not support.
    #
    # A program that triggers live-register save-restore with more than 4
    # locals may generate v6, v7, etc.  This validator catches that case
    # and requires the programmer to refactor (fewer locals, inline helpers).

    def _check_register_count(self, program: IrProgram) -> list[IrValidationError]:
        """Ensure no more than 6 distinct virtual register indices are used.

        Collects all IrRegister.index values from every instruction operand
        and counts unique values.

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
                        f"Intel 8008 supports at most {_MAX_VIRTUAL_REGISTERS} "
                        f"(v0–v5 mapping to B, A, C, D, E and one spare; "
                        f"H and L are reserved for memory addressing).  "
                        f"Reduce local variable count or avoid functions with "
                        f"4 locals that call other functions."
                    ),
                )
            ]
        return []

    # ------------------------------------------------------------------
    # Rule 5: Immediate values in LOAD_IMM / ADD_IMM ∈ [0, 255]
    # ------------------------------------------------------------------
    #
    # All Intel 8008 immediate instructions (MVI, ADI, ACI, SUI, ANI, XRI,
    # ORI, CPI) encode their literal operand in a single byte following the
    # opcode.  This means the immediate must fit in 8 bits: 0–255.
    #
    # LOAD_IMM lowers to MVI Rdst, imm.
    # ADD_IMM  lowers to MOV A, Ra; ADI imm; MOV Rdst, A.
    #
    # A value of 256 would require two bytes — impossible to encode in a
    # single MVI or ADI instruction.  Negative values are also out of range
    # for the unsigned byte field.
    #
    # We check both LOAD_IMM and ADD_IMM, and report every out-of-range
    # occurrence individually so the programmer can fix them all at once.

    def _check_imm_range(self, program: IrProgram) -> list[IrValidationError]:
        """Ensure every LOAD_IMM and ADD_IMM immediate fits in a u8 (0–255).

        Checks both LOAD_IMM and ADD_IMM instructions for IrImmediate
        operands outside [0, 255].

        Returns:
            A list of errors, one per out-of-range immediate found.
        """
        errors: list[IrValidationError] = []
        checked_ops = {IrOp.LOAD_IMM, IrOp.ADD_IMM}

        for instr in program.instructions:
            if instr.opcode not in checked_ops:
                continue
            for operand in instr.operands:
                if not isinstance(operand, IrImmediate):
                    continue
                val = operand.value
                if val < _MIN_IMM or val > _MAX_IMM:
                    op_name = instr.opcode.name
                    errors.append(
                        IrValidationError(
                            rule="imm_range",
                            message=(
                                f"{op_name} immediate {val} is out of range for "
                                f"Intel 8008.  Valid range is [{_MIN_IMM}, {_MAX_IMM}] "
                                "(u8: one byte, matching MVI/ADI instruction format).  "
                                f"Split large values or use a register load sequence."
                            ),
                        )
                    )

        return errors

    # ------------------------------------------------------------------
    # Rule 6: SYSCALL numbers in the 8008 whitelist
    # ------------------------------------------------------------------
    #
    # The Oct IR uses SYSCALL to represent hardware intrinsic operations.
    # Each SYSCALL number maps to a specific inline instruction sequence
    # in the 8008 code generator.  SYSCALL numbers outside the whitelist
    # have no defined 8008 assembly lowering and cannot be compiled.
    #
    # The whitelist (from OCT00 specification):
    #
    #    3  → adc(a, b)  ←  ADC r     (add with carry)
    #    4  → sbb(a, b)  ←  SBB r     (subtract with borrow)
    #   11  → rlc(a)     ←  RLC       (rotate left circular)
    #   12  → rrc(a)     ←  RRC       (rotate right circular)
    #   13  → ral(a)     ←  RAL       (rotate left through carry)
    #   14  → rar(a)     ←  RAR       (rotate right through carry)
    #   15  → carry()    ←  MVI A,0; ACI 0  (materialise carry flag)
    #   16  → parity(a)  ←  ORA A; JFP ...  (materialise parity flag)
    #  20–27 → in(p)     ←  IN p      (input port p, p ∈ 0–7)
    #  40–63 → out(p,v)  ←  OUT p     (output port p, p ∈ 0–23)
    #
    # Any other SYSCALL number is rejected with a clear error message
    # explaining which hardware intrinsic would need to be added to support
    # the requested operation.

    def _check_syscall_whitelist(self, program: IrProgram) -> list[IrValidationError]:
        """Ensure every SYSCALL instruction uses a valid 8008 syscall number.

        Collects all SYSCALL opcodes and checks their immediate operands
        against the hardware whitelist.  Each invalid number is reported
        individually.

        Returns:
            A list of errors, one per invalid SYSCALL number.
        """
        errors: list[IrValidationError] = []
        seen_bad: set[int] = set()  # Report each bad number only once

        for instr in program.instructions:
            if instr.opcode != IrOp.SYSCALL:
                continue
            if not instr.operands:
                continue
            op = instr.operands[0]
            if not isinstance(op, IrImmediate):
                continue
            num = op.value
            if num not in _VALID_SYSCALLS and num not in seen_bad:
                seen_bad.add(num)
                errors.append(
                    IrValidationError(
                        rule="syscall_whitelist",
                        message=(
                            f"SYSCALL {num} is not a valid Intel 8008 intrinsic.  "
                            f"Valid syscall numbers are: "
                            f"3–4 (adc/sbb), 11–16 (rotations/carry/parity), "
                            f"20–27 (in ports 0–7), 40–63 (out ports 0–23).  "
                            f"Check the Oct intrinsic call that produced SYSCALL {num}."
                        ),
                    )
                )

        return errors
