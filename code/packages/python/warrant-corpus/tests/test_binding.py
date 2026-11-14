"""The binding table: parsing, resolution and the subject wrapper."""

from __future__ import annotations

import pytest
from helpers import BINDING_TEXT, new_library

from coding_adventures_warrant_corpus.binding import (
    Subject,
    SubjectFailure,
    parse_binding,
    resolve_exports,
)
from coding_adventures_warrant_corpus.errors import BindingError, CorpusError
from coding_adventures_warrant_corpus.faults import FAULTS

GOOD = BINDING_TEXT.format(name="somelib").encode()


def test_parses_the_directive_file() -> None:
    binding = parse_binding(b"# comment\n\n" + GOOD)
    assert binding.lane == "python" and binding.module == "ct_compare"
    assert binding.seed == 0x5741_5252_414E_5421
    assert binding.ops["eq_fixed"].width == "implicit"
    assert binding.ops["select_bytes"].errors[0][0] == "CT_SELECT_LENGTH_MISMATCH"
    assert binding.ops["eq"].width == "none"
    assert binding.raw == b"# comment\n\n" + GOOD


def replace(old: str, new: str) -> bytes:
    text = GOOD.decode()
    assert old in text
    return text.replace(old, new).encode()


@pytest.mark.parametrize(
    "data",
    [
        b"\xff\xfe",  # not UTF-8
        b"x" * 70000,  # too large
        b"bogus directive\n",
        GOOD + b"lane python\n",  # duplicate single directive
        replace("lane python", "lane"),  # missing value
    ],
)
def test_malformed_bindings_are_refused(data: bytes) -> None:
    with pytest.raises(BindingError):
        parse_binding(data)


@pytest.mark.parametrize(
    "old,new",
    [
        ("seed 0x5741_5252_414e_5421", "seed banana"),
        ("seed 0x5741_5252_414e_5421", "seed 0x1_0000_0000_0000_0000"),
        ("import somelib", "import 9bad"),
        ("values Bool=bool Bytes=bytes Nat=int U64=int", "values U64=float"),
        ("values Bool=bool Bytes=bytes Nat=int U64=int", "values U64"),
        ("values Bool=bool Bytes=bytes Nat=int U64=int", "values U64=int U64=int"),
        ("op eq export=ct_eq\n", "op eq\n"),
        ("op eq export=ct_eq\n", "op Eq export=ct_eq\n"),
        ("op eq export=ct_eq\n", "op eq export=ct_eq export=ct_eq\n"),
        ("op eq export=ct_eq\n", "op eq export=ct_eq mystery=1\n"),
        ("op eq export=ct_eq\n", "op eq export=not-an-identifier\n"),
        ("op eq export=ct_eq\n", "op eq export=ct_eq width=static\n"),
        ("op eq export=ct_eq\n", "op eq export=ct_eq\nop eq export=ct_eq\n"),
        (
            "export=ct_select_bytes errors=CT_SELECT_LENGTH_MISMATCH:ValueError",
            "export=ct_select_bytes errors=lower:ValueError",
        ),
        (
            "errors=CT_SELECT_LENGTH_MISMATCH:ValueError",
            "errors=CT_SELECT_LENGTH_MISMATCH",
        ),
        (
            "errors=CT_SELECT_LENGTH_MISMATCH:ValueError",
            "errors=CT_SELECT_LENGTH_MISMATCH:Exception",
        ),
        (
            "errors=CT_SELECT_LENGTH_MISMATCH:ValueError",
            "errors=CT_SELECT_LENGTH_MISMATCH:BaseException",
        ),
        (
            "errors=CT_SELECT_LENGTH_MISMATCH:ValueError",
            "errors=CT_SELECT_LENGTH_MISMATCH:int",
        ),
        (
            "errors=CT_SELECT_LENGTH_MISMATCH:ValueError",
            "errors=CT_SELECT_LENGTH_MISMATCH:NoSuchError",
        ),
    ],
)
def test_malformed_directives_are_refused(old: str, new: str) -> None:
    text = GOOD.decode().replace("import {name}", "import somelib")
    with pytest.raises(BindingError):
        parse_binding(text.replace(old, new).encode())


def test_missing_directives_or_ops_are_refused() -> None:
    lines = GOOD.decode().splitlines()
    without_ops = "\n".join(line for line in lines if not line.startswith("op "))
    with pytest.raises(BindingError, match="lacks"):
        parse_binding(without_ops.encode())
    without_lane = "\n".join(line for line in lines if not line.startswith("lane"))
    with pytest.raises(BindingError, match="lacks"):
        parse_binding(without_lane.encode())


def test_resolve_exports_imports_the_library(tmp_path, monkeypatch) -> None:
    _name, package_dir, binding_path = new_library(tmp_path, monkeypatch)
    binding = parse_binding(binding_path.read_bytes())
    exports = resolve_exports(binding, package_dir)
    assert set(exports) == {"eq", "eq_fixed", "select_bytes", "eq_u64"}


def test_resolve_refuses_a_missing_module_export_or_wrong_source(
    tmp_path, monkeypatch
) -> None:
    name, package_dir, binding_path = new_library(tmp_path, monkeypatch)
    text = binding_path.read_text()
    with pytest.raises(BindingError, match="cannot import"):
        resolve_exports(
            parse_binding(
                text.replace(f"import {name}", "import not_a_module_xyz").encode()
            ),
            package_dir,
        )
    with pytest.raises(BindingError, match="missing or not callable"):
        resolve_exports(
            parse_binding(
                text.replace("export=ct_eq\n", "export=ct_absent\n").encode()
            ),
            package_dir,
        )
    with pytest.raises(BindingError, match="source root"):
        # an importable module that does not live under the declared source root
        resolve_exports(
            parse_binding(
                text.replace(f"source src/{name}", "source src/elsewhere").encode()
            ),
            package_dir,
        )
    with pytest.raises(BindingError, match="source root"):
        # a module with no __file__ at all
        resolve_exports(
            parse_binding(text.replace(f"import {name}", "import sys").encode()),
            package_dir,
        )


def make_subject(tmp_path, monkeypatch, overrides=None, fault=None, fault_op=None):
    from coding_adventures_warrant_corpus.values import OpSig, parse_type

    _n, package_dir, binding_path = new_library(tmp_path, monkeypatch, overrides)
    binding = parse_binding(binding_path.read_bytes())
    signatures = {
        "eq": OpSig(
            "eq",
            (("a", parse_type("Bytes")), ("b", parse_type("Bytes"))),
            parse_type("Bool"),
        ),
        "eq_fixed": OpSig(
            "eq_fixed",
            (("a", parse_type("Bytes<N>")), ("b", parse_type("Bytes<N>"))),
            parse_type("Bool"),
            ("N",),
        ),
        "select_bytes": OpSig(
            "select_bytes",
            (
                ("a", parse_type("Bytes")),
                ("b", parse_type("Bytes")),
                ("choice", parse_type("Bool")),
            ),
            parse_type("Result<Bytes,SelectError>"),
        ),
        "eq_u64": OpSig(
            "eq_u64",
            (("a", parse_type("U64")), ("b", parse_type("U64"))),
            parse_type("Bool"),
        ),
    }
    return Subject(
        binding, resolve_exports(binding, package_dir), signatures, fault, fault_op
    )


def test_subject_normalises_values_and_errors(tmp_path, monkeypatch) -> None:
    subject = make_subject(tmp_path, monkeypatch)
    assert subject.call("eq", [b"a", b"a"], None) is True
    assert subject.call("select_bytes", [b"a", b"b", True], None) == ("ok", b"a")
    assert subject.call("select_bytes", [b"a", b"bb", True], None) == (
        "err",
        "CT_SELECT_LENGTH_MISMATCH",
    )
    assert subject.calls["eq"] == 1 and subject.calls["select_bytes"] == 2
    assert subject.total_calls(["eq", "select_bytes"]) == 3


def test_unmapped_exceptions_are_unexpected_errors(tmp_path, monkeypatch) -> None:
    subject = make_subject(
        tmp_path, monkeypatch, {"ct_eq": "def ct_eq(a, b):\n    raise KeyError('x')\n"}
    )
    with pytest.raises(SubjectFailure) as caught:
        subject.call("eq", [b"", b""], None)
    assert caught.value.failure_id == "unexpected-error"


def test_a_returned_tuple_is_wrapped_not_trusted(tmp_path, monkeypatch) -> None:
    subject = make_subject(
        tmp_path,
        monkeypatch,
        {
            "ct_select_bytes": "def ct_select_bytes(a, b, choice):\n    return ('err', 'CT_SELECT_LENGTH_MISMATCH')\n"
        },
    )
    # Wrapped as Ok((...)): the runner's type check will then reject it.
    assert subject.call("select_bytes", [b"", b"", True], None) == (
        "ok",
        ("err", "CT_SELECT_LENGTH_MISMATCH"),
    )


def test_width_is_checked_and_runtime_width_is_passed(tmp_path, monkeypatch) -> None:
    subject = make_subject(tmp_path, monkeypatch)
    assert subject.call("eq_fixed", [b"ab", b"ab"], 2) is True
    with pytest.raises(CorpusError):
        subject.call("eq_fixed", [b"ab", b"abc"], 2)

    runtime = make_subject(
        tmp_path,
        monkeypatch,
        {
            "ct_eq_fixed": "def ct_eq_fixed(a, b, width):\n    return len(a) == width and a == b\n"
        },
    )
    runtime._binding.ops["eq_fixed"] = type(runtime._binding.ops["eq_fixed"])(
        "eq_fixed", "ct_eq_fixed", "runtime"
    )
    assert runtime.call("eq_fixed", [b"ab", b"ab"], 2) is True


def test_faults_wrap_only_the_chosen_op(tmp_path, monkeypatch) -> None:
    subject = make_subject(tmp_path, monkeypatch, fault=FAULTS["invert"], fault_op="eq")
    assert subject.call("eq", [b"a", b"a"], None) is False  # inverted
    assert subject.call("eq_u64", [1, 1], None) is True  # other ops untouched
    everywhere = make_subject(tmp_path, monkeypatch, fault=FAULTS["invert"])
    assert everywhere.call("eq_u64", [1, 1], None) is False
    assert everywhere.call("select_bytes", [b"", b"", True], None) == ("ok", b"")  # n/a
