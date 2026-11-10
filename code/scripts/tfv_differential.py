#!/usr/bin/env python3
"""Differential check: the Rust TFV parser against the Python reference.

`code/packages/rust/test-bootstrap/tests/tfv_exhaustive.rs` enumerates a few
hundred thousand small TFV documents and, when `TFV_DIFFERENTIAL_OUT=<path>`
is set, records what the Rust parser did with each:

    <path>          DOC <hex of document> ok  <fnv1a64 of canonical lines>
                    DOC <hex of document> err -
    <path>.matrix   UNI <hex of value tokens>      (one per universe member)
                    ROW <0/1 string>                (deep-equality matrix row)

This script replays every document through `tfv.py` (the independent,
standard-library-only reference of TF01 section 2) and demands:

  * the same accept / reject decision for every document;
  * for accepted documents, the same canonical re-encoding (compared by
    FNV-1a digest, so the file stays small); and
  * the same deep-equality matrix over the universe.

Usage:  tfv_differential.py <path>        (also reads <path>.matrix)
        tfv_differential.py --self-test   (proves the checker can fail)

Exit status is 0 on full agreement, 1 on any disagreement or unreadable input.
"""

from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import tfv  # noqa: E402  (path set above)

FNV_OFFSET = 0xCBF29CE484222325
FNV_PRIME = 0x100000001B3
MASK64 = (1 << 64) - 1


def fnv1a64(data: bytes) -> int:
    h = FNV_OFFSET
    for byte in data:
        h = ((h ^ byte) * FNV_PRIME) & MASK64
    return h


def canonical_digest(records: list[tfv.Record]) -> int:
    lines = [tfv.encode_record(record_id, fields) for record_id, fields in records]
    return fnv1a64("\n".join(lines).encode("utf-8"))


def python_verdict(text: str) -> tuple[bool, int | None]:
    try:
        records = tfv.parse_file(text)
    except tfv.TfvError:
        return False, None
    return True, canonical_digest(records)


def check_documents(path: str) -> tuple[int, int, list[str]]:
    total = accepted = 0
    problems: list[str] = []
    with open(path, encoding="ascii") as handle:
        for number, raw in enumerate(handle, start=1):
            kind, hex_text, status, digest = raw.rstrip("\n").split("\t")
            if kind != "DOC":
                problems.append(f"line {number}: unexpected record kind {kind!r}")
                continue
            raw_bytes = bytes.fromhex(hex_text)
            try:
                text = raw_bytes.decode("utf-8")
            except UnicodeDecodeError:
                problems.append(f"line {number}: Rust fed non-UTF-8 text to a &str API")
                continue
            total += 1
            py_ok, py_digest = python_verdict(text)
            rust_ok = status == "ok"
            if rust_ok != py_ok:
                problems.append(
                    f"line {number}: rust {'accepts' if rust_ok else 'rejects'}, "
                    f"python {'accepts' if py_ok else 'rejects'}: {text!r}"
                )
            elif rust_ok and int(digest, 16) != py_digest:
                problems.append(f"line {number}: canonical forms differ: {text!r}")
            accepted += 1 if py_ok else 0
    return total, accepted, problems


def parse_universe_token(tokens_text: str) -> tfv.Value:
    tokens = tokens_text.split(" ")
    value, position = tfv._parse_value(tokens, 0, 0, 0)
    if position != len(tokens):
        raise tfv.TfvError(f"trailing tokens in universe member {tokens_text!r}")
    return value


def check_matrix(path: str) -> tuple[int, list[str]]:
    problems: list[str] = []
    universe: list[tfv.Value] = []
    rows: list[str] = []
    with open(path, encoding="ascii") as handle:
        for raw in handle:
            kind, payload = raw.rstrip("\n").split("\t")
            if kind == "UNI":
                universe.append(parse_universe_token(bytes.fromhex(payload).decode("ascii")))
            elif kind == "ROW":
                rows.append(payload)
            else:
                problems.append(f"unexpected matrix record {kind!r}")
    if len(rows) != len(universe):
        problems.append(f"{len(universe)} universe members but {len(rows)} rows")
        return 0, problems
    cells = 0
    for i, row in enumerate(rows):
        if len(row) != len(universe):
            problems.append(f"row {i} has {len(row)} cells, expected {len(universe)}")
            continue
        for j, cell in enumerate(row):
            cells += 1
            want = tfv.values_equal(universe[i], universe[j])
            if (cell == "1") != want:
                problems.append(
                    f"equality ({i}, {j}): rust says {cell == '1'}, python says {want}"
                )
    return cells, problems


def self_test() -> int:
    """The checker must notice a corrupted verdict, digest and matrix cell."""
    import tempfile

    good = "tfv 1\nr R1 Kv I5\n"
    ok, digest = python_verdict(good)
    assert ok and digest is not None
    hex_good = good.encode().hex()
    cases = {
        "flipped verdict": f"DOC\t{hex_good}\terr\t-\n",
        "wrong digest": f"DOC\t{hex_good}\tok\t{digest ^ 1:016x}\n",
        "wrong accept": f"DOC\t{'tfv 1\nr R1 Kv I05\n'.encode().hex()}\tok\t{digest:016x}\n",
    }
    for label, content in cases.items():
        with tempfile.NamedTemporaryFile("w", suffix=".tsv", delete=False) as handle:
            handle.write(content)
        _, _, problems = check_documents(handle.name)
        os.unlink(handle.name)
        if not problems:
            print(f"self-test FAILED: checker missed {label}")
            return 1
    with tempfile.NamedTemporaryFile("w", suffix=".matrix", delete=False) as handle:
        handle.write(f"UNI\t{b'N'.hex()}\nROW\t0\n")  # N must equal N
    _, problems = check_matrix(handle.name)
    os.unlink(handle.name)
    if not problems:
        print("self-test FAILED: checker missed a corrupted matrix cell")
        return 1
    print("self-test ok: all 4 corruptions detected")
    return 0


def main(argv: list[str]) -> int:
    if argv == ["--self-test"]:
        return self_test()
    if len(argv) != 1:
        print(__doc__)
        return 1
    path = argv[0]
    total, accepted, problems = check_documents(path)
    cells, matrix_problems = check_matrix(path + ".matrix")
    problems += matrix_problems
    if total == 0 or cells == 0:
        problems.append("empty differential input: nothing was compared")
    for problem in problems[:25]:
        print("DISAGREEMENT:", problem)
    if problems:
        print(f"{len(problems)} disagreement(s)")
        return 1
    print(
        f"tfv differential ok: {total} documents ({accepted} accepted), "
        f"{cells} equality cells, rust and python agree"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
