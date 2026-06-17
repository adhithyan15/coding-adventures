#!/usr/bin/env python3
"""test_timing.py — guard the ADJ-native wait-vs-treat precedence ladder + runtime (CC-5).

Pure checks: the rulebook carries the functional decl + the priority-tiered rules; the runtime
rejects unsafe/unknown inputs. Engine-gated checks (if adj-lang-cli is built): the four ladder
outcomes resolve by precedence — resulted→targeted (mandatory), time-critical/unstable→treat-now
(authoritative, high risk), stable+routine+pending→await (specific, low), else→treat-now
(default, moderate). The decision is the engine's; delay_risk reads off the governing tier.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent.parent / "warm"))
import decide as decide_mod  # noqa: E402
import timing as timing_mod  # noqa: E402


def test_rulebook_has_functional_decl_and_priority_tiers():
    text = timing_mod.RULEBOOK.read_text()
    assert "functional timing(decision)" in text
    assert "priority: mandatory" in text and "priority: authoritative" in text
    assert "priority: specific" in text and "priority: default" in text


@pytest.mark.parametrize("bad", ["resultd", "Pending", "x;y", "pending)\n? evil("])
def test_unknown_inputs_are_rejected(bad):
    # (An empty string is NOT rejected — it is coerced to the conservative "unknown" default.)
    with pytest.raises(ValueError):
        timing_mod.derive_timing(_DUMMY_CLI, bad, "stable", "routine")


class _DummyCli:
    def __fspath__(self):  # pragma: no cover - never reached when the guard fires
        raise AssertionError("the engine must not be invoked for a rejected input")


_DUMMY_CLI = _DummyCli()


def _cli_or_skip():
    cli = decide_mod.find_cli()
    if cli is None:
        pytest.skip("adj-lang-cli not built")
    return cli


def test_resulted_culture_is_targeted_mandatory():
    cli = _cli_or_skip()
    r = timing_mod.derive_timing(cli, "resulted", "stable", "routine")
    assert r["decision"] == "targeted_culture_directed" and r["delay_risk"] == "none"
    assert r["standing"] == "mandatory" and r["governing"]


def test_time_critical_is_treat_now_authoritative_high():
    cli = _cli_or_skip()
    r = timing_mod.derive_timing(cli, "pending", "stable", "time_critical")
    assert r["decision"] == "treat_now_empiric" and r["delay_risk"] == "high"
    assert r["standing"] == "authoritative"


def test_unstable_patient_forces_treat_now():
    cli = _cli_or_skip()
    r = timing_mod.derive_timing(cli, "pending", "unstable", "routine")
    assert r["decision"] == "treat_now_empiric" and r["delay_risk"] == "high"


def test_stable_routine_pending_awaits_culture():
    cli = _cli_or_skip()
    r = timing_mod.derive_timing(cli, "pending", "stable", "routine")
    assert r["decision"] == "await_culture" and r["delay_risk"] == "low"
    assert r["standing"] == "specific"


def test_default_fallback_is_treat_now_moderate():
    cli = _cli_or_skip()
    # unknown culture + stable + routine: nothing specific fires → conservative default.
    r = timing_mod.derive_timing(cli, "unknown", "stable", "routine")
    assert r["decision"] == "treat_now_empiric" and r["delay_risk"] == "moderate"
    assert r["standing"] == "default"


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-q"]))
