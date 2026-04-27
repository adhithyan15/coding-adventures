"""Tests for the event-driven simulator."""

import pytest

from hardware_vm import Event, HardwareVM
from hdl_ir import (
    HIR,
    BinaryOp,
    Concat,
    ContAssign,
    Direction,
    Module,
    PortRef,
    TyLogic,
    TyVector,
)


# ---- Helpers ----


def make_buffer_hir() -> HIR:
    """Trivial buffer: y = a"""
    m = Module(
        name="buf",
        ports=[
            Port_in_("a", TyLogic()),
            Port_out_("y", TyLogic()),
        ],
        cont_assigns=[ContAssign(target=PortRef("y"), rhs=PortRef("a"))],
    )
    return HIR(top="buf", modules={"buf": m})


def Port_in_(name, ty):
    from hdl_ir import Port

    return Port(name=name, direction=Direction.IN, type=ty)


def Port_out_(name, ty):
    from hdl_ir import Port

    return Port(name=name, direction=Direction.OUT, type=ty)


def make_adder4_hir() -> HIR:
    """4-bit adder: {cout, sum} = a + b + cin."""
    m = Module(
        name="adder4",
        ports=[
            Port_in_("a", TyVector(TyLogic(), 4)),
            Port_in_("b", TyVector(TyLogic(), 4)),
            Port_in_("cin", TyLogic()),
            Port_out_("sum", TyVector(TyLogic(), 4)),
            Port_out_("cout", TyLogic()),
        ],
        cont_assigns=[
            ContAssign(
                target=Concat((PortRef("cout"), PortRef("sum"))),
                rhs=BinaryOp(
                    "+",
                    BinaryOp("+", PortRef("a"), PortRef("b")),
                    PortRef("cin"),
                ),
            )
        ],
    )
    return HIR(top="adder4", modules={"adder4": m})


# ---- Buffer ----


def test_buffer_passes_input_to_output():
    vm = HardwareVM(make_buffer_hir())
    vm.set_input("a", 1)
    assert vm.read("y") == 1
    vm.set_input("a", 0)
    assert vm.read("y") == 0


def test_unset_input_defaults_to_zero():
    vm = HardwareVM(make_buffer_hir())
    assert vm.read("a") == 0
    assert vm.read("y") == 0


# ---- 4-bit adder ----


def test_adder4_zero_plus_zero():
    vm = HardwareVM(make_adder4_hir())
    assert vm.read("sum") == 0
    assert vm.read("cout") == 0


def test_adder4_5_plus_3():
    vm = HardwareVM(make_adder4_hir())
    vm.set_input("a", 5)
    vm.set_input("b", 3)
    vm.set_input("cin", 0)
    assert vm.read("sum") == 8
    assert vm.read("cout") == 0


def test_adder4_carry_out():
    vm = HardwareVM(make_adder4_hir())
    vm.set_input("a", 0xF)
    vm.set_input("b", 0x1)
    vm.set_input("cin", 0)
    assert vm.read("sum") == 0
    assert vm.read("cout") == 1


def test_adder4_with_cin():
    vm = HardwareVM(make_adder4_hir())
    vm.set_input("a", 0xF)
    vm.set_input("b", 0x0)
    vm.set_input("cin", 1)
    assert vm.read("sum") == 0
    assert vm.read("cout") == 1


def test_adder4_exhaustive_carry_pattern():
    vm = HardwareVM(make_adder4_hir())
    for a in range(16):
        for b in range(16):
            for cin in (0, 1):
                vm.set_input("a", a)
                vm.set_input("b", b)
                vm.set_input("cin", cin)
                expected = a + b + cin
                got = (vm.read("cout") << 4) | vm.read("sum")
                assert got == expected, f"a={a} b={b} cin={cin}: got {got}, want {expected}"


# ---- Subscriber events ----


def test_subscribe_emits_events_on_value_change():
    events: list[Event] = []
    vm = HardwareVM(make_buffer_hir())
    vm.subscribe(events.append)
    vm.set_input("a", 1)

    # Should see at least 'a' changing 0->1 and 'y' changing 0->1
    signals = [e.signal for e in events]
    assert "a" in signals
    assert "y" in signals


def test_subscribe_no_event_when_value_unchanged():
    events: list[Event] = []
    vm = HardwareVM(make_buffer_hir())
    vm.subscribe(events.append)
    vm.set_input("a", 0)  # was already 0
    assert events == []


# ---- Force / release ----


def test_force_overrides_normal_driver():
    vm = HardwareVM(make_buffer_hir())
    vm.set_input("a", 1)
    assert vm.read("y") == 1
    vm.force("y", 0)
    assert vm.read("y") == 0
    # Even if input changes, forced value sticks
    vm.set_input("a", 0)
    assert vm.read("y") == 0


def test_release_lets_driver_take_over():
    vm = HardwareVM(make_buffer_hir())
    vm.set_input("a", 1)
    vm.force("y", 0)
    assert vm.read("y") == 0
    vm.release("y")
    assert vm.read("y") == 1


# ---- Errors ----


def test_set_input_on_output_raises():
    vm = HardwareVM(make_buffer_hir())
    with pytest.raises(ValueError, match="cannot set_input"):
        vm.set_input("y", 1)


def test_set_input_on_unknown_signal_raises():
    vm = HardwareVM(make_buffer_hir())
    with pytest.raises(ValueError, match="not a port"):
        vm.set_input("nonexistent", 0)


def test_unknown_top_raises():
    bad = HIR(top="missing", modules={})
    with pytest.raises(ValueError, match="top module"):
        HardwareVM(bad)


# ---- Run ----


def test_run_returns_stats():
    vm = HardwareVM(make_buffer_hir())
    result = vm.run()
    assert result.final_time == 0
    assert result.event_count >= 0


def test_step_returns_false_when_empty_queue():
    vm = HardwareVM(make_buffer_hir())
    assert vm.step() is False


# ---- End-to-end with hdl-elaboration (only if available) ----


def test_end_to_end_via_elaboration():
    """If hdl-elaboration is installed, we can elaborate a Verilog source
    and run it directly. Otherwise skip."""
    try:
        from hdl_elaboration import elaborate_verilog
    except ImportError:
        pytest.skip("hdl-elaboration not installed")

    src = """
    module adder4(input [3:0] a, input [3:0] b, input cin,
                  output [3:0] sum, output cout);
      assign {cout, sum} = a + b + cin;
    endmodule
    """
    hir = elaborate_verilog(src, top="adder4")
    vm = HardwareVM(hir)
    vm.set_input("a", 7)
    vm.set_input("b", 9)
    vm.set_input("cin", 0)
    assert vm.read("sum") == 0  # 16 mod 16
    assert vm.read("cout") == 1
