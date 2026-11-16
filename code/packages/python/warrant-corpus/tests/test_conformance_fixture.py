"""Run every ``warrant-corpus-conformance-v1`` case against this lane (WD-100).

The fixture (``code/specs/fixtures/warrant-corpus-conformance-v1``, VER04i 9.1)
holds the behaviours every lane's runner re-discovered in review: strict JSON,
canonical output, tagged values, path containment and the label digest.  Each
test below runs one category through *this lane's own* functions and compares
with the expected outcome stored in the fixture.

Failure messages name the case id and never print the input bytes.
"""

from __future__ import annotations

import hashlib

import pytest
from helpers import REPO_ROOT

from coding_adventures_warrant_corpus.containment import relative_inside
from coding_adventures_warrant_corpus.errors import CorpusError
from coding_adventures_warrant_corpus.jsonio import parse_json, render_json
from coding_adventures_warrant_corpus.labels import manifest_label_digest
from coding_adventures_warrant_corpus.values import (
    decode_json,
    encode_stream,
    parse_type,
)

FIXTURE = REPO_ROOT / "code" / "specs" / "fixtures" / "warrant-corpus-conformance-v1"
CATEGORIES = ("json", "tagged", "containment", "label-digest")


def _load(name: str) -> dict:
    # The fixture is read with this lane's own strict reader.
    document = parse_json((FIXTURE / name).read_bytes())
    assert isinstance(document, dict)
    return document


INDEX = _load("index.json")
CASES = {c: _load(f"{c}.json")["cases"] for c in CATEGORIES}


def _ids(category: str) -> list[str]:
    return [case["id"] for case in CASES[category]]


def test_index_matches_the_files_on_disk() -> None:
    assert INDEX["schema"] == "warrant-corpus-conformance-v1"
    rows = {row["category"]: row for row in INDEX["files"]}
    assert set(rows) == set(CATEGORIES)
    for category, row in rows.items():
        data = (FIXTURE / row["path"]).read_bytes()
        assert hashlib.sha256(data).hexdigest() == row["sha256"]
        assert len(CASES[category]) == row["cases"], f"{category}: case count moved"
    assert INDEX["total_cases"] == sum(len(v) for v in CASES.values())


@pytest.mark.parametrize("case", CASES["json"], ids=_ids("json"))
def test_json_case(case: dict) -> None:
    data = bytes.fromhex(case["input_hex"])
    expect = case["expect"]
    if expect["outcome"] == "reject":
        with pytest.raises(CorpusError) as caught:
            parse_json(data)
        assert caught.value.code == expect["error"], case["id"]
        return
    text = render_json(parse_json(data))
    assert hashlib.sha256(text.encode("utf-8")).hexdigest() == expect["canonical_sha256"], case["id"]
    if "canonical" in expect:
        assert text == expect["canonical"], case["id"]


@pytest.mark.parametrize("case", CASES["tagged"], ids=_ids("tagged"))
def test_tagged_case(case: dict) -> None:
    expect = case["expect"]
    errors = case.get("errors", {})
    ty = parse_type(case["type"])
    try:
        value = decode_json(parse_json(bytes.fromhex(case["input_hex"])), ty, errors)
    except CorpusError as error:
        assert expect["outcome"] == "reject", case["id"]
        assert error.code == expect["error"], case["id"]
        return
    assert expect["outcome"] == "accept", case["id"]
    stream = encode_stream(value, ty)
    assert stream.hex() == expect["stream_hex"], case["id"]
    assert hashlib.sha256(stream).hexdigest() == expect["stream_sha256"], case["id"]


@pytest.mark.parametrize("case", CASES["containment"], ids=_ids("containment"))
def test_containment_case(case: dict) -> None:
    verdict = relative_inside(case["root"], case["child"], case["flavor"])
    assert (verdict is not None) == case["expect"]["inside"], case["id"]
    assert (verdict == "") == case["expect"]["same"], case["id"]


@pytest.mark.parametrize("case", CASES["label-digest"], ids=_ids("label-digest"))
def test_label_digest_case(case: dict) -> None:
    digests = {c["id"]: manifest_label_digest(c["manifest"]) for c in CASES["label-digest"]}
    assert digests[case["id"]] == case["sha256"]
    if "same_as" in case:
        assert digests[case["id"]] == digests[case["same_as"]]
    if "differs_from" in case:
        assert digests[case["id"]] != digests[case["differs_from"]]


def test_containment_helper_refuses_an_unknown_flavor() -> None:
    with pytest.raises(ValueError):
        relative_inside("/r", "/r/x", "amiga")


def test_containment_helper_on_resolved_pure_paths() -> None:
    from pathlib import PurePosixPath, PureWindowsPath

    assert relative_inside(PurePosixPath("/r"), PurePosixPath("/r/.hidden/x")) == ".hidden/x"
    assert relative_inside(PurePosixPath("/r"), PurePosixPath("/r")) == ""
    assert relative_inside(PurePosixPath("/r"), PurePosixPath("/rr")) is None
    assert relative_inside(PureWindowsPath("D:/r"), PureWindowsPath("C:/r/x")) is None
