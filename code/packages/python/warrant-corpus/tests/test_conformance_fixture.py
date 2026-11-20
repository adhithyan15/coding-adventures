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
import json
import shutil
from pathlib import Path

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


# The fixture family grows by categories (VER04r section 11, [D40]).  Every
# category this lane knows must have a row; a row it does not know is accepted
# only when its name starts with ``v2-``, and then only digest-checked, never
# run (this runner does not read v2 yet).  Rows are ordered: the four v1 rows
# in their present order, then the v2 rows, the known ones in the kit's order.
V2_ORDER = ("v2-value", "v2-arith", "v2-ir", "v2-domain", "v2-chunk", "v2-fault", "v2-binding")


def check_index(fixture: Path) -> int:
    """Validate ``fixture/index.json`` and return the number of rows run.

    Raises ``AssertionError`` on any violation, a malformed row included.
    Reads only through this lane's strict reader and never opens a file
    outside ``fixture``.
    """
    document = parse_json((fixture / "index.json").read_bytes())
    assert isinstance(document, dict)
    assert document["schema"] == "warrant-corpus-conformance-v1"
    files = document["files"]
    assert isinstance(files, list) and all(isinstance(row, dict) for row in files), "malformed files list"
    names = [row.get("category") for row in files]
    assert all(isinstance(name, str) for name in names), "malformed row: category is not a string"
    assert len(set(names)) == len(names), "duplicate category row"
    assert tuple(names[: len(CATEGORIES)]) == CATEGORIES, "v1 rows missing or out of order"
    extra = names[len(CATEGORIES) :]
    for name in extra:
        assert name.startswith("v2-"), f"unknown category row {name!r}"
    known = [name for name in extra if name in V2_ORDER]
    assert known == sorted(known, key=V2_ORDER.index), "v2 rows out of order"
    total = 0
    for row in files:
        category = row["category"]
        # The category name is safe to splice into a file name: non-empty, no
        # separators and no ``..`` (known v1 names and ``v2-`` names were
        # checked above).  Every row's path is then exactly ``<category>.json``.
        assert category and "/" not in category and "\\" not in category and ".." not in category, (
            f"{category!r}: unsafe category name"
        )
        path = row.get("path")
        assert path == category + ".json", f"{category}: bad path"
        file = fixture / path
        assert file.is_file(), f"{row['category']}: file missing"
        data = file.read_bytes()
        assert hashlib.sha256(data).hexdigest() == row["sha256"], f"{row['category']}: sha256 moved"
        body = parse_json(data)
        assert isinstance(body, dict) and body.get("category") == category, f"{category}: wrong category in file"
        assert type(row.get("cases")) is int, f"{category}: cases is not an integer"
        assert isinstance(body.get("cases"), list), f"{category}: file cases is not a list"
        assert len(body["cases"]) == row["cases"], f"{category}: case count moved"
        total += row["cases"]
    assert type(document.get("total_cases")) is int, "total_cases is not an integer"
    assert document["total_cases"] == total, "total_cases is not the sum of the rows"
    return len(files)


def test_index_matches_the_files_on_disk() -> None:
    assert check_index(FIXTURE) >= len(CATEGORIES)
    rows = {row["category"]: row for row in INDEX["files"]}
    for category in CATEGORIES:
        assert len(CASES[category]) == rows[category]["cases"], f"{category}: case count moved"


def _scratch_fixture(tmp_path: Path, extra_rows: list[dict], files: dict[str, bytes]) -> Path:
    """A temp copy of the fixture with extra index rows and files."""
    scratch = tmp_path / "fixture"
    shutil.copytree(FIXTURE, scratch)
    document = json.loads((scratch / "index.json").read_text(encoding="utf-8"))
    document["files"].extend(extra_rows)
    document["total_cases"] += sum(row["cases"] for row in extra_rows)
    (scratch / "index.json").write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")
    for name, data in files.items():
        (scratch / name).write_bytes(data)
    return scratch


def _extra_file(category: str, count: int = 2) -> tuple[dict, bytes]:
    body = {"schema": "warrant-corpus-conformance-v1", "category": category, "cases": [{"id": f"{category}/{i}"} for i in range(count)]}
    data = (json.dumps(body, indent=2) + "\n").encode("utf-8")
    row = {"cases": count, "category": category, "path": f"{category}.json", "sha256": hashlib.sha256(data).hexdigest()}
    return row, data


def test_index_accepts_a_digest_checked_v2_row(tmp_path: Path) -> None:
    row, data = _extra_file("v2-x")
    assert check_index(_scratch_fixture(tmp_path, [row], {"v2-x.json": data})) == len(CATEGORIES) + 1


def test_index_rejects_an_unknown_row_that_is_not_v2(tmp_path: Path) -> None:
    row, data = _extra_file("extra")
    with pytest.raises(AssertionError, match="unknown category"):
        check_index(_scratch_fixture(tmp_path, [row], {"extra.json": data}))


def test_index_rejects_a_v2_row_with_a_wrong_digest(tmp_path: Path) -> None:
    row, data = _extra_file("v2-x")
    row["sha256"] = "0" * 64
    with pytest.raises(AssertionError, match="sha256"):
        check_index(_scratch_fixture(tmp_path, [row], {"v2-x.json": data}))


def test_index_rejects_a_v2_row_with_a_wrong_case_count(tmp_path: Path) -> None:
    row, data = _extra_file("v2-x")
    row["cases"] = 3
    with pytest.raises(AssertionError, match="case count"):
        check_index(_scratch_fixture(tmp_path, [row], {"v2-x.json": data}))


def test_index_rejects_a_v2_row_with_a_missing_file(tmp_path: Path) -> None:
    row, _ = _extra_file("v2-x")
    with pytest.raises(AssertionError, match="file missing"):
        check_index(_scratch_fixture(tmp_path, [row], {}))


def test_index_rejects_a_missing_v1_row_and_v2_rows_out_of_order(tmp_path: Path) -> None:
    scratch = _scratch_fixture(tmp_path, [], {})
    document = json.loads((scratch / "index.json").read_text(encoding="utf-8"))
    document["files"].pop()
    (scratch / "index.json").write_text(json.dumps(document), encoding="utf-8")
    with pytest.raises(AssertionError, match="v1 rows"):
        check_index(scratch)
    value, value_data = _extra_file("v2-value")
    arith, arith_data = _extra_file("v2-arith")
    files = {"v2-value.json": value_data, "v2-arith.json": arith_data}
    check_index(_scratch_fixture(tmp_path / "ok", [value, arith], files))
    with pytest.raises(AssertionError, match="out of order"):
        check_index(_scratch_fixture(tmp_path / "bad", [arith, value], files))


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


def test_index_rejects_a_row_whose_path_is_not_category_json(tmp_path: Path) -> None:
    row, data = _extra_file("v2-x")
    row["path"] = "v2-y.json"
    with pytest.raises(AssertionError, match="bad path"):
        check_index(_scratch_fixture(tmp_path, [row], {"v2-x.json": data, "v2-y.json": data}))


def test_index_rejects_a_boolean_case_count(tmp_path: Path) -> None:
    row, data = _extra_file("v2-x", count=1)
    row["cases"] = True
    with pytest.raises(AssertionError, match="not an integer"):
        check_index(_scratch_fixture(tmp_path, [row], {"v2-x.json": data}))
