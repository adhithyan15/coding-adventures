"""Hostile input testing for CWE-20, CWE-190, CWE-400, and CWE-1333 (VER15)."""

from __future__ import annotations

import ast
import itertools
import math
import struct
import sys
from collections.abc import Iterator
from fractions import Fraction
from pathlib import Path

import pytest
from oscillator import SampleBuffer

from pcm_audio import (
    MAX_CHANNELS,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    MAX_WAV_BYTES,
    PCMBuffer,
    PCMFormat,
    _safe_round_sample,
    encode_sample_buffer,
    from_wav_bytes,
    samples_to_pcm_buffer,
    validate_wav_header,
)


class _LyingSizedIterable:
    """Iterable whose __len__ lies (returns 1) while yielding > MAX_SAMPLES."""

    def __len__(self) -> int:
        return 1

    def __iter__(self) -> Iterator[int]:
        return itertools.repeat(0)


def _build_wav(
    *,
    channels: int = 1,
    sample_rate: int = 44_100,
    bits_per_sample: int = 16,
    audio_format: int = 1,
    pcm_data: bytes = b"\x00\x00\x00\x00",
) -> bytes:
    block_align = channels * (bits_per_sample // 8)
    byte_rate = sample_rate * block_align
    fmt = struct.pack(
        "<4sIHHIIHH",
        b"fmt ",
        16,
        audio_format,
        channels,
        sample_rate,
        byte_rate,
        block_align,
        bits_per_sample,
    )
    data = struct.pack("<4sI", b"data", len(pcm_data)) + pcm_data
    riff_len = 4 + len(fmt) + len(data)
    return struct.pack("<4sI4s", b"RIFF", riff_len, b"WAVE") + fmt + data


class TestCwe20TypeAndNumericValidation:
    @pytest.mark.parametrize(
        "bad_val",
        [
            True,
            False,
            float("nan"),
            float("inf"),
            float("-inf"),
            "0.5",
            None,
            complex(0.5, 0.0),
        ],
    )
    def test_rejects_bool_non_finite_and_non_real(self, bad_val: object) -> None:
        with pytest.raises(ValueError):
            _safe_round_sample(bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            PCMFormat(sample_rate_hz=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            PCMFormat(full_scale_voltage=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            PCMFormat(channel_count=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            PCMFormat(bit_depth=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            PCMBuffer((0,), PCMFormat(), start_time_seconds=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            PCMBuffer((0,), PCMFormat(), clipped_sample_count=bad_val)  # type: ignore[arg-type]

    @pytest.mark.parametrize(
        "huge_real",
        [10**400, -(10**400), Fraction(10**400, 3), Fraction(-(10**400), 3)],
        ids=["int-pos", "int-neg", "frac-pos", "frac-neg"],
    )
    def test_huge_real_overflows_raise_value_error_not_overflow_error(
        self, huge_real: object
    ) -> None:
        with pytest.raises(ValueError, match="overflows float range"):
            _safe_round_sample(huge_real)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="overflows float range"):
            PCMFormat(sample_rate_hz=huge_real)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="overflows float range"):
            PCMFormat(full_scale_voltage=huge_real)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="overflows float range"):
            samples_to_pcm_buffer((huge_real,), sample_rate_hz=44_100.0)  # type: ignore[arg-type]

    def test_subnormal_sample_rate_rejected(self) -> None:
        with pytest.raises(ValueError, match="subnormal"):
            PCMFormat(sample_rate_hz=5e-324)


class TestCwe190AndCorruptWavHeaders:
    @pytest.mark.parametrize(
        "huge_int",
        [10**5000, -(10**5000)],
        ids=["pos-10e5000", "neg-10e5000"],
    )
    def test_huge_ints_fail_in_o1_via_safe_repr(self, huge_int: int) -> None:
        with pytest.raises(ValueError, match="<out-of-range int>") as exc1:
            PCMBuffer((huge_int,), PCMFormat())
        assert len(str(exc1.value)) < 160

        buf = PCMBuffer((0, 1), PCMFormat())
        with pytest.raises(ValueError, match="<out-of-range int>") as exc2:
            buf.time_at(huge_int)
        assert len(str(exc2.value)) < 160

        with pytest.raises(ValueError, match="<out-of-range int>") as exc3:
            PCMBuffer((0,), PCMFormat(), clipped_sample_count=-abs(huge_int))
        assert len(str(exc3.value)) < 160

    def test_safe_repr_formats_long_and_short_strings(self) -> None:
        with pytest.raises(ValueError, match="<str len=5000>") as exc:
            _safe_round_sample("a" * 5000)  # type: ignore[arg-type]
        assert len(str(exc.value)) < 160
        with pytest.raises(ValueError, match="'bad'"):
            _safe_round_sample("bad")  # type: ignore[arg-type]

    def test_corrupt_riff_size_overflow_and_duplicate_fmt_rejected(self) -> None:
        # 0xFFFFFFFF RIFF payload size
        huge_riff = struct.pack("<4sI4s", b"RIFF", 0xFFFF_FFFF, b"WAVE") + b"\x00" * 32
        with pytest.raises(ValueError, match="MAX_WAV_BYTES"):
            validate_wav_header(huge_riff)

        # RIFF size larger than buffer but <= MAX_WAV_BYTES
        over_riff = struct.pack("<4sI4s", b"RIFF", 1000, b"WAVE") + b"\x00" * 32
        with pytest.raises(ValueError, match="exceeds buffer"):
            validate_wav_header(over_riff)

        # Chunk size 0x7FFFFFFF inside valid RIFF wrapper
        fmt = struct.pack("<4sIHHIIHH", b"fmt ", 16, 1, 1, 44_100, 88_200, 2, 16)
        bad_data = struct.pack("<4sI", b"data", 0x7FFF_FFFF) + b"\x00\x00"
        hdr = struct.pack("<4sI4s", b"RIFF", 4 + len(fmt) + len(bad_data), b"WAVE")
        with pytest.raises(ValueError, match="exceeds buffer"):
            validate_wav_header(hdr + fmt + bad_data)

        # Duplicate fmt chunk
        data = struct.pack("<4sI", b"data", 4) + b"\x00\x00\x00\x00"
        dup_hdr = struct.pack(
            "<4sI4s", b"RIFF", 4 + len(fmt) * 2 + len(data), b"WAVE"
        )
        with pytest.raises(ValueError, match="duplicate fmt chunk"):
            validate_wav_header(dup_hdr + fmt + fmt + data)

        # Sample rate > MAX_SAMPLE_RATE_HZ in WAV header
        bad_sr = _build_wav(sample_rate=int(MAX_SAMPLE_RATE_HZ) + 1)
        with pytest.raises(ValueError, match="outside"):
            validate_wav_header(bad_sr)

        # Channel count > MAX_CHANNELS in WAV header
        bad_ch = _build_wav(channels=MAX_CHANNELS + 1)
        with pytest.raises(ValueError, match="unsupported channel count"):
            validate_wav_header(bad_ch)

    @pytest.mark.parametrize("bad_f32", [math.nan, math.inf, -math.inf])
    def test_from_wav_bytes_rejects_non_finite_ieee_float_samples(
        self, bad_f32: float
    ) -> None:
        wav_bytes = _build_wav(
            audio_format=3,
            bits_per_sample=32,
            pcm_data=struct.pack("<f", bad_f32),
        )
        with pytest.raises(ValueError, match="must be finite"):
            from_wav_bytes(wav_bytes)


class TestCwe400ResourceExhaustionAndGeneratorDos:
    def test_pcm_buffer_rejects_string_bytes_and_non_iterables(self) -> None:
        for bad in ("0", b"\x00\x00", bytearray(b"\x00\x00"), 123, None):
            with pytest.raises(ValueError, match="iterable"):
                PCMBuffer(samples=bad, pcm_format=PCMFormat())  # type: ignore[arg-type]
            with pytest.raises(ValueError, match="iterable"):
                samples_to_pcm_buffer(bad, sample_rate_hz=44_100.0)  # type: ignore[arg-type]

    def test_pcm_buffer_aborts_infinite_generator_without_oom(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        import pcm_audio.pcm_audio as pcm_mod

        monkeypatch.setattr(pcm_mod, "MAX_SAMPLES", 12)
        yielded = 0

        def _gen() -> Iterator[int]:
            nonlocal yielded
            while True:
                yielded += 1
                yield 0

        with pytest.raises(ValueError, match="MAX_SAMPLES"):
            PCMBuffer(samples=_gen(), pcm_format=PCMFormat())  # type: ignore[arg-type]
        assert yielded == 13

    def test_pcm_buffer_aborts_lying_sized_iterable(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        import pcm_audio.pcm_audio as pcm_mod

        monkeypatch.setattr(pcm_mod, "MAX_SAMPLES", 6)
        with pytest.raises(ValueError, match="MAX_SAMPLES"):
            PCMBuffer(
                samples=_LyingSizedIterable(),  # type: ignore[arg-type]
                pcm_format=PCMFormat(),
            )

    def test_samples_to_pcm_buffer_preflight_sized_bound(self) -> None:
        class _HugeSized:
            def __len__(self) -> int:
                return MAX_SAMPLES + 1

            def __iter__(self) -> Iterator[float]:
                raise AssertionError(
                    "should not iterate when Sized exceeds MAX_SAMPLES"
                )

        with pytest.raises(ValueError, match="MAX_SAMPLES"):
            samples_to_pcm_buffer(_HugeSized(), sample_rate_hz=44_100.0)

    def test_samples_to_pcm_buffer_aborts_infinite_generator(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        import pcm_audio.pcm_audio as pcm_mod

        monkeypatch.setattr(pcm_mod, "MAX_SAMPLES", 8)
        yielded = 0

        def _gen() -> Iterator[float]:
            nonlocal yielded
            while True:
                yielded += 1
                yield 0.0

        with pytest.raises(ValueError, match="MAX_SAMPLES"):
            samples_to_pcm_buffer(_gen(), sample_rate_hz=44_100.0)
        assert yielded == 9

    def test_encode_sample_buffer_rejects_oversized_and_non_buffer(self) -> None:
        with pytest.raises(ValueError, match="SampleBuffer"):
            encode_sample_buffer("bad")  # type: ignore[arg-type]
        assert MAX_WAV_BYTES == 67_108_864

    def test_sized_len_overflow_raises_value_error_not_overflow_error(self) -> None:
        class _OverflowLenSized:
            def __len__(self) -> int:
                return 10**100

            def __iter__(self) -> Iterator[int]:
                return iter(())

        with pytest.raises(ValueError, match="MAX_SAMPLES"):
            PCMBuffer(
                samples=_OverflowLenSized(),  # type: ignore[arg-type]
                pcm_format=PCMFormat(),
            )
        with pytest.raises(ValueError, match="MAX_SAMPLES"):
            samples_to_pcm_buffer(
                _OverflowLenSized(),  # type: ignore[arg-type]
                sample_rate_hz=44_100.0,
            )

    def test_pcm_buffer_time_at_rejects_non_finite_overflow(self) -> None:
        buf = PCMBuffer(
            (0, 0),
            PCMFormat(sample_rate_hz=1e-308),
            start_time_seconds=sys.float_info.max,
        )
        with pytest.raises(ValueError, match="non-finite"):
            buf.time_at(1)

    def test_encode_sample_buffer_validates_pcm_format_before_iterating(
        self,
    ) -> None:
        sbuf = SampleBuffer(samples=(0.0, 0.5), sample_rate_hz=44_100.0)
        with pytest.raises(ValueError, match="pcm_format must be a PCMFormat"):
            encode_sample_buffer(sbuf, pcm_format="bad")  # type: ignore[arg-type]


class TestCwe1333ZeroRegexImports:
    def test_no_regex_imports_in_source(self) -> None:
        pkg_src = Path(__file__).resolve().parents[1] / "src" / "pcm_audio"
        for py_file in pkg_src.rglob("*.py"):
            tree = ast.parse(py_file.read_text(encoding="utf-8"))
            for node in ast.walk(tree):
                if isinstance(node, ast.Import):
                    assert all(alias.name != "re" for alias in node.names)
                elif isinstance(node, ast.ImportFrom):
                    assert node.module != "re"
