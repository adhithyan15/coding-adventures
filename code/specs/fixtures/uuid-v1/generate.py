"""Generate the checked-in uuid-v1 fixture from the independent oracle.

Deterministic: the only randomness is a SplitMix64 stream seeded with
``0x555549445F7631`` ("UUID_v1"). Regenerating must produce byte-identical
``cases.json``; ``validate.py`` enforces that in CI-adjacent checks.
"""

from __future__ import annotations

import json
from pathlib import Path

from oracle import (
    MAX,
    NAMESPACES,
    NIL,
    format_uuid,
    parse,
    v3,
    v5,
    variant_of,
    version_of,
)

ROOT = Path(__file__).resolve().parent
SEED = 0x555549445F7631  # "UUID_v1"
MASK64 = (1 << 64) - 1


class SplitMix64:
    def __init__(self, seed: int):
        self.state = seed & MASK64

    def next(self) -> int:
        self.state = (self.state + 0x9E3779B97F4A7C15) & MASK64
        value = self.state
        value = ((value ^ (value >> 30)) * 0xBF58476D1CE4E5B9) & MASK64
        value = ((value ^ (value >> 27)) * 0x94D049BB133111EB) & MASK64
        return (value ^ (value >> 31)) & MASK64

    def rand_range(self, n: int) -> int:
        return self.next() % n


def _bytes_from_rng(rng: SplitMix64, size: int) -> bytes:
    output = bytearray()
    while len(output) < size:
        output.extend(rng.next().to_bytes(8, "little"))
    return bytes(output[:size])


def _fields(raw: bytes) -> dict:
    return {
        "hex": raw.hex(),
        "string": format_uuid(raw),
        "urn": f"urn:uuid:{format_uuid(raw)}",
        "version": version_of(raw),
        "variant": variant_of(raw),
        "is_nil": raw == NIL,
        "is_max": raw == MAX,
    }


def build_fixture() -> dict:
    rng = SplitMix64(SEED)

    namespaces = {
        name: {"hex": raw.hex(), "string": format_uuid(raw)}
        for name, raw in NAMESPACES.items()
    }
    namespaces["nil"] = {"hex": NIL.hex(), "string": format_uuid(NIL)}
    namespaces["max"] = {"hex": MAX.hex(), "string": format_uuid(MAX)}

    known_answers = [
        {
            "id": "v3-dns-python.org",
            "op": "v3",
            "namespace": "dns",
            "name": "python.org",
            "expected": format_uuid(v3(NAMESPACES["dns"], "python.org")),
        },
        {
            "id": "v5-dns-python.org",
            "op": "v5",
            "namespace": "dns",
            "name": "python.org",
            "expected": format_uuid(v5(NAMESPACES["dns"], "python.org")),
        },
        {
            "id": "v5-url-widgets",
            "op": "v5",
            "namespace": "url",
            "name": "http://www.widgets.com/",
            "expected": format_uuid(v5(NAMESPACES["url"], "http://www.widgets.com/")),
        },
        {
            "id": "v5-dns-unicode",
            "op": "v5",
            "namespace": "dns",
            "name": "日本語",
            "expected": format_uuid(v5(NAMESPACES["dns"], "日本語")),
        },
    ]

    # Directed parse inputs: every accepted form, both cases, whitespace,
    # plus hyphen-position leniency (hyphens are deleted, not validated).
    directed_inputs = [
        "550e8400-e29b-41d4-a716-446655440000",
        "550E8400-E29B-41D4-A716-446655440000",
        "550e8400e29b41d4a716446655440000",
        "550E8400E29B41D4A716446655440000",
        "{550e8400-e29b-41d4-a716-446655440000}",
        "urn:uuid:550e8400-e29b-41d4-a716-446655440000",
        "  550e8400-e29b-41d4-a716-446655440000  ",
        "\t550e8400e29b41d4a716446655440000\n",
        "00000000-0000-0000-0000-000000000000",
        "ffffffff-ffff-ffff-ffff-ffffffffffff",
        "FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF",
        "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "5-50e8400-e29b-41d4-a716-446655440000",
        "550e840041d4a716446655440000-e29b-",
        "550e8400-e29b-41d4-a716-446655440000-",
    ]
    parse_cases = []
    for i, text in enumerate(directed_inputs):
        raw = parse(text)
        parse_cases.append({"id": f"parse-{i:03d}", "input": text, **_fields(raw)})

    invalid_inputs = [
        "",
        " ",
        "-",
        "{",
        "}",
        "{}",
        "urn:uuid:",
        "urn:uuid:{",
        "not-a-uuid",
        "hello",
        "xyz",
        "zzzzzzzz-zzzz-zzzz-zzzz-zzzzzzzzzzzz",
        "550e8400-e29b-41d4-a716-44665544000",
        "550e8400-e29b-41d4-a716-4466554400000",
        "550e8400-e29b-41d4-a716-44665544000g",
        "550e8400-e29b-41d4-a716-44665544000 ",
        "{{550e8400-e29b-41d4-a716-446655440000}}",
        "urn:uuid:550e8400-e29b-41d4-a716-44665544000",
        "g50e8400-e29b-41d4-a716-446655440000",
    ]

    # Generated name-based vectors from the seeded stream.
    ns_names = ["dns", "url", "oid", "x500"]
    v3_cases = []
    v5_cases = []
    for i in range(128):
        ns = ns_names[rng.rand_range(4)]
        name_len = rng.rand_range(48)
        alphabet = "abcdefghijklmnopqrstuvwxyz0123456789-._~:/?#[]@!$&'()*+,;=%"
        name = "".join(alphabet[rng.rand_range(len(alphabet))] for _ in range(name_len))
        v3_cases.append(
            {
                "id": f"v3-{i:03d}",
                "namespace": ns,
                "name": name,
                "expected": format_uuid(v3(NAMESPACES[ns], name)),
            }
        )
        v5_cases.append(
            {
                "id": f"v5-{i:03d}",
                "namespace": ns,
                "name": name,
                "expected": format_uuid(v5(NAMESPACES[ns], name)),
            }
        )

    # Byte patterns for round-trip checks: directed edges + seeded vectors.
    patterns = [bytes(16), bytes([0xFF] * 16)]
    for i in range(16):
        single = bytearray(16)
        single[i] = 0xFF
        patterns.append(bytes(single))
    patterns.append(bytes(range(16)))
    patterns.append(bytes((i * 37 + 11) & 0xFF for i in range(16)))
    for _ in range(64):
        patterns.append(_bytes_from_rng(rng, 16))
    byte_patterns = [
        {"id": f"bytes-{i:03d}", **_fields(raw)} for i, raw in enumerate(patterns)
    ]

    return {
        "schema": "uuid-v1",
        "oracle": "uuid-oracle-py-v1",
        "seed": f"0x{SEED:016x}",
        "namespaces": namespaces,
        "known_answers": known_answers,
        "parse_cases": parse_cases,
        "invalid_inputs": invalid_inputs,
        "v3_cases": v3_cases,
        "v5_cases": v5_cases,
        "byte_patterns": byte_patterns,
    }


def main() -> None:
    fixture = build_fixture()
    text = json.dumps(fixture, indent=2, sort_keys=False) + "\n"
    (ROOT / "cases.json").write_text(text, encoding="utf-8")
    counts = {k: len(v) for k, v in fixture.items() if isinstance(v, list)}
    print(f"wrote cases.json: {counts}")


if __name__ == "__main__":
    main()
