"""Shared fixtures: repository paths and a few small inline ``.warrant`` documents."""

from __future__ import annotations

import importlib.util
import pathlib
import sys
from types import SimpleNamespace

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


def write_lf(path: pathlib.Path, text: str) -> None:
    """Write ``text`` to ``path`` byte-for-byte, with LF line endings on every operating system.

    ``Path.write_text`` uses the platform newline, so on Windows it turns each ``\\n`` into
    ``\\r\\n``. The profile, bindings and corpus readers parse bytes strictly and refuse a CR
    as a ``corpus`` error, so a test that meant to break one defect would fail for CRLF
    instead. Encoding the text here keeps the fixture identical on Linux, macOS and Windows.
    """
    path.write_bytes(text.encode("utf-8"))


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


# -- the v2 conformance kit (slice S5a) ---------------------------------------------------------------

CONFORMANCE_DIR = pathlib.Path(__file__).resolve().parent / "fixtures" / "conformance-v2"


def load_make_conformance():
    """``tests/fixtures/conformance-v2/make_conformance.py`` as a module (it is a script, not a package)."""
    spec = importlib.util.spec_from_file_location("make_conformance", CONFORMANCE_DIR / "make_conformance.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules["make_conformance"] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="session")
def kit():
    """The whole v2 kit, derived once per test session (about a minute: the kit holds a 2**24 suite).

    ``kit.kits`` are the derived kit documents, ``kit.tables`` the six case tables, ``kit.files`` the
    text of the six ``v2-*.json`` files and ``kit.rows`` their index rows.
    """
    from coding_adventures_warrant_oracle import conformance as C

    mc = load_make_conformance()
    kits = mc.load_kits()
    tables = mc.load_tables(kits)
    files, rows = C.build_files(tables, kits)
    return SimpleNamespace(mc=mc, kits=kits, tables=tables, files=files, rows=rows)
