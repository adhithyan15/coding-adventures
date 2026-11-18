"""Slice S10c-1: ``derive`` and ``verify-v2`` on a synthetic ``ir.fixed`` corpus (VER04r 10.3, 13).

A document that needs ``ir.fixed`` now emits a corpus: the suites are evaluated and written, and
``verify-v2`` re-derives and compares every row.  Only the *fault evaluation* is gated: the control of an
``ir.fixed`` fault records ``deferred`` witnesses under the closed gate and chunk witnesses under the shipped one (open since the atomic flip, S10c, D29).  A mutated corpus (a
witness where the gate says ``deferred``, an altered fault expression, a wrong ``requires``, a wrong
shift-count descriptor) fails.
"""

from __future__ import annotations

import json
import pathlib

import pytest
from conftest import write_lf

from coding_adventures_warrant_oracle.cli import run
from coding_adventures_warrant_oracle.corpus_v2 import canonical, derive
from coding_adventures_warrant_oracle.verify_v2 import verify_v2

GEN = {"name": "test", "version": "0"}

DOC = b"""warrant v v1
  op rot(a: U8, n: Nat) -> U8
  op flipme(a: U16) -> U16
  op be(a: U32) -> Bytes<4>
  claim c_rot:
    forall a: U8, n: Nat. rot(a, n) == rotl(a, n)
  claim c_flip:
    forall a: U16. flipme(a) == a
  claim c_be:
    forall a: U32. be(a) == be_bytes(a)
  example e_rot:
    rot(129u8, 1) == 3u8
  example e_be:
    be(16909060u32) == b"01020304"
"""
PROFILE = b"chunk_size 1024\nfault flipme xor-lsb\n"


def write_corpus(tmp_path: pathlib.Path, source: bytes = DOC, profile: bytes = PROFILE):
    out = derive(source, profile, spec_ref="v.warrant", generator=GEN)
    root = tmp_path / "corpus"
    (root / "chunks").mkdir(parents=True)
    (root / "corpus.json").write_bytes(canonical(out.corpus))
    (root / "cases.tfv").write_bytes(out.tfv)
    for name, text in out.chunk_files.items():
        (root / "chunks" / name).write_bytes(text)
    warrant, prof = tmp_path / "v.warrant", tmp_path / "corpus.profile"
    warrant.write_bytes(source)
    write_lf(prof, profile.decode())
    return root, warrant, prof, out


def verify(tmp_path, root, warrant, prof):
    return verify_v2(root, warrant, prof, repo_root=tmp_path)[0]


def test_an_ir_fixed_document_derives_a_corpus_with_computed_requires(tmp_path):
    _root, _w, _p, out = write_corpus(tmp_path)
    corpus = out.corpus
    assert corpus["levels"] == ["ir.core", "ir.fixed"]
    rows = {s["claim"]: s for s in corpus["suites"]}
    assert rows["c_rot"]["requires"] == ["ir.core", "ir.fixed"]
    assert rows["c_rot"]["binders"][1][1]["domain"] == {"d": "nat_shift", "size": 73}
    assert rows["c_flip"]["binders"][0][1]["domain"] == {"d": "fx_full", "t": "U16", "size": 65536}
    assert rows["c_be"]["binders"][0][1]["domain"] == {"d": "fx_edge", "t": "U32", "size": 313}
    assert rows["c_rot"]["stream"] == "warrant-stream-v1"  # U8, Nat, U8: the v1 type set
    assert rows["c_be"]["stream"] == "warrant-stream-v2"  # a U32 binder: the first real use of v2
    assert [e["id"].rsplit("/", 1)[1] for e in corpus["examples"]] == ["e_rot", "e_be"]
    # the closed gate (explicit): the ir.fixed fault is data and its control says deferred
    closed = derive(DOC, PROFILE, spec_ref="v.warrant", generator=GEN, implemented=("ir.core",)).corpus
    (control,) = closed["controls"]
    assert control["requires"] == ["ir.fixed"]
    assert control["slim_witness"] == [{"claim": "warrant/v/exhaustive/c_flip", "deferred": "ir.fixed"}]
    assert corpus["faults"][0]["expr"] == ["bit_xor", ["var", "result"], ["lit", {"type": "U16", "value": "1"}]]


def test_a_closed_gate_corpus_fails_verify_v2_under_the_shipped_gate(tmp_path):
    # the `deferred` code path stays covered: a corpus derived under the closed gate says `deferred`
    # for an ir.fixed control, which the shipped (open) gate refutes
    root, warrant, prof, _ = write_corpus(tmp_path)
    closed = derive(DOC, PROFILE, spec_ref="v.warrant", generator=GEN, implemented=("ir.core",))
    assert closed.corpus["controls"][0]["slim_witness"][0].get("deferred") == "ir.fixed"
    (root / "corpus.json").write_bytes(canonical(closed.corpus))
    assert not verify(tmp_path, root, warrant, prof).ok


def test_verify_v2_accepts_a_derived_ir_fixed_corpus(tmp_path):
    root, warrant, prof, _ = write_corpus(tmp_path)
    report = verify(tmp_path, root, warrant, prof)
    assert report.ok, report.mismatches


def mutate(tmp_path, root, change) -> None:
    corpus = json.loads((root / "corpus.json").read_bytes())
    change(corpus)
    (root / "corpus.json").write_bytes(canonical(corpus))


@pytest.mark.parametrize(
    "name,change",
    [
        (
            "a stale deferred witness where the open gate says chunk",
            lambda c: c["controls"][0]["slim_witness"].__setitem__(
                0, {"claim": c["controls"][0]["slim_witness"][0]["claim"], "deferred": "ir.fixed"}
            ),
        ),
        (
            "a deferred level the gate implements",
            lambda c: c["controls"][0]["slim_witness"].__setitem__(
                0, {"claim": c["controls"][0]["slim_witness"][0]["claim"], "deferred": "ir.core"}
            ),
        ),
        ("an altered fault expression", lambda c: c["faults"][0].update({"expr": ["not", ["var", "result"]]})),
        ("a wrong suite requires", lambda c: c["suites"][0].update({"requires": ["ir.core"]})),
        (
            "a wrong shift-count descriptor",
            lambda c: c["suites"][0]["binders"][1][1].update({"domain": {"d": "nat_0_64", "size": 65}}),
        ),
        ("a wrong signed edge descriptor size", lambda c: c["suites"][2]["binders"][0][1]["domain"].update({"size": 377})),
        ("a wrong levels list", lambda c: c.update({"levels": ["ir.core"]})),
        ("a wrong digest", lambda c: c["suites"][1].update({"inputs_sha256": "0" * 64})),
    ],
)
def test_a_mutated_ir_fixed_corpus_fails_verify_v2(tmp_path, name, change):
    root, warrant, prof, _ = write_corpus(tmp_path)
    mutate(tmp_path, root, change)
    report = verify(tmp_path, root, warrant, prof)
    assert not report.ok, name


def test_the_open_gate_changes_only_the_control_and_the_slim_set(tmp_path):
    shipped = derive(DOC, PROFILE, spec_ref="v.warrant", generator=GEN, implemented=("ir.core",))
    opened = derive(DOC, PROFILE, spec_ref="v.warrant", generator=GEN, implemented=("ir.core", "ir.fixed", "ir.tuple"))
    default = derive(DOC, PROFILE, spec_ref="v.warrant", generator=GEN)
    assert default.corpus == opened.corpus  # the shipped gate is the open one
    assert shipped.corpus["controls"][0]["slim_witness"] != opened.corpus["controls"][0]["slim_witness"]
    assert opened.corpus["controls"][0]["slim_witness"] == [{"claim": "warrant/v/exhaustive/c_flip", "chunk": 0}]
    for key in ("suites", "faults", "examples", "ops", "models", "levels"):
        assert shipped.corpus[key] == opened.corpus[key]
    assert shipped.chunk_files == opened.chunk_files and shipped.tfv == opened.tfv


def test_the_cli_emits_an_ir_fixed_document(tmp_path, capsys):
    doc = tmp_path / "v.warrant"
    doc.write_bytes(DOC)
    out = tmp_path / "o.json"
    assert run(["emit-v2", "--spec", str(doc), "--out", str(out), "--depth", "full"]) == 0
    tree = json.loads(out.read_text())
    assert tree["levels_implemented"] == ["ir.core", "ir.fixed", "ir.tuple"]
    assert tree["summary"] == {"ok": 3, "not-live": 0, "refused": 0, "not_live_by_level": {}}
    assert tree["suites"]["c_rot"]["descriptors"][1] == {"d": "nat_shift", "size": 73}
    del capsys
