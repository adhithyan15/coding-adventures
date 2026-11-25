"""Conformance, boundary, and independent bounded model checks."""

import json
import random
from pathlib import Path

import pytest

from hdd_simulator import (
    ClockOverflowError,
    HddGeometryV1,
    HddSimulator,
    InvalidLbaError,
    InvalidSectorDataError,
)
from hdd_simulator.simulator import _MAX_SAFE_TICK

FIXTURE = (
    Path(__file__).resolve().parents[4]
    / "specs"
    / "fixtures"
    / "database-media-v1"
    / "hdd-simulator.json"
)


def _timing_dict(timing: object) -> dict[str, int]:
    return {
        key: getattr(timing, key)
        for key in (
            "start_tick",
            "seek_ticks",
            "rotation_ticks",
            "transfer_ticks",
            "total_ticks",
            "end_tick",
        )
    }


def test_shared_json_vectors() -> None:
    vectors = json.loads(FIXTURE.read_text())
    profile = vectors["profile"]
    geometry = HddGeometryV1()
    assert profile == {
        "surfaces": geometry.surfaces,
        "tracks_per_surface": geometry.tracks_per_surface,
        "sectors_per_track": geometry.sectors_per_track,
        "bytes_per_sector": geometry.bytes_per_sector,
        "seek_ticks_per_track": geometry.seek_ticks_per_track,
        "transfer_ticks_per_sector": geometry.transfer_ticks_per_sector,
    }
    for case in vectors["cases"]:
        simulator = HddSimulator()
        for operation in case["operations"]:
            before = simulator.get_state()
            if "error" in operation:
                error_type = {
                    "InvalidLbaError": InvalidLbaError,
                    "InvalidSectorDataError": InvalidSectorDataError,
                }[operation["error"]]
                with pytest.raises(error_type):
                    if operation["op"] == "read":
                        simulator.read_sector(operation["lba"])
                    else:
                        data = bytes.fromhex(operation["data_hex"])
                        simulator.write_sector(operation["lba"], data)
                assert simulator.get_state() == before
                continue

            if operation["op"] == "read":
                result = simulator.read_sector(operation["lba"])
                assert result.data.hex() == operation["data_hex"]
                timing = result.timing
            else:
                data = bytes.fromhex(operation["data_hex"])
                timing = simulator.write_sector(operation["lba"], data)
            assert _timing_dict(timing) == operation["timing"]
            state = simulator.get_state()
            assert {"tick": state.tick, "head_track": state.head_track} == operation[
                "state"
            ]

    for probe in vectors["timing_probes"]:
        simulator = HddSimulator()
        simulator._tick = probe["start_tick"]
        simulator._head_track = probe["head_track"]
        if "error" in probe:
            with pytest.raises(ClockOverflowError):
                simulator._timing_for(probe["target_track"], probe["target_sector"])
        else:
            timing = simulator._timing_for(
                probe["target_track"], probe["target_sector"]
            )
            assert _timing_dict(timing) == probe["timing"]


def test_geometry_initialization_and_snapshots_are_owned() -> None:
    simulator = HddSimulator()
    geometry = simulator.geometry
    state = simulator.get_state()
    assert geometry.sector_count == 32
    assert geometry.sectors_per_cylinder == 8
    assert state.tick == 0 and state.head_track == 0
    assert state.sectors == (bytes(8),) * 32
    simulator.write_sector(0, b"abcdefgh")
    assert state.sectors[0] == bytes(8)
    assert simulator.get_state().sectors[0] == b"abcdefgh"
    with pytest.raises((AttributeError, TypeError)):
        state.tick = 9  # type: ignore[misc]


@pytest.mark.parametrize("lba", [-1, 32, 100, 1.0, "0", None, True, False])
def test_invalid_lba_is_atomic(lba: object) -> None:
    simulator = HddSimulator()
    before = simulator.get_state()
    with pytest.raises(InvalidLbaError):
        simulator.read_sector(lba)  # type: ignore[arg-type]
    assert simulator.get_state() == before


@pytest.mark.parametrize(
    "data",
    [b"", b"1234567", b"123456789", "abcdefgh", 42, None],
)
def test_invalid_write_payload_is_atomic(data: object) -> None:
    simulator = HddSimulator()
    before = simulator.get_state()
    with pytest.raises(InvalidSectorDataError):
        simulator.write_sector(0, data)  # type: ignore[arg-type]
    assert simulator.get_state() == before


def test_valid_bytes_like_payloads_are_copied() -> None:
    simulator = HddSimulator()
    mutable = bytearray(b"12345678")
    simulator.write_sector(31, mutable)
    mutable[0] = 0
    assert simulator.read_sector(31).data == b"12345678"
    source = bytearray(b"abcdefgh")
    simulator.write_sector(30, memoryview(source))
    source[0] = 0
    assert simulator.read_sector(30).data == b"abcdefgh"


def test_exhaustive_address_mapping_and_timing_components() -> None:
    simulator = HddSimulator()
    for lba in range(32):
        track, sector = simulator._validate_lba(lba)
        assert track == lba // 8
        assert sector == lba % 4
    # Exhaust each possible clock phase, head track, and target track/sector.
    # The oracle advances one abstract tick at a time, independently of the
    # closed-form timing calculation used by the simulator.
    for start_phase in range(4):
        for head in range(4):
            for target in range(4):
                for sector in range(4):
                    start = 20 + start_phase
                    seek_remaining = abs(target - head) * 3
                    tick = start
                    while seek_remaining:
                        tick += 1
                        seek_remaining -= 1
                    rotation = 0
                    while tick % 4 != sector:
                        tick += 1
                        rotation += 1
                    tick += 1
                    simulator._tick = start
                    simulator._head_track = head
                    timing = simulator._timing_for(target, sector)
                    assert timing.start_tick == start
                    expected = abs(target - head) * 3 + rotation + 1
                    assert timing.total_ticks == expected


def test_seeded_operation_sequences_match_independent_tick_model() -> None:
    seed = 0x484444
    rng = random.Random(seed)
    simulator = HddSimulator()
    replay = HddSimulator()
    model_tick = 0
    model_head = 0
    model_media = [bytes(8) for _ in range(32)]
    for _ in range(250):
        lba = rng.randrange(32)
        target_track, target_sector = lba // 8, lba % 4
        payload = rng.randbytes(8)
        is_write = bool(rng.getrandbits(1))
        start = model_tick
        seek_steps = abs(target_track - model_head) * 3
        expected_seek = seek_steps
        while seek_steps:
            model_tick += 1
            seek_steps -= 1
        expected_rotation = 0
        while model_tick % 4 != target_sector:
            model_tick += 1
            expected_rotation += 1
        model_tick += 1
        expected_transfer = 1
        model_head = target_track
        if is_write:
            timing = simulator.write_sector(lba, payload)
            replay_timing = replay.write_sector(lba, payload)
            model_media[lba] = payload
        else:
            result = simulator.read_sector(lba)
            replay_result = replay.read_sector(lba)
            assert result.data == model_media[lba]
            assert replay_result.data == result.data
            timing = result.timing
            replay_timing = replay_result.timing
        assert replay_timing == timing
        assert replay.get_state() == simulator.get_state()
        assert _timing_dict(timing) == {
            "start_tick": start,
            "seek_ticks": expected_seek,
            "rotation_ticks": expected_rotation,
            "transfer_ticks": expected_transfer,
            "total_ticks": model_tick - start,
            "end_tick": model_tick,
        }
        assert simulator.get_state().tick == model_tick
        assert simulator.get_state().head_track == model_head
        assert simulator.get_state().sectors == tuple(model_media)


def test_malformed_fuzz_cases_and_deterministic_replay() -> None:
    seed = 0xBAD5EC7
    rng = random.Random(seed)
    malformed_lbas: list[object] = [True, False, -1, 32, 2**80, "31", 3.5, None]
    malformed_data: list[object] = [b"", b"x" * 7, b"y" * 9, "12345678", None, 7]
    malformed_lbas.extend(rng.choice(malformed_lbas) for _ in range(40))
    malformed_data.extend(rng.choice(malformed_data) for _ in range(40))
    first, second = HddSimulator(), HddSimulator()
    for lba in malformed_lbas:
        for simulator in (first, second):
            before = simulator.get_state()
            with pytest.raises(InvalidLbaError):
                simulator.read_sector(lba)  # type: ignore[arg-type]
            assert simulator.get_state() == before
    for data in malformed_data:
        for simulator in (first, second):
            before = simulator.get_state()
            with pytest.raises(InvalidSectorDataError):
                simulator.write_sector(0, data)  # type: ignore[arg-type]
            assert simulator.get_state() == before


def test_released_oversize_and_subclass_inputs_are_atomic() -> None:
    simulator = HddSimulator()
    before = simulator.get_state()

    released = memoryview(b"12345678")
    released.release()
    with pytest.raises(InvalidSectorDataError):
        simulator.write_sector(0, released)

    oversized = memoryview(bytes(16))
    with pytest.raises(InvalidSectorDataError):
        simulator.write_sector(0, oversized)

    class HostileBytes(bytes):
        def __bytes__(self) -> bytes:
            simulator.read_sector(0)
            return b"12345678"

    hostile = HostileBytes(b"12345678")
    with pytest.raises(InvalidSectorDataError):
        simulator.write_sector(0, hostile)
    assert simulator.get_state() == before


def test_memoryview_byte_count_and_source_copy() -> None:
    simulator = HddSimulator()
    source = bytearray(b"12345678")
    # A view with wider elements still represents eight bytes, not two.
    typed_view = memoryview(source).cast("I")
    simulator.write_sector(0, typed_view)
    source[0] = ord("x")
    assert simulator.read_sector(0).data == b"12345678"


def test_clock_overflow_rejects_operation_atomically() -> None:
    simulator = HddSimulator()
    simulator._tick = _MAX_SAFE_TICK
    before = simulator.get_state()
    with pytest.raises(ClockOverflowError):
        simulator.read_sector(0)
    assert simulator.get_state() == before
    with pytest.raises(InvalidSectorDataError):
        simulator.write_sector(0, b"short")
    assert simulator.get_state() == before
    with pytest.raises(ClockOverflowError):
        simulator.write_sector(0, b"12345678")
    assert simulator.get_state() == before


def test_int_subclass_lba_is_rejected_atomically() -> None:
    class ExtendedLba(int):
        pass

    simulator = HddSimulator()
    before = simulator.get_state()
    with pytest.raises(InvalidLbaError):
        simulator.read_sector(ExtendedLba(0))
    assert simulator.get_state() == before
