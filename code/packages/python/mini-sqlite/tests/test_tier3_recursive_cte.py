"""
Phase 5b: Recursive Common Table Expressions (WITH RECURSIVE).

Tests are organised in three classes:

  TestRecursiveCTEGrammar       — grammar / adapter / planner unit tests
  TestRecursiveCTEIntegration   — end-to-end SQL correctness tests
  TestRecursiveCTEErrors        — error / edge-case tests

Note on SQL style: the grammar requires explicit ``AS`` for table aliases
(``nodes AS n``, not bare ``nodes n``).
"""

from __future__ import annotations

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_conn() -> object:
    from mini_sqlite import connect  # type: ignore[import]

    return connect(":memory:")


def _setup_tree(conn: object) -> None:
    """Create a simple tree table used by most tests.

    Hierarchy::

        1 (root)
        ├── 2
        │   ├── 4
        │   └── 5
        └── 3
    """
    conn.execute("CREATE TABLE nodes (id INTEGER, parent_id INTEGER, name TEXT)")
    conn.execute("INSERT INTO nodes VALUES (1, NULL, 'root')")
    conn.execute("INSERT INTO nodes VALUES (2, 1,    'child1')")
    conn.execute("INSERT INTO nodes VALUES (3, 1,    'child2')")
    conn.execute("INSERT INTO nodes VALUES (4, 2,    'grandchild1')")
    conn.execute("INSERT INTO nodes VALUES (5, 2,    'grandchild2')")


def _setup_employee_hierarchy(conn: object) -> None:
    """Create a small employee / manager table.

    Hierarchy::

        1 = CEO
        └── 2 = VP
            └── 3 = Manager
                └── 4 = Staff
    """
    conn.execute("CREATE TABLE employees (id INTEGER, manager_id INTEGER, name TEXT)")
    conn.execute("INSERT INTO employees VALUES (1, NULL, 'CEO')")
    conn.execute("INSERT INTO employees VALUES (2, 1,    'VP')")
    conn.execute("INSERT INTO employees VALUES (3, 2,    'Manager')")
    conn.execute("INSERT INTO employees VALUES (4, 3,    'Staff')")


# ---------------------------------------------------------------------------
# TestRecursiveCTEGrammar — pipeline unit tests
# ---------------------------------------------------------------------------


class TestRecursiveCTEGrammar:
    """Verify that WITH RECURSIVE parses and the adapter produces RecursiveCTERef."""

    def test_grammar_parses_recursive_cte(self) -> None:
        """WITH RECURSIVE … UNION ALL … parses to a valid AST."""
        from sql_parser import parse_sql  # type: ignore[import]

        sql = (
            "WITH RECURSIVE t AS ("
            "  SELECT id, parent_id FROM nodes WHERE parent_id IS NULL"
            "  UNION ALL"
            "  SELECT n.id, n.parent_id FROM nodes AS n INNER JOIN t ON n.parent_id = t.id"
            ") SELECT id FROM t"
        )
        tree = parse_sql(sql)
        assert tree is not None
        assert tree.rule_name == "program"

    def test_adapter_produces_recursive_cte_ref(self) -> None:
        """Adapter converts WITH RECURSIVE body to a RecursiveCTERef in from_."""
        from sql_parser import parse_sql  # type: ignore[import]
        from sql_planner import RecursiveCTERef, SelectStmt  # type: ignore[import]

        from mini_sqlite.adapter import to_statement  # type: ignore[import]

        sql = (
            "WITH RECURSIVE t AS ("
            "  SELECT id, parent_id FROM nodes WHERE parent_id IS NULL"
            "  UNION ALL"
            "  SELECT n.id, n.parent_id FROM nodes AS n INNER JOIN t ON n.parent_id = t.id"
            ") SELECT id FROM t"
        )
        tree = parse_sql(sql)
        stmt = to_statement(tree)
        assert isinstance(stmt, SelectStmt)
        assert isinstance(stmt.from_, RecursiveCTERef)

    def test_adapter_recursive_cte_ref_has_anchor_and_recursive(self) -> None:
        """RecursiveCTERef carries the correct anchor and recursive SelectStmts."""
        from sql_parser import parse_sql  # type: ignore[import]
        from sql_planner import RecursiveCTERef, SelectStmt, TableRef  # type: ignore[import]

        from mini_sqlite.adapter import to_statement  # type: ignore[import]

        sql = (
            "WITH RECURSIVE t AS ("
            "  SELECT id, parent_id FROM nodes WHERE parent_id IS NULL"
            "  UNION ALL"
            "  SELECT n.id, n.parent_id FROM nodes AS n INNER JOIN t ON n.parent_id = t.id"
            ") SELECT id FROM t"
        )
        tree = parse_sql(sql)
        stmt = to_statement(tree)
        assert isinstance(stmt, SelectStmt)
        ref = stmt.from_
        assert isinstance(ref, RecursiveCTERef)
        assert ref.name == "t"
        assert ref.union_all is True
        assert isinstance(ref.anchor, SelectStmt)
        # The anchor FROM is the base table 'nodes'.
        assert isinstance(ref.anchor.from_, TableRef)
        assert ref.anchor.from_.table == "nodes"
        # The recursive step also starts from 'nodes'; the self-reference is a
        # plain TableRef(table='t') that the planner converts to WorkingSetScan.
        assert isinstance(ref.recursive, SelectStmt)

    def test_adapter_union_all_flag(self) -> None:
        """UNION ALL sets union_all=True; UNION sets union_all=False."""
        from sql_parser import parse_sql  # type: ignore[import]
        from sql_planner import RecursiveCTERef, SelectStmt  # type: ignore[import]

        from mini_sqlite.adapter import to_statement  # type: ignore[import]

        sql_all = (
            "WITH RECURSIVE t AS ("
            "  SELECT id FROM nodes WHERE parent_id IS NULL"
            "  UNION ALL"
            "  SELECT n.id FROM nodes AS n INNER JOIN t ON n.parent_id = t.id"
            ") SELECT id FROM t"
        )
        sql_dedup = (
            "WITH RECURSIVE t AS ("
            "  SELECT id FROM nodes WHERE parent_id IS NULL"
            "  UNION"
            "  SELECT n.id FROM nodes AS n INNER JOIN t ON n.parent_id = t.id"
            ") SELECT id FROM t"
        )
        for sql, expected_all in [(sql_all, True), (sql_dedup, False)]:
            tree = parse_sql(sql)
            stmt = to_statement(tree)
            assert isinstance(stmt, SelectStmt)
            ref = stmt.from_
            assert isinstance(ref, RecursiveCTERef)
            assert ref.union_all is expected_all, f"Expected union_all={expected_all}"

    def test_planner_produces_recursive_cte_plan_node(self) -> None:
        """Planner converts RecursiveCTERef to a RecursiveCTE plan node."""
        from sql_parser import parse_sql  # type: ignore[import]
        from sql_planner import (  # type: ignore[import]
            InMemorySchemaProvider,  # type: ignore[import]
            RecursiveCTE,
            WorkingSetScan,
            plan,
        )
        from sql_planner.plan import children  # type: ignore[import]

        from mini_sqlite.adapter import to_statement  # type: ignore[import]

        sql = (
            "WITH RECURSIVE t AS ("
            "  SELECT id, parent_id FROM nodes WHERE parent_id IS NULL"
            "  UNION ALL"
            "  SELECT n.id, n.parent_id FROM nodes AS n INNER JOIN t ON n.parent_id = t.id"
            ") SELECT id FROM t"
        )
        schema = InMemorySchemaProvider(
            {"nodes": ["id", "parent_id", "name"]}
        )
        tree = parse_sql(sql)
        stmt = to_statement(tree)
        logical = plan(stmt, schema)

        def _find(node: object, kind: type) -> object | None:
            if isinstance(node, kind):
                return node
            for child in children(node):  # type: ignore[arg-type]
                result = _find(child, kind)
                if result is not None:
                    return result
            return None

        rcte = _find(logical, RecursiveCTE)
        assert rcte is not None, "Expected a RecursiveCTE plan node"
        assert isinstance(rcte, RecursiveCTE)

        # The recursive sub-plan must contain a WorkingSetScan.
        wss = _find(rcte.recursive, WorkingSetScan)
        assert wss is not None, "Expected WorkingSetScan inside recursive plan"

    def test_non_recursive_cte_still_works(self) -> None:
        """WITH (without RECURSIVE) still resolves as DerivedTableRef / DerivedTable."""
        conn = _make_conn()
        _setup_tree(conn)
        cur = conn.execute(
            "WITH direct AS (SELECT id, name FROM nodes WHERE parent_id IS NULL)"
            " SELECT id, name FROM direct"
        )
        rows = cur.fetchall()
        assert len(rows) == 1
        assert rows[0][0] == 1


# ---------------------------------------------------------------------------
# TestRecursiveCTEIntegration — end-to-end correctness tests
# ---------------------------------------------------------------------------


class TestRecursiveCTEIntegration:
    """End-to-end tests that run full SQL through the mini-sqlite stack."""

    # ------------------------------------------------------------------
    # Basic tree traversal
    # ------------------------------------------------------------------

    def test_full_tree_traversal_union_all(self) -> None:
        """Recursive CTE returns every node in the tree (UNION ALL)."""
        conn = _make_conn()
        _setup_tree(conn)
        sql = (
            "WITH RECURSIVE t AS ("
            "  SELECT id, parent_id, name FROM nodes WHERE parent_id IS NULL"
            "  UNION ALL"
            "  SELECT n.id, n.parent_id, n.name FROM nodes AS n"
            "  INNER JOIN t ON n.parent_id = t.id"
            ") SELECT id FROM t ORDER BY id"
        )
        cur = conn.execute(sql)
        ids = [row[0] for row in cur.fetchall()]
        assert ids == [1, 2, 3, 4, 5]

    def test_subtree_traversal(self) -> None:
        """Start from a non-root node, get only its subtree."""
        conn = _make_conn()
        _setup_tree(conn)
        sql = (
            "WITH RECURSIVE sub AS ("
            "  SELECT id, parent_id FROM nodes WHERE id = 2"
            "  UNION ALL"
            "  SELECT n.id, n.parent_id FROM nodes AS n"
            "  INNER JOIN sub ON n.parent_id = sub.id"
            ") SELECT id FROM sub ORDER BY id"
        )
        cur = conn.execute(sql)
        ids = [row[0] for row in cur.fetchall()]
        assert ids == [2, 4, 5]

    def test_org_chart_full_traversal(self) -> None:
        """Linear chain: CEO → VP → Manager → Staff."""
        conn = _make_conn()
        _setup_employee_hierarchy(conn)
        sql = (
            "WITH RECURSIVE org AS ("
            "  SELECT id, manager_id, name FROM employees WHERE manager_id IS NULL"
            "  UNION ALL"
            "  SELECT e.id, e.manager_id, e.name FROM employees AS e"
            "  INNER JOIN org ON e.manager_id = org.id"
            ") SELECT id, name FROM org ORDER BY id"
        )
        cur = conn.execute(sql)
        rows = cur.fetchall()
        assert len(rows) == 4
        assert rows[0] == (1, "CEO")
        assert rows[3] == (4, "Staff")

    # ------------------------------------------------------------------
    # Filtering on the outer query
    # ------------------------------------------------------------------

    def test_filter_on_recursive_result(self) -> None:
        """WHERE on the outer query filters the recursive result correctly."""
        conn = _make_conn()
        _setup_tree(conn)
        sql = (
            "WITH RECURSIVE t AS ("
            "  SELECT id, parent_id, name FROM nodes WHERE parent_id IS NULL"
            "  UNION ALL"
            "  SELECT n.id, n.parent_id, n.name FROM nodes AS n"
            "  INNER JOIN t ON n.parent_id = t.id"
            ") SELECT id, name FROM t WHERE parent_id IS NULL"
        )
        cur = conn.execute(sql)
        rows = cur.fetchall()
        # Only the root has parent_id IS NULL.
        assert len(rows) == 1
        assert rows[0][0] == 1

    def test_count_aggregate_on_recursive_result(self) -> None:
        """Aggregate (COUNT) applied to the recursive CTE result."""
        conn = _make_conn()
        _setup_tree(conn)
        sql = (
            "WITH RECURSIVE t AS ("
            "  SELECT id, parent_id FROM nodes WHERE parent_id IS NULL"
            "  UNION ALL"
            "  SELECT n.id, n.parent_id FROM nodes AS n"
            "  INNER JOIN t ON n.parent_id = t.id"
            ") SELECT COUNT(id) FROM t"
        )
        cur = conn.execute(sql)
        rows = cur.fetchall()
        assert rows[0][0] == 5

    # ------------------------------------------------------------------
    # UNION (deduplication) mode
    # ------------------------------------------------------------------

    def test_union_dedup_no_duplicates_in_simple_tree(self) -> None:
        """UNION deduplication produces the same result for a tree (no cycles)."""
        conn = _make_conn()
        _setup_tree(conn)
        sql_all = (
            "WITH RECURSIVE t AS ("
            "  SELECT id, parent_id FROM nodes WHERE parent_id IS NULL"
            "  UNION ALL"
            "  SELECT n.id, n.parent_id FROM nodes AS n"
            "  INNER JOIN t ON n.parent_id = t.id"
            ") SELECT id FROM t ORDER BY id"
        )
        sql_dedup = (
            "WITH RECURSIVE t AS ("
            "  SELECT id, parent_id FROM nodes WHERE parent_id IS NULL"
            "  UNION"
            "  SELECT n.id, n.parent_id FROM nodes AS n"
            "  INNER JOIN t ON n.parent_id = t.id"
            ") SELECT id FROM t ORDER BY id"
        )
        ids_all = [r[0] for r in conn.execute(sql_all).fetchall()]
        ids_dedup = [r[0] for r in conn.execute(sql_dedup).fetchall()]
        # For a pure tree (no cycles) UNION and UNION ALL return the same set.
        assert sorted(ids_all) == sorted(ids_dedup)

    # ------------------------------------------------------------------
    # Depth / level computation
    # ------------------------------------------------------------------

    def test_depth_computation(self) -> None:
        """Carry a depth counter through recursion."""
        conn = _make_conn()
        _setup_employee_hierarchy(conn)
        sql = (
            "WITH RECURSIVE org AS ("
            "  SELECT id, manager_id, name, 0 AS depth"
            "  FROM employees WHERE manager_id IS NULL"
            "  UNION ALL"
            "  SELECT e.id, e.manager_id, e.name, org.depth + 1"
            "  FROM employees AS e INNER JOIN org ON e.manager_id = org.id"
            ") SELECT id, depth FROM org ORDER BY id"
        )
        cur = conn.execute(sql)
        rows = cur.fetchall()
        assert len(rows) == 4
        depths = {r[0]: r[1] for r in rows}
        assert depths[1] == 0  # CEO at depth 0
        assert depths[2] == 1
        assert depths[3] == 2
        assert depths[4] == 3

    # ------------------------------------------------------------------
    # Anchor returns multiple root rows
    # ------------------------------------------------------------------

    def test_multiple_root_nodes(self) -> None:
        """Anchor returns >1 row; recursion explores all starting points."""
        conn = _make_conn()
        conn.execute("CREATE TABLE forest (id INTEGER, parent_id INTEGER)")
        # Two separate trees: 1→2, 1→3; and 10→11
        conn.execute("INSERT INTO forest VALUES (1,  NULL)")
        conn.execute("INSERT INTO forest VALUES (2,  1)")
        conn.execute("INSERT INTO forest VALUES (3,  1)")
        conn.execute("INSERT INTO forest VALUES (10, NULL)")
        conn.execute("INSERT INTO forest VALUES (11, 10)")
        sql = (
            "WITH RECURSIVE t AS ("
            "  SELECT id, parent_id FROM forest WHERE parent_id IS NULL"
            "  UNION ALL"
            "  SELECT f.id, f.parent_id FROM forest AS f"
            "  INNER JOIN t ON f.parent_id = t.id"
            ") SELECT id FROM t ORDER BY id"
        )
        cur = conn.execute(sql)
        ids = [r[0] for r in cur.fetchall()]
        assert ids == [1, 2, 3, 10, 11]

    # ------------------------------------------------------------------
    # Anchor returns no rows → empty result
    # ------------------------------------------------------------------

    def test_empty_anchor_returns_empty_result(self) -> None:
        """When the anchor SELECT returns no rows the result is empty."""
        conn = _make_conn()
        _setup_tree(conn)
        sql = (
            "WITH RECURSIVE t AS ("
            "  SELECT id, parent_id FROM nodes WHERE id = 999"
            "  UNION ALL"
            "  SELECT n.id, n.parent_id FROM nodes AS n"
            "  INNER JOIN t ON n.parent_id = t.id"
            ") SELECT id FROM t"
        )
        cur = conn.execute(sql)
        assert cur.fetchall() == []

    # ------------------------------------------------------------------
    # Leaf nodes (no recursive step rows)
    # ------------------------------------------------------------------

    def test_leaf_anchor_no_recursive_rows(self) -> None:
        """When the anchor is a leaf (no children) the CTE returns just the anchor row."""
        conn = _make_conn()
        _setup_tree(conn)
        sql = (
            "WITH RECURSIVE t AS ("
            "  SELECT id, parent_id FROM nodes WHERE id = 4"
            "  UNION ALL"
            "  SELECT n.id, n.parent_id FROM nodes AS n"
            "  INNER JOIN t ON n.parent_id = t.id"
            ") SELECT id FROM t"
        )
        cur = conn.execute(sql)
        rows = cur.fetchall()
        assert len(rows) == 1
        assert rows[0][0] == 4

    # ------------------------------------------------------------------
    # JOIN inside recursive step
    # ------------------------------------------------------------------

    def test_recursive_step_with_inner_join(self) -> None:
        """The recursive step uses an INNER JOIN to attach to the working set."""
        conn = _make_conn()
        _setup_tree(conn)
        sql = (
            "WITH RECURSIVE t AS ("
            "  SELECT id, parent_id, name FROM nodes WHERE parent_id IS NULL"
            "  UNION ALL"
            "  SELECT n.id, n.parent_id, n.name FROM nodes AS n"
            "  INNER JOIN t ON n.parent_id = t.id"
            ") SELECT id FROM t ORDER BY id"
        )
        ids = [r[0] for r in conn.execute(sql).fetchall()]
        assert ids == [1, 2, 3, 4, 5]


# ---------------------------------------------------------------------------
# TestRecursiveCTEErrors — error / edge-case tests
# ---------------------------------------------------------------------------


class TestRecursiveCTEErrors:
    """Verify correct error handling for recursive CTE edge cases."""

    def test_non_recursive_keyword_still_substitutes_cte(self) -> None:
        """WITH (no RECURSIVE keyword) works for non-recursive CTEs as before."""
        conn = _make_conn()
        _setup_tree(conn)
        cur = conn.execute(
            "WITH roots AS (SELECT id, name FROM nodes WHERE parent_id IS NULL)"
            " SELECT name FROM roots"
        )
        rows = cur.fetchall()
        assert len(rows) == 1
        assert rows[0][0] == "root"

    def test_recursive_cte_with_where_on_anchor(self) -> None:
        """WHERE clause on anchor correctly limits the starting set."""
        conn = _make_conn()
        _setup_tree(conn)
        sql = (
            "WITH RECURSIVE t AS ("
            "  SELECT id, parent_id FROM nodes WHERE id = 2"
            "  UNION ALL"
            "  SELECT n.id, n.parent_id FROM nodes AS n"
            "  INNER JOIN t ON n.parent_id = t.id"
            ") SELECT COUNT(id) FROM t"
        )
        cur = conn.execute(sql)
        assert cur.fetchall()[0][0] == 3  # nodes 2, 4, 5

    def test_recursive_cte_outer_order_by(self) -> None:
        """ORDER BY on the outer SELECT sorts the accumulated recursive result."""
        conn = _make_conn()
        _setup_tree(conn)
        sql = (
            "WITH RECURSIVE t AS ("
            "  SELECT id, parent_id FROM nodes WHERE parent_id IS NULL"
            "  UNION ALL"
            "  SELECT n.id, n.parent_id FROM nodes AS n"
            "  INNER JOIN t ON n.parent_id = t.id"
            ") SELECT id FROM t ORDER BY id DESC"
        )
        ids = [r[0] for r in conn.execute(sql).fetchall()]
        assert ids == [5, 4, 3, 2, 1]

    def test_recursive_cte_outer_limit(self) -> None:
        """LIMIT on the outer SELECT truncates the recursive result."""
        conn = _make_conn()
        _setup_tree(conn)
        sql = (
            "WITH RECURSIVE t AS ("
            "  SELECT id, parent_id FROM nodes WHERE parent_id IS NULL"
            "  UNION ALL"
            "  SELECT n.id, n.parent_id FROM nodes AS n"
            "  INNER JOIN t ON n.parent_id = t.id"
            ") SELECT id FROM t ORDER BY id LIMIT 3"
        )
        ids = [r[0] for r in conn.execute(sql).fetchall()]
        assert len(ids) == 3
        assert ids == [1, 2, 3]

    def test_recursive_cte_with_column_alias(self) -> None:
        """Column aliases in the anchor are preserved through recursive iterations."""
        conn = _make_conn()
        _setup_employee_hierarchy(conn)
        sql = (
            "WITH RECURSIVE org AS ("
            "  SELECT id AS eid, manager_id AS mid FROM employees WHERE manager_id IS NULL"
            "  UNION ALL"
            "  SELECT e.id AS eid, e.manager_id AS mid FROM employees AS e"
            "  INNER JOIN org ON e.manager_id = org.eid"
            ") SELECT eid FROM org ORDER BY eid"
        )
        eids = [r[0] for r in conn.execute(sql).fetchall()]
        assert eids == [1, 2, 3, 4]
