"""The minimal TFV reader."""

from __future__ import annotations

import pytest

from coding_adventures_warrant_corpus.errors import CorpusError
from coding_adventures_warrant_corpus.tfv import parse_tfv


def test_parses_every_supported_value_kind() -> None:
    text = "tfv 1\nid/1 R6 Ka N Kb T Kc F Kd I-5 Ke X00ff Kf S6869\n"
    ((record, fields),) = parse_tfv(text)
    assert record == "id/1"
    assert fields["a"] == ("null",)
    assert fields["b"] == ("bool", True) and fields["c"] == ("bool", False)
    assert fields["d"] == ("int", -5)
    assert fields["e"] == ("bytes", b"\x00\xff")
    assert fields["f"] == ("text", "hi")


def test_parses_lists_and_maps() -> None:
    text = "tfv 1\nr R2 Kl L2 I1 X Km M1 S6b I2\n"
    ((_, fields),) = parse_tfv(text)
    assert fields["l"] == ("list", [("int", 1), ("bytes", b"")])
    assert fields["m"] == ("map", [(("text", "k"), ("int", 2))])


@pytest.mark.parametrize(
    "text",
    [
        "",  # no header
        "tfv 2\n",
        "tfv 1\nr R1 Ka T",  # no trailing newline
        "tfv 1\n\nr R1 Ka T\n",  # blank line
        "tfv 1\nr R1 Ka T\r\n",  # CR
        "tfv 1\nr\n",  # too short
        "tfv 1\nbad id R1 Ka T\n",
        "tfv 1\nr Rx Ka T\n",
        "tfv 1\nr R2 Ka T\n",  # fewer fields than announced
        "tfv 1\nr R1 Ka T Kb T\n",  # trailing tokens
        "tfv 1\nr R1 KA T\n",  # bad field name
        "tfv 1\nr R2 Ka T Ka F\n",  # duplicate field
        "tfv 1\nr R1 a T\n",  # missing K
        "tfv 1\nr R1 Ka Q\n",  # unknown kind
        "tfv 1\nr R1 Ka I-0\n",
        "tfv 1\nr R1 Ka Iabc\n",
        "tfv 1\nr R1 Ka XZZ\n",
        "tfv 1\nr R1 Ka Sff\n",  # not UTF-8
        "tfv 1\nr R1 Ka SZZ\n",
        "tfv 1\nr R1 Ka L9 I1\n",  # list longer than the record
        "tfv 1\nr R1 Ka Lx\n",
        "tfv 1\nr R1 Ka L2 I1\n",  # record ends inside a value
        "tfv 1\nr R1 Ka NN\n",  # `N` must stand alone
        "tfv 1\nr R1 Ka\n",  # field with no value
        "tfv 1\nr R1 Ka T\nr R1 Ka T\n",  # duplicate id
    ],
)
def test_rejects_malformed_tfv(text: str) -> None:
    with pytest.raises(CorpusError):
        parse_tfv(text)


def test_rejects_values_nested_too_deeply() -> None:
    deep = " ".join(["L1"] * 70) + " N"
    with pytest.raises(CorpusError):
        parse_tfv(f"tfv 1\nr R1 Ka {deep}\n")
