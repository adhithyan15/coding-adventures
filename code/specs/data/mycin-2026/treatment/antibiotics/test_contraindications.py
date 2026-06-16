#!/usr/bin/env python3
"""test_contraindications.py — guard the ADJ-native contraindication rulebook + runtime.

Pure checks (no engine): the generator emits a parseable, grounded rulebook and `--check`
agrees with the committed file; the runtime's injection guard refuses unsafe context tokens.
Engine-gated checks (if adj-lang-cli is built): the engine derives exactly the pregnancy
contraindications {moxifloxacin, tmp_smx} with grounded provenance, an unrelated context
derives nothing, and the genericity demo (the SAME rule firing for qt_prolongation) holds —
proving the reasoning lives in the language, not in Python.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent.parent / "warm"))
import contraindications as ci  # noqa: E402
import contraindications_build as cib  # noqa: E402
import decide as decide_mod  # noqa: E402


# --------------------------------------------------------------------------
# Generator (pure) — the rulebook is generated, grounded, and `--check`-stable.
# --------------------------------------------------------------------------
def test_generator_check_is_up_to_date():
    # The committed contraindications.adj + manifest must match a fresh regeneration, so the
    # CAS artifact is never hand-edited out of sync with the grounding (mirrors iem --check).
    assert cib.build(check=True) == 0, "contraindications.adj is OUT OF DATE — regenerate it"


def test_generated_rulebook_is_grounded_and_parseable():
    text = cib.ADJ.read_text()
    # Every contraindication fact carries a grounded byte-quote at authoritative trust.
    assert "class_contraindicated_in(fluoroquinolone, pregnancy)" in text
    assert "drug_contraindicated_in(tmp_smx, pregnancy)" in text
    assert "trust authoritative" in text and "dailymed" in text
    # The two GENERIC, context-scoped derivation rules are present (not drug-specific Python).
    assert text.count("rule {") == 2 and "active_context($C)" in text


# --------------------------------------------------------------------------
# Runtime injection guard (pure) — a context token is a closed-vocabulary symbol.
# --------------------------------------------------------------------------
@pytest.mark.parametrize("bad", ["pregnancy)\n? evil(", "PREGNANCY", "a b", "x;y", "", "1context"])
def test_unsafe_context_tokens_are_rejected(bad):
    with pytest.raises(ValueError):
        ci.derive_contraindications(_DUMMY_CLI, {bad})


def test_no_active_contexts_short_circuits_without_the_engine():
    # An empty context set must NOT touch the engine (cli unused) and derive nothing.
    assert ci.derive_contraindications(_DUMMY_CLI, set()) == {}


class _DummyCli:
    """A CLI path that would fail loudly if ever invoked — proves the guards run first."""
    def __fspath__(self):  # pragma: no cover - never reached when the guard fires
        raise AssertionError("the engine must not be invoked for a rejected/empty context")


_DUMMY_CLI = _DummyCli()


# --------------------------------------------------------------------------
# Engine-gated — the ENGINE derives the contraindications (the whole point).
# --------------------------------------------------------------------------
def _cli_or_skip():
    cli = decide_mod.find_cli()
    if cli is None:
        pytest.skip("adj-lang-cli not built")
    return cli


def test_engine_derives_the_pregnancy_contraindications_with_provenance():
    cli = _cli_or_skip()
    out = ci.derive_contraindications(cli, {"pregnancy"})
    assert set(out) == {"moxifloxacin", "tmp_smx"}
    for drug, info in out.items():
        assert info["context"] == "pregnancy"
        assert info["trust"] == "authoritative" and info["source"]  # grounded byte-quote flows through
        assert info["locator"] and "dailymed" in info["locator"]


def test_unrelated_context_derives_nothing():
    cli = _cli_or_skip()
    # A context with no grounded contraindication fact yields no exclusions (no fabrication).
    assert ci.derive_contraindications(cli, {"hypertension"}) == {}


def test_generic_rule_also_fires_for_qt_prolongation():
    cli = _cli_or_skip()
    # The SAME generic class rule excludes the fluoroquinolone in a DIFFERENT context — proving
    # the mechanism is context-driven, not pregnancy-hardcoded (the US-Code-style generality).
    out = ci.derive_contraindications(cli, {"qt_prolongation"})
    assert set(out) == {"moxifloxacin"} and out["moxifloxacin"]["context"] == "qt_prolongation"


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-q"]))
