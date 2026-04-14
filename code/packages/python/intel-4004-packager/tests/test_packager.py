"""Unit tests for Intel4004Packager class.

Tests the orchestration logic: correct stage sequencing, error wrapping,
and configuration options (optimize flag, origin offset).
"""

from __future__ import annotations

import pytest

from intel_4004_packager import Intel4004Packager, PackageError, PackageResult


class TestPackagerInit:
    def test_default_packager_creates(self) -> None:
        packager = Intel4004Packager()
        assert packager is not None

    def test_optimize_false_creates(self) -> None:
        packager = Intel4004Packager(optimize=False)
        assert packager is not None

    def test_custom_origin(self) -> None:
        packager = Intel4004Packager(origin=0x100)
        result = packager.pack_source("")
        # The first record's address should be 0x0100
        first_data_line = result.hex_text.splitlines()[0]
        assert first_data_line[3:7] == "0100", (
            f"expected address 0100, got {first_data_line[3:7]}"
        )


class TestPackagerResult:
    SOURCE = "fn main() -> u4 { return 4; }"

    def test_result_is_package_result(self) -> None:
        result = Intel4004Packager().pack_source(self.SOURCE)
        assert isinstance(result, PackageResult)

    def test_result_is_frozen(self) -> None:
        result = Intel4004Packager().pack_source(self.SOURCE)
        with pytest.raises((AttributeError, TypeError)):
            result.binary = b""  # type: ignore[misc]

    def test_hex_text_is_str(self) -> None:
        result = Intel4004Packager().pack_source(self.SOURCE)
        assert isinstance(result.hex_text, str)

    def test_binary_is_bytes(self) -> None:
        result = Intel4004Packager().pack_source(self.SOURCE)
        assert isinstance(result.binary, bytes)

    def test_asm_text_is_str(self) -> None:
        result = Intel4004Packager().pack_source(self.SOURCE)
        assert isinstance(result.asm_text, str)


class TestPackagerErrors:
    def test_package_error_is_exception(self) -> None:
        exc = PackageError("parse", "test error")
        assert isinstance(exc, Exception)

    def test_package_error_stage(self) -> None:
        exc = PackageError("backend", "isa error")
        assert exc.stage == "backend"

    def test_package_error_message(self) -> None:
        exc = PackageError("typecheck", "undefined variable")
        assert exc.message == "undefined variable"

    def test_package_error_cause_none_default(self) -> None:
        exc = PackageError("parse", "bad syntax")
        assert exc.cause is None

    def test_package_error_with_cause(self) -> None:
        cause = ValueError("inner error")
        exc = PackageError("assemble", "encode failed", cause)
        assert exc.cause is cause

    def test_package_error_str_format(self) -> None:
        exc = PackageError("backend", "too deep")
        assert "[backend]" in str(exc)
        assert "too deep" in str(exc)

    def test_package_error_str_with_cause(self) -> None:
        exc = PackageError("assemble", "bad opcode", ValueError("unknown NOP2"))
        s = str(exc)
        assert "[assemble]" in s
        assert "bad opcode" in s

    def test_bad_syntax_raises_parse_or_typecheck(self) -> None:
        with pytest.raises(PackageError) as exc_info:
            Intel4004Packager().pack_source("fn {{{")
        assert exc_info.value.stage in ("parse", "typecheck")

    def test_type_mismatch_raises_typecheck(self) -> None:
        # u4 variable assigned to u8 — type mismatch
        source = "fn main() -> u4 { let x: u4 = 5; let y: u8 = x; return y; }"
        with pytest.raises(PackageError) as exc_info:
            Intel4004Packager().pack_source(source)
        assert exc_info.value.stage == "typecheck"


class TestOptimizeFlag:
    SOURCE = """
    fn main() -> u4 {
        let x: u4 = 5;
        return x;
    }
    """

    def test_optimize_true_runs(self) -> None:
        result = Intel4004Packager(optimize=True).pack_source(self.SOURCE)
        assert result.binary

    def test_optimize_false_runs(self) -> None:
        result = Intel4004Packager(optimize=False).pack_source(self.SOURCE)
        assert result.binary

    def test_optimize_produces_valid_hex(self) -> None:
        result = Intel4004Packager(optimize=True).pack_source(self.SOURCE)
        assert ":00000001FF" in result.hex_text

    def test_no_optimize_produces_valid_hex(self) -> None:
        result = Intel4004Packager(optimize=False).pack_source(self.SOURCE)
        assert ":00000001FF" in result.hex_text
