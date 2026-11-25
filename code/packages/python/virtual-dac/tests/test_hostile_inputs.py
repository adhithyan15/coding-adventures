"""Hostile input testing for CWE-20, CWE-190, CWE-400, and CWE-1333 (VER15)."""

from __future__ import annotations

import ast
import sys
from fractions import Fraction
from pathlib import Path

import pytest
from pcm_audio import PCMBuffer, PCMFormat

from virtual_dac import (
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    ZeroOrderHoldDACSignal,
    pcm16_to_voltage,
)


class TestCwe20TypeAndNumericValidation:
    @pytest.mark.parametrize(
        "bad_val",
        [
            True,
            False,
            float("nan"),
            float("inf"),
            float("-inf"),
            "0.0",
            None,
            complex(0.0, 0.0),
        ],
    )
    def test_rejects_bool_non_finite_and_non_real(self, bad_val: object) -> None:
        with pytest.raises(ValueError):
            pcm16_to_voltage(bad_val)  # type: ignore[arg-type]

        dac = ZeroOrderHoldDACSignal(PCMBuffer((0,), PCMFormat(sample_rate_hz=10.0)))
        with pytest.raises(ValueError):
            dac.value_at(bad_val)  # type: ignore[arg-type]

    @pytest.mark.parametrize(
        "huge_real",
        [10**400, -(10**400), Fraction(10**400, 3), Fraction(-(10**400), 3)],
        ids=["int-pos", "int-neg", "frac-pos", "frac-neg"],
    )
    def test_huge_real_overflows_raise_value_error_not_overflow_error(
        self, huge_real: object
    ) -> None:
        dac = ZeroOrderHoldDACSignal(PCMBuffer((0,), PCMFormat(sample_rate_hz=10.0)))
        with pytest.raises(ValueError, match="overflows float range"):
            dac.value_at(huge_real)  # type: ignore[arg-type]

    def test_extreme_start_time_overflow_returns_zero_without_overflow_error(
        self,
    ) -> None:
        buf = PCMBuffer(
            (1000,),
            PCMFormat(sample_rate_hz=10.0),
            start_time_seconds=sys.float_info.max,
        )
        dac = ZeroOrderHoldDACSignal(buf)
        assert dac.value_at(sys.float_info.max) == 0.0

        # Mutated buffer with start=-1e308 and subnormal rate -> duration=inf,
        # so time=1e308 passes window check and (time - start) * rate is non-finite.
        bad_buf = PCMBuffer((1000,), PCMFormat(sample_rate_hz=10.0))
        object.__setattr__(bad_buf, "start_time_seconds", -1e308)
        object.__setattr__(bad_buf.pcm_format, "sample_rate_hz", 5e-324)
        bad_dac = ZeroOrderHoldDACSignal(bad_buf)
        assert bad_dac.value_at(1e308) == 0.0


class TestCwe190IntegerBoundsAndSafeRepr:
    @pytest.mark.parametrize(
        "huge_int",
        [10**5000, -(10**5000)],
        ids=["pos-10e5000", "neg-10e5000"],
    )
    def test_huge_int_fails_in_o1_via_safe_repr(self, huge_int: int) -> None:
        with pytest.raises(ValueError, match="<out-of-range int>") as exc_info:
            pcm16_to_voltage(huge_int)
        assert len(str(exc_info.value)) < 160

    def test_safe_repr_formats_long_and_short_strings(self) -> None:
        dac = ZeroOrderHoldDACSignal(PCMBuffer((0,), PCMFormat(sample_rate_hz=10.0)))
        with pytest.raises(ValueError, match="<str len=5000>") as exc1:
            dac.value_at("x" * 5000)  # type: ignore[arg-type]
        assert len(str(exc1.value)) < 160

        with pytest.raises(ValueError, match="'abc'"):
            dac.value_at("abc")  # type: ignore[arg-type]


class TestCwe400ResourceExhaustionBounds:
    def test_rejects_oversized_sample_count_and_rate(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        import virtual_dac.virtual_dac as dac_mod

        pcm = PCMBuffer((0, 1, 2), PCMFormat(sample_rate_hz=100.0))
        monkeypatch.setattr(dac_mod, "MAX_SAMPLES", 2)
        with pytest.raises(ValueError, match="MAX_SAMPLES"):
            ZeroOrderHoldDACSignal(pcm)

        monkeypatch.setattr(dac_mod, "MAX_SAMPLES", MAX_SAMPLES)
        monkeypatch.setattr(dac_mod, "MAX_SAMPLE_RATE_HZ", 50.0)
        with pytest.raises(ValueError, match="MAX_SAMPLE_RATE_HZ"):
            ZeroOrderHoldDACSignal(pcm)
        assert MAX_SAMPLE_RATE_HZ == 192_000.0


class TestCwe1333ZeroRegexImports:
    def test_no_regex_imports_in_source(self) -> None:
        pkg_src = Path(__file__).resolve().parents[1] / "src" / "virtual_dac"
        for py_file in pkg_src.rglob("*.py"):
            tree = ast.parse(py_file.read_text(encoding="utf-8"))
            for node in ast.walk(tree):
                if isinstance(node, ast.Import):
                    assert all(alias.name != "re" for alias in node.names)
                elif isinstance(node, ast.ImportFrom):
                    assert node.module != "re"
