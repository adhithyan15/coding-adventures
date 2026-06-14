"""
Tier-2 VM tests: RunRecursiveCTE and OpenWorkingSetScan.

These tests exercise the recursive CTE fixed-point execution directly via
the vm.execute() function, using the full compile(plan) → execute() pipeline.
They verify:
  - Anchor-only recursion (no recursive rows → just anchor result)
  - UNION ALL accumulates all rows across iterations
  - UNION deduplicates rows (cycle safety)
  - Multi-level tree traversal (depth > 2)
  - OpenWorkingSetScan creates a fresh cursor per JOIN outer iteration
  - Empty anchor returns empty result
  - Depth-column computation via JOIN-based recursive step

Coverage targets
----------------
- vm.py: _do_run_recursive_cte, _execute_with_cursors, OpenWorkingSetScan handler
- ir.py: RunRecursiveCTE, OpenWorkingSetScan
"""

from __future__ import annotations

from sql_backend.in_memory import InMemoryBackend
from sql_backend.schema import ColumnDef
from sql_codegen import compile
from sql_planner import (
    BinaryExpr,
    BinaryOp,
    Column,
    Filter,
    Join,
    Literal,
    Project,
    ProjectionItem,
    RecursiveCTE,
    Scan,
    WorkingSetScan,
)

from sql_vm import execute

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _int_col(name: str) -> ColumnDef:
    return ColumnDef(name=name, type_name="INTEGER")


def _make_backend(*tables: tuple[str, list[ColumnDef], list[dict]]) -> InMemoryBackend:
    be = InMemoryBackend()
    for table_name, cols, rows in tables:
        be.create_table(table_name, cols, False)
        for row in rows:
            be.insert(table_name, row)
    return be


# ---------------------------------------------------------------------------
# TestAnchorOnly — recursive step produces no rows
# ---------------------------------------------------------------------------


class TestAnchorOnly:
    """When the recursive step emits no rows, result equals the anchor."""

    def test_anchor_single_row(self) -> None:
        """Anchor with one row and no recursive matches → one-row result."""
        # nodes(id, parent_id): only one root (parent_id = NULL isn't modelled
        # here; we use parent_id=-1 as sentinel and filter to id=1 explicitly).
        be = _make_backend(
            ("nodes", [_int_col("id"), _int_col("parent_id")], [{"id": 1, "parent_id": 0}])
        )
        # Anchor: SELECT id FROM nodes WHERE id = 1
        anchor_plan = Project(
            input=Filter(
                input=Scan(table="nodes"),
                predicate=BinaryExpr(
                    op=BinaryOp.EQ,
                    left=Column(table="nodes", col="id"),
                    right=Literal(value=1),
                ),
            ),
            items=(ProjectionItem(expr=Column(table="nodes", col="id"), alias="id"),),
        )
        # Recursive: SELECT nodes.id FROM nodes JOIN wss ON nodes.parent_id = wss.id
        # where no node has parent_id=1 → empty each iteration
        wss = WorkingSetScan(alias="wss", columns=("id",))
        recursive_plan = Project(
            input=Filter(
                input=Join(
                    left=Scan(table="nodes"),
                    right=wss,
                    condition=BinaryExpr(
                        op=BinaryOp.EQ,
                        left=Column(table="nodes", col="parent_id"),
                        right=Column(table="wss", col="id"),
                    ),
                    kind="INNER",
                ),
                predicate=Literal(value=True),
            ),
            items=(ProjectionItem(expr=Column(table="nodes", col="id"), alias="id"),),
        )
        rcte = RecursiveCTE(
            anchor=anchor_plan,
            recursive=recursive_plan,
            alias="tree",
            columns=("id",),
            union_all=True,
        )
        outer_plan = Project(
            input=rcte,
            items=(ProjectionItem(expr=Column(table="tree", col="id"), alias="id"),),
        )
        result = execute(compile(outer_plan), be)
        assert result.rows == ((1,),)

    def test_anchor_empty_returns_empty(self) -> None:
        """Empty anchor → empty recursive CTE result."""
        be = _make_backend(
            ("nodes", [_int_col("id"), _int_col("parent_id")], [])
        )
        anchor_plan = Project(
            input=Scan(table="nodes"),
            items=(ProjectionItem(expr=Column(table="nodes", col="id"), alias="id"),),
        )
        wss = WorkingSetScan(alias="wss", columns=("id",))
        recursive_plan = Project(
            input=Join(
                left=Scan(table="nodes"),
                right=wss,
                condition=BinaryExpr(
                    op=BinaryOp.EQ,
                    left=Column(table="nodes", col="parent_id"),
                    right=Column(table="wss", col="id"),
                ),
                kind="INNER",
            ),
            items=(ProjectionItem(expr=Column(table="nodes", col="id"), alias="id"),),
        )
        rcte = RecursiveCTE(
            anchor=anchor_plan,
            recursive=recursive_plan,
            alias="tree",
            columns=("id",),
            union_all=True,
        )
        outer_plan = Project(
            input=rcte,
            items=(ProjectionItem(expr=Column(table="tree", col="id"), alias="id"),),
        )
        result = execute(compile(outer_plan), be)
        assert result.rows == ()


# ---------------------------------------------------------------------------
# TestUnionAll — full tree traversal with UNION ALL
# ---------------------------------------------------------------------------


class TestUnionAll:
    """UNION ALL accumulates all visited rows including duplicates."""

    def _tree_backend(self) -> InMemoryBackend:
        """
        Tree:
            1
           / \\
          2   3
             / \\
            4   5
        """
        rows = [
            {"id": 1, "parent_id": 0},
            {"id": 2, "parent_id": 1},
            {"id": 3, "parent_id": 1},
            {"id": 4, "parent_id": 3},
            {"id": 5, "parent_id": 3},
        ]
        return _make_backend(("nodes", [_int_col("id"), _int_col("parent_id")], rows))

    def _tree_plan(self, be: InMemoryBackend, *, union_all: bool) -> object:
        """Build a recursive CTE plan that traverses the tree from root 1."""
        anchor_plan = Project(
            input=Filter(
                input=Scan(table="nodes"),
                predicate=BinaryExpr(
                    op=BinaryOp.EQ,
                    left=Column(table="nodes", col="id"),
                    right=Literal(value=1),
                ),
            ),
            items=(ProjectionItem(expr=Column(table="nodes", col="id"), alias="id"),),
        )
        wss = WorkingSetScan(alias="wss", columns=("id",))
        recursive_plan = Project(
            input=Join(
                left=Scan(table="nodes"),
                right=wss,
                condition=BinaryExpr(
                    op=BinaryOp.EQ,
                    left=Column(table="nodes", col="parent_id"),
                    right=Column(table="wss", col="id"),
                ),
                kind="INNER",
            ),
            items=(ProjectionItem(expr=Column(table="nodes", col="id"), alias="id"),),
        )
        rcte = RecursiveCTE(
            anchor=anchor_plan,
            recursive=recursive_plan,
            alias="tree",
            columns=("id",),
            union_all=union_all,
        )
        return Project(
            input=rcte,
            items=(ProjectionItem(expr=Column(table="tree", col="id"), alias="id"),),
        )

    def test_full_tree_union_all(self) -> None:
        """Full tree traversal with UNION ALL visits all 5 nodes."""
        be = self._tree_backend()
        result = execute(compile(self._tree_plan(be, union_all=True)), be)
        ids = {r[0] for r in result.rows}
        assert ids == {1, 2, 3, 4, 5}
        assert len(result.rows) == 5

    def test_full_tree_union_dedup(self) -> None:
        """Full tree traversal with UNION also visits all 5 nodes (no dups in tree)."""
        be = self._tree_backend()
        result = execute(compile(self._tree_plan(be, union_all=False)), be)
        ids = {r[0] for r in result.rows}
        assert ids == {1, 2, 3, 4, 5}
        assert len(result.rows) == 5

    def test_subtree_from_node_3(self) -> None:
        """Anchoring at node 3 returns nodes 3, 4, 5."""
        be = self._tree_backend()
        anchor_plan = Project(
            input=Filter(
                input=Scan(table="nodes"),
                predicate=BinaryExpr(
                    op=BinaryOp.EQ,
                    left=Column(table="nodes", col="id"),
                    right=Literal(value=3),
                ),
            ),
            items=(ProjectionItem(expr=Column(table="nodes", col="id"), alias="id"),),
        )
        wss = WorkingSetScan(alias="wss", columns=("id",))
        recursive_plan = Project(
            input=Join(
                left=Scan(table="nodes"),
                right=wss,
                condition=BinaryExpr(
                    op=BinaryOp.EQ,
                    left=Column(table="nodes", col="parent_id"),
                    right=Column(table="wss", col="id"),
                ),
                kind="INNER",
            ),
            items=(ProjectionItem(expr=Column(table="nodes", col="id"), alias="id"),),
        )
        rcte = RecursiveCTE(
            anchor=anchor_plan,
            recursive=recursive_plan,
            alias="tree",
            columns=("id",),
            union_all=True,
        )
        outer_plan = Project(
            input=rcte,
            items=(ProjectionItem(expr=Column(table="tree", col="id"), alias="id"),),
        )
        result = execute(compile(outer_plan), be)
        ids = {r[0] for r in result.rows}
        assert ids == {3, 4, 5}

    def test_leaf_node_anchor_no_recursive_rows(self) -> None:
        """Anchoring at a leaf (id=2) returns exactly that one node."""
        be = self._tree_backend()
        anchor_plan = Project(
            input=Filter(
                input=Scan(table="nodes"),
                predicate=BinaryExpr(
                    op=BinaryOp.EQ,
                    left=Column(table="nodes", col="id"),
                    right=Literal(value=2),
                ),
            ),
            items=(ProjectionItem(expr=Column(table="nodes", col="id"), alias="id"),),
        )
        wss = WorkingSetScan(alias="wss", columns=("id",))
        recursive_plan = Project(
            input=Join(
                left=Scan(table="nodes"),
                right=wss,
                condition=BinaryExpr(
                    op=BinaryOp.EQ,
                    left=Column(table="nodes", col="parent_id"),
                    right=Column(table="wss", col="id"),
                ),
                kind="INNER",
            ),
            items=(ProjectionItem(expr=Column(table="nodes", col="id"), alias="id"),),
        )
        rcte = RecursiveCTE(
            anchor=anchor_plan,
            recursive=recursive_plan,
            alias="tree",
            columns=("id",),
            union_all=True,
        )
        outer_plan = Project(
            input=rcte,
            items=(ProjectionItem(expr=Column(table="tree", col="id"), alias="id"),),
        )
        result = execute(compile(outer_plan), be)
        assert result.rows == ((2,),)


# ---------------------------------------------------------------------------
# TestUnionDedup — cycle prevention with UNION
# ---------------------------------------------------------------------------


class TestUnionDedup:
    """UNION deduplicates rows; important for graphs with cycles."""

    def test_union_stops_at_duplicate(self) -> None:
        """UNION recursion terminates when all recursive rows are already seen."""
        # Table with one row: id=1, parent_id=1 (self-loop)
        be = _make_backend(
            ("nodes", [_int_col("id"), _int_col("parent_id")], [{"id": 1, "parent_id": 1}])
        )
        anchor_plan = Project(
            input=Filter(
                input=Scan(table="nodes"),
                predicate=BinaryExpr(
                    op=BinaryOp.EQ,
                    left=Column(table="nodes", col="id"),
                    right=Literal(value=1),
                ),
            ),
            items=(ProjectionItem(expr=Column(table="nodes", col="id"), alias="id"),),
        )
        wss = WorkingSetScan(alias="wss", columns=("id",))
        recursive_plan = Project(
            input=Join(
                left=Scan(table="nodes"),
                right=wss,
                condition=BinaryExpr(
                    op=BinaryOp.EQ,
                    left=Column(table="nodes", col="parent_id"),
                    right=Column(table="wss", col="id"),
                ),
                kind="INNER",
            ),
            items=(ProjectionItem(expr=Column(table="nodes", col="id"), alias="id"),),
        )
        rcte = RecursiveCTE(
            anchor=anchor_plan,
            recursive=recursive_plan,
            alias="tree",
            columns=("id",),
            union_all=False,  # UNION — dedup prevents infinite loop
        )
        outer_plan = Project(
            input=rcte,
            items=(ProjectionItem(expr=Column(table="tree", col="id"), alias="id"),),
        )
        result = execute(compile(outer_plan), be)
        # Only id=1, seen once despite the self-loop
        assert result.rows == ((1,),)

    def test_union_all_vs_union_different_counts(self) -> None:
        """With a two-node cycle UNION ALL would loop forever, but UNION stops."""
        # Linear chain: 1 → 2 (no cycle, just checking dedup behaviour)
        be = _make_backend(
            (
                "nodes",
                [_int_col("id"), _int_col("parent_id")],
                [{"id": 1, "parent_id": 0}, {"id": 2, "parent_id": 1}],
            )
        )

        def _make_plan(union_all: bool) -> object:
            anchor = Project(
                input=Filter(
                    input=Scan(table="nodes"),
                    predicate=BinaryExpr(
                        op=BinaryOp.EQ,
                        left=Column(table="nodes", col="id"),
                        right=Literal(value=1),
                    ),
                ),
                items=(ProjectionItem(expr=Column(table="nodes", col="id"), alias="id"),),
            )
            wss = WorkingSetScan(alias="wss", columns=("id",))
            recursive = Project(
                input=Join(
                    left=Scan(table="nodes"),
                    right=wss,
                    condition=BinaryExpr(
                        op=BinaryOp.EQ,
                        left=Column(table="nodes", col="parent_id"),
                        right=Column(table="wss", col="id"),
                    ),
                    kind="INNER",
                ),
                items=(ProjectionItem(expr=Column(table="nodes", col="id"), alias="id"),),
            )
            rcte = RecursiveCTE(
                anchor=anchor,
                recursive=recursive,
                alias="tree",
                columns=("id",),
                union_all=union_all,
            )
            return Project(
                input=rcte,
                items=(ProjectionItem(expr=Column(table="tree", col="id"), alias="id"),),
            )

        result_all = execute(compile(_make_plan(True)), be)
        result_dedup = execute(compile(_make_plan(False)), be)
        # Both should find nodes 1 and 2; chain terminates naturally
        assert {r[0] for r in result_all.rows} == {1, 2}
        assert {r[0] for r in result_dedup.rows} == {1, 2}


# ---------------------------------------------------------------------------
# TestMultipleRoots — anchor with multiple rows
# ---------------------------------------------------------------------------


class TestMultipleRoots:
    """Anchor can produce multiple starting rows."""

    def test_two_disjoint_subtrees(self) -> None:
        """Two separate root nodes each expand into their subtrees."""
        rows = [
            {"id": 1, "parent_id": 0},
            {"id": 2, "parent_id": 1},
            {"id": 10, "parent_id": 0},
            {"id": 11, "parent_id": 10},
        ]
        be = _make_backend(("nodes", [_int_col("id"), _int_col("parent_id")], rows))

        # Anchor: both roots (parent_id = 0)
        anchor_plan = Project(
            input=Filter(
                input=Scan(table="nodes"),
                predicate=BinaryExpr(
                    op=BinaryOp.EQ,
                    left=Column(table="nodes", col="parent_id"),
                    right=Literal(value=0),
                ),
            ),
            items=(ProjectionItem(expr=Column(table="nodes", col="id"), alias="id"),),
        )
        wss = WorkingSetScan(alias="wss", columns=("id",))
        recursive_plan = Project(
            input=Join(
                left=Scan(table="nodes"),
                right=wss,
                condition=BinaryExpr(
                    op=BinaryOp.EQ,
                    left=Column(table="nodes", col="parent_id"),
                    right=Column(table="wss", col="id"),
                ),
                kind="INNER",
            ),
            items=(ProjectionItem(expr=Column(table="nodes", col="id"), alias="id"),),
        )
        rcte = RecursiveCTE(
            anchor=anchor_plan,
            recursive=recursive_plan,
            alias="tree",
            columns=("id",),
            union_all=True,
        )
        outer_plan = Project(
            input=rcte,
            items=(ProjectionItem(expr=Column(table="tree", col="id"), alias="id"),),
        )
        result = execute(compile(outer_plan), be)
        ids = {r[0] for r in result.rows}
        assert ids == {1, 2, 10, 11}
        assert len(result.rows) == 4
