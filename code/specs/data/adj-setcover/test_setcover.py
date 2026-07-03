#!/usr/bin/env python3
"""test_setcover.py — guard the domain-agnostic set-cover library.

Pure checks (no model) cover the emitter, validation, and the content-hash; the
solve + cache + cross-domain checks run when adj-lang-cli is built (skipped cleanly
otherwise). CI runs this.
"""

from __future__ import annotations

import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import setcover as sc  # noqa: E402


def base_spec() -> sc.SetCoverSpec:
    # r1 covered only by the combination {a,b}; r2 by single c.
    return sc.SetCoverSpec(
        costs={"a": 1, "b": 1, "c": 1, "d": 5},
        requirements=["r1", "r2"],
        covers={"c": ["r2"]},
        combinations=[sc.Combination(("a", "b"), "r1")],
    )


def test_content_hash_is_order_stable() -> None:
    s1 = base_spec()
    s2 = sc.SetCoverSpec(
        costs={"d": 5, "c": 1, "b": 1, "a": 1},   # different dict order
        requirements=["r2", "r1"],
        covers={"c": ["r2"]},
        combinations=[sc.Combination(("a", "b"), "r1")],
    )
    assert s1.content_hash() == s2.content_hash(), "hash must be canonical/order-stable"
    # A material change (a defeater) changes the hash.
    s3 = base_spec()
    s3.defeated = [("c", "r2")]
    assert s3.content_hash() != s1.content_hash()


def test_emit_rejects_unsafe_tokens() -> None:
    bad = sc.SetCoverSpec(costs={"a": 1}, requirements=["r)\nminimize evil"], covers={"a": ["r)\nminimize evil"]})
    try:
        sc.emit_program(bad)
    except ValueError:
        return
    raise AssertionError("expected ValueError on an injection-shaped requirement")


def test_emit_is_well_formed() -> None:
    prog, var_to_elem, feasible = sc.emit_program(base_spec())
    assert feasible and "minimize" in prog
    assert "symbol y_0 : bool" in prog, "the {a,b} combination needs an AND aux"
    assert set(var_to_elem.values()) == {"a", "b", "c", "d"}


def test_exclusion_removes_elements() -> None:
    spec = base_spec()
    spec.excluded_by = {"c": ["banned"]}
    spec.exclusions = ["banned"]
    assert "c" not in sc.candidates(spec)


def main() -> int:
    test_content_hash_is_order_stable()
    test_emit_rejects_unsafe_tokens()
    test_emit_is_well_formed()
    test_exclusion_removes_elements()

    cli = sc.find_cli()
    if cli is None:
        print("test_setcover: PASS (emit/hash/validation); solve checks SKIPPED (adj-lang-cli not built)")
        return 0

    cache = HERE / "_test_cache"
    shutil.rmtree(cache, ignore_errors=True)

    # Combination is required: r1 only covered by {a,b} → both selected; c for r2.
    res = sc.solve(base_spec(), cli=cli, cache_dir=cache)
    assert res.outcome == "optimal" and set(res.selected) == {"a", "b", "c"}, res
    assert any(c.covers == "r1" for c in res.used_combinations)
    assert not res.cached
    # Re-solve → content-addressed cache hit, identical result.
    res2 = sc.solve(base_spec(), cli=cli, cache_dir=cache)
    assert res2.cached and set(res2.selected) == {"a", "b", "c"}, res2

    # Defeasance: void c's coverage of r2, and give an alternative d covering r2.
    spec = base_spec()
    spec.covers["d"] = ["r2"]
    spec.defeated = [("c", "r2")]
    res3 = sc.solve(spec, cli=cli, cache_dir=cache)
    assert "c" not in (res3.selected or []) and "d" in (res3.selected or []), res3

    # Infeasible: a requirement nothing covers → no cover, reported (not fabricated).
    bad = sc.SetCoverSpec(costs={"a": 1}, requirements=["unreachable"], covers={})
    res4 = sc.solve(bad, cli=cli)
    assert res4.selected is None and res4.outcome == "infeasible", res4

    shutil.rmtree(cache, ignore_errors=True)
    print("test_setcover: PASS (combination required; cache hit; defeasance re-derives; "
          "infeasible reported; domain-agnostic)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
