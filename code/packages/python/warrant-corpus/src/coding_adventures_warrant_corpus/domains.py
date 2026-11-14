"""The finite input domains of VER04i section 2.6, enumerated in this lane.

The corpus does not ship its exhaustive inputs (that would be ~28 MB of text
per library).  It ships a *descriptor* -- ``{"domain": "bytes_d1"}`` -- and each
lane re-creates the values itself.  The descriptors are defined once, by the
spec, and every lane must produce exactly the same values in exactly the same
order; the ``inputs_sha256`` digest in the corpus is what proves it did.

==============  ===========================================================
descriptor      values, in order
==============  ===========================================================
``bool``        ``False``, ``True``
``u8_all``      ``0 .. 255``
``u64_edge``    ``0..255``; ``1<<i`` for ``i = 8..63``; ``MAX``; then
                ``MAX ^ (1<<i)`` for ``i = 0..63``              (377 values)
``nat_0_64``    ``0 .. 64``
``bytes_d1``    ``b""``; every single byte; every length-2 and length-3
                string over the alphabet ``A = {00, 01, 80, ff}`` in
                lexicographic order; for ``L = 4..8`` the strings ``00``x``L``
                and ``a5``x``L``                                 (347 values)
``bytes_n(N)``  for ``N <= 3`` the ``bytes_d1`` members of length ``N``;
                otherwise the four uniform fills over ``A``, then for every
                position and each of ``01, 80, ff`` the zero string with that
                one byte set
==============  ===========================================================

A multi-binder claim enumerates the Cartesian product with the **last binder
varying fastest** (row-major).
"""

from __future__ import annotations

import itertools
import re

from .errors import CorpusError, need
from .values import MAX_WIDTH, U64_MAX

_ALPHABET = (0x00, 0x01, 0x80, 0xFF)
_BYTES_N = re.compile(r"bytes_n\(([0-9]+)\)")

# domain name -> the Warrant type its values have
_SCALAR_TYPES = {
    "bool": "Bool",
    "u8_all": "U8",
    "u64_edge": "U64",
    "nat_0_64": "Nat",
    "bytes_d1": "Bytes",
}


def _bytes_d1() -> list[bytes]:
    values: list[bytes] = [b""]
    values += [bytes((i,)) for i in range(256)]
    for length in (2, 3):
        values += [bytes(p) for p in itertools.product(_ALPHABET, repeat=length)]
    for length in range(4, 9):
        values += [bytes([fill]) * length for fill in (0x00, 0xA5)]
    return values


def _bytes_n(width: int) -> list[bytes]:
    need(0 <= width <= MAX_WIDTH, "descriptor", "Bytes width is out of bounds")
    if width <= 3:
        return [value for value in _bytes_d1() if len(value) == width]
    values = [bytes([fill]) * width for fill in _ALPHABET]
    for position in range(width):
        for symbol in (0x01, 0x80, 0xFF):
            buffer = bytearray(width)
            buffer[position] = symbol
            values.append(bytes(buffer))
    return values


def _u64_edge() -> list[int]:
    return [
        *range(256),
        *(1 << shift for shift in range(8, 64)),
        U64_MAX,
        *(U64_MAX ^ (1 << shift) for shift in range(64)),
    ]


def domain_values(name: str) -> tuple[list[object], str]:
    """Enumerate the descriptor ``name``; return ``(values, Warrant type text)``."""
    if name == "bool":
        return [False, True], "Bool"
    if name == "u8_all":
        return list(range(256)), "U8"
    if name == "u64_edge":
        return list(_u64_edge()), "U64"
    if name == "nat_0_64":
        return list(range(65)), "Nat"
    if name == "bytes_d1":
        return list(_bytes_d1()), "Bytes"
    match = _BYTES_N.fullmatch(name)
    if match:
        width = int(match.group(1))
        return list(_bytes_n(width)), f"Bytes<{width}>"
    raise CorpusError("unsupported-domain", "unknown domain descriptor")


def binder_domain(descriptor: object) -> tuple[list[object], str]:
    """Resolve one binder's ``{"domain": ..., "type": ...}`` descriptor.

    The declared ``type`` must agree with the type the descriptor produces, so
    a corpus cannot label byte strings as integers and have the encoders
    quietly disagree with the generator.
    """
    need(
        type(descriptor) is dict
        and set(descriptor) <= {"domain", "type"}
        and type(descriptor.get("domain")) is str,
        "descriptor",
        "malformed binder domain",
    )
    assert isinstance(descriptor, dict)
    values, produced = domain_values(descriptor["domain"])
    need(
        descriptor.get("type", produced) == produced,
        "descriptor",
        "binder type disagrees with its domain",
    )
    return values, produced


def known_domain_names() -> list[str]:
    """The fixed (non-parameterised) descriptors, for diagnostics and tests."""
    return sorted(_SCALAR_TYPES)
