#!/usr/bin/env python3
"""test_step_therapy.py — guard the ADJ-native step-therapy precedence rule + runtime.

Pure checks (no engine): the rulebook is present and carries the negation-as-failure rule;
the runtime's injection guard refuses unsafe drug tokens and short-circuits an empty policy.
Engine-gated checks (if adj-lang-cli is built): the engine derives exactly the blocked drugs
via `reimbursement_blocked($Y) when: requires_prerequisite($Y,$X), not already_tried($X)` —
a restricted drug whose prerequisite is untried is blocked; once the prerequisite is tried it
is not; an unrestricted drug is never blocked. The precedence reasoning is the engine's.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent.parent / "warm"))
import decide as decide_mod  # noqa: E402
import step_therapy as st  # noqa: E402


# --------------------------------------------------------------------------
# Rulebook + injection guard (pure).
# --------------------------------------------------------------------------
def test_rulebook_carries_the_naf_precedence_rule():
    text = st.RULEBOOK.read_text()
    assert "reimbursement_blocked($Y)" in text
    assert "requires_prerequisite($Y, $X)" in text and "not already_tried($X)" in text


@pytest.mark.parametrize("bad", ["cefepime)\n? evil(", "Cefepime", "a b", "x;y", "", "1drug"])
def test_unsafe_drug_tokens_are_rejected(bad):
    with pytest.raises(ValueError):
        st.derive_blocked(_DUMMY_CLI, {(bad, "meropenem")}, set())
    with pytest.raises(ValueError):
        st.derive_blocked(_DUMMY_CLI, {("cefepime", "meropenem")}, {bad})


def test_empty_policy_short_circuits_without_the_engine():
    assert st.derive_blocked(_DUMMY_CLI, set(), {"vancomycin"}) == set()


class _DummyCli:
    def __fspath__(self):  # pragma: no cover - never reached when the guard fires
        raise AssertionError("the engine must not be invoked for a rejected/empty policy")


_DUMMY_CLI = _DummyCli()


# --------------------------------------------------------------------------
# Engine-gated — the ENGINE derives the blocked set (negation-as-failure).
# --------------------------------------------------------------------------
def _cli_or_skip():
    cli = decide_mod.find_cli()
    if cli is None:
        pytest.skip("adj-lang-cli not built")
    return cli


def test_engine_blocks_a_drug_whose_prerequisite_is_untried():
    cli = _cli_or_skip()
    assert st.derive_blocked(cli, {("cefepime", "meropenem")}, set()) == {"cefepime"}


def test_engine_unblocks_once_the_prerequisite_is_tried():
    cli = _cli_or_skip()
    assert st.derive_blocked(cli, {("cefepime", "meropenem")}, {"meropenem"}) == set()


def test_engine_handles_multiple_policies_independently():
    cli = _cli_or_skip()
    # cefepime's prereq (meropenem) untried → blocked; linezolid's (vancomycin) tried → not.
    out = st.derive_blocked(cli, {("cefepime", "meropenem"), ("linezolid", "vancomycin")},
                            {"vancomycin"})
    assert out == {"cefepime"}


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-q"]))
