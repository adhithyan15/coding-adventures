"""LANG17 PR3 — ``VMCore.execute_traced`` and the ``VMTracer`` / ``VMTrace`` pair.

These tests exercise the opt-in per-instruction trace path.  They
verify:

1. The normal ``execute`` path never allocates a trace (zero cost).
2. ``execute_traced`` returns one ``VMTrace`` per dispatched instruction.
3. Register snapshots accurately reflect the state *before* and
   *after* each instruction.
4. Slot-delta records capture profiler observations as they happen.
5. Function calls produce traces at the right ``frame_depth``.
6. The ``VMTrace`` payloads are language-agnostic — no Tetrad-specific
   assumptions about register counts, type names, or instruction
   layout.
"""

from __future__ import annotations

from interpreter_ir import IIRFunction, IIRInstr, IIRModule

from vm_core import VMCore, VMTrace, VMTracer


def _one_fn(name: str, instrs: list[IIRInstr], *,
            params: list[tuple[str, str]] | None = None,
            return_type: str = "any") -> IIRModule:
    fn = IIRFunction(
        name=name,
        params=params or [],
        return_type=return_type,
        instructions=instrs,
    )
    return IIRModule(name="test", functions=[fn], entry_point=name)


# ---------------------------------------------------------------------------
# Normal execute path pays zero tracing cost
# ---------------------------------------------------------------------------


def test_normal_execute_does_not_populate_tracer() -> None:
    vm = VMCore()
    module = _one_fn(
        "fn",
        [
            IIRInstr("const", "x", [42], "u8"),
            IIRInstr("ret", None, ["x"], "u8"),
        ],
    )
    result = vm.execute(module, fn="fn")
    assert result == 42
    assert vm._tracer is None


# ---------------------------------------------------------------------------
# execute_traced returns (result, traces) and produces one entry per instr
# ---------------------------------------------------------------------------


def test_execute_traced_returns_result_and_one_trace_per_instr() -> None:
    vm = VMCore()
    module = _one_fn(
        "fn",
        [
            IIRInstr("const", "x", [1], "u8"),
            IIRInstr("const", "y", [2], "u8"),
            IIRInstr("add", "z", ["x", "y"], "u8"),
            IIRInstr("ret", None, ["z"], "u8"),
        ],
    )
    result, traces = vm.execute_traced(module, fn="fn")
    assert result == 3
    # Four instructions dispatched → four trace entries.
    assert len(traces) == 4
    # Ordering preserved.
    assert [t.instr.op for t in traces] == ["const", "const", "add", "ret"]


def test_trace_captures_ip_and_fn_name() -> None:
    vm = VMCore()
    module = _one_fn(
        "demo",
        [
            IIRInstr("const", "x", [5], "u8"),
            IIRInstr("ret", None, ["x"], "u8"),
        ],
    )
    _, traces = vm.execute_traced(module, fn="demo")
    assert all(t.fn_name == "demo" for t in traces)
    # IP is the pre-dispatch index.
    assert [t.ip for t in traces] == [0, 1]


def test_trace_instr_is_same_object_as_module() -> None:
    """``VMTrace.instr`` is a reference — consumers can cross-reference
    back to the module."""
    vm = VMCore()
    module = _one_fn(
        "fn",
        [IIRInstr("const", "x", [7], "u8"), IIRInstr("ret", None, ["x"], "u8")],
    )
    _, traces = vm.execute_traced(module, fn="fn")
    assert traces[0].instr is module.functions[0].instructions[0]


# ---------------------------------------------------------------------------
# Register snapshots
# ---------------------------------------------------------------------------


def test_register_snapshots_show_before_and_after() -> None:
    """After ``const x, 42``, the register holding ``x`` is 42."""
    vm = VMCore()
    module = _one_fn(
        "fn",
        [
            IIRInstr("const", "x", [42], "u8"),
            IIRInstr("ret", None, ["x"], "u8"),
        ],
    )
    _, traces = vm.execute_traced(module, fn="fn")
    # Before the const fires, the register is zero.
    assert 42 not in traces[0].registers_before
    # After, 42 appears in the register file.
    assert 42 in traces[0].registers_after


def test_register_snapshots_are_copies_not_live_views() -> None:
    """Mutating a snapshot's list must not affect future traces."""
    vm = VMCore()
    module = _one_fn(
        "fn",
        [
            IIRInstr("const", "x", [1], "u8"),
            IIRInstr("const", "y", [2], "u8"),
            IIRInstr("ret", None, ["y"], "u8"),
        ],
    )
    _, traces = vm.execute_traced(module, fn="fn")
    # Mutating the first trace's register snapshot must not change the
    # second trace's before/after (they are independent copies).
    traces[0].registers_after.append(999)
    assert 999 not in traces[1].registers_before


# ---------------------------------------------------------------------------
# Slot-delta captures profiler observations
# ---------------------------------------------------------------------------


def test_slot_delta_populated_when_profiler_observes() -> None:
    """An ``"any"``-typed instruction produces a slot delta on the trace
    of the instruction that created it."""
    vm = VMCore()
    module = _one_fn(
        "fn",
        [
            IIRInstr("const", "x", [42], "any"),   # profiler observes x's result
            IIRInstr("ret", None, ["x"], "any"),
        ],
    )
    _, traces = vm.execute_traced(module, fn="fn")

    # The const instruction should have observed its result "u8".
    const_trace = traces[0]
    assert len(const_trace.slot_delta) == 1
    idx, slot = const_trace.slot_delta[0]
    assert idx == 0  # the const's own IP
    assert slot.observations == ["u8"]


def test_slot_delta_empty_for_typed_instructions() -> None:
    """Concrete-typed instructions are not profiled — no slot delta."""
    vm = VMCore()
    module = _one_fn(
        "fn",
        [
            IIRInstr("const", "x", [42], "u8"),  # typed — profiler skips
            IIRInstr("ret", None, ["x"], "u8"),
        ],
    )
    _, traces = vm.execute_traced(module, fn="fn")
    assert traces[0].slot_delta == []


def test_slot_delta_snapshot_is_isolated_from_future_observations() -> None:
    """Traces retain the slot state *at trace time* — later observations
    do not mutate earlier trace entries."""
    vm = VMCore()
    module = _one_fn(
        "fn",
        [
            IIRInstr("const", "x", [42], "any"),
            IIRInstr("ret", None, ["x"], "any"),
        ],
    )
    # First run — slot is MONOMORPHIC after.
    _, traces_run1 = vm.execute_traced(module, fn="fn")
    assert len(traces_run1[0].slot_delta) == 1
    _, slot_at_run1 = traces_run1[0].slot_delta[0]
    assert slot_at_run1.count == 1

    # Second run — the live slot advances to count=2, but the snapshot
    # from run 1 still shows count=1.
    vm.execute_traced(module, fn="fn")
    assert slot_at_run1.count == 1


# ---------------------------------------------------------------------------
# Function calls cross frames
# ---------------------------------------------------------------------------


def test_traces_follow_calls_into_called_functions() -> None:
    """A call produces traces for the callee's instructions at
    ``frame_depth > 0``."""
    vm = VMCore()
    callee = IIRFunction(
        name="double",
        params=[("n", "any")],
        return_type="any",
        instructions=[
            IIRInstr("add", "r", ["n", "n"], "any"),
            IIRInstr("ret", None, ["r"], "any"),
        ],
    )
    caller = IIRFunction(
        name="entry",
        params=[],
        return_type="any",
        instructions=[
            IIRInstr("call", "t", ["double", 5], "any"),
            IIRInstr("ret", None, ["t"], "any"),
        ],
    )
    module = IIRModule(name="m", functions=[caller, callee], entry_point="entry")

    result, traces = vm.execute_traced(module, fn="entry")
    assert result == 10

    # Caller should have a trace at depth 0; callee's instructions at depth 1.
    by_fn = {t.fn_name for t in traces}
    assert by_fn == {"entry", "double"}

    callee_traces = [t for t in traces if t.fn_name == "double"]
    # Both callee instructions traced.
    assert [t.instr.op for t in callee_traces] == ["add", "ret"]
    # Callee's frame depth is deeper than caller's.
    caller_traces = [t for t in traces if t.fn_name == "entry"]
    assert all(t.frame_depth > caller_traces[0].frame_depth for t in callee_traces)


# ---------------------------------------------------------------------------
# VMTracer helper
# ---------------------------------------------------------------------------


def test_vm_tracer_len_grows_with_traces() -> None:
    tracer = VMTracer()
    assert len(tracer) == 0
    vm = VMCore()
    vm._tracer = tracer
    try:
        module = _one_fn(
            "fn",
            [IIRInstr("const", "x", [1], "u8"), IIRInstr("ret", None, ["x"], "u8")],
        )
        vm.execute(module, fn="fn")
    finally:
        vm._tracer = None
    assert len(tracer) == 2


def test_vm_trace_dataclass_defaults() -> None:
    """VMTrace's ``slot_delta`` defaults to an empty list."""
    # Construct directly for API coverage.
    trace = VMTrace(
        frame_depth=0,
        fn_name="x",
        ip=5,
        instr=IIRInstr("const", "y", [1], "u8"),
        registers_before=[0, 0],
        registers_after=[1, 0],
    )
    assert trace.slot_delta == []
    assert trace.frame_depth == 0
    assert trace.ip == 5


# ---------------------------------------------------------------------------
# Language-agnosticism — tracing works for non-primitive runtime values
# ---------------------------------------------------------------------------


def test_tracing_works_with_non_primitive_values() -> None:
    """Register snapshots hold whatever the frontend's runtime values
    are — no assumption about primitiveness."""
    class LispCons:
        def __init__(self, car, cdr):
            self.car, self.cdr = car, cdr

    def lisp_mapper(v):
        return "cons" if isinstance(v, LispCons) else "any"

    vm = VMCore(type_mapper=lisp_mapper)
    cell = LispCons(1, None)

    def build_cons(args):
        return cell
    vm.register_builtin("cons", build_cons)

    module = _one_fn(
        "fn",
        [
            IIRInstr("call_builtin", "c", ["cons"], "any"),
            IIRInstr("ret", None, ["c"], "any"),
        ],
    )
    result, traces = vm.execute_traced(module, fn="fn")
    assert result is cell
    # The register snapshot preserves the LispCons object identity.
    assert cell in traces[0].registers_after
    # The profiler ran the Lisp mapper and observed "cons".
    const_delta = traces[0].slot_delta
    assert len(const_delta) == 1
    _, slot = const_delta[0]
    assert slot.observations == ["cons"]
