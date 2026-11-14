"""The ten fault wrappers and their applicability."""

from __future__ import annotations

import pytest

from coding_adventures_warrant_corpus.errors import UsageError
from coding_adventures_warrant_corpus.faults import FAULTS, fault_applies, get_fault
from coding_adventures_warrant_corpus.values import OpSig, Type, parse_type


def sig(params: list[tuple[str, str]], ret: str) -> OpSig:
    return OpSig("op", tuple((n, parse_type(t)) for n, t in params), parse_type(ret))


EQ = sig([("a", "Bytes"), ("b", "Bytes")], "Bool")
SELECT = sig([("a", "Bytes"), ("b", "Bytes"), ("choice", "Bool")], "Result<Bytes,E>")
U64 = sig([("a", "U64"), ("b", "U64")], "Bool")
ADD = sig([("a", "U64"), ("b", "U64")], "U64")
BYTES = sig([("a", "Bytes")], "Bytes")


def test_the_registry_has_exactly_the_ten_portable_faults() -> None:
    assert sorted(FAULTS) == sorted(
        [
            "invert", "ignore-last-byte", "ignore-high-bit", "swap-choice",
            "missing-error", "xor-lsb", "off-by-one", "return-first-arg",
            "truncate-last", "drop-to-none",
        ]
    )  # fmt: skip


def test_unknown_fault_is_a_usage_error() -> None:
    with pytest.raises(UsageError):
        get_fault("nope")


def test_applicability_follows_the_signature() -> None:
    assert fault_applies("invert", EQ) and not fault_applies("invert", ADD)
    assert fault_applies("ignore-last-byte", EQ) and not fault_applies(
        "ignore-last-byte", U64
    )
    assert fault_applies("ignore-high-bit", U64) and not fault_applies(
        "ignore-high-bit", EQ
    )
    assert fault_applies("swap-choice", SELECT) and not fault_applies("swap-choice", EQ)
    assert fault_applies("missing-error", SELECT) and not fault_applies(
        "missing-error", EQ
    )
    assert fault_applies("xor-lsb", ADD) and not fault_applies("xor-lsb", EQ)
    assert fault_applies("off-by-one", ADD)
    assert fault_applies("return-first-arg", ADD) and not fault_applies(
        "return-first-arg", EQ
    )
    assert fault_applies("truncate-last", BYTES) and not fault_applies(
        "truncate-last", EQ
    )
    assert not fault_applies("drop-to-none", EQ)
    # Option is outside the value model, so build such a signature by hand.
    optional = OpSig("op", (), Type("Option", "Option<Bytes>"))
    assert fault_applies("drop-to-none", optional)


def test_input_faults() -> None:
    assert FAULTS["ignore-last-byte"].on_input([b"abc", b""], EQ) == [b"ab\x00", b""]
    high = (1 << 64) - 1
    assert FAULTS["ignore-high-bit"].on_input([high, 5], U64) == [high >> 1, 5]
    assert FAULTS["swap-choice"].on_input([b"", b"", True], SELECT) == [b"", b"", False]
    assert FAULTS["invert"].on_input([b"x", b"y"], EQ) == [b"x", b"y"]  # output-only


def test_output_faults() -> None:
    args: list[object] = [b"zz", b"yy"]
    assert FAULTS["invert"].on_output(True, args, EQ) is False
    assert FAULTS["xor-lsb"].on_output(6, [1, 2], ADD) == 7
    assert FAULTS["off-by-one"].on_output(5, [1, 2], ADD) == 6
    assert FAULTS["off-by-one"].on_output((1 << 64) - 1, [1, 2], ADD) == 0  # masked
    assert FAULTS["return-first-arg"].on_output(0, [9, 8], ADD) == 9
    assert FAULTS["truncate-last"].on_output(b"abc", [b"abc"], BYTES) == b"ab"
    assert FAULTS["drop-to-none"].on_output(b"x", [], BYTES) == ("none",)


def test_missing_error_turns_an_err_into_an_ok_of_the_first_bytes_arg() -> None:
    fault = FAULTS["missing-error"]
    args: list[object] = [b"first", b"second", False]
    assert fault.on_output(("err", "X"), args, SELECT) == ("ok", b"first")
    assert fault.on_output(("ok", b"keep"), args, SELECT) == ("ok", b"keep")
    # no Bytes argument to echo: the Err is left alone
    no_bytes = sig([("c", "Bool")], "Result<Bytes,E>")
    assert fault.on_output(("err", "X"), [True], no_bytes) == ("err", "X")
