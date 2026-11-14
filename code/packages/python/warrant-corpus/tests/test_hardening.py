"""Hardening round (security review): bounded work, honest labels, strict types."""

from __future__ import annotations

import dataclasses
import json
import os
from pathlib import Path

import pytest
from helpers import copy_corpus, mini_corpus, new_library
from test_binding import make_subject

from coding_adventures_warrant_corpus import (
    RunConfig,
    execute,
    load_corpus,
    main,
    prepare,
)
from coding_adventures_warrant_corpus.binding import SubjectFailure
from coding_adventures_warrant_corpus.cli import _child_env
from coding_adventures_warrant_corpus.errors import CorpusError
from coding_adventures_warrant_corpus.ir import MAX_IR_COST, Compiler
from coding_adventures_warrant_corpus.labels import derive_labels, domain_label
from coding_adventures_warrant_corpus.runner import run_suite
from coding_adventures_warrant_corpus.values import OpSig, parse_type

W2 = "eq_fixed_agrees_w2"


def loads_mini(tmp_path, change):
    root, corpus_dir = mini_corpus(tmp_path)
    document = json.loads((corpus_dir / "corpus.json").read_text())
    change(document)
    (corpus_dir / "corpus.json").write_bytes((json.dumps(document)).encode())
    return root, corpus_dir


def suite_row(document, claim=W2):
    return next(s for s in document["suites"] if s["claim"] == claim)


# -- 1. bounded work ------------------------------------------------------
def test_a_product_that_disagrees_with_cases_plus_discarded_is_refused(
    tmp_path,
) -> None:
    for index, (field, value) in enumerate(
        (("cases", 255), ("discarded", 1), ("cases", 10**9))
    ):
        root, corpus_dir = loads_mini(
            tmp_path / str(index),
            lambda d, field=field, value=value: suite_row(d).update({field: value}),
        )
        with pytest.raises(CorpusError, match="declared"):
            load_corpus(root, corpus_dir)


def test_run_suite_rechecks_the_product_before_enumerating(
    tmp_path, monkeypatch, corpus
) -> None:
    subject = make_subject(tmp_path, monkeypatch)
    real = next(s for s in corpus.suites if s.claim == W2)
    lying = dataclasses.replace(real, cases=1)
    with pytest.raises(CorpusError, match="declared"):
        run_suite(corpus, lying, subject, fail_fast=False)
    assert sum(subject.calls.values()) == 0  # nothing ran


def test_a_suite_over_the_per_suite_cap_is_refused(tmp_path, monkeypatch) -> None:
    root, corpus_dir = mini_corpus(tmp_path)
    monkeypatch.setattr(
        "coding_adventures_warrant_corpus.corpus.MAX_SUITE_PRODUCT", 100
    )
    with pytest.raises(CorpusError, match="work"):
        load_corpus(root, corpus_dir)


def test_total_work_including_control_reruns_is_capped(tmp_path, monkeypatch) -> None:
    root, corpus_dir = mini_corpus(tmp_path)
    load_corpus(root, corpus_dir)  # 512 + 2 controls x 512 = 1536 units: fine
    monkeypatch.setattr("coding_adventures_warrant_corpus.corpus.MAX_TOTAL_WORK", 1535)
    with pytest.raises(CorpusError, match="total work"):
        load_corpus(root, corpus_dir)


def test_zero_admitted_cases_is_refused(tmp_path) -> None:
    root, corpus_dir = loads_mini(
        tmp_path, lambda d: suite_row(d).update(cases=0, discarded=256)
    )
    with pytest.raises(CorpusError, match="vacuous|zero|at least one"):
        load_corpus(root, corpus_dir)


MODELS_BOMB = {
    **{
        f"m{i}": {
            "params": ["a"],
            "body": [
                "and",
                ["model_call", f"m{i + 1}", [["var", "a"]]],
                ["model_call", f"m{i + 1}", [["var", "a"]]],
            ],
        }
        for i in range(40)
    },
    "m40": {"params": ["a"], "body": ["lit", True]},
}


def test_a_model_call_bomb_is_refused_at_compile_time() -> None:
    compiler = Compiler(MODELS_BOMB, None, {})
    with pytest.raises(CorpusError, match="cost"):
        compiler.compile(["model_call", "m0", [["lit", True]]], set())
    assert MAX_IR_COST >= 100


def test_a_very_long_model_chain_is_a_corpus_error_not_a_crash() -> None:
    models = {
        f"c{i}": {"params": [], "body": ["model_call", f"c{i + 1}", []]}
        for i in range(3000)
    }
    models["c3000"] = {"params": [], "body": ["lit", True]}
    with pytest.raises(CorpusError):
        Compiler(models, None, {}).compile(["model_call", "c0", []], set())


def test_a_model_bomb_in_a_corpus_is_refused_before_any_case(
    tmp_path, monkeypatch
) -> None:
    def tamper(document):
        document["models"] = MODELS_BOMB
        document["models"]["same"] = {
            "params": ["a", "b"],
            "body": ["model_call", "m0", [["var", "a"]]],
        }

    root, corpus_dir = loads_mini(tmp_path, tamper)
    _n, package_dir, binding = new_library(tmp_path, monkeypatch)
    prepared = prepare(RunConfig(root, corpus_dir, binding, package_dir))
    with pytest.raises(CorpusError):
        execute(prepared, lambda _l: None)


# -- 6. arithmetic operands -----------------------------------------------
@pytest.mark.parametrize("tag", ["mul", "add", "sub"])
def test_arithmetic_operands_are_range_checked_first(tag) -> None:
    compiler = Compiler({}, None, {})
    for left, right in ((1 << 200, 2), (-1, 1), (2, 1 << 64)):
        function = compiler.compile([tag, ["lit", left], ["lit", right]], set())
        with pytest.raises(CorpusError):
            function({})


def test_arithmetic_rejects_booleans() -> None:
    function = Compiler({}, None, {}).compile(["add", ["lit", True], ["lit", 1]], set())
    with pytest.raises(CorpusError):
        function({})


# -- 3. exact exception classes -------------------------------------------
def test_a_subclass_of_the_mapped_exception_is_not_the_declared_error(
    tmp_path, monkeypatch
) -> None:
    source = (
        "class Unrelated(ValueError):\n    pass\n"
        "def ct_select_bytes(a, b, choice):\n    raise Unrelated('not a length mismatch')\n"
    )
    subject = make_subject(tmp_path, monkeypatch, {"ct_select_bytes": source})
    with pytest.raises(SubjectFailure) as caught:
        subject.call("select_bytes", [b"", b"", True], None)
    assert caught.value.failure_id == "unexpected-error"


def test_the_exact_class_still_maps(tmp_path, monkeypatch) -> None:
    subject = make_subject(tmp_path, monkeypatch)
    assert subject.call("select_bytes", [b"a", b"bb", True], None) == (
        "err",
        "CT_SELECT_LENGTH_MISMATCH",
    )


# -- 4. malformed structure is a CorpusError, never a TypeError ------------
@pytest.mark.parametrize(
    "change",
    [
        lambda d: d["ops"]["eq"].update(params=5),
        lambda d: d["ops"]["eq"].update(params=[5]),
        lambda d: d["ops"]["eq"].update(params=[[1, 2]]),
        lambda d: d["ops"]["eq"].update(ret=5),
        lambda d: d["ops"]["eq"].update(width_params=5),
        lambda d: d["ops"].update(eq=5),
        lambda d: d.update(ops=5),
        lambda d: d.update(types=5),
        lambda d: d.update(types={"E": 5}),
        lambda d: d.update(models=5),
        lambda d: d.update(examples=5),
        lambda d: d["examples"].append(5),
        lambda d: d["examples"][0].update(args=5),
        lambda d: d["examples"][0].update(id=[1]),
        lambda d: d["suites"].append(5),
        lambda d: d["suites"][0].update(ops=[[1]]),
        lambda d: d["suites"][0].update(ops=5),
        lambda d: d["suites"][0].update(binders=5),
        lambda d: d["suites"][0].update(binders=[5]),
        lambda d: d["suites"][0].update(cases="many"),
        lambda d: d["suites"][0].update(cases=True),
        lambda d: d["suites"][0].update(result_type=5),
        lambda d: d["faults"].append(5),
        lambda d: d.update(faults=5),
        lambda d: d["faults"][0].update(fault=[1]),
        lambda d: d["controls"].append(5),
        lambda d: d["controls"][0].update(required_claims=5),
        lambda d: d["controls"][0].update(required_claims=[1]),
        lambda d: d.update(controls=5),
        lambda d: d.update(spec=5),
    ],
)
def test_malformed_field_types_are_corpus_errors(tmp_path, change) -> None:
    root, corpus_dir = copy_corpus(tmp_path, corpus=change)
    with pytest.raises(CorpusError):
        load_corpus(root, corpus_dir)


# -- 7. widths ------------------------------------------------------------
FIXED = {
    "eq_fixed": OpSig(
        "eq_fixed", (("a", parse_type("Bytes<N>")),), parse_type("Bool"), ("N",)
    )
}
PLAIN = {"eq": OpSig("eq", (("a", parse_type("Bytes")),), parse_type("Bool"))}


def test_a_fixed_width_op_must_state_its_width_in_the_ir() -> None:
    compiler = Compiler({}, lambda *_: True, FIXED)
    with pytest.raises(CorpusError, match="width"):
        compiler.compile(["subject_op", "eq_fixed", [["lit", True]]], set())
    compiler.compile(["subject_op", "eq_fixed", [["lit", True]], [2]], set())


def test_a_width_on_an_op_without_width_params_is_refused() -> None:
    with pytest.raises(CorpusError, match="width"):
        Compiler({}, lambda *_: True, PLAIN).compile(
            ["subject_op", "eq", [["lit", True]], [2]], set()
        )


def test_literal_widths_in_the_signature_are_asserted_at_call_time(
    tmp_path, monkeypatch
) -> None:
    subject = make_subject(tmp_path, monkeypatch)
    subject._signatures["eq"] = OpSig(
        "eq",
        (("a", parse_type("Bytes<2>")), ("b", parse_type("Bytes<2>"))),
        parse_type("Bool"),
    )
    assert subject.call("eq", [b"ab", b"ab"], None) is True
    with pytest.raises(CorpusError, match="length"):
        subject.call("eq", [b"abc", b"ab"], None)


# -- 2. honest labels -----------------------------------------------------
@pytest.mark.parametrize(
    ("domain", "label"),
    [
        ("bool", "full"),
        ("u8_all", "full"),
        ("bytes_n(0)", "full"),
        ("bytes_n(1)", "full"),
        ("bytes_n(2)", "bounded-set"),
        ("bytes_n(16)", "bounded-set"),
        ("bytes_d1", "bounded-set"),
        ("u64_edge", "bounded-set"),
        ("nat_0_64", "bounded-set"),
    ],
)
def test_domain_labels(domain, label) -> None:
    assert domain_label(domain) == label


def test_unknown_domains_have_no_label() -> None:
    with pytest.raises(CorpusError):
        domain_label("mystery")


def test_the_real_corpus_labels_two_full_and_seven_bounded_suites(corpus) -> None:
    labels = derive_labels(corpus, "python/ct-compare", "ct_compare", "m" * 64)
    by_label = {}
    for row in labels["suites"]:
        by_label.setdefault(row["label"], []).append(row["id"].rsplit("_", 1)[-1])
    assert sorted(by_label["full"]) == ["w0", "w1"]
    assert len(by_label["bounded-set"]) == 7 and "sample" not in by_label
    assert labels["cases_by_label"] == {
        "bounded-set": 572_109 - 1 - 65_536,
        "full": 1 + 65_536,
    }
    assert "LP-012" in labels["note"]
    assert labels["schema"] == "warrant-lane-domain-labels-v1"


def test_run_writes_the_label_sidecar_and_check_guards_it(
    tmp_path, monkeypatch, capsys
) -> None:
    root, corpus_dir = mini_corpus(tmp_path)
    _n, package_dir, binding = new_library(tmp_path, monkeypatch)
    common = ["--repo-root", str(root), "--corpus", str(corpus_dir),
              "--binding", str(binding), "--package-dir", str(package_dir)]  # fmt: skip
    out = tmp_path / "evidence" / "test-manifest.json"
    assert main(["run", *common, "--out", str(out)]) == 0
    sidecar = out.parent / "domain-labels.json"
    document = json.loads(sidecar.read_text())
    assert document["suites"][0]["label"] in ("full", "bounded-set")
    assert main(["check", *common, str(out)]) == 0
    assert main(["validate-manifest", *common, str(out)]) == 0

    # an overclaiming label is drift, and invalid
    document["suites"][0]["label"] = (
        "full" if document["suites"][0]["label"] != "full" else "sample"
    )
    sidecar.write_bytes(
        (json.dumps(document, indent=2, sort_keys=True) + "\n").encode()
    )
    assert main(["check", *common, str(out)]) == 1
    assert main(["validate-manifest", *common, str(out)]) == 2
    sidecar.unlink()
    assert main(["check", *common, str(out)]) == 1
    assert "domain-labels" in capsys.readouterr().out


# -- 8. sabotage child environment ----------------------------------------
def test_the_sabotage_child_always_finds_the_runner() -> None:
    env = _child_env({"PYTHONPATH": "elsewhere", "OTHER": "1"}, "invert:eq")
    entries = env["PYTHONPATH"].split(os.pathsep)
    assert "elsewhere" in entries
    import coding_adventures_warrant_corpus as pkg

    assert str(Path(pkg.__file__).resolve().parent.parent) in entries
    assert entries.index(str(Path(pkg.__file__).resolve().parent.parent)) == 0
    assert env["WARRANT_EVIDENCE_FAULT"] == "invert:eq" and env["OTHER"] == "1"
    assert "WARRANT_EVIDENCE_OUT" not in _child_env(
        {"WARRANT_EVIDENCE_OUT": "x"}, "a:b"
    )
    assert _child_env({}, "a:b")["PYTHONPATH"] == str(
        Path(pkg.__file__).resolve().parent.parent
    )


def test_fixture_helpers_preserve_bytes_exactly(tmp_path) -> None:
    """Text-mode copies translate LF to CRLF on Windows and break the TFV header."""
    from helpers import CORPUS_DIR

    _root, corpus_dir = copy_corpus(tmp_path)
    assert (corpus_dir / "cases.tfv").read_bytes() == (
        CORPUS_DIR / "cases.tfv"
    ).read_bytes()
    assert b"\r" not in (corpus_dir / "corpus.json").read_bytes()
    _root, mini = mini_corpus(tmp_path)
    assert b"\r" not in (mini / "cases.tfv").read_bytes()


def test_a_crlf_tfv_is_refused_not_tolerated(tmp_path) -> None:
    root, corpus_dir = copy_corpus(tmp_path)
    tfv = corpus_dir / "cases.tfv"
    tfv.write_bytes(tfv.read_bytes().replace(b"\n", b"\r\n"))
    with pytest.raises(CorpusError):
        load_corpus(root, corpus_dir)
