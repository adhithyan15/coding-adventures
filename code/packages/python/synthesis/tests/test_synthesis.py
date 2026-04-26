"""Tests for HIR -> HNL synthesis."""

from hdl_ir import (
    HIR,
    BinaryOp,
    Concat,
    ContAssign,
    Direction,
    Lit,
    Module,
    Port,
    PortRef,
    TyLogic,
    TyVector,
    UnaryOp,
)
from synthesis import synthesize


def make_hir(mod: Module, top: str | None = None) -> HIR:
    return HIR(top=top or mod.name, modules={mod.name: mod})


# ---- Trivial buffer: y = a ----


def test_buffer_synthesis():
    m = Module(
        name="buf",
        ports=[
            Port("a", Direction.IN, TyLogic()),
            Port("y", Direction.OUT, TyLogic()),
        ],
        cont_assigns=[ContAssign(target=PortRef("y"), rhs=PortRef("a"))],
    )
    hnl = synthesize(make_hir(m))
    assert hnl.top == "buf"
    bm = hnl.modules["buf"]
    assert bm.port("a") is not None
    assert bm.port("y") is not None
    # Should at least have one BUF
    assert any(i.cell_type == "BUF" for i in bm.instances)


# ---- Bitwise AND ----


def test_bitwise_and_4bit():
    m = Module(
        name="band",
        ports=[
            Port("a", Direction.IN, TyVector(TyLogic(), 4)),
            Port("b", Direction.IN, TyVector(TyLogic(), 4)),
            Port("y", Direction.OUT, TyVector(TyLogic(), 4)),
        ],
        cont_assigns=[
            ContAssign(target=PortRef("y"), rhs=BinaryOp("&", PortRef("a"), PortRef("b"))),
        ],
    )
    hnl = synthesize(make_hir(m))
    bm = hnl.modules["band"]
    and_cells = [i for i in bm.instances if i.cell_type == "AND2"]
    assert len(and_cells) == 4


# ---- Bitwise XOR ----


def test_bitwise_xor():
    m = Module(
        name="bx",
        ports=[
            Port("a", Direction.IN, TyVector(TyLogic(), 8)),
            Port("b", Direction.IN, TyVector(TyLogic(), 8)),
            Port("y", Direction.OUT, TyVector(TyLogic(), 8)),
        ],
        cont_assigns=[
            ContAssign(target=PortRef("y"), rhs=BinaryOp("^", PortRef("a"), PortRef("b"))),
        ],
    )
    hnl = synthesize(make_hir(m))
    bm = hnl.modules["bx"]
    assert sum(1 for i in bm.instances if i.cell_type == "XOR2") == 8


# ---- NOT ----


def test_unary_not():
    m = Module(
        name="inv",
        ports=[
            Port("a", Direction.IN, TyVector(TyLogic(), 4)),
            Port("y", Direction.OUT, TyVector(TyLogic(), 4)),
        ],
        cont_assigns=[
            ContAssign(target=PortRef("y"), rhs=UnaryOp("NOT", PortRef("a"))),
        ],
    )
    hnl = synthesize(make_hir(m))
    bm = hnl.modules["inv"]
    assert sum(1 for i in bm.instances if i.cell_type == "NOT") == 4


# ---- Reduction AND ----


def test_reduction_and():
    m = Module(
        name="redand",
        ports=[
            Port("a", Direction.IN, TyVector(TyLogic(), 4)),
            Port("y", Direction.OUT, TyLogic()),
        ],
        cont_assigns=[
            ContAssign(target=PortRef("y"), rhs=UnaryOp("AND_RED", PortRef("a"))),
        ],
    )
    hnl = synthesize(make_hir(m))
    bm = hnl.modules["redand"]
    # 4 input bits -> 2 ANDs in level 1 + 1 AND in level 2 = 3 ANDs total
    and_count = sum(1 for i in bm.instances if i.cell_type == "AND2")
    assert and_count >= 3


# ---- Adder ----


def test_adder_1bit():
    """1-bit adder: (cout, sum) = a + b. Should produce a half-adder (XOR + AND) plus the chain wraps."""
    m = Module(
        name="ha",
        ports=[
            Port("a", Direction.IN, TyLogic()),
            Port("b", Direction.IN, TyLogic()),
            Port("sum", Direction.OUT, TyLogic()),
            Port("cout", Direction.OUT, TyLogic()),
        ],
        cont_assigns=[
            ContAssign(
                target=Concat((PortRef("cout"), PortRef("sum"))),
                rhs=BinaryOp("+", PortRef("a"), PortRef("b")),
            )
        ],
    )
    hnl = synthesize(make_hir(m))
    bm = hnl.modules["ha"]
    counts = {}
    for i in bm.instances:
        counts[i.cell_type] = counts.get(i.cell_type, 0) + 1
    # At minimum: 2 XOR2, 2 AND2, 1 OR2 (full adder structure)
    assert counts.get("XOR2", 0) >= 2
    assert counts.get("AND2", 0) >= 2
    assert counts.get("OR2", 0) >= 1


def test_adder_4bit_canonical():
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
            ),
        ],
    )
    hnl = synthesize(make_hir(m))
    bm = hnl.modules["adder4"]
    counts = {}
    for i in bm.instances:
        counts[i.cell_type] = counts.get(i.cell_type, 0) + 1
    # We expect lots of cells. The two adders together generate roughly:
    # 2x (8 XOR2 + 4 AND2 + 4 OR2) = 16 XOR2, 8 AND2, 8 OR2 plus extras for
    # constants and BUFs. Sanity-check that we have substantial gate counts.
    assert counts.get("XOR2", 0) >= 8
    assert counts.get("AND2", 0) >= 8
    assert counts.get("OR2", 0) >= 4


# ---- HNL validates ----


def test_synthesized_hnl_validates():
    """Sanity: synthesized adder produces an HNL that's structurally valid."""
    m = Module(
        name="add2",
        ports=[
            Port("a", Direction.IN, TyVector(TyLogic(), 2)),
            Port("b", Direction.IN, TyVector(TyLogic(), 2)),
            Port("sum", Direction.OUT, TyVector(TyLogic(), 2)),
            Port("cout", Direction.OUT, TyLogic()),
        ],
        cont_assigns=[
            ContAssign(
                target=Concat((PortRef("cout"), PortRef("sum"))),
                rhs=BinaryOp("+", PortRef("a"), PortRef("b")),
            )
        ],
    )
    hnl = synthesize(make_hir(m))
    report = hnl.validate()
    # Structural validity: no R3/R4 errors (we connect every input pin we mention)
    assert all("R4" not in e for e in report.errors), f"R4 errors: {[e for e in report.errors if 'R4' in e]}"


# ---- Literal ----


def test_literal_constants():
    m = Module(
        name="c",
        ports=[Port("y", Direction.OUT, TyVector(TyLogic(), 4))],
        cont_assigns=[
            ContAssign(target=PortRef("y"), rhs=Lit(value=0b1010, type=TyVector(TyLogic(), 4))),
        ],
    )
    hnl = synthesize(make_hir(m))
    bm = hnl.modules["c"]
    consts = [i for i in bm.instances if i.cell_type in ("CONST_0", "CONST_1")]
    assert len(consts) == 4  # one per bit
