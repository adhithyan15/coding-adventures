"""wasm-opcodes — Complete WASM 1.0 opcode table with metadata.

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.

-------------------------------------------------------------------------------
WHAT IS A WASM OPCODE?
-------------------------------------------------------------------------------

A WebAssembly module's code section contains sequences of *instructions*. Each
instruction begins with a 1-byte opcode — a number from 0x00 to 0xFF — that
tells the WASM interpreter what operation to perform. Some instructions are
followed by *immediates*: additional encoded values (like a constant integer,
a memory offset, or a label index) that parameterize the operation.

WASM is a *stack machine*. Instructions communicate via an implicit *operand
stack*. An instruction like i32.add does not name registers; instead it *pops*
two i32 values from the stack and *pushes* their sum back. The "stack effect"
of an instruction is simply: how many values does it pop, and how many does it
push?

Stack effect diagram for i32.add (pops 2, pushes 1):

    Before:           After:
    ┌──────────┐      ┌──────────┐
    │  7 (i32) │      │  3 (i32) │  ← result (7 - 4 = 3 ... wait, it's add)
    ├──────────┤      └──────────┘
    │  4 (i32) │
    └──────────┘
                    Actually 7 + 4 = 11:
    Before:           After:
    ┌──────────┐      ┌──────────┐
    │ 11 (i32) │      │ 11 (i32) │
    ├──────────┤      └──────────┘
    │          │  ←  i32.add consumed both, produced one
    └──────────┘

-------------------------------------------------------------------------------
WHY DOES WASM USE STRUCTURED CONTROL FLOW?
-------------------------------------------------------------------------------

Traditional bytecode formats (JVM, x86) use arbitrary jumps: a branch
instruction gives a raw byte offset to jump to. WASM deliberately forbids
arbitrary jumps. Instead, control flow is *structured*:

  - block / loop / if    — push a "label" (a nesting level) onto a control stack
  - br / br_if / br_table — branch to a label by nesting *depth* (0 = innermost)
  - end                  — close the innermost structured block

This design has several advantages:
  1. Validation is a single linear pass — no need to build a control-flow graph.
  2. JIT compilation is straightforward — structured blocks map directly to
     native conditional branches and loops.
  3. Security: impossible to jump into the middle of an instruction or to a
     label in a different function.
  4. Streaming: a WASM binary can be compiled while it is still downloading,
     because the bytecode can be validated and compiled top-to-bottom without
     needing to look ahead at jump targets.

Compare with x86 assembly where `jmp 0x1234` could point anywhere in memory.
In WASM, `br 0` means "branch to the immediately enclosing block's exit" — the
target is always well-defined by the nesting structure.

-------------------------------------------------------------------------------
WHAT IS A MEMARG?
-------------------------------------------------------------------------------

Memory load and store instructions access the module's linear memory. They need
two pieces of information beyond the memory address on the stack:

  - align:  A power-of-two alignment hint (encoded as log2). For example,
            align=2 means 4-byte alignment (2^2 = 4). The runtime *may* use
            this hint to generate faster code, but is not required to. A
            misaligned access is still valid.
  - offset: A constant byte offset added to the runtime address on the stack.
            This lets the compiler encode struct field accesses as a single
            instruction: load the base pointer, add the constant field offset.

Together (align, offset) are called a "memarg" and they appear as two LEB128
values immediately after the opcode byte:

    0x28               i32.load
    0x02               align = 2 (4-byte aligned)
    0x04               offset = 4 (bytes from the base address)

Example in wat (text format):
    ;; C equivalent: int y = ((struct Point*)ptr)->y;
    ;; where y is at offset 4 in the struct
    local.get 0        ;; push ptr
    i32.load offset=4  ;; read 4 bytes at ptr+4

-------------------------------------------------------------------------------
OPCODE CATEGORIES
-------------------------------------------------------------------------------

    control      — structured control flow: block, loop, if, br, call, ...
    parametric   — type-agnostic stack manipulation: drop, select
    variable     — local and global variable access
    memory       — loads, stores, memory.size, memory.grow
    numeric_i32  — 32-bit integer arithmetic, comparisons, bitwise ops
    numeric_i64  — 64-bit integer arithmetic, comparisons, bitwise ops
    numeric_f32  — 32-bit floating-point arithmetic
    numeric_f64  — 64-bit floating-point arithmetic
    conversion   — type conversions: wraps, truncs, extends, reinterprets

-------------------------------------------------------------------------------
COMPLETE OPCODE TABLE (WASM 1.0 — 183 instructions)
-------------------------------------------------------------------------------

Control instructions:
  0x00  unreachable        ()               pop=0 push=0
  0x01  nop                ()               pop=0 push=0
  0x02  block              (blocktype,)     pop=0 push=0
  0x03  loop               (blocktype,)     pop=0 push=0
  0x04  if                 (blocktype,)     pop=1 push=0
  0x05  else               ()               pop=0 push=0
  0x0B  end                ()               pop=0 push=0
  0x0C  br                 (labelidx,)      pop=0 push=0
  0x0D  br_if              (labelidx,)      pop=1 push=0
  0x0E  br_table           (vec_labelidx,)  pop=1 push=0
  0x0F  return             ()               pop=0 push=0
  0x10  call               (funcidx,)       pop=0 push=0
  0x11  call_indirect      (typeidx,tableidx) pop=1 push=0

Parametric instructions:
  0x1A  drop               ()               pop=1 push=0
  0x1B  select             ()               pop=3 push=1

Variable instructions:
  0x20  local.get          (localidx,)      pop=0 push=1
  0x21  local.set          (localidx,)      pop=1 push=0
  0x22  local.tee          (localidx,)      pop=1 push=1
  0x23  global.get         (globalidx,)     pop=0 push=1
  0x24  global.set         (globalidx,)     pop=1 push=0

Memory load instructions (memarg = align + offset immediates):
  0x28  i32.load           (memarg,)        pop=1 push=1
  0x29  i64.load           (memarg,)        pop=1 push=1
  0x2A  f32.load           (memarg,)        pop=1 push=1
  0x2B  f64.load           (memarg,)        pop=1 push=1
  0x2C  i32.load8_s        (memarg,)        pop=1 push=1
  0x2D  i32.load8_u        (memarg,)        pop=1 push=1
  0x2E  i32.load16_s       (memarg,)        pop=1 push=1
  0x2F  i32.load16_u       (memarg,)        pop=1 push=1
  0x30  i64.load8_s        (memarg,)        pop=1 push=1
  0x31  i64.load8_u        (memarg,)        pop=1 push=1
  0x32  i64.load16_s       (memarg,)        pop=1 push=1
  0x33  i64.load16_u       (memarg,)        pop=1 push=1
  0x34  i64.load32_s       (memarg,)        pop=1 push=1
  0x35  i64.load32_u       (memarg,)        pop=1 push=1

Memory store instructions (memarg = align + offset immediates):
  0x36  i32.store          (memarg,)        pop=2 push=0
  0x37  i64.store          (memarg,)        pop=2 push=0
  0x38  f32.store          (memarg,)        pop=2 push=0
  0x39  f64.store          (memarg,)        pop=2 push=0
  0x3A  i32.store8         (memarg,)        pop=2 push=0
  0x3B  i32.store16        (memarg,)        pop=2 push=0
  0x3C  i64.store8         (memarg,)        pop=2 push=0
  0x3D  i64.store16        (memarg,)        pop=2 push=0
  0x3E  i64.store32        (memarg,)        pop=2 push=0

Memory management:
  0x3F  memory.size        (memidx,)        pop=0 push=1
  0x40  memory.grow        (memidx,)        pop=1 push=1

i32 numeric instructions:
  0x41  i32.const          (i32,)           pop=0 push=1
  0x45  i32.eqz            ()               pop=1 push=1
  0x46  i32.eq             ()               pop=2 push=1
  0x47  i32.ne             ()               pop=2 push=1
  0x48  i32.lt_s           ()               pop=2 push=1
  0x49  i32.lt_u           ()               pop=2 push=1
  0x4A  i32.gt_s           ()               pop=2 push=1
  0x4B  i32.gt_u           ()               pop=2 push=1
  0x4C  i32.le_s           ()               pop=2 push=1
  0x4D  i32.le_u           ()               pop=2 push=1
  0x4E  i32.ge_s           ()               pop=2 push=1
  0x4F  i32.ge_u           ()               pop=2 push=1
  0x67  i32.clz            ()               pop=1 push=1
  0x68  i32.ctz            ()               pop=1 push=1
  0x69  i32.popcnt         ()               pop=1 push=1
  0x6A  i32.add            ()               pop=2 push=1
  0x6B  i32.sub            ()               pop=2 push=1
  0x6C  i32.mul            ()               pop=2 push=1
  0x6D  i32.div_s          ()               pop=2 push=1
  0x6E  i32.div_u          ()               pop=2 push=1
  0x6F  i32.rem_s          ()               pop=2 push=1
  0x70  i32.rem_u          ()               pop=2 push=1
  0x71  i32.and            ()               pop=2 push=1
  0x72  i32.or             ()               pop=2 push=1
  0x73  i32.xor            ()               pop=2 push=1
  0x74  i32.shl            ()               pop=2 push=1
  0x75  i32.shr_s          ()               pop=2 push=1
  0x76  i32.shr_u          ()               pop=2 push=1
  0x77  i32.rotl           ()               pop=2 push=1
  0x78  i32.rotr           ()               pop=2 push=1

i64 numeric instructions:
  0x42  i64.const          (i64,)           pop=0 push=1
  0x50  i64.eqz            ()               pop=1 push=1
  0x51  i64.eq             ()               pop=2 push=1
  0x52  i64.ne             ()               pop=2 push=1
  0x53  i64.lt_s           ()               pop=2 push=1
  0x54  i64.lt_u           ()               pop=2 push=1
  0x55  i64.gt_s           ()               pop=2 push=1
  0x56  i64.gt_u           ()               pop=2 push=1
  0x57  i64.le_s           ()               pop=2 push=1
  0x58  i64.le_u           ()               pop=2 push=1
  0x59  i64.ge_s           ()               pop=2 push=1
  0x5A  i64.ge_u           ()               pop=2 push=1
  0x79  i64.clz            ()               pop=1 push=1
  0x7A  i64.ctz            ()               pop=1 push=1
  0x7B  i64.popcnt         ()               pop=1 push=1
  0x7C  i64.add            ()               pop=2 push=1
  0x7D  i64.sub            ()               pop=2 push=1
  0x7E  i64.mul            ()               pop=2 push=1
  0x7F  i64.div_s          ()               pop=2 push=1
  0x80  i64.div_u          ()               pop=2 push=1
  0x81  i64.rem_s          ()               pop=2 push=1
  0x82  i64.rem_u          ()               pop=2 push=1
  0x83  i64.and            ()               pop=2 push=1
  0x84  i64.or             ()               pop=2 push=1
  0x85  i64.xor            ()               pop=2 push=1
  0x86  i64.shl            ()               pop=2 push=1
  0x87  i64.shr_s          ()               pop=2 push=1
  0x88  i64.shr_u          ()               pop=2 push=1
  0x89  i64.rotl           ()               pop=2 push=1
  0x8A  i64.rotr           ()               pop=2 push=1

f32 numeric instructions:
  0x43  f32.const          (f32,)           pop=0 push=1
  0x5B  f32.eq             ()               pop=2 push=1
  0x5C  f32.ne             ()               pop=2 push=1
  0x5D  f32.lt             ()               pop=2 push=1
  0x5E  f32.gt             ()               pop=2 push=1
  0x5F  f32.le             ()               pop=2 push=1
  0x60  f32.ge             ()               pop=2 push=1
  0x8B  f32.abs            ()               pop=1 push=1
  0x8C  f32.neg            ()               pop=1 push=1
  0x8D  f32.ceil           ()               pop=1 push=1
  0x8E  f32.floor          ()               pop=1 push=1
  0x8F  f32.trunc          ()               pop=1 push=1
  0x90  f32.nearest        ()               pop=1 push=1
  0x91  f32.sqrt           ()               pop=1 push=1
  0x92  f32.add            ()               pop=2 push=1
  0x93  f32.sub            ()               pop=2 push=1
  0x94  f32.mul            ()               pop=2 push=1
  0x95  f32.div            ()               pop=2 push=1
  0x96  f32.min            ()               pop=2 push=1
  0x97  f32.max            ()               pop=2 push=1
  0x98  f32.copysign       ()               pop=2 push=1

f64 numeric instructions:
  0x44  f64.const          (f64,)           pop=0 push=1
  0x61  f64.eq             ()               pop=2 push=1
  0x62  f64.ne             ()               pop=2 push=1
  0x63  f64.lt             ()               pop=2 push=1
  0x64  f64.gt             ()               pop=2 push=1
  0x65  f64.le             ()               pop=2 push=1
  0x66  f64.ge             ()               pop=2 push=1
  0x99  f64.abs            ()               pop=1 push=1
  0x9A  f64.neg            ()               pop=1 push=1
  0x9B  f64.ceil           ()               pop=1 push=1
  0x9C  f64.floor          ()               pop=1 push=1
  0x9D  f64.trunc          ()               pop=1 push=1
  0x9E  f64.nearest        ()               pop=1 push=1
  0x9F  f64.sqrt           ()               pop=1 push=1
  0xA0  f64.add            ()               pop=2 push=1
  0xA1  f64.sub            ()               pop=2 push=1
  0xA2  f64.mul            ()               pop=2 push=1
  0xA3  f64.div            ()               pop=2 push=1
  0xA4  f64.min            ()               pop=2 push=1
  0xA5  f64.max            ()               pop=2 push=1
  0xA6  f64.copysign       ()               pop=2 push=1

Conversion instructions (all pop=1 push=1):
  0xA7  i32.wrap_i64
  0xA8  i32.trunc_f32_s
  0xA9  i32.trunc_f32_u
  0xAA  i32.trunc_f64_s
  0xAB  i32.trunc_f64_u
  0xAC  i64.extend_i32_s
  0xAD  i64.extend_i32_u
  0xAE  i64.trunc_f32_s
  0xAF  i64.trunc_f32_u
  0xB0  i64.trunc_f64_s
  0xB1  i64.trunc_f64_u
  0xB2  f32.convert_i32_s
  0xB3  f32.convert_i32_u
  0xB4  f32.convert_i64_s
  0xB5  f32.convert_i64_u
  0xB6  f32.demote_f64
  0xB7  f64.convert_i32_s
  0xB8  f64.convert_i32_u
  0xB9  f64.convert_i64_s
  0xBA  f64.convert_i64_u
  0xBB  f64.promote_f32
  0xBC  i32.reinterpret_f32
  0xBD  i64.reinterpret_f64
  0xBE  f32.reinterpret_i32
  0xBF  f64.reinterpret_i64
"""

from __future__ import annotations

from dataclasses import dataclass

__version__ = "0.1.0"


# ---------------------------------------------------------------------------
# OPCODEINFO — THE METADATA RECORD FOR A SINGLE INSTRUCTION
#
# Each WASM instruction has five pieces of metadata:
#
#   name       — the human-readable mnemonic used in .wat text format
#                e.g., "i32.add", "memory.grow", "br_if"
#
#   opcode     — the 1-byte value that appears in the binary encoding
#                e.g., 0x6A for i32.add
#
#   category   — groups instructions by what they operate on:
#                "control", "parametric", "variable", "memory",
#                "numeric_i32", "numeric_i64", "numeric_f32", "numeric_f64",
#                "conversion"
#
#   immediates — tuple of strings naming the encoded operands that follow
#                the opcode byte in the binary stream. These are not the
#                values on the operand *stack* — those are the implicit
#                pop/push. Immediates are explicit bytes in the instruction
#                encoding. Examples:
#                  ()               — no immediates (e.g., i32.add)
#                  ("i32",)         — one LEB128 i32 constant (i32.const)
#                  ("memarg",)      — alignment + offset (i32.load)
#                  ("labelidx",)    — branch target depth (br)
#                  ("blocktype",)   — block result type (block, loop, if)
#                  ("funcidx",)     — function index (call)
#                  ("localidx",)    — local variable index (local.get)
#                  ("globalidx",)   — global variable index (global.get)
#                  ("memidx",)      — memory index (memory.size, memory.grow)
#                  ("typeidx", "tableidx") — two indices (call_indirect)
#                  ("vec_labelidx",)       — a vector of label indices (br_table)
#
#   stack_pop  — number of values consumed from the operand stack
#   stack_push — number of values produced onto the operand stack
#
# The dataclass is frozen (immutable) so instances can be used as dict values
# or even dict keys. Tuple is used for immediates (not list) for the same
# reason: immutability all the way down.
#
# Stack effect analogy: think of the stack like a plate dispenser in a
# cafeteria. stack_pop = how many plates you take off the top. stack_push =
# how many new plates you set on top when done. Net effect = push - pop.
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class OpcodeInfo:
    """Metadata record for a single WASM 1.0 instruction.

    Attributes:
        name:       Mnemonic from the WASM text format (e.g., "i32.add").
        opcode:     Byte value in the binary encoding (e.g., 0x6A).
        category:   Instruction category (e.g., "numeric_i32", "control").
        immediates: Tuple of immediate operand names encoded after the opcode.
        stack_pop:  Number of values consumed from the operand stack.
        stack_push: Number of values pushed onto the operand stack.

    Example:
        >>> info = OpcodeInfo(
        ...     name="i32.add",
        ...     opcode=0x6A,
        ...     category="numeric_i32",
        ...     immediates=(),
        ...     stack_pop=2,
        ...     stack_push=1,
        ... )
        >>> info.name
        'i32.add'
        >>> info.stack_pop
        2
    """

    name: str
    opcode: int
    category: str
    immediates: tuple[str, ...]
    stack_pop: int
    stack_push: int


# ---------------------------------------------------------------------------
# RAW TABLE — THE MASTER LIST OF ALL 183 WASM 1.0 INSTRUCTIONS
#
# Defined as a flat list of tuples for compactness. Each tuple is:
#   (name, opcode, category, immediates_tuple, stack_pop, stack_push)
#
# The list is processed once at module load time to build two lookup dicts:
#   OPCODES         — keyed by byte value (int)
#   OPCODES_BY_NAME — keyed by mnemonic (str)
#
# Why tuples instead of constructing OpcodeInfo directly in the list?
# Tuple literals are more compact and easier to diff/review when the table
# is hundreds of rows long. The conversion to OpcodeInfo happens in one place
# just below the table.
#
# Naming note: "vec_labelidx" means the immediate is a *vector* (length-
# prefixed sequence) of label indices. In the binary format, br_table encodes:
#   LEB128 count n
#   n × LEB128 labelidx   (the branch table targets)
#   LEB128 labelidx       (the default target)
# We collapse this to the single token "vec_labelidx" in the immediates tuple.
# ---------------------------------------------------------------------------

_RAW_TABLE: list[tuple[str, int, str, tuple[str, ...], int, int]] = [
    # -----------------------------------------------------------------------
    # CONTROL — structured control flow
    #
    # WASM uses structured control flow instead of arbitrary jumps (see module
    # docstring). The "unreachable" instruction is special: it marks dead code
    # that the validator knows will never execute. If it *is* executed, the
    # runtime traps immediately. It is often used as a compiler assertion:
    # "I know we can't reach here."
    #
    # "nop" does nothing. It is sometimes emitted by code generators as a
    # placeholder or alignment filler.
    #
    # "block", "loop", "if" all push a new control frame. "end" pops it.
    # "br 0" exits the innermost block. In a *loop*, "br 0" jumps back to the
    # loop's *start* (making loops actually loop). In a *block*, "br 0" jumps
    # to the block's *end* (breaking out). This asymmetry is intentional:
    # it makes both forward exits (break) and backward jumps (continue) natural.
    # -----------------------------------------------------------------------
    ("unreachable",   0x00, "control",     (),                         0, 0),
    ("nop",           0x01, "control",     (),                         0, 0),
    ("block",         0x02, "control",     ("blocktype",),             0, 0),
    ("loop",          0x03, "control",     ("blocktype",),             0, 0),
    ("if",            0x04, "control",     ("blocktype",),             1, 0),
    ("else",          0x05, "control",     (),                         0, 0),
    ("end",           0x0B, "control",     (),                         0, 0),
    ("br",            0x0C, "control",     ("labelidx",),              0, 0),
    ("br_if",         0x0D, "control",     ("labelidx",),              1, 0),
    ("br_table",      0x0E, "control",     ("vec_labelidx",),          1, 0),
    ("return",        0x0F, "control",     (),                         0, 0),
    # "call" pops the function's arguments and pushes its results. We record
    # pop=0/push=0 here because the actual arity depends on the called function's
    # type — it's not a fixed number. The caller must consult the type section.
    ("call",          0x10, "control",     ("funcidx",),               0, 0),
    # "call_indirect" pops an i32 table index plus the function's arguments.
    # pop=1 records the table-index pop; the argument pops are type-dependent.
    ("call_indirect", 0x11, "control",     ("typeidx", "tableidx"),    1, 0),

    # -----------------------------------------------------------------------
    # PARAMETRIC — type-agnostic stack manipulation
    #
    # "drop" discards the top-of-stack value regardless of its type. Useful
    # when a function returns a value you don't need.
    #
    # "select" is WASM's conditional expression (like C's ternary ?:).
    # It pops: condition (i32), val_if_false, val_if_true — and pushes one
    # result. Importantly, *both* branches are already evaluated (both values
    # are already on the stack). This is different from an if/else block where
    # only one branch executes. select is only valid when both values have the
    # same type.
    #
    # Stack diagram for select:
    #   Before: [ val_true | val_false | condition ]  (condition = top)
    #   After:  [ val_true ]  if condition != 0
    #           [ val_false ] if condition == 0
    # -----------------------------------------------------------------------
    ("drop",          0x1A, "parametric",  (),                         1, 0),
    ("select",        0x1B, "parametric",  (),                         3, 1),

    # -----------------------------------------------------------------------
    # VARIABLE — local and global variable access
    #
    # Locals include function parameters (0..n_params-1) and declared local
    # variables (n_params..n_params+n_locals-1).
    #
    # "local.tee" is like "local.set" but also leaves the value on the stack.
    # Name comes from a T-junction: the value flows both to the local slot
    # AND stays on the stack. Equivalent to: local.set N; local.get N.
    #
    # "global.set" is only valid on *mutable* globals; the validator rejects
    # set on immutable globals at compile time.
    # -----------------------------------------------------------------------
    ("local.get",     0x20, "variable",    ("localidx",),              0, 1),
    ("local.set",     0x21, "variable",    ("localidx",),              1, 0),
    ("local.tee",     0x22, "variable",    ("localidx",),              1, 1),
    ("global.get",    0x23, "variable",    ("globalidx",),             0, 1),
    ("global.set",    0x24, "variable",    ("globalidx",),             1, 0),

    # -----------------------------------------------------------------------
    # MEMORY LOADS — read from linear memory
    #
    # All load instructions follow this pattern:
    #   1. Pop one i32 address from the stack.
    #   2. Add the memarg offset (a constant immediate) to get the effective addr.
    #   3. Read N bytes from memory at that effective address.
    #   4. Optionally sign-extend or zero-extend the result.
    #   5. Push the result value.
    #
    # "_s" suffix = sign-extended (the high bits are filled with the sign bit).
    # "_u" suffix = zero-extended (the high bits are filled with 0).
    #
    # Example — i32.load8_s reads 1 byte and sign-extends to 32 bits:
    #   byte value 0xFF is treated as -1 (because 0xFF = 1111_1111 as signed)
    # Example — i32.load8_u reads 1 byte and zero-extends to 32 bits:
    #   byte value 0xFF is treated as 255 (unsigned)
    #
    # The memarg immediate consists of two LEB128 values:
    #   align: log2 of the expected alignment (e.g., 2 means 4-byte aligned)
    #   offset: constant byte offset added to the runtime address
    # -----------------------------------------------------------------------
    ("i32.load",      0x28, "memory",      ("memarg",),                1, 1),
    ("i64.load",      0x29, "memory",      ("memarg",),                1, 1),
    ("f32.load",      0x2A, "memory",      ("memarg",),                1, 1),
    ("f64.load",      0x2B, "memory",      ("memarg",),                1, 1),
    ("i32.load8_s",   0x2C, "memory",      ("memarg",),                1, 1),
    ("i32.load8_u",   0x2D, "memory",      ("memarg",),                1, 1),
    ("i32.load16_s",  0x2E, "memory",      ("memarg",),                1, 1),
    ("i32.load16_u",  0x2F, "memory",      ("memarg",),                1, 1),
    ("i64.load8_s",   0x30, "memory",      ("memarg",),                1, 1),
    ("i64.load8_u",   0x31, "memory",      ("memarg",),                1, 1),
    ("i64.load16_s",  0x32, "memory",      ("memarg",),                1, 1),
    ("i64.load16_u",  0x33, "memory",      ("memarg",),                1, 1),
    ("i64.load32_s",  0x34, "memory",      ("memarg",),                1, 1),
    ("i64.load32_u",  0x35, "memory",      ("memarg",),                1, 1),

    # -----------------------------------------------------------------------
    # MEMORY STORES — write to linear memory
    #
    # Store instructions pop TWO values:
    #   1. The value to store (pushed first, so it's deeper in the stack)
    #   2. The i32 address (pushed second, so it's on top)
    #
    # Stack order for i32.store:
    #   Before: [ value (i32) | address (i32) ]  ← address on top
    #   After:  []  (both consumed, nothing pushed)
    #
    # Truncating stores (i32.store8, i32.store16, i64.store8, etc.) write only
    # the low N bits of the value; the rest is silently discarded.
    # -----------------------------------------------------------------------
    ("i32.store",     0x36, "memory",      ("memarg",),                2, 0),
    ("i64.store",     0x37, "memory",      ("memarg",),                2, 0),
    ("f32.store",     0x38, "memory",      ("memarg",),                2, 0),
    ("f64.store",     0x39, "memory",      ("memarg",),                2, 0),
    ("i32.store8",    0x3A, "memory",      ("memarg",),                2, 0),
    ("i32.store16",   0x3B, "memory",      ("memarg",),                2, 0),
    ("i64.store8",    0x3C, "memory",      ("memarg",),                2, 0),
    ("i64.store16",   0x3D, "memory",      ("memarg",),                2, 0),
    ("i64.store32",   0x3E, "memory",      ("memarg",),                2, 0),

    # -----------------------------------------------------------------------
    # MEMORY MANAGEMENT
    #
    # "memory.size" pushes the current size of the memory in 64-KiB pages.
    # "memory.grow" pops the number of pages to add, and pushes the previous
    #   size (before growing), or -1 if growth failed (e.g., OOM or exceeded
    #   the maximum declared in the memory type).
    #
    # The "memidx" immediate is always 0 in WASM 1.0 (only one memory allowed).
    # It is present in the binary encoding for forward-compatibility with future
    # multi-memory proposals.
    #
    # Memory size calculation:
    #   byte_size = page_count * 65536
    #   max WASM 1.0 memory: 65536 pages * 65536 bytes/page = 4 GiB
    # -----------------------------------------------------------------------
    ("memory.size",   0x3F, "memory",      ("memidx",),                0, 1),
    ("memory.grow",   0x40, "memory",      ("memidx",),                1, 1),

    # -----------------------------------------------------------------------
    # i32 NUMERIC — 32-bit integer operations
    #
    # "i32.const" pushes a constant i32 value. The value is encoded as a
    # signed LEB128 integer immediately following the opcode byte.
    #
    # Comparison instructions push 1 (true) or 0 (false) as an i32 result.
    # WASM has no boolean type; booleans are i32 values (0 or 1).
    #
    # "eqz" tests if a value equals zero. It pops one value, unlike eq/ne
    # which pop two. Useful for implementing "while (x)" as:
    #   loop
    #     local.get $x
    #     i32.eqz
    #     br_if 1   ;; exit if x == 0
    #     ...
    #     br 0      ;; continue loop
    #   end
    #
    # "clz" = count leading zeros, "ctz" = count trailing zeros,
    # "popcnt" = population count (number of 1 bits).
    #
    # "_s" = signed interpretation, "_u" = unsigned interpretation.
    # For addition, subtraction, multiplication: the bit pattern is the same
    # regardless of sign, so there's only one i32.add (no signed/unsigned split).
    # -----------------------------------------------------------------------
    ("i32.const",     0x41, "numeric_i32", ("i32",),                   0, 1),
    ("i32.eqz",       0x45, "numeric_i32", (),                         1, 1),
    ("i32.eq",        0x46, "numeric_i32", (),                         2, 1),
    ("i32.ne",        0x47, "numeric_i32", (),                         2, 1),
    ("i32.lt_s",      0x48, "numeric_i32", (),                         2, 1),
    ("i32.lt_u",      0x49, "numeric_i32", (),                         2, 1),
    ("i32.gt_s",      0x4A, "numeric_i32", (),                         2, 1),
    ("i32.gt_u",      0x4B, "numeric_i32", (),                         2, 1),
    ("i32.le_s",      0x4C, "numeric_i32", (),                         2, 1),
    ("i32.le_u",      0x4D, "numeric_i32", (),                         2, 1),
    ("i32.ge_s",      0x4E, "numeric_i32", (),                         2, 1),
    ("i32.ge_u",      0x4F, "numeric_i32", (),                         2, 1),
    ("i32.clz",       0x67, "numeric_i32", (),                         1, 1),
    ("i32.ctz",       0x68, "numeric_i32", (),                         1, 1),
    ("i32.popcnt",    0x69, "numeric_i32", (),                         1, 1),
    ("i32.add",       0x6A, "numeric_i32", (),                         2, 1),
    ("i32.sub",       0x6B, "numeric_i32", (),                         2, 1),
    ("i32.mul",       0x6C, "numeric_i32", (),                         2, 1),
    ("i32.div_s",     0x6D, "numeric_i32", (),                         2, 1),
    ("i32.div_u",     0x6E, "numeric_i32", (),                         2, 1),
    ("i32.rem_s",     0x6F, "numeric_i32", (),                         2, 1),
    ("i32.rem_u",     0x70, "numeric_i32", (),                         2, 1),
    ("i32.and",       0x71, "numeric_i32", (),                         2, 1),
    ("i32.or",        0x72, "numeric_i32", (),                         2, 1),
    ("i32.xor",       0x73, "numeric_i32", (),                         2, 1),
    ("i32.shl",       0x74, "numeric_i32", (),                         2, 1),
    ("i32.shr_s",     0x75, "numeric_i32", (),                         2, 1),
    ("i32.shr_u",     0x76, "numeric_i32", (),                         2, 1),
    ("i32.rotl",      0x77, "numeric_i32", (),                         2, 1),
    ("i32.rotr",      0x78, "numeric_i32", (),                         2, 1),

    # -----------------------------------------------------------------------
    # i64 NUMERIC — 64-bit integer operations
    #
    # The structure mirrors i32. All the same operation categories exist.
    # i64 is used for 64-bit counters, timestamps, file offsets, and any
    # integer that doesn't fit in 32 bits.
    # -----------------------------------------------------------------------
    ("i64.const",     0x42, "numeric_i64", ("i64",),                   0, 1),
    ("i64.eqz",       0x50, "numeric_i64", (),                         1, 1),
    ("i64.eq",        0x51, "numeric_i64", (),                         2, 1),
    ("i64.ne",        0x52, "numeric_i64", (),                         2, 1),
    ("i64.lt_s",      0x53, "numeric_i64", (),                         2, 1),
    ("i64.lt_u",      0x54, "numeric_i64", (),                         2, 1),
    ("i64.gt_s",      0x55, "numeric_i64", (),                         2, 1),
    ("i64.gt_u",      0x56, "numeric_i64", (),                         2, 1),
    ("i64.le_s",      0x57, "numeric_i64", (),                         2, 1),
    ("i64.le_u",      0x58, "numeric_i64", (),                         2, 1),
    ("i64.ge_s",      0x59, "numeric_i64", (),                         2, 1),
    ("i64.ge_u",      0x5A, "numeric_i64", (),                         2, 1),
    ("i64.clz",       0x79, "numeric_i64", (),                         1, 1),
    ("i64.ctz",       0x7A, "numeric_i64", (),                         1, 1),
    ("i64.popcnt",    0x7B, "numeric_i64", (),                         1, 1),
    ("i64.add",       0x7C, "numeric_i64", (),                         2, 1),
    ("i64.sub",       0x7D, "numeric_i64", (),                         2, 1),
    ("i64.mul",       0x7E, "numeric_i64", (),                         2, 1),
    ("i64.div_s",     0x7F, "numeric_i64", (),                         2, 1),
    ("i64.div_u",     0x80, "numeric_i64", (),                         2, 1),
    ("i64.rem_s",     0x81, "numeric_i64", (),                         2, 1),
    ("i64.rem_u",     0x82, "numeric_i64", (),                         2, 1),
    ("i64.and",       0x83, "numeric_i64", (),                         2, 1),
    ("i64.or",        0x84, "numeric_i64", (),                         2, 1),
    ("i64.xor",       0x85, "numeric_i64", (),                         2, 1),
    ("i64.shl",       0x86, "numeric_i64", (),                         2, 1),
    ("i64.shr_s",     0x87, "numeric_i64", (),                         2, 1),
    ("i64.shr_u",     0x88, "numeric_i64", (),                         2, 1),
    ("i64.rotl",      0x89, "numeric_i64", (),                         2, 1),
    ("i64.rotr",      0x8A, "numeric_i64", (),                         2, 1),

    # -----------------------------------------------------------------------
    # f32 NUMERIC — 32-bit IEEE 754 floating-point operations
    #
    # f32.const encodes a 32-bit float as 4 raw bytes (little-endian IEEE 754).
    # Unlike the integer consts which use LEB128, floats use a fixed-width
    # encoding to preserve the exact bit pattern.
    #
    # "nearest" rounds to the nearest even integer (banker's rounding / round
    # half to even). This is different from C's round() which rounds half away
    # from zero. WASM follows the IEEE 754 default rounding mode.
    #
    # "copysign" takes the magnitude of the first argument and the sign bit of
    # the second argument: copysign(|a|, sign(b)).
    # -----------------------------------------------------------------------
    ("f32.const",     0x43, "numeric_f32", ("f32",),                   0, 1),
    ("f32.eq",        0x5B, "numeric_f32", (),                         2, 1),
    ("f32.ne",        0x5C, "numeric_f32", (),                         2, 1),
    ("f32.lt",        0x5D, "numeric_f32", (),                         2, 1),
    ("f32.gt",        0x5E, "numeric_f32", (),                         2, 1),
    ("f32.le",        0x5F, "numeric_f32", (),                         2, 1),
    ("f32.ge",        0x60, "numeric_f32", (),                         2, 1),
    ("f32.abs",       0x8B, "numeric_f32", (),                         1, 1),
    ("f32.neg",       0x8C, "numeric_f32", (),                         1, 1),
    ("f32.ceil",      0x8D, "numeric_f32", (),                         1, 1),
    ("f32.floor",     0x8E, "numeric_f32", (),                         1, 1),
    ("f32.trunc",     0x8F, "numeric_f32", (),                         1, 1),
    ("f32.nearest",   0x90, "numeric_f32", (),                         1, 1),
    ("f32.sqrt",      0x91, "numeric_f32", (),                         1, 1),
    ("f32.add",       0x92, "numeric_f32", (),                         2, 1),
    ("f32.sub",       0x93, "numeric_f32", (),                         2, 1),
    ("f32.mul",       0x94, "numeric_f32", (),                         2, 1),
    ("f32.div",       0x95, "numeric_f32", (),                         2, 1),
    ("f32.min",       0x96, "numeric_f32", (),                         2, 1),
    ("f32.max",       0x97, "numeric_f32", (),                         2, 1),
    ("f32.copysign",  0x98, "numeric_f32", (),                         2, 1),

    # -----------------------------------------------------------------------
    # f64 NUMERIC — 64-bit IEEE 754 floating-point operations
    #
    # f64.const encodes a 64-bit double as 8 raw bytes (little-endian IEEE 754).
    # The operation set mirrors f32 exactly.
    # -----------------------------------------------------------------------
    ("f64.const",     0x44, "numeric_f64", ("f64",),                   0, 1),
    ("f64.eq",        0x61, "numeric_f64", (),                         2, 1),
    ("f64.ne",        0x62, "numeric_f64", (),                         2, 1),
    ("f64.lt",        0x63, "numeric_f64", (),                         2, 1),
    ("f64.gt",        0x64, "numeric_f64", (),                         2, 1),
    ("f64.le",        0x65, "numeric_f64", (),                         2, 1),
    ("f64.ge",        0x66, "numeric_f64", (),                         2, 1),
    ("f64.abs",       0x99, "numeric_f64", (),                         1, 1),
    ("f64.neg",       0x9A, "numeric_f64", (),                         1, 1),
    ("f64.ceil",      0x9B, "numeric_f64", (),                         1, 1),
    ("f64.floor",     0x9C, "numeric_f64", (),                         1, 1),
    ("f64.trunc",     0x9D, "numeric_f64", (),                         1, 1),
    ("f64.nearest",   0x9E, "numeric_f64", (),                         1, 1),
    ("f64.sqrt",      0x9F, "numeric_f64", (),                         1, 1),
    ("f64.add",       0xA0, "numeric_f64", (),                         2, 1),
    ("f64.sub",       0xA1, "numeric_f64", (),                         2, 1),
    ("f64.mul",       0xA2, "numeric_f64", (),                         2, 1),
    ("f64.div",       0xA3, "numeric_f64", (),                         2, 1),
    ("f64.min",       0xA4, "numeric_f64", (),                         2, 1),
    ("f64.max",       0xA5, "numeric_f64", (),                         2, 1),
    ("f64.copysign",  0xA6, "numeric_f64", (),                         2, 1),

    # -----------------------------------------------------------------------
    # CONVERSIONS — type conversion instructions
    #
    # All conversion instructions pop 1 value and push 1 value (different type).
    #
    # Naming convention: result_type.operation_source_type
    #   e.g., i32.wrap_i64    = take an i64, wrap it to i32 (truncate high bits)
    #         i64.extend_i32_s = take an i32, sign-extend it to i64
    #         f32.convert_i32_s = convert signed i32 to f32 (may lose precision)
    #         i32.trunc_f32_s  = truncate f32 toward zero, result is signed i32
    #         i32.reinterpret_f32 = reinterpret the raw bits of an f32 as i32
    #
    # "wrap" (i32.wrap_i64): keep only the low 32 bits of a 64-bit integer.
    #   i64 value 0x0000_0001_FFFF_FFFF → i32 value 0xFFFF_FFFF (= -1 signed)
    #
    # "extend" (i64.extend_i32_s/u): widen a 32-bit integer to 64 bits.
    #   _s: sign-extend (fill high 32 bits with the sign bit of the i32)
    #   _u: zero-extend (fill high 32 bits with 0)
    #
    # "trunc" (i32.trunc_f32_s etc.): truncate a float to an integer by
    # discarding the fractional part (toward zero). Traps if the float is
    # NaN or if the result would overflow the integer type.
    #
    # "reinterpret": no numeric conversion — just reinterpret the same 32 or
    # 64 bits as a different type. Like a C union or memcpy cast.
    #   i32.reinterpret_f32: same 4 bytes, now treated as an i32 bit pattern
    # -----------------------------------------------------------------------
    ("i32.wrap_i64",        0xA7, "conversion", (), 1, 1),
    ("i32.trunc_f32_s",     0xA8, "conversion", (), 1, 1),
    ("i32.trunc_f32_u",     0xA9, "conversion", (), 1, 1),
    ("i32.trunc_f64_s",     0xAA, "conversion", (), 1, 1),
    ("i32.trunc_f64_u",     0xAB, "conversion", (), 1, 1),
    ("i64.extend_i32_s",    0xAC, "conversion", (), 1, 1),
    ("i64.extend_i32_u",    0xAD, "conversion", (), 1, 1),
    ("i64.trunc_f32_s",     0xAE, "conversion", (), 1, 1),
    ("i64.trunc_f32_u",     0xAF, "conversion", (), 1, 1),
    ("i64.trunc_f64_s",     0xB0, "conversion", (), 1, 1),
    ("i64.trunc_f64_u",     0xB1, "conversion", (), 1, 1),
    ("f32.convert_i32_s",   0xB2, "conversion", (), 1, 1),
    ("f32.convert_i32_u",   0xB3, "conversion", (), 1, 1),
    ("f32.convert_i64_s",   0xB4, "conversion", (), 1, 1),
    ("f32.convert_i64_u",   0xB5, "conversion", (), 1, 1),
    ("f32.demote_f64",      0xB6, "conversion", (), 1, 1),
    ("f64.convert_i32_s",   0xB7, "conversion", (), 1, 1),
    ("f64.convert_i32_u",   0xB8, "conversion", (), 1, 1),
    ("f64.convert_i64_s",   0xB9, "conversion", (), 1, 1),
    ("f64.convert_i64_u",   0xBA, "conversion", (), 1, 1),
    ("f64.promote_f32",     0xBB, "conversion", (), 1, 1),
    ("i32.reinterpret_f32", 0xBC, "conversion", (), 1, 1),
    ("i64.reinterpret_f64", 0xBD, "conversion", (), 1, 1),
    ("f32.reinterpret_i32", 0xBE, "conversion", (), 1, 1),
    ("f64.reinterpret_i64", 0xBF, "conversion", (), 1, 1),
]


# ---------------------------------------------------------------------------
# BUILD THE LOOKUP DICTIONARIES
#
# We convert the raw tuple list into two dicts exactly once at module import
# time. The dicts are then frozen in the module namespace as constants.
#
# This is the only place where OpcodeInfo objects are constructed. After this
# point, the raw table is no longer needed (though it stays in memory).
#
# Dict comprehension reads: for each row in the raw table, construct an
# OpcodeInfo and map it from its opcode byte (or name) to that info object.
# ---------------------------------------------------------------------------

OPCODES: dict[int, OpcodeInfo] = {
    opcode: OpcodeInfo(
        name=name,
        opcode=opcode,
        category=category,
        immediates=immediates,
        stack_pop=stack_pop,
        stack_push=stack_push,
    )
    for name, opcode, category, immediates, stack_pop, stack_push in _RAW_TABLE
}
"""Lookup table keyed by opcode byte value.

Example:
    >>> OPCODES[0x6A].name
    'i32.add'
    >>> OPCODES[0x00].name
    'unreachable'
"""

OPCODES_BY_NAME: dict[str, OpcodeInfo] = {
    info.name: info for info in OPCODES.values()
}
"""Lookup table keyed by instruction mnemonic.

Example:
    >>> OPCODES_BY_NAME["i32.add"].opcode
    106
    >>> OPCODES_BY_NAME["memory.grow"].category
    'memory'
"""


# ---------------------------------------------------------------------------
# CONVENIENCE FUNCTIONS
#
# These thin wrappers provide a None-returning interface rather than a
# KeyError-raising one. They are useful in parsers and validators that need
# to check whether a byte or name is a valid opcode before processing it.
#
# Performance note: both functions are O(1) — they delegate directly to dict
# lookup. Python's dict is a hash table, so lookup is constant time regardless
# of the table size.
# ---------------------------------------------------------------------------


def get_opcode(byte: int) -> OpcodeInfo | None:
    """Return the OpcodeInfo for a given byte value, or None if unknown.

    Args:
        byte: The opcode byte value (0x00–0xFF).

    Returns:
        An OpcodeInfo if the byte is a valid WASM 1.0 opcode, else None.

    Example:
        >>> get_opcode(0x6A).name
        'i32.add'
        >>> get_opcode(0xFF) is None
        True
    """
    return OPCODES.get(byte)


def get_opcode_by_name(name: str) -> OpcodeInfo | None:
    """Return the OpcodeInfo for a given mnemonic, or None if unknown.

    Args:
        name: The WASM text format mnemonic (e.g., "i32.add", "memory.grow").

    Returns:
        An OpcodeInfo if the name is a valid WASM 1.0 mnemonic, else None.

    Example:
        >>> get_opcode_by_name("i32.add").opcode
        106
        >>> get_opcode_by_name("not_a_real_op") is None
        True
    """
    return OPCODES_BY_NAME.get(name)
