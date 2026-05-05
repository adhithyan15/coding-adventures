"""Expression compilation — post-order stack-machine emission."""

from __future__ import annotations

from sql_planner import (
    Between,
    BinaryExpr,
    Column,
    FuncArg,
    FunctionCall,
    In,
    IsNotNull,
    IsNull,
    Like,
    Literal,
    NotIn,
    NotLike,
    UnaryExpr,
)
from sql_planner import (
    BinaryOp as AstOp,
)
from sql_planner import (
    UnaryOp as AstUnOp,
)

from sql_codegen import (
    Between as IrBetween,
)
from sql_codegen import (
    BinaryOp,
    BinaryOpCode,
    CallScalar,
    InList,
    LoadColumn,
    LoadConst,
    UnaryOp,
    UnaryOpCode,
    compile_expr,
)
from sql_codegen import (
    IsNotNull as IrIsNotNull,
)
from sql_codegen import (
    IsNull as IrIsNull,
)
from sql_codegen import (
    Like as IrLike,
)


def test_literal_compiles_to_load_const() -> None:
    assert compile_expr(Literal(5)) == [LoadConst(value=5)]


def test_column_compiles_to_load_column() -> None:
    instrs = compile_expr(Column("t", "x"))
    assert len(instrs) == 1
    assert isinstance(instrs[0], LoadColumn)
    assert instrs[0].column == "x"


def test_binary_post_order() -> None:
    # 2 + 3 → LoadConst 2, LoadConst 3, BinaryOp(ADD)
    expr = BinaryExpr(op=AstOp.ADD, left=Literal(2), right=Literal(3))
    instrs = compile_expr(expr)
    assert instrs == [LoadConst(2), LoadConst(3), BinaryOp(op=BinaryOpCode.ADD)]


def test_unary_neg() -> None:
    instrs = compile_expr(UnaryExpr(op=AstUnOp.NEG, operand=Literal(5)))
    assert instrs[-1] == UnaryOp(op=UnaryOpCode.NEG)


def test_unary_not() -> None:
    instrs = compile_expr(UnaryExpr(op=AstUnOp.NOT, operand=Literal(True)))
    assert instrs[-1] == UnaryOp(op=UnaryOpCode.NOT)


def test_is_null() -> None:
    instrs = compile_expr(IsNull(operand=Literal(None)))
    assert instrs[-1] == IrIsNull()


def test_is_not_null() -> None:
    instrs = compile_expr(IsNotNull(operand=Literal(5)))
    assert instrs[-1] == IrIsNotNull()


def test_between() -> None:
    instrs = compile_expr(
        Between(operand=Literal(5), low=Literal(1), high=Literal(10))
    )
    # Three values pushed, then Between.
    assert instrs[-1] == IrBetween()
    assert len(instrs) == 4


def test_in_emits_inlist() -> None:
    instrs = compile_expr(
        In(operand=Column("t", "x"), values=(Literal(1), Literal(2), Literal(3)))
    )
    assert instrs[-1] == InList(n=3)


def test_not_in_emits_not_of_inlist() -> None:
    instrs = compile_expr(
        NotIn(operand=Column("t", "x"), values=(Literal(1),))
    )
    assert instrs[-2] == InList(n=1)
    assert instrs[-1] == UnaryOp(op=UnaryOpCode.NOT)


def test_like() -> None:
    instrs = compile_expr(Like(operand=Column("t", "x"), pattern="%a%"))
    assert instrs[-1] == IrLike(negated=False)
    assert LoadConst(value="%a%") in instrs


def test_not_like() -> None:
    instrs = compile_expr(NotLike(operand=Column("t", "x"), pattern="%b%"))
    assert instrs[-1] == IrLike(negated=True)


def test_coalesce() -> None:
    # COALESCE now routes through CallScalar — the Coalesce IR instruction is
    # kept for backwards compatibility but new codegen always emits CallScalar.
    instrs = compile_expr(
        FunctionCall(
            name="coalesce",
            args=(FuncArg(value=Column("t", "a")), FuncArg(value=Literal(0))),
        )
    )
    assert instrs[-1] == CallScalar(func="coalesce", n_args=2)


def test_unknown_function_compiles_to_call_scalar() -> None:
    # Unknown function names are deferred to the VM — codegen always emits
    # CallScalar and never raises UnsupportedNode for function calls.
    # The VM raises UnsupportedFunction at runtime if the function is missing.
    instrs = compile_expr(FunctionCall(name="unknown_fn", args=()))
    assert instrs[-1] == CallScalar(func="unknown_fn", n_args=0)


def test_concat_emits_binary_concat() -> None:
    # SQL || maps to BinaryOpCode.CONCAT — same post-order stack shape as ADD.
    # Left push, right push, then the binary operator instruction.
    expr = BinaryExpr(op=AstOp.CONCAT, left=Literal("hello"), right=Literal("world"))
    instrs = compile_expr(expr)
    assert instrs == [
        LoadConst("hello"),
        LoadConst("world"),
        BinaryOp(op=BinaryOpCode.CONCAT),
    ]


def test_concat_column_and_literal() -> None:
    # Column || literal — checks that non-literal concat still uses CONCAT opcode.
    expr = BinaryExpr(op=AstOp.CONCAT, left=Column("t", "name"), right=Literal("!"))
    instrs = compile_expr(expr)
    assert any(isinstance(i, LoadColumn) for i in instrs)
    assert instrs[-1] == BinaryOp(op=BinaryOpCode.CONCAT)
