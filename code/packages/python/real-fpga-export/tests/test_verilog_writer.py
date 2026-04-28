"""Tests for the HIR -> Verilog writer."""

from pathlib import Path

import pytest

from hdl_ir import (
    HIR,
    BinaryOp,
    Concat,
    ContAssign,
    Direction,
    Instance,
    Lit,
    Module,
    Net,
    NetKind,
    NetRef,
    Port,
    PortRef,
    Slice,
    Ternary,
    TyLogic,
    TyVector,
    UnaryOp,
)
from real_fpga_export import (
    ToolchainOptions,
    ToolchainResult,
    to_ice40,
    write_verilog,
    write_verilog_str,
)


# ---- Helpers ----


def make_buffer_hir() -> HIR:
    m = Module(
        name="my_buffer",
        ports=[
            Port("a", Direction.IN, TyLogic()),
            Port("y", Direction.OUT, TyLogic()),
        ],
        cont_assigns=[ContAssign(target=PortRef("y"), rhs=PortRef("a"))],
    )
    return HIR(top="my_buffer", modules={"my_buffer": m})


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


# ---- Buffer ----


def test_buffer_module(tmp_path: Path):
    p = tmp_path / "buf.v"
    write_verilog(make_buffer_hir(), p)
    text = p.read_text()
    assert "module my_buffer" in text
    assert "input" in text
    assert "output" in text
    assert "assign y = a;" in text


def test_buffer_string():
    s = write_verilog_str(make_buffer_hir())
    assert "module my_buffer" in s


# ---- 4-bit adder ----


def test_adder4_emission(tmp_path: Path):
    p = tmp_path / "adder.v"
    write_verilog(make_adder4_hir(), p)
    text = p.read_text()
    assert "module adder4" in text
    assert "[3:0]" in text  # vector range
    assert "input  [3:0] a" in text or "input [3:0] a" in text or "input  [3:0]a" in text or "input [3:0]a" in text
    assert "assign {cout, sum} = " in text
    assert "+ cin" in text


# ---- Type ranges ----


def test_vector_port_range(tmp_path: Path):
    m = Module(
        name="x",
        ports=[
            Port("data", Direction.IN, TyVector(TyLogic(), 8)),
        ],
    )
    hir = HIR(top="x", modules={"x": m})
    s = write_verilog_str(hir)
    assert "[7:0]" in s


def test_scalar_port_no_range(tmp_path: Path):
    m = Module(
        name="x",
        ports=[Port("clk", Direction.IN, TyLogic())],
    )
    hir = HIR(top="x", modules={"x": m})
    s = write_verilog_str(hir)
    # Should NOT have a range for a 1-bit port
    assert "[" not in s.split("module x")[1].split("\n)\n")[0]


# ---- Expressions ----


def test_binary_op():
    m = Module(
        name="x",
        ports=[
            Port("a", Direction.IN, TyLogic()),
            Port("b", Direction.IN, TyLogic()),
            Port("y", Direction.OUT, TyLogic()),
        ],
        cont_assigns=[
            ContAssign(target=PortRef("y"), rhs=BinaryOp("&", PortRef("a"), PortRef("b"))),
        ],
    )
    s = write_verilog_str(HIR(top="x", modules={"x": m}))
    assert "(a & b)" in s


def test_unary_not():
    m = Module(
        name="x",
        ports=[
            Port("a", Direction.IN, TyLogic()),
            Port("y", Direction.OUT, TyLogic()),
        ],
        cont_assigns=[
            ContAssign(target=PortRef("y"), rhs=UnaryOp("NOT", PortRef("a"))),
        ],
    )
    s = write_verilog_str(HIR(top="x", modules={"x": m}))
    assert "(~a)" in s


def test_ternary():
    m = Module(
        name="x",
        ports=[
            Port("s", Direction.IN, TyLogic()),
            Port("a", Direction.IN, TyLogic()),
            Port("b", Direction.IN, TyLogic()),
            Port("y", Direction.OUT, TyLogic()),
        ],
        cont_assigns=[
            ContAssign(
                target=PortRef("y"),
                rhs=Ternary(cond=PortRef("s"), then_expr=PortRef("a"), else_expr=PortRef("b")),
            )
        ],
    )
    s = write_verilog_str(HIR(top="x", modules={"x": m}))
    assert "? a : b" in s


def test_slice():
    m = Module(
        name="x",
        ports=[
            Port("d", Direction.IN, TyVector(TyLogic(), 8)),
            Port("y", Direction.OUT, TyVector(TyLogic(), 4)),
        ],
        cont_assigns=[
            ContAssign(target=PortRef("y"), rhs=Slice(base=PortRef("d"), msb=3, lsb=0)),
        ],
    )
    s = write_verilog_str(HIR(top="x", modules={"x": m}))
    assert "d[3:0]" in s


def test_concat():
    m = Module(
        name="x",
        ports=[
            Port("a", Direction.IN, TyLogic()),
            Port("b", Direction.IN, TyVector(TyLogic(), 4)),
            Port("y", Direction.OUT, TyVector(TyLogic(), 5)),
        ],
        cont_assigns=[
            ContAssign(
                target=PortRef("y"), rhs=Concat((PortRef("a"), PortRef("b")))
            )
        ],
    )
    s = write_verilog_str(HIR(top="x", modules={"x": m}))
    assert "{a, b}" in s


def test_lit_int():
    m = Module(
        name="x",
        ports=[Port("y", Direction.OUT, TyVector(TyLogic(), 4))],
        cont_assigns=[
            ContAssign(target=PortRef("y"), rhs=Lit(value=10, type=TyVector(TyLogic(), 4))),
        ],
    )
    s = write_verilog_str(HIR(top="x", modules={"x": m}))
    assert "4'd10" in s


def test_lit_bool():
    m = Module(
        name="x",
        ports=[Port("y", Direction.OUT, TyLogic())],
        cont_assigns=[
            ContAssign(target=PortRef("y"), rhs=Lit(value=True, type=TyLogic())),
        ],
    )
    s = write_verilog_str(HIR(top="x", modules={"x": m}))
    assert "1'b1" in s


# ---- Instances ----


def test_instance_emission():
    child = Module(
        name="child",
        ports=[
            Port("a", Direction.IN, TyLogic()),
            Port("y", Direction.OUT, TyLogic()),
        ],
    )
    parent = Module(
        name="parent",
        ports=[
            Port("p_a", Direction.IN, TyLogic()),
            Port("p_y", Direction.OUT, TyLogic()),
        ],
        instances=[
            Instance(
                name="u",
                module="child",
                connections={"a": PortRef("p_a"), "y": PortRef("p_y")},
            )
        ],
    )
    hir = HIR(top="parent", modules={"parent": parent, "child": child})
    s = write_verilog_str(hir)
    assert "module parent" in s
    assert "module child" in s
    assert "child u" in s
    assert ".a(p_a)" in s
    assert ".y(p_y)" in s


# ---- Reserved words ----


def test_reserved_word_escaped():
    m = Module(
        name="x",
        ports=[
            Port("input_signal", Direction.IN, TyLogic()),  # 'input' is reserved
            Port("y", Direction.OUT, TyLogic()),
        ],
    )
    # Just verify writer doesn't crash on a complicated name
    s = write_verilog_str(HIR(top="x", modules={"x": m}))
    assert "input_signal" in s


# ---- Internal nets ----


def test_internal_net():
    m = Module(
        name="x",
        ports=[
            Port("a", Direction.IN, TyLogic()),
            Port("y", Direction.OUT, TyLogic()),
        ],
        nets=[Net("internal", TyLogic(), kind=NetKind.WIRE)],
        cont_assigns=[
            ContAssign(target=NetRef("internal"), rhs=PortRef("a")),
            ContAssign(target=PortRef("y"), rhs=NetRef("internal")),
        ],
    )
    s = write_verilog_str(HIR(top="x", modules={"x": m}))
    assert "wire internal" in s


# ---- Toolchain (skip_missing path) ----


def test_to_ice40_skip_missing(tmp_path: Path):
    """When yosys is not installed, skip_missing=True returns after Verilog
    emission without raising."""
    hir = make_buffer_hir()
    out = tmp_path / "out"
    result = to_ice40(hir, top="my_buffer", out_dir=out, skip_missing=True,
                     opts=ToolchainOptions(yosys="this-tool-does-not-exist-xyz"))
    assert result.verilog_path.exists()


def test_to_ice40_creates_output_dir(tmp_path: Path):
    hir = make_buffer_hir()
    out = tmp_path / "build" / "nested"
    assert not out.exists()
    result = to_ice40(hir, top="my_buffer", out_dir=out, skip_missing=True,
                     opts=ToolchainOptions(yosys="this-tool-does-not-exist-xyz"))
    assert out.exists()
    assert result.verilog_path.exists()
