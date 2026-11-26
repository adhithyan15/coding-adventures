"""VER33 4.9 / ADL-001a: the ``profile`` header clause, as the independent reader sees it.

The Rust reader (``warrant-grammar``) owns the profile semantics and the W0801 to W0805
diagnostics.  The oracle verifies *corpus* subjects, so a ``profile algorithm`` document
(a model file with no ops) is recognised and refused by name, never read as a pure document
and never skipped; a malformed profile clause is a parse error exactly where Rust says W0801.
"""

from __future__ import annotations

import pytest

from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.parser import parse_source

BODY = "  model f(a: Int) -> Int = a\n"


def test_a_pure_document_still_parses() -> None:
    assert parse_source(("warrant m v1\n" + BODY).encode()).version == "v1"
    assert parse_source(("warrant m\n" + BODY).encode()).version == "v0"


def test_an_algorithm_document_is_named_and_refused_as_a_level() -> None:
    with pytest.raises(OracleError) as err:
        parse_source(("warrant m v1 profile algorithm\n" + BODY).encode())
    assert err.value.code == "level"
    assert err.value.level == "profile.algorithm"


@pytest.mark.parametrize(
    "header",
    [
        "warrant m profile algorithm",  # a profile needs v1
        "warrant m v1 profile pure",  # the only profile is `algorithm`
        "warrant m v1 profile",  # a missing name
        "warrant m v1 profile algorithm profile",  # one clause only
    ],
)
def test_a_malformed_profile_clause_is_a_parse_error(header: str) -> None:
    with pytest.raises(OracleError) as err:
        parse_source((header + "\n" + BODY).encode())
    assert err.value.code == "parse"
