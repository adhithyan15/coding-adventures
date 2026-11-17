"""Shared fixtures: repository paths and a few small inline ``.warrant`` documents."""

from __future__ import annotations

import pathlib

import pytest

# tests/ -> warrant-oracle -> python -> packages -> code
CODE = pathlib.Path(__file__).resolve().parents[4]
SPECS = CODE / "specs"
ADOPTERS = ["ct-compare", "gf256", "wasm-leb128", "logic-gates", "lzw", "arithmetic", "x25519"]
V1_CORPUS = SPECS / "fixtures" / "ct-compare-warrant-v1" / "corpus.json"


@pytest.fixture(scope="session")
def specs_dir() -> pathlib.Path:
    return SPECS


def adopter_bytes(name: str) -> bytes:
    return (SPECS / f"{name}.warrant").read_bytes()


#: A tiny v0 document used all over the tests: three binders of different sizes,
#: a refinement, a relational claim and a reference claim.
TINY = b"""warrant tiny v1
  error E = BAD | WORSE
  model inc(a: U8) -> U8 = a + 1u8
  model same(a: U8, b: U8) -> Bool = a == b
  op bump(a: U8) -> U8
  op pick(a: Bool, b: Bool) -> Result<Bool, E>
  claim bump_is_inc:
    forall a: U8. bump(a) == inc(a)
  claim bump_commutes_with_itself:
    forall a: U8, b: U8 | a != b. bump(a) != bump(b)
  claim pick_total:
    forall a: Bool, b: Bool. pick(a, b) == (if a then Ok(b) else Err(E.BAD))
  example e1:
    bump(1u8) == 2u8
"""


@pytest.fixture(scope="session")
def tiny() -> bytes:
    return TINY
