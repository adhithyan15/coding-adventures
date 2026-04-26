"""Tests for the Intel 4004 codegen (``intel4004_backend.codegen`` and
``intel4004_backend.ir``).

History: these tests were originally part of ``tetrad-jit``'s suite,
then moved to ``tetrad-runtime`` when ``tetrad-jit`` was retired,
and now live here as part of the ``intel4004-backend`` package
extraction.  They exercise:

- ``ir.evaluate_op`` — abstract-evaluation helper used by the
  constant-folding optimiser.
- ``codegen.codegen`` — IRInstr → 4004 abstract-assembly → binary.
- ``codegen.run_on_4004`` — load a 4004 binary into
  ``intel4004-simulator`` and return the u8 accumulator.

End-to-end: ``codegen(ir)`` → ``run_on_4004(binary, args)`` should
produce the value the IR represents.
"""

from __future__ import annotations

from intel4004_backend import (
    IRInstr,
    codegen,
    evaluate_op,
    run_on_4004,
)

# ---------------------------------------------------------------------------
# evaluate_op — abstract-eval helper for constant folding
# ---------------------------------------------------------------------------


class TestEvaluateOp:
    def test_add_wrap(self) -> None:
        assert evaluate_op("add", 200, 100) == 44

    def test_sub_wrap(self) -> None:
        assert evaluate_op("sub", 3, 10) == 249

    def test_mul_wrap(self) -> None:
        assert evaluate_op("mul", 20, 20) == 144

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
# codegen — IRInstr → 4004 binary, end-to-end via the simulator
# ---------------------------------------------------------------------------


class TestCodegen:
    @staticmethod
    def _ir_const_ret(n: int) -> list[IRInstr]:
        return [
            IRInstr("const", "v0", [n], "u8"),
            IRInstr("ret",   None, ["v0"], "u8"),
        ]

    def test_const_return_zero(self) -> None:
        binary = codegen(self._ir_const_ret(0))
        assert binary is not None
        assert run_on_4004(binary, []) == 0

    def test_const_return_42(self) -> None:
        binary = codegen(self._ir_const_ret(42))
        assert binary is not None
        assert run_on_4004(binary, []) == 42

    def test_const_return_255(self) -> None:
        binary = codegen(self._ir_const_ret(255))
        assert binary is not None
        assert run_on_4004(binary, []) == 255

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
        assert run_on_4004(binary, [3, 10]) == 249

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

    def test_cmp_lt(self) -> None:
        ir = [
            IRInstr("param",  "v0", [0], "u8"),
            IRInstr("param",  "v1", [1], "u8"),
            IRInstr("cmp_lt", "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",    None, ["v2"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert run_on_4004(binary, [3, 10]) == 1
        assert run_on_4004(binary, [10, 3]) == 0
        assert run_on_4004(binary, [5, 5]) == 0

    def test_cmp_le(self) -> None:
        ir = [
            IRInstr("param",  "v0", [0], "u8"),
            IRInstr("param",  "v1", [1], "u8"),
            IRInstr("cmp_le", "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",    None, ["v2"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert run_on_4004(binary, [3, 10]) == 1
        assert run_on_4004(binary, [5, 5]) == 1
        assert run_on_4004(binary, [10, 3]) == 0

    def test_cmp_gt(self) -> None:
        ir = [
            IRInstr("param",  "v0", [0], "u8"),
            IRInstr("param",  "v1", [1], "u8"),
            IRInstr("cmp_gt", "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",    None, ["v2"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert run_on_4004(binary, [10, 3]) == 1
        assert run_on_4004(binary, [3, 10]) == 0
        assert run_on_4004(binary, [5, 5]) == 0

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
        assert run_on_4004(binary, [5, 5]) == 1
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
        assert run_on_4004(binary, [5, 5]) == 1
        assert run_on_4004(binary, [5, 6]) == 0
        assert run_on_4004(binary, [0, 0]) == 1
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
        assert run_on_4004(binary, [0]) == 0

    def test_jmp_unconditional(self) -> None:
        ir = [
            IRInstr("const", "v0", [1], "u8"),
            IRInstr("jmp",   None, ["done"], ""),
            IRInstr("const", "v0", [99], "u8"),   # unreachable
            IRInstr("label", None, ["done"], ""),
            IRInstr("ret",   None, ["v0"], "u8"),
        ]
        binary = codegen(ir)
        assert binary is not None
        assert run_on_4004(binary, []) == 1

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
        assert run_on_4004(binary, []) == 7

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
        assert run_on_4004(binary, []) == 99

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
            IRInstr("param", "v0", [0], "u8"),
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
# Deopt — codegen returns None for ops it does not yet support
# ---------------------------------------------------------------------------


class TestCodegenDeopt:
    """Operations the 4004 codegen explicitly does not support trigger
    a graceful deopt — ``codegen()`` returns ``None`` instead of raising.
    The Intel4004Backend then lets jit-core fall back to interpretation."""

    def test_mul_deopts(self) -> None:
        ir = [
            IRInstr("param", "v0", [0], "u8"),
            IRInstr("param", "v1", [1], "u8"),
            IRInstr("mul",   "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",   None, ["v2"], "u8"),
        ]
        assert codegen(ir) is None

    def test_div_deopts(self) -> None:
        ir = [
            IRInstr("param", "v0", [0], "u8"),
            IRInstr("param", "v1", [1], "u8"),
            IRInstr("div",   "v2", ["v0", "v1"], "u8"),
            IRInstr("ret",   None, ["v2"], "u8"),
        ]
        assert codegen(ir) is None

    def test_call_deopts(self) -> None:
        ir = [
            IRInstr("call", "v0", ["other"], "u8"),
            IRInstr("ret",  None, ["v0"], "u8"),
        ]
        assert codegen(ir) is None
