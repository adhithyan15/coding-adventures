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
    try:
        ty = parse_type(case["type"])
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


def test_production_calls_apply_the_host_flavour_rules(monkeypatch) -> None:
    """`corpus_relative` and the binding's source check call the helper with no
    flavour; the device and `//` rules must apply on that path too."""
    from coding_adventures_warrant_corpus import containment
    from coding_adventures_warrant_corpus.errors import BindingError
    from coding_adventures_warrant_corpus.session import corpus_relative

    class Fake:
        def __init__(self, text: str) -> None:
            self.text = text

        def resolve(self) -> Fake:
            return self

        def __str__(self) -> str:
            return self.text

    monkeypatch.setattr(containment, "HOST_FLAVOR", "win32")
    assert corpus_relative(Fake("D:\\repo"), Fake("D:\\repo\\specs")) == "specs"
    for child in ("\\/?\\D:\\repo\\x", "//?/D:/repo/x", "\\\\?\\UNC\\s\\h\\x", "C:\\tmp"):
        with pytest.raises(BindingError):
            corpus_relative(Fake("D:\\repo"), Fake(child))
    monkeypatch.setattr(containment, "HOST_FLAVOR", "posix")
    for child in ("//repo/x", "/rr"):
        with pytest.raises(BindingError):
            corpus_relative(Fake("/repo"), Fake(child))


def test_containment_helper_refuses_an_unknown_flavor() -> None:
    with pytest.raises(ValueError):
        relative_inside("/r", "/r/x", "amiga")


def test_containment_helper_relative_text() -> None:
    assert relative_inside("/r", "/r/.hidden/x", "posix") == ".hidden/x"
    assert relative_inside("/r", "/r", "posix") == ""
    assert relative_inside("/r", "/rr", "posix") is None
    assert relative_inside("D:/r", "d:/R/x/y", "win32") == "x/y"
    assert relative_inside("D:/r", "C:/r/x", "win32") is None


def test_reserved_names_and_trailing_dots_are_refused_components() -> None:
    from coding_adventures_warrant_corpus.containment import refused_component

    for name in ("CON", "con", "CON.txt", "NUL.", "nul ", "COM1", "com9.log", "LPT1", "com1.tar.gz", "x.", "CON .txt", "NUL  .tar", "CON:stream", "CON:", "nul::$DATA", "file.txt:NUL", "COM\u00b9", "LPT\u00b2.txt", "CONIN$", "conout$"):
        assert refused_component(name), name
    for name in ("console.txt", "COM10", "LPT0", "COM0", "CONx", "xCON", ".hidden", "a.CON.b", "\u017fon", "CONsole", "CLOCK$", "\uff23\uff2f\uff2e", "CONIN$x", "COM\u2074"):
        assert not refused_component(name), name


def test_ascii_fold_maps_only_a_to_z() -> None:
    from coding_adventures_warrant_corpus.containment import ascii_fold

    assert ascii_fold("Repo-\u00c9") == "repo-\u00c9"
    for text in ("\u017f", "\u212a", "\u0130", "\u03a3", "\u03c2"):
        assert ascii_fold(text) == text


def test_production_win32_helper_refuses_reserved_names_and_folds_ascii_only(monkeypatch) -> None:
    from coding_adventures_warrant_corpus import containment

    monkeypatch.setattr(containment, "HOST_FLAVOR", "win32")
    assert relative_inside("D:\\repo", "d:\\REPO\\x") == "x"
    assert relative_inside("D:\\repo", "D:\\repo\\console.txt") == "console.txt"
    for child in ("D:\\repo\\NUL", "D:\\repo\\con.txt", "D:\\repo\\AUX\\x", "D:\\repo\\a."):
        assert relative_inside("D:\\repo", child) is None
    assert relative_inside("D:\\\u212a", "D:\\k\\x") is None
