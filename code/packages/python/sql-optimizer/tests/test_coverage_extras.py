"""Additional tests exercising uncommon paths to push coverage over 80%."""

from __future__ import annotations

from sql_planner import (
    AggFunc,
    Aggregate,
    AggregateItem,
    Between,
    BinaryExpr,
    BinaryOp,
    Column,
    CreateTable,
    Delete,
    Distinct,
    DropTable,
    EmptyResult,
    Filter,
    FuncArg,
    FunctionCall,
    Having,
    In,
    Insert,
    InsertSource,
    IsNotNull,
    IsNull,
    Join,
    JoinKind,
    Like,
    Literal,
    NotIn,
    NotLike,
    Project,
    ProjectionItem,
    Scan,
    Sort,
    UnaryExpr,
    UnaryOp,
    Union,
    Update,
    Wildcard,
)
from sql_planner.plan import Assignment as PlanAssignment
from sql_planner.plan import Limit, SortKey

from sql_optimizer import (
    ConstantFolding,
    DeadCodeElimination,
    LimitPushdown,
    PredicatePushdown,
    ProjectionPruning,
)

cf = ConstantFolding()
pp = PredicatePushdown()
pr = ProjectionPruning()
dce = DeadCodeElimination()
lp = LimitPushdown()


class TestConstantFoldingExprs:
    def test_unary_not_true(self) -> None:
        pred = UnaryExpr(op=UnaryOp.NOT, operand=Literal(True))
        out = cf(Filter(input=Scan(table="t"), predicate=pred))
        assert out.predicate == Literal(False)

    def test_unary_neg_null(self) -> None:
        pred = UnaryExpr(op=UnaryOp.NEG, operand=Literal(None))
        out = cf(Filter(input=Scan(table="t"), predicate=pred))
        assert out.predicate == Literal(None)

    def test_unary_neg_int(self) -> None:
        pred = UnaryExpr(op=UnaryOp.NEG, operand=Literal(5))
        out = cf(Filter(input=Scan(table="t"), predicate=pred))
        assert out.predicate == Literal(-5)

    def test_unary_not_column_stays(self) -> None:
        out = cf(
            Filter(
                input=Scan(table="t"),
                predicate=UnaryExpr(op=UnaryOp.NOT, operand=Column("t", "x")),
            )
        )
        assert isinstance(out.predicate, UnaryExpr)

    def test_between_folded(self) -> None:
        pred = Between(operand=Literal(5), low=Literal(1), high=Literal(10))
        out = cf(Filter(input=Scan(table="t"), predicate=pred))
        assert isinstance(out.predicate, Between)

    def test_in_folded(self) -> None:
        pred = In(operand=Column("t", "x"), values=(Literal(1), Literal(2)))
        out = cf(Filter(input=Scan(table="t"), predicate=pred))
        assert isinstance(out.predicate, In)

    def test_not_in_folded(self) -> None:
        pred = NotIn(operand=Column("t", "x"), values=(Literal(1),))
        out = cf(Filter(input=Scan(table="t"), predicate=pred))
        assert isinstance(out.predicate, NotIn)

    def test_like_folded(self) -> None:
        pred = Like(operand=Column("t", "x"), pattern="%a%")
        out = cf(Filter(input=Scan(table="t"), predicate=pred))
        assert isinstance(out.predicate, Like)

    def test_not_like_folded(self) -> None:
        pred = NotLike(operand=Column("t", "x"), pattern="%a%")
        out = cf(Filter(input=Scan(table="t"), predicate=pred))
        assert isinstance(out.predicate, NotLike)

    def test_isnull_column_stays(self) -> None:
        pred = IsNull(operand=Column("t", "x"))
        out = cf(Filter(input=Scan(table="t"), predicate=pred))
        assert isinstance(out.predicate, IsNull)

    def test_isnull_literal_null(self) -> None:
        pred = IsNull(operand=Literal(None))
        out = cf(Filter(input=Scan(table="t"), predicate=pred))
        assert out.predicate == Literal(True)

    def test_isnotnull_literal(self) -> None:
        pred = IsNotNull(operand=Literal(5))
        out = cf(Filter(input=Scan(table="t"), predicate=pred))
        assert out.predicate == Literal(True)

    def test_isnotnull_column_stays(self) -> None:
        pred = IsNotNull(operand=Column("t", "x"))
        out = cf(Filter(input=Scan(table="t"), predicate=pred))
        assert isinstance(out.predicate, IsNotNull)

    def test_wildcard_unchanged(self) -> None:
        tree = Project(
            input=Scan(table="t"), items=(ProjectionItem(expr=Wildcard(), alias=None),)
        )
        assert cf(tree) == tree

    def test_or_true_simplifies(self) -> None:
        pred = BinaryExpr(op=BinaryOp.OR, left=Literal(True), right=Column("t", "x"))
        out = cf(Filter(input=Scan(table="t"), predicate=pred))
        assert out.predicate == Literal(True)

    def test_or_false_returns_right(self) -> None:
        pred = BinaryExpr(op=BinaryOp.OR, left=Literal(False), right=Column("t", "x"))
        out = cf(Filter(input=Scan(table="t"), predicate=pred))
        assert out.predicate == Column("t", "x")

    def test_or_both_null(self) -> None:
        pred = BinaryExpr(op=BinaryOp.OR, left=Literal(None), right=Literal(None))
        out = cf(Filter(input=Scan(table="t"), predicate=pred))
        assert out.predicate == Literal(None)

    def test_and_both_null(self) -> None:
        pred = BinaryExpr(op=BinaryOp.AND, left=Literal(None), right=Literal(None))
        out = cf(Filter(input=Scan(table="t"), predicate=pred))
        assert out.predicate == Literal(None)

    def test_binary_null_arith_propagates(self) -> None:
        pred = BinaryExpr(op=BinaryOp.ADD, left=Literal(1), right=Literal(None))
        out = cf(Filter(input=Scan(table="t"), predicate=pred))
        assert out.predicate == Literal(None)

    def test_type_error_not_folded(self) -> None:
        pred = BinaryExpr(op=BinaryOp.ADD, left=Literal("a"), right=Literal(1))
        out = cf(Filter(input=Scan(table="t"), predicate=pred))
        # stays as BinaryExpr because + raises TypeError
        assert isinstance(out.predicate, BinaryExpr)


class TestConstantFoldingPlans:
    def test_aggregate_gb_folds(self) -> None:
        tree = Aggregate(
            input=Scan(table="t"),
            group_by=(BinaryExpr(op=BinaryOp.ADD, left=Literal(1), right=Literal(2)),),
            aggregates=(),
        )
        out = cf(tree)
        assert out.group_by == (Literal(3),)

    def test_having_folds(self) -> None:
        tree = Having(
            input=Scan(table="t"),
            predicate=BinaryExpr(op=BinaryOp.ADD, left=Literal(1), right=Literal(2)),
        )
        out = cf(tree)
        assert out.predicate == Literal(3)

    def test_sort_key_folds(self) -> None:
        tree = Sort(
            input=Scan(table="t"),
            keys=(SortKey(expr=BinaryExpr(op=BinaryOp.ADD, left=Literal(1), right=Literal(2))),),
        )
        out = cf(tree)
        assert out.keys[0].expr == Literal(3)

    def test_limit_folds(self) -> None:
        tree = Limit(input=Scan(table="t"), count=5)
        assert cf(tree) == tree

    def test_distinct_folds(self) -> None:
        tree = Distinct(input=Scan(table="t"))
        assert cf(tree) == tree

    def test_union_folds(self) -> None:
        tree = Union(left=Scan(table="a"), right=Scan(table="b"))
        assert cf(tree) == tree

    def test_join_with_cond_folds(self) -> None:
        tree = Join(
            left=Scan(table="a"),
            right=Scan(table="b"),
            kind=JoinKind.INNER,
            condition=BinaryExpr(op=BinaryOp.ADD, left=Literal(1), right=Literal(2)),
        )
        out = cf(tree)
        assert out.condition == Literal(3)

    def test_empty_result_untouched(self) -> None:
        tree = EmptyResult()
        assert cf(tree) == tree

    def test_create_table_untouched(self) -> None:
        tree = CreateTable(table="t", columns=(), if_not_exists=True)
        assert cf(tree) == tree

    def test_drop_table_untouched(self) -> None:
        tree = DropTable(table="t", if_exists=True)
        assert cf(tree) == tree

    def test_insert_values_folds(self) -> None:
        tree = Insert(
            table="t",
            columns=("x",),
            source=InsertSource(
                values=((BinaryExpr(op=BinaryOp.ADD, left=Literal(1), right=Literal(2)),),),
            ),
        )
        out = cf(tree)
        assert out.source.values == ((Literal(3),),)

    def test_insert_query_folds(self) -> None:
        tree = Insert(
            table="t",
            columns=None,
            source=InsertSource(query=Scan(table="src")),
        )
        out = cf(tree)
        assert isinstance(out.source.query, Scan)

    def test_update_folds(self) -> None:
        tree = Update(
            table="t",
            assignments=(
                PlanAssignment(
                    column="x",
                    value=BinaryExpr(op=BinaryOp.ADD, left=Literal(1), right=Literal(2)),
                ),
            ),
            predicate=BinaryExpr(op=BinaryOp.ADD, left=Literal(3), right=Literal(4)),
        )
        out = cf(tree)
        assert out.assignments[0].value == Literal(3)
        assert out.predicate == Literal(7)

    def test_update_no_pred(self) -> None:
        tree = Update(
            table="t",
            assignments=(PlanAssignment(column="x", value=Literal(1)),),
            predicate=None,
        )
        out = cf(tree)
        assert out.predicate is None

    def test_delete_folds(self) -> None:
        tree = Delete(
            table="t",
            predicate=BinaryExpr(op=BinaryOp.ADD, left=Literal(1), right=Literal(2)),
        )
        out = cf(tree)
        assert out.predicate == Literal(3)

    def test_delete_no_pred(self) -> None:
        tree = Delete(table="t", predicate=None)
        assert cf(tree).predicate is None


class TestPredicatePushdownExtras:
    def test_scan_passthrough(self) -> None:
        assert pp(Scan(table="t")) == Scan(table="t")

    def test_distinct_recurses(self) -> None:
        tree = Distinct(input=Scan(table="t"))
        assert pp(tree) == tree

    def test_union_recurses(self) -> None:
        tree = Union(left=Scan(table="a"), right=Scan(table="b"))
        assert pp(tree) == tree

    def test_aggregate_recurses(self) -> None:
        tree = Aggregate(
            input=Scan(table="t"),
            group_by=(),
            aggregates=(
                AggregateItem(
                    func=AggFunc.COUNT, arg=FuncArg(star=True, value=None), alias="c"
                ),
            ),
        )
        assert pp(tree) == tree

    def test_sort_recurses(self) -> None:
        tree = Sort(input=Scan(table="t"), keys=(SortKey(expr=Column("t", "x")),))
        assert pp(tree) == tree

    def test_limit_recurses(self) -> None:
        tree = Limit(input=Scan(table="t"), count=5)
        assert pp(tree) == tree

    def test_filter_below_distinct_pushes(self) -> None:
        tree = Filter(
            input=Distinct(input=Scan(table="t", alias="t")),
            predicate=BinaryExpr(op=BinaryOp.EQ, left=Column("t", "x"), right=Literal(1)),
        )
        out = pp(tree)
        assert isinstance(out, Distinct)
        assert isinstance(out.input, Filter)

    def test_filter_with_between(self) -> None:
        pred = Between(operand=Column("t", "x"), low=Literal(1), high=Literal(10))
        tree = Filter(input=Scan(table="t", alias="t"), predicate=pred)
        assert isinstance(pp(tree), Filter)

    def test_filter_with_in(self) -> None:
        pred = In(operand=Column("t", "x"), values=(Literal(1),))
        tree = Filter(input=Scan(table="t", alias="t"), predicate=pred)
        assert isinstance(pp(tree), Filter)

    def test_filter_unary(self) -> None:
        pred = UnaryExpr(op=UnaryOp.NOT, operand=Column("t", "x"))
        tree = Filter(input=Scan(table="t", alias="t"), predicate=pred)
        assert isinstance(pp(tree), Filter)


class TestProjectionPruningExtras:
    def test_aggregate_gets_groupby_cols(self) -> None:
        tree = Aggregate(
            input=Scan(table="t", alias="t"),
            group_by=(Column("t", "region"),),
            aggregates=(
                AggregateItem(
                    func=AggFunc.SUM,
                    arg=FuncArg(value=Column("t", "amount")),
                    alias="s",
                ),
            ),
        )
        out = pr(tree)
        assert isinstance(out, Aggregate)
        assert set(out.input.required_columns) == {"region", "amount"}

    def test_having_propagates(self) -> None:
        inner = Aggregate(
            input=Scan(table="t", alias="t"),
            group_by=(Column("t", "x"),),
            aggregates=(),
        )
        tree = Having(
            input=inner,
            predicate=BinaryExpr(op=BinaryOp.GT, left=Column("t", "x"), right=Literal(0)),
        )
        out = pr(tree)
        assert isinstance(out, Having)

    def test_sort_propagates(self) -> None:
        tree = Sort(
            input=Scan(table="t", alias="t"),
            keys=(SortKey(expr=Column("t", "x")),),
        )
        out = pr(tree)
        assert set(out.input.required_columns) == {"x"}

    def test_distinct_propagates(self) -> None:
        tree = Distinct(input=Scan(table="t"))
        assert pr(tree) == tree

    def test_union_propagates(self) -> None:
        tree = Union(left=Scan(table="a"), right=Scan(table="b"))
        assert pr(tree) == tree

    def test_limit_propagates(self) -> None:
        tree = Limit(input=Scan(table="t"), count=5)
        assert pr(tree) == tree

    def test_empty_result_passthrough(self) -> None:
        assert pr(EmptyResult()) == EmptyResult()

    def test_wildcard_in_binary(self) -> None:
        tree = Project(
            input=Scan(table="t", alias="t"),
            items=(
                ProjectionItem(
                    expr=BinaryExpr(op=BinaryOp.ADD, left=Wildcard(), right=Literal(1)),
                    alias="x",
                ),
            ),
        )
        out = pr(tree)
        # Wildcard triggers no pruning — scan stays unannotated.
        assert out.input.required_columns in (None, ())

    def test_function_call_extracts_cols(self) -> None:
        tree = Project(
            input=Scan(table="t", alias="t"),
            items=(
                ProjectionItem(
                    expr=FunctionCall(name="f", args=(FuncArg(value=Column("t", "x")),)),
                    alias="y",
                ),
            ),
        )
        out = pr(tree)
        assert "x" in out.input.required_columns

    def test_is_null_extracts_cols(self) -> None:
        tree = Project(
            input=Filter(
                input=Scan(table="t", alias="t"),
                predicate=IsNull(operand=Column("t", "x")),
            ),
            items=(ProjectionItem(expr=Column("t", "y"), alias="y"),),
        )
        out = pr(tree)
        assert set(out.input.input.required_columns) == {"x", "y"}

    def test_between_extracts_cols(self) -> None:
        tree = Project(
            input=Filter(
                input=Scan(table="t", alias="t"),
                predicate=Between(
                    operand=Column("t", "a"), low=Literal(1), high=Literal(9)
                ),
            ),
            items=(ProjectionItem(expr=Column("t", "b"), alias="b"),),
        )
        out = pr(tree)
        assert set(out.input.input.required_columns) == {"a", "b"}

    def test_in_extracts_cols(self) -> None:
        tree = Project(
            input=Filter(
                input=Scan(table="t", alias="t"),
                predicate=In(operand=Column("t", "a"), values=(Literal(1),)),
            ),
            items=(ProjectionItem(expr=Column("t", "b"), alias="b"),),
        )
        out = pr(tree)
        assert set(out.input.input.required_columns) == {"a", "b"}


class TestDeadCodeExtras:
    def test_scan_passthrough(self) -> None:
        assert dce(Scan(table="t")) == Scan(table="t")

    def test_empty_passthrough(self) -> None:
        assert dce(EmptyResult()) == EmptyResult()

    def test_filter_over_empty_propagates(self) -> None:
        tree = Filter(
            input=Filter(input=Scan(table="t"), predicate=Literal(False)),
            predicate=Literal(True),
        )
        assert isinstance(dce(tree), EmptyResult)

    def test_having_over_empty(self) -> None:
        inner = Aggregate(
            input=Filter(input=Scan(table="t"), predicate=Literal(False)),
            group_by=(),
            aggregates=(),
        )
        tree = Having(input=inner, predicate=Literal(True))
        # Aggregate preserves on empty; Having preserves too.
        assert isinstance(dce(tree), Having)

    def test_outer_join_does_not_propagate(self) -> None:
        tree = Join(
            left=Filter(input=Scan(table="a"), predicate=Literal(False)),
            right=Scan(table="b"),
            kind=JoinKind.LEFT,
            condition=None,
        )
        out = dce(tree)
        assert isinstance(out, Join)

    def test_project_empty_preserves_columns(self) -> None:
        tree = Project(
            input=Filter(input=Scan(table="t"), predicate=Literal(False)),
            items=(ProjectionItem(expr=Wildcard(), alias=None),),
        )
        out = dce(tree)
        assert isinstance(out, EmptyResult)


class TestLimitPushdownExtras:
    def test_distinct_stops_push(self) -> None:
        tree = Limit(input=Distinct(input=Scan(table="t")), count=5)
        out = lp(tree)
        scan = out.input.input
        assert scan.scan_limit is None

    def test_scan_passthrough(self) -> None:
        assert lp(Scan(table="t")) == Scan(table="t")

    def test_filter_recurses_without_limit(self) -> None:
        # A Filter without an enclosing Limit just recurses.
        tree = Filter(input=Scan(table="t"), predicate=Literal(True))
        assert lp(tree) == tree

    def test_limit_preserves_tighter_existing(self) -> None:
        tree = Limit(
            input=Project(
                input=Scan(table="t", scan_limit=3),
                items=(ProjectionItem(expr=Column("t", "x"), alias="x"),),
            ),
            count=10,
        )
        out = lp(tree)
        assert out.input.input.scan_limit == 3

    def test_aggregate_inside_limit(self) -> None:
        tree = Limit(
            input=Aggregate(input=Scan(table="t"), group_by=(), aggregates=()),
            count=5,
        )
        out = lp(tree)
        # Cannot push through aggregate.
        assert out.input.input.scan_limit is None

    def test_union_recurses(self) -> None:
        tree = Union(left=Scan(table="a"), right=Scan(table="b"))
        assert lp(tree) == tree

    def test_join_recurses(self) -> None:
        tree = Join(
            left=Scan(table="a"),
            right=Scan(table="b"),
            kind=JoinKind.CROSS,
            condition=None,
        )
        assert lp(tree) == tree

    def test_having_recurses(self) -> None:
        tree = Having(input=Scan(table="t"), predicate=Literal(True))
        assert lp(tree) == tree

    def test_sort_recurses_without_limit(self) -> None:
        tree = Sort(input=Scan(table="t"), keys=(SortKey(expr=Column("t", "x")),))
        assert lp(tree) == tree
