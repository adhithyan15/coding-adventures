"""End-to-end pipeline integration tests.

=== What This Test Suite Proves ===

These tests exercise the FULL Nib compiler pipeline:

    Nib source text
        → nib-parser         (text → AST)
        → nib-type-checker   (AST → typed AST)
        → nib-ir-compiler    (typed AST → IrProgram)
        → ir-optimizer       (IrProgram → optimized IrProgram)
        → intel-4004-backend (IrProgram → assembly text)
        → intel-4004-assembler (assembly text → binary bytes)
        → intel-4004-packager  (binary → Intel HEX)

Each test uses a complete, syntactically valid Nib program (statements
terminated with ``;`` as required by the grammar) and verifies that the
pipeline produces correct output at multiple levels.

=== Nib Syntax Notes ===

Nib requires semicolons as statement terminators:

  fn main() -> u4 {
      let x: u4 = 5;       ← semicolon required
      return x;            ← semicolon required
  }

Operators: use ``+%`` for wrapping unsigned addition on u4/u8 (not plain ``+``).
Wrap-add on u4: (15 +% 1) == 0 because u4 holds 0–15 and wraps at 16.

=== End-to-End Simulation ===

For tests that check *runtime behavior*, we use ``Intel4004Simulator`` to
execute the compiled binary and inspect register state via:

    result = simulator.execute(binary, max_steps=10_000)
    assert result.ok
    state = result.final_state
    assert state.registers[1] == expected_value  # R1 = return value

Register layout after compilation:
  R0 = zero constant (v0)
  R1 = return value / scratch (v1)
  R2 = first named variable (v2)
  R3 = second named variable (v3)
  ...
"""

from __future__ import annotations

import pytest

from intel_4004_packager import Intel4004Packager, PackageError, decode_hex


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _all_record_checksums_valid(hex_text: str) -> bool:
    """Verify the Intel HEX checksum invariant on every record."""
    for line in hex_text.splitlines():
        line = line.strip()
        if not line:
            continue
        record_bytes = bytes.fromhex(line[1:])
        if sum(record_bytes) % 256 != 0:
            return False
    return True


# ---------------------------------------------------------------------------
# Intel HEX format properties (pipeline-level)
# ---------------------------------------------------------------------------


class TestHexOutputFormat:
    """Every compiled program must produce well-formed Intel HEX."""

    PROGRAMS = [
        ("empty program", ""),
        (
            "main returns constant",
            "fn main() -> u4 { return 7; }",
        ),
        (
            "let binding",
            "fn main() -> u4 { let x: u4 = 3; return x; }",
        ),
    ]

    @pytest.mark.parametrize("desc,source", PROGRAMS)
    def test_starts_with_colon(self, desc: str, source: str) -> None:
        packager = Intel4004Packager()
        result = packager.pack_source(source)
        assert result.hex_text.startswith(":")

    @pytest.mark.parametrize("desc,source", PROGRAMS)
    def test_ends_with_eof_record(self, desc: str, source: str) -> None:
        packager = Intel4004Packager()
        result = packager.pack_source(source)
        non_empty = [l for l in result.hex_text.splitlines() if l.strip()]
        assert non_empty[-1] == ":00000001FF"

    @pytest.mark.parametrize("desc,source", PROGRAMS)
    def test_all_checksums_valid(self, desc: str, source: str) -> None:
        packager = Intel4004Packager()
        result = packager.pack_source(source)
        assert _all_record_checksums_valid(result.hex_text)

    @pytest.mark.parametrize("desc,source", PROGRAMS)
    def test_hex_roundtrip(self, desc: str, source: str) -> None:
        packager = Intel4004Packager()
        result = packager.pack_source(source)
        origin, decoded = decode_hex(result.hex_text)
        assert origin == 0
        assert decoded == result.binary


# ---------------------------------------------------------------------------
# Pipeline artifacts
# ---------------------------------------------------------------------------


class TestPipelineArtifacts:
    """PackageResult must contain all intermediate pipeline artifacts."""

    SOURCE = "fn main() -> u4 { let x: u4 = 5; return x; }"

    def test_typed_ast_present(self) -> None:
        result = Intel4004Packager().pack_source(self.SOURCE)
        assert result.typed_ast is not None

    def test_raw_ir_present(self) -> None:
        result = Intel4004Packager().pack_source(self.SOURCE)
        assert result.raw_ir is not None
        assert len(result.raw_ir.instructions) > 0

    def test_optimized_ir_present(self) -> None:
        result = Intel4004Packager().pack_source(self.SOURCE)
        assert result.optimized_ir is not None

    def test_asm_text_present(self) -> None:
        result = Intel4004Packager().pack_source(self.SOURCE)
        assert result.asm_text
        assert "ORG" in result.asm_text

    def test_binary_present(self) -> None:
        result = Intel4004Packager().pack_source(self.SOURCE)
        assert isinstance(result.binary, bytes)
        assert len(result.binary) > 0

    def test_hex_text_present(self) -> None:
        result = Intel4004Packager().pack_source(self.SOURCE)
        assert result.hex_text
        assert ":00000001FF" in result.hex_text

    def test_optimizer_does_not_increase_instructions(self) -> None:
        result = Intel4004Packager().pack_source(self.SOURCE)
        assert len(result.optimized_ir.instructions) <= len(result.raw_ir.instructions)

    def test_no_optimize_flag(self) -> None:
        result = Intel4004Packager(optimize=False).pack_source(self.SOURCE)
        assert result.raw_ir is not None
        assert result.optimized_ir is not None

    def test_binary_is_bytes(self) -> None:
        result = Intel4004Packager().pack_source(self.SOURCE)
        assert isinstance(result.binary, bytes)


# ---------------------------------------------------------------------------
# Assembly text spot checks
# ---------------------------------------------------------------------------


class TestAssemblyText:
    """Verify key patterns in the emitted Intel 4004 assembly text."""

    def test_org_directive_present(self) -> None:
        source = "fn main() -> u4 { return 0; }"
        result = Intel4004Packager().pack_source(source)
        assert "ORG" in result.asm_text

    def test_start_label_present(self) -> None:
        result = Intel4004Packager().pack_source("")
        assert "_start:" in result.asm_text

    def test_constant_load_uses_ldm(self) -> None:
        source = "fn main() -> u4 { let x: u4 = 9; return x; }"
        result = Intel4004Packager().pack_source(source)
        assert "LDM" in result.asm_text

    def test_halt_instruction_present(self) -> None:
        result = Intel4004Packager().pack_source("")
        assert "HLT" in result.asm_text or "JUN $" in result.asm_text

    def test_function_call_uses_jms(self) -> None:
        source = (
            "fn helper() -> u4 { return 1; } "
            "fn main() -> u4 { return helper(); }"
        )
        result = Intel4004Packager().pack_source(source)
        assert "JMS" in result.asm_text

    def test_return_uses_bbl(self) -> None:
        source = "fn main() -> u4 { return 3; }"
        result = Intel4004Packager().pack_source(source)
        assert "BBL" in result.asm_text


# ---------------------------------------------------------------------------
# Error handling
# ---------------------------------------------------------------------------


class TestPipelineErrors:
    """Pipeline errors must be wrapped in PackageError with the correct stage."""

    def test_syntax_error_raises_package_error(self) -> None:
        with pytest.raises(PackageError) as exc_info:
            Intel4004Packager().pack_source("fn broken { }")
        assert exc_info.value.stage in ("parse", "typecheck")

    def test_type_error_raises_package_error(self) -> None:
        # u4 variable assigned to u8 is a type mismatch
        source = "fn main() -> u4 { let x: u4 = 5; let y: u8 = x; return y; }"
        with pytest.raises(PackageError) as exc_info:
            Intel4004Packager().pack_source(source)
        assert exc_info.value.stage == "typecheck"

    def test_package_error_has_stage(self) -> None:
        try:
            Intel4004Packager().pack_source("fn main() -> u4 { let x: u4 = 5; let y: u8 = x; return y; }")
        except PackageError as exc:
            assert exc.stage in ("parse", "typecheck", "ir_compile", "backend", "assemble", "pack")

    def test_package_error_str_includes_stage(self) -> None:
        try:
            Intel4004Packager().pack_source("fn main() -> u4 { let x: u4 = 5; let y: u8 = x; return y; }")
        except PackageError as exc:
            assert "[" in str(exc)


# ---------------------------------------------------------------------------
# End-to-end: compile → binary → Intel 4004 simulator → register state
# ---------------------------------------------------------------------------


class TestEndToEnd:
    """Full pipeline: Nib source → binary → Intel 4004 simulator → register state.

    These tests load the compiled binary into Intel4004Simulator and check
    that the correct value ends up in R1 (the return value register).

    Virtual register layout:
      v0 → R0 (zero constant)
      v1 → R1 (return value / scratch)
      v2 → R2 (first named variable)
      ...

    After ``fn main() -> u4 { return 7; }`` the compiled binary:
      1. Calls main (JMS _fn_main)
      2. main: LDM 7, XCH R1, BBL 0
      3. HLT at _start

    So R1 == 7 after execution.
    """

    @pytest.fixture()
    def simulator(self):  # noqa: ANN201
        """Return a fresh Intel 4004 simulator."""
        try:
            from intel4004_simulator import Intel4004Simulator  # type: ignore[import]
            return Intel4004Simulator()
        except ImportError:
            pytest.skip("intel4004-simulator not installed — skipping simulation tests")

    def _compile(self, source: str) -> bytes:
        return Intel4004Packager().pack_source(source).binary

    def test_empty_program_halts(self, simulator) -> None:  # noqa: ANN001
        binary = self._compile("")
        result = simulator.execute(binary, max_steps=10_000)
        assert result.ok, f"simulator did not halt cleanly: {result.error}"

    def test_constant_main_halts(self, simulator) -> None:  # noqa: ANN001
        source = "fn main() -> u4 { return 7; }"
        binary = self._compile(source)
        result = simulator.execute(binary, max_steps=10_000)
        assert result.ok, f"simulator error: {result.error}"

    def test_let_binding_loads_register(self, simulator) -> None:  # noqa: ANN001
        """let x: u4 = 5 → R1 = 5 after main returns."""
        source = "fn main() -> u4 { let x: u4 = 5; return x; }"
        binary = self._compile(source)
        result = simulator.execute(binary, max_steps=10_000)
        assert result.ok, f"simulator error: {result.error}"
        assert result.final_state.registers[1] == 5, (
            f"expected R1=5, got R1={result.final_state.registers[1]}"
        )

    def test_addition_result(self, simulator) -> None:  # noqa: ANN001
        """3 +% 4 = 7."""
        source = "fn main() -> u4 { let x: u4 = 3; let y: u4 = x +% 4; return y; }"
        binary = self._compile(source)
        result = simulator.execute(binary, max_steps=10_000)
        assert result.ok, f"simulator error: {result.error}"
        assert result.final_state.registers[1] == 7, (
            f"expected R1=7, got R1={result.final_state.registers[1]}"
        )

    def test_wrap_add_u4_wraps_at_16(self, simulator) -> None:  # noqa: ANN001
        """15 +% 1 = 0 (u4 wraps at 16)."""
        source = "fn main() -> u4 { let x: u4 = 15; let y: u4 = x +% 1; return y; }"
        binary = self._compile(source)
        result = simulator.execute(binary, max_steps=10_000)
        assert result.ok, f"simulator error: {result.error}"
        assert result.final_state.registers[1] == 0, (
            f"expected R1=0 (15+1 wraps to 0 on u4), got R1={result.final_state.registers[1]}"
        )

    def test_two_functions(self, simulator) -> None:  # noqa: ANN001
        """add_one(6) = 7."""
        source = (
            "fn add_one(x: u4) -> u4 { return x +% 1; } "
            "fn main() -> u4 { return add_one(6); }"
        )
        binary = self._compile(source)
        result = simulator.execute(binary, max_steps=10_000)
        assert result.ok, f"simulator error: {result.error}"
        assert result.final_state.registers[1] == 7, (
            f"expected add_one(6)=7, got R1={result.final_state.registers[1]}"
        )

    def test_compiled_binary_is_loadable(self, simulator) -> None:  # noqa: ANN001
        source = "fn main() -> u4 { let x: u4 = 1; return x; }"
        binary = self._compile(source)
        result = simulator.execute(binary, max_steps=10_000)
        assert result.ok, f"simulator error: {result.error}"
        assert result.final_state.registers[1] == 1, (
            f"expected R1=1, got R1={result.final_state.registers[1]}"
        )

    def test_hex_roundtrip_then_simulate(self, simulator) -> None:  # noqa: ANN001
        """Binary → Intel HEX → binary → simulate gives same result."""
        source = "fn main() -> u4 { return 9; }"
        result = Intel4004Packager().pack_source(source)
        _origin, recovered = decode_hex(result.hex_text)
        assert recovered == result.binary
        sim_result = simulator.execute(recovered, max_steps=10_000)
        assert sim_result.ok, f"simulator error: {sim_result.error}"

    def test_step_count_is_positive(self, simulator) -> None:  # noqa: ANN001
        source = "fn main() -> u4 { let x: u4 = 2; let y: u4 = x +% 3; return y; }"
        binary = self._compile(source)
        result = simulator.execute(binary, max_steps=10_000)
        assert result.ok
        assert result.steps > 1, f"expected >1 steps, got {result.steps}"
