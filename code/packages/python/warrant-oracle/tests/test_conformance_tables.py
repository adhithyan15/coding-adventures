"""The v2 conformance case tables and ``make_conformance.py`` (slice S5a).

``kit`` (see ``conftest.py``) derives the whole kit once per session; the tests here check what S5a's
acceptance names: the build reproduces, every case has ``requires`` ``["ir.core"]``, the ten slugs of
section 3.4, every Appendix A tag and every ``ir.core`` leaf descriptor are present, every code of every
closed error set is exercised, and ``--write`` then ``--check`` into a scratch directory exit 0.
"""

from __future__ import annotations

import json
import re

import pytest

from coding_adventures_warrant_oracle import conformance as C
from coding_adventures_warrant_oracle import levels as L
from coding_adventures_warrant_oracle.corpus_v2 import canonical
from coding_adventures_warrant_oracle.irtree import children, is_node


def cases_of(kit, category):
    return json.loads(kit.files[f"{category}.json"])["cases"]


def all_cases(kit):
    return {c: cases_of(kit, c) for c in C.CATEGORIES}


def test_the_kit_has_six_files_in_the_fixed_order(kit):
    assert list(kit.files) == [f"{c}.json" for c in C.CATEGORIES]
    assert [r["category"] for r in kit.rows] == list(C.CATEGORIES)
    assert sum(r["cases"] for r in kit.rows) == sum(len(rows) for rows in kit.tables.values())
    for r in kit.rows:
        assert r["path"] == f"{r['category']}.json"


def test_the_build_is_deterministic(kit):
    """A second build from the same tables and kit documents gives the same bytes (cases are in table order)."""
    files, rows = C.build_files(kit.mc.load_tables(kit.kits), kit.kits)
    assert files == kit.files and rows == kit.rows


def test_files_are_canonical_ascii_lf_and_have_the_family_schema(kit):
    for category, text in ((c, kit.files[f"{c}.json"]) for c in C.CATEGORIES):
        assert text.isascii() and "\r" not in text and text.endswith("}\n") and not text.endswith("\n\n")
        doc = json.loads(text)
        assert set(doc) == {"schema", "category", "cases"}
        assert doc["schema"] == "warrant-corpus-conformance-v1" and doc["category"] == category
        assert canonical(doc).decode() == text, f"{category} is not in the canonical form"


def test_ids_are_unique_prefixed_slugged_and_in_table_order(kit):
    seen = set()
    for category, cases in all_cases(kit).items():
        assert [c["id"] for c in cases] == [f"{category}/{row.slug}" for row in kit.tables[category]]
        for case in cases:
            assert re.fullmatch(r"v2-[a-z]+/[a-z0-9]+(-[a-z0-9]+)*", case["id"]), case["id"]
            assert case["id"] not in seen
            seen.add(case["id"])
            assert case["summary"].isascii() and "\n" not in case["summary"] and 0 < len(case["summary"]) <= 240
            assert set(case) >= {"id", "summary", "requires", "expect"}


def test_every_s5_case_requires_exactly_ir_core(kit):
    for cases in all_cases(kit).values():
        for case in cases:
            assert case["requires"] == ["ir.core"], case["id"]


def test_error_codes_stay_in_the_closed_sets_and_every_code_is_used(kit):
    for category, cases in all_cases(kit).items():
        used = {c["expect"]["error"] for c in cases if "error" in c["expect"]}
        # [D74] (16.22) adds ``ill-typed`` to ``v2-fault``; its kit vectors are S10d's and S11d's, so the committed
        # kit does not use it yet.  [D77] (16.23) adds ``unknown-tag``, ``depth`` and ``model-order``, likewise.
        pending = {"ill-typed", "unknown-tag", "depth", "model-order"} if category == "v2-fault" else set()
        assert used == C.ERROR_CODES[category] - pending, (category, used ^ C.ERROR_CODES[category])
        for case in cases:
            if category in ("v2-value",):
                assert case["expect"]["outcome"] in ("accept", "reject")
                assert ("error" in case["expect"]) == (case["expect"]["outcome"] == "reject")
            elif "error" in case["expect"] and category != "v2-fault":
                assert set(case["expect"]) <= {"error", "class"}


def test_the_ten_slugs_of_section_3_4(kit):
    ids = {c["id"] for c in cases_of(kit, "v2-arith")}
    for slug in ["u8-add-wrap", "u8-sub-wrap", "u8-mul-wrap", "u64-add-wrap", "u64-mul-wrap", "i8-neg-min", "i64-sub-wrap", "nat-sub-is-int", "nat-bound-generation-error", "v0-u8-wrap-guard"]:
        assert f"v2-arith/{slug}" in ids
    by_id = {c["id"]: c for c in cases_of(kit, "v2-arith")}
    assert by_id["v2-arith/v0-u8-wrap-guard"]["document"] == "v0"
    assert by_id["v2-arith/v0-u8-wrap-guard"]["expect"] == {"type": "U8", "value": "0"}
    assert by_id["v2-arith/nat-bound-generation-error"]["expect"] == {"error": "ir-overflow"}
    assert by_id["v2-arith/nat-sub-is-int"]["expect"] == {"type": "Int", "value": "-2"}
    assert "document" not in by_id["v2-arith/u8-add-wrap"]


def tags_of(ir):
    out = set()
    stack = [ir]
    while stack:
        node = stack.pop()
        if is_node(node):
            out.add(node[0])
            stack.extend(children(node))
    return out


def test_every_appendix_a_tag_occurs_in_v2_ir(kit):
    seen = set()
    for case in cases_of(kit, "v2-ir"):
        seen |= tags_of(case["ir"])
        for model in case["models"].values():
            seen |= tags_of(model["body"])
    assert seen >= L.CORE_TAGS, sorted(L.CORE_TAGS - seen)
    assert len(L.CORE_TAGS) == 26


def test_level_missing_uses_only_fixed_and_tuple_tags_and_unknown_tag_avoids_reserved_names(kit):
    """S5-0 erratum (PR 19723): an unknown-tag vector never uses a tag of ir.data or ir.seq.

    S5e (R69): a ``level-missing`` that a *type* causes has no offending tag, and a model that ``ir`` does not
    reach may hold a tag of a missing level in a case that derives a value.
    """
    reserved = {"some", "none", "ctor", "field", "list", *L.SEQ_TAGS}
    for case in cases_of(kit, "v2-ir"):
        tags = tags_of(case["ir"]).union(*(tags_of(m["body"]) for m in case["models"].values()))
        reached = set().union(*(tags_of(t) for t in C.reach(case)[0]))
        if case["expect"].get("error") == "level-missing":
            offending = {t for t in tags if t not in L.CORE_TAGS}
            if offending:
                assert offending <= (L.FIXED_TAGS | L.TUPLE_TAGS), case["id"]
            else:  # a type of a missing level in env: a tuple or the reserved ir.list
                assert any(t["type"].startswith(("(", "List<")) for _, t in case["env"]), case["id"]
        if case["expect"].get("error") == "unknown-tag":
            assert not (tags & reserved), case["id"]
            assert any(L.tag_level(t) is None and t != "match" for t in tags), case["id"]
        if case["expect"].get("error") not in ("level-missing", "unknown-tag"):
            assert reached <= L.CORE_TAGS, case["id"]
            assert tags <= L.CORE_TAGS or case["id"] == "v2-ir/level-missing-model-not-reached-is-a-value", case["id"]


LEAVES = {
    "bool": None,
    "u8_all": None,
    "u64_edge": None,
    "nat_0_64": None,
    "bytes_d1": None,
    "bytes_n": {0, 1, 2, 4, 16},
    "fx_full": {"I8", "U16", "I16"},
    "fx_edge": {"U16", "I16", "U32", "I32", "I64"},
    "nat_count": {0, 1, 4096},
}


def kind_of(case):
    d = case["descriptor"]
    return d.get("d") if isinstance(d, dict) else None


def test_every_ir_core_leaf_descriptor_is_present_and_probed_at_the_ends(kit):
    cases = cases_of(kit, "v2-domain")
    valid = {}  # the descriptors that are accepted: json text -> descriptor
    for c in cases:
        if c["expect"].get("error") != "descriptor" and isinstance(c["descriptor"], dict):
            valid[json.dumps(c["descriptor"], sort_keys=True)] = c["descriptor"]
    for kind, params in LEAVES.items():
        mine = [d for d in valid.values() if d["d"] == kind]
        assert mine, kind
        key = {"bytes_n": "n", "nat_count": "k", "fx_full": "t", "fx_edge": "t"}.get(kind)
        if params is not None:
            assert {d[key] for d in mine} >= params, kind
    # every leaf is read at index 0, at size - 1 and at size (the index error)
    for text, d in valid.items():
        if d["d"] not in LEAVES:
            continue
        probes = [c for c in cases if c["descriptor"] == d]
        size = d["size"]
        indexes = {c["index"] for c in probes if "index" in c}
        ranges = [c["range"] for c in probes if "range" in c]
        assert 0 in indexes or any(r[0] == 0 for r in ranges), text
        assert size in indexes, text
        assert size - 1 in indexes or any(r[1] == size for r in ranges), text
    assert any(c["expect"].get("size") for c in cases if kind_of(c) == "result")
    assert not {kind_of(c) for c in cases} & {"tuple", "nat_shift", "variant", "option", "list"}


def test_the_mandatory_value_vectors_are_present(kit):
    by_slug = {c["id"].split("/")[1]: c for c in cases_of(kit, "v2-value")}
    assert by_slug["i8-minus-one"]["expect"]["stream_hex"] == "11" + "ff" * 8
    assert by_slug["i8-min"]["expect"]["stream_hex"] == "11" + "80" + "ff" * 7
    for slug in ("decimal-leading-zero", "decimal-plus-sign", "decimal-21-digits", "i8-negative-zero", "bytes-n-short",
                 "bytes-n-long", "result-unknown-case", "type-depth-17-wrappers"):
        assert by_slug[slug]["expect"]["outcome"] == "reject", slug
    assert by_slug["type-depth-17-wrappers"]["expect"]["error"] == "type"
    assert by_slug["result-unknown-case"]["expect"]["error"] == "value"


def test_the_mandatory_ir_vectors_are_present(kit):
    by_slug = {c["id"].split("/")[1]: c for c in cases_of(kit, "v2-ir")}
    assert by_slug["short-circuit-and-untaken"]["expect"]["calls"] == 0
    assert by_slug["short-circuit-and-taken"]["expect"]["calls"] == 1
    assert by_slug["impure-guard-if"]["expect"] == {"error": "impure-guard"}
    assert by_slug["subject-op-nested-associativity"]["expect"]["calls"] == 4
    assert by_slug["unknown-tag-frobnicate"]["expect"] == {"error": "unknown-tag"}
    assert by_slug["level-missing-bit-xor"]["expect"] == {"error": "level-missing"}
    assert by_slug["overflow-nat-product-corpus-class"]["expect"] == {"error": "ir-overflow", "class": "corpus"}
    assert by_slug["overflow-below-subject-case-class"]["expect"] == {"error": "ir-overflow", "class": "case"}
    assert by_slug["overflow-doubling-concat-chain"]["expect"]["error"] == "ir-overflow"
    assert by_slug["depth-expression-64-accepted"]["expect"]["type"] == "Bool"
    assert by_slug["depth-expression-65-refused"]["expect"] == {"error": "depth"}
    assert by_slug["depth-model-frames-65-refused"]["expect"] == {"error": "depth"}
    assert by_slug["max-ir-cost-65537-refused"]["max_ir_cost"] == 65537
    assert all("max_ir_cost" not in c for c in cases_of(kit, "v2-ir") if c["id"].endswith(("cost-2000-accepted", "short-circuit-and-taken")))


def test_the_mandatory_chunk_vectors_are_present(kit):
    cases = {c["id"].split("/")[1]: c for c in cases_of(kit, "v2-chunk")}
    for slug in ("big-first-chunk", "big-middle-chunk", "big-last-chunk"):
        assert cases[slug]["suite"]["product"] == 2**24 and cases[slug]["suite"]["chunks"] == 256
        assert cases[slug]["also_sequential"] is True
    assert cases["gap-chunk-with-zero-admitted-cases"]["expect"]["cases"] == 0
    assert cases["gap-chunk-after-the-discarded-run"]["expect"]["first_admitted"] == 512
    assert cases["short-predicate-chunk"]["suite"]["oracle"]["kind"] == "predicate"
    assert "expected_sha256" not in cases["short-predicate-chunk"]["expect"]
    assert cases["grid-first-chunk-1024"]["corpus"]["chunk_size"] == 1024
    assert cases["chunk-file-first-admitted-off-by-one"]["expect"] == {"error": "chunk-file"}
    assert cases["slim-chunks-omits-the-first-chunk"]["expect"] == {"error": "slim-chunks"}
    assert cases["slim-chunks-omits-the-last-chunk"]["expect"] == {"error": "slim-chunks"}
    assert cases["negs-signed-chunk"]["suite"]["stream"] == "warrant-stream-v2"
    for c in cases.values():
        assert set(c) <= {"id", "summary", "requires", "corpus", "suite", "chunk_file_hex", "chunk", "also_sequential", "expect"}
        assert set(c["corpus"]) == C.CHUNK_CORPUS_KEYS or c["expect"].get("error") == "corpus"


def test_the_fault_cases_use_the_oracles_own_templates(kit):
    from coding_adventures_warrant_oracle.faults import REGISTRY

    names = {c["fault"]["fault"] for c in cases_of(kit, "v2-fault")}
    assert {"invert", "off-by-one", "truncate-last", "drop-last-result-byte", "missing-error", "swap-choice",
            "ignore-last-byte", "xor-lsb"} <= names & set(REGISTRY) | names
    for case in cases_of(kit, "v2-fault"):
        assert set(case) == {"id", "summary", "requires", "fault", "op", "types", "levels", "args", "result", "expect"}
        assert case["levels"] == ["ir.core"]


# -- make_conformance.py ---------------------------------------------------------------------------------


def test_write_then_check_into_a_scratch_directory(kit, tmp_path):
    assert kit.mc.main(["--write", "--fixture", str(tmp_path / "out")]) == 0
    written = {p.name: p.read_bytes() for p in (tmp_path / "out").iterdir()}
    binding_text, _ = kit.mc.build_binding()  # S5c: the seventh file is written beside the six, still no index.json
    assert written == {**{name: text.encode() for name, text in kit.files.items()}, "v2-binding.json": binding_text.encode()}
    assert kit.mc.main(["--check", "--fixture", str(tmp_path / "out")]) == 0


@pytest.fixture()
def prebuilt(kit, monkeypatch):
    """``build()`` answers from the session's derivation, so the exit-code tests do not rebuild the kit."""
    monkeypatch.setattr(kit.mc, "build", lambda: (kit.files, kit.rows))
    return kit


def all_rows(kit):
    """The six derived rows and the seventh, ``v2-binding`` (S5d: ``--check`` requires it like the others)."""
    return [*kit.rows, kit.mc.build_binding()[1]]


def write_files(kit, directory, index=True):
    directory.mkdir(parents=True, exist_ok=True)
    for name, text in {**kit.files, "v2-binding.json": kit.mc.build_binding()[0]}.items():
        (directory / name).write_bytes(text.encode())
    if index:
        v1 = [{"category": "json", "path": "json.json", "cases": 1, "sha256": "0" * 64}]
        (directory / "index.json").write_bytes(canonical({"schema": "x", "total_cases": 1, "files": v1 + all_rows(kit)}))


def test_check_exit_codes(prebuilt, tmp_path, capsys):
    kit, mc = prebuilt, prebuilt.mc
    write_files(kit, tmp_path)
    assert mc.main(["--check", "--fixture", str(tmp_path)]) == 0
    assert "identical" in capsys.readouterr().out
    # the bytes differ
    path = tmp_path / "v2-arith.json"
    path.write_bytes(path.read_bytes().replace(b"u8-add-wrap", b"u8-add-wraps", 1))
    assert mc.main(["--check", "--fixture", str(tmp_path)]) == 1
    assert "v2-arith.json differs" in capsys.readouterr().err
    path.unlink()  # a missing file differs too
    assert mc.main(["--check", "--fixture", str(tmp_path)]) == 1


def test_check_verifies_index_rows_and_skips_a_missing_index(prebuilt, tmp_path, capsys):
    kit, mc = prebuilt, prebuilt.mc
    write_files(kit, tmp_path / "none", index=False)
    assert mc.main(["--check", "--fixture", str(tmp_path / "none")]) == 0  # no index.json: that check is skipped
    write_files(kit, tmp_path / "ok")
    assert mc.main(["--check", "--fixture", str(tmp_path / "ok")]) == 0
    for name, edit in (
        ("stale", lambda rows: [{**rows[0], "sha256": "f" * 64}, *rows[1:]]),
        ("missing", lambda rows: rows[1:]),
        ("extra", lambda rows: [*rows, {**rows[0], "category": "v2-extra"}]),
        ("order", lambda rows: rows[::-1]),
    ):
        write_files(kit, tmp_path / name)
        v1 = [{"category": "json", "path": "json.json", "cases": 1, "sha256": "0" * 64}]
        (tmp_path / name / "index.json").write_bytes(canonical({"schema": "x", "total_cases": 1, "files": v1 + edit(all_rows(kit))}))
        assert mc.main(["--check", "--fixture", str(tmp_path / name)]) == 1, name
        assert "index.json" in capsys.readouterr().err
    (tmp_path / "ok" / "index.json").write_bytes(b"not json")
    assert mc.main(["--check", "--fixture", str(tmp_path / "ok")]) == 1
    (tmp_path / "ok" / "index.json").write_bytes(b"{}")
    assert mc.main(["--check", "--fixture", str(tmp_path / "ok")]) == 1


def test_a_table_that_disagrees_with_the_oracle_is_exit_2_naming_the_case(kit, monkeypatch, tmp_path, capsys):
    mc = kit.mc

    def broken():
        raise C.KitError("the table intends X but the oracle derives Y", "v2-arith/u8-add-wrap")

    monkeypatch.setattr(mc, "build", broken)
    assert mc.main(["--check", "--fixture", str(tmp_path)]) == 2
    assert "v2-arith/u8-add-wrap" in capsys.readouterr().err
    monkeypatch.setattr(mc, "build", lambda: 1 / 0)
    assert mc.main(["--write", "--fixture", str(tmp_path)]) == 2
    assert "ZeroDivisionError" in capsys.readouterr().err


def test_an_intent_that_disagrees_with_the_oracle_fails_the_build_naming_the_case(kit):
    rows = list(kit.tables["v2-arith"])
    bad = C.Row(rows[0].slug, rows[0].summary, rows[0].fields, {"type": "U8", "value": "255"})
    with pytest.raises(C.KitError, match=rf"v2-arith/{rows[0].slug}"):
        C.build_category("v2-arith", [bad])


def test_the_script_requires_a_mode(kit):
    with pytest.raises(SystemExit):
        kit.mc.main([])


def test_the_default_fixture_directory_is_the_family_directory(kit):
    assert kit.mc.FAMILY.name == "warrant-corpus-conformance-v1"
    assert kit.mc.FAMILY.parent.name == "fixtures"
