"""The shared fixture and a tiny independent oracle specify the media contract."""

from __future__ import annotations

import json
from itertools import product
from pathlib import Path

import pytest

from nand_flash import (
    AddressError,
    EraseLimitError,
    GeometryError,
    NANDFlash,
    NANDFlashError,
    PayloadError,
    ProgramError,
)

FIXTURE = (
    Path(__file__).parents[4]
    / "specs"
    / "fixtures"
    / "database-media-v1"
    / "nand-flash.json"
)


def test_shared_vectors_include_result_error_and_state_after_every_step() -> None:
    """A language-neutral JSON sequence pins both successes and atomic failures."""
    vectors = json.loads(FIXTURE.read_text(encoding="utf-8"))
    assert vectors["format"] == "coding-adventures-nand-flash-v1"
    device = NANDFlash(**vectors["geometry"])
    assert device.snapshot() == vectors["initial_state"]

    for operation, expected_state in zip(
        vectors["operations"], vectors["states_after_operations"], strict=True
    ):
        arguments = {
            key: value
            for key, value in operation.items()
            if key not in {"op", "result", "error", "data"}
        }
        if "data" in operation:
            arguments["data"] = bytes.fromhex(operation["data"])
        command = getattr(device, operation["op"])
        error = operation.get("error")
        if error:
            with pytest.raises(NANDFlashError) as caught:
                command(**arguments)
            assert type(caught.value).__name__ == error["type"]
            assert str(caught.value) == error["message"]
        else:
            actual = command(**arguments)
            if operation["op"] == "read":
                assert actual.hex() == operation["result"]
            else:
                assert actual is operation["result"]
        assert device.snapshot() == expected_state


def test_read_program_erase_boundaries_and_detached_snapshot() -> None:
    device = NANDFlash(1, 2, 2)
    assert (device.block_count, device.pages_per_block, device.page_size_bytes) == (
        1,
        2,
        2,
    )
    assert device.max_erase_cycles is None
    snap = device.snapshot()
    snap["pages"][0][0] = "0000"  # type: ignore[index]
    assert device.read(0, 0) == b"\xff\xff"
    device.program(0, 0, b"\x00\x55")
    device.erase_block(0)
    assert device.read(0, 0) == b"\xff\xff"
    assert device.snapshot()["erase_counts"] == [1]


@pytest.mark.parametrize(
    "dimensions",
    [
        (0, 1, 1, None),
        (1, 0, 1, None),
        (1, 1, 0, None),
        (True, 1, 1, None),
        (1, False, 1, None),
        (1, 1, True, None),
        (1025, 1, 1, None),
        (1, 257, 1, None),
        (1, 1, 65537, None),
        (1024, 256, 65536, None),
        (1, 1, 1, -1),
        (1, 1, 1, True),
        (1, 1, 1, 2**53),
    ],
)
def test_invalid_geometry_is_rejected(dimensions: tuple[object, ...]) -> None:
    with pytest.raises(
        GeometryError, match="^geometry values are outside supported bounds$"
    ):
        NANDFlash(*dimensions)


@pytest.mark.parametrize("block", [-1, 1, True, "0", None])
def test_invalid_blocks_do_not_mutate(block: object) -> None:
    device = NANDFlash(1, 1, 1)
    before = device.snapshot()
    with pytest.raises(AddressError, match="^block index out of range$"):
        device.erase_block(block)
    assert device.snapshot() == before


@pytest.mark.parametrize("page", [-1, 1, True, "0", None])
def test_invalid_pages_do_not_mutate(page: object) -> None:
    device = NANDFlash(1, 1, 1)
    before = device.snapshot()
    with pytest.raises(AddressError, match="^page index out of range$"):
        device.read(0, page)
    assert device.snapshot() == before


def test_validation_order_checks_block_then_page_then_payload_then_marker() -> None:
    device = NANDFlash(1, 1, 1)
    with pytest.raises(AddressError, match="^block index out of range$"):
        device.program(2, 3, "not bytes")  # type: ignore[arg-type]
    with pytest.raises(AddressError, match="^page index out of range$"):
        device.program(0, 3, "not bytes")  # type: ignore[arg-type]

    class BytesSubclass(bytes):
        pass

    with pytest.raises(PayloadError, match="^page data must be bytes$"):
        device.program(0, 0, BytesSubclass(b"x"))
    with pytest.raises(PayloadError, match="^page data must be bytes$"):
        device.program(0, 0, bytearray(b"x"))  # type: ignore[arg-type]
    with pytest.raises(PayloadError, match="^page data must match page_size_bytes$"):
        device.program(0, 0, b"")
    device.program(0, 0, b"x")
    with pytest.raises(PayloadError, match="^page data must match page_size_bytes$"):
        device.program(0, 0, b"")


def test_all_ff_program_consumes_marker_and_failed_program_is_atomic() -> None:
    device = NANDFlash(1, 1, 2)
    device.program(0, 0, b"\xff\xff")
    before = device.snapshot()
    with pytest.raises(
        ProgramError, match="^page has already been programmed since erase$"
    ):
        device.program(0, 0, b"\x00\x00")
    assert device.snapshot() == before


def test_erase_budget_is_per_block_and_zero_budget_rejects_without_change() -> None:
    device = NANDFlash(2, 1, 1, max_erase_cycles=1)
    device.erase_block(0)
    device.erase_block(1)
    before = device.snapshot()
    with pytest.raises(EraseLimitError, match="^block erase limit reached$"):
        device.erase_block(0)
    assert device.snapshot() == before
    zero = NANDFlash(1, 1, 1, max_erase_cycles=0)
    with pytest.raises(EraseLimitError, match="^block erase limit reached$"):
        zero.erase_block(0)
    assert zero.snapshot()["erase_counts"] == [0]

    # Exercise the portable counter ceiling without performing quadrillions of erases.
    unbounded = NANDFlash(1, 1, 1)
    unbounded._erase_counts[0] = 2**53 - 1
    before = unbounded.snapshot()
    with pytest.raises(EraseLimitError, match="^block erase limit reached$"):
        unbounded.erase_block(0)
    assert unbounded.snapshot() == before


def test_small_device_traces_match_an_independent_transition_model() -> None:
    """Exhaust every length-three trace over writes, reads, and erases on tiny media."""
    # The oracle uses integer page contents and an independent flag/count model.
    actions = ("read", "write_ff", "write_00", "erase")
    for trace in product(actions, repeat=3):
        device = NANDFlash(1, 1, 1, max_erase_cycles=1)
        oracle_data = 0xFF
        oracle_written = False
        oracle_erases = 0
        for action in trace:
            before = device.snapshot()
            if action == "read":
                assert device.read(0, 0) == bytes([oracle_data])
            elif action.startswith("write"):
                value = 0xFF if action == "write_ff" else 0
                if oracle_written:
                    with pytest.raises(ProgramError):
                        device.program(0, 0, bytes([value]))
                    assert device.snapshot() == before
                else:
                    device.program(0, 0, bytes([value]))
                    oracle_data &= value
                    oracle_written = True
            elif oracle_erases == 1:
                with pytest.raises(EraseLimitError):
                    device.erase_block(0)
                assert device.snapshot() == before
            else:
                device.erase_block(0)
                oracle_data, oracle_written = 0xFF, False
                oracle_erases += 1
            assert device.read(0, 0) == bytes([oracle_data])
            state = device.snapshot()
            assert state["erase_counts"] == [oracle_erases]
            assert state["programmed"] == [[oracle_written]]


def test_error_taxonomy_is_catchable_through_common_base() -> None:
    assert issubclass(GeometryError, NANDFlashError)
    assert issubclass(AddressError, NANDFlashError)
    assert issubclass(PayloadError, NANDFlashError)
    assert issubclass(ProgramError, NANDFlashError)
    assert issubclass(EraseLimitError, NANDFlashError)


def test_seeded_generated_traces_replay_against_independent_state_oracle() -> None:
    """Seeded command streams keep a reproducible oracle trail for future ports."""
    import random

    seed = 0xDB01
    rng = random.Random(seed)
    media = NANDFlash(3, 2, 4, max_erase_cycles=3)
    # Oracle stores each byte independently from the implementation's byte pages.
    pages = [[[255] * 4 for _ in range(2)] for _ in range(3)]
    programmed = [[False] * 2 for _ in range(3)]
    erases = [0, 0, 0]
    trace: list[tuple[str, int, int, bytes | None]] = []

    for _ in range(250):
        block = rng.randrange(3)
        page = rng.randrange(2)
        action = rng.choice(("read", "program", "erase"))
        payload = (
            bytes(rng.randrange(256) for _ in range(4)) if action == "program" else None
        )
        trace.append((action, block, page, payload))
        if action == "read":
            actual = media.read(block, page)
            assert list(actual) == pages[block][page], (seed, trace)
        elif action == "program":
            if programmed[block][page]:
                before = media.snapshot()
                with pytest.raises(ProgramError):
                    media.program(block, page, payload or b"")
                assert media.snapshot() == before, (seed, trace)
            else:
                media.program(block, page, payload or b"")
                pages[block][page] = [
                    old & new
                    for old, new in zip(pages[block][page], payload or b"", strict=True)
                ]
                programmed[block][page] = True
        elif erases[block] == 3:
            before = media.snapshot()
            with pytest.raises(EraseLimitError):
                media.erase_block(block)
            assert media.snapshot() == before, (seed, trace)
        else:
            media.erase_block(block)
            pages[block] = [[255] * 4 for _ in range(2)]
            programmed[block] = [False] * 2
            erases[block] += 1
        assert media.snapshot() == {
            "pages": [
                [bytes(page_bytes).hex() for page_bytes in block_pages]
                for block_pages in pages
            ],
            "erase_counts": erases,
            "programmed": programmed,
        }, (seed, trace)


def test_seeded_malformed_commands_fail_with_typed_errors_and_no_mutation() -> None:
    """A bounded pseudo-fuzz pass probes wrong Python shapes and operation order."""
    import random

    rng = random.Random(0xBADDA7A)
    device = NANDFlash(2, 2, 3, max_erase_cycles=2)
    bad_addresses: list[object] = [-1, 2, True, False, None, "0", 0.0]
    bad_payloads: list[object] = [
        None,
        "abc",
        bytearray(b"abc"),
        memoryview(b"abc"),
        b"",
        b"ab",
    ]

    for _ in range(100):
        before = device.snapshot()
        if rng.randrange(2):
            block = rng.choice(bad_addresses)
            with pytest.raises(AddressError):
                device.read(block, 0)
        else:
            page = rng.choice(bad_addresses)
            with pytest.raises(AddressError):
                device.read(0, page)
        assert device.snapshot() == before

        block, page = rng.randrange(2), rng.randrange(2)
        data = rng.choice(bad_payloads)
        with pytest.raises(PayloadError):
            device.program(block, page, data)
        assert device.snapshot() == before


def test_snapshot_mutations_are_detached_for_every_nested_collection() -> None:
    device = NANDFlash(1, 1, 1)
    snapshot = device.snapshot()
    snapshot["pages"][0][0] = "00"  # type: ignore[index]
    snapshot["erase_counts"][0] = 99  # type: ignore[index]
    snapshot["programmed"][0][0] = True  # type: ignore[index]
    assert device.snapshot() == {
        "pages": [["ff"]],
        "erase_counts": [0],
        "programmed": [[False]],
    }
