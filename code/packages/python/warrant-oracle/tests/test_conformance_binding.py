"""The ``v2-binding`` kit (VER04r section 11, slice S5c): the table, its derivation, its mutants.

``cases_binding.py`` states each case's inputs and its intent; ``bindings_v2.py`` derives the outcome.  This file
checks what S5c's acceptance names: **every mandatory binding vector of section 11 is present and derived**, the
``reason`` set is **closed and listed in the package README**, every case has the envelope of [D41] and
``requires`` ``["ir.core"]``, and a validator that forgets a rule changes the derived outcome of a case (the
mutant discipline of [D42], applied to the validator).  None of this builds the 2**24 suite of the other
categories, so the file is fast.
"""

from __future__ import annotations

import json
import pathlib
import re

import pytest
from conftest import load_make_conformance

from coding_adventures_warrant_oracle import bindings_v2 as BV
from coding_adventures_warrant_oracle import conformance as C
from coding_adventures_warrant_oracle.corpus_v2 import canonical

README = pathlib.Path(__file__).resolve().parents[1] / "README.md"


@pytest.fixture(scope="module")
def mc():
    return load_make_conformance()


@pytest.fixture(scope="module")
def built(mc):
    text, row = mc.build_binding()
    return SimpleCases(text, row, json.loads(text)["cases"], mc)


class SimpleCases:
    def __init__(self, text, row, cases, mc):
        self.text, self.row, self.cases, self.mc = text, row, cases, mc
        self.by_slug = {c["id"].split("/", 1)[1]: c for c in cases}


def raw(case) -> bytes:
    return bytes.fromhex(case["binding_hex"])


def reason_of(case):
    return case["expect"].get("reason")


# -- the file ---------------------------------------------------------------------------------------------------------------


def test_the_file_is_canonical_ascii_lf_with_the_family_schema(built):
    text = built.text
    assert text.isascii() and "\r" not in text and text.endswith("}\n") and not text.endswith("\n\n")
    doc = json.loads(text)
    assert set(doc) == {"schema", "category", "cases"}
    assert doc["schema"] == "warrant-corpus-conformance-v1" and doc["category"] == "v2-binding"
    assert canonical(doc).decode() == text


def test_the_index_row_describes_the_file(built):
    import hashlib

    assert built.row == {
        "category": "v2-binding",
        "path": "v2-binding.json",
        "cases": len(built.cases),
        "sha256": hashlib.sha256(built.text.encode()).hexdigest(),
    }


def test_the_build_is_deterministic(built, mc):
    assert mc.build_binding() == (built.text, built.row)


def test_ids_are_unique_prefixed_slugged_and_in_table_order(built, mc):
    import cases_binding

    assert [c["id"] for c in built.cases] == [f"v2-binding/{r.slug}" for r in cases_binding.ROWS]
    for case in built.cases:
        assert re.fullmatch(r"v2-binding/[a-z0-9]+(-[a-z0-9]+)*", case["id"]), case["id"]
        assert case["summary"].isascii() and "\n" not in case["summary"] and 0 < len(case["summary"]) <= 240
    assert len({c["id"] for c in built.cases}) == len(built.cases)


def test_every_case_has_the_envelope_and_requires_ir_core(built):
    for case in built.cases:
        assert set(case) == {"id", "summary", "requires", "binding_hex", "corpus", "host", "expect"}, case["id"]
        assert case["requires"] == ["ir.core"], case["id"]
        assert set(case["corpus"]) == {"types", "ops"}
        assert re.fullmatch(r"(?:[0-9a-f]{2})*", case["binding_hex"])


def test_the_outcomes_have_the_shape_of_section_11(built):
    for case in built.cases:
        expect = case["expect"]
        if expect["outcome"] == "accept":
            assert set(expect) == {"outcome", "normalized"}
            assert {"lane", "package", "module", "import", "source", "seed", "depth", "levels", "values", "types", "ops"} == set(
                expect["normalized"]
            )
        else:
            assert expect["outcome"] == "reject"
            assert set(expect) == {"outcome", "error", "reason"}
            assert expect["error"] == "BindingInvalid" and expect["reason"] in BV.REASONS
    assert C.ERROR_CODES["v2-binding"] == {"BindingInvalid"}


def test_each_case_is_rederived_to_exactly_what_the_file_holds(built):
    for case in built.cases:
        fields = {k: case[k] for k in ("binding_hex", "corpus", "host")}
        assert BV.derive_binding(fields) == case["expect"], case["id"]
        found = BV.violations(raw(case), case["corpus"], case["host"])
        assert len(found) == (0 if case["expect"]["outcome"] == "accept" else 1), case["id"]


# -- the closed reason set ----------------------------------------------------------------------------------------------------


def test_every_reason_of_the_closed_set_has_a_case_and_no_case_uses_another(built):
    used = {reason_of(c) for c in built.cases if c["expect"]["outcome"] == "reject"}
    assert used == set(BV.REASONS)


def test_the_closed_set_is_listed_in_the_package_readme():
    text = README.read_text(encoding="utf-8")
    section = text[text.index("The `v2-binding` reason set") :]
    listed = re.findall(r"^\| `([a-z0-9-]+)` \|", section, re.M)
    assert listed == list(BV.REASONS)


def test_the_module_docstring_table_lists_the_same_set():
    doc = BV.__doc__
    listed = re.findall(r"^``([a-z0-9-]+)``\s", doc, re.M)
    assert [r for r in listed if r in BV.REASONS] == list(BV.REASONS)


# -- the mandatory vectors of section 11 ---------------------------------------------------------------------------------------


def test_two_cases_of_one_op_sharing_a_discriminator_is_binding_invalid(built):
    case = built.by_slug["discriminator-shared-by-two-cases"]
    assert case["expect"] == {"outcome": "reject", "error": "BindingInvalid", "reason": "discriminator-shared"}
    assert b"errors=BAD:ValueError,WORSE:ValueError" in raw(case)
    assert len(case["corpus"]["types"]["Pick"]["cases"]) == 2


def test_err_any_on_a_two_case_error_type_is_binding_invalid(built):
    case = built.by_slug["err-any-two-cases"]
    assert reason_of(case) == "err-any-multi"
    assert b"err_any=BAD" in raw(case) and len(case["corpus"]["types"]["Pick"]["cases"]) == 2


def test_cr_a_bom_and_invalid_utf8_are_cases(built):
    assert reason_of(built.by_slug["cr-in-a-line-ending"]) == "cr" and b"\r\n" in raw(built.by_slug["cr-in-a-line-ending"])
    assert reason_of(built.by_slug["bom-at-the-start"]) == "bom" and raw(built.by_slug["bom-at-the-start"])[:3] == b"\xef\xbb\xbf"
    for slug in ("invalid-utf8-lone-continuation", "invalid-utf8-overlong-slash", "invalid-utf8-surrogate", "invalid-utf8-truncated"):
        data = raw(built.by_slug[slug])
        assert reason_of(built.by_slug[slug]) == "invalid-utf8"
        with pytest.raises(UnicodeDecodeError):
            data.decode("utf-8")


def test_the_directive_level_aliasing_vectors_are_present(built):
    """Aliasing (``v2-binding`` plus a runner fixture): the binding states a freshness mode and an inout form."""
    assert reason_of(built.by_slug["fresh-immutable-on-a-mutable-bytes"]) == "fresh"
    assert built.by_slug["accept-fresh-immutable-on-bytes"]["expect"]["normalized"]["ops"]["select_bytes"]["fresh"] == "immutable"
    forms = {s: reason_of(c) for s, c in built.by_slug.items() if s.startswith("inout-form-")}
    assert forms and set(forms.values()) == {"inout-form"}
    assert {s for s in built.by_slug if s.startswith("inout-param-")}
    assert built.by_slug["accept-inout-forms"]["expect"]["normalized"]["ops"]["zeroize"]["inout"] == {"buf": "mutable"}


def test_the_size_cap_has_both_sides_of_its_boundary(built):
    assert len(raw(built.by_slug["accept-size-cap-boundary"])) == 65536
    assert len(raw(built.by_slug["too-large-by-one-byte"])) == 65537


def test_every_level_name_including_the_reserved_ones_is_known(built):
    case = built.by_slug["accept-every-level-name"]
    assert case["expect"]["normalized"]["levels"] == sorted(BV.LEVEL_NEEDS)


def test_the_lattice_cases_cover_data_list_and_fixed(built):
    assert reason_of(built.by_slug["level-implication-data-without-tuple"]) == "level-implication"
    assert reason_of(built.by_slug["level-implication-list-without-data"]) == "level-implication"
    assert built.by_slug["accept-levels-fixed-and-tuple-are-siblings"]["expect"]["outcome"] == "accept"


def test_the_corpus_and_host_of_a_case_are_self_contained(built):
    for case in built.cases:
        BV.read_corpus(case["corpus"])
        BV.read_host(case["host"])


# -- make_conformance.py -----------------------------------------------------------------------------------------------------


@pytest.fixture()
def fast(mc, monkeypatch):
    """``build()`` answers with no six-category files, so these tests never derive the 2**24 suite."""
    monkeypatch.setattr(mc, "build", lambda: ({}, []))
    return mc


def index_bytes(rows):
    v1 = [{"category": "json", "path": "json.json", "cases": 1, "sha256": "0" * 64}]
    return canonical({"schema": "x", "total_cases": 1, "files": v1 + rows})


def test_write_puts_the_binding_file_beside_the_six_and_check_accepts_it(fast, tmp_path, built):
    assert fast.main(["--write", "--fixture", str(tmp_path)]) == 0
    assert (tmp_path / "v2-binding.json").read_bytes() == built.text.encode()
    assert fast.main(["--check", "--fixture", str(tmp_path)]) == 0


def test_the_binding_file_is_pending_until_it_or_its_index_row_exists(fast, tmp_path, capsys, built):
    assert fast.main(["--check", "--fixture", str(tmp_path)]) == 0  # no file, no index
    assert "not committed yet" in capsys.readouterr().err
    (tmp_path / "index.json").write_bytes(index_bytes([]))
    assert fast.main(["--check", "--fixture", str(tmp_path)]) == 0  # no file, an index without a row
    (tmp_path / "index.json").write_bytes(index_bytes([built.row]))
    assert fast.main(["--check", "--fixture", str(tmp_path)]) == 1  # a row without the file
    assert "v2-binding.json differs" in capsys.readouterr().err
    (tmp_path / "v2-binding.json").write_bytes(built.text.encode())
    assert fast.main(["--check", "--fixture", str(tmp_path)]) == 0
    (tmp_path / "index.json").write_bytes(index_bytes([]))
    assert fast.main(["--check", "--fixture", str(tmp_path)]) == 1  # the file without an index row
    assert "index.json" in capsys.readouterr().err


def test_a_stale_binding_file_is_exit_1_and_so_is_an_unreadable_index(fast, tmp_path, built, capsys):
    (tmp_path / "v2-binding.json").write_bytes(built.text.encode().replace(b"accept-ct-compare", b"accept-ct-compare2", 1))
    assert fast.main(["--check", "--fixture", str(tmp_path)]) == 1
    assert "v2-binding.json differs" in capsys.readouterr().err
    (tmp_path / "v2-binding.json").unlink()
    (tmp_path / "index.json").write_bytes(b"not json")
    assert fast.main(["--check", "--fixture", str(tmp_path)]) == 1  # as for the six: an unreadable index is stale


def test_a_binding_table_that_disagrees_with_the_validator_is_exit_2_naming_the_case(mc, monkeypatch, tmp_path, capsys):
    import cases_binding

    rows = list(cases_binding.ROWS)
    bad = C.Row(rows[0].slug, rows[0].summary, rows[0].fields, {"outcome": "reject"})
    monkeypatch.setattr(cases_binding, "ROWS", [bad, *rows[1:]])
    monkeypatch.setattr(mc, "build", lambda: ({}, []))
    assert mc.main(["--check", "--fixture", str(tmp_path)]) == 2
    assert f"v2-binding/{rows[0].slug}" in capsys.readouterr().err


# -- the mutants: a validator that forgets a rule must change the derived outcome of a case -----------------------------------


def derived(rows):
    out = {}
    for row in rows:
        try:
            out[row.slug] = BV.derive_binding(row.fields)
        except C.KitError as exc:  # a mutant that breaks the "exactly one violation" discipline is killed by it
            out[row.slug] = {"kit-error": str(exc)}
    return out


@pytest.fixture(scope="module")
def baseline():
    import cases_binding

    return derived(cases_binding.ROWS)


def changed(baseline):
    import cases_binding

    now = derived(cases_binding.ROWS)
    return sorted(s for s in now if now[s] != baseline[s])


@pytest.mark.parametrize("reason", BV.REASONS)
def test_a_validator_that_never_reports_a_reason_is_killed_by_a_case_of_that_reason(baseline, monkeypatch, reason):
    """The generic mutant: the validator forgot one rule entirely."""
    real = BV._run

    def forgetful(binding, corpus, host):
        found, normalized = real(binding, corpus, host)
        return [v for v in found if v.reason != reason], normalized

    monkeypatch.setattr(BV, "_run", forgetful)
    killed = changed(baseline)
    assert killed, f"no case notices a validator that never reports {reason}"
    import cases_binding

    by_slug = {r.slug: r for r in cases_binding.ROWS}
    assert any(by_slug[s].intent.get("reason") == reason for s in killed)


def test_a_lattice_without_data_implies_tuple_is_killed(baseline, monkeypatch):
    monkeypatch.setitem(BV.LEVEL_NEEDS, "ir.data", ())
    assert "level-implication-data-without-tuple" in changed(baseline)


def test_a_lattice_without_list_implies_data_is_killed(baseline, monkeypatch):
    monkeypatch.setitem(BV.LEVEL_NEEDS, "ir.list", ())
    assert "level-implication-list-without-data" in changed(baseline)


def test_a_lattice_that_forgets_the_reserved_levels_is_killed(baseline, monkeypatch):
    monkeypatch.delitem(BV.LEVEL_NEEDS, "ir.list")
    assert "accept-every-level-name" in changed(baseline)


def test_a_validator_that_allows_bare_classes_is_killed(baseline, monkeypatch):
    monkeypatch.setattr(BV, "BARE_CLASSES", ())
    assert "discriminator-bare-exception" in changed(baseline)


def test_a_size_cap_off_by_one_is_killed(baseline, monkeypatch):
    monkeypatch.setattr(BV, "MAX_BINDING_BYTES", 65537)
    assert "too-large-by-one-byte" in changed(baseline)
    monkeypatch.setattr(BV, "MAX_BINDING_BYTES", 65535)
    assert "accept-size-cap-boundary" in changed(baseline)


def test_a_reader_that_normalizes_crlf_is_killed(baseline, monkeypatch):
    real = BV.byte_violation
    monkeypatch.setattr(BV, "byte_violation", lambda data: real(data.replace(b"\r\n", b"\n")))
    assert "cr-in-a-line-ending" in changed(baseline)


def test_a_reader_that_skips_a_bom_is_killed(baseline, monkeypatch):
    real = BV.byte_violation
    monkeypatch.setattr(BV, "byte_violation", lambda data: real(data.removeprefix(b"\xef\xbb\xbf")))
    assert "bom-at-the-start" in changed(baseline)


def test_a_validator_that_ignores_what_an_inout_form_fits_is_killed(baseline, monkeypatch):
    monkeypatch.setattr(BV, "_form_fits", lambda form, ty: True)
    assert {"inout-form-cell-for-bytes", "inout-form-mutable-for-an-integer"} <= set(changed(baseline))


def test_a_validator_that_knows_one_inout_form_too_few_is_killed(baseline, monkeypatch):
    monkeypatch.setattr(BV, "INOUT_FORMS", ("mutable", "cell", "returned"))
    assert "accept-inout-growable-and-returned" in changed(baseline)


def test_a_validator_that_lets_values_name_anything_is_killed(baseline, monkeypatch):
    monkeypatch.setattr(BV, "VALUE_NAMES", (*BV.VALUE_NAMES, "Float", "bool"))
    assert "values-name-not-a-warrant-value" in changed(baseline)


def test_a_validator_whose_seed_has_no_upper_bound_is_killed(baseline, monkeypatch):
    monkeypatch.setattr(BV, "seed_value", lambda text: 0)
    assert "bad-value-seed-2-to-the-64" in changed(baseline)
