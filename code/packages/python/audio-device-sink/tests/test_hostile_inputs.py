"""Hostile input tests for CWE-20, CWE-190, CWE-400, and CWE-476 bounds (VER15)."""

from __future__ import annotations

import time
from collections.abc import Iterator, Sequence
from fractions import Fraction
from types import SimpleNamespace

import pytest
from audio_device_sink.audio_device_sink import _play_samples  # type: ignore[import]

import audio_device_sink as audio_device_sink_module
from audio_device_sink import (
    MAX_BLOCKING_DURATION_SECONDS,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    MIN_SAMPLE_RATE_HZ,
    PCM16_MAX,
    PCM16_MIN,
    play_pcm_buffer,
    play_samples,
)


class _VirtualLargeSequence(Sequence[int]):
    """Sized sequence reporting a huge length in O(1) without allocating RAM."""

    def __init__(self, virtual_length: int, val: int = 0) -> None:
        self._len = virtual_length
        self._val = val

    def __len__(self) -> int:
        return self._len

    def __getitem__(self, index: int) -> int:  # type: ignore[override]
        if index < 0 or index >= self._len:
            raise IndexError("index out of range")
        return self._val


class _ExplodingLenSequence(Sequence[int]):
    """Sequence whose __len__ overflows C ssize_t / sys.maxsize."""

    def __len__(self) -> int:
        raise OverflowError("cannot fit 'int' into an index-sized integer")

    def __getitem__(self, index: int) -> int:  # type: ignore[override]
        raise IndexError("unused")


def test_hostile_cwe_20_bool_rejection_on_int_params() -> None:
    for bad_bool in (True, False):
        with pytest.raises(TypeError, match="sample_rate_hz"):
            play_samples([], sample_rate_hz=bad_bool)
        with pytest.raises(TypeError, match="channel_count"):
            play_samples([], sample_rate_hz=44_100, channel_count=bad_bool)
        with pytest.raises(TypeError, match="samples\\[0\\]"):
            play_samples([bad_bool], sample_rate_hz=44_100)
        with pytest.raises(TypeError, match="bit_depth"):
            play_pcm_buffer(
                SimpleNamespace(
                    samples=[],
                    pcm_format=SimpleNamespace(
                        sample_rate_hz=44_100, channel_count=1, bit_depth=bad_bool
                    ),
                )
            )


def test_hostile_cwe_20_nan_inf_and_overflow_rates() -> None:
    for bad_float in (
        float("nan"),
        float("inf"),
        float("-inf"),
        44_100.5,
        -1.0,
        0.0,
        Fraction(10**500, 1),
        Fraction(44_100 * 10**20 + 1, 10**20),
    ):
        with pytest.raises(ValueError):
            play_samples([], sample_rate_hz=bad_float)

    for bad_int_rate in (0, -1, -44_100, MAX_SAMPLE_RATE_HZ + 1, 10**100):
        with pytest.raises(ValueError):
            play_samples([], sample_rate_hz=bad_int_rate)

    for bad_type in ("44100", None, complex(1, 0), object()):
        with pytest.raises(TypeError):
            play_samples([], sample_rate_hz=bad_type)

    assert MIN_SAMPLE_RATE_HZ == 1


def test_hostile_o1_error_formatting_on_huge_integers() -> None:
    # Exceeds Python's 4300-digit int_max_str_digits threshold; error formatting
    # must stay O(1) and raise ValueError cleanly rather than crashing on str(value).
    huge_int = 10**10_000
    t0 = time.monotonic()
    with pytest.raises(ValueError, match="out-of-range integer"):
        play_samples([huge_int], sample_rate_hz=44_100)
    with pytest.raises(ValueError, match="out-of-range integer"):
        play_samples([-huge_int], sample_rate_hz=44_100)
    elapsed = time.monotonic() - t0
    assert elapsed < 1.0


def test_hostile_cwe_400_preflight_bounds_and_streaming_caps() -> None:
    # Sized huge sequences must be rejected in O(1) before iteration
    t0 = time.monotonic()
    with pytest.raises(ValueError, match="limited"):
        play_samples(_VirtualLargeSequence(MAX_SAMPLES + 1), sample_rate_hz=44_100)
    with pytest.raises(ValueError, match="limited"):
        play_samples(range(10**12), sample_rate_hz=44_100)
    with pytest.raises(ValueError, match="limited"):
        play_samples(_ExplodingLenSequence(), sample_rate_hz=44_100)
    assert time.monotonic() - t0 < 1.0

    # Unsized generator exceeding cap at 1 Hz (limit = 600 samples)
    def gen(count: int) -> Iterator[int]:
        for _ in range(count):
            yield 0

    with pytest.raises(ValueError, match="limited"):
        play_samples(gen(601), sample_rate_hz=1)

    for bad_iterable in ("1234", b"\x00\x01", bytearray(b"\x00\x01"), 123, None):
        with pytest.raises(TypeError):
            play_samples(bad_iterable, sample_rate_hz=44_100)  # type: ignore[arg-type]


def test_hostile_native_ffi_boundary_guards() -> None:
    # Wrong arity / missing args (CWE-476 / CWE-20)
    with pytest.raises(TypeError):
        _play_samples()
    with pytest.raises(TypeError):
        _play_samples([], 44_100)
    with pytest.raises(TypeError):
        _play_samples([], 44_100, 1, "extra")

    # Non-integer / bool / out-of-range rate & channel at raw FFI boundary
    for bad_rate in (
        True,
        False,
        0,
        -1,
        44_100.0,
        MAX_SAMPLE_RATE_HZ + 1,
        10**100,
        None,
    ):
        with pytest.raises(TypeError):
            _play_samples([], bad_rate, 1)

    for bad_ch in (True, False, 0, 2, -1, 10**100, None):
        with pytest.raises(TypeError):
            _play_samples([], 44_100, bad_ch)

    # String/bytes/bytearray/non-iterable rejected with TypeError at raw FFI boundary
    for bad_type_samples in ("bad", b"\x00\x01", bytearray(b"\x00\x01"), None, 123):
        with pytest.raises(TypeError):
            _play_samples(bad_type_samples, 44_100, 1)

    # OverflowError on oversized __len__() propagates untouched in O(1)
    t0 = time.monotonic()
    with pytest.raises(OverflowError):
        _play_samples(_ExplodingLenSequence(), 44_100, 1)
    with pytest.raises(OverflowError):
        _play_samples(range(1 << 100), 44_100, 1)
    assert time.monotonic() - t0 < 1.0

    # Custom iterator exception propagates untouched instead of ValueError
    class _BrokenIterator:
        def __iter__(self) -> _BrokenIterator:
            return self

        def __next__(self) -> int:
            raise RuntimeError("iterator boom")

    with pytest.raises(RuntimeError, match="iterator boom"):
        _play_samples(_BrokenIterator(), 44_100, 1)

    # Sample bounds and O(1) pre-flight size cap at raw FFI boundary
    for bad_samples in (
        [True],
        [False],
        [PCM16_MIN - 1],
        [PCM16_MAX + 1],
        [10**100],
        [1.5],
        _VirtualLargeSequence(MAX_SAMPLES + 1),
        range(10**12),
        (0 for _ in range(601)),
    ):
        rate = 1 if isinstance(bad_samples, Iterator) else 44_100
        with pytest.raises(ValueError):
            _play_samples(bad_samples, rate, 1)


def test_hostile_play_pcm_buffer_rejects_bad_bit_depth_and_bad_native_report(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    bad_buf = SimpleNamespace(
        samples=[0],
        pcm_format=SimpleNamespace(
            sample_rate_hz=44_100, channel_count=1, bit_depth=24
        ),
    )
    with pytest.raises(ValueError, match="16-bit"):
        play_pcm_buffer(bad_buf)

    monkeypatch.setattr(
        audio_device_sink_module,
        "_play_samples",
        lambda _s, r, c: (1, r, c, float("nan"), "fake"),
    )
    with pytest.raises(audio_device_sink_module.AudioDeviceError, match="non-finite"):
        play_samples([0], sample_rate_hz=44_100)

    assert MAX_BLOCKING_DURATION_SECONDS == 600.0
