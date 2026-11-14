"""Shared test scaffolding: a reference library, bindings and tampered corpora.

The runner must work with *any* Python library, so these tests do not import
``coding_adventures_ct_compare`` (the runner has no dependency on a leaf
library).  Instead each test writes a small reference library to a temporary
directory, with optional one-line mutations, and binds it.  The real corpus
(emitted by the Rust compiler) is used unmodified wherever possible: its
digests come from outside this package, which is what makes agreeing with them
meaningful.
"""

from __future__ import annotations

import itertools
import json
import shutil
from collections.abc import Callable
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[5]
CORPUS_DIR = REPO_ROOT / "code" / "specs" / "fixtures" / "ct-compare-warrant-v1"
SPEC_PATH = REPO_ROOT / "code" / "specs" / "ct-compare.warrant"
REGISTRY_PATH = CORPUS_DIR / "consumers.json"

_counter = itertools.count()

CORRECT = {
    "ct_eq": "def ct_eq(a, b):\n    return a == b\n",
    "ct_eq_fixed": "def ct_eq_fixed(a, b):\n    return a == b\n",
    "ct_select_bytes": (
        "def ct_select_bytes(a, b, choice):\n"
        "    if len(a) != len(b):\n"
        "        raise ValueError('length mismatch')\n"
        "    return bytes(a) if choice else bytes(b)\n"
    ),
    "ct_eq_u64": "def ct_eq_u64(a, b):\n    return a == b\n",
}

BINDING_TEXT = """\
lane python
package python/ct-compare
module ct_compare
import {name}
source src/{name}
seed 0x5741_5252_414e_5421
values Bool=bool Bytes=bytes Nat=int U64=int
op eq export=ct_eq
op eq_fixed export=ct_eq_fixed width=implicit
op select_bytes export=ct_select_bytes errors=CT_SELECT_LENGTH_MISMATCH:ValueError
op eq_u64 export=ct_eq_u64
"""


def new_library(
    tmp_path: Path, monkeypatch, overrides: dict[str, str] | None = None
) -> tuple[str, Path, Path]:
    """Write a reference library; return ``(module name, package dir, binding)``."""
    name = f"ctref_{next(_counter)}"
    functions = {**CORRECT, **(overrides or {})}
    source_dir = tmp_path / "src" / name
    source_dir.mkdir(parents=True)
    (source_dir / "__init__.py").write_text("\n".join(functions.values()))
    binding = tmp_path / "evidence.bindings"
    binding.write_text(BINDING_TEXT.format(name=name))
    monkeypatch.syspath_prepend(str(tmp_path / "src"))
    return name, tmp_path, binding


def copy_corpus(
    tmp_path: Path,
    corpus: Callable[[dict], None] | None = None,
    tfv: Callable[[str], str] | None = None,
    spec: Callable[[bytes], bytes] | None = None,
) -> tuple[Path, Path]:
    """Copy the real corpus into a scratch repo, optionally tampering with it."""
    root = tmp_path / f"repo{next(_counter)}"
    corpus_dir = root / "code" / "specs" / "fixtures" / "ct-compare-warrant-v1"
    corpus_dir.mkdir(parents=True)
    (root / "code" / "specs").mkdir(exist_ok=True)
    document = json.loads((CORPUS_DIR / "corpus.json").read_text())
    if corpus:
        corpus(document)
    (corpus_dir / "corpus.json").write_text(json.dumps(document, indent=1))
    text = (CORPUS_DIR / "cases.tfv").read_text()
    (corpus_dir / "cases.tfv").write_text(tfv(text) if tfv else text)
    spec_bytes = SPEC_PATH.read_bytes()
    (root / "code" / "specs" / "ct-compare.warrant").write_bytes(
        spec(spec_bytes) if spec else spec_bytes
    )
    shutil.copy(REGISTRY_PATH, corpus_dir / "consumers.json")
    return root, corpus_dir


_KEEP = ("eq_fixed_agrees_w2", "eq_fixed_agrees_w4")


def mini_corpus(tmp_path: Path) -> tuple[Path, Path]:
    """A consistent two-suite slice of the real corpus (512 cases in all).

    Whole-corpus runs take seconds (and minutes under coverage), so tests of
    command-line and manifest *plumbing* use this slice; one session-wide test
    runs the full corpus and checks its digests against the Rust compiler's.
    """

    def keep_slice(document: dict) -> None:
        document["suites"] = [s for s in document["suites"] if s["claim"] in _KEEP]
        document["examples"] = [
            e for e in document["examples"] if e["op"] == "eq_fixed"
        ]
        document["faults"] = [f for f in document["faults"] if f["op"] == "eq_fixed"]
        document["controls"] = [
            c for c in document["controls"] if c["op"] == "eq_fixed"
        ]
        for control in document["controls"]:
            control["required_claims"] = [
                f"warrant/ct_compare/exhaustive/{claim}" for claim in _KEEP
            ]

    def keep_tfv(text: str) -> str:
        lines = text.split("\n")
        return (
            "\n".join(
                [lines[0], *[ln for ln in lines[1:] if "Kop S65715f6669786564" in ln]]
            )
            + "\n"
        )

    return copy_corpus(tmp_path, corpus=keep_slice, tfv=keep_tfv)
