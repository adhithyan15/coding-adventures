"""CV2-F8: a ``match`` bind may not shadow a model, op, error type, constructor or reserved built-in.

The Rust checker refuses such a bind (``check_arm_binder``, W0203); the oracle used to accept it, so
the two tools could disagree on what a name in an arm refers to.  VER04r 16.10 R2 makes any shadowing
bind a ``corpus`` error, and the oracle refuses the new cases the same way.
"""

from __future__ import annotations

import re
from pathlib import Path

import pytest

from coding_adventures_warrant_oracle import lower as LW
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source

BASE = (
    "warrant t{v}\n"
    "  error E = A | B\n"
    "  op h(a: Bool) -> Result<Bool,E>\n"
    "  model helper(a: Bool) -> Bool = a\n"
)


def lower(body: str, version: str = " v1", params: str = "r: Result<Bool,E>"):
    variant = "  variant Shape = Circle(radius: Nat) | Dot\n" if version else ""
    text = BASE.format(v=version) + variant + f"  model m({params}) -> Bool = {body}\n"
    return lower_document(parse_source(text))


def refused(body: str, version: str = " v1", params: str = "r: Result<Bool,E>") -> OracleError:
    with pytest.raises(OracleError) as err:
        lower(body, version, params)
    return err.value


@pytest.mark.parametrize(
    "name", ["helper", "m", "h", "E", "take", "widen", "len", "Result", "U32", "fold", "div"]
)
@pytest.mark.parametrize("arm", ["Ok({n}) -> true | Err(_) -> false", "Ok(_) -> true | Err({n}) -> false"])
def test_a_reserved_bind_name_is_a_corpus_error(name, arm):
    e = refused("match r with " + arm.format(n=name))
    assert e.code == "corpus"
    assert "shadows" in e.message


def test_a_model_name_is_refused():
    assert refused("match r with Ok(helper) -> true | Err(_) -> false").code == "corpus"


def test_an_op_name_is_refused():
    assert refused("match r with Ok(h) -> true | Err(_) -> false").code == "corpus"


def test_an_error_type_name_is_refused():
    assert refused("match r with Ok(E) -> true | Err(_) -> false").code == "corpus"


def test_a_reserved_built_in_is_refused_in_a_tuple_pattern():
    e = refused(
        "match r with Ok((rotl, b)) -> b | Err(_) -> false", params="r: Result<(Bool,Bool),E>"
    )
    assert e.code == "corpus"


def test_a_whole_value_bind_is_refused():
    assert refused("match r with take -> true").code == "corpus"


def test_constructors_are_still_refused():
    # A level refusal is recorded on the model, not raised (an `ir.data` document still lowers).
    for ctor in ("Some", "Circle"):
        m = lower(f"match r with Ok({ctor}) -> true | Err(_) -> false").models["m"]
        assert m.unsupported is not None and m.unsupported.code == "level"
    assert refused("match r with Ok(Ok) -> true | Err(_) -> false").code == "type"


def test_a_v1_built_in_is_an_ordinary_name_in_a_v0_document():
    # Version scoped (VER04j 1.2): `take` is only reserved in v1, but `len` is reserved everywhere.
    assert "take" in LW._RESERVED_V1 and "take" not in LW._RESERVED_V0
    assert lower("match r with Ok(take) -> take | Err(_) -> false", "").models["m"]
    assert refused("match r with Ok(len) -> len | Err(_) -> false", "").code == "corpus"


def test_an_ordinary_bind_name_still_lowers():
    m = lower("match r with Ok(x) -> x | Err(e) -> false").models["m"]
    assert m.ir[0] == "match"


def _checker_names() -> tuple[set[str], set[str]] | None:
    src = Path(__file__).resolve().parents[3] / "rust/warrant-grammar/src/checker.rs"
    if not src.exists():
        return None
    text = src.read_text()
    def scan(const: str) -> set[str]:
        i = text.index(f"const {const}")
        j = text.index("];", i)
        return set(re.findall(r'"(\w+)"', text[i:j]))

    v0 = scan("V0_BUILTINS")
    return v0, v0 | scan("V1_TYPES") | scan("V1_FUNCTIONS")


def test_the_reserved_set_equals_the_checkers():
    names = _checker_names()
    if names is None:
        pytest.skip("the Rust checker is not in this checkout")
    v0, v1 = names
    # V0_BUILTINS' header is `[&str; 18]`, which has no quotes, so the scan sees only names.
    assert v0 == set(LW._RESERVED_V0)
    assert v1 == set(LW._RESERVED_V1)


@pytest.mark.parametrize("pattern", ["Ok(Point)", "Err(Point)"])
def test_a_bind_named_like_a_record_is_refused(pattern):
    text = (
        "warrant t v1\n"
        "  error E = A | B\n"
        "  record Point(x: Nat)\n"
        "  model m(r: Result<Bool,E>) -> Bool = match r with "
        + pattern
        + " -> true | "
        + ("Err(_)" if pattern.startswith("Ok") else "Ok(_)")
        + " -> false\n"
    )
    # A level refusal is recorded on the model, not raised.
    m = lower_document(parse_source(text)).models["m"]
    assert m.unsupported is not None and m.unsupported.code == "level"
