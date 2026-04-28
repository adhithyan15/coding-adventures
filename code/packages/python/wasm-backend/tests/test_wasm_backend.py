"""Tests for WASMBackend — end-to-end CIR → WASM pipeline.

Test structure
--------------

Section 1: Unit tests — WASMBackend in isolation (no Tetrad frontend).
    Craft CIRInstr lists directly, compile, run, and check results.
    This is fast and dependency-light.

Section 2: Integration tests — full Tetrad → WASM pipeline.
    Start with Tetrad source, run through:
      tetrad-runtime   (compile_to_iir)
      jit-core         (specialise, forced AOT with min_observations=0)
      cir-to-compiler-ir  (lower_cir_to_ir_program, via WASMBackend)
      ir-to-wasm-compiler (WASMBackend.compile)
      wasm-runtime     (WASMBackend.run)

    This demonstrates the complete pipeline described in the project goals.

Section 3: BackendProtocol compatibility tests.
    Verify WASMBackend satisfies the structural BackendProtocol check.

Section 4: Helper — _collect_cir_registers.
    Unit tests for the internal register-map helper.

Section 5: End-to-end with JITCore.
    Use TetradRuntime.run_with_jit(source, backend=WASMBackend()) to
    show the fully-integrated path.
"""

from __future__ import annotations

from codegen_core import CIRInstr

from wasm_backend import WASMBackend
from wasm_backend.backend import _collect_cir_registers

# ============================================================================
# Shared helpers
# ============================================================================


def _compile_and_run(cir: list[CIRInstr]) -> object:
    """Compile CIR → WASM bytes → run; return the WASM result."""
    backend = WASMBackend()
    binary = backend.compile(cir)
    assert binary is not None, "WASMBackend.compile() returned None"
    assert len(binary) > 0, "compile() returned empty bytes"
    return backend.run(binary, [])


# ============================================================================
# Section 1: Unit tests — CIR → WASM (no Tetrad frontend)
# ============================================================================


class TestConstantReturn:
    """The simplest possible program: return a constant integer."""

    def test_const_i32_return(self) -> None:
        """ret_i32 with a single constant: result = 42."""
        # CIR: x = 42; return x
        # Pass 1: x → register 0
        # Fixup: ADD_IMM r1, r0, 0  (copy x into scratch before HALT)
        cir = [
            CIRInstr("const_i32", "x", [42], "i32"),
            CIRInstr("ret_i32", None, ["x"], "i32"),
        ]
        result = _compile_and_run(cir)
        assert result == 42

    def test_const_u8_return(self) -> None:
        """Same as above but with u8 type."""
        cir = [
            CIRInstr("const_u8", "x", [200], "u8"),
            CIRInstr("ret_u8", None, ["x"], "u8"),
        ]
        result = _compile_and_run(cir)
        assert result == 200

    def test_const_zero(self) -> None:
        """Edge case: return 0."""
        cir = [
            CIRInstr("const_i32", "r", [0], "i32"),
            CIRInstr("ret_i32", None, ["r"], "i32"),
        ]
        result = _compile_and_run(cir)
        assert result == 0

    def test_const_bool_true(self) -> None:
        """Boolean true (1) returned as i32."""
        cir = [
            CIRInstr("const_bool", "flag", [True], "bool"),
            CIRInstr("ret_bool", None, ["flag"], "bool"),
        ]
        result = _compile_and_run(cir)
        assert result == 1

    def test_const_bool_false(self) -> None:
        """Boolean false (0)."""
        cir = [
            CIRInstr("const_bool", "flag", [False], "bool"),
            CIRInstr("ret_bool", None, ["flag"], "bool"),
        ]
        result = _compile_and_run(cir)
        assert result == 0


class TestReturnVoid:
    """ret_void: no return value, WASM run returns None."""

    def test_ret_void_returns_none(self) -> None:
        cir = [
            CIRInstr("const_i32", "x", [99], "i32"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        backend = WASMBackend()
        binary = backend.compile(cir)
        assert binary is not None
        # Void return → run returns None (empty list from WasmRuntime)
        result = backend.run(binary, [])
        assert result is None


class TestArithmetic:
    """Integer arithmetic: add, sub, mul, div."""

    def test_add_i32(self) -> None:
        """40 + 2 = 42."""
        # CIR emitted by the JIT for "return 40 + 2":
        #   const_i32 t0 [40]
        #   const_i32 t1 [2]
        #   add_i32 result [t0, t1]
        #   ret_i32 None [result]
        # Pass 1: t0→0, t1→1, result→2
        # Fixup: ADD_IMM r1, r2, 0
        cir = [
            CIRInstr("const_i32", "t0", [40], "i32"),
            CIRInstr("const_i32", "t1", [2], "i32"),
            CIRInstr("add_i32", "result", ["t0", "t1"], "i32"),
            CIRInstr("ret_i32", None, ["result"], "i32"),
        ]
        assert _compile_and_run(cir) == 42

    def test_add_u8(self) -> None:
        """u8 addition: 10 + 20 = 30."""
        cir = [
            CIRInstr("const_u8", "a", [10], "u8"),
            CIRInstr("const_u8", "b", [20], "u8"),
            CIRInstr("add_u8", "s", ["a", "b"], "u8"),
            CIRInstr("ret_u8", None, ["s"], "u8"),
        ]
        assert _compile_and_run(cir) == 30

    def test_sub_i32(self) -> None:
        """100 - 58 = 42."""
        cir = [
            CIRInstr("const_i32", "a", [100], "i32"),
            CIRInstr("const_i32", "b", [58], "i32"),
            CIRInstr("sub_i32", "r", ["a", "b"], "i32"),
            CIRInstr("ret_i32", None, ["r"], "i32"),
        ]
        assert _compile_and_run(cir) == 42

    def test_mul_i32(self) -> None:
        """6 * 7 = 42."""
        cir = [
            CIRInstr("const_i32", "a", [6], "i32"),
            CIRInstr("const_i32", "b", [7], "i32"),
            CIRInstr("mul_i32", "r", ["a", "b"], "i32"),
            CIRInstr("ret_i32", None, ["r"], "i32"),
        ]
        assert _compile_and_run(cir) == 42

    def test_div_i32(self) -> None:
        """84 / 2 = 42."""
        cir = [
            CIRInstr("const_i32", "a", [84], "i32"),
            CIRInstr("const_i32", "b", [2], "i32"),
            CIRInstr("div_i32", "r", ["a", "b"], "i32"),
            CIRInstr("ret_i32", None, ["r"], "i32"),
        ]
        assert _compile_and_run(cir) == 42


class TestBitwiseOps:
    """Bitwise AND, OR, XOR, NOT."""

    def test_and_i32(self) -> None:
        """0xFF & 0x2A = 42 (0x2A = 42)."""
        cir = [
            CIRInstr("const_i32", "a", [0xFF], "i32"),
            CIRInstr("const_i32", "b", [0x2A], "i32"),
            CIRInstr("and_i32", "r", ["a", "b"], "i32"),
            CIRInstr("ret_i32", None, ["r"], "i32"),
        ]
        assert _compile_and_run(cir) == 42

    def test_or_i32(self) -> None:
        """0x28 | 0x02 = 0x2A = 42."""
        cir = [
            CIRInstr("const_i32", "a", [0x28], "i32"),
            CIRInstr("const_i32", "b", [0x02], "i32"),
            CIRInstr("or_i32", "r", ["a", "b"], "i32"),
            CIRInstr("ret_i32", None, ["r"], "i32"),
        ]
        assert _compile_and_run(cir) == 42

    def test_xor_i32(self) -> None:
        """0x2B ^ 0x01 = 0x2A = 42."""
        cir = [
            CIRInstr("const_i32", "a", [0x2B], "i32"),
            CIRInstr("const_i32", "b", [0x01], "i32"),
            CIRInstr("xor_i32", "r", ["a", "b"], "i32"),
            CIRInstr("ret_i32", None, ["r"], "i32"),
        ]
        assert _compile_and_run(cir) == 42

    def test_not_i32_bitwise_of_zero(self) -> None:
        """IrOp.NOT is BITWISE complement: ~0 = 0xFFFFFFFF = -1 in signed i32.

        The WASM backend emits ``i32.xor`` with 0xFFFFFFFF (all-ones mask).
        This is bitwise NOT, not logical NOT.  Bitwise NOT of 0 is -1.
        """
        cir = [
            CIRInstr("const_i32", "a", [0], "i32"),
            CIRInstr("not_i32", "r", ["a"], "i32"),
            CIRInstr("ret_i32", None, ["r"], "i32"),
        ]
        # ~0 as a signed i32 = -1 (0xFFFFFFFF)
        assert _compile_and_run(cir) == -1

    def test_not_i32_bitwise_of_minus_one(self) -> None:
        """~(-1) = 0 (all bits flipped)."""
        cir = [
            CIRInstr("const_i32", "a", [-1], "i32"),
            CIRInstr("not_i32", "r", ["a"], "i32"),
            CIRInstr("ret_i32", None, ["r"], "i32"),
        ]
        assert _compile_and_run(cir) == 0


class TestNegation:
    """neg_{int} synthesised as LOAD_IMM(0) + SUB."""

    def test_neg_i32(self) -> None:
        """neg(42) = -42 mod 2^32 in WASM i32 signed arithmetic.

        WASM's ``i32.sub`` operates in two's complement.  The result
        0 - 42 = -42, which as an i32 is -42 (signed).  WASM's
        ``WasmRuntime`` returns this as a Python int: -42.
        """
        cir = [
            CIRInstr("const_i32", "a", [42], "i32"),
            CIRInstr("neg_i32", "r", ["a"], "i32"),
            CIRInstr("ret_i32", None, ["r"], "i32"),
        ]
        result = _compile_and_run(cir)
        assert result == -42


class TestComparisons:
    """Integer comparison ops, including the synthesised cmp_le / cmp_ge."""

    def test_cmp_eq_true(self) -> None:
        """5 == 5 → 1."""
        cir = [
            CIRInstr("const_i32", "a", [5], "i32"),
            CIRInstr("const_i32", "b", [5], "i32"),
            CIRInstr("cmp_eq_i32", "r", ["a", "b"], "i32"),
            CIRInstr("ret_i32", None, ["r"], "i32"),
        ]
        assert _compile_and_run(cir) == 1

    def test_cmp_eq_false(self) -> None:
        """5 == 6 → 0."""
        cir = [
            CIRInstr("const_i32", "a", [5], "i32"),
            CIRInstr("const_i32", "b", [6], "i32"),
            CIRInstr("cmp_eq_i32", "r", ["a", "b"], "i32"),
            CIRInstr("ret_i32", None, ["r"], "i32"),
        ]
        assert _compile_and_run(cir) == 0

    def test_cmp_lt_true(self) -> None:
        """3 < 5 → 1."""
        cir = [
            CIRInstr("const_i32", "a", [3], "i32"),
            CIRInstr("const_i32", "b", [5], "i32"),
            CIRInstr("cmp_lt_i32", "r", ["a", "b"], "i32"),
            CIRInstr("ret_i32", None, ["r"], "i32"),
        ]
        assert _compile_and_run(cir) == 1

    def test_cmp_gt_true(self) -> None:
        """10 > 3 → 1."""
        cir = [
            CIRInstr("const_i32", "a", [10], "i32"),
            CIRInstr("const_i32", "b", [3], "i32"),
            CIRInstr("cmp_gt_i32", "r", ["a", "b"], "i32"),
            CIRInstr("ret_i32", None, ["r"], "i32"),
        ]
        assert _compile_and_run(cir) == 1

    def test_cmp_le_true_equal(self) -> None:
        """5 <= 5 → 1 (synthesised as NOT(CMP_GT))."""
        cir = [
            CIRInstr("const_i32", "a", [5], "i32"),
            CIRInstr("const_i32", "b", [5], "i32"),
            CIRInstr("cmp_le_i32", "r", ["a", "b"], "i32"),
            CIRInstr("ret_i32", None, ["r"], "i32"),
        ]
        assert _compile_and_run(cir) == 1

    def test_cmp_le_false(self) -> None:
        """6 <= 5 → 0."""
        cir = [
            CIRInstr("const_i32", "a", [6], "i32"),
            CIRInstr("const_i32", "b", [5], "i32"),
            CIRInstr("cmp_le_i32", "r", ["a", "b"], "i32"),
            CIRInstr("ret_i32", None, ["r"], "i32"),
        ]
        assert _compile_and_run(cir) == 0

    def test_cmp_ge_true(self) -> None:
        """7 >= 7 → 1 (synthesised as NOT(CMP_LT))."""
        cir = [
            CIRInstr("const_i32", "a", [7], "i32"),
            CIRInstr("const_i32", "b", [7], "i32"),
            CIRInstr("cmp_ge_i32", "r", ["a", "b"], "i32"),
            CIRInstr("ret_i32", None, ["r"], "i32"),
        ]
        assert _compile_and_run(cir) == 1


class TestControlFlow:
    """Branches: BRANCH_NZ (jmp_if_true) and BRANCH_Z (jmp_if_false).

    WASM structured-control-flow constraint
    ----------------------------------------
    The ir-to-wasm-compiler uses a "structured" lowering strategy that maps
    IR branches to WASM ``if / else / end`` blocks.  The strategy requires
    branch targets to follow the naming convention ``if_N_else`` / ``if_N_end``
    (where N is any digit string), matched by ``re.compile(r'^if_\\d+_else$')``.

    Branch semantics in the structured lowering:

      BRANCH_NZ cond if_0_else  →  ``local.get cond; i32.eqz; if {then} else {else}``
      BRANCH_Z  cond if_0_else  →  ``local.get cond;          if {then} else {else}``

    The "then" body (between branch and ``if_N_else``) runs when the condition
    is *false* for the branch op:
      • BRANCH_NZ — then runs when cond == 0 (branch NOT taken)
      • BRANCH_Z  — then runs when cond != 0 (branch NOT taken)

    The "else" body (between ``if_N_else`` and ``if_N_end``) runs when the
    condition is *true* for the branch op (branch TAKEN).

    CIR structure
    -------------
    ::

        BRANCH_{X} cond if_0_else  # choose path based on cond
        <then body>                 # cond NOT matching — branch not taken
        JUMP if_0_end
        LABEL if_0_else
        <else body>                 # cond matching — branch taken
        LABEL if_0_end
        ret_...
    """

    def test_jmp_if_true_taken(self) -> None:
        """Condition 1 (non-zero) → jmp_if_true branch taken → result = 10.

        ``jmp_if_true`` maps to ``BRANCH_NZ``.  In the structured WASM lowering
        this targets ``if_0_else`` and the WASM emitter prefixes the condition
        with ``i32.eqz`` so the ``if`` block runs the "then" body when
        cond == 0 and the "else" body when cond != 0.

        With cond = 1 (truthy), the branch IS taken → else body → result = 10.
        """
        cir = [
            CIRInstr("const_i32", "cond", [1], "i32"),
            # BRANCH_NZ: branch (to else) when cond != 0
            CIRInstr("jmp_if_true", None, ["cond", "if_0_else"], "bool"),
            # then body: cond == 0 (NOT taken path)
            CIRInstr("const_i32", "result", [99], "i32"),
            CIRInstr("jmp", None, ["if_0_end"], "void"),
            CIRInstr("label", None, ["if_0_else"], "void"),
            # else body: cond != 0 (TAKEN path) → result = 10
            CIRInstr("const_i32", "result", [10], "i32"),
            CIRInstr("label", None, ["if_0_end"], "void"),
            CIRInstr("ret_i32", None, ["result"], "i32"),
        ]
        assert _compile_and_run(cir) == 10

    def test_jmp_if_false_taken(self) -> None:
        """Condition 0 → jmp_if_false branch taken → result = 77.

        ``jmp_if_false`` maps to ``BRANCH_Z``.  In the structured WASM lowering
        the WASM ``if`` block condition is the raw register (no eqz), so the
        "then" body runs when cond != 0 and the "else" body runs when cond == 0.

        With cond = 0, the branch IS taken → else body → result = 77.
        """
        cir = [
            CIRInstr("const_i32", "cond", [0], "i32"),
            # BRANCH_Z: branch (to else) when cond == 0
            CIRInstr("jmp_if_false", None, ["cond", "if_0_else"], "bool"),
            # then body: cond != 0 (NOT taken path)
            CIRInstr("const_i32", "result", [1], "i32"),
            CIRInstr("jmp", None, ["if_0_end"], "void"),
            CIRInstr("label", None, ["if_0_else"], "void"),
            # else body: cond == 0 (TAKEN path) → result = 77
            CIRInstr("const_i32", "result", [77], "i32"),
            CIRInstr("label", None, ["if_0_end"], "void"),
            CIRInstr("ret_i32", None, ["result"], "i32"),
        ]
        assert _compile_and_run(cir) == 77


class TestTypeAssert:
    """type_assert lowered to COMMENT — no runtime effect on result."""

    def test_type_assert_passes_through(self) -> None:
        """type_assert must not affect the computed result."""
        cir = [
            CIRInstr("const_i32", "x", [99], "i32"),
            CIRInstr("type_assert", None, ["x", "i32"], "void"),
            CIRInstr("ret_i32", None, ["x"], "i32"),
        ]
        assert _compile_and_run(cir) == 99


# ============================================================================
# Section 2: Full Tetrad → WASM pipeline
# ============================================================================
# These tests use the real Tetrad frontend (tetrad-runtime) and JIT core
# to demonstrate the complete compilation pipeline described in the project.
# ============================================================================


class TestTetradToWASM:
    """End-to-end: Tetrad source → JIT specialise → WASM → result."""

    def _tetrad_to_wasm(self, source: str) -> object:
        """Run the full pipeline on a Tetrad source string.

        Steps:
        1. compile_to_iir(source)          → IIRModule
        2. specialise(main_fn, min_obs=0)  → list[CIRInstr]
        3. WASMBackend.compile(cir)        → bytes  (LANG21 + LANG20 inside)
        4. WASMBackend.run(binary, [])     → result
        """
        from interpreter_ir.function import FunctionTypeStatus
        from jit_core import specialise
        from tetrad_runtime import compile_to_iir

        module = compile_to_iir(source)
        main_fn = module.get_function("main")
        assert main_fn is not None, "Tetrad source must define fn main()"

        # Force AOT: min_observations=0 accepts any observed type without
        # requiring a minimum sample count.  FULLY_TYPED functions work
        # without any VM observations at all.
        assert main_fn.type_status in (
            FunctionTypeStatus.FULLY_TYPED,
            FunctionTypeStatus.PARTIALLY_TYPED,
            FunctionTypeStatus.UNTYPED,
        )

        cir = specialise(main_fn, min_observations=0)
        assert len(cir) > 0, "specialise() returned empty CIR"

        backend = WASMBackend()
        binary = backend.compile(cir)
        assert binary is not None, "WASMBackend.compile() returned None"
        return backend.run(binary, [])

    def test_return_constant(self) -> None:
        """fn main() -> u8 { return 42; }  →  42."""
        result = self._tetrad_to_wasm("fn main() -> u8 { return 42; }")
        assert result == 42

    def test_return_addition(self) -> None:
        """fn main() -> u8 { return 40 + 2; }  →  42."""
        result = self._tetrad_to_wasm("fn main() -> u8 { return 40 + 2; }")
        assert result == 42

    def test_return_subtraction(self) -> None:
        """fn main() -> u8 { return 50 - 8; }  →  42."""
        result = self._tetrad_to_wasm("fn main() -> u8 { return 50 - 8; }")
        assert result == 42

    def test_return_multiplication(self) -> None:
        """fn main() -> u8 { return 6 * 7; }  →  42."""
        result = self._tetrad_to_wasm("fn main() -> u8 { return 6 * 7; }")
        assert result == 42

    def test_return_zero(self) -> None:
        """fn main() -> u8 { return 0; }  →  0."""
        result = self._tetrad_to_wasm("fn main() -> u8 { return 0; }")
        assert result == 0

    def test_return_255(self) -> None:
        """fn main() -> u8 { return 255; }  →  255."""
        result = self._tetrad_to_wasm("fn main() -> u8 { return 255; }")
        assert result == 255

    def test_result_matches_interpreter(self) -> None:
        """The WASM result must match the interpreter result (42 + 1 = 43).

        This is the definitive correctness check: both execution paths
        (VM interpreter and WASM JIT) must agree on the output.
        """
        from tetrad_runtime import TetradRuntime

        source = "fn main() -> u8 { return 42 + 1; }"
        rt = TetradRuntime()
        interpreted = rt.run(source)

        wasm_result = self._tetrad_to_wasm(source)
        assert wasm_result == interpreted, (
            f"WASM result {wasm_result!r} does not match "
            f"interpreter result {interpreted!r}"
        )

    def test_full_pipeline_via_jitcore(self) -> None:
        """Show the pipeline working via JITCore (not just raw specialise).

        JITCore.execute_with_jit():
          Phase 1 — eager compile of FULLY_TYPED functions via WASMBackend
          Phase 2 — VM execution (JIT handler fires for main)
          Phase 3 — promote any new hot functions
        """
        from jit_core import JITCore
        from tetrad_runtime import compile_to_iir
        from vm_core import VMCore

        source = "fn main() -> u8 { return 3 + 4; }"
        module = compile_to_iir(source)

        backend = WASMBackend()
        vm = VMCore(
            opcodes={},
            u8_wrap=True,
        )
        jit = JITCore(vm, backend, min_observations=0)

        result = jit.execute_with_jit(module, fn="main")
        assert result == 7, f"expected 7 but JITCore returned {result!r}"

    def test_run_with_jit_api(self) -> None:
        """TetradRuntime.run_with_jit(source, backend=WASMBackend()) → result.

        This is the top-level user-facing API.  One line of code runs
        Tetrad source through the entire compiler stack and returns the
        answer computed by the WASM runtime.
        """
        from tetrad_runtime import TetradRuntime

        rt = TetradRuntime()
        result = rt.run_with_jit(
            "fn main() -> u8 { return 40 + 2; }",
            backend=WASMBackend(),
        )
        assert result == 42, f"expected 42 but got {result!r}"


# ============================================================================
# Section 3: BackendProtocol compatibility
# ============================================================================


class TestBackendProtocol:
    """WASMBackend must satisfy the structural BackendProtocol check."""

    def test_isinstance_check(self) -> None:
        """isinstance(WASMBackend(), BackendProtocol) is True."""
        from codegen_core import BackendProtocol

        assert isinstance(WASMBackend(), BackendProtocol)

    def test_has_compile(self) -> None:
        """compile() method is callable."""
        assert callable(getattr(WASMBackend(), "compile"))

    def test_has_run(self) -> None:
        """run() method is callable."""
        assert callable(getattr(WASMBackend(), "run"))

    def test_compile_returns_bytes_or_none(self) -> None:
        """compile() must return bytes or None — never raises unexpectedly."""
        backend = WASMBackend()
        cir = [
            CIRInstr("const_i32", "x", [1], "i32"),
            CIRInstr("ret_i32", None, ["x"], "i32"),
        ]
        result = backend.compile(cir)
        assert result is None or isinstance(result, bytes)

    def test_compile_bad_cir_returns_none(self) -> None:
        """An unsupported op triggers deopt (returns None), not an exception."""
        backend = WASMBackend()
        cir = [
            CIRInstr("call_runtime", None, ["gc_alloc"], "void"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        # call_runtime raises CIRLoweringError which compile() catches and
        # converts to None (deopt signal).
        result = backend.compile(cir)
        assert result is None

    def test_custom_entry_label(self) -> None:
        """entry_label parameter customises the WASM export name."""
        backend = WASMBackend(entry_label="my_func")
        cir = [
            CIRInstr("const_i32", "x", [7], "i32"),
            CIRInstr("ret_i32", None, ["x"], "i32"),
        ]
        binary = backend.compile(cir)
        assert binary is not None
        result = backend.run(binary, [])
        assert result == 7


# ============================================================================
# Section 4: _collect_cir_registers helper
# ============================================================================


class TestCollectCirRegisters:
    """Unit tests for the internal _collect_cir_registers() helper."""

    def test_single_dest(self) -> None:
        """One instruction with a single dest."""
        cir = [CIRInstr("const_i32", "x", [1], "i32")]
        reg_map = _collect_cir_registers(cir)
        assert reg_map == {"x": 0}

    def test_dest_and_srcs(self) -> None:
        """Dest before srcs; each new name gets the next index."""
        cir = [
            CIRInstr("const_i32", "a", [1], "i32"),
            CIRInstr("const_i32", "b", [2], "i32"),
            CIRInstr("add_i32", "c", ["a", "b"], "i32"),
        ]
        reg_map = _collect_cir_registers(cir)
        assert reg_map["a"] == 0
        assert reg_map["b"] == 1
        assert reg_map["c"] == 2

    def test_label_op_srcs_not_collected(self) -> None:
        """jmp / label / call: srcs are label names, not variable names."""
        cir = [
            CIRInstr("label", None, ["loop_start"], "void"),
            CIRInstr("jmp", None, ["loop_start"], "void"),
        ]
        reg_map = _collect_cir_registers(cir)
        assert "loop_start" not in reg_map
        assert reg_map == {}

    def test_conditional_branch_label_not_collected(self) -> None:
        """Conditional branch: srcs[0] is a variable, srcs[1] is a label."""
        cir = [
            CIRInstr("const_i32", "cond", [1], "i32"),
            CIRInstr("jmp_if_true", None, ["cond", "target"], "bool"),
            CIRInstr("label", None, ["target"], "void"),
        ]
        reg_map = _collect_cir_registers(cir)
        assert "cond" in reg_map
        assert "target" not in reg_map

    def test_type_assert_type_not_collected(self) -> None:
        """type_assert: srcs[0] is a variable, srcs[1] is a type name."""
        cir = [
            CIRInstr("const_i32", "x", [1], "i32"),
            CIRInstr("type_assert", None, ["x", "i32"], "void"),
        ]
        reg_map = _collect_cir_registers(cir)
        assert "x" in reg_map
        assert "i32" not in reg_map

    def test_same_var_same_index(self) -> None:
        """The same variable always gets the same register (SSA property)."""
        cir = [
            CIRInstr("const_i32", "x", [1], "i32"),
            CIRInstr("add_i32", "y", ["x", "x"], "i32"),
        ]
        reg_map = _collect_cir_registers(cir)
        # x appears three times (once as dest, twice as src) — always index 0
        assert reg_map["x"] == 0
        assert reg_map["y"] == 1
        assert len(reg_map) == 2

    def test_ret_src_collected(self) -> None:
        """The variable in ret_{type}.srcs[0] is collected as a variable."""
        cir = [
            CIRInstr("const_i32", "result", [42], "i32"),
            CIRInstr("ret_i32", None, ["result"], "i32"),
        ]
        reg_map = _collect_cir_registers(cir)
        assert "result" in reg_map


# ============================================================================
# Section 5: Register fixup edge cases
# ============================================================================


class TestRegisterFixup:
    """The result-register fixup must produce the correct WASM return value."""

    def test_result_in_register_0(self) -> None:
        """Result is the first variable (register 0) → fixup to register 1."""
        # The JIT constant-folds and emits: const_u8 result [42]; ret_u8 result
        # Pass 1: result → 0.  Fixup: ADD_IMM r1, r0, 0.
        cir = [
            CIRInstr("const_u8", "result", [42], "u8"),
            CIRInstr("ret_u8", None, ["result"], "u8"),
        ]
        assert _compile_and_run(cir) == 42

    def test_result_in_register_1_no_fixup_needed(self) -> None:
        """When result lands in register 1, no fixup is added (already correct)."""
        # Pass 1: a→0, result→1 (result is the second variable)
        cir = [
            CIRInstr("const_i32", "a", [1], "i32"),
            CIRInstr("const_i32", "result", [55], "i32"),
            CIRInstr("ret_i32", None, ["result"], "i32"),
        ]
        assert _compile_and_run(cir) == 55

    def test_result_in_register_2(self) -> None:
        """Result in register 2 (classic a + b case) → fixup copies to r1."""
        # a→0, b→1, result→2 → fixup: ADD_IMM r1, r2, 0
        cir = [
            CIRInstr("const_i32", "a", [20], "i32"),
            CIRInstr("const_i32", "b", [22], "i32"),
            CIRInstr("add_i32", "result", ["a", "b"], "i32"),
            CIRInstr("ret_i32", None, ["result"], "i32"),
        ]
        assert _compile_and_run(cir) == 42

    def test_type_asserts_do_not_shift_result_register(self) -> None:
        """type_assert instructions add no registers — result stays correct."""
        # type_assert has no dest, and srcs[0] is an already-registered var.
        cir = [
            CIRInstr("const_i32", "x", [10], "i32"),
            CIRInstr("const_i32", "y", [32], "i32"),
            CIRInstr("type_assert", None, ["x", "i32"], "void"),
            CIRInstr("type_assert", None, ["y", "i32"], "void"),
            CIRInstr("add_i32", "result", ["x", "y"], "i32"),
            CIRInstr("ret_i32", None, ["result"], "i32"),
        ]
        assert _compile_and_run(cir) == 42
