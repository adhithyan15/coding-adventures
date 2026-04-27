"""CLR IL Simulator — Microsoft's answer to the JVM.

=== What is the CLR? ===

The Common Language Runtime (CLR) is the virtual machine at the heart of
Microsoft's .NET framework, first released in 2002 with .NET 1.0. Just as
the JVM runs Java bytecode, the CLR runs Common Intermediate Language (CIL),
also called MSIL (Microsoft Intermediate Language). C#, F#, VB.NET, and even
PowerShell all compile down to CIL bytecode, which the CLR then executes.

The CLR was Microsoft's response to Sun Microsystems' "write once, run
anywhere" promise with Java. The idea: any .NET language compiles to the same
intermediate format, and the CLR handles JIT compilation to native code on
each platform. With .NET Core (2016) and .NET 5+ (2020), this has become
truly cross-platform — Linux, macOS, and Windows all run the same CIL.

=== CLR vs JVM: Two philosophies of stack machines ===

Both the JVM and CLR are stack-based virtual machines, but they take different
approaches to type information in their instruction sets:

    JVM approach — type in the opcode:
        iconst_1    ← "i" means int32
        iconst_2
        iadd        ← "i" means int32 addition
        istore_0    ← "i" means store an int32

    CLR approach — type inferred from the stack:
        ldc.i4.1    ← push int32 constant 1
        ldc.i4.2    ← push int32 constant 2
        add         ← type inferred! works for int32, int64, float...
        stloc.0     ← store whatever type is on the stack

The JVM has separate opcodes for each type: iadd (int), ladd (long), fadd
(float), dadd (double) — four opcodes for the same operation. The CLR has
just one `add` opcode and infers the type from what's currently on the
evaluation stack.

This makes CLR bytecode more compact and the instruction set smaller, but it
means the runtime must track types on the stack (the JVM can derive types
from the opcodes alone).

=== Short encodings: CLR's optimization trick ===

Both VMs optimize for common cases with short encodings, but the CLR goes
further:

    JVM:  iconst_0 through iconst_5   (6 shortcuts, 0-5)
    CLR:  ldc.i4.0 through ldc.i4.8   (9 shortcuts, 0-8!)

Similarly, both have shortcuts for the first few local variables:

    JVM:  iload_0 through iload_3     (4 shortcuts)
    CLR:  ldloc.0 through ldloc.3     (4 shortcuts, same as JVM)

These short encodings save space because they pack the operand into the
opcode itself — no additional bytes needed. Since most methods use small
constants and few locals, this is a big win in practice.

=== The 0xFE prefix: Two-byte opcodes ===

The CLR has more than 256 instructions, so it uses a prefix byte (0xFE)
to create a second "page" of opcodes. The comparison instructions live
in this extended space:

    ceq  = 0xFE 0x01   (compare equal)
    cgt  = 0xFE 0x02   (compare greater than)
    clt  = 0xFE 0x04   (compare less than)

When the decoder sees 0xFE, it reads the next byte to determine the actual
opcode. This is similar to x86's multi-byte opcode prefixes, but much simpler
— the CLR only has one prefix byte.

Why not just assign single-byte opcodes to ceq/cgt/clt? Because the single-
byte opcode space (0x00-0xFD) was already full with more commonly used
instructions. Comparisons happen less often than arithmetic and loads/stores,
so they get the two-byte encoding.

=== Branch offset convention ===

CLR branch offsets are relative to the NEXT instruction's PC, not the current
one. So if br.s is at PC=10 (consuming 2 bytes: opcode + offset), and the
offset is +3, the target is:

    target = PC_of_next_instruction + offset
    target = (10 + 2) + 3 = 15

This convention means an offset of 0 is a no-op branch (falls through to
the next instruction). The JVM uses the same convention for its branch
offsets, so this is fairly standard for stack-based VMs.

=== Instruction encoding summary ===

    Instruction     Encoding                     Width
    ──────────────  ────────────────────────────  ─────
    nop             0x00                          1 byte
    ldnull          0x01                          1 byte
    ldc.i4.0..8    0x16..0x1E                    1 byte
    ldc.i4.s V      0x1F V                        2 bytes (V = signed int8)
    ldc.i4 V        0x20 V[0..3]                  5 bytes (V = LE int32)
    ldloc.0..3     0x06..0x09                    1 byte
    stloc.0..3     0x0A..0x0D                    1 byte
    ldloc.s N       0x11 N                        2 bytes (N = uint8)
    stloc.s N       0x13 N                        2 bytes (N = uint8)
    add             0x58                          1 byte
    sub             0x59                          1 byte
    mul             0x5A                          1 byte
    div             0x5B                          1 byte
    ret             0x2A                          1 byte
    br.s OFF        0x2B OFF                      2 bytes (OFF = signed int8)
    brfalse.s OFF   0x2C OFF                      2 bytes (OFF = signed int8)
    brtrue.s OFF    0x2D OFF                      2 bytes (OFF = signed int8)
    ceq             0xFE 0x01                     2 bytes
    cgt             0xFE 0x02                     2 bytes
    clt             0xFE 0x04                     2 bytes
"""

from __future__ import annotations

import struct
from dataclasses import dataclass
from enum import IntEnum
from typing import TYPE_CHECKING

from clr_bytecode_disassembler import CLRInstruction, CLRMethodBody

if TYPE_CHECKING:
    from simulator_protocol import ExecutionResult

    from clr_simulator.state import CLRState

# ---------------------------------------------------------------------------
# Opcode definitions — real CLR IL opcode values
# ---------------------------------------------------------------------------
# These values come from the ECMA-335 standard (Common Language Infrastructure).
# Using real values means our bytecode is educational AND accurate — a student
# could look up any opcode in the official spec and find the same hex value.


class CLROpcode(IntEnum):
    """CLR IL opcodes with their real byte values from ECMA-335.

    The CLR instruction set is divided into single-byte opcodes (0x00-0xFD)
    and two-byte opcodes that start with the 0xFE prefix. We represent the
    two-byte opcodes (ceq, cgt, clt) with just their second byte here, since
    the prefix is handled separately during decoding.
    """

    # --- No operation ---
    NOP = 0x00  # Do nothing (used for debugging breakpoints)

    # --- Null ---
    LDNULL = 0x01  # Push a null reference onto the stack

    # --- Load local variable (short forms for slots 0-3) ---
    # These are the most commonly accessed locals, so they get 1-byte opcodes.
    # The generic ldloc.s uses 2 bytes (opcode + index).
    LDLOC_0 = 0x06  # Push the value of local variable 0
    LDLOC_1 = 0x07  # Push the value of local variable 1
    LDLOC_2 = 0x08  # Push the value of local variable 2
    LDLOC_3 = 0x09  # Push the value of local variable 3

    # --- Store local variable (short forms for slots 0-3) ---
    STLOC_0 = 0x0A  # Pop the stack into local variable 0
    STLOC_1 = 0x0B  # Pop the stack into local variable 1
    STLOC_2 = 0x0C  # Pop the stack into local variable 2
    STLOC_3 = 0x0D  # Pop the stack into local variable 3

    # --- Load/store local variable (generic forms) ---
    LDLOC_S = 0x11  # Push local variable N (2 bytes: opcode + uint8 index)
    STLOC_S = 0x13  # Pop into local variable N (2 bytes: opcode + uint8 index)

    # --- Load constant int32 (short forms for 0-8) ---
    # The CLR provides 9 short forms (0-8), more than the JVM's 6 (0-5).
    # These are extremely common in real programs — loop counters, array
    # indices, boolean flags, small arithmetic.
    LDC_I4_0 = 0x16  # Push int32 value 0
    LDC_I4_1 = 0x17  # Push int32 value 1
    LDC_I4_2 = 0x18  # Push int32 value 2
    LDC_I4_3 = 0x19  # Push int32 value 3
    LDC_I4_4 = 0x1A  # Push int32 value 4
    LDC_I4_5 = 0x1B  # Push int32 value 5
    LDC_I4_6 = 0x1C  # Push int32 value 6
    LDC_I4_7 = 0x1D  # Push int32 value 7
    LDC_I4_8 = 0x1E  # Push int32 value 8

    # --- Load constant int32 (general forms) ---
    LDC_I4_S = 0x1F  # Push signed int8 as int32 (2 bytes: opcode + int8)
    LDC_I4 = 0x20  # Push int32 (5 bytes: opcode + LE int32)

    # --- Return ---
    RET = 0x2A  # Return from the current method

    # --- Branch instructions ---
    # These use short (1-byte) signed offsets, relative to the NEXT instruction.
    BR_S = 0x2B  # Unconditional branch (2 bytes: opcode + int8 offset)
    BRFALSE_S = 0x2C  # Branch if zero/false (2 bytes: opcode + int8 offset)
    BRTRUE_S = 0x2D  # Branch if nonzero/true (2 bytes: opcode + int8 offset)

    # --- Arithmetic ---
    # Unlike the JVM (iadd, ladd, fadd, dadd), the CLR has ONE opcode per
    # operation. The type is inferred from the values on the evaluation stack.
    ADD = 0x58  # Pop two values, push their sum
    SUB = 0x59  # Pop two values, push their difference
    MUL = 0x5A  # Pop two values, push their product
    DIV = 0x5B  # Pop two values, push their quotient

    # --- Two-byte opcode prefix ---
    PREFIX_FE = 0xFE  # Signals that the next byte is the real opcode


# ---------------------------------------------------------------------------
# Two-byte opcode second bytes (separated from CLROpcode to avoid IntEnum
# alias conflicts — CEQ's second byte 0x01 would conflict with LDNULL 0x01)
# ---------------------------------------------------------------------------

CEQ_BYTE = 0x01  # ceq: compare equal (second byte after 0xFE prefix)
CGT_BYTE = 0x02  # cgt: compare greater than (second byte after 0xFE prefix)
CLT_BYTE = 0x04  # clt: compare less than (second byte after 0xFE prefix)


# ---------------------------------------------------------------------------
# Mnemonic lookup tables
# ---------------------------------------------------------------------------
# We use these to convert opcodes back to human-readable mnemonics for traces.

_SINGLE_BYTE_MNEMONICS: dict[int, str] = {
    CLROpcode.NOP: "nop",
    CLROpcode.LDNULL: "ldnull",
    CLROpcode.LDLOC_0: "ldloc.0",
    CLROpcode.LDLOC_1: "ldloc.1",
    CLROpcode.LDLOC_2: "ldloc.2",
    CLROpcode.LDLOC_3: "ldloc.3",
    CLROpcode.STLOC_0: "stloc.0",
    CLROpcode.STLOC_1: "stloc.1",
    CLROpcode.STLOC_2: "stloc.2",
    CLROpcode.STLOC_3: "stloc.3",
    CLROpcode.LDLOC_S: "ldloc.s",
    CLROpcode.STLOC_S: "stloc.s",
    CLROpcode.LDC_I4_0: "ldc.i4.0",
    CLROpcode.LDC_I4_1: "ldc.i4.1",
    CLROpcode.LDC_I4_2: "ldc.i4.2",
    CLROpcode.LDC_I4_3: "ldc.i4.3",
    CLROpcode.LDC_I4_4: "ldc.i4.4",
    CLROpcode.LDC_I4_5: "ldc.i4.5",
    CLROpcode.LDC_I4_6: "ldc.i4.6",
    CLROpcode.LDC_I4_7: "ldc.i4.7",
    CLROpcode.LDC_I4_8: "ldc.i4.8",
    CLROpcode.LDC_I4_S: "ldc.i4.s",
    CLROpcode.LDC_I4: "ldc.i4",
    CLROpcode.RET: "ret",
    CLROpcode.BR_S: "br.s",
    CLROpcode.BRFALSE_S: "brfalse.s",
    CLROpcode.BRTRUE_S: "brtrue.s",
    CLROpcode.ADD: "add",
    CLROpcode.SUB: "sub",
    CLROpcode.MUL: "mul",
    CLROpcode.DIV: "div",
}

_TWO_BYTE_MNEMONICS: dict[int, str] = {
    CEQ_BYTE: "ceq",
    CGT_BYTE: "cgt",
    CLT_BYTE: "clt",
}


# ---------------------------------------------------------------------------
# Trace dataclass
# ---------------------------------------------------------------------------


@dataclass
class CLRTrace:
    """A trace of one CLR IL instruction execution.

    Each step through the simulator produces a CLRTrace that captures the
    complete state transition: what the stack looked like before and after,
    what the local variables contained, and a human-readable description
    of what happened.

    This is the CLR equivalent of the WASM simulator's WasmStepTrace.
    The key difference: CLR locals can be None (uninitialized), while WASM
    locals are always initialized to zero.

    Example trace for `add` when the stack was [1, 2]:
        CLRTrace(
            pc=5,
            opcode="add",
            stack_before=[1, 2],
            stack_after=[3],
            locals_snapshot=[None, None, ...],
            description="pop 2 and 1, push 3",
        )
    """

    pc: int  # Program counter where this instruction was located
    opcode: str  # Human-readable mnemonic (e.g., "ldc.i4.1", "add")
    stack_before: list[object | None]  # Evaluation stack snapshot before execution
    stack_after: list[object | None]  # Evaluation stack snapshot after execution
    locals_snapshot: list[object | None]  # Local variable slots after execution
    description: str  # Human-readable description of what happened


# ---------------------------------------------------------------------------
# CLR Simulator
# ---------------------------------------------------------------------------


class CLRSimulator:
    """A simulator for the CLR Intermediate Language (CIL/MSIL).

    This simulator executes real CLR IL bytecode, instruction by instruction,
    producing detailed traces at each step. It implements the core execution
    model of the .NET Common Language Runtime.

    === Design decisions ===

    Like our WASM simulator, this is a standalone implementation rather than
    wrapping the generic CPU class. The CLR's variable-width instruction
    encoding and stack-based execution model are different enough from a
    register-based CPU that a dedicated simulator is cleaner.

    The simulator maintains:
        - stack:    The evaluation stack (CLR's operand stack)
        - locals:   Local variable slots (like registers, but accessed via stack)
        - pc:       Program counter (byte offset into bytecode)
        - bytecode: The raw IL program bytes
        - halted:   Whether execution has finished (via ret)

    === The CLR execution model ===

    Each step follows the same cycle:

        1. FETCH:   Read the byte at PC to get the opcode
        2. DECODE:  Determine the instruction and read any operand bytes
        3. EXECUTE: Perform the operation (push/pop stack, read/write locals)
        4. ADVANCE: Move PC forward by the instruction's byte width

    For two-byte opcodes (prefix 0xFE), step 1-2 reads two bytes for the
    opcode itself, then any additional operand bytes.

    === Example: x = 1 + 2 ===

        >>> sim = CLRSimulator()
        >>> sim.load(assemble_clr(
        ...     encode_ldc_i4(1),     # ldc.i4.1 — push 1
        ...     encode_ldc_i4(2),     # ldc.i4.2 — push 2
        ...     (CLROpcode.ADD,),     # add — pop 2 and 1, push 3
        ...     encode_stloc(0),      # stloc.0 — pop 3, store in local 0
        ...     (CLROpcode.RET,),     # ret — halt
        ... ))
        >>> traces = sim.run()
        >>> sim.locals[0]
        3

    Step-by-step stack evolution:
        Step 0: ldc.i4.1    stack: [] -> [1]
        Step 1: ldc.i4.2    stack: [1] -> [1, 2]
        Step 2: add          stack: [1, 2] -> [3]
        Step 3: stloc.0      stack: [3] -> []       locals[0] = 3
        Step 4: ret           halt
    """

    def __init__(self, host: object | None = None) -> None:
        """Initialize the CLR simulator with empty state.

        Creates:
            - An empty evaluation stack
            - 16 local variable slots (all None / uninitialized)
            - PC at 0
            - No bytecode loaded
        """
        self.stack: list[object | None] = []
        self.locals: list[object | None] = [None] * 16
        self.pc: int = 0
        self.bytecode: bytes = b""
        self.halted: bool = False
        self._host = host
        self._method_body: CLRMethodBody | None = None
        self._instruction_map: dict[int, tuple[int, CLRInstruction]] = {}
        self._instruction_offsets: list[int] = []

    def load(self, bytecode: bytes, num_locals: int = 16) -> None:
        """Load a CLR IL bytecode program into the simulator.

        Resets all simulator state: stack, locals, PC, and halted flag.
        This is equivalent to starting a fresh method execution in the CLR.

        Args:
            bytecode: Raw IL bytecode to execute.
            num_locals: Number of local variable slots to allocate.
                       The CLR method header specifies this; we default to 16.
        """
        self.bytecode = bytecode
        self.stack = []
        self.locals = [None] * num_locals
        self.pc = 0
        self.halted = False
        self._method_body = None
        self._instruction_map = {}
        self._instruction_offsets = []

    def load_method_body(
        self,
        method_body: CLRMethodBody,
        num_locals: int | None = None,
    ) -> None:
        """Load a disassembled CLR method body."""
        self.bytecode = method_body.il_bytes
        self.stack = []
        local_count = num_locals if num_locals is not None else method_body.local_count
        self.locals = [None] * local_count
        self.halted = False
        self._method_body = method_body
        self._instruction_offsets = [
            instruction.offset for instruction in method_body.instructions
        ]
        self._instruction_map = {
            instruction.offset: (index, instruction)
            for index, instruction in enumerate(method_body.instructions)
        }
        self.pc = self._instruction_offsets[0] if self._instruction_offsets else 0

    def step(self) -> CLRTrace:
        """Execute one CLR IL instruction and return a trace.

        This implements the CLR's fetch-decode-execute cycle for a single
        instruction. The trace captures the complete state transition.

        The method handles:
            - Single-byte opcodes (nop, ldc.i4.0, add, ret, etc.)
            - Two-byte opcodes with 0xFE prefix (ceq, cgt, clt)
            - Operand decoding (signed int8 for ldc.i4.s, LE int32 for ldc.i4)
            - Branch offset calculation (relative to NEXT instruction)

        Returns:
            CLRTrace with pc, opcode mnemonic, stack before/after, locals,
            and a human-readable description.

        Raises:
            RuntimeError: If the simulator has already halted.
            ValueError: If an unknown opcode is encountered.
        """
        if self.halted:
            msg = "CLR simulator has halted — no more instructions to execute"
            raise RuntimeError(msg)

        if self._method_body is not None:
            return self._step_disassembled()

        if self.pc >= len(self.bytecode):
            msg = (
                f"PC ({self.pc}) is beyond the end of bytecode "
                f"(length {len(self.bytecode)})"
            )
            raise RuntimeError(msg)

        # Snapshot the stack before execution for the trace
        stack_before = list(self.stack)

        # === FETCH: Read the opcode byte ===
        opcode_byte = self.bytecode[self.pc]

        # === DECODE & EXECUTE ===
        # Dispatch based on the opcode. Each handler reads any operand bytes,
        # performs the operation, advances PC, and returns a trace.

        # --- Two-byte opcodes (0xFE prefix) ---
        if opcode_byte == CLROpcode.PREFIX_FE:
            return self._execute_two_byte_opcode(stack_before)

        # --- NOP: No operation ---
        if opcode_byte == CLROpcode.NOP:
            mnemonic = "nop"
            self.pc += 1
            return CLRTrace(
                pc=self.pc - 1,
                opcode=mnemonic,
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description="no operation",
            )

        # --- LDNULL: Push null ---
        if opcode_byte == CLROpcode.LDNULL:
            original_pc = self.pc
            self.stack.append(None)
            self.pc += 1
            return CLRTrace(
                pc=original_pc,
                opcode="ldnull",
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description="push null",
            )

        # --- LDC.I4.0 through LDC.I4.8: Push small int32 constants ---
        # These are the short-form constant loaders. The value is encoded
        # in the opcode itself — no operand bytes needed.
        if CLROpcode.LDC_I4_0 <= opcode_byte <= CLROpcode.LDC_I4_8:
            value = opcode_byte - CLROpcode.LDC_I4_0
            mnemonic = f"ldc.i4.{value}"
            original_pc = self.pc
            self.stack.append(value)
            self.pc += 1
            return CLRTrace(
                pc=original_pc,
                opcode=mnemonic,
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"push {value}",
            )

        # --- LDC.I4.S: Push signed int8 as int32 ---
        # Two bytes: opcode (0x1F) + signed int8 value.
        # Useful for constants -128 to 127 that don't have a short form.
        if opcode_byte == CLROpcode.LDC_I4_S:
            original_pc = self.pc
            # Read signed int8 operand
            value = struct.unpack_from("b", self.bytecode, self.pc + 1)[0]
            self.stack.append(value)
            self.pc += 2
            return CLRTrace(
                pc=original_pc,
                opcode="ldc.i4.s",
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"push {value}",
            )

        # --- LDC.I4: Push int32 ---
        # Five bytes: opcode (0x20) + little-endian signed int32.
        # The general form for any 32-bit integer constant.
        if opcode_byte == CLROpcode.LDC_I4:
            original_pc = self.pc
            value = struct.unpack_from("<i", self.bytecode, self.pc + 1)[0]
            self.stack.append(value)
            self.pc += 5
            return CLRTrace(
                pc=original_pc,
                opcode="ldc.i4",
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"push {value}",
            )

        # --- LDLOC.0 through LDLOC.3: Load local variable (short form) ---
        if CLROpcode.LDLOC_0 <= opcode_byte <= CLROpcode.LDLOC_3:
            slot = opcode_byte - CLROpcode.LDLOC_0
            mnemonic = f"ldloc.{slot}"
            original_pc = self.pc
            value = self.locals[slot]
            if value is None:
                msg = f"Local variable {slot} is uninitialized"
                raise RuntimeError(msg)
            self.stack.append(value)
            self.pc += 1
            return CLRTrace(
                pc=original_pc,
                opcode=mnemonic,
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"push locals[{slot}] = {value}",
            )

        # --- STLOC.0 through STLOC.3: Store to local variable (short form) ---
        if CLROpcode.STLOC_0 <= opcode_byte <= CLROpcode.STLOC_3:
            slot = opcode_byte - CLROpcode.STLOC_0
            mnemonic = f"stloc.{slot}"
            original_pc = self.pc
            value = self.stack.pop()
            self.locals[slot] = value
            self.pc += 1
            return CLRTrace(
                pc=original_pc,
                opcode=mnemonic,
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"pop {value}, store in locals[{slot}]",
            )

        # --- LDLOC.S: Load local variable (generic form) ---
        # Two bytes: opcode (0x11) + uint8 index.
        if opcode_byte == CLROpcode.LDLOC_S:
            original_pc = self.pc
            slot = self.bytecode[self.pc + 1]
            value = self.locals[slot]
            if value is None:
                msg = f"Local variable {slot} is uninitialized"
                raise RuntimeError(msg)
            self.stack.append(value)
            self.pc += 2
            return CLRTrace(
                pc=original_pc,
                opcode="ldloc.s",
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"push locals[{slot}] = {value}",
            )

        # --- STLOC.S: Store to local variable (generic form) ---
        # Two bytes: opcode (0x13) + uint8 index.
        if opcode_byte == CLROpcode.STLOC_S:
            original_pc = self.pc
            slot = self.bytecode[self.pc + 1]
            value = self.stack.pop()
            self.locals[slot] = value
            self.pc += 2
            return CLRTrace(
                pc=original_pc,
                opcode="stloc.s",
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"pop {value}, store in locals[{slot}]",
            )

        # --- Arithmetic: ADD, SUB, MUL, DIV ---
        # These are type-inferred: one opcode for all numeric types.
        # We pop two values, perform the operation, push the result.
        # Operand order: second-to-top is left, top is right.
        # So "push 5; push 3; sub" computes 5 - 3 = 2 (not 3 - 5).
        if opcode_byte == CLROpcode.ADD:
            return self._execute_arithmetic(stack_before, "add", lambda a, b: a + b)

        if opcode_byte == CLROpcode.SUB:
            return self._execute_arithmetic(stack_before, "sub", lambda a, b: a - b)

        if opcode_byte == CLROpcode.MUL:
            return self._execute_arithmetic(stack_before, "mul", lambda a, b: a * b)

        if opcode_byte == CLROpcode.DIV:
            return self._execute_div(stack_before)

        # --- RET: Return from method ---
        if opcode_byte == CLROpcode.RET:
            original_pc = self.pc
            self.pc += 1
            self.halted = True
            return CLRTrace(
                pc=original_pc,
                opcode="ret",
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description="return",
            )

        # --- BR.S: Unconditional short branch ---
        if opcode_byte == CLROpcode.BR_S:
            return self._execute_branch_s(stack_before, "br.s", branch_always=True)

        # --- BRFALSE.S: Branch if zero/false ---
        if opcode_byte == CLROpcode.BRFALSE_S:
            return self._execute_conditional_branch_s(
                stack_before, "brfalse.s", take_if_zero=True
            )

        # --- BRTRUE.S: Branch if nonzero/true ---
        if opcode_byte == CLROpcode.BRTRUE_S:
            return self._execute_conditional_branch_s(
                stack_before, "brtrue.s", take_if_zero=False
            )

        msg = f"Unknown CLR opcode: 0x{opcode_byte:02X} at PC={self.pc}"
        raise ValueError(msg)

    def run(self, max_steps: int = 10000) -> list[CLRTrace]:
        """Execute until ret instruction, returning all traces.

        Runs the loaded program step by step, collecting a trace at each
        instruction. Stops when the simulator halts (ret instruction) or
        when max_steps is reached (safety limit for infinite loops).

        Args:
            max_steps: Maximum number of instructions to execute.

        Returns:
            List of CLRTrace objects, one per instruction executed.

        Raises:
            RuntimeError: If max_steps is exceeded (likely an infinite loop).
        """
        traces: list[CLRTrace] = []
        for _ in range(max_steps):
            if self.halted:
                break
            traces.append(self.step())
        if not self.halted:
            msg = f"CLR simulator exceeded max_steps ({max_steps})"
            raise RuntimeError(msg)
        return traces

    # -------------------------------------------------------------------
    # simulator-protocol conformance methods
    # -------------------------------------------------------------------

    def get_state(self) -> CLRState:
        """Return a frozen snapshot of the current CLR simulator state.

        All mutable lists are converted to tuples so the result is a true
        immutable value.  The snapshot will not change even if the simulator
        continues executing after this call returns.

        This method satisfies the ``Simulator[CLRState]`` protocol from the
        ``simulator-protocol`` package.

        Returns
        -------
        CLRState:
            Frozen dataclass capturing: evaluation stack, local variable
            slots, program counter, and halted flag.

        Examples
        --------
        >>> sim = CLRSimulator()
        >>> sim.load(assemble_clr((CLROpcode.LDC_I4_1,), (CLROpcode.RET,)))
        >>> sim.run()
        [...]
        >>> state = sim.get_state()
        >>> state.halted
        True
        """
        from clr_simulator.state import CLRState

        return CLRState(
            stack=tuple(self.stack),
            locals=tuple(self.locals),
            pc=self.pc,
            halted=self.halted,
        )

    def execute(
        self,
        program: bytes,
        max_steps: int = 100_000,
    ) -> ExecutionResult[CLRState]:
        """Load program, run to RET or max_steps, return ExecutionResult.

        This is the protocol-conforming entry point for the
        ``Simulator[CLRState]`` protocol defined in the ``simulator-protocol``
        package.  It resets internal state, loads the program, runs the
        execution loop, and returns a rich result type.

        The existing ``load()`` and ``run()`` methods are unchanged — this
        method calls them internally and adapts the return value.

        Parameters
        ----------
        program:
            Raw CLR IL bytecode bytes.
        max_steps:
            Maximum instructions to execute before giving up (default 100,000).

        Returns
        -------
        ExecutionResult[CLRState]:
            - ``halted``: True if RET was reached.
            - ``steps``: total instructions executed.
            - ``final_state``: frozen ``CLRState`` snapshot at termination.
            - ``error``: None on clean halt; error string otherwise.
            - ``traces``: one ``StepTrace`` per instruction executed.

        Examples
        --------
        >>> sim = CLRSimulator()
        >>> from clr_simulator.simulator import (
        ...     CLROpcode,
        ...     assemble_clr,
        ...     encode_ldc_i4,
        ...     encode_stloc,
        ... )
        >>> program = assemble_clr(
        ...     encode_ldc_i4(1),
        ...     encode_ldc_i4(2),
        ...     (CLROpcode.ADD,),
        ...     encode_stloc(0),
        ...     (CLROpcode.RET,),
        ... )
        >>> result = sim.execute(program)
        >>> result.ok
        True
        >>> result.final_state.locals[0]
        3
        """
        from simulator_protocol import ExecutionResult, StepTrace

        # Reset and load
        self.load(program)

        step_traces: list[StepTrace] = []
        steps = 0
        error: str | None = None

        try:
            while not self.halted and steps < max_steps:
                pc_before = self.pc
                clr_trace = self.step()
                step_traces.append(
                    StepTrace(
                        pc_before=pc_before,
                        pc_after=self.pc,
                        mnemonic=clr_trace.opcode,
                        description=clr_trace.description,
                    )
                )
                steps += 1
        except Exception as exc:
            error = str(exc)

        if error is None and not self.halted:
            error = f"max_steps ({max_steps}) exceeded"

        return ExecutionResult(
            halted=self.halted,
            steps=steps,
            final_state=self.get_state(),
            error=error,
            traces=step_traces,
        )

    def reset(self) -> None:
        """Reset all simulator state to initial values.

        Clears the evaluation stack, locals, program counter, bytecode,
        and halted flag.  After ``reset()``, the simulator is in the same
        state as a freshly constructed ``CLRSimulator()``.

        This method satisfies the ``reset()`` requirement of the
        ``Simulator[CLRState]`` protocol.

        Examples
        --------
        >>> sim = CLRSimulator()
        >>> sim.load(bytes([0x17, 0x2A]))  # ldc.i4.1, ret
        >>> sim.run()
        [...]
        >>> sim.halted
        True
        >>> sim.reset()
        >>> sim.halted
        False
        >>> sim.stack
        []
        """
        self.stack = []
        self.locals = [None] * len(self.locals)
        self.pc = 0
        self.bytecode = b""
        self.halted = False
        self._method_body = None
        self._instruction_map = {}
        self._instruction_offsets = []

    def _step_disassembled(self) -> CLRTrace:
        if self.pc not in self._instruction_map:
            msg = f"PC ({self.pc}) is not aligned to a disassembled instruction"
            raise RuntimeError(msg)

        instruction_index, instruction = self._instruction_map[self.pc]
        stack_before = list(self.stack)
        opcode = instruction.opcode

        if opcode == "nop":
            self._advance_to_next_instruction(instruction_index)
            return self._trace(instruction, stack_before, "no operation")

        if opcode == "ldnull":
            self.stack.append(None)
            self._advance_to_next_instruction(instruction_index)
            return self._trace(instruction, stack_before, "push null")

        if opcode.startswith("ldc.i4"):
            self.stack.append(instruction.operand)
            self._advance_to_next_instruction(instruction_index)
            return self._trace(instruction, stack_before, f"push {instruction.operand}")

        if opcode == "ldstr":
            self.stack.append(instruction.operand)
            self._advance_to_next_instruction(instruction_index)
            description = f"push {instruction.operand!r}"
            return self._trace(instruction, stack_before, description)

        if opcode.startswith("ldloc"):
            slot = int(instruction.operand)
            value = self.locals[slot]
            if value is None:
                msg = f"Local variable {slot} is uninitialized"
                raise RuntimeError(msg)
            self.stack.append(value)
            self._advance_to_next_instruction(instruction_index)
            return self._trace(
                instruction,
                stack_before,
                f"push locals[{slot}] = {value}",
            )

        if opcode.startswith("stloc"):
            slot = int(instruction.operand)
            value = self.stack.pop()
            self.locals[slot] = value
            self._advance_to_next_instruction(instruction_index)
            return self._trace(
                instruction,
                stack_before,
                f"pop {value}, store in locals[{slot}]",
            )

        if opcode == "add":
            return self._step_disassembled_arithmetic(
                instruction,
                stack_before,
                lambda a, b: a + b,
            )
        if opcode == "sub":
            return self._step_disassembled_arithmetic(
                instruction,
                stack_before,
                lambda a, b: a - b,
            )
        if opcode == "mul":
            return self._step_disassembled_arithmetic(
                instruction,
                stack_before,
                lambda a, b: a * b,
            )
        if opcode == "div":
            return self._step_disassembled_arithmetic(
                instruction,
                stack_before,
                lambda a, b: a // b,
            )
        if opcode == "ceq":
            return self._step_disassembled_compare(
                instruction,
                stack_before,
                lambda a, b: a == b,
            )
        if opcode == "cgt":
            return self._step_disassembled_compare(
                instruction,
                stack_before,
                lambda a, b: a > b,
            )
        if opcode == "clt":
            return self._step_disassembled_compare(
                instruction,
                stack_before,
                lambda a, b: a < b,
            )

        if opcode == "call":
            return self._step_disassembled_call(
                instruction_index,
                instruction,
                stack_before,
            )

        if opcode in {"br", "br.s"}:
            self.pc = int(instruction.operand)
            return self._trace(
                instruction,
                stack_before,
                f"branch to IL_{self.pc:04x}",
            )

        if opcode in {"brfalse.s", "brtrue.s"}:
            value = self.stack.pop()
            should_branch = bool(value)
            if opcode == "brfalse.s":
                should_branch = not should_branch
            if should_branch:
                self.pc = int(instruction.operand)
                description = f"branch to IL_{self.pc:04x}"
            else:
                self._advance_to_next_instruction(instruction_index)
                description = "fall through"
            return self._trace(instruction, stack_before, description)

        if opcode == "ret":
            self.halted = True
            self._advance_to_next_instruction(instruction_index)
            return self._trace(instruction, stack_before, "return")

        msg = f"Unsupported disassembled CLR opcode {opcode}"
        raise RuntimeError(msg)

    def _advance_to_next_instruction(self, instruction_index: int) -> None:
        next_index = instruction_index + 1
        if next_index >= len(self._instruction_offsets):
            self.pc = len(self.bytecode)
        else:
            self.pc = self._instruction_offsets[next_index]

    def _trace(
        self,
        instruction: CLRInstruction,
        stack_before: list[object | None],
        description: str,
    ) -> CLRTrace:
        return CLRTrace(
            pc=instruction.offset,
            opcode=instruction.opcode,
            stack_before=stack_before,
            stack_after=list(self.stack),
            locals_snapshot=list(self.locals),
            description=description,
        )

    def _step_disassembled_arithmetic(
        self,
        instruction: CLRInstruction,
        stack_before: list[object | None],
        op: object,
    ) -> CLRTrace:
        right = self.stack.pop()
        left = self.stack.pop()
        result = op(left, right)
        self.stack.append(result)
        instruction_index, _ = self._instruction_map[instruction.offset]
        self._advance_to_next_instruction(instruction_index)
        return self._trace(
            instruction,
            stack_before,
            f"pop {right} and {left}, push {result}",
        )

    def _step_disassembled_compare(
        self,
        instruction: CLRInstruction,
        stack_before: list[object | None],
        op: object,
    ) -> CLRTrace:
        right = self.stack.pop()
        left = self.stack.pop()
        result = 1 if op(left, right) else 0
        self.stack.append(result)
        instruction_index, _ = self._instruction_map[instruction.offset]
        self._advance_to_next_instruction(instruction_index)
        return self._trace(
            instruction,
            stack_before,
            f"compare {left} and {right}, push {result}",
        )

    def _step_disassembled_call(
        self,
        instruction_index: int,
        instruction: CLRInstruction,
        stack_before: list[object | None],
    ) -> CLRTrace:
        target = instruction.operand
        if not hasattr(target, "signature"):
            msg = "CLR simulator cannot yet invoke internal method definitions"
            raise RuntimeError(msg)
        argument_count = len(target.signature.parameter_types)
        args = [self.stack.pop() for _ in range(argument_count)]
        args.reverse()
        if self._host is None or not hasattr(self._host, "call_method"):
            msg = (
                "No CLR host available to invoke "
                f"{target.declaring_type}.{target.name}"
            )
            raise RuntimeError(msg)
        result = self._host.call_method(target, args)
        if target.signature.return_type != "void":
            self.stack.append(result)
        self._advance_to_next_instruction(instruction_index)
        return self._trace(
            instruction,
            stack_before,
            f"call {target.declaring_type}.{target.name}",
        )

    # --- Private helper methods ---

    def _execute_arithmetic(
        self,
        stack_before: list[int | None],
        mnemonic: str,
        op: object,  # Callable[[int, int], int] but avoiding ANN type issues
    ) -> CLRTrace:
        """Execute a binary arithmetic instruction (add, sub, mul).

        Pops two values from the stack, applies the operation, and pushes
        the result. The CLR uses type-inferred arithmetic — we only handle
        int32 in this simulator.

        Args:
            stack_before: Stack snapshot before this instruction.
            mnemonic: Human-readable name (e.g., "add").
            op: The arithmetic operation as a callable.
        """
        original_pc = self.pc
        b = self.stack.pop()
        a = self.stack.pop()
        assert isinstance(a, int) and isinstance(b, int)
        result = op(a, b)  # type: ignore[operator]
        self.stack.append(result)
        self.pc += 1
        return CLRTrace(
            pc=original_pc,
            opcode=mnemonic,
            stack_before=stack_before,
            stack_after=list(self.stack),
            locals_snapshot=list(self.locals),
            description=f"pop {b} and {a}, push {result}",
        )

    def _execute_div(self, stack_before: list[int | None]) -> CLRTrace:
        """Execute the div instruction with division-by-zero checking.

        The CLR's div instruction performs integer division (truncating toward
        zero), matching C#'s `/` operator for integers. Division by zero raises
        a System.DivideByZeroException in real .NET; we raise a ZeroDivisionError.
        """
        original_pc = self.pc
        b = self.stack.pop()
        a = self.stack.pop()
        assert isinstance(a, int) and isinstance(b, int)
        if b == 0:
            msg = "System.DivideByZeroException: division by zero"
            raise ZeroDivisionError(msg)
        # CLR integer division truncates toward zero (like C's /)
        # Python's // truncates toward negative infinity, so we use int()
        # to match the CLR behavior.
        result = int(a / b)
        self.stack.append(result)
        self.pc += 1
        return CLRTrace(
            pc=original_pc,
            opcode="div",
            stack_before=stack_before,
            stack_after=list(self.stack),
            locals_snapshot=list(self.locals),
            description=f"pop {b} and {a}, push {result}",
        )

    def _execute_two_byte_opcode(
        self, stack_before: list[int | None]
    ) -> CLRTrace:
        """Decode and execute a two-byte opcode (0xFE prefix).

        The CLR uses the 0xFE prefix to extend the opcode space beyond 256.
        After seeing 0xFE, we read the next byte to determine the actual
        instruction. Currently supported: ceq, cgt, clt.

        These comparison opcodes pop two values and push 1 (true) or 0 (false).
        """
        original_pc = self.pc
        if self.pc + 1 >= len(self.bytecode):
            msg = f"Incomplete two-byte opcode at PC={self.pc}"
            raise ValueError(msg)

        second_byte = self.bytecode[self.pc + 1]

        if second_byte == CEQ_BYTE:
            b = self.stack.pop()
            a = self.stack.pop()
            result = 1 if a == b else 0
            self.stack.append(result)
            self.pc += 2
            return CLRTrace(
                pc=original_pc,
                opcode="ceq",
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"pop {b} and {a}, push {result} ({a} == {b})",
            )

        if second_byte == CGT_BYTE:
            b = self.stack.pop()
            a = self.stack.pop()
            result = 1 if a > b else 0
            self.stack.append(result)
            self.pc += 2
            return CLRTrace(
                pc=original_pc,
                opcode="cgt",
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"pop {b} and {a}, push {result} ({a} > {b})",
            )

        if second_byte == CLT_BYTE:
            b = self.stack.pop()
            a = self.stack.pop()
            result = 1 if a < b else 0
            self.stack.append(result)
            self.pc += 2
            return CLRTrace(
                pc=original_pc,
                opcode="clt",
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"pop {b} and {a}, push {result} ({a} < {b})",
            )

        msg = f"Unknown two-byte opcode: 0xFE 0x{second_byte:02X} at PC={self.pc}"
        raise ValueError(msg)

    def _execute_branch_s(
        self,
        stack_before: list[int | None],
        mnemonic: str,
        branch_always: bool = True,
    ) -> CLRTrace:
        """Execute an unconditional short branch (br.s).

        The offset is a signed int8, relative to the NEXT instruction's PC.
        So br.s at PC=10 with offset=+3 jumps to PC = (10 + 2) + 3 = 15.
        """
        original_pc = self.pc
        offset = struct.unpack_from("b", self.bytecode, self.pc + 1)[0]
        next_pc = self.pc + 2  # PC after this 2-byte instruction
        target = next_pc + offset
        self.pc = target
        return CLRTrace(
            pc=original_pc,
            opcode=mnemonic,
            stack_before=stack_before,
            stack_after=list(self.stack),
            locals_snapshot=list(self.locals),
            description=f"branch to PC={target} (offset {offset:+d})",
        )

    def _execute_conditional_branch_s(
        self,
        stack_before: list[int | None],
        mnemonic: str,
        take_if_zero: bool,
    ) -> CLRTrace:
        """Execute a conditional short branch (brfalse.s or brtrue.s).

        Pops one value from the stack and branches if the condition is met:
            - brfalse.s: branch if value == 0 (false/null)
            - brtrue.s:  branch if value != 0 (true/non-null)

        The offset is relative to the NEXT instruction's PC, same as br.s.
        """
        original_pc = self.pc
        offset = struct.unpack_from("b", self.bytecode, self.pc + 1)[0]
        next_pc = self.pc + 2  # PC after this 2-byte instruction
        target = next_pc + offset

        value = self.stack.pop()
        # For null values, treat as 0 (false)
        numeric_value = 0 if value is None else value

        should_branch = (numeric_value == 0) if take_if_zero else (numeric_value != 0)

        if should_branch:
            self.pc = target
            return CLRTrace(
                pc=original_pc,
                opcode=mnemonic,
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"pop {value}, branch taken to PC={target}",
            )
        else:
            self.pc = next_pc
            return CLRTrace(
                pc=original_pc,
                opcode=mnemonic,
                stack_before=stack_before,
                stack_after=list(self.stack),
                locals_snapshot=list(self.locals),
                description=f"pop {value}, branch not taken",
            )


# ---------------------------------------------------------------------------
# Helper functions — a mini CLR IL assembler
# ---------------------------------------------------------------------------
# These functions make it easy to construct CLR IL bytecode in tests.
# Instead of manually specifying hex bytes, you describe instructions as
# tuples and let assemble_clr() concatenate them into raw bytecode.


def assemble_clr(*instructions: tuple[int, ...] | bytes) -> bytes:
    """Assemble CLR IL from instruction tuples or raw bytes.

    Each instruction is either:
        - A tuple of ints: each int becomes one byte
        - A bytes object: used as-is (from encode_* helpers)

    Examples:
        >>> # Using tuples directly:
        >>> assemble_clr((CLROpcode.LDC_I4_1,), (CLROpcode.RET,))
        b'\\x17\\x2a'

        >>> # Using encode helpers:
        >>> assemble_clr(encode_ldc_i4(42), (CLROpcode.RET,))

        >>> # Mixing both:
        >>> assemble_clr(
        ...     encode_ldc_i4(1),
        ...     encode_ldc_i4(2),
        ...     (CLROpcode.ADD,),
        ...     encode_stloc(0),
        ...     (CLROpcode.RET,),
        ... )
    """
    result = bytearray()
    for instr in instructions:
        if isinstance(instr, bytes):
            result.extend(instr)
        else:
            result.extend(bytes(instr))
    return bytes(result)


def encode_ldc_i4(n: int) -> bytes:
    """Encode pushing an int32 constant, picking the optimal encoding.

    The CLR has three ways to push an integer constant:

        1. ldc.i4.N (1 byte)   — for values 0 through 8
        2. ldc.i4.s V (2 bytes) — for values -128 through 127
        3. ldc.i4 V (5 bytes)  — for any 32-bit integer

    This function automatically picks the smallest encoding, just like
    a real CLR compiler would.

    Examples:
        >>> encode_ldc_i4(0)    # Uses ldc.i4.0 (1 byte)
        b'\\x16'
        >>> encode_ldc_i4(5)    # Uses ldc.i4.5 (1 byte)
        b'\\x1b'
        >>> encode_ldc_i4(42)   # Uses ldc.i4.s 42 (2 bytes)
        b'\\x1f\\x2a'
        >>> encode_ldc_i4(1000) # Uses ldc.i4 1000 (5 bytes)
        b'\\x20\\xe8\\x03\\x00\\x00'
    """
    # Short forms: 0 through 8 → single-byte opcodes
    if 0 <= n <= 8:
        return bytes([CLROpcode.LDC_I4_0 + n])

    # Medium form: -128 through 127 → 2-byte opcode + signed int8
    if -128 <= n <= 127:
        return bytes([CLROpcode.LDC_I4_S]) + struct.pack("b", n)

    # General form: any 32-bit integer → 5-byte opcode + LE int32
    return bytes([CLROpcode.LDC_I4]) + struct.pack("<i", n)


def encode_stloc(slot: int) -> bytes:
    """Encode storing to a local variable slot.

    Uses the short form (stloc.0 through stloc.3) when possible,
    otherwise falls back to the generic stloc.s form.

    Examples:
        >>> encode_stloc(0)   # Uses stloc.0 (1 byte)
        b'\\x0a'
        >>> encode_stloc(3)   # Uses stloc.3 (1 byte)
        b'\\x0d'
        >>> encode_stloc(10)  # Uses stloc.s 10 (2 bytes)
        b'\\x13\\x0a'
    """
    if 0 <= slot <= 3:
        return bytes([CLROpcode.STLOC_0 + slot])
    return bytes([CLROpcode.STLOC_S, slot])


def encode_ldloc(slot: int) -> bytes:
    """Encode loading from a local variable slot.

    Uses the short form (ldloc.0 through ldloc.3) when possible,
    otherwise falls back to the generic ldloc.s form.

    Examples:
        >>> encode_ldloc(0)   # Uses ldloc.0 (1 byte)
        b'\\x06'
        >>> encode_ldloc(3)   # Uses ldloc.3 (1 byte)
        b'\\x09'
        >>> encode_ldloc(10)  # Uses ldloc.s 10 (2 bytes)
        b'\\x11\\x0a'
    """
    if 0 <= slot <= 3:
        return bytes([CLROpcode.LDLOC_0 + slot])
    return bytes([CLROpcode.LDLOC_S, slot])
