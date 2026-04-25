"""LANG17 PR4 — TetradRuntime legacy-shape API parity tests.

These tests verify that ``TetradRuntime`` exposes every metric API the
legacy ``TetradVM`` exposed, with the **same return-type signatures**.
A caller can switch from ``TetradVM`` → ``TetradRuntime`` without
rewriting metric-reading code.

Why we don't compare values directly with TetradVM
--------------------------------------------------

The two runtimes execute Tetrad programs *differently*:

- ``TetradVM.execute(code)`` runs the top-level CodeObject only (a
  HALT for most programs); the user's ``fn main()`` is a sibling
  CodeObject that is never auto-called.  ``TetradJIT.execute_with_jit``
  is what wraps main into the executed flow.
- ``TetradRuntime.run(source)`` always wraps the user's ``fn main()``
  in a synthetic ``__entry__`` function that runs globals and calls
  main, so main and its callees are always in the call counts.

These different execution scopes mean per-call-count values will
naturally differ.  What MUST match is the API shape — return types,
empty-default semantics, and the existence of every legacy method.
The shape tests below pin that down; the integration test suite in
``test_runtime.py`` already verifies that ``TetradRuntime`` produces
the same *result* as semantic Tetrad.

We use the legacy ``TetradVM`` import as a smoke check that both
runtimes can run the same program without crashing — if a program
that runs on legacy fails on TetradRuntime (or vice versa), the
parity contract is broken.
"""

from __future__ import annotations

from interpreter_ir import SlotKind
from tetrad_compiler import compile_program
from tetrad_vm import TetradVM

from tetrad_runtime import TetradRuntime

# ---------------------------------------------------------------------------
# Shared test programs
# ---------------------------------------------------------------------------


_LOOP_PROGRAM = """
fn count(n: u8) -> u8 {
  let i = 0;
  while i < n { i = i + 1; }
  return i;
}
fn main() -> u8 { return count(5); }
"""


_BRANCH_PROGRAM = """
fn pick(c: u8) -> u8 {
  if c { return 100; } else { return 200; }
}
fn main() -> u8 { return pick(1); }
"""


_HOT_PROGRAM = """
fn helper() -> u8 { return 1; }
fn main() -> u8 {
  let s = 0;
  let i = 0;
  while i < 5 {
    s = s + helper();
    i = i + 1;
  }
  return s;
}
"""


# ---------------------------------------------------------------------------
# hot_functions — same set of names + counts at threshold=1
# ---------------------------------------------------------------------------


def test_hot_functions_returns_list_of_str() -> None:
    """Shape: ``hot_functions(threshold)`` returns ``list[str]``."""
    runtime = TetradRuntime()
    runtime.run(_HOT_PROGRAM)
    hot = runtime.hot_functions(threshold=1)
    assert isinstance(hot, list)
    assert all(isinstance(name, str) for name in hot)
    # ``helper`` is called 5 times → meets threshold=1.
    assert "helper" in hot


def test_hot_functions_threshold_filters_correctly() -> None:
    """Threshold filtering matches legacy semantics: count >= threshold."""
    runtime = TetradRuntime()
    runtime.run(_HOT_PROGRAM)
    # helper called 5 times, so threshold > 5 excludes it.
    assert "helper" not in runtime.hot_functions(threshold=10)
    # Threshold of 5 includes it (5 >= 5).
    assert "helper" in runtime.hot_functions(threshold=5)


def test_legacy_runtime_imports_for_smoke() -> None:
    """Importing TetradVM and running a Tetrad program does not crash —
    a smoke check that the legacy runtime is still functional alongside
    TetradRuntime."""
    legacy = TetradVM()
    code = compile_program(_HOT_PROGRAM)
    # Legacy execute runs the top-level (HALT) — does not raise.
    legacy.execute(code)


# ---------------------------------------------------------------------------
# loop_iterations — both should report the loop ran 5 times
# ---------------------------------------------------------------------------


def test_loop_iterations_returns_dict_int_int() -> None:
    """Shape: ``loop_iterations(fn)`` returns ``dict[int, int]``."""
    runtime = TetradRuntime()
    runtime.run(_LOOP_PROGRAM)
    iters = runtime.loop_iterations("count")
    assert isinstance(iters, dict)
    assert all(isinstance(k, int) for k in iters)
    assert all(isinstance(v, int) for v in iters.values())


def test_loop_iterations_count_matches_n() -> None:
    """A ``while i < n { i+1 }`` loop with n=5 should fire its
    back-edge 5 times in the new runtime."""
    runtime = TetradRuntime()
    runtime.run(_LOOP_PROGRAM)
    total = sum(runtime.loop_iterations("count").values())
    assert total == 5


# ---------------------------------------------------------------------------
# branch_profile — totals at the conditional branch agree
# ---------------------------------------------------------------------------


def test_branch_profile_returns_branchstats_or_none() -> None:
    """Shape: ``branch_profile(fn, tetrad_ip)`` returns
    ``BranchStats | None``.

    We discover a valid Tetrad IP by walking the IIR module's
    source_map; querying that IP returns BranchStats; querying an
    invalid IP returns None.
    """
    runtime = TetradRuntime()
    runtime.run(_BRANCH_PROGRAM)

    pick_fn = runtime.last_module.get_function("pick")
    assert pick_fn is not None
    # Find a Tetrad IP for which the IIR translation contains a
    # ``jmp_if_*`` instruction; that's the IP whose branch_profile
    # should resolve.
    tetrad_branch_ips = []
    for iir_start, tetrad_ip, _ in pick_fn.source_map:
        # Find the next iir_start (or end of instrs) and scan within range.
        next_starts = [
            s for s, _, _ in pick_fn.source_map if s > iir_start
        ]
        upper = min(next_starts) if next_starts else len(pick_fn.instructions)
        for ip in range(iir_start, upper):
            if pick_fn.instructions[ip].op in ("jmp_if_true", "jmp_if_false"):
                tetrad_branch_ips.append(tetrad_ip)
                break

    assert tetrad_branch_ips, "pick should contain at least one branch"
    # First branch in pick should resolve to BranchStats.
    bs = runtime.branch_profile("pick", tetrad_branch_ips[0])
    assert bs is not None
    assert bs.taken_count + bs.not_taken_count >= 1

    # An IP that doesn't match any source-map entry returns None.
    assert runtime.branch_profile("pick", 99999) is None


# ---------------------------------------------------------------------------
# feedback_vector / type_profile — legacy SlotKind shape preserved
# ---------------------------------------------------------------------------


# A program with untyped operations — Tetrad's compiler allocates
# feedback slots for arithmetic on values whose static type is not u8.
# Using an untyped function parameter forces that path.
_UNTYPED_PROGRAM = """
fn untyped_sum(n) -> u8 {
  let acc = 0;
  let i = 0;
  while i < n {
    acc = acc + i;
    i = i + 1;
  }
  return acc;
}
fn main() -> u8 { return untyped_sum(3); }
"""


def test_feedback_vector_returns_slotstate_list_shape() -> None:
    """``feedback_vector(fn)`` returns ``list[SlotState] | None`` —
    shape parity check.  Uses an untyped function so the compiler
    actually allocates feedback slots."""
    runtime = TetradRuntime()
    runtime.run(_UNTYPED_PROGRAM)

    fv = runtime.feedback_vector("untyped_sum")
    assert fv is not None
    assert len(fv) > 0
    for state in fv:
        assert hasattr(state, "kind")
        assert hasattr(state, "observations")
        assert hasattr(state, "count")


def test_feedback_vector_returns_none_for_unknown_fn() -> None:
    runtime = TetradRuntime()
    runtime.run(_UNTYPED_PROGRAM)
    assert runtime.feedback_vector("not_a_function") is None


def test_feedback_vector_returns_none_for_fully_typed_fn() -> None:
    """Fully-typed functions allocate no slots; ``feedback_vector``
    returns ``None`` (matching legacy ``TetradVM`` behaviour for
    functions with ``feedback_slot_count == 0``)."""
    runtime = TetradRuntime()
    runtime.run(_LOOP_PROGRAM)  # ``count`` is fully typed
    assert runtime.feedback_vector("count") is None


def test_type_profile_indexes_into_feedback_vector() -> None:
    """``type_profile(fn, i)`` returns the same data as ``feedback_vector(fn)[i]``.

    Compares by equality (not identity) — for never-observed slots,
    each call produces a fresh ``SlotState()`` object.  Observed slots
    point at the live ``IIRInstr.observed_slot``, so they are
    identity-shared.
    """
    runtime = TetradRuntime()
    runtime.run(_UNTYPED_PROGRAM)
    fv = runtime.feedback_vector("untyped_sum")
    assert fv is not None
    p = runtime.type_profile("untyped_sum", 0)
    assert p == fv[0]


def test_type_profile_out_of_range_returns_none() -> None:
    runtime = TetradRuntime()
    runtime.run(_UNTYPED_PROGRAM)
    assert runtime.type_profile("untyped_sum", 9999) is None


def test_call_site_shape_returns_uninitialized_for_unknown() -> None:
    runtime = TetradRuntime()
    runtime.run(_LOOP_PROGRAM)
    # Asking for a function that doesn't exist returns UNINITIALIZED.
    assert runtime.call_site_shape("nope", 0) is SlotKind.UNINITIALIZED


# ---------------------------------------------------------------------------
# reset_metrics — clears the live counters
# ---------------------------------------------------------------------------


def test_reset_metrics_clears_state() -> None:
    runtime = TetradRuntime()
    runtime.run(_LOOP_PROGRAM)
    assert runtime.loop_iterations("count") != {}

    runtime.reset_metrics()
    assert runtime.loop_iterations("count") == {}
    assert runtime.hot_functions(threshold=1) == []


# ---------------------------------------------------------------------------
# execute_traced — produces a list of VMTrace records
# ---------------------------------------------------------------------------


def test_execute_traced_returns_traces() -> None:
    runtime = TetradRuntime()
    result, traces = runtime.execute_traced(
        "fn main() -> u8 { return 7; }"
    )
    assert result == 7
    assert len(traces) > 0
    # Every trace has the standard VMTrace shape.
    for t in traces:
        assert hasattr(t, "fn_name")
        assert hasattr(t, "ip")
        assert hasattr(t, "registers_before")
        assert hasattr(t, "registers_after")


# ---------------------------------------------------------------------------
# unrun runtime gracefully returns empty / None
# ---------------------------------------------------------------------------


def test_metrics_on_unrun_runtime_are_safe_defaults() -> None:
    """Before any ``run()``, all metric APIs return the legacy defaults."""
    runtime = TetradRuntime()
    assert runtime.hot_functions() == []
    assert runtime.feedback_vector("anything") is None
    assert runtime.type_profile("anything", 0) is None
    assert runtime.call_site_shape("anything", 0) is SlotKind.UNINITIALIZED
    assert runtime.branch_profile("anything", 0) is None
    assert runtime.loop_iterations("anything") == {}
    # reset_metrics is a no-op (does not raise).
    runtime.reset_metrics()
