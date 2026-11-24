"""Tests for the VER06d Python fuzz engine: mutators, campaign, coverage."""

from __future__ import annotations

import importlib.util
import sys
import time
from pathlib import Path
from typing import Any

import pytest

from coding_adventures_test_bootstrap import fuzz
from coding_adventures_test_bootstrap.fuzz import (
    COVERAGE_BYTES,
    FUZZ_FAILURE_IDS,
    MAX_INPUT_BYTES,
    TIER_BLACK_BOX,
    TIER_MONITORING,
    TIER_REPORTED,
    TIER_SETTRACE,
    CoverageBitmap,
    CrashRecord,
    FuzzOptions,
    FuzzReport,
    Harness,
    OutcomeCrash,
    OutcomeOom,
    OutcomePass,
    OutcomeTimeout,
    bit_flip,
    block_delete,
    block_duplicate,
    block_insert,
    boundary_insert,
    byte_arithmetic,
    byte_flip,
    calculate_energy,
    corpus_admit,
    crash_signature,
    default_scope,
    format_tap_failure,
    frames_of,
    fuzz_bytes,
    fuzz_function,
    fuzz_target,
    fuzz_text,
    is_duplicate_crash,
    is_fuzz_failure_id,
    minimize_crashing_input,
    mutate_sequence,
    splice,
    tap_failures,
)
from coding_adventures_test_bootstrap.prng import SplitMix64


def _reference() -> Any:
    """The independent reference oracle, ``code/scripts/fuzz_conformance.py``."""
    for parent in Path(__file__).resolve().parents:
        script = parent / "code/scripts/fuzz_conformance.py"
        if script.is_file():
            spec = importlib.util.spec_from_file_location("fuzz_conformance", script)
            assert spec and spec.loader
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            return module
    raise AssertionError("repository root with fuzz_conformance.py not found")


REF = _reference()


# --- mutators: differential against the reference --------------------------------


def _inputs(rng: SplitMix64, count: int, max_len: int) -> list[bytes]:
    out = [b""]
    for _ in range(count):
        n = rng.bounded(max_len + 1)
        out.append(bytes(rng.bounded(256) for _ in range(n)))
    return out


def test_mutators_match_the_reference_on_random_inputs() -> None:
    rng = SplitMix64(7)
    for data in _inputs(rng, 150, 10):
        off = rng.bounded(40) - 5  # negative offsets wrap too
        count = rng.bounded(40)
        assert bit_flip(data, off, count) == REF.bit_flip(data, off, count)
        assert byte_flip(data, off, count) == REF.byte_flip(data, off, count)
        for width in (1, 2, 4, 8):
            for endian in ("little", "big"):
                delta = rng.bounded(1000) - 500
                assert byte_arithmetic(data, off, delta, width, endian) == (
                    REF.byte_arithmetic(data, off, delta, width, endian)
                )
                for val in fuzz.INTEREST_BY_WIDTH[width]:
                    assert boundary_insert(data, off, width, val, endian) == (
                        REF.boundary_insert(data, off, width, val, endian)
                    )
        length = rng.bounded(8)
        ins = bytes([rng.bounded(256), rng.bounded(256)])
        assert block_delete(data, off, length) == REF.block_delete(data, off, length)
        assert block_insert(data, off, ins) == REF.block_insert(data, off, ins)
        dst = rng.bounded(30)
        assert block_duplicate(data, off, length, dst) == (
            REF.block_duplicate(data, off, length, dst)
        )
        other = bytes(range(rng.bounded(6)))
        assert splice(data, other, off, dst) == REF.splice(data, other, off, dst)


def test_mutate_sequence_matches_the_reference_including_one_byte_buffers() -> None:
    rng = SplitMix64(99)
    for data in _inputs(rng, 80, 6):
        for steps in range(0, 9):
            seed = rng.next_u64()
            assert mutate_sequence(data, seed, steps) == (
                REF.mutate_sequence(data, seed, steps)
            ), (data.hex(), seed, steps)
    # a stream that shrinks to one byte exercises bounded(1): one word consumed
    one = b"\x05"
    for seed in range(300):
        assert mutate_sequence(one, seed, 6) == REF.mutate_sequence(one, seed, 6)


# --- mutators: contracts ---------------------------------------------------------


def test_flips_apply_by_parity_for_enormous_counts() -> None:
    usize_max = 2**64 - 1
    assert bit_flip(b"\x00\x00", 0, usize_max) == b"\x00\x80"
    assert bit_flip(b"\x00\x00", usize_max, 2) == b"\x01\x80"
    assert byte_flip(b"\x12\x34\x56", usize_max, 2) == b"\xed\xcb\x56"
    assert bit_flip(b"\x00", 0, 10**30) == bit_flip(b"\x00", 0, 10**30 % 8)
    assert bit_flip(b"\x00\x00", 0, 17) == b"\xfe\xff"  # bit 0 twice, rest once
    t0 = time.monotonic()
    bit_flip(bytes(100_000), 3, 10**18 + 5)
    assert time.monotonic() - t0 < 5  # O(len), not O(count)


def test_flips_are_involutions_and_keep_length() -> None:
    for n in range(3):
        for data in (b"", b"\x00", b"\xa5", b"\x00\x00", b"\x12\xff"):
            data = data[:n]
            for off in (0, 1, 9, 2**64 - 1):
                for count in (0, 1, 4, 17, 2**64 - 1):
                    once = bit_flip(data, off, count)
                    assert len(once) == len(data)
                    assert bit_flip(once, off, count) == data
                    twice = byte_flip(data, off, count)
                    assert len(twice) == len(data)
                    assert byte_flip(twice, off, count) == data


def test_flip_edge_cases() -> None:
    assert bit_flip(b"", 0, 5) == b""
    assert bit_flip(b"a", 0, 0) == b"a"
    assert bit_flip(b"a", 0, -3) == b"a"
    assert byte_flip(b"\x00", 0, 2) == b"\x00"
    assert bit_flip(bytearray(b"\x00"), 0, 1) == b"\x01"  # bytes-like accepted


def test_arithmetic_wraps_and_ignores_bad_widths() -> None:
    assert byte_arithmetic(b"\xff\x00", 0, 1, 2, "little") == b"\x00\x01"
    assert byte_arithmetic(b"\x00\x00", 0, -1, 1) == b"\xff\x00"
    assert byte_arithmetic(b"\x00" * 3, 0, 1, 3) == b"\x00" * 3  # width 3: no-op
    assert byte_arithmetic(b"\x00", 0, 1, 2) == b"\x00"  # too short
    assert byte_arithmetic(b"\x00" * 8, 0, -1, 8, "big") == b"\xff" * 8
    undone = byte_arithmetic(byte_arithmetic(b"\x10\x20", 1, 35, 1), 1, -35, 1)
    assert undone == b"\x10\x20"


def test_boundary_truncates_to_the_width() -> None:
    assert boundary_insert(b"\x00" * 4, 0, 4, 0x80000000) == b"\x00\x00\x00\x80"
    assert boundary_insert(b"\x00" * 2, 0, 2, 2**16 + 5) == b"\x05\x00"
    assert boundary_insert(b"\x00" * 2, 0, 2, -1) == b"\xff\xff"
    assert boundary_insert(b"\x00" * 2, 0, 3, 1) == b"\x00\x00"
    assert boundary_insert(b"\x00", 0, 8, 1) == b"\x00"


def test_interest_lists_are_the_spec_ones_and_fit_their_width() -> None:
    assert fuzz.INTEREST_8 == (0, 1, 127, 128, 255)
    assert fuzz.INTEREST_16 == (0, 1, 255, 256, 32767, 32768, 65535)
    assert fuzz.INTEREST_32[-3:] == (0x7FFFFFFF, 0x80000000, 0xFFFFFFFF)
    assert fuzz.INTEREST_64[-1] == 2**64 - 1
    for width, values in fuzz.INTEREST_BY_WIDTH.items():
        assert all(0 <= v < 2 ** (8 * width) for v in values)


def test_block_operations_edge_cases() -> None:
    assert block_delete(b"", 0, 3) == b""
    assert block_delete(b"abc", 0, 0) == b"abc"
    assert block_delete(b"abc", 2, 99) == b"ab"
    assert block_insert(b"", 5, b"xy") == b"xy"
    assert block_insert(b"ab", 2, b"X") == b"abX"
    assert block_duplicate(b"", 0, 2, 0) == b""
    assert block_duplicate(b"ab", 0, 0, 0) == b"ab"
    assert block_duplicate(b"ab", 1, 9, 2) == b"abb"
    assert splice(b"abcd", b"wxyz", -5, 99) == b""
    assert splice(b"abcd", b"wxyz", 99, -5) == b"abcdwxyz"


def test_strict_argument_types() -> None:
    for call in (
        lambda: bit_flip(b"a", True, 1),
        lambda: bit_flip(b"a", 0, 1.0),  # type: ignore[arg-type]
        lambda: byte_flip("a", 0, 1),  # type: ignore[arg-type]
        lambda: block_insert(b"a", 0, "x"),  # type: ignore[arg-type]
        lambda: splice(b"a", None, 0, 0),  # type: ignore[arg-type]
        lambda: mutate_sequence(b"a", 1, True),
    ):
        with pytest.raises(TypeError):
            call()
    with pytest.raises(ValueError, match="endian"):
        byte_arithmetic(b"abc", 0, 1, 1, "middle")
    with pytest.raises(ValueError, match="endian"):
        boundary_insert(b"abc", 0, 1, 1, "")


def test_mutate_sequence_bounds_growth_and_steps() -> None:
    grown = mutate_sequence(b"\x01" * 4, 5, 5000, max_len=64)
    assert len(grown) <= 64
    # the cap skips growth after drawing, so the capped stream is a prefix-compatible
    # replay: uncapped and capped agree while the buffer is below the cap
    assert mutate_sequence(b"abcd", 3, 4) == mutate_sequence(b"abcd", 3, 4, max_len=99)
    with pytest.raises(ValueError, match="steps"):
        mutate_sequence(b"a", 1, fuzz.MAX_ITERATIONS + 1)
    assert mutate_sequence(b"a", 1, -3) == b"a"
    assert mutate_sequence(b"", 1, 0) == b""


# --- bitmap, admission, energy ---------------------------------------------------


def test_bitmap_records_masks_and_counts() -> None:
    bm = CoverageBitmap()
    assert bm.count() == 0 and bm.raw == bytes(COVERAGE_BYTES)
    bm.record_edge(65_536 + 9)  # masked to 9
    assert bm.has_edge(9) and bm.has_edge(65_536 + 9) and not bm.has_edge(10)
    assert bm.raw[1] == 0b10
    assert bm.admit_trace([9, 10, 10, 11]) == (True, 2)
    assert bm.admit_trace([9, 10]) == (False, 0)
    assert bm.count() == 3 and bm.covered_edges() == [9, 10, 11]
    other = CoverageBitmap()
    other.record_edge(500)
    bm.merge(other)
    assert bm.has_edge(500) and bm.covered_edges() == [9, 10, 11, 500]
    with pytest.raises(TypeError):
        bm.record_edge(1.5)  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        bm.admit_trace([True])


def test_corpus_admit_counts_unique_novel_edges() -> None:
    assert corpus_admit([1, 2, 3], [2, 4]) == (True, 1)
    assert corpus_admit([1, 2, 3], [1, 2]) == (False, 0)
    assert corpus_admit([], [7, 7, 7]) == (True, 1)
    assert corpus_admit([], []) == (False, 0)


def test_energy_schedule() -> None:
    assert calculate_energy(10, 4, 3, 6) == 8  # 7.5 rounds half away from zero
    assert calculate_energy(5, 4, 0, 0) == 3  # 2.5: Python round() would give 2
    assert calculate_energy(10, 0, 0, 0) == 10
    assert calculate_energy(0, 10**6, 0, 5) == 1  # never below 1
    assert calculate_energy(10, 1, 6, 6) == 20
    assert calculate_energy(10, 16, 0, 3) > calculate_energy(10, 64, 0, 3)


# --- signatures, TAP, vocabulary -------------------------------------------------


def test_signatures_and_dedup() -> None:
    sig = crash_signature("panic", ["eval", "step"])
    assert len(sig) == 8 and sig == sig.lower()
    assert sig == REF.crash_signature("panic", ["eval", "step"])
    assert is_duplicate_crash("panic", ["eval"], "panic", ["eval"])
    assert not is_duplicate_crash("panic", ["eval"], "segv", ["eval"])
    assert crash_signature("k", []) == REF.crash_signature("k", [])
    with pytest.raises(TypeError):
        crash_signature(1, [])  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        crash_signature("k", "abc")
    with pytest.raises(TypeError):
        crash_signature("k", [1])  # type: ignore[list-item]


def test_failure_vocabulary_is_closed() -> None:
    assert FUZZ_FAILURE_IDS == (
        "fuzz-crash",
        "fuzz-timeout",
        "fuzz-oom",
        "fuzz-leak",
        "fuzz-panic",
    )
    assert all(is_fuzz_failure_id(i) for i in FUZZ_FAILURE_IDS)
    for bad in ("fuzz-arbitrary-error", "", "FUZZ-CRASH", None, 5, b"fuzz-crash"):
        assert not is_fuzz_failure_id(bad)


def test_tap_line_is_opaque_and_unforgeable() -> None:
    assert format_tap_failure(1, "ct_compare", "fuzz-crash", "3a8f19b2") == (
        "not ok 1 - ct_compare [fuzz-crash:3a8f19b2]"
    )
    forged = format_tap_failure(2, "t\nok 3 - forged", "fuzz-crash\r\n", "AB\nzz12")
    assert "\n" not in forged and "\r" not in forged
    assert forged == "not ok 2 - t_ok_3_-_forged [fuzz-crash__:_____12]"
    with pytest.raises(TypeError):
        format_tap_failure("1", "t", "fuzz-crash", "00000000")  # type: ignore[arg-type]


def test_tap_failures_render_every_bucket_without_payload() -> None:
    secret = b"SECRETPAYLOAD"
    report = FuzzReport(
        iterations=1,
        admitted_seeds=1,
        total_edges=0,
        crashes=[CrashRecord("deadbeef", "K", ("m.f",), secret, 3)],
        timeouts=2,
        ooms=1,
        corpus=(secret,),
    )
    lines = tap_failures(report, "tgt", first_test=5)
    assert lines[0] == "not ok 5 - tgt [fuzz-crash:deadbeef]"
    assert lines[1].startswith("not ok 6 - tgt [fuzz-timeout:")
    assert lines[2].startswith("not ok 7 - tgt [fuzz-oom:")
    blob = "\n".join(lines) + repr(report) + repr(report.crashes)
    assert "SECRET" not in blob and secret.hex() not in blob
    clean = FuzzReport(1, 1, 0, [])
    assert tap_failures(clean, "tgt") == []


# --- minimisation ----------------------------------------------------------------


def _crash(kind: str, *frames: str) -> OutcomeCrash:
    return OutcomeCrash(kind, tuple(frames))


def test_minimizer_reaches_the_empty_input() -> None:
    sig = crash_signature("K", ["f"])
    always = lambda _d: _crash("K", "f")  # noqa: E731
    assert minimize_crashing_input(b"\x07", sig, always) == b""
    assert minimize_crashing_input(b"abcdef", sig, always) == b""
    assert minimize_crashing_input(b"", sig, always) == b""


def test_minimizer_keeps_the_exact_signature_and_zeroes_bytes() -> None:
    sig = crash_signature("K", ["f"])

    def target(data: bytes) -> Any:
        return _crash("K", "f") if b"XY" in data else OutcomePass()

    assert minimize_crashing_input(b"aaXYbb", sig, target) == b"XY"

    def zeroing(data: bytes) -> Any:  # crashes while byte 0 is nonzero or len > 2
        return _crash("K", "f") if len(data) >= 3 and data[0] != 0 else OutcomePass()

    assert minimize_crashing_input(b"\x09\x08\x07\x06", sig, zeroing) == b"\x08\x00\x00"


def test_minimizer_never_trades_one_crash_for_another() -> None:
    sig = crash_signature("A", ["f"])

    def target(data: bytes) -> Any:
        if data == b"abc":
            return _crash("A", "f")
        if len(data) < 3:
            return _crash("B", "g")  # a different, easier crash
        return OutcomePass()

    assert minimize_crashing_input(b"abc", sig, target) == b"abc"


def test_minimizer_budget_and_non_crashes() -> None:
    sig = crash_signature("K", ["f"])
    calls = []

    def target(data: bytes) -> Any:
        calls.append(data)
        return _crash("K", "f") if data else OutcomePass()

    out = minimize_crashing_input(b"x" * 64, sig, target, budget=3)
    assert len(calls) == 3 and len(out) > 0
    calls.clear()
    minimize_crashing_input(b"\x01" * 4, sig, target, budget=0)
    assert calls == []
    assert minimize_crashing_input(b"zz", sig, lambda _d: OutcomeTimeout()) == b"zz"
    # budget exhausted during the zeroing pass
    pinned = lambda d: _crash("K", "f") if d == b"\x01\x02\x03" else OutcomePass()  # noqa: E731
    # five executions: 3 block deletions, then the zeroing pass is cut short
    assert minimize_crashing_input(b"\x01\x02\x03", sig, pinned, budget=4) == (
        b"\x01\x02\x03"
    )


# --- campaign contracts (VER06 9.1), with synthetic targets ----------------------


def _blind(outcome_for: Any = None) -> Any:
    def target(data: bytes, cov: CoverageBitmap) -> Any:
        return outcome_for(data, cov) if outcome_for else OutcomePass()

    return target


def test_campaign_is_deterministic_and_seed_sensitive() -> None:
    def tgt(data: bytes, cov: CoverageBitmap) -> Any:
        cov.record_edge(len(data) % 7)
        cov.record_edge(100 + (data[0] % 5 if data else 0))
        return _crash("K", "f") if data[:1] == b"\xff" else OutcomePass()

    opts = FuzzOptions(max_iterations=400, seed=5)
    a = fuzz_target([b"\x01\x02"], opts, tgt)
    b = fuzz_target([b"\x01\x02"], opts, tgt)
    assert a == b and a.total_edges > 0
    c = fuzz_target([b"\x01\x02"], FuzzOptions(max_iterations=400, seed=6), tgt)
    assert (c.corpus, c.crashes) != (a.corpus, a.crashes)


def test_budget_length_cap_and_empty_corpus() -> None:
    seen: list[int] = []

    def tgt(data: bytes, _cov: CoverageBitmap) -> Any:
        seen.append(len(data))
        return OutcomePass()

    report = fuzz_target([], FuzzOptions(max_iterations=250, max_input_len=3), tgt)
    assert report.iterations == 250 and report.is_clean
    assert seen[0] == 1  # the replacement [00] seed ran first
    assert max(seen[1:]) <= 3  # mutated candidates are truncated
    long_seed = b"\x01" * 50  # initial seeds run as given
    seen.clear()
    fuzz_target([long_seed], FuzzOptions(max_iterations=5, max_input_len=3), tgt)
    assert seen[0] == 50 and max(seen[1:]) <= 3
    for n in (1, 7, 333):
        r = fuzz_target([b"ab"], FuzzOptions(max_iterations=n, base_energy=1000), tgt)
        assert r.iterations == n


def test_crashing_seeds_are_not_mutated_and_never_clean() -> None:
    calls: list[bytes] = []

    def tgt(data: bytes, _cov: CoverageBitmap) -> Any:
        calls.append(data)
        return _crash("K", "f")

    r = fuzz_target([b"a", b"bb"], FuzzOptions(max_iterations=500), tgt)
    assert r.iterations == 0 and not r.is_clean
    assert len(r.crashes) == 1 and r.crashes[0].hits == 2
    assert r.crashes[0].minimal_input == b""
    assert r.admitted_seeds == 0
    assert calls[0] == b"a" and b"bb" in calls  # both seeds ran; none was mutated


def test_every_execution_lands_in_a_bucket() -> None:
    plan = {b"\x01": OutcomeTimeout(), b"\x02": OutcomeOom(), b"\x03": _crash("K", "f")}

    def tgt(data: bytes, _cov: CoverageBitmap) -> Any:
        return plan.get(data, OutcomePass())

    r = fuzz_target([b"\x00", b"\x01", b"\x02", b"\x03"], FuzzOptions(0), tgt)
    assert (r.timeouts, r.ooms, len(r.crashes), r.admitted_seeds) == (1, 1, 1, 1)
    assert r.iterations == 0 and not r.is_clean


def test_is_clean_requires_iterations_and_no_failure_of_any_kind() -> None:
    ok = fuzz_target([b"a"], FuzzOptions(max_iterations=5), _blind())
    assert ok.is_clean and ok.iterations == 5
    zero = fuzz_target([b"a"], FuzzOptions(max_iterations=0), _blind())
    assert not zero.is_clean and zero.iterations == 0
    # a target that times out on every input is not clean
    only_timeouts = fuzz_target(
        [b"a"], FuzzOptions(max_iterations=0), _blind(lambda d, c: OutcomeTimeout())
    )
    assert only_timeouts.timeouts == 1 and not only_timeouts.is_clean
    assert only_timeouts.iterations == 0

    def late_timeout(data: bytes, _cov: CoverageBitmap) -> Any:
        return OutcomeTimeout() if len(data) != 1 else OutcomePass()

    r = fuzz_target([b"a"], FuzzOptions(max_iterations=300), late_timeout)
    assert r.timeouts > 0 and not r.is_clean and r.iterations == 300

    def late_oom(data: bytes, _cov: CoverageBitmap) -> Any:
        return OutcomeOom() if len(data) != 1 else OutcomePass()

    r = fuzz_target([b"a"], FuzzOptions(max_iterations=300), late_oom)
    assert r.ooms > 0 and not r.is_clean


def test_dedup_one_record_per_signature_with_correct_hits() -> None:
    def tgt(data: bytes, _cov: CoverageBitmap) -> Any:
        if not data:
            return OutcomePass()
        return _crash("K", f"f{data[0] % 2}") if data[0] >= 0x80 else OutcomePass()

    seeds = [b"\x01", b"\x90\x01", b"\x91", b"\x92\x00\x00"]
    r = fuzz_target(seeds, FuzzOptions(max_iterations=0), tgt)
    # f0 (0x90, 0x92) and f1 (0x91): two signatures, hits 2 and 1
    sigs = {c.signature: c for c in r.crashes}
    assert len(r.crashes) == 2 == len(sigs)
    f0 = sigs[crash_signature("K", ["f0"])]
    f1 = sigs[crash_signature("K", ["f1"])]
    assert (f0.hits, f1.hits) == (2, 1)
    assert f0.minimal_input == b"\x90" and f1.minimal_input == b"\x91"
    # in the mutation loop hits keep counting and the shorter input is kept
    r2 = fuzz_target([b"\x01"], FuzzOptions(max_iterations=2000, seed=3), tgt)
    assert all(c.hits >= 1 and len(c.minimal_input) <= 1 for c in r2.crashes)
    assert sum(c.hits for c in r2.crashes) > len(r2.crashes)


def test_hits_keep_the_shorter_input(monkeypatch: pytest.MonkeyPatch) -> None:
    long_crash = b"\x90" + b"\x01" * 5

    def tgt(data: bytes, _cov: CoverageBitmap) -> Any:
        # an unminimisable crash: only these exact two inputs reproduce it
        if data in (long_crash, b"\x90\x01"):
            return _crash("K", "f")
        return OutcomePass()

    r = fuzz_target([long_crash, b"\x90\x01"], FuzzOptions(max_iterations=0), tgt)
    assert len(r.crashes) == 1 and r.crashes[0].hits == 2
    assert r.crashes[0].minimal_input == b"\x90\x01"


def test_crash_cap_counts_dropped_signatures(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(fuzz, "MAX_CRASHES", 2)

    def tgt(data: bytes, _cov: CoverageBitmap) -> Any:
        return _crash(f"K{data[0] if data else 0}", "f")

    r = fuzz_target([b"\x01", b"\x02", b"\x03", b"\x04"], FuzzOptions(0), tgt)
    assert len(r.crashes) == 2 and r.crashes_dropped == 2
    assert r.unique_crashes == 4 and not r.is_clean


def test_coverage_admission_grows_the_corpus() -> None:
    def tgt(data: bytes, cov: CoverageBitmap) -> Any:
        cov.record_edge(len(data))
        return OutcomePass()

    r = fuzz_target([b"a"], FuzzOptions(max_iterations=500, max_input_len=64), tgt)
    assert r.admitted_seeds > 1 and r.total_edges == r.admitted_seeds
    assert r.coverage_tier == TIER_REPORTED
    # the first passing seed is admitted even without coverage (black-box)
    bb = fuzz_target([b"a", b"b"], FuzzOptions(max_iterations=10), _blind())
    assert bb.admitted_seeds == 1 and bb.coverage_tier == TIER_BLACK_BOX


def test_campaign_argument_validation() -> None:
    for bad in ("abc", b"abc", 5, None):
        with pytest.raises(TypeError):
            fuzz_target(bad, None, _blind())  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        fuzz_target([b"a"], "opts", _blind())  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        fuzz_target([b"a", "b"], None, _blind())  # type: ignore[list-item]
    with pytest.raises(ValueError, match="initial_corpus"):
        fuzz_target([b""] * (fuzz.MAX_INITIAL_CORPUS + 1), None, _blind())
    with pytest.raises(ValueError, match="MAX_INPUT_BYTES"):
        fuzz_target([bytes(MAX_INPUT_BYTES + 1)], None, _blind())
    with pytest.raises(TypeError, match="FuzzOutcome"):
        fuzz_target([b"a"], FuzzOptions(1), lambda d, c: "pass")  # type: ignore[arg-type,return-value]
    for kwargs in (
        {"max_iterations": -1},
        {"max_iterations": fuzz.MAX_ITERATIONS + 1},
        {"max_input_len": -1},
        {"max_input_len": MAX_INPUT_BYTES + 1},
        {"base_energy": -1},
        {"minimize_budget": -1},
    ):
        with pytest.raises(ValueError):
            FuzzOptions(**kwargs)
    for kwargs in ({"seed": 1.5}, {"max_iterations": True}):
        with pytest.raises(TypeError):
            FuzzOptions(**kwargs)  # type: ignore[arg-type]


# --- coverage feedback is observable ---------------------------------------------


class _Boom(Exception):
    pass


def _staircase(data: bytes) -> None:
    if len(data) < 4 or data[0] != 0xFF:
        return
    if data[1] != 0x80:
        return
    if data[2] != 0x7F:
        return
    if data[3] == 0x01:
        raise _Boom


@pytest.mark.parametrize("seed", [1, 2, 4, 5, 6])  # 7 of 8 seeds crack it
def test_coverage_feedback_cracks_a_staircase_blind_mutation_does_not(
    seed: int,
) -> None:
    opts = FuzzOptions(max_iterations=6000, seed=seed)
    guided = fuzz_function(_staircase, [bytes(4)], opts)
    blind = fuzz_function(_staircase, [bytes(4)], opts, coverage="none")
    assert len(guided.crashes) == 1 and guided.crashes[0].kind == "_Boom"
    assert guided.crashes[0].minimal_input[:4] == b"\xff\x80\x7f\x01"
    assert guided.coverage_tier in (TIER_MONITORING, TIER_SETTRACE)
    assert guided.admitted_seeds >= 4 and guided.total_edges >= 7
    assert blind.crashes == [] and blind.coverage_tier == TIER_BLACK_BOX
    assert blind.admitted_seeds == 1


def test_instrumented_campaign_is_deterministic_across_runs() -> None:
    opts = FuzzOptions(max_iterations=800, seed=11)
    runs = [fuzz_function(_staircase, [bytes(4)], opts) for _ in range(3)]
    assert runs[0] == runs[1] == runs[2]
    settrace = [
        fuzz_function(_staircase, [bytes(4)], opts, coverage="settrace")
        for _ in range(2)
    ]
    assert settrace[0] == settrace[1]


# --- Harness: classification -----------------------------------------------------


def _run(fn: Any, data: bytes = b"x", **kwargs: Any) -> Any:
    with Harness(fn, **kwargs) as h:
        return h(data, CoverageBitmap())


@pytest.mark.parametrize("mode", ["auto", "monitoring", "settrace"])
def test_harness_classifies_outcomes(mode: str) -> None:
    def ok(_d: bytes) -> None:
        return None

    def bad(_d: bytes) -> None:
        raise KeyError("secret-bytes-in-message")

    def exits(_d: bytes) -> None:
        raise SystemExit(3)

    def oom(_d: bytes) -> None:
        raise MemoryError

    def spin(_d: bytes) -> None:
        n = 0
        while True:
            n += 1

    def swallow(_d: bytes) -> None:
        n = 0
        while n < 10_000:
            try:  # noqa: SIM105
                n += 1
            except Exception:  # noqa: S110
                pass

    assert _run(ok, coverage=mode) == OutcomePass()
    crash = _run(bad, coverage=mode)
    assert isinstance(crash, OutcomeCrash) and crash.kind == "KeyError"
    assert "secret" not in repr(crash)
    assert _run(exits, coverage=mode).kind == "SystemExit"
    assert _run(oom, coverage=mode) == OutcomeOom()
    assert _run(spin, coverage=mode, step_budget=500) == OutcomeTimeout()
    # except Exception cannot swallow the budget
    assert _run(swallow, coverage=mode, step_budget=50) == OutcomeTimeout()


def test_keyboard_interrupt_propagates_and_tracing_is_restored() -> None:
    def interrupt(_d: bytes) -> None:
        raise KeyboardInterrupt

    before = sys.gettrace()
    for mode in ("auto", "settrace"):
        with pytest.raises(KeyboardInterrupt):
            _run(interrupt, coverage=mode)
        assert sys.gettrace() is before


def test_a_timeout_is_deterministic_and_wall_clock_free() -> None:
    def spin(_d: bytes) -> None:
        n = 0
        while True:
            n += 1

    runs = [_run(spin, step_budget=2000) for _ in range(3)]
    assert runs == [OutcomeTimeout()] * 3


# --- Harness: frames and signatures ----------------------------------------------


def _inner(depth: int) -> None:
    if depth:
        _inner(depth - 1)
    else:
        raise ValueError("boom")


def test_frames_are_normalised_without_paths_lines_or_recursion_depth() -> None:
    shallow = _run(lambda d: _inner(3))
    deep = _run(lambda d: _inner(40))
    assert shallow.frames == deep.frames  # recursion collapses
    assert shallow.frames[-1].endswith("._inner")
    for frame in shallow.frames:
        assert "/" not in frame and "\\" not in frame and ":" not in frame
        assert not any(ch.isdigit() for ch in frame.rsplit(".", 1)[-1])
    assert crash_signature(shallow.kind, shallow.frames) == (
        crash_signature(deep.kind, deep.frames)
    )
    assert fuzz.__file__ not in "".join(shallow.frames)


def test_frames_of_limits_and_labels() -> None:
    assert frames_of(None) == ()

    def recurse(n: int) -> Any:
        try:
            if n:
                return recurse(n - 1)
            else:
                raise ValueError
        except ValueError as exc:
            return exc.__traceback__

    tb = recurse(3)
    assert frames_of(tb)[-1].endswith("recurse")
    assert fuzz._frame_label("a:b c\n") == "a_b_c_"
    assert len(fuzz._frame_label("x" * 1000)) == 200


def test_different_failures_get_different_signatures() -> None:
    def a(_d: bytes) -> None:
        raise ValueError

    def b(_d: bytes) -> None:
        raise TypeError

    ra, rb = _run(a), _run(b)
    assert crash_signature(ra.kind, ra.frames) != crash_signature(rb.kind, rb.frames)


# --- Harness: modes, lifecycle, scope --------------------------------------------


def test_harness_modes_and_tier_labels() -> None:
    r = fuzz_function(_staircase, [bytes(4)], FuzzOptions(50), coverage="monitoring")
    assert r.coverage_tier == TIER_MONITORING
    r = fuzz_function(_staircase, [bytes(4)], FuzzOptions(50), coverage="settrace")
    assert r.coverage_tier == TIER_SETTRACE
    r = fuzz_function(_staircase, [bytes(4)], FuzzOptions(50), coverage="none")
    assert r.coverage_tier == TIER_BLACK_BOX

    def reporter(data: bytes, cov: CoverageBitmap) -> None:
        cov.record_edge(data[0] if data else 0)

    r = fuzz_function(reporter, [b"\x01"], FuzzOptions(200), coverage="reporter")
    assert r.coverage_tier == TIER_REPORTED and r.total_edges > 1
    # an instrumented run over code with no Python frames collects nothing
    r = fuzz_function(bytes.hex, [b"\x01"], FuzzOptions(20))
    assert r.total_edges == 0 and r.coverage_tier == TIER_BLACK_BOX


def test_harness_validation_and_misuse() -> None:
    with pytest.raises(TypeError):
        Harness(5)  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="coverage"):
        Harness(print, coverage="magic")
    with pytest.raises(TypeError):
        Harness(print, step_budget=True)  # type: ignore[arg-type]
    for bad in (0, fuzz.MAX_STEP_BUDGET + 1):
        with pytest.raises(ValueError, match="step_budget"):
            Harness(print, step_budget=bad)
    with pytest.raises(RuntimeError, match="context manager"):
        Harness(print)(b"", CoverageBitmap())


def test_monitoring_unavailable_falls_back_or_raises(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.delattr(sys, "monitoring")
    with (
        pytest.raises(RuntimeError, match="monitoring"),
        Harness(_staircase, coverage="monitoring"),
    ):
        pass
    with Harness(_staircase) as h:
        assert h.mechanism == TIER_SETTRACE


def test_tool_ids_are_claimed_released_and_never_stolen() -> None:
    mon = sys.monitoring
    with Harness(_staircase, coverage="monitoring") as h:
        used = h._tool
        assert used in (3, 4, 5)
    mon.use_tool_id(used, "probe")  # was released by __exit__
    mon.free_tool_id(used)
    held = []
    try:
        for tool in (3, 4, 5):
            mon.use_tool_id(tool, "squatter")
            held.append(tool)
        with Harness(_staircase) as h:  # auto: every id busy -> settrace
            assert h.mechanism == TIER_SETTRACE
        with pytest.raises(RuntimeError), Harness(_staircase, coverage="monitoring"):
            pass
    finally:
        for tool in held:
            mon.free_tool_id(tool)


def test_default_scope_rules() -> None:
    assert not default_scope("<string>", "f")
    assert not default_scope("/x/y.py", "<module>")
    assert not default_scope(fuzz.__file__, "fuzz_target")
    assert not default_scope(sys.modules["abc"].__file__, "f")  # stdlib
    assert default_scope("/somewhere/else/mod.py", "f")
    third = fuzz._STDLIB_DIR + "/site-packages/pkg/mod.py"
    assert default_scope(third, "f")
    assert default_scope(__file__, "f")


def test_custom_scope_restricts_coverage() -> None:
    def helper(d: bytes) -> int:
        return d[0] if d else 0

    def target(d: bytes) -> None:
        if helper(d) > 3:
            pass

    only_helper = lambda _file, name: name.endswith("helper")  # noqa: E731
    with Harness(target, scope=only_helper) as h:
        cov = CoverageBitmap()
        h(b"\x05", cov)
    with Harness(target) as h2:
        wide = CoverageBitmap()
        h2(b"\x05", wide)
    assert 0 < cov.count() < wide.count()


def test_settrace_callbacks_directly() -> None:
    h = Harness(_staircase, coverage="settrace", step_budget=3)
    frame = sys._getframe()
    assert h._global_trace(frame, "return", None) is None
    skip = Harness(_staircase, coverage="settrace", scope=lambda *_: False)
    assert skip._global_trace(frame, "call", None) is None
    local = h._global_trace(frame, "call", None)
    assert callable(local)
    assert local(frame, "call", None) is local  # non-line events are ignored
    assert local(frame, "line", None) is local
    assert local(frame, "line", None) is local
    assert len(h._edges) >= 2
    with pytest.raises(fuzz._StepBudgetExceeded):
        local(frame, "line", None)
        local(frame, "line", None)


def test_code_info_line_tables() -> None:
    def two_lines(d: bytes) -> int:
        x = d[0]
        return x + 1

    info = fuzz._CodeInfo(two_lines.__code__, True)
    first = two_lines.__code__.co_firstlineno
    assert info.line_of(0) in range(first, first + 4)
    assert info.line_of(10**6) >= first
    assert info.edge(1, 2) == info.edge(1, 2) and info.edge(1, 2) != info.edge(2, 1)
    assert 0 <= info.edge(1, 2) < 65_536
    empty = fuzz._CodeInfo(compile("", "<x>", "exec"), False)
    empty._starts, empty._lines = [], []
    assert empty.line_of(4) in range(0, 100)


def test_edge_ids_ignore_absolute_paths() -> None:
    code_a = compile("x = 1", "/one/place/mod.py", "exec")
    code_b = compile("x = 1", "/another/dir/mod.py", "exec")
    a, b = fuzz._CodeInfo(code_a, True), fuzz._CodeInfo(code_b, True)
    assert a.edge(3, 4) == b.edge(3, 4)


# --- compatibility wrappers ------------------------------------------------------


def test_fuzz_bytes_keeps_its_signature_and_report_aliases() -> None:
    def target(data: bytes) -> None:
        if data[:2] == b"\xff\xff":
            raise ValueError("bad")

    r = fuzz_bytes(
        target, seed=7, iterations=300, max_input_bytes=16, initial_corpus=[b"\xff\x00"]
    )
    assert r.iterations_run == r.iterations == 300 and r.seed == 7
    assert r.corpus_size == r.admitted_seeds
    assert r.unique_crashes == len(r.crashes) == 1
    assert r.crashes[0].minimal_input == b"\xff\xff"
    again = fuzz_bytes(
        target, seed=7, iterations=300, max_input_bytes=16, initial_corpus=[b"\xff\x00"]
    )
    assert again == r
    assert fuzz_bytes(lambda d: None, iterations=3).is_clean
    with pytest.raises(ValueError):
        fuzz_bytes(target, iterations=0)
    with pytest.raises(TypeError):
        fuzz_bytes(target, iterations=True)  # type: ignore[arg-type]


def test_fuzz_text_decodes_and_validates() -> None:
    seen: list[str] = []

    def target(text: str) -> None:
        seen.append(text)
        if text.startswith("�"):
            raise ValueError

    r = fuzz_text(target, iterations=100, initial_corpus=["héllo"])
    assert seen[0] == "héllo" and r.iterations == 100
    assert fuzz_text(lambda t: None, iterations=2).is_clean
    with pytest.raises(TypeError):
        fuzz_text(target, initial_corpus=[b"bytes"])  # type: ignore[list-item]


def test_one_byte_buffers_need_the_stream_word_for_bounded_one(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """Sabotage: a draw helper that skips the word for n == 1 desynchronises."""

    def lazy(rng: SplitMix64, n: int) -> int:
        return rng.bounded(n)  # SplitMix64.bounded(1) consumes nothing

    monkeypatch.setattr(fuzz, "_below", lazy)
    differing = sum(
        mutate_sequence(b"\x05", seed, 6) != REF.mutate_sequence(b"\x05", seed, 6)
        for seed in range(200)
    )
    assert differing > 0


def test_monitoring_callbacks_directly() -> None:
    h = Harness(_staircase, step_budget=2)
    code = _staircase.__code__
    assert h._on_line(code, code.co_firstlineno + 1) is None
    assert h._on_branch(code, 4, 10) is None
    assert len(h._edges) >= 1
    with pytest.raises(fuzz._StepBudgetExceeded):
        h._on_line(code, code.co_firstlineno + 2)
        h._on_line(code, code.co_firstlineno + 3)
    out = Harness(_staircase, scope=lambda *_: False)
    assert out._on_line(code, 5) is sys.monitoring.DISABLE
    assert out._on_branch(code, 4, 10) is sys.monitoring.DISABLE


def test_minimizer_skips_zero_bytes() -> None:
    sig = crash_signature("K", ["f"])
    exact = lambda d: _crash("K", "f") if d == b"\x01\x00\x02" else OutcomePass()  # noqa: E731
    assert minimize_crashing_input(b"\x01\x00\x02", sig, exact) == b"\x01\x00\x02"
