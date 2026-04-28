"""Tests for HIR validation rules."""

from hdl_ir import (
    HIR,
    BinaryOp,
    Concat,
    ContAssign,
    Direction,
    Instance,
    Level,
    Lit,
    Module,
    Net,
    NetRef,
    Port,
    PortRef,
    Process,
    ProcessKind,
    SensitivityItem,
    TyLogic,
    TyVector,
    validate,
)


def make_minimal_module(name: str = "m") -> Module:
    return Module(
        name=name,
        ports=[
            Port("a", Direction.IN, TyLogic()),
            Port("y", Direction.OUT, TyLogic()),
        ],
        cont_assigns=[ContAssign(target=PortRef("y"), rhs=PortRef("a"))],
    )


# ---- H1: top exists ----


def test_h1_top_exists_passes():
    m = make_minimal_module()
    hir = HIR(top="m", modules={"m": m})
    r = validate(hir)
    assert r.ok


def test_h1_top_missing_fails():
    hir = HIR(top="not_there", modules={})
    r = validate(hir)
    assert not r.ok
    assert any("H1" in e for e in r.errors)


# ---- H3: instance.module resolves ----


def test_h3_unknown_module_fails():
    parent = Module(
        name="parent",
        instances=[Instance(name="u", module="nonexistent_module")],
    )
    hir = HIR(top="parent", modules={"parent": parent})
    r = validate(hir)
    assert any("H3" in e for e in r.errors)


def test_h3_known_module_passes():
    child = Module(name="child")
    parent = Module(name="parent", instances=[Instance(name="u", module="child")])
    hir = HIR(top="parent", modules={"parent": parent, "child": child})
    r = validate(hir)
    assert r.ok or all("H3" not in e for e in r.errors)


# ---- H4: connection key is real port ----


def test_h4_unknown_pin_fails():
    child = Module(
        name="child",
        ports=[Port("a", Direction.IN, TyLogic())],
    )
    parent = Module(
        name="parent",
        nets=[Net("x", TyLogic())],
        instances=[
            Instance(
                name="u",
                module="child",
                connections={"not_a_real_pin": NetRef("x")},
            )
        ],
    )
    hir = HIR(top="parent", modules={"parent": parent, "child": child})
    r = validate(hir)
    assert any("H4" in e for e in r.errors)


def test_h4_real_pin_passes():
    child = Module(
        name="child",
        ports=[Port("a", Direction.IN, TyLogic())],
    )
    parent = Module(
        name="parent",
        nets=[Net("x", TyLogic())],
        instances=[
            Instance(name="u", module="child", connections={"a": NetRef("x")})
        ],
    )
    hir = HIR(top="parent", modules={"parent": parent, "child": child})
    r = validate(hir)
    assert all("H4" not in e for e in r.errors)


# ---- H6: refs resolve ----


def test_h6_undefined_net_ref_fails():
    m = Module(
        name="m",
        ports=[Port("y", Direction.OUT, TyLogic())],
        cont_assigns=[ContAssign(target=PortRef("y"), rhs=NetRef("undefined"))],
    )
    hir = HIR(top="m", modules={"m": m})
    r = validate(hir)
    assert any("H6" in e for e in r.errors)


def test_h6_defined_net_ref_passes():
    m = Module(
        name="m",
        ports=[Port("y", Direction.OUT, TyLogic())],
        nets=[Net("internal", TyLogic())],
        cont_assigns=[ContAssign(target=PortRef("y"), rhs=NetRef("internal"))],
    )
    hir = HIR(top="m", modules={"m": m})
    r = validate(hir)
    assert all("H6" not in e for e in r.errors)


def test_h6_undefined_port_ref_fails():
    m = Module(
        name="m",
        ports=[Port("y", Direction.OUT, TyLogic())],
        cont_assigns=[ContAssign(target=PortRef("y"), rhs=PortRef("missing"))],
    )
    hir = HIR(top="m", modules={"m": m})
    r = validate(hir)
    assert any("H6" in e for e in r.errors)


# ---- H6: nested expression refs ----


def test_h6_in_concat_fails():
    m = Module(
        name="m",
        ports=[Port("y", Direction.OUT, TyVector(TyLogic(), 2))],
        cont_assigns=[
            ContAssign(
                target=PortRef("y"),
                rhs=Concat((PortRef("x"), PortRef("z"))),  # both undefined
            )
        ],
    )
    hir = HIR(top="m", modules={"m": m})
    r = validate(hir)
    assert sum(1 for e in r.errors if "H6" in e) >= 2


def test_h6_in_binary_op_fails():
    m = Module(
        name="m",
        ports=[Port("y", Direction.OUT, TyLogic())],
        cont_assigns=[
            ContAssign(target=PortRef("y"), rhs=BinaryOp("+", NetRef("a"), NetRef("b")))
        ],
    )
    hir = HIR(top="m", modules={"m": m})
    r = validate(hir)
    assert sum(1 for e in r.errors if "H6" in e) == 2


# ---- H12: structural has no processes ----


def test_h12_structural_with_process_fails():
    m = Module(
        name="m",
        level=Level.STRUCTURAL,
        processes=[Process(kind=ProcessKind.ALWAYS, body=())],
    )
    hir = HIR(top="m", modules={"m": m})
    r = validate(hir)
    assert any("H12" in e for e in r.errors)


def test_h12_behavioral_with_process_passes():
    m = Module(
        name="m",
        level=Level.BEHAVIORAL,
        processes=[Process(kind=ProcessKind.ALWAYS, body=())],
    )
    hir = HIR(top="m", modules={"m": m})
    r = validate(hir)
    assert all("H12" not in e for e in r.errors)


# ---- H20: no transitive self-instantiation ----


def test_h20_direct_self_loop_fails():
    m = Module(name="m", instances=[Instance(name="self", module="m")])
    hir = HIR(top="m", modules={"m": m})
    r = validate(hir)
    assert any("H20" in e for e in r.errors)


def test_h20_two_module_cycle_fails():
    a = Module(name="a", instances=[Instance(name="ub", module="b")])
    b = Module(name="b", instances=[Instance(name="ua", module="a")])
    hir = HIR(top="a", modules={"a": a, "b": b})
    r = validate(hir)
    assert any("H20" in e for e in r.errors)


def test_h20_no_cycle_passes():
    leaf = Module(name="leaf")
    mid = Module(name="mid", instances=[Instance(name="u", module="leaf")])
    top = Module(name="top", instances=[Instance(name="u", module="mid")])
    hir = HIR(top="top", modules={"top": top, "mid": mid, "leaf": leaf})
    r = validate(hir)
    assert all("H20" not in e for e in r.errors)


# ---- Duplicate names ----


def test_duplicate_port_names_fails():
    m = Module(
        name="m",
        ports=[
            Port("a", Direction.IN, TyLogic()),
            Port("a", Direction.OUT, TyLogic()),
        ],
    )
    hir = HIR(top="m", modules={"m": m})
    r = validate(hir)
    assert any("duplicate port" in e for e in r.errors)


def test_duplicate_net_names_fails():
    m = Module(
        name="m",
        nets=[Net("x", TyLogic()), Net("x", TyLogic())],
    )
    hir = HIR(top="m", modules={"m": m})
    r = validate(hir)
    assert any("duplicate net" in e for e in r.errors)


# ---- Process sensitivity refs check ----


def test_process_sensitivity_undefined_signal_fails():
    m = Module(
        name="m",
        processes=[
            Process(
                kind=ProcessKind.ALWAYS,
                sensitivity=(SensitivityItem("posedge", NetRef("undefined_clk")),),
                body=(),
            )
        ],
    )
    hir = HIR(top="m", modules={"m": m})
    r = validate(hir)
    assert any("H6" in e for e in r.errors)


# ---- Parameterized module reference ----


def test_unknown_parameter_warns():
    child = Module(name="child")
    parent = Module(
        name="parent",
        instances=[
            Instance(
                name="u",
                module="child",
                parameters={"WIDTH": Lit(8, TyLogic())},
            )
        ],
    )
    hir = HIR(top="parent", modules={"parent": parent, "child": child})
    r = validate(hir)
    assert any("WIDTH" in w for w in r.warnings)
