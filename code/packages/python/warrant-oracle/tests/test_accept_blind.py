"""``accept_blind``, ``--expect-generator`` and the profile reader's whitespace rule.

Owner decisions of 2026-11-17 (VER04r section 16.8): [D13] the generator name and version
can be pinned by the build, and [D15] a profile line ``accept_blind <op> <fault>`` excuses a
fault every candidate claim is blind to.  The corpus under test is ``fixtures/accept-blind-v2``
(written by the Rust generator; the generator's own tests regenerate it byte for byte).
"""

from __future__ import annotations

import json
import pathlib
import shutil

import pytest

from coding_adventures_warrant_oracle.cli import run
from coding_adventures_warrant_oracle.corpus_v2 import canonical, derive
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.profile import parse_profile
from coding_adventures_warrant_oracle.verify_v2 import parse_expected_generator, verify_v2

HERE = pathlib.Path(__file__).resolve().parent / "fixtures" / "accept-blind-v2"
WARRANT = HERE / "blind.warrant"
PROFILE = HERE / "corpus.profile"
CORPUS = HERE / "corpus"
GEN = {"name": "test", "version": "0"}
CACHE: dict = {}

EQ_BLIND = "warrant/blindacc/negctl/exhaustive/eq_fixed/ignore-last-byte"
FLAG_BLIND = "warrant/blindacc/negctl/exhaustive/flag/swap-choice"


def check(root=CORPUS, profile=PROFILE, **kw):
    return verify_v2(root, WARRANT, profile, cache=CACHE, **kw)


def codes(root, **kw):
    report, _ = check(root, **kw)
    return {m["code"] for m in report.mismatches}


def gen(profile: str, spec: bytes | None = None):
    return derive(
        spec if spec is not None else WARRANT.read_bytes(),
        profile.encode(),
        spec_ref="b.warrant",
        generator=GEN,
    )


# -- the profile reader -------------------------------------------------------------------------


def test_accept_blind_lines_parse_in_order():
    p = parse_profile(b"fault a b\nfault c d\naccept_blind c d\naccept_blind a b\n")
    assert p.accept_blind == [("c", "d"), ("a", "b")]
    assert parse_profile(b"accept_blind a b\nfault a b\n").accept_blind == [("a", "b")]
    assert parse_profile(b"").accept_blind == []


@pytest.mark.parametrize(
    "text",
    [
        b"accept_blind a b\n",  # no fault line
        b"fault a b\naccept_blind a c\n",
        b"fault a b\naccept_blind c b\n",
        b"fault a b\naccept_blind a b\naccept_blind a b\n",  # a repeat
        b"fault a b\naccept_blind a\n",
        b"fault a b\naccept_blind a b c\n",
        b"fault a b\naccept_blind  a b\n",  # an empty token
        b"fault a b\naccept_blind a b \n",
        b"fault a b\nACCEPT_BLIND a b\n",
    ],
)
def test_a_malformed_accept_blind_line_is_a_profile_error(text):
    with pytest.raises(OracleError) as err:
        parse_profile(text)
    assert err.value.code == "corpus"


@pytest.mark.parametrize("line", [b" ", b"\t", b"\x0b", b"\x0c", " ".encode(), " ".encode()])
def test_a_whitespace_only_profile_line_is_refused(line):
    # the generator's reader skipped these until CV2-S3d; both readers refuse them now
    for text in (line + b"\n", b"budget 1\n" + line + b"\nslim_stride 1\n", b"slim_stride 7\n" + line):
        with pytest.raises(OracleError):
            parse_profile(text)
    assert parse_profile(b"\n\nbudget 1").budget == 1


@pytest.mark.parametrize(
    "text",
    [
        "budget 1\t\n",
        "fault a\u00a0b c\n",
        "fault a\u00a0 b\n",
        "fault a b\u0085\n",
        "fault a b\u001c\n",
        "\ufeffbudget 1\n",
        "fault a \u00a0\n",
        "fault a\tb c\n",
        "fault \u00e9 b\n",
    ],
)
def test_profile_tokens_are_printable_ascii_only(text):
    # the same table is in the generator's profile.rs tests
    with pytest.raises(OracleError) as err:
        parse_profile(text.encode())
    assert err.value.code == "corpus"
    assert parse_profile(b"fault a-b_c.d b~!\n").faults == [("a-b_c.d", "b~!")]


# -- derivation ---------------------------------------------------------------------------------


def test_an_all_blind_fault_named_by_accept_blind_is_recorded_sorted():
    out = gen(PROFILE.read_text())
    assert out.corpus["accepted_blind_faults"] == [EQ_BLIND, FLAG_BLIND]
    rows = {c["id"]: c for c in out.corpus["controls"]}
    for ident, claim in ((EQ_BLIND, "w0"), (FLAG_BLIND, "flag_ignores_choice")):
        assert rows[ident]["required_claims"] == []
        assert rows[ident]["blind_claims"] == [f"warrant/blindacc/exhaustive/{claim}"]
        assert rows[ident]["slim_witness"] == []
    others = [c for i, c in rows.items() if i not in (EQ_BLIND, FLAG_BLIND)]
    assert len(others) == 2 and all(c["required_claims"] for c in others)
    assert len(out.corpus["faults"]) == 4
    # the order of the profile's lines does not change the corpus (only profile_sha256)
    flipped = gen(
        "fault eq_fixed invert\nfault flag invert\nfault eq_fixed ignore-last-byte\n"
        "fault flag swap-choice\naccept_blind eq_fixed ignore-last-byte\n"
        "accept_blind flag swap-choice\n"
    )
    assert flipped.corpus["accepted_blind_faults"] == out.corpus["accepted_blind_faults"]
    assert flipped.corpus["controls"] == out.corpus["controls"]


def test_without_accept_blind_an_all_blind_fault_still_fails_generation():
    with pytest.raises(OracleError) as err:
        gen("fault eq_fixed ignore-last-byte\n")
    assert err.value.code == "generation" and "blind" in err.value.message


def test_accepting_a_fault_a_claim_can_see_is_a_profile_error():
    with pytest.raises(OracleError) as err:
        gen("fault eq_fixed invert\naccept_blind eq_fixed invert\n")
    assert err.value.code == "corpus" and "not all-blind" in err.value.message
    with pytest.raises(OracleError) as err:
        gen(
            "fault eq_fixed invert\nfault eq_fixed ignore-last-byte\n"
            "accept_blind eq_fixed ignore-last-byte\naccept_blind eq_fixed invert\n"
        )
    assert err.value.code == "corpus"


def test_accept_blind_does_not_excuse_a_fault_no_claim_calls():
    cut = WARRANT.read_text().replace(
        "  claim flag_ignores_choice:\n    forall a: Bytes, c: Bool. flag(a, c) == keep(a, c)\n", ""
    )
    assert cut != WARRANT.read_text()
    with pytest.raises(OracleError) as err:
        gen("fault flag invert\naccept_blind flag invert\n", cut.encode())
    assert err.value.code == "generation" and "no claim calls" in err.value.message


# -- verify-v2 on the committed fixture and seeded-wrong variants -------------------------------


def test_the_generator_written_fixture_passes():
    report, derived = check()
    assert report.ok, report.mismatches
    assert (CORPUS / "corpus.json").read_bytes() == canonical(derived.corpus)


def _fresh(tmp_path: pathlib.Path) -> pathlib.Path:
    dest = tmp_path / "corpus"
    shutil.copytree(CORPUS, dest)
    return dest


def _edit(root: pathlib.Path, mutate) -> None:
    corpus = json.loads((root / "corpus.json").read_text())
    mutate(corpus)
    (root / "corpus.json").write_bytes(canonical(corpus))


@pytest.mark.parametrize(
    "mutate",
    [
        lambda c: c.update(accepted_blind_faults=[]),
        lambda c: c.update(accepted_blind_faults=[EQ_BLIND]),
        lambda c: c.update(accepted_blind_faults=[FLAG_BLIND, EQ_BLIND]),
        lambda c: c.update(accepted_blind_faults=[EQ_BLIND, FLAG_BLIND, EQ_BLIND]),
        lambda c: c.update(accepted_blind_faults=[EQ_BLIND, "warrant/blindacc/negctl/exhaustive/flag/invert"]),
    ],
)
def test_a_wrong_accepted_blind_faults_list_fails(tmp_path, mutate):
    root = _fresh(tmp_path)
    _edit(root, mutate)
    assert codes(root) == {"controls"}


def test_a_stale_accept_blind_line_fails_the_profile_check(tmp_path):
    # a profile that no longer excuses a fault the committed corpus accepts
    profile = tmp_path / "corpus.profile"
    profile.write_text(PROFILE.read_text().replace("accept_blind flag swap-choice\n", ""))
    with pytest.raises(OracleError) as err:
        check(profile=profile)
    assert err.value.code == "generation"
    profile.write_text(PROFILE.read_text() + "accept_blind flag invert\n")
    with pytest.raises(OracleError) as err:
        check(profile=profile)
    assert err.value.code == "corpus"


# -- the generator pin ([D13]) ------------------------------------------------------------------


def _generator() -> tuple[str, str]:
    g = json.loads((CORPUS / "corpus.json").read_text())["generator"]
    return g["name"], g["version"]


def test_expect_generator_accepts_the_recorded_pair_and_refuses_any_other():
    name, version = _generator()
    assert name == "warrant-corpus-gen"
    ok, _ = check(expect_generator=(name, version))
    assert ok.ok, ok.mismatches
    for pair in ((name, "0.0.0"), ("other", version), (version, name)):
        report, _ = check(expect_generator=pair)
        assert [(m["code"], m["where"]) for m in report.mismatches] == [("header", "generator")]


def test_expect_generator_syntax():
    assert parse_expected_generator("warrant-corpus-gen@0.5.0") == ("warrant-corpus-gen", "0.4.0")
    for bad in ("", "x", "@1", "x@", "x@y@z", "x y@1", "x@1 ", "x@é1", "a" * 65 + "@1"):
        with pytest.raises(OracleError) as err:
            parse_expected_generator(bad)
        assert err.value.code == "source"


def test_cli_expect_generator(capsys):
    name, version = _generator()
    argv = ["verify-v2", str(CORPUS), "--warrant", str(WARRANT), "--profile", str(PROFILE)]
    assert run([*argv, "--expect-generator", f"{name}@{version}"]) == 0
    assert run([*argv, "--expect-generator", f"{name}@{version}9"]) == 1
    assert "MISMATCH header generator" in capsys.readouterr().err
    assert run([*argv, "--expect-generator", "nonsense"]) == 2
    assert run([*argv, "--expect-generator", ""]) == 2  # an empty value must not disable the pin
    assert run(argv) == 0
