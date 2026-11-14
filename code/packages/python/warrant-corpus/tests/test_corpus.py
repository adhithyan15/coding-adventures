"""Loading the corpus, and refusing every kind of tampered corpus."""

from __future__ import annotations

import json

import pytest
from helpers import CORPUS_DIR, REPO_ROOT, copy_corpus

from coding_adventures_warrant_corpus.corpus import list_control_rows, load_corpus
from coding_adventures_warrant_corpus.errors import CorpusError


def test_the_real_corpus_loads(corpus) -> None:
    assert corpus.module == "ct_compare"
    assert (len(corpus.examples), len(corpus.suites), len(corpus.controls)) == (
        12,
        9,
        8,
    )
    assert sum(suite.cases for suite in corpus.suites) == 572_109
    assert corpus.errors == {"SelectError": ["CT_SELECT_LENGTH_MISMATCH"]}
    assert corpus.ops["eq_fixed"].width_params == ("N",)
    assert corpus.ops["select_bytes"].fresh
    assert len(corpus.corpus_sha256) == 64 and len(corpus.cases_sha256) == 64


def test_list_control_rows() -> None:
    rows = list_control_rows(CORPUS_DIR)
    assert rows[0] == "ignore-last-byte:eq" and len(rows) == 8


def test_list_control_rows_needs_controls(tmp_path) -> None:
    (tmp_path / "corpus.json").write_bytes(b'{"controls": 3}')
    with pytest.raises(CorpusError):
        list_control_rows(tmp_path)
    (tmp_path / "corpus.json").write_bytes(b'{"controls": [3]}')
    with pytest.raises(CorpusError):
        list_control_rows(tmp_path)


def test_an_untouched_copy_loads(tmp_path) -> None:
    root, corpus_dir = copy_corpus(tmp_path)
    assert load_corpus(root, corpus_dir).module == "ct_compare"


def rejects(tmp_path, code: str | None = None, **tamper) -> None:
    root, corpus_dir = copy_corpus(tmp_path, **tamper)
    with pytest.raises(CorpusError) as caught:
        load_corpus(root, corpus_dir)
    if code is not None:
        assert caught.value.code == code


def test_a_changed_spec_is_stale_evidence(tmp_path) -> None:
    rejects(tmp_path, "stale-evidence", spec=lambda data: data + b"\n# edit\n")


def test_unreadable_files_are_refused(tmp_path) -> None:
    root, corpus_dir = copy_corpus(tmp_path)
    (corpus_dir / "cases.tfv").unlink()
    with pytest.raises(CorpusError, match="io"):
        load_corpus(root, corpus_dir)
    (corpus_dir / "cases.tfv").mkdir()  # a directory is not a regular file
    with pytest.raises(CorpusError, match="regular"):
        load_corpus(root, corpus_dir)


def test_oversized_input_is_refused(tmp_path, monkeypatch) -> None:
    monkeypatch.setattr("coding_adventures_warrant_corpus.corpus.MAX_CORPUS_BYTES", 10)
    root, corpus_dir = copy_corpus(tmp_path)
    with pytest.raises(CorpusError, match="input-limit"):
        load_corpus(root, corpus_dir)


def test_malformed_json_is_refused(tmp_path) -> None:
    root, corpus_dir = copy_corpus(tmp_path)
    for text in ("{", '{"a": 1, "a": 2}', "NaN", "\xff"):
        (corpus_dir / "corpus.json").write_bytes(text.encode("latin-1"))
        with pytest.raises(CorpusError):
            load_corpus(root, corpus_dir)


def test_top_level_shape_is_closed(tmp_path) -> None:
    rejects(tmp_path, "schema", corpus=lambda d: d.update(extra=1))
    rejects(tmp_path, "schema", corpus=lambda d: d.pop("controls"))
    rejects(tmp_path, "schema", corpus=lambda d: d.update(profile="warrant-corpus-v2"))
    rejects(tmp_path, "schema", corpus=lambda d: d.update(schema_version=2))
    rejects(tmp_path, "schema", corpus=lambda d: d.update(module="Bad-Module"))
    rejects(tmp_path, "schema", corpus=lambda d: d.update(spec="../escape.warrant"))
    rejects(tmp_path, "schema", corpus=lambda d: d.update(spec_sha256="nothex"))
    rejects(tmp_path, "schema", corpus=lambda d: d.update(types=[]))
    rejects(tmp_path, "schema", corpus=lambda d: d.update(types={"lower": ["X"]}))
    rejects(tmp_path, "schema", corpus=lambda d: d.update(models=[]))
    rejects(tmp_path, "schema", corpus=lambda d: d.update(examples={}))
    rejects(tmp_path, "schema", corpus=lambda d: d.update(suites=[]))
    rejects(tmp_path, "schema", corpus=lambda d: d.update(controls={}))


def test_operation_signatures_are_closed(tmp_path) -> None:
    rejects(tmp_path, corpus=lambda d: d.update(ops={}))
    rejects(tmp_path, corpus=lambda d: d["ops"]["eq"].update(extra=1))
    rejects(tmp_path, corpus=lambda d: d["ops"]["eq"].update(params=[["a"]]))
    rejects(tmp_path, corpus=lambda d: d["ops"]["eq"].update(ret="Float"))
    rejects(tmp_path, corpus=lambda d: d["ops"]["eq"].update(width_params="N"))
    rejects(tmp_path, corpus=lambda d: d["ops"].update({"BadName": d["ops"]["eq"]}))


def test_examples_are_checked(tmp_path) -> None:
    rejects(tmp_path, "schema", corpus=lambda d: d["examples"][0].update(extra=1))
    rejects(tmp_path, "schema", corpus=lambda d: d["examples"][0].update(id="other/id"))
    rejects(tmp_path, "example", corpus=lambda d: d["examples"][0].update(op="ghost"))
    rejects(tmp_path, "example", corpus=lambda d: d["examples"][0].update(args=[]))
    rejects(
        tmp_path, "example", corpus=lambda d: d["examples"][0].update(result_type="U8")
    )
    rejects(tmp_path, "example", corpus=lambda d: d["examples"][0].update(width=3))
    rejects(tmp_path, "example", corpus=lambda d: d["examples"][4].update(width=None))
    rejects(tmp_path, "example", corpus=lambda d: d["examples"][4].update(width=9))
    rejects(tmp_path, "schema", corpus=lambda d: d["examples"].append(d["examples"][0]))


def test_suites_are_checked(tmp_path) -> None:
    rejects(tmp_path, "schema", corpus=lambda d: d["suites"][0].update(extra=1))
    rejects(
        tmp_path,
        "schema",
        corpus=lambda d: d["suites"][0].update(
            id="warrant/ct_compare/exhaustive/other"
        ),
    )
    rejects(tmp_path, "schema", corpus=lambda d: d["suites"][0].update(ops=["ghost"]))
    rejects(tmp_path, "schema", corpus=lambda d: d["suites"][0].update(binders=[]))
    rejects(tmp_path, "schema", corpus=lambda d: d["suites"][0].update(binders=[["a"]]))
    rejects(
        tmp_path,
        "schema",
        corpus=lambda d: d["suites"][0].update(
            binders=[["a", {"domain": "bool"}], ["a", {"domain": "bool"}]]
        ),
    )
    rejects(
        tmp_path,
        "unsupported-domain",
        corpus=lambda d: d["suites"][0].update(binders=[["a", {"domain": "nope"}]]),
    )
    rejects(
        tmp_path,
        "unsupported-type",
        corpus=lambda d: d["suites"][0].update(result_type="List<U8>"),
    )
    rejects(tmp_path, "schema", corpus=lambda d: d["suites"][0].update(cases=-1))
    rejects(
        tmp_path,
        "schema",
        corpus=lambda d: d["suites"][0].update(inputs_sha256="short"),
    )
    rejects(tmp_path, "schema", corpus=lambda d: d["suites"][0].update(domain_text=""))
    rejects(tmp_path, "schema", corpus=lambda d: d["suites"].append(d["suites"][0]))


def test_controls_are_rederived_from_the_faults(tmp_path) -> None:
    rejects(tmp_path, "control", corpus=lambda d: d["controls"].pop())
    rejects(
        tmp_path,
        "control",
        corpus=lambda d: d["controls"][0].update(required_claims=[]),
    )
    rejects(tmp_path, "control", corpus=lambda d: d["controls"].reverse())
    rejects(tmp_path, "schema", corpus=lambda d: d["controls"][0].update(extra=1))
    rejects(tmp_path, "schema", corpus=lambda d: d.update(faults=[]))
    rejects(tmp_path, "schema", corpus=lambda d: d["faults"][0].update(extra=1))
    rejects(
        tmp_path, "fault", corpus=lambda d: d["faults"][0].update(fault="no-such-fault")
    )
    rejects(tmp_path, "fault", corpus=lambda d: d["faults"][0].update(op="ghost"))
    rejects(
        tmp_path, "fault", corpus=lambda d: d["faults"][0].update(fault="swap-choice")
    )
    rejects(
        tmp_path, "fault", corpus=lambda d: d["faults"][0].update(applies="sideways")
    )
    rejects(
        tmp_path, "fault", corpus=lambda d: d["faults"].append(dict(d["faults"][0]))
    )


def test_a_fault_no_claim_can_catch_is_refused(tmp_path) -> None:
    def unreachable(document: dict) -> None:
        for suite in document["suites"]:
            if suite["ops"] == ["eq_u64"]:
                suite["ops"] = ["eq"]  # no suite mentions eq_u64 any more

    rejects(tmp_path, "fault", corpus=unreachable)


def test_tfv_must_agree_with_the_examples(tmp_path) -> None:
    rejects(tmp_path, "tfv", tfv=lambda t: t.replace("Kexpect T", "Kexpect F", 1))
    rejects(
        tmp_path, "tfv", tfv=lambda t: t.rsplit("\n", 2)[0] + "\n"
    )  # missing record
    rejects(tmp_path, "tfv", tfv=lambda t: t.replace("eq_empty", "eq_other", 1))
    rejects(tmp_path, "tfv", tfv=lambda t: t.replace("Kop S6571", "Kop S6572", 1))
    rejects(
        tmp_path, "tfv", tfv=lambda t: t.replace("Ka X00 Kb X00", "Ka X01 Kb X00", 1)
    )
    rejects(tmp_path, "tfv", tfv=lambda t: t.replace("Kwidth I0", "Kwidth I1", 1))
    rejects(
        tmp_path,
        "tfv",
        tfv=lambda t: t.replace(
            "R4 Kop S6571 Ka X Kb X ", "R4 Kop S6571 Ka X Kc X ", 1
        ),
    )


def test_repo_root_is_used_for_the_spec() -> None:
    assert (REPO_ROOT / "code" / "specs" / "ct-compare.warrant").is_file()
    document = json.loads((CORPUS_DIR / "corpus.json").read_text())
    assert document["spec"] == "code/specs/ct-compare.warrant"
