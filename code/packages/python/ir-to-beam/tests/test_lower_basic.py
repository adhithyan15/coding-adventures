"""Unit tests for the basic IR-op → BEAM-op lowering.

These tests don't need ``erl`` — they verify the lowering produces
a structurally-correct ``BEAMModule`` and that ``encode_beam`` can
round-trip the result through ``beam-bytes-decoder``.
"""

from __future__ import annotations

import pytest
from beam_bytecode_encoder import BEAMTag, encode_beam
from beam_bytes_decoder import decode_beam_module
from compiler_ir import (
    IDGenerator,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)

from ir_to_beam import (
    BEAMBackendConfig,
    BEAMBackendError,
    lower_ir_to_beam,
)


def _label(name: str) -> IrLabel:
    return IrLabel(name=name)


def _reg(i: int) -> IrRegister:
    return IrRegister(index=i)


def _imm(v: int) -> IrImmediate:
    return IrImmediate(value=v)


def _build_identity_program() -> IrProgram:
    """``identity()`` returns the constant 42.

    TW03 Phase 1 convention: the result lands in ``r1``
    (``_REG_HALT_RESULT``) before ``RET`` because ir-to-beam's
    RET lowering reads y1 → x0 → return.
    """
    gen = IDGenerator()
    program = IrProgram(entry_label="identity")
    program.add_instruction(
        IrInstruction(IrOp.LABEL, [_label("identity")], id=-1)
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(42)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))
    return program


def _build_add_program() -> IrProgram:
    """``add() -> 17 + 25`` (= 42).  Result in r1."""
    gen = IDGenerator()
    program = IrProgram(entry_label="add")
    program.add_instruction(IrInstruction(IrOp.LABEL, [_label("add")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(2), _imm(17)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(3), _imm(25)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.ADD, [_reg(1), _reg(2), _reg(3)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))
    return program


# ---------------------------------------------------------------------------
# Module shape
# ---------------------------------------------------------------------------


class TestModuleShape:
    def test_module_name_lands_at_atom_one(self) -> None:
        m = lower_ir_to_beam(
            _build_identity_program(),
            BEAMBackendConfig(module_name="answer"),
        )
        assert m.name == "answer"
        assert m.atoms[0] == "answer"

    def test_user_function_is_exported(self) -> None:
        m = lower_ir_to_beam(
            _build_identity_program(),
            BEAMBackendConfig(module_name="answer"),
        )
        # We expect three exports: identity/0, module_info/0, module_info/1.
        assert len(m.exports) == 3
        names = {m.atoms[e.function_atom_index - 1] for e in m.exports}
        assert names == {"identity", "module_info"}

    def test_module_info_imports_synthesised(self) -> None:
        m = lower_ir_to_beam(
            _build_identity_program(),
            BEAMBackendConfig(module_name="answer"),
        )
        # erlang:get_module_info/1 and /2 must both be in the import table.
        triples = {
            (
                m.atoms[row.module_atom_index - 1],
                m.atoms[row.function_atom_index - 1],
                row.arity,
            )
            for row in m.imports
        }
        assert ("erlang", "get_module_info", 1) in triples
        assert ("erlang", "get_module_info", 2) in triples


# ---------------------------------------------------------------------------
# Encoder round-trip
# ---------------------------------------------------------------------------


class TestEncoderRoundTrip:
    def test_identity_program_encodes_and_decodes(self) -> None:
        m = lower_ir_to_beam(
            _build_identity_program(),
            BEAMBackendConfig(module_name="answer"),
        )
        decoded = decode_beam_module(encode_beam(m))
        assert decoded.module_name == "answer"
        # The decoder prepends None at index 0.
        assert decoded.atoms[1] == "answer"
        # All exports landed.
        export_names = {e.function for e in decoded.exports}
        assert "identity" in export_names
        assert "module_info" in export_names

    def test_add_program_uses_arithmetic_bif(self) -> None:
        m = lower_ir_to_beam(
            _build_add_program(),
            BEAMBackendConfig(module_name="adder"),
        )
        decoded = decode_beam_module(encode_beam(m))
        # erlang:+/2 must be in the import table.
        triples = {
            (imp.module, imp.function, imp.arity) for imp in decoded.imports
        }
        assert ("erlang", "+", 2) in triples


# ---------------------------------------------------------------------------
# Validation / error cases
# ---------------------------------------------------------------------------


class TestErrors:
    def test_missing_module_name_rejected(self) -> None:
        with pytest.raises(BEAMBackendError, match="module_name"):
            lower_ir_to_beam(
                _build_identity_program(), BEAMBackendConfig(module_name="")
            )

    def test_program_without_label_rejected(self) -> None:
        program = IrProgram(entry_label="x")
        program.add_instruction(IrInstruction(IrOp.RET, []))
        with pytest.raises(BEAMBackendError, match="must begin with at least one LABEL"):
            lower_ir_to_beam(program, BEAMBackendConfig(module_name="m"))

    def test_unsupported_opcode_rejected_with_clear_message(self) -> None:
        gen = IDGenerator()
        program = IrProgram(entry_label="boom")
        program.add_instruction(
            IrInstruction(IrOp.LABEL, [_label("boom")], id=-1)
        )
        # SYSCALL isn't supported in TW03 Phase 1.
        program.add_instruction(
            IrInstruction(IrOp.SYSCALL, [_imm(1), _reg(1)], id=gen.next())
        )
        with pytest.raises(BEAMBackendError, match="unsupported IR op SYSCALL"):
            lower_ir_to_beam(program, BEAMBackendConfig(module_name="boom"))

    def test_load_imm_negative_rejected(self) -> None:
        gen = IDGenerator()
        program = IrProgram(entry_label="neg")
        program.add_instruction(
            IrInstruction(IrOp.LABEL, [_label("neg")], id=-1)
        )
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [_reg(0), _imm(-1)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))
        with pytest.raises(BEAMBackendError, match="negative integer"):
            lower_ir_to_beam(program, BEAMBackendConfig(module_name="neg"))


# ---------------------------------------------------------------------------
# Sanity: opcode bytes look right
# ---------------------------------------------------------------------------


class TestOpcodeChoices:
    def test_load_imm_emits_move_opcode(self) -> None:
        m = lower_ir_to_beam(
            _build_identity_program(),
            BEAMBackendConfig(module_name="answer"),
        )
        # Opcodes used should include 64 (move), 19 (return),
        # 1 (label), 2 (func_info), 3 (int_code_end).
        used = {ins.opcode for ins in m.instructions}
        assert 64 in used   # move
        assert 19 in used   # return
        assert 1 in used    # label
        assert 2 in used    # func_info
        assert 3 in used    # int_code_end

    def test_arithmetic_emits_gc_bif2(self) -> None:
        m = lower_ir_to_beam(
            _build_add_program(),
            BEAMBackendConfig(module_name="adder"),
        )
        used = {ins.opcode for ins in m.instructions}
        assert 125 in used  # gc_bif2

    def test_arithmetic_operand_tags(self) -> None:
        m = lower_ir_to_beam(
            _build_add_program(),
            BEAMBackendConfig(module_name="adder"),
        )
        gc_bif2_instructions = [ins for ins in m.instructions if ins.opcode == 125]
        assert len(gc_bif2_instructions) == 1
        ops = gc_bif2_instructions[0].operands
        # gc_bif2 has 6 operands: fail-label, live, bif-idx, src1, src2, dest.
        # TW03 Phase 1 — sources/dest are y-registers (callee-saves
        # state survives recursive calls).
        assert len(ops) == 6
        assert ops[0].tag == BEAMTag.F
        assert ops[1].tag == BEAMTag.U
        assert ops[2].tag == BEAMTag.U
        assert ops[3].tag == BEAMTag.Y
        assert ops[4].tag == BEAMTag.Y
        assert ops[5].tag == BEAMTag.Y

    def test_branch_lowering_uses_is_ne_exact(self) -> None:
        """``BRANCH_Z r, label`` lowers to ``is_ne_exact label, {y,r}, {integer,0}``."""
        from compiler_ir import IDGenerator
        gen = IDGenerator()
        program = IrProgram(entry_label="b")
        program.add_instruction(IrInstruction(IrOp.LABEL, [_label("b")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(0)], id=gen.next())
        )
        program.add_instruction(
            IrInstruction(
                IrOp.BRANCH_Z, [_reg(1), _label("end")], id=gen.next()
            )
        )
        program.add_instruction(IrInstruction(IrOp.LABEL, [_label("end")], id=-1))
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

        m = lower_ir_to_beam(program, BEAMBackendConfig(module_name="b"))
        used = {ins.opcode for ins in m.instructions}
        assert 44 in used  # is_ne_exact

    def test_cmp_eq_lowers_to_5_instructions(self) -> None:
        """``CMP_EQ`` becomes is_eq_exact + move 1 + jump + label + move 0 + label."""
        from compiler_ir import IDGenerator
        gen = IDGenerator()
        program = IrProgram(entry_label="c")
        program.add_instruction(IrInstruction(IrOp.LABEL, [_label("c")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [_reg(2), _imm(1)], id=gen.next())
        )
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [_reg(3), _imm(2)], id=gen.next())
        )
        program.add_instruction(
            IrInstruction(IrOp.CMP_EQ, [_reg(1), _reg(2), _reg(3)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

        m = lower_ir_to_beam(program, BEAMBackendConfig(module_name="c"))
        used = {ins.opcode for ins in m.instructions}
        assert 43 in used  # is_eq_exact
        assert 61 in used  # jump

    def test_recursive_function_emits_allocate_call_deallocate(self) -> None:
        """Recursive ``fact`` IR uses CALL — backend emits allocate / call / deallocate."""
        from compiler_ir import IDGenerator
        gen = IDGenerator()
        program = IrProgram(entry_label="main")
        # fact(n)
        program.add_instruction(IrInstruction(IrOp.LABEL, [_label("fact")], id=-1))
        # Body: just CALL itself (artificial — no termination, but enough
        # to exercise allocate + call + deallocate emission).
        program.add_instruction(IrInstruction(IrOp.CALL, [_label("fact")], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))
        # main
        program.add_instruction(IrInstruction(IrOp.LABEL, [_label("main")], id=-1))
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

        m = lower_ir_to_beam(
            program,
            BEAMBackendConfig(
                module_name="rec",
                arity_overrides={"fact": 1},
            ),
        )
        used = {ins.opcode for ins in m.instructions}
        assert 12 in used   # allocate
        assert 18 in used   # deallocate
        assert 4 in used    # call


# ---------------------------------------------------------------------------
# TW03 Phase 2 — closure lowering shape
# ---------------------------------------------------------------------------


class TestClosureLowering:
    """Structural tests for MAKE_CLOSURE / APPLY_CLOSURE lowering.

    These assert on the emitted BEAMModule (atoms / instructions /
    exports / imports) rather than running anything against ``erl``,
    so they cover the closure code paths in CI environments without
    Erlang installed.  The end-to-end ``((make-adder 7) 35) → 42``
    test against real ``erl`` lives in ``test_real_erl.py``.
    """

    def _build_closure_program(self) -> IrProgram:
        """The hand-built closure_adder fixture used throughout —
        ``make_adder(N) = lambda(X) -> X + N``, called as
        ``main() = (make_adder(7))(35)``.
        """
        gen = IDGenerator()
        program = IrProgram(entry_label="main")

        # _lambda_0(N, X) — captures-first layout: y2=N, y3=X.
        program.add_instruction(
            IrInstruction(IrOp.LABEL, [_label("_lambda_0")], id=-1)
        )
        program.add_instruction(
            IrInstruction(IrOp.ADD, [_reg(1), _reg(2), _reg(3)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

        # make_adder(n) -> closure capturing n.
        program.add_instruction(
            IrInstruction(IrOp.LABEL, [_label("make_adder")], id=-1)
        )
        program.add_instruction(
            IrInstruction(
                IrOp.MAKE_CLOSURE,
                [_reg(1), _label("_lambda_0"), _imm(1), _reg(2)],
                id=gen.next(),
            )
        )
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

        # main(): call make_adder(7), then APPLY_CLOSURE with 35.
        program.add_instruction(IrInstruction(IrOp.LABEL, [_label("main")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [_reg(2), _imm(7)], id=gen.next())
        )
        program.add_instruction(
            IrInstruction(IrOp.CALL, [_label("make_adder")], id=gen.next())
        )
        program.add_instruction(
            IrInstruction(
                IrOp.ADD_IMM, [_reg(10), _reg(1), _imm(0)], id=gen.next()
            )
        )
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [_reg(11), _imm(35)], id=gen.next())
        )
        program.add_instruction(
            IrInstruction(
                IrOp.APPLY_CLOSURE,
                [_reg(1), _reg(10), _imm(1), _reg(11)],
                id=gen.next(),
            )
        )
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

        return program

    def _config(self) -> BEAMBackendConfig:
        # Lambda's explicit arity is 1 (X); the backend widens to 2
        # internally because num_free=1 (N).
        return BEAMBackendConfig(
            module_name="closure_adder",
            arity_overrides={"_lambda_0": 1, "make_adder": 1, "main": 0},
            closure_free_var_counts={"_lambda_0": 1},
        )

    def test_apply_imports_pp_and_apply_3_bifs(self) -> None:
        """``erlang:'++'/2`` and ``erlang:apply/3`` get wired into the
        ImpT table when APPLY_CLOSURE appears anywhere in the IR."""
        m = lower_ir_to_beam(self._build_closure_program(), self._config())
        triples = {
            (m.atoms[imp.module_atom_index - 1], m.atoms[imp.function_atom_index - 1], imp.arity)
            for imp in m.imports
        }
        assert ("erlang", "++", 2) in triples
        assert ("erlang", "apply", 3) in triples

    def test_lifted_lambda_is_exported_with_full_arity(self) -> None:
        """The lifted ``_lambda_0`` is exported (apply/3 looks it up
        by atom name), and its arity equals ``num_free + explicit``
        — for ``(lambda (x) ...)`` capturing 1 var, that's 2."""
        m = lower_ir_to_beam(self._build_closure_program(), self._config())
        exports = {
            (m.atoms[ex.function_atom_index - 1], ex.arity) for ex in m.exports
        }
        assert ("_lambda_0", 2) in exports

    def test_make_closure_emits_put_list_chain(self) -> None:
        """MAKE_CLOSURE lowers to a chain of ``put_list`` opcodes
        (69) preceded by a ``test_heap`` (16); no ``make_fun*``
        opcode appears."""
        m = lower_ir_to_beam(self._build_closure_program(), self._config())
        opcodes = [ins.opcode for ins in m.instructions]
        assert 16 in opcodes  # test_heap
        assert 69 in opcodes  # put_list
        assert 103 not in opcodes  # NO make_fun2
        assert 171 not in opcodes  # NO make_fun3

    def test_apply_closure_emits_get_hd_get_tl_call_ext(self) -> None:
        """APPLY_CLOSURE lowers using ``get_hd`` (162), ``get_tl``
        (163), and ``call_ext`` (7) for the apply/3 dispatch."""
        m = lower_ir_to_beam(self._build_closure_program(), self._config())
        opcodes = [ins.opcode for ins in m.instructions]
        assert 162 in opcodes  # get_hd
        assert 163 in opcodes  # get_tl
        assert 7 in opcodes    # call_ext

    def test_no_make_fun_nor_call_fun(self) -> None:
        """Sanity: even though the Phase 2b FunT scaffolding exists,
        the Phase 2c lowering deliberately avoids ``make_fun*`` and
        ``call_fun`` because OTP 28 rejects the former and the
        latter only pairs with funs."""
        m = lower_ir_to_beam(self._build_closure_program(), self._config())
        opcodes = {ins.opcode for ins in m.instructions}
        assert 75 not in opcodes   # call_fun
        assert 76 not in opcodes   # make_fun
        assert 103 not in opcodes  # make_fun2
        assert 171 not in opcodes  # make_fun3
        # And no FunT chunk gets emitted — funs tuple is empty.
        assert m.funs == ()

    def test_make_closure_with_zero_captures(self) -> None:
        """MAKE_CLOSURE with num_captured=0 (a function value with
        no captures) still emits a 1-element list ``[FnAtom]``."""
        gen = IDGenerator()
        program = IrProgram(entry_label="main")
        program.add_instruction(
            IrInstruction(IrOp.LABEL, [_label("_lambda_0")], id=-1)
        )
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(7)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LABEL, [_label("main")], id=-1))
        program.add_instruction(
            IrInstruction(
                IrOp.MAKE_CLOSURE,
                [_reg(1), _label("_lambda_0"), _imm(0)],
                id=gen.next(),
            )
        )
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

        m = lower_ir_to_beam(
            program,
            BEAMBackendConfig(
                module_name="zero_caps",
                arity_overrides={"_lambda_0": 0, "main": 0},
                closure_free_var_counts={"_lambda_0": 0},
            ),
        )
        opcodes = [ins.opcode for ins in m.instructions]
        assert 69 in opcodes  # put_list — still needed for the [Fn] cons
        assert 16 in opcodes  # test_heap reservation

    def test_make_closure_unknown_lambda_rejected(self) -> None:
        gen = IDGenerator()
        program = IrProgram(entry_label="main")
        program.add_instruction(IrInstruction(IrOp.LABEL, [_label("main")], id=-1))
        program.add_instruction(
            IrInstruction(
                IrOp.MAKE_CLOSURE,
                [_reg(1), _label("nope"), _imm(0)],
                id=gen.next(),
            )
        )
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))
        with pytest.raises(BEAMBackendError, match="no closure region"):
            lower_ir_to_beam(
                program,
                BEAMBackendConfig(
                    module_name="bad",
                    arity_overrides={"main": 0},
                    # Note: closure_free_var_counts left empty so the
                    # MAKE_CLOSURE target ``nope`` is unknown.
                ),
            )

    def test_make_closure_capture_count_mismatch_rejected(self) -> None:
        gen = IDGenerator()
        program = IrProgram(entry_label="main")
        program.add_instruction(
            IrInstruction(IrOp.LABEL, [_label("_lambda_0")], id=-1)
        )
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LABEL, [_label("main")], id=-1))
        # Says num_captured=2 but only 1 capture operand provided.
        program.add_instruction(
            IrInstruction(
                IrOp.MAKE_CLOSURE,
                [_reg(1), _label("_lambda_0"), _imm(2), _reg(2)],
                id=gen.next(),
            )
        )
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))
        with pytest.raises(BEAMBackendError, match="capture operands"):
            lower_ir_to_beam(
                program,
                BEAMBackendConfig(
                    module_name="bad",
                    arity_overrides={"_lambda_0": 0, "main": 0},
                    closure_free_var_counts={"_lambda_0": 2},
                ),
            )

    def test_closure_free_var_counts_unknown_region_rejected(self) -> None:
        gen = IDGenerator()
        program = IrProgram(entry_label="main")
        program.add_instruction(IrInstruction(IrOp.LABEL, [_label("main")], id=-1))
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))
        with pytest.raises(BEAMBackendError, match="no\\s+callable region"):
            lower_ir_to_beam(
                program,
                BEAMBackendConfig(
                    module_name="bad",
                    arity_overrides={"main": 0},
                    closure_free_var_counts={"_ghost_lambda": 1},
                ),
            )


# ─────────────────────────────────────────────────────────────────────────
# TW03 Phase 3d — heap primitives (cons / symbol / nil) on BEAM
# ─────────────────────────────────────────────────────────────────────────
#
# BEAM is the simplest of the three native backends here: cons cells and
# atoms are first-class BEAM terms with native opcodes (put_list /
# get_hd / get_tl / is_nil / is_nonempty_list / is_atom).  No "runtime
# classes" needed.

class TestHeapOpLowering:
    """Each new heap opcode produces the specific BEAM opcode that
    uniquely identifies its lowering.  Asserts on the emitted opcode
    sequence rather than running real ``erl`` (those tests live in
    ``test_real_erl.py``)."""

    def _make_main_only_program(
        self, instructions: list[IrInstruction],
    ) -> IrProgram:
        gen = IDGenerator()
        program = IrProgram(entry_label="main")
        program.add_instruction(
            IrInstruction(IrOp.LABEL, [_label("main")], id=-1)
        )
        for ins in instructions:
            if ins.id == -1 and ins.opcode is not IrOp.LABEL:
                ins.id = gen.next()
            program.add_instruction(ins)
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))
        return program

    def _opcodes_in(self, instructions) -> list[int]:
        return [instr.opcode for instr in instructions]

    def test_make_cons_emits_test_heap_and_put_list(self) -> None:
        program = self._make_main_only_program([
            IrInstruction(
                IrOp.MAKE_CONS,
                [_reg(1), _reg(2), _reg(3)],
                id=-1,
            ),
        ])
        module = lower_ir_to_beam(
            program, BEAMBackendConfig(
                module_name="m", arity_overrides={"main": 0},
            ),
        )
        opcodes = self._opcodes_in(module.instructions)
        # _OP_TEST_HEAP = 16, _OP_PUT_LIST = 69
        assert 16 in opcodes
        assert 69 in opcodes

    def test_car_emits_get_hd(self) -> None:
        program = self._make_main_only_program([
            IrInstruction(IrOp.CAR, [_reg(1), _reg(2)], id=-1),
        ])
        module = lower_ir_to_beam(
            program, BEAMBackendConfig(
                module_name="m", arity_overrides={"main": 0},
            ),
        )
        # _OP_GET_HD = 162
        assert 162 in self._opcodes_in(module.instructions)

    def test_cdr_emits_get_tl(self) -> None:
        program = self._make_main_only_program([
            IrInstruction(IrOp.CDR, [_reg(1), _reg(2)], id=-1),
        ])
        module = lower_ir_to_beam(
            program, BEAMBackendConfig(
                module_name="m", arity_overrides={"main": 0},
            ),
        )
        # _OP_GET_TL = 163
        assert 163 in self._opcodes_in(module.instructions)

    def test_is_null_emits_is_nil(self) -> None:
        program = self._make_main_only_program([
            IrInstruction(IrOp.IS_NULL, [_reg(1), _reg(2)], id=-1),
        ])
        module = lower_ir_to_beam(
            program, BEAMBackendConfig(
                module_name="m", arity_overrides={"main": 0},
            ),
        )
        # _OP_IS_NIL = 52
        assert 52 in self._opcodes_in(module.instructions)

    def test_is_pair_emits_is_nonempty_list(self) -> None:
        program = self._make_main_only_program([
            IrInstruction(IrOp.IS_PAIR, [_reg(1), _reg(2)], id=-1),
        ])
        module = lower_ir_to_beam(
            program, BEAMBackendConfig(
                module_name="m", arity_overrides={"main": 0},
            ),
        )
        # _OP_IS_NONEMPTY_LIST = 56
        assert 56 in self._opcodes_in(module.instructions)

    def test_is_symbol_emits_is_atom(self) -> None:
        program = self._make_main_only_program([
            IrInstruction(IrOp.IS_SYMBOL, [_reg(1), _reg(2)], id=-1),
        ])
        module = lower_ir_to_beam(
            program, BEAMBackendConfig(
                module_name="m", arity_overrides={"main": 0},
            ),
        )
        # _OP_IS_ATOM = 48
        assert 48 in self._opcodes_in(module.instructions)

    def test_make_symbol_interns_atom(self) -> None:
        program = self._make_main_only_program([
            IrInstruction(
                IrOp.MAKE_SYMBOL, [_reg(1), _label("foo")], id=-1,
            ),
        ])
        module = lower_ir_to_beam(
            program, BEAMBackendConfig(
                module_name="m", arity_overrides={"main": 0},
            ),
        )
        # The atom name "foo" is in the atom table.
        assert "foo" in module.atoms

    def test_load_nil_emits_move_atom_zero(self) -> None:
        program = self._make_main_only_program([
            IrInstruction(IrOp.LOAD_NIL, [_reg(1)], id=-1),
        ])
        module = lower_ir_to_beam(
            program, BEAMBackendConfig(
                module_name="m", arity_overrides={"main": 0},
            ),
        )
        # _OP_MOVE = 64.  Plenty of moves in a typical module
        # (entry/exit shuffle) — just check at least one move
        # references atom 0 (nil) as the source operand.
        moves = [
            i for i in module.instructions if i.opcode == 64
        ]
        # BEAMTag.A is the atom-tagged operand.
        assert any(
            o.tag is BEAMTag.A and o.value == 0
            for instr in moves
            for o in instr.operands
        )

    def test_make_cons_arity_validation(self) -> None:
        program = self._make_main_only_program([
            IrInstruction(IrOp.MAKE_CONS, [_reg(1)], id=-1),
        ])
        with pytest.raises(BEAMBackendError, match="MAKE_CONS expects"):
            lower_ir_to_beam(
                program, BEAMBackendConfig(
                    module_name="m", arity_overrides={"main": 0},
                ),
            )

    def test_load_nil_arity_validation(self) -> None:
        program = self._make_main_only_program([
            IrInstruction(IrOp.LOAD_NIL, [_reg(1), _reg(2)], id=-1),
        ])
        with pytest.raises(BEAMBackendError, match="LOAD_NIL expects"):
            lower_ir_to_beam(
                program, BEAMBackendConfig(
                    module_name="m", arity_overrides={"main": 0},
                ),
            )
