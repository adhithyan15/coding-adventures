"""Tests for the testbench framework."""

import pytest

from hdl_ir import (
    HIR,
    BinaryOp,
    Concat,
    ContAssign,
    Direction,
    Module,
    Port,
    PortRef,
    TyLogic,
    TyVector,
)
from testbench_framework import (
    TestCase,
    TestReport,
    clear_registry,
    discover,
    exhaustive,
    random_stimulus,
    run,
    test,
)


@pytest.fixture(autouse=True)
def _clear():
    clear_registry()
    yield
    clear_registry()


def make_buffer_hir() -> HIR:
    m = Module(
        name="bf",
        ports=[
            Port("a", Direction.IN, TyLogic()),
            Port("y", Direction.OUT, TyLogic()),
        ],
        cont_assigns=[ContAssign(target=PortRef("y"), rhs=PortRef("a"))],
    )
    return HIR(top="bf", modules={"bf": m})


def make_adder4_hir() -> HIR:
    m = Module(
        name="adder4",
        ports=[
            Port("a", Direction.IN, TyVector(TyLogic(), 4)),
            Port("b", Direction.IN, TyVector(TyLogic(), 4)),
            Port("cin", Direction.IN, TyLogic()),
            Port("sum", Direction.OUT, TyVector(TyLogic(), 4)),
            Port("cout", Direction.OUT, TyLogic()),
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


# ---- Decorator + discovery ----


def test_register_and_discover():
    @test
    def t1(dut):
        pass

    @test
    def t2(dut):
        pass

    cases = discover()
    assert len(cases) == 2
    assert {c.name for c in cases} == {"t1", "t2"}


def test_decorator_with_args():
    @test(name="custom", timeout_s=10)
    def t1(dut):
        pass

    cases = discover()
    assert cases[0].name == "custom"
    assert cases[0].timeout_s == 10


def test_clear_registry_works():
    @test
    def t1(dut):
        pass

    assert len(discover()) == 1
    clear_registry()
    assert len(discover()) == 0


# ---- run() ----


def test_run_passing():
    @test
    def passes(dut):
        dut.a.value = 1
        assert dut.y.value == 1

    rep = run(make_buffer_hir())
    assert rep.all_passed
    assert "passes" in rep.passed
    assert rep.failed == []


def test_run_failing():
    @test
    def fails(dut):
        dut.a.value = 1
        assert dut.y.value == 0  # wrong on purpose

    rep = run(make_buffer_hir())
    assert not rep.all_passed
    assert any("fails" == n for n, _msg in rep.failed)


def test_run_unexpected_exception():
    @test
    def explodes(dut):
        raise RuntimeError("boom")

    rep = run(make_buffer_hir())
    assert not rep.all_passed
    msg = rep.failed[0][1]
    assert "boom" in msg


def test_run_negative_test_passes():
    @test(should_fail=True)
    def must_fail(dut):
        assert False, "intentional"

    rep = run(make_buffer_hir())
    assert rep.all_passed


def test_run_negative_test_fails_when_passes():
    @test(should_fail=True)
    def should_have_failed(dut):
        pass  # no assertion

    rep = run(make_buffer_hir())
    assert not rep.all_passed


def test_run_with_explicit_tests_list():
    def my_func(dut):
        assert True

    tc = TestCase(name="explicit", func=my_func)
    rep = run(make_buffer_hir(), tests=[tc])
    assert "explicit" in rep.passed


def test_run_isolates_state_between_tests():
    """Each test should get a fresh VM."""
    @test
    def first(dut):
        dut.a.value = 1
        assert dut.y.value == 1

    @test
    def second(dut):
        # If state leaked, dut.a.value would still be 1
        assert dut.a.value == 0
        assert dut.y.value == 0

    rep = run(make_buffer_hir())
    assert rep.all_passed


# ---- TestReport ----


def test_report_summary():
    rep = TestReport(passed=["a"], failed=[("b", "boom")], skipped=[])
    s = rep.summary()
    assert "1 passed" in s
    assert "1 failed" in s


# ---- DUTHandle ----


def test_dut_handle_via_buffer():
    @test
    def buf_test(dut):
        dut.a.value = 1
        assert dut.y.value == 1
        dut.a.value = 0
        assert dut.y.value == 0

    rep = run(make_buffer_hir())
    assert rep.all_passed


def test_signal_handle_repr():
    from hardware_vm import HardwareVM
    from testbench_framework.runner import SignalHandle

    vm = HardwareVM(make_buffer_hir())
    h = SignalHandle(vm, "a")
    assert "a" in repr(h)
    assert "value" in repr(h)


# ---- Stimulus helpers ----


def test_exhaustive_drives_all_combinations():
    @test
    def adder_exhaustive(dut):
        seen = set()

        def check(d):
            seen.add((d.a.value, d.b.value, d.cin.value))
            expected = (d.a.value + d.b.value + d.cin.value) & 0x1F
            actual = (d.cout.value << 4) | d.sum.value
            assert actual == expected

        exhaustive(dut, inputs={"a": 4, "b": 4, "cin": 1}, on_step=check)
        assert len(seen) == 16 * 16 * 2

    rep = run(make_adder4_hir())
    assert rep.all_passed


def test_exhaustive_too_many_bits_raises():
    from testbench_framework.runner import DUTHandle
    from hardware_vm import HardwareVM

    vm = HardwareVM(make_buffer_hir())
    dut = DUTHandle(vm)
    with pytest.raises(ValueError, match="exhaustive over"):
        exhaustive(dut, inputs={"a": 25})


def test_random_stimulus_reproducible():
    @test
    def adder_random(dut):
        results_a = []

        def collect(d):
            results_a.append(d.a.value)

        random_stimulus(dut, inputs={"a": 4, "b": 4, "cin": 1},
                        iterations=20, seed=42, on_step=collect)
        # Re-run with the same seed should produce same sequence
        results_b = []

        def collect2(d):
            results_b.append(d.a.value)

        random_stimulus(dut, inputs={"a": 4, "b": 4, "cin": 1},
                        iterations=20, seed=42, on_step=collect2)
        assert results_a == results_b

    rep = run(make_adder4_hir())
    assert rep.all_passed


# ---- DUTHandle private attr ----


def test_dut_handle_private_attr_raises():
    from testbench_framework.runner import DUTHandle
    from hardware_vm import HardwareVM

    vm = HardwareVM(make_buffer_hir())
    dut = DUTHandle(vm)
    with pytest.raises(AttributeError):
        _ = dut.__private__
