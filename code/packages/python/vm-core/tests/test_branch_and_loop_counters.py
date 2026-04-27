"""LANG17 PR2 — branch stats and loop iteration counters.

These tests drive real IIR programs through the dispatch loop and
verify that the per-site branch and back-edge counters land in the
right places.  Separating this file from the existing
``test_control_flow.py`` makes the LANG17 additions easy to find in
isolation.
"""

from __future__ import annotations

from interpreter_ir import IIRFunction, IIRInstr, IIRModule

from vm_core import BranchStats, VMCore


def _wrap(name: str, instrs: list[IIRInstr], *,
          params: list[tuple[str, str]] | None = None) -> IIRModule:
    fn = IIRFunction(
        name=name,
        params=params or [],
        return_type="any",
        instructions=instrs,
    )
    return IIRModule(name="test", functions=[fn], entry_point=name)


# ---------------------------------------------------------------------------
# BranchStats basics
# ---------------------------------------------------------------------------


def test_branch_stats_defaults_are_zero() -> None:
    stats = BranchStats()
    assert stats.taken_count == 0
    assert stats.not_taken_count == 0
    assert stats.taken_ratio == 0.0
    assert stats.total == 0


def test_branch_stats_taken_ratio_and_total() -> None:
    stats = BranchStats(taken_count=3, not_taken_count=1)
    assert stats.total == 4
    assert stats.taken_ratio == 0.75


# ---------------------------------------------------------------------------
# jmp_if_true — taken and not-taken arms counted separately
# ---------------------------------------------------------------------------


def test_jmp_if_true_counts_taken() -> None:
    """Pass ``True`` → branch is taken; ``False`` → not taken."""
    vm = VMCore()

    # Build a module with a jmp_if_true that either jumps to ``end`` or
    # falls through to another ``ret``.
    module = _wrap(
        "fn",
        [
            IIRInstr("jmp_if_true", None, ["c", "end"], "any"),  # ip 0
            IIRInstr("const", "fallthrough", [1], "u8"),         # ip 1
            IIRInstr("ret", None, ["fallthrough"], "any"),        # ip 2
            IIRInstr("label", None, ["end"], "any"),              # ip 3
            IIRInstr("const", "taken", [2], "u8"),                # ip 4
            IIRInstr("ret", None, ["taken"], "any"),              # ip 5
        ],
        params=[("c", "any")],
    )

    # True twice, False once: taken_count=2, not_taken_count=1.
    vm.execute(module, fn="fn", args=[True])
    vm.execute(module, fn="fn", args=[True])
    vm.execute(module, fn="fn", args=[False])

    stats = vm.branch_profile("fn", source_ip=0)
    assert stats is not None
    assert stats.taken_count == 2
    assert stats.not_taken_count == 1
    assert stats.taken_ratio == 2 / 3


def test_jmp_if_false_counts_taken() -> None:
    """For jmp_if_false, the branch is "taken" when the condition is False."""
    vm = VMCore()
    module = _wrap(
        "fn",
        [
            IIRInstr("jmp_if_false", None, ["c", "skip"], "any"),  # ip 0
            IIRInstr("const", "a", [11], "u8"),                    # ip 1
            IIRInstr("ret", None, ["a"], "any"),                    # ip 2
            IIRInstr("label", None, ["skip"], "any"),               # ip 3
            IIRInstr("const", "b", [22], "u8"),                     # ip 4
            IIRInstr("ret", None, ["b"], "any"),                    # ip 5
        ],
        params=[("c", "any")],
    )
    # Passing ``False`` → branch taken; ``True`` → not taken.
    vm.execute(module, fn="fn", args=[False])
    vm.execute(module, fn="fn", args=[False])
    vm.execute(module, fn="fn", args=[True])
    vm.execute(module, fn="fn", args=[True])

    stats = vm.branch_profile("fn", source_ip=0)
    assert stats is not None
    assert stats.taken_count == 2
    assert stats.not_taken_count == 2
    assert stats.taken_ratio == 0.5


def test_unreached_branch_returns_none() -> None:
    """A function that never executes its branch leaves no counters."""
    vm = VMCore()
    module = _wrap("fn", [IIRInstr("ret_void", None, [], "void")])
    vm.execute(module, fn="fn")
    assert vm.branch_profile("fn", source_ip=0) is None


# ---------------------------------------------------------------------------
# Unconditional jmp — not a branch; only contributes to back-edge counts
# ---------------------------------------------------------------------------


def test_unconditional_jmp_does_not_create_branch_stats() -> None:
    """Forward ``jmp`` is counted for neither branch nor loop stats."""
    vm = VMCore()
    module = _wrap(
        "fn",
        [
            IIRInstr("jmp", None, ["end"], "any"),           # ip 0 — forward
            IIRInstr("const", "x", [99], "u8"),               # ip 1 — skipped
            IIRInstr("label", None, ["end"], "any"),          # ip 2
            IIRInstr("const", "y", [7], "u8"),                # ip 3
            IIRInstr("ret", None, ["y"], "any"),              # ip 4
        ],
    )
    vm.execute(module, fn="fn")
    # No branch stats for unconditional jumps.
    assert vm.branch_profile("fn", source_ip=0) is None
    # No back-edge either (forward jump).
    assert vm.loop_iterations("fn") == {}


# ---------------------------------------------------------------------------
# Back-edge counts — loops
# ---------------------------------------------------------------------------


def test_back_edge_counts_iteration_of_while_loop() -> None:
    """A while-loop pattern: body + ``jmp back_to_cond`` back-edge."""
    vm = VMCore()
    # Emulates:  while (i < 3) { i = i + 1; }
    # Source layout (index: instruction):
    #   0: label cond_head
    #   1: cmp_lt t, i, 3
    #   2: jmp_if_false t, cond_exit     (exit when t==False)
    #   3: add i, i, 1
    #   4: jmp cond_head                 (back-edge)
    #   5: label cond_exit
    #   6: ret i
    module = _wrap(
        "loop",
        [
            IIRInstr("label", None, ["cond_head"], "any"),              # 0
            IIRInstr("cmp_lt", "t", ["i", 3], "bool"),                   # 1
            IIRInstr("jmp_if_false", None, ["t", "cond_exit"], "any"),   # 2
            IIRInstr("add", "i", ["i", 1], "u8"),                         # 3
            IIRInstr("jmp", None, ["cond_head"], "any"),                 # 4 back-edge
            IIRInstr("label", None, ["cond_exit"], "any"),                # 5
            IIRInstr("ret", None, ["i"], "any"),                          # 6
        ],
        params=[("i", "any")],
    )
    result = vm.execute(module, fn="loop", args=[0])
    assert result == 3

    # The back-edge at ip=4 fires three times (i=0→1, 1→2, 2→3).
    iter_counts = vm.loop_iterations("loop")
    assert iter_counts == {4: 3}

    # The conditional branch at ip=2 runs 4 times total:
    #   i=0,1,2  → not taken (continue loop)
    #   i=3      → taken (exit loop)
    branch = vm.branch_profile("loop", source_ip=2)
    assert branch is not None
    assert branch.taken_count == 1      # the exit
    assert branch.not_taken_count == 3  # three continues


def test_back_edge_counts_on_conditional_backward_jump() -> None:
    """A taken backward ``jmp_if_true`` counts *both* as a branch hit and
    as a back-edge traversal."""
    vm = VMCore()
    # Counts down from 3 using a conditional backward jump.
    module = _wrap(
        "countdown",
        [
            IIRInstr("label", None, ["top"], "any"),                  # 0
            IIRInstr("sub", "n", ["n", 1], "u8"),                      # 1
            IIRInstr("cmp_gt", "cond", ["n", 0], "bool"),              # 2
            IIRInstr("jmp_if_true", None, ["cond", "top"], "any"),     # 3 ← back-edge
            IIRInstr("ret", None, ["n"], "any"),                       # 4
        ],
        params=[("n", "any")],
    )
    result = vm.execute(module, fn="countdown", args=[3])
    assert result == 0

    # Branch at ip=3 fires 3 times: two taken (n=2, n=1) + one not-taken (n=0).
    branch = vm.branch_profile("countdown", source_ip=3)
    assert branch is not None
    assert branch.taken_count == 2
    assert branch.not_taken_count == 1

    # Back-edge fires only on the taken arms → 2 hits.
    assert vm.loop_iterations("countdown") == {3: 2}


# ---------------------------------------------------------------------------
# metrics() snapshot carries branch + loop stats
# ---------------------------------------------------------------------------


def test_metrics_snapshot_deep_copies_branch_stats() -> None:
    """Mutating the snapshot must not affect the live VM state."""
    vm = VMCore()
    module = _wrap(
        "fn",
        [
            IIRInstr("jmp_if_true", None, ["c", "end"], "any"),
            IIRInstr("ret_void", None, [], "void"),
            IIRInstr("label", None, ["end"], "any"),
            IIRInstr("ret_void", None, [], "void"),
        ],
        params=[("c", "any")],
    )
    vm.execute(module, fn="fn", args=[True])

    snap = vm.metrics()
    assert snap.branch_stats["fn"][0].taken_count == 1

    # Mutate the snapshot.
    snap.branch_stats["fn"][0].taken_count = 999

    # Live state unchanged.
    assert vm.branch_profile("fn", 0).taken_count == 1


# ---------------------------------------------------------------------------
# hot_functions helper
# ---------------------------------------------------------------------------


def test_hot_functions_threshold() -> None:
    vm = VMCore()
    module = _wrap("hot_fn", [IIRInstr("ret_void", None, [], "void")])
    for _ in range(5):
        vm.execute(module, fn="hot_fn")

    assert vm.hot_functions(threshold=5) == ["hot_fn"]
    assert vm.hot_functions(threshold=6) == []
    assert vm.hot_functions(threshold=1) == ["hot_fn"]


# ---------------------------------------------------------------------------
# reset_metrics wipes the counters (but leaves per-IIRInstr state alone)
# ---------------------------------------------------------------------------


def test_reset_metrics_clears_branch_and_loop_state() -> None:
    vm = VMCore()
    module = _wrap(
        "fn",
        [
            IIRInstr("label", None, ["top"], "any"),
            IIRInstr("sub", "n", ["n", 1], "u8"),
            IIRInstr("cmp_gt", "cond", ["n", 0], "bool"),
            IIRInstr("jmp_if_true", None, ["cond", "top"], "any"),
            IIRInstr("ret_void", None, [], "void"),
        ],
        params=[("n", "any")],
    )
    vm.execute(module, fn="fn", args=[5])

    assert vm.loop_iterations("fn") != {}
    assert vm.branch_profile("fn", 3) is not None

    vm.reset_metrics()

    assert vm.loop_iterations("fn") == {}
    assert vm.branch_profile("fn", 3) is None
    snap = vm.metrics()
    assert snap.branch_stats == {}
    assert snap.loop_back_edge_counts == {}
    assert snap.function_call_counts == {}
    assert snap.total_instructions_executed == 0


# ---------------------------------------------------------------------------
# Language-agnosticism — counters work on any language's IIR
# ---------------------------------------------------------------------------


def test_works_for_fully_untyped_function() -> None:
    """Back-edge detection is purely structural — type hints don't matter."""
    vm = VMCore()
    module = _wrap(
        "fn",
        [
            IIRInstr("label", None, ["loop"], "any"),
            IIRInstr("sub", "i", ["i", 1], "any"),        # "any" types
            IIRInstr("cmp_gt", "c", ["i", 0], "any"),
            IIRInstr("jmp_if_true", None, ["c", "loop"], "any"),
            IIRInstr("ret_void", None, [], "void"),
        ],
        params=[("i", "any")],
    )
    vm.execute(module, fn="fn", args=[2])
    # Back-edge fires once (i=2→1 is the only "continue", then exits at i=0).
    assert vm.loop_iterations("fn") == {3: 1}
