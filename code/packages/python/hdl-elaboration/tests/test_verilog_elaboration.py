"""Tests for Verilog -> HIR elaboration."""

import pytest

from hdl_elaboration import elaborate_verilog
from hdl_ir import (
    BinaryOp,
    Concat,
    ContAssign,
    Direction,
    PortRef,
    SourceLang,
    TyLogic,
    TyVector,
)


# ---- Smoke tests ----


def test_minimal_module():
    src = """
    module simple(input a, output y);
      assign y = a;
    endmodule
    """
    hir = elaborate_verilog(src, top="simple")
    assert hir.top == "simple"
    assert "simple" in hir.modules
    m = hir.modules["simple"]
    assert m.name == "simple"
    assert len(m.ports) == 2
    assert m.ports[0].name == "a"
    assert m.ports[0].direction == Direction.IN
    assert m.ports[1].name == "y"
    assert m.ports[1].direction == Direction.OUT
    assert len(m.cont_assigns) == 1


def test_provenance_attached():
    src = "module m(input a, output y); assign y = a; endmodule"
    hir = elaborate_verilog(src, top="m")
    assert hir.modules["m"].provenance is not None
    assert hir.modules["m"].provenance.lang == SourceLang.VERILOG


# ---- 4-bit adder ----


def test_adder4_ports_and_widths():
    src = """
    module adder4(input [3:0] a, input [3:0] b, input cin,
                  output [3:0] sum, output cout);
      assign {cout, sum} = a + b + cin;
    endmodule
    """
    hir = elaborate_verilog(src, top="adder4")
    m = hir.modules["adder4"]

    assert len(m.ports) == 5

    # 4-bit input a
    a_port = m.find_port("a")
    assert a_port is not None
    assert a_port.direction == Direction.IN
    assert isinstance(a_port.type, TyVector)
    assert a_port.type.width == 4

    # 1-bit cin
    cin_port = m.find_port("cin")
    assert cin_port is not None
    assert isinstance(cin_port.type, TyLogic)

    # 4-bit output sum
    sum_port = m.find_port("sum")
    assert sum_port is not None
    assert sum_port.direction == Direction.OUT
    assert isinstance(sum_port.type, TyVector)
    assert sum_port.type.width == 4

    # 1-bit cout
    cout_port = m.find_port("cout")
    assert cout_port is not None
    assert cout_port.direction == Direction.OUT


def test_adder4_continuous_assign_structure():
    src = """
    module adder4(input [3:0] a, input [3:0] b, input cin,
                  output [3:0] sum, output cout);
      assign {cout, sum} = a + b + cin;
    endmodule
    """
    hir = elaborate_verilog(src, top="adder4")
    m = hir.modules["adder4"]

    assert len(m.cont_assigns) == 1
    ca = m.cont_assigns[0]
    assert isinstance(ca, ContAssign)

    # Target should be a concat of cout and sum
    assert isinstance(ca.target, Concat)
    assert len(ca.target.parts) == 2

    # RHS should be a binary + chain
    assert isinstance(ca.rhs, BinaryOp)
    assert ca.rhs.op == "+"


def test_adder4_validates_clean():
    src = """
    module adder4(input [3:0] a, input [3:0] b, input cin,
                  output [3:0] sum, output cout);
      assign {cout, sum} = a + b + cin;
    endmodule
    """
    hir = elaborate_verilog(src, top="adder4")
    from hdl_ir import validate
    report = validate(hir)
    # H6 (ref resolution) should pass - all PortRefs resolve to declared ports.
    h6_errors = [e for e in report.errors if "H6" in e]
    assert h6_errors == [], f"unexpected H6 errors: {h6_errors}"


def test_adder4_json_round_trip(tmp_path):
    src = """
    module adder4(input [3:0] a, input [3:0] b, input cin,
                  output [3:0] sum, output cout);
      assign {cout, sum} = a + b + cin;
    endmodule
    """
    hir = elaborate_verilog(src, top="adder4")
    p = tmp_path / "adder4.json"
    hir.to_json(p)
    from hdl_ir import HIR
    rt = HIR.from_json(p)
    assert rt.top == hir.top
    assert "adder4" in rt.modules
    assert len(rt.modules["adder4"].ports) == 5


# ---- Multiple operators ----


def test_bitwise_and():
    src = "module m(input a, input b, output y); assign y = a & b; endmodule"
    hir = elaborate_verilog(src, top="m")
    ca = hir.modules["m"].cont_assigns[0]
    assert isinstance(ca.rhs, BinaryOp)
    assert ca.rhs.op == "&"


def test_bitwise_or():
    src = "module m(input a, input b, output y); assign y = a | b; endmodule"
    hir = elaborate_verilog(src, top="m")
    ca = hir.modules["m"].cont_assigns[0]
    assert isinstance(ca.rhs, BinaryOp)
    assert ca.rhs.op == "|"


def test_bitwise_xor():
    src = "module m(input a, input b, output y); assign y = a ^ b; endmodule"
    hir = elaborate_verilog(src, top="m")
    ca = hir.modules["m"].cont_assigns[0]
    assert isinstance(ca.rhs, BinaryOp)
    assert ca.rhs.op == "^"


def test_subtract():
    src = "module m(input [3:0] a, input [3:0] b, output [3:0] y); assign y = a - b; endmodule"
    hir = elaborate_verilog(src, top="m")
    ca = hir.modules["m"].cont_assigns[0]
    assert isinstance(ca.rhs, BinaryOp)
    assert ca.rhs.op == "-"


# ---- Error cases ----


def test_unknown_top_raises():
    src = "module foo(input a, output y); assign y = a; endmodule"
    with pytest.raises(KeyError, match="top module"):
        elaborate_verilog(src, top="not_there")
