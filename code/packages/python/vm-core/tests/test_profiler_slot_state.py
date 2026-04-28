"""Integration tests for the VMProfiler → SlotState pipeline (LANG17).

These exercise the profiler through a real VMCore dispatch, confirming
that:

1. Dynamically-typed instructions gain an ``observed_slot`` that walks
   the state machine as values flow through.
2. Statically-typed instructions are still skipped (zero profiler cost
   for fully-typed programs).
3. A custom ``type_mapper`` is honoured — critical for any language
   whose runtime values are not Python primitives.
"""

from __future__ import annotations

from interpreter_ir import (
    IIRFunction,
    IIRInstr,
    IIRModule,
    SlotKind,
)

from vm_core import VMCore, default_type_mapper


def _single_fn(
    name: str,
    instrs: list[IIRInstr],
    *,
    params: list[tuple[str, str]] | None = None,
    return_type: str = "any",
) -> IIRModule:
    """Build a module with one function for brevity in tests."""
    fn = IIRFunction(
        name=name,
        params=params or [],
        return_type=return_type,
        instructions=instrs,
    )
    return IIRModule(name="test", functions=[fn], entry_point=name)


# ---------------------------------------------------------------------------
# Untyped programs — slot walks the state machine
# ---------------------------------------------------------------------------


def test_untyped_monomorphic_run_populates_slot() -> None:
    """Repeatedly calling an untyped function with one type keeps the
    slot MONOMORPHIC and dominant_type() returns the type."""
    vm = VMCore()
    module = _single_fn(
        "add_any",
        [
            IIRInstr("add", "t", ["a", "b"], "any"),
            IIRInstr("ret", None, ["t"], "any"),
        ],
        params=[("a", "any"), ("b", "any")],
    )
    for _ in range(3):
        result = vm.execute(module, fn="add_any", args=[3, 4])
        assert result == 7

    add_instr = module.functions[0].instructions[0]
    assert add_instr.observed_slot is not None
    assert add_instr.observed_slot.kind is SlotKind.MONOMORPHIC
    assert add_instr.observed_slot.dominant_type() == "u8"
    assert add_instr.observation_count == 3


def test_untyped_polymorphic_then_megamorphic() -> None:
    """Feeding five different result types drives the slot to MEGAMORPHIC."""
    vm = VMCore()
    # Use a call-builtin that returns whatever value the host supplies.
    results = [1, 1000, 1_000_000, 1.5, "hello"]
    # Five types: u8, u16, u32, f64, str

    def echo_host(args):
        return args[0]

    vm.register_builtin("echo", echo_host)

    module = _single_fn(
        "call_echo",
        [
            IIRInstr("call_builtin", "v", ["echo", "x"], "any"),
            IIRInstr("ret", None, ["v"], "any"),
        ],
        params=[("x", "any")],
    )
    for r in results:
        vm.execute(module, fn="call_echo", args=[r])

    echo_instr = module.functions[0].instructions[0]
    # Five distinct types → MEGAMORPHIC, observations discarded.
    assert echo_instr.observed_slot is not None
    assert echo_instr.observed_slot.kind is SlotKind.MEGAMORPHIC
    assert echo_instr.observed_slot.observations == []
    assert echo_instr.observation_count == 5
    # Legacy view still reports polymorphic — back-compat preserved.
    assert echo_instr.observed_type == "polymorphic"


# ---------------------------------------------------------------------------
# Typed programs — profiler skip path
# ---------------------------------------------------------------------------


def test_typed_instruction_is_not_profiled() -> None:
    """A ``type_hint`` other than ``"any"`` means the profiler does nothing —
    fully-typed programs pay zero overhead."""
    vm = VMCore()
    module = _single_fn(
        "add_typed",
        [
            IIRInstr("add", "t", ["a", "b"], "u8"),
            IIRInstr("ret", None, ["t"], "u8"),
        ],
        params=[("a", "u8"), ("b", "u8")],
        return_type="u8",
    )
    for _ in range(10):
        vm.execute(module, fn="add_typed", args=[1, 2])

    add_instr = module.functions[0].instructions[0]
    # Slot remains untouched because type_hint is concrete.
    assert add_instr.observed_slot is None
    assert add_instr.observed_type is None
    assert add_instr.observation_count == 0


def test_profiler_can_be_disabled() -> None:
    """``profiler_enabled=False`` silences the profiler for every op."""
    vm = VMCore(profiler_enabled=False)
    module = _single_fn(
        "add_any",
        [
            IIRInstr("add", "t", ["a", "b"], "any"),
            IIRInstr("ret", None, ["t"], "any"),
        ],
        params=[("a", "any"), ("b", "any")],
    )
    vm.execute(module, fn="add_any", args=[1, 2])

    add_instr = module.functions[0].instructions[0]
    assert add_instr.observed_slot is None


# ---------------------------------------------------------------------------
# Pluggable type_mapper — the critical generic-language contract
# ---------------------------------------------------------------------------


def test_custom_type_mapper_is_used() -> None:
    """A custom ``type_mapper`` replaces the default primitive classifier.

    Simulates a Lisp-like frontend where every integer is classified as
    ``"fixnum"`` and every string as ``"symbol"``.  The state machine
    observes these symbolic names, not the default ``"u8"`` / ``"str"``.
    """

    def lisp_mapper(value):
        if isinstance(value, bool):
            return "bool"
        if isinstance(value, int):
            return "fixnum"
        if isinstance(value, str):
            return "symbol"
        return "any"

    vm = VMCore(type_mapper=lisp_mapper)

    def echo_host(args):
        return args[0]
    vm.register_builtin("echo", echo_host)

    module = _single_fn(
        "call_echo",
        [
            IIRInstr("call_builtin", "v", ["echo", "x"], "any"),
            IIRInstr("ret", None, ["v"], "any"),
        ],
        params=[("x", "any")],
    )
    vm.execute(module, fn="call_echo", args=[42])
    vm.execute(module, fn="call_echo", args=[7])

    echo_instr = module.functions[0].instructions[0]
    assert echo_instr.observed_slot is not None
    assert echo_instr.observed_slot.kind is SlotKind.MONOMORPHIC
    assert echo_instr.observed_slot.observations == ["fixnum"]

    # Feed a string → distinct type in Lisp-mapper's scheme.
    vm.execute(module, fn="call_echo", args=["hello"])
    assert echo_instr.observed_slot.kind is SlotKind.POLYMORPHIC
    assert echo_instr.observed_slot.observations == ["fixnum", "symbol"]


def test_vm_exposes_active_type_mapper_via_profiler() -> None:
    """The profiler's ``type_mapper`` property is the active one."""
    vm = VMCore()
    assert vm.profiler.type_mapper is default_type_mapper

    custom = lambda _v: "always_any"  # noqa: E731
    vm2 = VMCore(type_mapper=custom)
    assert vm2.profiler.type_mapper is custom


# ---------------------------------------------------------------------------
# Partially-typed programs — profiler only touches "any" slots
# ---------------------------------------------------------------------------


def test_partially_typed_mixed_observation() -> None:
    """In a function with one concrete-typed and one ``"any"`` op, only
    the ``"any"`` op gains an observed_slot."""
    vm = VMCore()

    def echo_host(args):
        return args[0]
    vm.register_builtin("echo", echo_host)

    module = _single_fn(
        "mix",
        [
            # Typed: should NOT be profiled.
            IIRInstr("const", "c", [42], "u8"),
            # Untyped: should be profiled.
            IIRInstr("call_builtin", "v", ["echo", "x"], "any"),
            IIRInstr("ret", None, ["v"], "any"),
        ],
        params=[("x", "any")],
    )
    vm.execute(module, fn="mix", args=[7])

    typed_instr = module.functions[0].instructions[0]
    any_instr = module.functions[0].instructions[1]

    assert typed_instr.observed_slot is None
    assert any_instr.observed_slot is not None
    assert any_instr.observed_slot.kind is SlotKind.MONOMORPHIC
    assert any_instr.observed_slot.dominant_type() == "u8"
