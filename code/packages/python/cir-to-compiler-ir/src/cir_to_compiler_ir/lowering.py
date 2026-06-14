"""_CIRLowerer — two-pass CIR-to-IrProgram lowering.

This module implements ``lower_cir_to_ir_program()``, the core function of the
``cir-to-compiler-ir`` package.  It converts a ``list[CIRInstr]`` produced by
the JIT/AOT specialisation pass into a target-independent ``IrProgram`` that any
``ir-to-*`` backend can consume.

Why two passes?
---------------

CIR uses *named* virtual variables (``"x"``, ``"v0"``, ``"loop_cond"``).
``IrProgram`` uses *integer-indexed* virtual registers (``IrRegister(0)``,
``IrRegister(1)``, …).  The register allocator inside each backend takes care
of mapping these to physical registers.

To build the name→index dictionary before emitting the first instruction, we
do a short preliminary walk (Pass 1) that collects every variable name and
assigns it a unique index in order of first occurrence.  Pass 2 then emits
real ``IrInstruction`` objects, looking up each name in the dictionary.

This is the same strategy used by LLVM's SSA construction pass and is a
classic "number the values first, then emit the instructions" pattern.

Type erasure
------------

CIR carries concrete types to let the backend select the right instruction
encoding (e.g., ``add_u8``, ``add_i32``, ``add_f64``).  The ``IrProgram``
representation is untyped:

  - All integer arithmetic → ``ADD``, ``SUB``, ``MUL``, ``DIV``, …
  - All float arithmetic   → ``F64_ADD``, ``F64_SUB``, ``F64_MUL``, ``F64_DIV``
  - Comparisons follow the same pattern: integer → ``CMP_EQ``/``CMP_LT``/…,
    float → ``F64_CMP_EQ``/``F64_CMP_LT``/…

The backend then re-introduces types during code generation — for example, the
WASM backend emits ``i32.add`` for all integer ``ADD`` instructions.

Synthesised opcodes
-------------------

``IrOp`` has no ``CMP_LE`` or ``CMP_GE`` for integers — only the float
variants ``F64_CMP_LE`` and ``F64_CMP_GE`` exist.  This is intentional: most
register-machine ISAs (Intel 8008, GE-225, 4004) have compare-and-branch
primitives that test ``<`` and ``>``, not ``<=`` and ``>=``.  The backends
synthesise those using NOT.

The lowerer follows the same convention:

  cmp_le_{int}  →  tmp = CMP_GT(src0, src1); dest = NOT(tmp)
  cmp_ge_{int}  →  tmp = CMP_LT(src0, src1); dest = NOT(tmp)
  neg_{int}     →  tmp = LOAD_IMM(0);         dest = SUB(tmp, src0)
  neg_f64       →  tmp = LOAD_F64_IMM(0.0);   dest = F64_SUB(tmp, src0)

Scratch registers for these synthetic operations are allocated *after* all
named variables, so they never shadow a variable the user wrote.

Usage
-----
::

    from codegen_core import CIRInstr
    from cir_to_compiler_ir import lower_cir_to_ir_program

    instrs = [
        CIRInstr("const_i32", "x", [40], "i32"),
        CIRInstr("const_i32", "y", [2],  "i32"),
        CIRInstr("add_i32",   "z", ["x", "y"], "i32"),
        CIRInstr("ret_void",  None, [], "void"),
    ]
    prog = lower_cir_to_ir_program(instrs)
    # prog.instructions[0]  →  LABEL  _start
    # prog.instructions[1]  →  LOAD_IMM  v0, 40
    # prog.instructions[2]  →  LOAD_IMM  v1, 2
    # prog.instructions[3]  →  ADD       v2, v0, v1
    # prog.instructions[4]  →  HALT
"""

from __future__ import annotations

from codegen_core import CIRInstr
from compiler_ir import (
    IDGenerator,
    IrFloatImmediate,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)

from cir_to_compiler_ir.errors import CIRLoweringError
from cir_to_compiler_ir.validator import validate_cir_for_lowering

# ---------------------------------------------------------------------------
# Integer and float type suffix sets
# ---------------------------------------------------------------------------
#
# CIR encodes the value type in the op suffix: ``add_u8``, ``cmp_lt_i32``.
# We strip the suffix (everything after the last ``_``) and look it up here
# to decide which IR opcode family to use.
#
# Integer family  →  untyped IR ops (ADD, SUB, …, CMP_EQ, …)
# Float family    →  floating-point IR ops (F64_ADD, F64_CMP_EQ, …)
#
# The ``const_`` and ``neg_`` rules also use these sets.
# ---------------------------------------------------------------------------

_INT_TYPES: frozenset[str] = frozenset(
    {"u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64", "bool"}
)

_FLOAT_TYPES: frozenset[str] = frozenset({"f32", "f64"})

# ---------------------------------------------------------------------------
# Heap base offset for ``load_mem`` / ``store_mem`` lowering.
# ---------------------------------------------------------------------------
#
# When the WASM backend compiles a program that uses both byte-tape
# memory ops AND WASI ``fd_write`` / ``fd_read`` (BF06: Brainfuck's
# `.` / `,`), it places a 16-byte WASI scratch region at the bottom of
# linear memory.  A naive ``base = 0`` for tape access would alias the
# tape's first 16 cells with that scratch — every ``putchar`` would
# trample tape cells 0..15.
#
# Fix: pad past the WASI scratch.  ``_HEAP_BASE_OFFSET`` reserves the
# first 16 bytes of linear memory for backend use; tape cell N lives
# at WASM linear-memory address ``_HEAP_BASE_OFFSET + N``.  This costs
# 16 bytes per program and zero ops at runtime (the offset is a
# compile-time constant), and cleanly avoids the collision regardless
# of whether the final program uses WASI.
_HEAP_BASE_OFFSET = 16


# ---------------------------------------------------------------------------
# _CIRLowerer — the actual lowering engine
# ---------------------------------------------------------------------------


class _CIRLowerer:
    """Two-pass CIR-to-IrProgram lowering engine.

    **Do not instantiate directly** — call ``lower_cir_to_ir_program()``
    instead, which validates first and wraps any internal errors.

    Internal state
    --------------
    ``_reg``
        Maps CIR variable names to ``IrRegister`` indices.  Populated during
        Pass 1 (variable collection) and queried during Pass 2 (instruction
        emission).

    ``_next``
        The next free register index.  Incremented every time a new name is
        assigned (Pass 1) or a scratch register is allocated (Pass 2).

    ``_prog``
        The ``IrProgram`` being built.  Modified in-place during Pass 2.

    ``_gen``
        The ``IDGenerator`` that assigns unique IDs to real instructions.
        LABEL pseudo-instructions always use ``id=-1``.
    """

    def __init__(self, entry_label: str) -> None:
        self._entry_label = entry_label
        self._reg: dict[str, int] = {}
        self._next: int = 0
        self._prog = IrProgram(entry_label=entry_label)
        self._gen = IDGenerator()

    # ------------------------------------------------------------------
    # Register helpers
    # ------------------------------------------------------------------

    def _var(self, name: str) -> IrRegister:
        """Return the ``IrRegister`` assigned to *name*.

        If *name* has not been seen before, assign it the next free index.
        This ensures the same variable name always maps to the same register,
        maintaining the single-assignment property of CIR.

        Args:
            name: A CIR variable name (e.g. ``"x"``, ``"loop_cond"``).

        Returns:
            The corresponding ``IrRegister``.
        """
        if name not in self._reg:
            self._reg[name] = self._next
            self._next += 1
        return IrRegister(index=self._reg[name])

    def _fresh(self) -> IrRegister:
        """Allocate a fresh scratch register for synthetic instructions.

        Scratch registers are *not* recorded in ``_reg`` — they are
        anonymous temporaries used only for synthesised ops like
        ``cmp_le`` (which needs an intermediate result for the NOT step).

        Returns:
            A new ``IrRegister`` with a unique index.
        """
        idx = self._next
        self._next += 1
        return IrRegister(index=idx)

    # ------------------------------------------------------------------
    # Literal helpers
    # ------------------------------------------------------------------

    def _operand(
        self, src: str | int | float | bool
    ) -> IrRegister | IrImmediate | IrFloatImmediate:
        """Convert a CIR ``srcs`` element to an IR operand.

        CIR source operands are one of:
          - ``str``   → a named variable  → ``IrRegister``
          - ``int``   → integer literal   → ``IrImmediate``
          - ``float`` → float literal     → ``IrFloatImmediate``
          - ``bool``  → boolean literal   → ``IrImmediate(1 or 0)``

        Note that Python's ``bool`` is a subclass of ``int``, so the
        ``bool`` branch must come first in the isinstance chain.

        Args:
            src: One element from ``CIRInstr.srcs``.

        Returns:
            The corresponding IR operand.
        """
        if isinstance(src, bool):
            # Must check bool before int — bool is a subclass of int in Python.
            # True → 1, False → 0, matching how IR represents boolean values.
            return IrImmediate(value=1 if src else 0)
        if isinstance(src, int):
            return IrImmediate(value=src)
        if isinstance(src, float):
            return IrFloatImmediate(value=src)
        # str → named variable → register lookup
        return self._var(src)

    # ------------------------------------------------------------------
    # Instruction emission helpers
    # ------------------------------------------------------------------

    def _emit(self, opcode: IrOp, *operands: object) -> None:
        """Append one real instruction to the program.

        Real instructions get a unique monotonically increasing ID from
        ``_gen``.  LABEL pseudo-instructions use ``_emit_label()`` instead.

        Args:
            opcode: The ``IrOp`` to emit.
            *operands: The ``IrRegister``, ``IrImmediate``, ``IrFloatImmediate``,
                       or ``IrLabel`` operands for the instruction.
        """
        self._prog.add_instruction(
            IrInstruction(
                opcode=opcode,
                operands=list(operands),  # type: ignore[arg-type]
                id=self._gen.next(),
            )
        )

    def _emit_label(self, name: str) -> None:
        """Append a LABEL pseudo-instruction (``id = -1``).

        LABEL instructions produce no machine code — they only mark an
        address that JUMP/BRANCH instructions can target.  They always
        receive ``id = -1`` so they are excluded from the source map.

        Args:
            name: The label name (e.g. ``"_start"``, ``"loop_0_end"``).
        """
        self._prog.add_instruction(
            IrInstruction(
                opcode=IrOp.LABEL,
                operands=[IrLabel(name=name)],
                id=-1,
            )
        )

    # ------------------------------------------------------------------
    # Pass 1 — variable collection
    # ------------------------------------------------------------------

    def _collect_vars(self, instrs: list[CIRInstr]) -> None:
        """Walk all instructions and register every named variable.

        This pass assigns a stable register index to each unique variable
        name, in order of first occurrence.  By doing this before Pass 2,
        every variable has a register assigned before any instruction that
        *uses* it is emitted — even if the use precedes the definition
        syntactically (which can happen in loops).

        The pass processes both ``dest`` fields and ``srcs`` entries that
        are strings (variable references, not label names from jmp/label ops).

        Args:
            instrs: The full CIR instruction list.
        """
        for instr in instrs:
            # Assign dest
            if instr.dest is not None:
                self._var(instr.dest)

            # Assign variable-typed sources.
            # Skip label names (srcs[0] for "label", "jmp", "jmp_if_true",
            # "jmp_if_false", "br_true_bool", "br_false_bool", "call"):
            # these are not variable names — they are IrLabel strings.
            is_label_src_op = instr.op in (
                "label",
                "jmp",
                "call",
            )
            # Conditional branches: srcs[0] = variable, srcs[1] = label
            is_conditional_branch = instr.op in (
                "jmp_if_true",
                "jmp_if_false",
                "br_true_bool",
                "br_false_bool",
            )
            # type_assert: srcs[0] = variable, srcs[1] = type string (not a var)
            is_type_assert = instr.op == "type_assert"

            for idx, src in enumerate(instr.srcs):
                if not isinstance(src, str):
                    continue  # literal — nothing to register

                if is_label_src_op:
                    continue  # all srcs are label names, not variable names

                if is_conditional_branch and idx == 1:
                    continue  # srcs[1] is the label name, not a variable

                if is_type_assert and idx == 1:
                    continue  # srcs[1] is a type name string, not a variable

                self._var(src)

    # ------------------------------------------------------------------
    # Pass 2 — instruction emission
    # ------------------------------------------------------------------

    def _lower_instr(self, instr: CIRInstr) -> None:
        """Translate one ``CIRInstr`` into one or more ``IrInstruction``s.

        Dispatches on the CIR op string using a series of string prefix
        checks.  The type suffix (e.g. ``_i32``, ``_f64``) is stripped
        and used to select between the integer and float IR opcode families.

        Args:
            instr: The CIR instruction to lower.

        Raises:
            CIRLoweringError: If the op is unsupported (``call_runtime``,
                              ``io_in``, ``io_out``) or unknown.
        """
        op = instr.op
        dest = instr.dest

        # ── Constants ───────────────────────────────────────────────────────

        if op.startswith("const_"):
            # const_{int_type}  → LOAD_IMM(dest, literal)
            # const_f64         → LOAD_F64_IMM(dest, literal)
            # const_bool        → LOAD_IMM(dest, 1 or 0)
            assert dest is not None, f"const op must have a dest: {instr}"
            suffix = op[len("const_"):]
            dest_reg = self._var(dest)
            literal = instr.srcs[0]

            if suffix in _FLOAT_TYPES:
                # Float constant — emit LOAD_F64_IMM
                val = float(literal)
                self._emit(IrOp.LOAD_F64_IMM, dest_reg, IrFloatImmediate(val))
            elif suffix in _INT_TYPES or suffix == "bool":
                # Integer / boolean constant — emit LOAD_IMM
                # bool literals arrive as Python booleans (True/False).
                val = 1 if literal is True else 0 if literal is False else int(literal)  # type: ignore[arg-type]
                self._emit(IrOp.LOAD_IMM, dest_reg, IrImmediate(val))
            else:
                raise CIRLoweringError(
                    f"unknown const suffix '{suffix}' in op '{op}'"
                )
            return

        # ── Binary arithmetic ────────────────────────────────────────────────
        #
        # Pattern: {family}_{type}  where family ∈ {add, sub, mul, div,
        #                                           and, or, xor}
        # All produce [dest, src0, src1].
        #
        # Immediate-operand handling
        # --------------------------
        # The JIT specialiser may emit a CIR instruction where one of the
        # source operands is a literal integer rather than a variable name.
        # For example:
        #   add_u8 _acc [_acc, 2]   # src1 = 2 (int, not a variable name)
        #
        # Most IR backends require both arithmetic sources to be registers.
        # IrOp has immediate variants for some operations (ADD_IMM, AND_IMM,
        # OR_IMM, XOR_IMM) — we prefer these when src1 is a literal.
        # For operations without an immediate variant (SUB, MUL, DIV), we
        # load the literal into a fresh scratch register first.
        # The same applies when src0 is a literal (commute or load-then-op).

        _BINARY_INT_MAP: dict[str, IrOp] = {
            "add": IrOp.ADD,
            "sub": IrOp.SUB,
            "mul": IrOp.MUL,
            "div": IrOp.DIV,
            "and": IrOp.AND,
            "or":  IrOp.OR,
            "xor": IrOp.XOR,
        }

        # Operations that have a [dest, src_reg, src_imm] immediate variant.
        _BINARY_INT_IMM_MAP: dict[str, IrOp] = {
            "add": IrOp.ADD_IMM,
            "and": IrOp.AND_IMM,
            "or":  IrOp.OR_IMM,
            "xor": IrOp.XOR_IMM,
        }

        _BINARY_FLOAT_MAP: dict[str, IrOp] = {
            "add": IrOp.F64_ADD,
            "sub": IrOp.F64_SUB,
            "mul": IrOp.F64_MUL,
            "div": IrOp.F64_DIV,
        }

        # Detect binary arithmetic by splitting on the first underscore:
        #   "add_i32"  → ("add", "i32")
        #   "or_u8"    → ("or",  "u8")
        parts = op.split("_", 1)
        if len(parts) == 2 and parts[0] in _BINARY_INT_MAP and parts[1] in _INT_TYPES:
            assert dest is not None
            family = parts[0]
            ir_op = _BINARY_INT_MAP[family]
            dest_reg = self._var(dest)
            raw0 = instr.srcs[0]
            raw1 = instr.srcs[1]
            src0 = self._operand(raw0)
            src1 = self._operand(raw1)

            # ── Case: src1 is an immediate ───────────────────────────────────
            if isinstance(src1, IrImmediate):
                imm_op = _BINARY_INT_IMM_MAP.get(family)
                if imm_op is not None:
                    # Use the immediate variant: ADD_IMM, AND_IMM, etc.
                    # src0 must be a register here.
                    if isinstance(src0, IrImmediate):
                        # Both immediates: load src0 into a scratch first.
                        s = self._fresh()
                        self._emit(IrOp.LOAD_IMM, s, src0)
                        self._emit(imm_op, dest_reg, s, src1)
                    else:
                        self._emit(imm_op, dest_reg, src0, src1)
                else:
                    # No immediate variant: load src1 into a scratch register.
                    s = self._fresh()
                    self._emit(IrOp.LOAD_IMM, s, src1)
                    if isinstance(src0, IrImmediate):
                        s0 = self._fresh()
                        self._emit(IrOp.LOAD_IMM, s0, src0)
                        self._emit(ir_op, dest_reg, s0, s)
                    else:
                        self._emit(ir_op, dest_reg, src0, s)
                return

            # ── Case: src0 is an immediate (src1 is a register) ─────────────
            if isinstance(src0, IrImmediate):
                # Load src0 into a scratch; commutative ops could swap but
                # we always load to preserve correctness for non-commutative ops.
                s = self._fresh()
                self._emit(IrOp.LOAD_IMM, s, src0)
                self._emit(ir_op, dest_reg, s, src1)
                return

            # ── Normal register-register case ────────────────────────────────
            self._emit(ir_op, dest_reg, src0, src1)
            return

        if (
            len(parts) == 2
            and parts[0] in _BINARY_FLOAT_MAP
            and parts[1] in _FLOAT_TYPES
        ):
            assert dest is not None
            ir_op = _BINARY_FLOAT_MAP[parts[0]]
            dest_reg = self._var(dest)
            src0 = self._operand(instr.srcs[0])
            src1 = self._operand(instr.srcs[1])
            self._emit(ir_op, dest_reg, src0, src1)
            return

        # ── Unary ops ───────────────────────────────────────────────────────

        if op.startswith("not_"):
            # not_{int_type} → NOT(dest, src)
            suffix = op[len("not_"):]
            if suffix in _INT_TYPES:
                assert dest is not None
                dest_reg = self._var(dest)
                src0 = self._operand(instr.srcs[0])
                self._emit(IrOp.NOT, dest_reg, src0)
                return

        if op.startswith("neg_"):
            # neg_{int_type}  →  scratch = LOAD_IMM(0); dest = SUB(scratch, src)
            # neg_f64  →  scratch = LOAD_F64_IMM(0.0); dest = F64_SUB(scratch, src)
            #
            # This is the standard two-instruction negation sequence: the
            # equivalence is: -x == 0 - x.
            suffix = op[len("neg_"):]
            assert dest is not None
            dest_reg = self._var(dest)
            src0 = self._operand(instr.srcs[0])
            scratch = self._fresh()

            if suffix in _INT_TYPES:
                self._emit(IrOp.LOAD_IMM, scratch, IrImmediate(0))
                self._emit(IrOp.SUB, dest_reg, scratch, src0)
                return
            if suffix in _FLOAT_TYPES:
                self._emit(IrOp.LOAD_F64_IMM, scratch, IrFloatImmediate(0.0))
                self._emit(IrOp.F64_SUB, dest_reg, scratch, src0)
                return

        # ── Integer comparisons ──────────────────────────────────────────────
        #
        # Pattern: cmp_{rel}_{int_type}
        # IrOp has: CMP_EQ, CMP_NE, CMP_LT, CMP_GT — but NO CMP_LE or CMP_GE.
        # cmp_le and cmp_ge are synthesised using three instructions:
        #
        #   cmp_le(a, b):
        #     gt   = CMP_GT(a, b)      # → 0 or 1
        #     zero = LOAD_IMM(0)
        #     dest = CMP_EQ(gt, zero)  # → 1 if a≤b (gt==0), else 0
        #
        #   cmp_ge(a, b):
        #     lt   = CMP_LT(a, b)
        #     zero = LOAD_IMM(0)
        #     dest = CMP_EQ(lt, zero)  # → 1 if a≥b (lt==0), else 0
        #
        # Why NOT is not used here
        # -------------------------
        # IrOp.NOT is bitwise complement (XOR with 0xFFFFFFFF in WASM).
        # Bitwise NOT of 0 is -1 (0xFFFFFFFF), not 1 — so NOT cannot be used
        # to negate a boolean 0/1 value reliably across all backends.
        # CMP_EQ with zero is the universally correct "logical NOT" for
        # comparison results that are guaranteed to be 0 or 1.
        #
        # Truth table for cmp_le_i32 (via CMP_EQ(CMP_GT, 0)):
        #   a=1, b=2: CMP_GT(1,2)=0 → CMP_EQ(0,0)=1 ✓  (1 ≤ 2 is true)
        #   a=2, b=2: CMP_GT(2,2)=0 → CMP_EQ(0,0)=1 ✓  (2 ≤ 2 is true)
        #   a=3, b=2: CMP_GT(3,2)=1 → CMP_EQ(1,0)=0 ✓  (3 ≤ 2 is false)

        _CMP_INT_MAP: dict[str, IrOp] = {
            "eq": IrOp.CMP_EQ,
            "ne": IrOp.CMP_NE,
            "lt": IrOp.CMP_LT,
            "gt": IrOp.CMP_GT,
        }

        if op.startswith("cmp_"):
            cmp_body = op[len("cmp_"):]  # e.g. "eq_i32", "le_u8"
            cmp_parts = cmp_body.split("_", 1)
            if len(cmp_parts) == 2:
                rel, type_suffix = cmp_parts[0], cmp_parts[1]

                if type_suffix in _INT_TYPES:
                    assert dest is not None
                    dest_reg = self._var(dest)
                    src0 = self._operand(instr.srcs[0])
                    src1 = self._operand(instr.srcs[1])

                    if rel in _CMP_INT_MAP:
                        self._emit(_CMP_INT_MAP[rel], dest_reg, src0, src1)
                        return

                    if rel == "le":
                        # Synthesise: gt=CMP_GT(a,b); zero=0; dest=CMP_EQ(gt,zero)
                        gt = self._fresh()
                        zero = self._fresh()
                        self._emit(IrOp.CMP_GT, gt, src0, src1)
                        self._emit(IrOp.LOAD_IMM, zero, IrImmediate(0))
                        self._emit(IrOp.CMP_EQ, dest_reg, gt, zero)
                        return

                    if rel == "ge":
                        # Synthesise: lt=CMP_LT(a,b); zero=0; dest=CMP_EQ(lt,zero)
                        lt = self._fresh()
                        zero = self._fresh()
                        self._emit(IrOp.CMP_LT, lt, src0, src1)
                        self._emit(IrOp.LOAD_IMM, zero, IrImmediate(0))
                        self._emit(IrOp.CMP_EQ, dest_reg, lt, zero)
                        return

                # Float comparisons — IrOp has all six variants directly
                _CMP_FLOAT_MAP: dict[str, IrOp] = {
                    "eq": IrOp.F64_CMP_EQ,
                    "ne": IrOp.F64_CMP_NE,
                    "lt": IrOp.F64_CMP_LT,
                    "gt": IrOp.F64_CMP_GT,
                    "le": IrOp.F64_CMP_LE,
                    "ge": IrOp.F64_CMP_GE,
                }

                if type_suffix in _FLOAT_TYPES and rel in _CMP_FLOAT_MAP:
                    assert dest is not None
                    dest_reg = self._var(dest)
                    src0 = self._operand(instr.srcs[0])
                    src1 = self._operand(instr.srcs[1])
                    self._emit(_CMP_FLOAT_MAP[rel], dest_reg, src0, src1)
                    return

        # ── Control flow ─────────────────────────────────────────────────────

        if op == "label":
            # Label: srcs[0] is the label name string
            self._emit_label(str(instr.srcs[0]))
            return

        if op == "jmp":
            # Unconditional jump: srcs[0] is the target label name
            self._emit(IrOp.JUMP, IrLabel(str(instr.srcs[0])))
            return

        if op in ("jmp_if_true", "br_true_bool"):
            # Conditional branch if non-zero: srcs[0]=cond_var, srcs[1]=label
            cond = self._var(str(instr.srcs[0]))
            self._emit(IrOp.BRANCH_NZ, cond, IrLabel(str(instr.srcs[1])))
            return

        if op in ("jmp_if_false", "br_false_bool"):
            # Conditional branch if zero: srcs[0]=cond_var, srcs[1]=label
            cond = self._var(str(instr.srcs[0]))
            self._emit(IrOp.BRANCH_Z, cond, IrLabel(str(instr.srcs[1])))
            return

        if op == "call":
            # Call: srcs[0] is the function label name
            self._emit(IrOp.CALL, IrLabel(str(instr.srcs[0])))
            return

        # ── Return / halt ────────────────────────────────────────────────────

        if op == "ret_void" or op.startswith("ret_"):
            # V1: return value is ignored — simply halt.
            # LANG22 will add multi-function support with proper return conventions.
            self._emit(IrOp.HALT)
            return

        # ── Meta / guards ────────────────────────────────────────────────────

        if op == "type_assert":
            # Type guards have already been enforced by the virtual machine at
            # the point where this CIR was emitted.  We retain them as COMMENT
            # instructions so the IR printer can show them for debugging.
            var_name = str(instr.srcs[0]) if instr.srcs else "?"
            type_name = str(instr.srcs[1]) if len(instr.srcs) > 1 else "?"
            self._emit(
                IrOp.COMMENT,
                IrLabel(f"type_assert {var_name} : {type_name}"),
            )
            return

        # ── Tetrad VM move instruction ───────────────────────────────────────
        #
        # The Tetrad VM and JIT specialiser emit ``tetrad.move`` as an
        # explicit register-to-register copy during specialisation.  It is
        # not part of the stable CIR opcode set defined in LANG21's spec, but
        # the specialiser produces it for temporary value moves that would
        # otherwise require SSA φ-nodes.
        #
        # Lowering strategy: ``ADD_IMM dest, src, 0`` (the canonical
        # "MOV via add-zero" pattern used throughout the IrProgram backends).
        # If the source is a literal (unusual but possible), we fall back to
        # ``LOAD_IMM`` instead.

        if op == "tetrad.move":
            assert dest is not None, "tetrad.move must have a dest"
            dest_reg = self._var(dest)
            src_operand = self._operand(instr.srcs[0])
            if isinstance(src_operand, IrImmediate):
                # Literal source — just load it directly.
                self._emit(IrOp.LOAD_IMM, dest_reg, src_operand)
            else:
                # Register source — copy via ADD_IMM with zero.
                self._emit(IrOp.ADD_IMM, dest_reg, src_operand, IrImmediate(0))
            return

        # ── Memory access ────────────────────────────────────────────────────
        #
        # Brainfuck (and any other byte-tape language) compiles to ``load_mem``
        # / ``store_mem`` against a single address operand — the data pointer.
        # ``jit-core`` keeps these in ``_PASSTHROUGH_OPS``, so they arrive
        # here with their bare names and the value width stored in
        # ``CIRInstr.type``.
        #
        # The static IR's byte-access form is three-operand:
        #
        #   LOAD_BYTE  dst, base, offset   ; dst = mem[base + offset] & 0xFF
        #   STORE_BYTE src, base, offset   ; mem[base + offset] = src & 0xFF
        #
        # Brainfuck's tape lives at WASM linear-memory address 0, so we
        # synthesise a ``base = 0`` register on each access.  An IR-level
        # optimiser could hoist the redundant zero-load out of the inner
        # loop, but for V1 mechanical correctness wins over micro-perf.
        #
        # Type handling
        # -------------
        # We accept any integer type in ``instr.type`` — ``LOAD_BYTE`` /
        # ``STORE_BYTE`` mask to a byte regardless of the requested width.
        # Wider memory ops (``LOAD_WORD`` / ``STORE_WORD``) are out of
        # scope until a language wider than Brainfuck needs them.

        if op == "load_mem":
            if instr.type not in _INT_TYPES:
                raise CIRLoweringError(
                    f"unsupported load_mem type {instr.type!r}: "
                    "expected an integer width"
                )
            assert dest is not None, "load_mem must have a dest"
            dest_reg = self._var(dest)
            ptr_operand = self._operand(instr.srcs[0])

            # The IR's LOAD_BYTE wants the offset in a register.  If the CIR
            # gave us an immediate pointer (rare but possible after constant
            # folding), materialise it into a scratch first.
            if isinstance(ptr_operand, IrImmediate):
                ptr_reg = self._fresh()
                self._emit(IrOp.LOAD_IMM, ptr_reg, ptr_operand)
            else:
                ptr_reg = ptr_operand

            base = self._fresh()
            self._emit(IrOp.LOAD_IMM, base, IrImmediate(_HEAP_BASE_OFFSET))
            self._emit(IrOp.LOAD_BYTE, dest_reg, base, ptr_reg)
            return

        if op == "store_mem":
            if instr.type not in _INT_TYPES:
                raise CIRLoweringError(
                    f"unsupported store_mem type {instr.type!r}: "
                    "expected an integer width"
                )
            ptr_operand = self._operand(instr.srcs[0])
            val_operand = self._operand(instr.srcs[1])

            if isinstance(ptr_operand, IrImmediate):
                ptr_reg = self._fresh()
                self._emit(IrOp.LOAD_IMM, ptr_reg, ptr_operand)
            else:
                ptr_reg = ptr_operand

            if isinstance(val_operand, IrImmediate):
                val_reg = self._fresh()
                self._emit(IrOp.LOAD_IMM, val_reg, val_operand)
            else:
                val_reg = val_operand

            base = self._fresh()
            self._emit(IrOp.LOAD_IMM, base, IrImmediate(_HEAP_BASE_OFFSET))
            self._emit(IrOp.STORE_BYTE, val_reg, base, ptr_reg)
            return

        # ── WASI byte-level I/O (BF06) ──────────────────────────────────────
        #
        # ``call_builtin`` is a generic host-call seam in IIR.  Different
        # languages register different host callables behind it (Brainfuck:
        # ``putchar`` / ``getchar``; Twig: ``cons`` / ``car`` / ``cdr`` /
        # ``apply_closure`` / …).  ``ir-to-wasm-compiler`` already knows
        # how to emit WASI ``fd_write`` / ``fd_read`` sequences from the
        # ``IrOp.SYSCALL`` opcode — see
        # ``ir_to_wasm_compiler.compiler._emit_wasi_write`` /
        # ``_emit_wasi_read``.  So for the byte-I/O builtins we lower
        # ``call_builtin`` straight to ``SYSCALL`` and inherit the WASI
        # plumbing for free.
        #
        # CIR shapes:
        #   call_builtin dest=None  srcs=["putchar", "v"]   type="void"
        #     →   SYSCALL imm=1, val_reg
        #
        #   call_builtin dest="v"   srcs=["getchar"]        type="u8"
        #     →   SYSCALL imm=2, dest_reg
        #
        # Every other ``call_builtin`` name still falls through to the
        # generic "unknown op" path below, which causes the JIT to deopt
        # to the interpreter — that's the documented contract.

        if op == "call_builtin" and instr.srcs:
            builtin = str(instr.srcs[0])
            if builtin == "putchar":
                if len(instr.srcs) < 2:
                    raise CIRLoweringError(
                        "call_builtin 'putchar' expects 1 value argument"
                    )
                val_operand = self._operand(instr.srcs[1])
                if isinstance(val_operand, IrImmediate):
                    val_reg = self._fresh()
                    self._emit(IrOp.LOAD_IMM, val_reg, val_operand)
                else:
                    val_reg = val_operand
                self._emit(IrOp.SYSCALL, IrImmediate(1), val_reg)
                return

            if builtin == "getchar":
                if dest is None:
                    raise CIRLoweringError(
                        "call_builtin 'getchar' must have a dest register"
                    )
                dest_reg = self._var(dest)
                self._emit(IrOp.SYSCALL, IrImmediate(2), dest_reg)
                return

            # Fall through — other builtin names hit the unknown-op branch
            # below.  This is the deliberate deopt path: the JIT backend
            # converts ``CIRLoweringError`` into a "deopt to interpreter"
            # signal so unsupported builtins simply slow down rather than
            # crash.

        # ── Unsupported ops ──────────────────────────────────────────────────

        if op == "call_runtime":
            rt_name = str(instr.srcs[0]) if instr.srcs else "<unnamed>"
            raise CIRLoweringError(
                f"unsupported op 'call_runtime' ('{rt_name}'): "
                "generic runtime dispatch cannot be lowered in V1 — "
                "see LANG24 for the planned resolution"
            )

        if op in ("io_in", "io_out"):
            raise CIRLoweringError(
                f"unsupported op '{op}': I/O operations are backend-specific "
                "and cannot be lowered generically in V1 — see LANG23"
            )

        # Unknown op — safety net for future CIR extensions
        raise CIRLoweringError(
            f"unknown CIR op '{op}': no lowering rule defined in"
            " cir-to-compiler-ir v0.1"
        )

    def lower(self, instrs: list[CIRInstr]) -> IrProgram:
        """Run both passes and return the completed ``IrProgram``.

        Pass 1: collect variable names → assign register indices.
        Pass 2: emit IR instructions.

        The program always starts with a LABEL for the entry point, as
        required by the JVM and CIL backends.

        Args:
            instrs: The validated CIR instruction list.

        Returns:
            A complete ``IrProgram`` ready for any ``ir-to-*`` backend.

        Raises:
            CIRLoweringError: If an unsupported or unknown op is encountered
                              during emission.
        """
        # ── Pass 1: collect all variable names ──────────────────────────────
        self._collect_vars(instrs)

        # ── Emit entry label (always first) ─────────────────────────────────
        #
        # JVM and CIL backends require a LABEL at the start so they know where
        # to place the method preamble.  The WASM backend also uses it as the
        # function name.
        self._emit_label(self._entry_label)

        # ── Pass 2: translate each CIR instruction ──────────────────────────
        for instr in instrs:
            self._lower_instr(instr)

        return self._prog


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def lower_cir_to_ir_program(
    instrs: list[CIRInstr],
    entry_label: str = "_start",
) -> IrProgram:
    """Lower a ``list[CIRInstr]`` to an ``IrProgram``.

    This is the primary public function of the ``cir-to-compiler-ir`` package.
    It validates the instruction list, then runs the two-pass lowering
    algorithm, and returns a complete ``IrProgram`` ready for any
    ``ir-to-*`` backend.

    Args:
        instrs: The CIR instruction list to lower.  Produced by
                ``jit_core.specialise()`` (with ``min_observations=0`` for
                forced AOT) or ``aot_core.aot_specialise()``.
        entry_label: The name of the entry-point label emitted at the start
                     of the ``IrProgram``.  Defaults to ``"_start"``.

    Returns:
        A complete ``IrProgram`` with:
          - A LABEL pseudo-instruction at index 0 (entry point)
          - One ``IrInstruction`` per CIR instruction (some ops expand to 2)
          - Unique monotonically increasing IDs on all real instructions

    Raises:
        CIRLoweringError: If ``validate_cir_for_lowering`` finds any errors,
                          or if the lowerer encounters an unsupported/unknown op.

    Full pipeline example::

        from codegen_core import CIRInstr
        from cir_to_compiler_ir import lower_cir_to_ir_program
        from ir_to_wasm_compiler import validate_for_wasm

        instrs = [
            CIRInstr("const_i32", "x", [40], "i32"),
            CIRInstr("const_i32", "y", [2],  "i32"),
            CIRInstr("add_i32",   "z", ["x", "y"], "i32"),
            CIRInstr("ret_void",  None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        assert validate_for_wasm(prog) == []  # ready for WASM backend
    """
    # Validate first — collect all errors at once before raising.
    errors = validate_cir_for_lowering(instrs)
    if errors:
        joined = "; ".join(errors)
        raise CIRLoweringError(
            f"CIR validation failed ({len(errors)} error(s)): {joined}"
        )

    lowerer = _CIRLowerer(entry_label=entry_label)
    return lowerer.lower(instrs)
