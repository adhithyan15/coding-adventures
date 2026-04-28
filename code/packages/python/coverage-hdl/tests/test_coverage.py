"""Tests for coverage measurement."""

from coverage_hdl import (
    CoverageRecorder,
    CoverageReport,
    Coverpoint,
    CrossPoint,
    bin_default,
    bin_range,
    bin_value,
)
from hardware_vm import HardwareVM
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


# ---- Bin constructors ----


def test_bin_value():
    b = bin_value("v", 5)
    assert b.matcher(5)
    assert not b.matcher(6)


def test_bin_range():
    b = bin_range("r", 1, 10)
    assert b.matcher(5)
    assert b.matcher(1)
    assert b.matcher(10)
    assert not b.matcher(0)
    assert not b.matcher(11)


def test_bin_default():
    b = bin_default()
    assert b.matcher(0)
    assert b.matcher(999)


# ---- Coverpoint ----


def test_coverpoint_initial_zero_hits():
    cp = Coverpoint(
        name="x", signal="s",
        bins=[bin_value("a", 0), bin_value("b", 1)],
    )
    assert cp.hits["a"] == 0
    assert cp.hits["b"] == 0


def test_coverpoint_sample_first_match():
    cp = Coverpoint(
        name="x", signal="s",
        bins=[bin_range("low", 0, 5), bin_range("high", 6, 10)],
    )
    cp.sample(3)
    assert cp.hits["low"] == 1
    assert cp.hits["high"] == 0
    cp.sample(8)
    assert cp.hits["high"] == 1


def test_coverpoint_unmatched_value():
    cp = Coverpoint(
        name="x", signal="s",
        bins=[bin_value("a", 0)],
    )
    cp.sample(99)  # no bin matches; nothing accumulates
    assert cp.hits["a"] == 0


def test_coverpoint_coverage_percentage():
    cp = Coverpoint(
        name="x", signal="s",
        bins=[bin_value("a", 0), bin_value("b", 1)],
    )
    assert cp.coverage == 0.0
    cp.sample(0)
    assert cp.coverage == 0.5
    cp.sample(1)
    assert cp.coverage == 1.0


def test_coverpoint_no_bins_is_full_coverage():
    cp = Coverpoint(name="x", signal="s", bins=[])
    assert cp.coverage == 1.0


# ---- CoverageRecorder + toggle ----


def test_toggle_rising_and_falling():
    vm = HardwareVM(make_buffer_hir())
    cov = CoverageRecorder(vm)
    cov.enable_toggle_coverage(["a", "y"])

    vm.set_input("a", 1)  # 0->1 rising
    vm.set_input("a", 0)  # 1->0 falling
    vm.set_input("a", 1)  # rising

    rep = cov.report()
    assert rep.toggle["a"].rising == 2
    assert rep.toggle["a"].falling == 1
    # y also toggles since y = a
    assert rep.toggle["y"].rising >= 1


def test_toggle_only_for_enabled_signals():
    vm = HardwareVM(make_buffer_hir())
    cov = CoverageRecorder(vm)
    cov.enable_toggle_coverage(["a"])  # only a, not y

    vm.set_input("a", 1)
    rep = cov.report()
    assert "a" in rep.toggle
    assert "y" not in rep.toggle


# ---- CoverageRecorder + coverpoints ----


def test_coverpoint_via_vm():
    vm = HardwareVM(make_buffer_hir())
    cov = CoverageRecorder(vm)

    cov.add_coverpoint(Coverpoint(
        name="a_val",
        signal="a",
        bins=[bin_value("zero", 0), bin_value("one", 1)],
    ))

    vm.set_input("a", 1)
    rep = cov.report()
    assert rep.coverpoints["a_val"]["one"] == 1
    # 'zero' wasn't sampled because a started at 0 and the first event was 0->1.
    # The recorder doesn't sample initial values; only changes.
    assert rep.coverpoints["a_val"]["zero"] == 0


def test_overall_coverage():
    vm = HardwareVM(make_buffer_hir())
    cov = CoverageRecorder(vm)

    cov.add_coverpoint(Coverpoint(
        name="cp1",
        signal="a",
        bins=[bin_value("zero", 0), bin_value("one", 1)],
    ))

    assert cov.overall_coverage == 0.0
    vm.set_input("a", 1)
    # One bin hit out of two; coverage = 0.5
    assert cov.overall_coverage == 0.5


def test_overall_coverage_no_points():
    vm = HardwareVM(make_buffer_hir())
    cov = CoverageRecorder(vm)
    assert cov.overall_coverage == 0.0


# ---- CrossPoint ----


def test_cross_records_joint_hits():
    vm = HardwareVM(make_adder4_hir())
    cov = CoverageRecorder(vm)

    cp_cin = Coverpoint(
        name="cin",
        signal="cin",
        bins=[bin_value("zero", 0), bin_value("one", 1)],
    )
    cp_a = Coverpoint(
        name="a",
        signal="a",
        bins=[bin_range("low", 0, 7), bin_range("high", 8, 15)],
    )

    cov.add_coverpoint(cp_cin)
    cov.add_coverpoint(cp_a)

    cross = CrossPoint(name="cin_x_a", coverpoints=[cp_cin, cp_a])
    cov.add_cross(cross)

    vm.set_input("a", 5)
    vm.set_input("cin", 1)
    cov.sample_cross()

    rep = cov.report()
    # Joint: cin=one, a=low
    assert rep.crosses["cin_x_a"][("one", "low")] == 1


def test_cross_coverage_zero_when_unsampled():
    cp_a = Coverpoint(name="a", signal="a", bins=[bin_value("z", 0)])
    cross = CrossPoint(name="x", coverpoints=[cp_a])
    assert cross.coverage == 0.0


def test_cross_coverage_no_coverpoints():
    cross = CrossPoint(name="x", coverpoints=[])
    assert cross.coverage == 1.0


def test_cross_skips_unmatched_value():
    """If the current signal value matches no bin, the cross sample is dropped."""
    vm = HardwareVM(make_buffer_hir())
    cov = CoverageRecorder(vm)
    cp = Coverpoint(name="a", signal="a", bins=[bin_value("only_5", 5)])
    cov.add_coverpoint(cp)
    cross = CrossPoint(name="x", coverpoints=[cp])
    cov.add_cross(cross)
    vm.set_input("a", 1)  # not 5
    cov.sample_cross()
    assert cov.report().crosses["x"] == {}


def test_sample_cross_by_name():
    cp = Coverpoint(name="cp", signal="a", bins=[bin_default()])
    cross = CrossPoint(name="cross", coverpoints=[cp])
    cross._last_values["a"] = 3
    cross.sample()
    assert cross.hits[("default",)] == 1


def test_sample_cross_returns_early_without_value():
    cp = Coverpoint(name="cp", signal="a", bins=[bin_value("v", 0)])
    cross = CrossPoint(name="cross", coverpoints=[cp])
    # No _last_values populated -> sample short-circuits
    cross.sample()
    assert cross.hits == {}
