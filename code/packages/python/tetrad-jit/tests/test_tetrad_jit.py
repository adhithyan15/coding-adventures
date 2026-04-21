"""Tests for the Tetrad JIT compiler — Intel 4004 backend (TET05).

Test organisation:
    TestIRTranslation   — bytecode → IR (translate.py)
    TestConstantFold    — constant folding pass (passes.py)
    TestDCE             — dead code elimination (passes.py)
    TestCodegen         — IR → 4004 binary; run on simulator (codegen_4004.py)
    TestEndToEnd        — full pipeline via TetradJIT public API
    TestJITCache        — cache.py unit tests
    TestDeopt           — deopt paths (unsupported ops return False / None)
"""

from __future__ import annotations

import pytest
from tetrad_compiler import compile_program
from tetrad_compiler.bytecode import CodeObject, Instruction, Op
from tetrad_vm import TetradVM

from tetrad_jit import TetradJIT
from tetrad_jit.cache import JITCache, JITCacheEntry
from tetrad_jit.codegen_4004 import codegen, run_on_4004
from tetrad_jit.ir import IRInstr, evaluate_op
from tetrad_jit.passes import constant_fold, dead_code_eliminate
from tetrad_jit.translate import TranslationError, translate

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_code(name: str, params: list[str], instrs: list[Instruction]) -> CodeObject:
    """Build a minimal CodeObject for testing."""
    return CodeObject(
        name=name,
        params=params,
        instructions=instrs,
        register_count=len(params),
    )


def _simple_add_code() -> CodeObject:
    """fn add(a, b) { return a + b; } — typed path, no slot."""
    return _make_code("add", ["a", "b"], [
        Instruction(Op.LDA_REG, [0]),   # acc = a (from R[0])
        Instruction(Op.STA_REG, [2]),   # R[2] = acc
        Instruction(Op.LDA_REG, [1]),   # acc = b (from R[1])
        Instruction(Op.STA_REG, [3]),   # R[3] = acc
        Instruction(Op.LDA_REG, [2]),   # acc = a
        Instruction(Op.ADD, [3]),       # acc = a + b (typed: 1 operand)
        Instruction(Op.RET, []),
    ])


def _compile_fn(source: str, fn_name: str) -> CodeObject:
    """Compile a Tetrad source, return the named function's CodeObject."""
    code = compile_program(source)
    for fn in code.functions:
        if fn.name == fn_name:
            return fn
    raise ValueError(f"function {fn_name!r} not found")


# ---------------------------------------------------------------------------
# TestIRTranslation
# ---------------------------------------------------------------------------


class TestIRTranslation:
    def test_lda_imm_emits_const(self) -> None:
        code = _make_code("f", [], [
            Instruction(Op.LDA_IMM, [42]),
            Instruction(Op.RET, []),
        ])
        ir = translate(code)
        ops = [i.op for i in ir]
        assert "const" in ops
        assert "ret" in ops

    def test_lda_zero_emits_const_zero(self) -> None:
        code = _make_code("f", [], [
            Instruction(Op.LDA_ZERO, []),
            Instruction(Op.RET, []),
        ])
        ir = translate(code)
        consts = [i for i in ir if i.op == "const"]
        assert len(consts) == 1
        assert consts[0].srcs[0] == 0

    def test_params_preloaded(self) -> None:
        code = _make_code("f", ["a", "b"], [
            Instruction(Op.RET, []),
        ])
        ir = translate(code)
        params = [i for i in ir if i.op == "param"]
        assert len(params) == 2
        assert params[0].srcs[0] == 0
        assert params[1].srcs[0] == 1

    def test_sta_var_emits_store_var(self) -> None:
        code = _make_code("f", [], [
            Instruction(Op.LDA_IMM, [7]),
            Instruction(Op.STA_VAR, [0]),
            Instruction(Op.RET, []),
        ])
        ir = translate(code)
        stores = [i for i in ir if i.op == "store_var"]
        assert len(stores) == 1
        assert stores[0].srcs[0] == 0   # var index

    def test_lda_var_emits_load_var(self) -> None:
        code = _make_code("f", [], [
            Instruction(Op.LDA_IMM, [5]),
            Instruction(Op.STA_VAR, [0]),
            Instruction(Op.LDA_VAR, [0]),
            Instruction(Op.RET, []),
        ])
        ir = translate(code)
        loads = [i for i in ir if i.op == "load_var"]
        assert len(loads) == 1

    def test_add_emits_add_with_u8_type(self) -> None:
        code = _simple_add_code()
        ir = translate(code)
        adds = [i for i in ir if i.op == "add"]
        assert len(adds) == 1
        assert adds[0].ty == "u8"

    def test_add_with_slot_emits_unknown_type(self) -> None:
        code = _make_code("f", ["a"], [
            Instruction(Op.LDA_REG, [0]),
            Instruction(Op.STA_REG, [2]),
            Instruction(Op.LDA_REG, [0]),
            Instruction(Op.ADD, [2, 0]),   # 2 operands → untyped path
            Instruction(Op.RET, []),
        ])
        ir = translate(code)
        adds = [i for i in ir if i.op == "add"]
        assert len(adds) == 1
        assert adds[0].ty == "unknown"

    def test_jmp_emits_jmp_with_label(self) -> None:
        # Simple infinite loop: JMP −1 (back to itself)
        code = _make_code("f", [], [
            Instruction(Op.LDA_IMM, [0]),
            Instruction(Op.JMP, [-2]),   # offset −2 → target = 1+(-2)+1 = 0
            Instruction(Op.RET, []),
        ])
        ir = translate(code)
        labels = [i for i in ir if i.op == "label"]
        jumps = [i for i in ir if i.op == "jmp"]
        assert len(labels) >= 1
        assert len(jumps) == 1
        assert isinstance(jumps[0].srcs[0], str)

    def test_jz_emits_jz(self) -> None:
        code = _make_code("f", [], [
            Instruction(Op.LDA_IMM, [1]),
            Instruction(Op.JZ, [1]),     # target = instruction 2+1=3
            Instruction(Op.LDA_IMM, [0]),
            Instruction(Op.RET, []),
        ])
        ir = translate(code)
        jz = [i for i in ir if i.op == "jz"]
        assert len(jz) == 1

    def test_ret_emits_ret(self) -> None:
        code = _make_code("f", [], [
            Instruction(Op.LDA_IMM, [99]),
            Instruction(Op.RET, []),
        ])
        ir = translate(code)
        rets = [i for i in ir if i.op == "ret"]
        assert len(rets) == 1

    def test_halt_stops_translation(self) -> None:
        code = _make_code("f", [], [
            Instruction(Op.LDA_IMM, [1]),
            Instruction(Op.HALT, []),
            Instruction(Op.LDA_IMM, [2]),   # should not appear
        ])
        ir = translate(code)
        consts = [i for i in ir if i.op == "const"]
        assert len(consts) == 1
        assert consts[0].srcs[0] == 1

    def test_add_imm_emits_add(self) -> None:
        code = _make_code("f", [], [
            Instruction(Op.LDA_IMM, [10]),
            Instruction(Op.ADD_IMM, [5]),
            Instruction(Op.RET, []),
        ])
        ir = translate(code)
        adds = [i for i in ir if i.op == "add"]
        assert len(adds) == 1
        assert adds[0].srcs[1] == 5   # immediate is a src int

    def test_sub_imm_emits_sub(self) -> None:
        code = _make_code("f", [], [
            Instruction(Op.LDA_IMM, [20]),
            Instruction(Op.SUB_IMM, [3]),
            Instruction(Op.RET, []),
        ])
        ir = translate(code)
        subs = [i for i in ir if i.op == "sub"]
        assert len(subs) == 1

    def test_cmp_ops_emitted(self) -> None:
        for opcode, expected_ir in [
            (Op.EQ,  "cmp_eq"),
            (Op.NEQ, "cmp_ne"),
            (Op.LT,  "cmp_lt"),
            (Op.LTE, "cmp_le"),
            (Op.GT,  "cmp_gt"),
            (Op.GTE, "cmp_ge"),
        ]:
            code = _make_code("f", ["a"], [
                Instruction(Op.LDA_REG, [0]),
                Instruction(Op.STA_REG, [2]),
                Instruction(Op.LDA_REG, [0]),
                Instruction(opcode, [2]),
                Instruction(Op.RET, []),
            ])
            ir = translate(code)
            cmps = [i for i in ir if i.op == expected_ir]
            assert len(cmps) == 1, f"expected {expected_ir} for opcode {opcode}"

    def test_logical_not_emitted(self) -> None:
        code = _make_code("f", [], [
            Instruction(Op.LDA_IMM, [5]),
            Instruction(Op.LOGICAL_NOT, []),
            Instruction(Op.RET, []),
        ])
        ir = translate(code)
        lnots = [i for i in ir if i.op == "logical_not"]
        assert len(lnots) == 1

    def test_io_in_emitted(self) -> None:
        code = _make_code("f", [], [
            Instruction(Op.IO_IN, []),
            Instruction(Op.RET, []),
        ])
        ir = translate(code)
        io = [i for i in ir if i.op == "io_in"]
        assert len(io) == 1

    def test_io_out_emitted(self) -> None:
        code = _make_code("f", [], [
            Instruction(Op.LDA_IMM, [7]),
            Instruction(Op.IO_OUT, []),
            Instruction(Op.RET, []),
        ])
        ir = translate(code)
        io = [i for i in ir if i.op == "io_out"]
        assert len(io) == 1

    def test_call_emitted(self) -> None:
        code = _make_code("f", [], [
            Instruction(Op.CALL, [0, 0, 0]),
            Instruction(Op.RET, []),
        ])
        ir = translate(code)
        calls = [i for i in ir if i.op == "call"]
        assert len(calls) == 1

    def test_unknown_opcode_raises(self) -> None:
        code = _make_code("f", [], [
            Instruction(0xBE, []),   # invalid opcode
        ])
        with pytest.raises(TranslationError):
            translate(code)

    def test_ssa_variables_are_unique(self) -> None:
        code = _make_code("f", [], [
            Instruction(Op.LDA_IMM, [1]),
            Instruction(Op.LDA_IMM, [2]),
            Instruction(Op.RET, []),
        ])
        ir = translate(code)
        dsts = [i.dst for i in ir if i.dst is not None]
        assert len(dsts) == len(set(dsts)), "SSA variables must be unique"


# ---------------------------------------------------------------------------
# TestConstantFold
# ---------------------------------------------------------------------------


class TestConstantFold:
    def test_add_two_consts(self) -> None:
        ir = [
            IRInstr("const", "v0", [10], "u8"),
            IRInstr("const", "v1", [5],  "u8"),
            IRInstr("add",   "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",   None, ["v2"], "u8"),
        ]
        folded = constant_fold(ir)
        consts = [i for i in folded if i.op == "const"]
        # v2 should be folded to const 15
        v2_const = [c for c in consts if c.dst == "v2"]
        assert len(v2_const) == 1
        assert v2_const[0].srcs[0] == 15

    def test_add_with_u8_wrap(self) -> None:
        ir = [
            IRInstr("const", "v0", [200], "u8"),
            IRInstr("const", "v1", [100], "u8"),
            IRInstr("add",   "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",   None, ["v2"], "u8"),
        ]
        folded = constant_fold(ir)
        v2 = [i for i in folded if i.dst == "v2"]
        assert v2[0].srcs[0] == 44   # (200+100)%256 = 44

    def test_sub_folded(self) -> None:
        ir = [
            IRInstr("const", "v0", [20], "u8"),
            IRInstr("const", "v1", [7],  "u8"),
            IRInstr("sub",   "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",   None, ["v2"], "u8"),
        ]
        folded = constant_fold(ir)
        v2 = [i for i in folded if i.dst == "v2"]
        assert v2[0].srcs[0] == 13

    def test_cmp_lt_folded(self) -> None:
        ir = [
            IRInstr("const",  "v0", [3],  "u8"),
            IRInstr("const",  "v1", [10], "u8"),
            IRInstr("cmp_lt", "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",    None, ["v2"], "u8"),
        ]
        folded = constant_fold(ir)
        v2 = [i for i in folded if i.dst == "v2"]
        assert v2[0].srcs[0] == 1   # 3 < 10 = True = 1

    def test_cmp_lt_false_folded(self) -> None:
        ir = [
            IRInstr("const",  "v0", [10], "u8"),
            IRInstr("const",  "v1", [3],  "u8"),
            IRInstr("cmp_lt", "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",    None, ["v2"], "u8"),
        ]
        folded = constant_fold(ir)
        v2 = [i for i in folded if i.dst == "v2"]
        assert v2[0].srcs[0] == 0   # 10 < 3 = False = 0

    def test_not_folded(self) -> None:
        ir = [
            IRInstr("const", "v0", [0x0F], "u8"),
            IRInstr("not",   "v1", ["v0"],   "u8"),
            IRInstr("ret",   None, ["v1"], "u8"),
        ]
        folded = constant_fold(ir)
        v1 = [i for i in folded if i.dst == "v1"]
        assert v1[0].srcs[0] == 0xF0

    def test_logical_not_zero(self) -> None:
        ir = [
            IRInstr("const",       "v0", [0], "u8"),
            IRInstr("logical_not", "v1", ["v0"], "u8"),
            IRInstr("ret",         None, ["v1"], "u8"),
        ]
        folded = constant_fold(ir)
        v1 = [i for i in folded if i.dst == "v1"]
        assert v1[0].srcs[0] == 1

    def test_logical_not_nonzero(self) -> None:
        ir = [
            IRInstr("const",       "v0", [5], "u8"),
            IRInstr("logical_not", "v1", ["v0"], "u8"),
            IRInstr("ret",         None, ["v1"], "u8"),
        ]
        folded = constant_fold(ir)
        v1 = [i for i in folded if i.dst == "v1"]
        assert v1[0].srcs[0] == 0

    def test_unknown_source_not_folded(self) -> None:
        ir = [
            IRInstr("param", "v0", [0], "u8"),     # runtime value
            IRInstr("const", "v1", [5],  "u8"),
            IRInstr("add",   "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",   None, ["v2"], "u8"),
        ]
        folded = constant_fold(ir)
        v2 = [i for i in folded if i.dst == "v2"]
        assert v2[0].op == "add"   # not folded

    def test_immediate_int_source_folded(self) -> None:
        # add v0, 5  where v0 = const 10 and 5 is an int src
        ir = [
            IRInstr("const", "v0", [10], "u8"),
            IRInstr("add",   "v1", ["v0", 5],  "u8"),
            IRInstr("ret",   None, ["v1"], "u8"),
        ]
        folded = constant_fold(ir)
        v1 = [i for i in folded if i.dst == "v1"]
        assert v1[0].srcs[0] == 15   # folded


# ---------------------------------------------------------------------------
# TestDCE
# ---------------------------------------------------------------------------


class TestDCE:
    def test_unused_const_removed(self) -> None:
        ir = [
            IRInstr("const", "v0", [42], "u8"),    # never used
            IRInstr("const", "v1", [7],  "u8"),
            IRInstr("ret",   None, ["v1"], "u8"),
        ]
        result = dead_code_eliminate(ir)
        dsts = [i.dst for i in result]
        assert "v0" not in dsts
        assert "v1" in dsts

    def test_used_const_kept(self) -> None:
        ir = [
            IRInstr("const", "v0", [5], "u8"),
            IRInstr("ret",   None, ["v0"], "u8"),
        ]
        result = dead_code_eliminate(ir)
        assert any(i.dst == "v0" for i in result)

    def test_side_effect_op_always_kept(self) -> None:
        ir = [
            IRInstr("const",     "v0", [5], "u8"),
            IRInstr("store_var", None, [0, "v0"], "u8"),
            IRInstr("ret",       None, ["v0"], "u8"),
        ]
        result = dead_code_eliminate(ir)
        ops = [i.op for i in result]
        assert "store_var" in ops

    def test_label_always_kept(self) -> None:
        ir = [
            IRInstr("label", None, ["lbl_0"], ""),
            IRInstr("const", "v0", [1], "u8"),
            IRInstr("ret",   None, ["v0"], "u8"),
        ]
        result = dead_code_eliminate(ir)
        lbls = [i for i in result if i.op == "label"]
        assert len(lbls) == 1

    def test_param_always_kept(self) -> None:
        ir = [
            IRInstr("param", "v0", [0], "u8"),
            IRInstr("const", "v1", [0], "u8"),   # unused
            IRInstr("ret",   None, ["v0"], "u8"),
        ]
        result = dead_code_eliminate(ir)
        assert any(i.op == "param" for i in result)

    def test_transitive_dce(self) -> None:
        ir = [
            IRInstr("const", "v0", [1], "u8"),
            IRInstr("const", "v1", [2], "u8"),
            IRInstr("add",   "v2", ["v0", "v1"], "u8"),  # v2 unused
            IRInstr("const", "v3", [9], "u8"),
            IRInstr("ret",   None, ["v3"], "u8"),
        ]
        result = dead_code_eliminate(ir)
        dsts = [i.dst for i in result if i.dst]
        # v0, v1, v2 should all be removed
        assert "v2" not in dsts


# ---------------------------------------------------------------------------
# TestEvaluateOp
# ---------------------------------------------------------------------------


class TestEvaluateOp:
    def test_add_wrap(self) -> None:
        assert evaluate_op("add", 200, 100) == 44

    def test_sub_wrap(self) -> None:
        assert evaluate_op("sub", 3, 10) == 249   # (3-10)%256

    def test_mul_wrap(self) -> None:
        assert evaluate_op("mul", 20, 20) == 144  # 400%256

    def test_div_zero(self) -> None:
        assert evaluate_op("div", 10, 0) == 0

    def test_mod_zero(self) -> None:
        assert evaluate_op("mod", 10, 0) == 0

    def test_and(self) -> None:
        assert evaluate_op("and", 0xF0, 0x0F) == 0x00

    def test_or(self) -> None:
        assert evaluate_op("or", 0xF0, 0x0F) == 0xFF

    def test_xor(self) -> None:
        assert evaluate_op("xor", 0xFF, 0x0F) == 0xF0

    def test_shl_wrap(self) -> None:
        assert evaluate_op("shl", 0xFF, 1) == 0xFE

    def test_shr(self) -> None:
        assert evaluate_op("shr", 0xFF, 4) == 0x0F

    def test_cmp_eq_true(self) -> None:
        assert evaluate_op("cmp_eq", 5, 5) == 1

    def test_cmp_eq_false(self) -> None:
        assert evaluate_op("cmp_eq", 5, 6) == 0

    def test_unknown_op(self) -> None:
        assert evaluate_op("bogus", 1, 2) == 0


# ---------------------------------------------------------------------------
# TestCodegen (IR → 4004 binary; runs on intel4004-simulator)
# ---------------------------------------------------------------------------


class TestCodegen:
    def _ir_const_ret(self, n: int) -> list[IRInstr]:
        return [
            IRInstr("const", "v0", [n], "u8"),
            IRInstr("ret",   None, ["v0"], "u8"),
        ]

    def test_const_return_zero(self) -> None:
        binary = codegen(self._ir_const_ret(0))
        assert binary is not None
        result = run_on_4004(binary, [])
        assert result == 0

    def test_const_return_42(self) -> None:
        binary = codegen(self._ir_const_ret(42))
        assert binary is not None
        result = run_on_4004(binary, [])
        assert result == 42

    def test_const_return_255(self) -> None:
        binary = codegen(self._ir_const_ret(255))
        assert binary is not None
        result = run_on_4004(binary, [])
        assert result == 255

    def test_add_two_args(self) -> None:
        ir = [
            IRInstr("param", "v0", [0], "u8"),
            IRInstr("param", "v1", [1], "u8"),
            IRInstr("add",   "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",   None, ["v2"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert run_on_4004(binary, [3, 4]) == 7
        assert run_on_4004(binary, [0, 0]) == 0
        assert run_on_4004(binary, [200, 100]) == 44   # u8 wrap

    def test_sub_two_args(self) -> None:
        ir = [
            IRInstr("param", "v0", [0], "u8"),
            IRInstr("param", "v1", [1], "u8"),
            IRInstr("sub",   "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",   None, ["v2"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert run_on_4004(binary, [10, 3]) == 7
        assert run_on_4004(binary, [3, 10]) == 249   # (3-10)%256 = 249

    def test_add_immediate(self) -> None:
        ir = [
            IRInstr("param", "v0", [0], "u8"),
            IRInstr("add",   "v1", ["v0", 5], "u8"),
            IRInstr("ret",   None, ["v1"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert run_on_4004(binary, [10]) == 15
        assert run_on_4004(binary, [255]) == 4   # (255+5)%256 = 4

    def test_cmp_lt_true(self) -> None:
        ir = [
            IRInstr("param",  "v0", [0], "u8"),
            IRInstr("param",  "v1", [1], "u8"),
            IRInstr("cmp_lt", "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",    None, ["v2"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert run_on_4004(binary, [3, 10]) == 1   # 3 < 10
        assert run_on_4004(binary, [10, 3]) == 0   # 10 < 3 = false
        assert run_on_4004(binary, [5, 5])  == 0   # 5 < 5 = false

    def test_cmp_le(self) -> None:
        ir = [
            IRInstr("param",  "v0", [0], "u8"),
            IRInstr("param",  "v1", [1], "u8"),
            IRInstr("cmp_le", "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",    None, ["v2"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert run_on_4004(binary, [3, 10]) == 1   # 3 <= 10
        assert run_on_4004(binary, [5, 5])  == 1   # 5 <= 5
        assert run_on_4004(binary, [10, 3]) == 0   # 10 <= 3 = false

    def test_cmp_gt(self) -> None:
        ir = [
            IRInstr("param",  "v0", [0], "u8"),
            IRInstr("param",  "v1", [1], "u8"),
            IRInstr("cmp_gt", "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",    None, ["v2"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert run_on_4004(binary, [10, 3]) == 1   # 10 > 3
        assert run_on_4004(binary, [3, 10]) == 0
        assert run_on_4004(binary, [5, 5])  == 0

    def test_cmp_ge(self) -> None:
        ir = [
            IRInstr("param",  "v0", [0], "u8"),
            IRInstr("param",  "v1", [1], "u8"),
            IRInstr("cmp_ge", "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",    None, ["v2"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert run_on_4004(binary, [10, 3]) == 1
        assert run_on_4004(binary, [5, 5])  == 1
        assert run_on_4004(binary, [3, 10]) == 0

    def test_cmp_eq(self) -> None:
        ir = [
            IRInstr("param",  "v0", [0], "u8"),
            IRInstr("param",  "v1", [1], "u8"),
            IRInstr("cmp_eq", "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",    None, ["v2"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert run_on_4004(binary, [5, 5])  == 1
        assert run_on_4004(binary, [5, 6])  == 0
        assert run_on_4004(binary, [0, 0])  == 1
        assert run_on_4004(binary, [255, 255]) == 1
        assert run_on_4004(binary, [255, 254]) == 0

    def test_cmp_ne(self) -> None:
        ir = [
            IRInstr("param",  "v0", [0], "u8"),
            IRInstr("param",  "v1", [1], "u8"),
            IRInstr("cmp_ne", "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",    None, ["v2"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert run_on_4004(binary, [5, 5]) == 0
        assert run_on_4004(binary, [5, 6]) == 1

    def test_store_and_load_var(self) -> None:
        ir = [
            IRInstr("param",     "v0", [0], "u8"),
            IRInstr("store_var", None, [0, "v0"], "u8"),
            IRInstr("load_var",  "v1", [0], "u8"),
            IRInstr("ret",       None, ["v1"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert run_on_4004(binary, [77]) == 77
        assert run_on_4004(binary, [0])  == 0

    def test_jmp_unconditional(self) -> None:
        ir = [
            IRInstr("const", "v0", [1], "u8"),
            IRInstr("jmp",   None, ["done"], ""),
            IRInstr("const", "v0", [99], "u8"),   # unreachable; v0 reused SSA-side? just for test
            IRInstr("label", None, ["done"], ""),
            IRInstr("ret",   None, ["v0"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        result = run_on_4004(binary, [])
        assert result == 1   # jmp skips the second const

    def test_jz_taken(self) -> None:
        ir = [
            IRInstr("const", "v0", [0], "u8"),
            IRInstr("jz",    None, ["v0", "zero_branch"], ""),
            IRInstr("const", "v1", [99], "u8"),
            IRInstr("ret",   None, ["v1"], "u8"),
            IRInstr("label", None, ["zero_branch"], ""),
            IRInstr("const", "v2", [7], "u8"),
            IRInstr("ret",   None, ["v2"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert run_on_4004(binary, []) == 7   # jz taken → 7

    def test_jz_not_taken(self) -> None:
        ir = [
            IRInstr("const", "v0", [1], "u8"),
            IRInstr("jz",    None, ["v0", "zero_branch"], ""),
            IRInstr("const", "v1", [99], "u8"),
            IRInstr("ret",   None, ["v1"], "u8"),
            IRInstr("label", None, ["zero_branch"], ""),
            IRInstr("const", "v2", [7], "u8"),
            IRInstr("ret",   None, ["v2"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert run_on_4004(binary, []) == 99   # jz not taken → 99

    def test_jnz_taken(self) -> None:
        ir = [
            IRInstr("const", "v0", [3], "u8"),
            IRInstr("jnz",   None, ["v0", "nonzero"], ""),
            IRInstr("const", "v1", [0], "u8"),
            IRInstr("ret",   None, ["v1"], "u8"),
            IRInstr("label", None, ["nonzero"], ""),
            IRInstr("const", "v2", [42], "u8"),
            IRInstr("ret",   None, ["v2"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert run_on_4004(binary, []) == 42

    def test_jnz_not_taken(self) -> None:
        ir = [
            IRInstr("const", "v0", [0], "u8"),
            IRInstr("jnz",   None, ["v0", "nonzero"], ""),
            IRInstr("const", "v1", [55], "u8"),
            IRInstr("ret",   None, ["v1"], "u8"),
            IRInstr("label", None, ["nonzero"], ""),
            IRInstr("const", "v2", [42], "u8"),
            IRInstr("ret",   None, ["v2"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert run_on_4004(binary, []) == 55

    def test_result_already_in_p0(self) -> None:
        ir = [
            IRInstr("param", "v0", [0], "u8"),   # arg 0 → P0 (already there)
            IRInstr("ret",   None, ["v0"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert run_on_4004(binary, [123]) == 123

    def test_binary_is_bytes(self) -> None:
        ir = [
            IRInstr("const", "v0", [1], "u8"),
            IRInstr("ret",   None, ["v0"], "u8"),
        ]
        binary = codegen(ir)
        assert isinstance(binary, bytes)
        assert len(binary) > 0

    def test_fits_in_one_page(self) -> None:
        ir = [
            IRInstr("const", "v0", [1], "u8"),
            IRInstr("ret",   None, ["v0"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert len(binary) <= 256


# ---------------------------------------------------------------------------
# TestDeopt
# ---------------------------------------------------------------------------


class TestDeopt:
    def test_mul_returns_none(self) -> None:
        ir = [
            IRInstr("param", "v0", [0], "u8"),
            IRInstr("param", "v1", [1], "u8"),
            IRInstr("mul",   "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",   None, ["v2"], "u8"),
        ]
        assert codegen(ir) is None

    def test_div_returns_none(self) -> None:
        ir = [
            IRInstr("param", "v0", [0], "u8"),
            IRInstr("param", "v1", [1], "u8"),
            IRInstr("div",   "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",   None, ["v2"], "u8"),
        ]
        assert codegen(ir) is None

    def test_and_returns_none(self) -> None:
        ir = [
            IRInstr("param", "v0", [0], "u8"),
            IRInstr("param", "v1", [1], "u8"),
            IRInstr("and",   "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",   None, ["v2"], "u8"),
        ]
        assert codegen(ir) is None

    def test_call_returns_none(self) -> None:
        ir = [
            IRInstr("call", "v0", [0], "u8"),
            IRInstr("ret",  None, ["v0"], "u8"),
        ]
        assert codegen(ir) is None

    def test_io_in_returns_none(self) -> None:
        ir = [
            IRInstr("io_in", "v0", [], "u8"),
            IRInstr("ret",   None, ["v0"], "u8"),
        ]
        assert codegen(ir) is None

    def test_too_many_params_returns_none(self) -> None:
        ir = [
            IRInstr("param", "v0", [0], "u8"),
            IRInstr("param", "v1", [1], "u8"),
            IRInstr("param", "v2", [2], "u8"),  # 3rd param → deopt
            IRInstr("ret",   None, ["v0"], "u8"),
        ]
        assert codegen(ir) is None

    def test_too_many_vars_returns_none(self) -> None:
        # P0-P5 = 6 pairs max.  7 vars that are ALL simultaneously live when
        # the 7th is allocated → no pair can be recycled → deopt.
        # If the vars were used before all 7 are defined, liveness recycling
        # would allow reuse and the limit would not be hit.
        ir = [
            IRInstr("const", f"v{i}", [i], "u8") for i in range(7)
        ] + [
            # Each vN is first used here, so all 7 must be live while v6 is
            # being allocated (at IR index 6).
            IRInstr("add", "s0", ["v0", "v1"], "u8"),
            IRInstr("add", "s1", ["v2", "v3"], "u8"),
            IRInstr("add", "s2", ["v4", "v5"], "u8"),
            IRInstr("add", "s3", ["v6", "s0"], "u8"),
            IRInstr("add", "s4", ["s1", "s2"], "u8"),
            IRInstr("add", "s5", ["s3", "s4"], "u8"),
            IRInstr("ret", None, ["s5"], "u8"),
        ]
        assert codegen(ir) is None


# ---------------------------------------------------------------------------
# TestJITCache
# ---------------------------------------------------------------------------


class TestJITCache:
    def test_get_missing_returns_none(self) -> None:
        cache = JITCache()
        assert cache.get("missing") is None

    def test_put_and_get(self) -> None:
        cache = JITCache()
        entry = JITCacheEntry(fn_name="f", binary=b"\x01", param_count=0)
        cache.put(entry)
        result = cache.get("f")
        assert result is not None
        assert result.binary == b"\x01"

    def test_contains(self) -> None:
        cache = JITCache()
        assert "f" not in cache
        cache.put(JITCacheEntry("f", b"\x01", 0))
        assert "f" in cache

    def test_invalidate(self) -> None:
        cache = JITCache()
        cache.put(JITCacheEntry("f", b"\x01", 0))
        cache.invalidate("f")
        assert "f" not in cache

    def test_invalidate_missing_is_noop(self) -> None:
        cache = JITCache()
        cache.invalidate("nope")   # no error

    def test_stats_empty(self) -> None:
        assert JITCache().stats() == {}

    def test_stats_has_entry(self) -> None:
        cache = JITCache()
        cache.put(JITCacheEntry("g", b"\x01\x02", 1, [], 500))
        s = cache.stats()
        assert "g" in s
        assert s["g"]["binary_bytes"] == 2
        assert s["g"]["param_count"] == 1

    def test_now_ns_is_int(self) -> None:
        assert isinstance(JITCache.now_ns(), int)


# ---------------------------------------------------------------------------
# TestEndToEnd  (full pipeline via TetradJIT)
# ---------------------------------------------------------------------------


class TestEndToEnd:
    def _jit(self, source: str) -> TetradJIT:
        code = compile_program(source)
        vm  = TetradVM()
        jit = TetradJIT(vm)
        jit._main_code = code
        return jit

    def test_compile_typed_add(self) -> None:
        source = "fn add(a: u8, b: u8) -> u8 { return a + b; }"
        jit = self._jit(source)
        assert jit.compile("add")
        assert jit.is_compiled("add")

    def test_execute_add(self) -> None:
        source = "fn add(a: u8, b: u8) -> u8 { return a + b; }"
        jit = self._jit(source)
        jit.compile("add")
        assert jit.execute("add", [10, 20]) == 30
        assert jit.execute("add", [200, 100]) == 44   # u8 wrap

    def test_execute_const_return(self) -> None:
        source = "fn answer() -> u8 { return 42; }"
        jit = self._jit(source)
        jit.compile("answer")
        assert jit.execute("answer", []) == 42

    def test_execute_sub(self) -> None:
        source = "fn sub(a: u8, b: u8) -> u8 { return a - b; }"
        jit = self._jit(source)
        jit.compile("sub")
        assert jit.execute("sub", [10, 3]) == 7
        assert jit.execute("sub", [3, 10]) == 249   # (3-10)%256

    def test_execute_identity(self) -> None:
        source = "fn id(n: u8) -> u8 { return n; }"
        jit = self._jit(source)
        jit.compile("id")
        assert jit.execute("id", [77]) == 77
        assert jit.execute("id", [0])  == 0

    def test_deopt_for_mul(self) -> None:
        source = "fn mul(a: u8, b: u8) -> u8 { return a * b; }"
        jit = self._jit(source)
        # compile should fail (deopt) but not raise
        result = jit.compile("mul")
        assert not result
        assert not jit.is_compiled("mul")

    def test_is_compiled_false_before_compile(self) -> None:
        source = "fn add(a: u8, b: u8) -> u8 { return a + b; }"
        jit = self._jit(source)
        assert not jit.is_compiled("add")

    def test_compile_unknown_function_returns_false(self) -> None:
        source = "fn add(a: u8, b: u8) -> u8 { return a + b; }"
        jit = self._jit(source)
        assert not jit.compile("nonexistent")

    def test_cache_stats_empty_before_compile(self) -> None:
        source = "fn add(a: u8, b: u8) -> u8 { return a + b; }"
        jit = self._jit(source)
        assert jit.cache_stats() == {}

    def test_cache_stats_after_compile(self) -> None:
        source = "fn add(a: u8, b: u8) -> u8 { return a + b; }"
        jit = self._jit(source)
        jit.compile("add")
        stats = jit.cache_stats()
        assert "add" in stats
        assert stats["add"]["param_count"] == 2

    def test_dump_ir_before_compile(self) -> None:
        source = "fn add(a: u8, b: u8) -> u8 { return a + b; }"
        jit = self._jit(source)
        dump = jit.dump_ir("add")
        assert "not compiled" in dump.lower()

    def test_dump_ir_after_compile(self) -> None:
        source = "fn add(a: u8, b: u8) -> u8 { return a + b; }"
        jit = self._jit(source)
        jit.compile("add")
        dump = jit.dump_ir("add")
        assert len(dump) > 0
        assert "ret" in dump

    def test_execute_with_jit_compiles_fully_typed(self) -> None:
        source = """
fn add(a: u8, b: u8) -> u8 { return a + b; }
fn main() -> u8 { return add(3, 4); }
"""
        code = compile_program(source)
        vm = TetradVM()
        jit = TetradJIT(vm)
        jit.execute_with_jit(code)
        # FULLY_TYPED functions should have been compiled before the interpreter ran
        assert jit.is_compiled("add")

    def test_execute_with_jit_returns_correct_value(self) -> None:
        source = """
fn add(a: u8, b: u8) -> u8 { return a + b; }
fn main() -> u8 { return add(10, 20); }
"""
        code = compile_program(source)
        vm = TetradVM()
        jit = TetradJIT(vm)
        result = jit.execute_with_jit(code)
        assert result == 30

    def test_cmp_lt_function(self) -> None:
        source = "fn lt(a: u8, b: u8) -> u8 { return a < b; }"
        jit = self._jit(source)
        jit.compile("lt")
        assert jit.execute("lt", [3, 10]) == 1
        assert jit.execute("lt", [10, 3]) == 0
        assert jit.execute("lt", [5, 5])  == 0

    def test_compare_with_branch_compiles_via_liveness_recycling(self) -> None:
        # compare() generates 7 SSA variables (v0..v6).  With liveness-based
        # register recycling, variables dead on each branch share pairs, so
        # the total live simultaneously stays ≤ 6 and compilation succeeds.
        source = """
fn compare(a: u8, b: u8) -> u8 {
  if a < b { return 1; }
  return 0;
}
"""
        jit = self._jit(source)
        assert jit.compile("compare"), "expected liveness recycling to allow compile"
        assert jit.execute("compare", [3, 10]) == 1
        assert jit.execute("compare", [10, 3]) == 0
        assert jit.execute("compare", [5, 5])  == 0
        assert jit.execute("compare", [0, 255]) == 1
        assert jit.execute("compare", [255, 0]) == 0

    def test_add_boundary_values(self) -> None:
        source = "fn add(a: u8, b: u8) -> u8 { return a + b; }"
        jit = self._jit(source)
        jit.compile("add")
        assert jit.execute("add", [0, 0])     == 0
        assert jit.execute("add", [255, 1])   == 0     # wrap to 0
        assert jit.execute("add", [127, 128]) == 255
        assert jit.execute("add", [128, 128]) == 0     # wrap
