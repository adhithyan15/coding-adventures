"""The reference binding validator of VER04r section 7 (slice S5c, [D43]), tested directly.

``test_conformance_binding.py`` checks the *kit* the validator derives (the ``v2-binding`` table); this file
checks the validator itself: the order of its checks, the shapes it normalizes to, the schema errors of its
two inputs, and the branches the kit's corpora do not reach (a ``List`` of ``Option`` of a tuple, a record
parameter with ``args=``).  Where a test builds a binding it writes the *text*, line by line, the way a lane's
author would.
"""

from __future__ import annotations

import pytest

from coding_adventures_warrant_oracle import bindings_v2 as BV
from coding_adventures_warrant_oracle.conformance import KitError, Row

CORE = ["ir.core"]
HOST = {
    "values": {
        "Bool": ["bool"],
        "U8": ["int"],
        "U64": ["int"],
        "Nat": ["int"],
        "Bytes": ["bytes"],
        "Tuple": ["tuple"],
        "Option": ["optional"],
        "List": ["list"],
    },
    "exports": ["f", "g", "Point"],
    "discriminators": ["ValueError", "panics"],
    "immutable": ["bytes", "int", "bool", "tuple", "optional"],
}
HEAD = (
    "lane python\npackage python/x\nmodule x\nimport x\nsource src\nseed 1\ndepth slim\nlevels ir.core\n"
)
CORPUS = {
    "types": {"E": {"kind": "error", "cases": ["BAD"]}},
    "ops": {"f": {"params": [["a", "U8"]], "ret": "Bool", "requires": CORE}},
}


def lines(*body: str, head: str = HEAD) -> bytes:
    return (head + "\n".join(body) + "\n").encode()


F_LINE = "values Bool=bool U8=int\nop f export=f"


def reasons(binding: bytes, corpus=CORPUS, host=HOST) -> list[str]:
    return [v.reason for v in BV.violations(binding, corpus, host)]


def test_a_minimal_binding_normalizes():
    got = BV.validate_binding(lines(F_LINE), CORPUS, HOST)
    assert got == {
        "lane": "python",
        "package": "python/x",
        "module": "x",
        "import": "x",
        "source": "src",
        "seed": "1",
        "depth": "slim",
        "levels": ["ir.core"],
        "values": {"Bool": "bool", "U8": "int"},
        "types": {},
        "ops": {"f": {"export": "f"}},
    }


def test_a_missing_final_lf_is_accepted_and_a_blank_line_is_ignored():
    assert reasons((HEAD + "\n" + F_LINE).encode()) == []


def test_the_bytes_are_checked_in_a_fixed_order_and_alone():
    good = lines(F_LINE)
    assert reasons(b"\xef\xbb\xbf" + good.replace(b"\n", b"\r\n")) == ["bom"]  # BOM before CR
    assert reasons(b"\xff" + b"x" * 70000) == ["too-large"]  # size before UTF-8
    assert reasons(b"\xef\xbb\xbf\xff") == ["invalid-utf8"]
    assert reasons(good.replace(b"\n", b"\r\n") + b"\t") == ["cr"]  # CR before another control character


def test_line_problems_stop_the_model_checks():
    # A bad depth and a missing seed: only the line problem is reported, the model is not read.
    binding = lines(F_LINE, head=HEAD.replace("depth slim", "depth deep").replace("seed 1\n", ""))
    assert reasons(binding) == ["bad-value"]


def test_every_line_problem_is_reported_and_the_first_is_raised():
    binding = lines("bogus x", F_LINE, "lane go", "depth", head=HEAD)
    assert reasons(binding) == ["unknown-directive", "duplicate-directive", "syntax"]
    with pytest.raises(BV.BindingRefused) as info:
        BV.validate_binding(binding, CORPUS, HOST)
    assert info.value.reason == "unknown-directive"
    assert "BindingInvalid(unknown-directive)" in str(info.value) and "line 9" in info.value.detail


def test_the_model_reports_every_problem_in_the_order_of_the_checks():
    head = "lane python\npackage python/x\nmodule x\nimport x\nsource src\ndepth slim\nlevels ir.core,ir.nope\n"
    binding = lines("values Bool=wrong", "op f export=nothing", head=head)
    assert reasons(binding) == [
        "missing-directive",  # seed
        "level-unknown",
        "values-representation",
        "values-missing",  # U8
        "export-unknown",
    ]


def test_seed_values():
    assert BV.seed_value("0x1F") == 31 and BV.seed_value("1_000") == 1000 and BV.seed_value("0x1_0") == 16


def test_the_package_must_name_the_lane_and_a_lane_that_is_absent_is_not_compared():
    assert reasons(lines(F_LINE, head=HEAD.replace("package python/x", "package go/x"))) == ["bad-value"]
    assert reasons(lines(F_LINE, head=HEAD.replace("lane python\n", ""))) == ["missing-directive"]


def test_a_values_line_is_optional_when_the_corpus_needs_no_value_but_every_op_has_a_result():
    # every op returns a type, so a corpus always needs some value name: the entry cannot be omitted
    assert reasons(lines("op f export=f")) == ["values-missing", "values-missing"]  # Bool and U8


# -- the corpus fragment and the host object: schema errors are kit errors, never vectors ------------------------------------


@pytest.mark.parametrize(
    "corpus",
    [
        [],
        {"types": {}},
        {"types": [], "ops": {}},
        {"types": {"T": {"kind": "other"}}, "ops": {}},
        {"types": {"T": 5}, "ops": {}},
        {"types": {"E": {"kind": "error", "cases": []}}, "ops": {}},
        {"types": {"R": {"kind": "record", "fields": []}}, "ops": {}},
        {"types": {"V": {"kind": "variant", "ctors": []}}, "ops": {}},
        {"types": {}, "ops": {"f": {"params": []}}},
        {"types": {}, "ops": {"f": {"params": [], "ret": "Bool", "inout": ["x"]}}},
        {"types": {}, "ops": {"f": {"params": [["a", 5]], "ret": "Bool"}}},
        {"types": {}, "ops": {"f": {"params": [["a", "Ghost"]], "ret": "Bool"}}},
        {"types": {}, "ops": {"f": {"params": [["a", "Result<Bool,E>"]], "ret": "Bool"}}},
        {"types": {}, "ops": {"f": {"params": [["a", "(Bool,"]], "ret": "Bool"}}},
        {"types": {}, "ops": {"f": {"params": [["a", "Bytes<"]], "ret": "Bool"}}},
        {"types": {}, "ops": {"f": {"params": [["a", "Bool Bool"]], "ret": "Bool"}}},
        {"types": {}, "ops": {"f": {"params": [["a", "(Bool"]], "ret": "Bool"}}},
        {"types": {"R": {"kind": "record", "fields": [["x", "Ghost"]]}}, "ops": {}},
        {"types": {"V": {"kind": "variant", "ctors": [["A", ["Ghost"]], ["B", []]]}}, "ops": {}},
    ],
)
def test_a_malformed_corpus_is_a_kit_error(corpus):
    with pytest.raises(BV.BindingKitError):
        BV.read_corpus(corpus)


@pytest.mark.parametrize(
    "host",
    [
        [],
        {"values": {}, "exports": []},
        {"values": [], "exports": [], "discriminators": [], "immutable": []},
        {"values": {"Float": ["f"]}, "exports": [], "discriminators": [], "immutable": []},
        {"values": {}, "exports": ["a", "a"], "discriminators": [], "immutable": []},
        {"values": {}, "exports": [""], "discriminators": [], "immutable": []},
        {"values": {"Bool": "bool"}, "exports": [], "discriminators": [], "immutable": []},
    ],
)
def test_a_malformed_host_is_a_kit_error(host):
    with pytest.raises(BV.BindingKitError):
        BV.read_host(host)


def test_a_kit_error_is_a_kit_error():
    assert issubclass(BV.BindingKitError, KitError)


def test_signature_types_parse_to_shapes():
    types = {
        "E": {"kind": "error", "cases": ["BAD"]},
        "P": {"kind": "record", "fields": [["x", "U8"]]},
    }
    ty = BV.parse_ty("List<Option<(Bytes<N>,P)>>", types, ("N",))
    assert ty.kind == "list" and ty.args[0].kind == "option"
    tup = ty.args[0].args[0]
    assert [t.kind for t in tup.args] == ["bytes", "named"]
    assert BV.key_of(ty) == "List" and BV.key_of(tup) == "Tuple" and BV.key_of(tup.args[0]) == "Bytes"
    assert BV.key_of(tup.args[1]) is None
    res = BV.parse_ty("Result<Bytes<16>,E>", types)
    assert res.kind == "result" and BV.key_of(res) == "Bytes" and res.name == "E"
    assert BV.key_of(BV.parse_ty("U64", types)) == "U64" and BV.key_of(BV.parse_ty("Option<Bool>", types)) == "Option"


def test_a_signature_that_needs_lists_options_tuples_and_records_needs_every_name():
    corpus = {
        "types": {"P": {"kind": "record", "fields": [["x", "U8"], ["y", "List<Option<Bool>>"]]}},
        "ops": {"f": {"params": [["a", "P"]], "ret": "(P,Nat)"}},
    }
    full = (
        "values Bool=bool U8=int Nat=int List=list Option=optional Tuple=tuple\n"
        "type P host=Point fields=x,y\nop f export=f args=Point ret=tuple"
    )
    assert reasons(lines(full), corpus) == []
    assert reasons(lines(full.replace("Option=optional ", "")), corpus) == ["values-missing"]
    assert reasons(lines(full.replace("type P host=Point fields=x,y\n", "")), corpus) == ["type-missing"]


def test_args_and_ret_of_a_record_are_the_types_host_export():
    corpus = {
        "types": {"P": {"kind": "record", "fields": [["x", "U8"]]}},
        "ops": {"f": {"params": [["a", "P"]], "ret": "P"}},
    }
    ok = "values U8=int\ntype P host=Point fields=x\nop f export=f args=Point ret=Point"
    assert reasons(lines(ok), corpus) == []
    assert reasons(lines(ok.replace("args=Point", "args=g")), corpus) == ["args"]
    assert reasons(lines(ok.replace("ret=Point", "ret=g")), corpus) == ["ret"]
    # without the type line the signature is reported once as type-missing, and args are not also refused
    assert reasons(lines(ok.replace("type P host=Point fields=x\n", "")), corpus) == ["type-missing"]


def test_the_error_forms_of_a_multi_case_type_and_the_panics_list():
    corpus = {
        "types": {"E": {"kind": "error", "cases": ["A", "B"]}},
        "ops": {"g": {"params": [], "ret": "Result<Bool,E>"}},
    }

    def check(op_tail, host=HOST):
        return reasons(lines("values Bool=bool", f"op g export=g {op_tail}".strip()), corpus, host)

    assert check("errors=A:ValueError,B:panics") == ["panics-form"]
    assert check("errors=A:ValueError,B:ValueError") == ["discriminator-shared"]
    assert check("errors=A:ValueError,B:Error") == ["discriminator-bare"]
    assert check("errors=A:ValueError,B:Zed") == ["discriminator-unknown"]
    assert check("errors=A:ValueError") == ["errors-incomplete"]
    assert check("errors=A:ValueError,B:a:b", {**HOST, "discriminators": ["ValueError", "a:b"]}) == []  # first ':' splits
    assert check("err_any=A") == ["err-any-multi"]
    assert check("error=A") == ["panics-form"]
    assert check("") == ["errors-missing"]


def test_a_discriminator_may_hold_a_colon_and_an_inout_form_may_hold_one_too():
    corpus = {
        "types": {},
        "ops": {"z": {"params": [["b", "Bytes"]], "ret": "Bytes", "inout": ["b"], "post_slot": "S1"}},
    }
    host = {**HOST, "exports": ["z"]}
    body = "values Bytes=bytes\nop z export=z inout=b:"
    assert reasons(lines(body + "struct:Q"), corpus, host) == ["inout-form"]  # reserved (ir.inout.records)
    assert reasons(lines(body + "slice-of:struct:Q"), corpus, host) == ["inout-form"]
    assert reasons(lines(body + "codec:Q"), corpus, host) == ["inout-form"]
    assert reasons(lines(body + "vec-of:struct:Q"), corpus, host) == ["inout-form"]
    assert reasons(lines(body + "growable"), corpus, host) == []


def test_fresh_immutable_checks_the_representation_of_a_composite_result_by_its_key():
    corpus = {
        "types": {},
        "ops": {"f": {"params": [], "ret": "(Bytes,Bool)", "fresh": True}},
    }
    base = "values Bytes=bytes Bool=bool Tuple=tuple\nop f export=f fresh=immutable"
    assert reasons(lines(base), corpus) == []
    host = {**HOST, "immutable": ["bytes", "bool"]}  # a host tuple is mutable here
    assert reasons(lines(base), corpus, host) == ["fresh"]


def test_fresh_defaults_to_probe_in_the_normalized_form():
    corpus = {"types": {}, "ops": {"f": {"params": [], "ret": "Bytes", "fresh": True}}}
    got = BV.validate_binding(lines("values Bytes=bytes\nop f export=f"), corpus, HOST)
    assert got["ops"]["f"] == {"export": "f", "fresh": "probe"}


def test_fresh_immutable_checks_a_record_by_its_host_class_and_a_tuple_by_its_components():
    corpus = {
        "types": {"P": {"kind": "record", "fields": [["x", "U8"]]}},
        "ops": {
            "r": {"params": [], "ret": "P", "fresh": True},
            "t": {"params": [], "ret": "(Bytes,Bool)", "fresh": True},
        },
    }
    host = {**HOST, "exports": ["f", "g", "Point"], "immutable": ["bytes", "bool", "tuple", "int"]}
    head = "values Bytes=bytes Bool=bool Tuple=tuple U8=int\ntype P host=Point fields=x\n"
    assert reasons(lines(head + "op r export=f fresh=immutable\nop t export=g"), corpus, host) == ["fresh"]
    both = {**host, "immutable": [*host["immutable"], "Point"]}
    assert reasons(lines(head + "op r export=f fresh=immutable\nop t export=g fresh=immutable"), corpus, both) == []
    mutable_part = head.replace("Bytes=bytes", "Bytes=bytearray")
    host_ba = {**both, "values": {**HOST["values"], "Bytes": ["bytes", "bytearray"]}}
    assert reasons(lines(mutable_part + "op r export=f\nop t export=g fresh=immutable"), corpus, host_ba) == ["fresh"]


# -- the seed: strict grammar, bounded before int() ---------------------------------------------------------------------------


def seed_of(text: str):
    binding = HEAD.replace("seed 1", f"seed {text}") + F_LINE + "\n"
    found = reasons(binding.encode())
    return found, (BV.validate_binding(binding.encode(), CORPUS, HOST)["seed"] if not found else None)


def test_a_seed_is_bounded_by_its_significant_digits_before_it_is_converted():
    assert seed_of("0" * 4400 + "1") == ([], "1")  # one significant digit: no int() over 4,300 characters
    assert seed_of("0x" + "0" * 4400 + "1") == ([], "1")
    assert seed_of("0_0_0_7") == ([], "7")
    assert seed_of("0") == ([], "0") and seed_of("0x0") == ([], "0") and seed_of("0" * 30) == ([], "0")
    assert seed_of("18446744073709551615")[0] == [] and seed_of("0xffff_ffff_ffff_ffff")[0] == []
    for refused in ("9" * 5000, "1" + "0" * 20, "18446744073709551616", "0x1" + "0" * 16, "0x1_0000_0000_0000_0000"):
        assert seed_of(refused)[0] == ["bad-value"], refused[:30]
    assert BV.seed_value("9" * 5000) is None and BV.seed_value("0x" + "f" * 17) is None


def test_a_package_name_follows_the_repository_spelling():
    for ok in ("x", "gf256_native", "Barcode1D", "Mosaic.Flux", "a-b_c.d", "7zip"):
        binding = HEAD.replace("package python/x", f"package python/{ok}") + F_LINE + "\n"
        assert reasons(binding.encode()) == [], ok
    for refused in ("-x", ".x", "_x", "a/b", "a b", ""):
        binding = HEAD.replace("package python/x", f"package python/{refused}") + F_LINE + "\n"
        assert reasons(binding.encode()) != [], refused


def test_a_corpus_that_breaks_the_grammar_of_section_2_1_is_a_kit_error():
    for ret in ("(Bytes)", "Bytes<0999>", "Bytes<4097>", "Bytes<M>"):  # one component, leading zero, too wide, no width_param
        corpus = {"types": {}, "ops": {"f": {"params": [], "ret": ret}}}
        with pytest.raises(BV.BindingKitError):
            BV.read_corpus(corpus)
    ok = {"types": {}, "ops": {"f": {"params": [["a", "Bytes<M>"], ["b", "Bytes<4096>"]], "ret": "Bytes<0>", "width_params": ["M"]}}}
    assert BV.read_corpus(ok)
    both = {"types": {}, "ops": {"f": {"params": [["a", "Bytes"]], "ret": "Bytes", "inout": ["a"], "fresh": True}}}
    with pytest.raises(BV.BindingKitError, match="fresh and inout"):
        BV.read_corpus(both)


# -- the kit helpers --------------------------------------------------------------------------------------------------------


def test_derive_binding_outcomes_and_kit_errors():
    good = {"binding_hex": lines(F_LINE).hex(), "corpus": CORPUS, "host": HOST}
    assert BV.derive_binding(good)["outcome"] == "accept"
    bad = {**good, "binding_hex": lines("op f export=nowhere").hex()}  # export-unknown and values-missing x2
    with pytest.raises(BV.BindingKitError, match="exactly one violation"):
        BV.derive_binding(bad)
    one = {**good, "binding_hex": lines("values Bool=bool U8=int", "op f export=nowhere").hex()}
    assert BV.derive_binding(one) == {"outcome": "reject", "error": "BindingInvalid", "reason": "export-unknown"}
    for hexed in ("0g", "ABCD", "abc", 5):
        with pytest.raises(BV.BindingKitError, match="lowercase hex"):
            BV.derive_binding({**good, "binding_hex": hexed})


def test_build_binding_names_the_case_on_a_disagreement_and_refuses_a_duplicate_id(monkeypatch):
    fields = {"binding_hex": lines(F_LINE).hex(), "corpus": CORPUS, "host": HOST}
    normalized = BV.derive_binding(fields)["normalized"]
    row = Row("one", "one case", fields, {"outcome": "accept", "normalized": normalized})
    text, index = BV.build_binding([row])
    assert index["cases"] == 1 and index["path"] == "v2-binding.json" and text.endswith("}\n")
    with pytest.raises(KitError, match=r"v2-binding/one"):
        BV.build_binding([Row("one", "one case", fields, {"outcome": "reject"})])
    # the normalized form of an accepted case is compared by equality: a missing or an extra key is seen
    for intent in ({"outcome": "accept"}, {"outcome": "accept", "normalized": {**normalized, "extra": 1}}):
        with pytest.raises(KitError, match=r"v2-binding/one.*not equal"):
            BV.build_binding([Row("one", "one case", fields, intent)])
    with pytest.raises(KitError, match="duplicate id"):
        BV.build_binding([row, row])
    bad_fields = {**fields, "binding_hex": "zz"}
    with pytest.raises(KitError, match=r"v2-binding/two"):
        BV.build_binding([Row("two", "bad hex", bad_fields, {"outcome": "accept"})])
    monkeypatch.setattr(BV, "REASONS", ("bom",))
    broken = {**fields, "binding_hex": lines("values Bool=bool U8=int", "op f export=nowhere").hex()}
    with pytest.raises(KitError, match="closed set"):
        BV.build_binding([Row("three", "not in the set", broken, {"outcome": "reject"})])


def test_the_reason_set_is_closed_and_has_no_repeats():
    assert len(set(BV.REASONS)) == len(BV.REASONS) == 39
    assert all(r == r.lower() and " " not in r for r in BV.REASONS)
