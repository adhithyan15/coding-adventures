"""``verify-v2`` (VER04r section 10.3, slice S4 part 2): the passing fixture and seeded-wrong corpora.

The corpus under test is ``fixtures/synth-v2/corpus`` -- produced by the Rust generator
from ``synth.warrant`` and ``corpus.profile`` (see that directory's README).  The oracle
derives its own copy from the two input files; every test below corrupts the on-disk copy
one way and asserts that the verifier names the right group of check.
"""

from __future__ import annotations

import copy
import json
import os
import pathlib
import shutil

import pytest

from coding_adventures_warrant_oracle.cli import run
from coding_adventures_warrant_oracle.corpus_v2 import canonical
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.verify_v2 import verify_v2

HERE = pathlib.Path(__file__).resolve().parent / "fixtures" / "synth-v2"
WARRANT = HERE / "synth.warrant"
PROFILE = HERE / "corpus.profile"
CORPUS = HERE / "corpus"


CACHE: dict = {}  # one derivation per (spec, profile): the oracle's own enumeration takes seconds


def check(root, warrant, profile, bindings=None):
    return verify_v2(root, warrant, profile, bindings, cache=CACHE)


def fresh(tmp_path: pathlib.Path) -> pathlib.Path:
    dest = tmp_path / "corpus"
    shutil.copytree(CORPUS, dest)
    return dest


def load(root: pathlib.Path) -> dict:
    return json.loads((root / "corpus.json").read_text())


def save(root: pathlib.Path, corpus: dict) -> None:
    (root / "corpus.json").write_bytes(canonical(corpus))


def codes(root: pathlib.Path, **kw) -> set[str]:
    report, _ = check(root, kw.get("warrant", WARRANT), kw.get("profile", PROFILE), kw.get("bindings"))
    return {m["code"] for m in report.mismatches}


def suite(corpus: dict, claim: str) -> dict:
    return next(s for s in corpus["suites"] if s["claim"] == claim)


def control(corpus: dict, op: str, fault: str) -> dict:
    return next(c for c in corpus["controls"] if c["op"] == op and c["fault"] == fault)


# -- the passing run -----------------------------------------------------------------------


@pytest.fixture(scope="module")
def good():
    report, derived = check(CORPUS, WARRANT, PROFILE)
    return report, derived


def test_the_generator_corpus_passes(good):
    report, derived = good
    assert report.ok, report.mismatches
    assert sum(s["cases"] for s in report.suites.values()) == 524_651
    # the derived corpus is exactly the committed bytes (key order and all)
    assert (CORPUS / "corpus.json").read_bytes() == canonical(derived.corpus)
    for name, text in derived.chunk_files.items():
        assert (CORPUS / "chunks" / name).read_bytes() == text
    assert (CORPUS / "cases.tfv").read_bytes() == derived.tfv


def test_the_derived_corpus_has_every_feature_the_fixture_exists_for(good):
    corpus = good[1].corpus
    assert len(suite(corpus, "add2_model")["slim_chunks"]) > 14
    assert 27 in suite(corpus, "add2_model")["slim_chunks"]  # a witness outside the stride
    assert 40 in suite(corpus, "add2_commutes")["slim_chunks"]  # a slim_chunk line
    miss = control(corpus, "pick", "missing-error")
    assert miss["blind_claims"] == ["warrant/synth/exhaustive/pick_equal_length"]
    assert miss["required_claims"] == ["warrant/synth/exhaustive/pick_mismatch"]
    off = control(corpus, "add2", "off-by-one")
    assert off["slim_witness"][0] == {"claim": "warrant/synth/exhaustive/add2_model", "chunk": 27}
    assert off["slim_witness"][1] == {
        "claim": "warrant/synth/exhaustive/add2_commutes",
        "unproven": True,
    }
    deferred = control(corpus, "bump", "xor-lsb")
    assert deferred["requires"] == ["ir.fixed"]
    assert deferred["slim_witness"] == [
        {"claim": "warrant/synth/exhaustive/bump_is_inc", "deferred": "ir.fixed"}
    ]
    assert corpus["accepted_blind_faults"] == [] and corpus["levels"] == ["ir.core", "ir.fixed"]


def test_cli_verify_v2_exit_codes(tmp_path, capsys):
    argv = ["verify-v2", str(CORPUS), "--warrant", str(WARRANT), "--profile", str(PROFILE)]
    assert run(argv) == 0
    assert "verify-v2: ok: 524651 cases across 8 suites" in capsys.readouterr().out
    bad = fresh(tmp_path)
    (bad / "cases.tfv").write_bytes(b"tfv 1\n")
    assert run(["verify-v2", str(bad), *argv[2:]]) == 1
    assert "MISMATCH" in capsys.readouterr().err
    assert run(["verify-v2", str(tmp_path / "nope"), *argv[2:]]) == 2
    assert "warrant-oracle:" in capsys.readouterr().err


# -- seeded-wrong corpora (one per kind of lie) ------------------------------------------------


def test_a_flipped_byte_in_a_chunk_line_fails(tmp_path):
    root = fresh(tmp_path)
    path = root / "chunks" / "cmp_agrees.txt"
    text = bytearray(path.read_bytes())
    i = text.index(b"\n") + 20  # inside the digest on the second line
    text[i] = ord("f") if text[i] != ord("f") else ord("0")
    path.write_bytes(bytes(text))
    assert codes(root) == {"files"}


@pytest.mark.parametrize(
    ("claim", "key", "group"),
    [
        ("bump_is_inc", "expected_sha256", "enumeration"),
        ("bump_is_inc", "inputs_sha256", "enumeration"),
        ("cmp_agrees", "slim_inputs_sha256", "enumeration"),
        ("cmp_agrees", "slim_expected_sha256", "enumeration"),
        ("cmp_agrees", "v1_inputs_sha256", "enumeration"),
        ("cmp_agrees", "v1_expected_sha256", "enumeration"),
        ("cmp_agrees", "chunks_sha256", "enumeration"),
    ],
)
def test_a_wrong_digest_fails(tmp_path, claim, key, group):
    root = fresh(tmp_path)
    corpus = load(root)
    row = suite(corpus, claim)
    row[key] = "0" * 64
    save(root, corpus)
    assert codes(root) == {group}


@pytest.mark.parametrize(
    ("claim", "key", "value"),
    [
        ("cmp_agrees", "cases", 1),
        ("cmp_agrees", "discarded", 1),
        ("cmp_agrees", "expected_calls", 1),
        ("cmp_agrees", "wraps", 1),
        ("cmp_agrees", "ir_cost", 1),
        ("cmp_agrees", "product", 1),
        ("cmp_agrees", "chunks", 117),
        ("cmp_agrees", "stream", "warrant-stream-v2"),
        ("cmp_agrees", "domain_label", "full"),
        ("cmp_agrees", "demoted", ["0"]),
        ("cmp_agrees", "slim_chunks", [0, 117]),  # a slim list that thins the suite
        ("cmp_agrees", "slim_chunks", [117]),  # a slim list without the first chunk
        ("cmp_agrees", "kind", "claim"),
        ("cmp_agrees", "ops", ["pick"]),
        ("cmp_agrees", "requires", ["ir.fixed"]),
        ("cmp_agrees", "result_type", "U8"),
        ("cmp_agrees", "admit", ["lit", False]),
        ("cmp_agrees", "oracle", {"kind": "predicate"}),
        ("cmp_agrees", "extra", 1),
        ("cmp_agrees", "binders", []),
        ("cmp_agrees", "cases", True),  # type-strict: not 1, not "true"
        ("bump_is_inc", "wraps", 1.0),  # a float is not the integer
    ],
)
def test_a_wrong_suite_field_fails(tmp_path, claim, key, value):
    root = fresh(tmp_path)
    corpus = load(root)
    row = suite(corpus, claim)
    if row.get(key) == value and type(row.get(key)) is type(value):  # pragma: no cover
        pytest.skip("not a change")
    row[key] = value
    save(root, corpus)
    assert codes(root)


def test_a_missing_suite_key_and_a_dropped_conditional_key_fail(tmp_path):
    root = fresh(tmp_path)
    corpus = load(root)
    del suite(corpus, "cmp_agrees")["expected_sha256"]
    del suite(corpus, "wide_is_identity")["slim_inputs_sha256"]
    save(root, corpus)
    report, _ = check(root, WARRANT, PROFILE)
    assert {m["where"] for m in report.mismatches} >= {"suites[3:cmp_agrees]", "suites[7:wide_is_identity]"}


def test_a_blind_claim_moved_into_required_claims_fails(tmp_path):
    root = fresh(tmp_path)
    corpus = load(root)
    ctl = control(corpus, "pick", "missing-error")
    ctl["required_claims"] = ctl["blind_claims"] + ctl["required_claims"]
    ctl["blind_claims"] = []
    save(root, corpus)
    assert codes(root) == {"controls"}


def test_a_witness_pointing_at_the_wrong_chunk_fails(tmp_path):
    root = fresh(tmp_path)
    corpus = load(root)
    control(corpus, "add2", "off-by-one")["slim_witness"][0]["chunk"] = 26
    save(root, corpus)
    assert "controls" in codes(root)


def test_a_witness_chunk_missing_from_the_slim_set_fails(tmp_path):
    root = fresh(tmp_path)
    corpus = load(root)
    row = suite(corpus, "add2_model")
    row["slim_chunks"].remove(27)
    save(root, corpus)
    assert codes(root) == {"enumeration"}


def test_a_deferred_control_made_live_fails(tmp_path):
    root = fresh(tmp_path)
    corpus = load(root)
    ctl = control(corpus, "bump", "xor-lsb")
    ctl["requires"] = ["ir.core"]
    ctl["slim_witness"] = [{"claim": ctl["required_claims"][0], "chunk": 0}]
    save(root, corpus)
    assert codes(root) == {"controls"}


def test_a_live_control_made_deferred_fails(tmp_path):
    root = fresh(tmp_path)
    corpus = load(root)
    ctl = control(corpus, "cmp", "invert")
    ctl["slim_witness"] = [{"claim": ctl["required_claims"][0], "deferred": "ir.core"}]
    save(root, corpus)
    assert codes(root) == {"controls"}


def test_a_removed_control_row_fails(tmp_path):
    root = fresh(tmp_path)
    corpus = load(root)
    corpus["controls"].pop(0)
    save(root, corpus)
    assert codes(root) == {"controls"}


def test_a_reordered_control_list_fails(tmp_path):
    root = fresh(tmp_path)
    corpus = load(root)
    corpus["controls"].reverse()
    save(root, corpus)
    assert codes(root) == {"controls"}


@pytest.mark.parametrize(
    "mutate",
    [
        lambda f: f.update(kind="replace"),
        lambda f: f.update(requires=["ir.fixed"]),
        lambda f: f.update(op="bump"),
        lambda f: f.update(expr=["var", "result"]),
        lambda f: f.update(extra=1),
    ],
)
def test_a_changed_fault_row_fails(tmp_path, mutate):
    root = fresh(tmp_path)
    corpus = load(root)
    mutate(next(f for f in corpus["faults"] if f["op"] == "cmp" and f["fault"] == "invert"))
    save(root, corpus)
    assert "faults" in codes(root)


def test_a_changed_pre_fault_rewrite_fails(tmp_path):
    root = fresh(tmp_path)
    corpus = load(root)
    fault = next(f for f in corpus["faults"] if f["fault"] == "ignore-last-byte")
    fault["rewrites"][0]["expr"][2] = ["var", "b"]
    save(root, corpus)
    assert codes(root) == {"faults"}


def test_a_changed_opaque_ir_fixed_expression_fails(tmp_path):
    root = fresh(tmp_path)
    corpus = load(root)
    fault = next(f for f in corpus["faults"] if f["fault"] == "xor-lsb" and f["op"] == "bump")
    assert fault["expr"][0] == "bit_xor"
    fault["expr"][0] = "bit_and"
    save(root, corpus)
    assert codes(root) == {"faults"}


def test_accepted_blind_faults_must_stay_empty(tmp_path):
    root = fresh(tmp_path)
    corpus = load(root)
    corpus["accepted_blind_faults"] = ["warrant/synth/negctl/exhaustive/pick/swap-choice"]
    save(root, corpus)
    assert codes(root) == {"controls"}


def test_a_reordered_key_fails_the_canonical_form(tmp_path):
    root = fresh(tmp_path)
    corpus = load(root)
    reordered = {k: corpus[k] for k in sorted(corpus, reverse=True)}
    (root / "corpus.json").write_text(json.dumps(reordered, indent=2) + "\n")
    assert codes(root) == {"bytes"}


@pytest.mark.parametrize(
    "mangle",
    [
        lambda b: b.replace(b"\n", b"\r\n"),  # CRLF
        lambda b: b.rstrip(b"\n"),  # no final LF
        lambda b: json.dumps(json.loads(b)).encode() + b"\n",  # compact
        lambda b: json.dumps(json.loads(b), sort_keys=True, indent=4).encode() + b"\n",
    ],
)
def test_a_non_canonical_rendering_fails(tmp_path, mangle):
    root = fresh(tmp_path)
    path = root / "corpus.json"
    path.write_bytes(mangle(path.read_bytes()))
    assert codes(root) == {"bytes"}


def test_a_cases_tfv_change_fails(tmp_path):
    root = fresh(tmp_path)
    path = root / "cases.tfv"
    path.write_bytes(path.read_bytes().replace(b"Kexpect T", b"Kexpect F", 1))
    assert codes(root) == {"lowering"}


def test_an_extra_chunk_file_fails(tmp_path):
    root = fresh(tmp_path)
    (root / "chunks" / "extra.txt").write_bytes(b"")
    assert codes(root) == {"files"}


def test_a_missing_chunk_file_fails(tmp_path):
    root = fresh(tmp_path)
    (root / "chunks" / "cmp_agrees.txt").unlink()
    assert "files" in codes(root)


def test_a_missing_chunks_directory_fails(tmp_path):
    root = fresh(tmp_path)
    shutil.rmtree(root / "chunks")
    assert "files" in codes(root)


def test_a_missing_cases_tfv_fails(tmp_path):
    root = fresh(tmp_path)
    (root / "cases.tfv").unlink()
    assert "files" in codes(root)


@pytest.mark.skipif(not hasattr(os, "symlink") or os.name == "nt", reason="needs symlinks")
def test_symbolic_links_are_refused(tmp_path):
    root = fresh(tmp_path)
    target = root / "chunks" / "cmp_agrees.txt"
    real = tmp_path / "real.txt"
    real.write_bytes(target.read_bytes())
    target.unlink()
    target.symlink_to(real)
    assert "files" in codes(root)
    shutil.rmtree(root / "chunks")
    (tmp_path / "elsewhere").mkdir()
    shutil.copytree(CORPUS / "chunks", tmp_path / "elsewhere" / "chunks")
    (root / "chunks").symlink_to(tmp_path / "elsewhere" / "chunks")
    assert "files" in codes(root)
    link = tmp_path / "link"
    link.symlink_to(CORPUS)
    with pytest.raises(OracleError):
        check(link, WARRANT, PROFILE)
    with pytest.raises(OracleError):
        check(CORPUS, WARRANT, link)


def test_a_non_directory_fixture_and_oversize_inputs_are_refused(tmp_path):
    with pytest.raises(OracleError):
        check(WARRANT, WARRANT, PROFILE)
    with pytest.raises(OracleError):
        check(tmp_path / "missing", WARRANT, PROFILE)
    root = fresh(tmp_path)
    (root / "corpus.json").write_bytes(b" " * ((16 << 20) + 1))
    with pytest.raises(OracleError) as err:
        check(root, WARRANT, PROFILE)
    assert err.value.code == "limit"
    (root / "corpus.json").write_bytes(b"[]")
    with pytest.raises(OracleError):
        check(root, WARRANT, PROFILE)
    (root / "corpus.json").write_bytes(b'{"a": 1, "a": 2}')
    with pytest.raises(OracleError):
        check(root, WARRANT, PROFILE)
    (root / "corpus.json").write_bytes(b"[NaN]")
    with pytest.raises(OracleError):
        check(root, WARRANT, PROFILE)
    os.mkdir(root / "dir.json")
    with pytest.raises(OracleError):
        from coding_adventures_warrant_oracle.verify_v2 import read_regular

        read_regular(root / "dir.json", 10)


@pytest.mark.parametrize(
    "mutate",
    [
        lambda c: c.update(spec_sha256="0" * 64),
        lambda c: c.update(profile_sha256="0" * 64),
        lambda c: c.update(module="other"),
        lambda c: c.update(document="v0"),
        lambda c: c.update(profile="warrant-corpus-v1"),
        lambda c: c.update(schema_version=1),
        lambda c: c.update(budget=1),
        lambda c: c.update(chunk_size=2048),
        lambda c: c.update(max_ir_cost=4000),
        lambda c: c.update(levels=["ir.core"]),
        lambda c: c.update(assumes=["x"]),
        lambda c: c.update(fault_models={"m": {}}),
        lambda c: c.update(model_order=list(reversed(c["model_order"]))),
        lambda c: c.update(unknown_key=1),
        lambda c: c.pop("assumes"),
        lambda c: c.update(types={"E": {"kind": "error", "cases": ["WORSE", "BAD"]}}),
        lambda c: c["models"]["inc"]["body"].__setitem__(0, "sub"),
        lambda c: c["models"].pop("same"),
        lambda c: c["ops"]["wide"].pop("admit"),
        lambda c: c["ops"]["pick"].pop("fresh"),
        lambda c: c["ops"]["cmp_fixed"].pop("width_params"),
        lambda c: c["examples"].pop(),
        lambda c: c["examples"][0].update(expected="1"),
        lambda c: c["examples"][4].pop("width"),
        lambda c: c.update(generator={"name": "x"}),
        lambda c: c.update(spec="elsewhere/synth.warrant"),
        lambda c: c.update(spec="/abs/synth.warrant"),
    ],
)
def test_a_wrong_header_or_lowering_fails(tmp_path, mutate):
    root = fresh(tmp_path)
    corpus = load(root)
    mutate(corpus)
    save(root, corpus)
    assert codes(root)


def test_lowering_mismatches_are_grouped_as_lowering(tmp_path):
    root = fresh(tmp_path)
    corpus = load(root)
    corpus["models"]["inc"]["body"][0] = "sub"  # the generator that lowers a + b as a - b
    suite(corpus, "bump_is_inc")["oracle"]["expected"] = ["lit", True]
    save(root, corpus)
    assert codes(root) == {"lowering"}


def test_a_changed_binder_domain_is_enumeration_and_a_changed_binder_type_is_lowering(tmp_path):
    root = fresh(tmp_path)
    corpus = load(root)
    suite(corpus, "bump_is_inc")["binders"][0][1]["domain"]["size"] = 255
    save(root, corpus)
    assert codes(root) == {"enumeration"}
    corpus = load(root)
    suite(corpus, "bump_is_inc")["binders"][0][1]["domain"]["size"] = 256
    suite(corpus, "bump_is_inc")["binders"][0][1]["type"] = "U64"
    save(root, corpus)
    assert codes(root) == {"lowering"}
    corpus = load(root)
    suite(corpus, "bump_is_inc")["binders"][0] = "a"
    save(root, corpus)
    assert "lowering" in codes(root)
    corpus = load(root)
    suite(corpus, "bump_is_inc")["binders"] = []
    save(root, corpus)
    assert "enumeration" in codes(root)


def test_suite_set_and_shape_mismatches(tmp_path):
    root = fresh(tmp_path)
    corpus = load(root)
    corpus["suites"].pop()
    save(root, corpus)
    assert codes(root) == {"enumeration"}
    corpus["suites"] = "no"
    save(root, corpus)
    assert codes(root) == {"enumeration"}
    corpus["suites"] = [1]
    save(root, corpus)
    assert codes(root) == {"enumeration"}
    corpus = load(CORPUS)
    corpus["suites"][0] = "x"  # one row that is not an object, the others untouched
    save(root, corpus)
    assert "enumeration" in codes(root)


def test_a_diff_report_is_bounded(tmp_path):
    root = fresh(tmp_path)
    corpus = load(root)
    for s in corpus["suites"]:
        for key in ("cases", "discarded", "product", "ir_cost", "wraps"):
            s[key] = -1
    save(root, corpus)
    report, _ = check(root, WARRANT, PROFILE)
    assert 0 < len(report.mismatches) <= 8 * 5


# -- the profile and the bindings ---------------------------------------------------------------


def test_a_profile_that_is_not_the_one_the_corpus_was_made_from_fails(tmp_path):
    other = tmp_path / "other.profile"
    other.write_bytes(PROFILE.read_bytes().replace(b"slim_stride 12", b"slim_stride 11"))
    report, _ = check(CORPUS, WARRANT, other)
    got = {m["code"] for m in report.mismatches}
    assert {"header", "enumeration"} <= got  # profile_sha256 and the slim sets


def test_the_fixture_profile_must_equal_the_profile_argument(tmp_path):
    root = fresh(tmp_path)
    (root / "corpus.profile").write_bytes(PROFILE.read_bytes())
    assert codes(root) == set()
    (root / "corpus.profile").write_bytes(PROFILE.read_bytes() + b"\n")
    assert codes(root) == {"header"}
    if os.name != "nt":
        (root / "corpus.profile").unlink()
        (root / "corpus.profile").symlink_to(PROFILE)
        assert codes(root) == {"files"}


def test_the_spec_must_be_the_one_the_corpus_was_made_from(tmp_path):
    other = tmp_path / "synth.warrant"
    other.write_bytes(WARRANT.read_bytes().replace(b"b < 200u8", b"b < 201u8") + b"\n")
    got = codes(CORPUS, warrant=other)
    assert "header" in got  # spec_sha256 and the spec path


def test_bindings_must_agree_with_the_profile(tmp_path):
    good_bindings = tmp_path / "evidence.bindings"
    lines = ["# a comment", "package p", "op bump export=b faults=off-by-one,xor-lsb"]
    lines += ["op add2 export=a faults=off-by-one", "op cmp export=c faults=invert,ignore-last-byte"]
    lines += ["op cmp_fixed export=f static=N faults=invert", "op pick export=p fresh faults=missing-error,swap-choice"]
    lines += ["op wide export=w faults=xor-lsb,off-by-one"]
    good_bindings.write_text("\n".join(lines) + "\n")
    assert codes(CORPUS, bindings=good_bindings) == set()
    good_bindings.write_text("\n".join(lines[:-1] + ["op wide export=w faults=xor-lsb"]) + "\n")
    assert codes(CORPUS, bindings=good_bindings) == {"drift"}
    good_bindings.write_text("\n".join(lines + ["op extra export=e faults=invert"]) + "\n")
    assert codes(CORPUS, bindings=good_bindings) == {"drift"}
    good_bindings.write_text("\n".join(lines + [lines[2]]) + "\n")
    with pytest.raises(OracleError):
        check(CORPUS, WARRANT, PROFILE, good_bindings)
    good_bindings.write_bytes(b"\xff\xfe")
    with pytest.raises(OracleError):
        check(CORPUS, WARRANT, PROFILE, good_bindings)


def test_a_corpus_copy_is_not_mutated_by_verification(tmp_path):
    root = fresh(tmp_path)
    before = copy.deepcopy(load(root))
    check(root, WARRANT, PROFILE)
    assert load(root) == before


# -- the real adopter, when one is available -------------------------------------------------------


@pytest.mark.skipif(
    "WARRANT_CT_COMPARE_V2" not in os.environ,
    reason="set WARRANT_CT_COMPARE_V2 to a generated ct-compare v2 directory",
)
def test_ct_compare_v2_generated_by_the_rust_cli_verifies():  # pragma: no cover
    from conftest import CODE

    corpus = pathlib.Path(os.environ["WARRANT_CT_COMPARE_V2"])
    profile = CODE / "packages/rust/warrant-corpus-gen/tests/fixtures/ct-compare.corpus.profile"
    report, _ = check(
        corpus,
        CODE / "specs" / "ct-compare.warrant",
        profile,
        CODE / "packages/rust/ct-compare/evidence.bindings",
    )
    assert report.ok, report.mismatches
