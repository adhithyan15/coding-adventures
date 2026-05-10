"""
Tier-3 integration tests — Extended window functions
=====================================================

End-to-end tests for window functions added in the second increment:
LAG, LEAD, NTILE, PERCENT_RANK, CUME_DIST, NTH_VALUE.

All tests exercise the full pipeline: SQL string → parser → adapter →
planner → codegen → VM → result via ``mini_sqlite.connect(":memory:")``.

Data set
--------
Five employees in two departments, ordered by salary within each dept:

    id  name   dept   salary
    1   Alice  eng    90000
    2   Bob    eng    80000
    3   Carol  sales  70000
    4   Dave   sales  75000
    5   Eve    eng    85000

When sorted by salary ASC within eng: Bob 80k, Eve 85k, Alice 90k
When sorted by salary ASC within sales: Carol 70k, Dave 75k
"""

from __future__ import annotations

import mini_sqlite


def _conn():
    """Return a fresh in-memory connection with the employees table pre-loaded."""
    con = mini_sqlite.connect(":memory:")
    con.execute(
        "CREATE TABLE employees (id INTEGER, name TEXT, dept TEXT, salary INTEGER)"
    )
    con.executemany(
        "INSERT INTO employees VALUES (?, ?, ?, ?)",
        [
            (1, "Alice", "eng",   90000),
            (2, "Bob",   "eng",   80000),
            (3, "Carol", "sales", 70000),
            (4, "Dave",  "sales", 75000),
            (5, "Eve",   "eng",   85000),
        ],
    )
    return con


# ---------------------------------------------------------------------------
# LAG
# ---------------------------------------------------------------------------


class TestLagEndToEnd:
    def test_lag_default_offset_partitioned(self):
        """LAG(salary) OVER (PARTITION BY dept ORDER BY salary) — one row back."""
        con = _conn()
        rows = con.execute(
            """
            SELECT name, dept, salary,
                   LAG(salary) OVER (PARTITION BY dept ORDER BY salary) AS prev_sal
            FROM employees
            ORDER BY dept, salary
            """
        ).fetchall()
        by_name = {r[0]: r[3] for r in rows}
        # eng sorted ASC: Bob 80k→NULL, Eve 85k→80k, Alice 90k→85k
        assert by_name["Bob"] is None
        assert by_name["Eve"] == 80000
        assert by_name["Alice"] == 85000
        # sales sorted ASC: Carol 70k→NULL, Dave 75k→70k
        assert by_name["Carol"] is None
        assert by_name["Dave"] == 70000

    def test_lag_explicit_offset_2(self):
        """LAG(salary, 2) skips two positions back."""
        con = _conn()
        rows = con.execute(
            """
            SELECT name, salary,
                   LAG(salary, 2) OVER (PARTITION BY dept ORDER BY salary) AS lag2
            FROM employees
            WHERE dept = 'eng'
            ORDER BY salary
            """
        ).fetchall()
        by_name = {r[0]: r[2] for r in rows}
        # eng: Bob 80k→NULL, Eve 85k→NULL, Alice 90k→80k
        assert by_name["Bob"] is None
        assert by_name["Eve"] is None
        assert by_name["Alice"] == 80000

    def test_lag_with_explicit_default(self):
        """LAG(salary, 1, -1) uses -1 when no preceding row exists."""
        con = _conn()
        rows = con.execute(
            """
            SELECT name, dept, salary,
                   LAG(salary, 1, -1) OVER (PARTITION BY dept ORDER BY salary) AS prev
            FROM employees
            ORDER BY dept, salary
            """
        ).fetchall()
        by_name = {r[0]: r[3] for r in rows}
        assert by_name["Bob"] == -1      # first in eng partition → default
        assert by_name["Carol"] == -1    # first in sales partition → default
        assert by_name["Alice"] == 85000

    def test_lag_global_no_partition(self):
        """LAG(id, 1) over all rows with no PARTITION BY."""
        con = _conn()
        rows = con.execute(
            """
            SELECT id,
                   LAG(id, 1) OVER (ORDER BY id) AS prev_id
            FROM employees
            ORDER BY id
            """
        ).fetchall()
        by_id = {r[0]: r[1] for r in rows}
        assert by_id[1] is None
        assert by_id[2] == 1
        assert by_id[5] == 4


# ---------------------------------------------------------------------------
# LEAD
# ---------------------------------------------------------------------------


class TestLeadEndToEnd:
    def test_lead_default_offset_partitioned(self):
        """LEAD(salary) OVER (PARTITION BY dept ORDER BY salary) — one row ahead."""
        con = _conn()
        rows = con.execute(
            """
            SELECT name, dept, salary,
                   LEAD(salary) OVER (PARTITION BY dept ORDER BY salary) AS next_sal
            FROM employees
            ORDER BY dept, salary
            """
        ).fetchall()
        by_name = {r[0]: r[3] for r in rows}
        # eng ASC: Bob→85k, Eve→90k, Alice→NULL
        assert by_name["Bob"] == 85000
        assert by_name["Eve"] == 90000
        assert by_name["Alice"] is None
        # sales ASC: Carol→75k, Dave→NULL
        assert by_name["Carol"] == 75000
        assert by_name["Dave"] is None

    def test_lead_explicit_offset_2(self):
        """LEAD(salary, 2) looks two rows ahead."""
        con = _conn()
        rows = con.execute(
            """
            SELECT name, salary,
                   LEAD(salary, 2) OVER (PARTITION BY dept ORDER BY salary) AS lead2
            FROM employees
            WHERE dept = 'eng'
            ORDER BY salary
            """
        ).fetchall()
        by_name = {r[0]: r[2] for r in rows}
        # eng: Bob→90k, Eve→NULL, Alice→NULL
        assert by_name["Bob"] == 90000
        assert by_name["Eve"] is None
        assert by_name["Alice"] is None

    def test_lead_with_default(self):
        """LEAD(salary, 1, 0) fills boundary rows with 0."""
        con = _conn()
        rows = con.execute(
            """
            SELECT name, dept, salary,
                   LEAD(salary, 1, 0) OVER (PARTITION BY dept ORDER BY salary) AS nxt
            FROM employees
            ORDER BY dept, salary
            """
        ).fetchall()
        by_name = {r[0]: r[3] for r in rows}
        assert by_name["Alice"] == 0    # last in eng → default
        assert by_name["Dave"] == 0     # last in sales → default


# ---------------------------------------------------------------------------
# NTILE
# ---------------------------------------------------------------------------


class TestNtileEndToEnd:
    def test_ntile_3_global(self):
        """NTILE(3) over all 5 rows: 2+2+1 distribution."""
        con = _conn()
        rows = con.execute(
            """
            SELECT id, NTILE(3) OVER (ORDER BY id) AS bucket
            FROM employees
            ORDER BY id
            """
        ).fetchall()
        by_id = {r[0]: r[1] for r in rows}
        # ids 1-5 sorted: bucket 1→{1,2}, bucket 2→{3,4}, bucket 3→{5}
        assert by_id[1] == 1
        assert by_id[2] == 1
        assert by_id[3] == 2
        assert by_id[4] == 2
        assert by_id[5] == 3

    def test_ntile_2_partitioned(self):
        """NTILE(2) within each dept partition."""
        con = _conn()
        rows = con.execute(
            """
            SELECT name, dept, salary,
                   NTILE(2) OVER (PARTITION BY dept ORDER BY salary) AS bucket
            FROM employees
            ORDER BY dept, salary
            """
        ).fetchall()
        by_name = {r[0]: r[3] for r in rows}
        # eng (3 rows, 2 buckets): Bob→1, Eve→1, Alice→2
        assert by_name["Bob"] == 1
        assert by_name["Eve"] == 1
        assert by_name["Alice"] == 2
        # sales (2 rows, 2 buckets): Carol→1, Dave→2
        assert by_name["Carol"] == 1
        assert by_name["Dave"] == 2

    def test_ntile_1_bucket_all_same(self):
        """NTILE(1): every row is in bucket 1."""
        con = _conn()
        rows = con.execute(
            "SELECT NTILE(1) OVER (ORDER BY id) AS b FROM employees"
        ).fetchall()
        assert all(r[0] == 1 for r in rows)

    def test_ntile_larger_than_rows(self):
        """NTILE(10) with 5 rows: each row gets its own bucket 1..5."""
        con = _conn()
        rows = con.execute(
            "SELECT id, NTILE(10) OVER (ORDER BY id) AS b FROM employees ORDER BY id"
        ).fetchall()
        buckets = [r[1] for r in rows]
        assert buckets == [1, 2, 3, 4, 5]


# ---------------------------------------------------------------------------
# PERCENT_RANK
# ---------------------------------------------------------------------------


class TestPercentRankEndToEnd:
    def test_percent_rank_global_unique(self):
        """5 distinct salaries → PERCENT_RANK values 0, 0.25, 0.5, 0.75, 1.0."""
        con = _conn()
        rows = con.execute(
            """
            SELECT salary,
                   PERCENT_RANK() OVER (ORDER BY salary) AS pr
            FROM employees
            ORDER BY salary
            """
        ).fetchall()
        prs = [r[1] for r in rows]
        expected = [0.0, 0.25, 0.5, 0.75, 1.0]
        for got, exp in zip(prs, expected, strict=True):
            assert abs(got - exp) < 1e-9, f"Expected {exp}, got {got}"

    def test_percent_rank_partitioned(self):
        """PERCENT_RANK within each dept."""
        con = _conn()
        rows = con.execute(
            """
            SELECT name, dept, salary,
                   PERCENT_RANK() OVER (PARTITION BY dept ORDER BY salary) AS pr
            FROM employees
            ORDER BY dept, salary
            """
        ).fetchall()
        by_name = {r[0]: r[3] for r in rows}
        # eng (3 rows): Bob→0.0, Eve→0.5, Alice→1.0
        assert abs(by_name["Bob"] - 0.0) < 1e-9
        assert abs(by_name["Eve"] - 0.5) < 1e-9
        assert abs(by_name["Alice"] - 1.0) < 1e-9
        # sales (2 rows): Carol→0.0, Dave→1.0
        assert abs(by_name["Carol"] - 0.0) < 1e-9
        assert abs(by_name["Dave"] - 1.0) < 1e-9

    def test_percent_rank_with_ties(self):
        """Tied rows share the same PERCENT_RANK."""
        con = mini_sqlite.connect(":memory:")
        con.execute("CREATE TABLE t (v INTEGER)")
        con.executemany("INSERT INTO t VALUES (?)", [(1,), (1,), (2,)])
        rows = con.execute(
            "SELECT v, PERCENT_RANK() OVER (ORDER BY v) AS pr FROM t ORDER BY v"
        ).fetchall()
        # v=1 tie at rank 1: PR = 0/2 = 0.0; v=2 at rank 3: PR = 2/2 = 1.0
        prs_by_v = {r[0]: r[1] for r in rows}
        assert abs(prs_by_v[1] - 0.0) < 1e-9
        assert abs(prs_by_v[2] - 1.0) < 1e-9


# ---------------------------------------------------------------------------
# CUME_DIST
# ---------------------------------------------------------------------------


class TestCumeDistEndToEnd:
    def test_cume_dist_global_unique(self):
        """5 distinct salaries → CUME_DIST values 0.2, 0.4, 0.6, 0.8, 1.0."""
        con = _conn()
        rows = con.execute(
            """
            SELECT salary,
                   CUME_DIST() OVER (ORDER BY salary) AS cd
            FROM employees
            ORDER BY salary
            """
        ).fetchall()
        cds = [r[1] for r in rows]
        expected = [0.2, 0.4, 0.6, 0.8, 1.0]
        for got, exp in zip(cds, expected, strict=True):
            assert abs(got - exp) < 1e-9

    def test_cume_dist_partitioned(self):
        """CUME_DIST within each dept."""
        con = _conn()
        rows = con.execute(
            """
            SELECT name, dept, salary,
                   CUME_DIST() OVER (PARTITION BY dept ORDER BY salary) AS cd
            FROM employees
            ORDER BY dept, salary
            """
        ).fetchall()
        by_name = {r[0]: r[3] for r in rows}
        # eng (3 rows): Bob→1/3, Eve→2/3, Alice→1.0
        assert abs(by_name["Bob"]   - 1/3) < 1e-9
        assert abs(by_name["Eve"]   - 2/3) < 1e-9
        assert abs(by_name["Alice"] - 1.0) < 1e-9
        # sales (2 rows): Carol→0.5, Dave→1.0
        assert abs(by_name["Carol"] - 0.5) < 1e-9
        assert abs(by_name["Dave"]  - 1.0) < 1e-9

    def test_cume_dist_with_ties(self):
        """Tied rows share the peer-group endpoint."""
        con = mini_sqlite.connect(":memory:")
        con.execute("CREATE TABLE t (v INTEGER)")
        con.executemany("INSERT INTO t VALUES (?)", [(1,), (1,), (3,)])
        rows = con.execute(
            "SELECT v, CUME_DIST() OVER (ORDER BY v) AS cd FROM t ORDER BY v"
        ).fetchall()
        # v=1 (tie group ends at pos 2 / 3 rows) → cd = 2/3
        # v=3 → cd = 3/3 = 1.0
        v1 = [r[1] for r in rows if r[0] == 1]
        v3 = [r[1] for r in rows if r[0] == 3]
        assert all(abs(cd - 2/3) < 1e-9 for cd in v1)
        assert all(abs(cd - 1.0) < 1e-9 for cd in v3)


# ---------------------------------------------------------------------------
# NTH_VALUE
# ---------------------------------------------------------------------------


class TestNthValueEndToEnd:
    def test_nth_value_first(self):
        """NTH_VALUE(salary, 1) == FIRST_VALUE(salary)."""
        con = _conn()
        rows = con.execute(
            """
            SELECT name, dept, salary,
                   NTH_VALUE(salary, 1) OVER (PARTITION BY dept ORDER BY salary) AS nth
            FROM employees
            ORDER BY dept, salary
            """
        ).fetchall()
        by_name = {r[0]: r[3] for r in rows}
        # First in eng (salary ASC) = Bob = 80000
        assert by_name["Bob"] == 80000
        assert by_name["Eve"] == 80000
        assert by_name["Alice"] == 80000
        # First in sales = Carol = 70000
        assert by_name["Carol"] == 70000
        assert by_name["Dave"] == 70000

    def test_nth_value_second(self):
        """NTH_VALUE(salary, 2): the second row in each partition."""
        con = _conn()
        rows = con.execute(
            """
            SELECT name, dept, salary,
                   NTH_VALUE(salary, 2) OVER (PARTITION BY dept ORDER BY salary) AS nth
            FROM employees
            ORDER BY dept, salary
            """
        ).fetchall()
        by_name = {r[0]: r[3] for r in rows}
        # eng 2nd = Eve = 85000
        assert by_name["Bob"] == 85000
        assert by_name["Eve"] == 85000
        assert by_name["Alice"] == 85000
        # sales 2nd = Dave = 75000
        assert by_name["Carol"] == 75000
        assert by_name["Dave"] == 75000

    def test_nth_value_beyond_partition_returns_null(self):
        """NTH_VALUE(salary, 99) returns NULL (beyond partition size)."""
        con = _conn()
        rows = con.execute(
            """
            SELECT NTH_VALUE(salary, 99) OVER (PARTITION BY dept ORDER BY salary) AS nth
            FROM employees
            """
        ).fetchall()
        assert all(r[0] is None for r in rows)

    def test_nth_value_global(self):
        """NTH_VALUE(salary, 3) over all rows (no partition) = the 3rd smallest salary."""
        con = _conn()
        rows = con.execute(
            """
            SELECT salary,
                   NTH_VALUE(salary, 3) OVER (ORDER BY salary) AS nth
            FROM employees
            ORDER BY salary
            """
        ).fetchall()
        # All salaries ASC: 70000, 75000, 80000, 85000, 90000 → 3rd = 80000
        assert all(r[1] == 80000 for r in rows)


# ---------------------------------------------------------------------------
# Parser / adapter — grammar accepts new function names
# ---------------------------------------------------------------------------


class TestNewFunctionsParseAndAdapt:
    def test_lag_parses(self):
        """Parser accepts LAG(col, n) OVER (...)."""
        con = mini_sqlite.connect(":memory:")
        con.execute("CREATE TABLE t (x INTEGER)")
        con.execute("INSERT INTO t VALUES (1)")
        rows = con.execute(
            "SELECT LAG(x, 1) OVER (ORDER BY x) FROM t"
        ).fetchall()
        assert rows == [(None,)]

    def test_lead_parses(self):
        """Parser accepts LEAD(col) OVER (...)."""
        con = mini_sqlite.connect(":memory:")
        con.execute("CREATE TABLE t (x INTEGER)")
        con.execute("INSERT INTO t VALUES (1)")
        rows = con.execute(
            "SELECT LEAD(x) OVER (ORDER BY x) FROM t"
        ).fetchall()
        assert rows == [(None,)]

    def test_ntile_parses(self):
        """Parser accepts NTILE(n) OVER (ORDER BY ...)."""
        con = mini_sqlite.connect(":memory:")
        con.execute("CREATE TABLE t (x INTEGER)")
        con.execute("INSERT INTO t VALUES (1)")
        rows = con.execute(
            "SELECT NTILE(2) OVER (ORDER BY x) FROM t"
        ).fetchall()
        assert rows == [(1,)]

    def test_percent_rank_parses(self):
        """Parser accepts PERCENT_RANK() OVER (ORDER BY ...)."""
        con = mini_sqlite.connect(":memory:")
        con.execute("CREATE TABLE t (x INTEGER)")
        con.execute("INSERT INTO t VALUES (42)")
        rows = con.execute(
            "SELECT PERCENT_RANK() OVER (ORDER BY x) FROM t"
        ).fetchall()
        assert rows == [(0.0,)]

    def test_cume_dist_parses(self):
        """Parser accepts CUME_DIST() OVER (ORDER BY ...)."""
        con = mini_sqlite.connect(":memory:")
        con.execute("CREATE TABLE t (x INTEGER)")
        con.execute("INSERT INTO t VALUES (42)")
        rows = con.execute(
            "SELECT CUME_DIST() OVER (ORDER BY x) FROM t"
        ).fetchall()
        assert rows == [(1.0,)]

    def test_nth_value_parses(self):
        """Parser accepts NTH_VALUE(col, n) OVER (ORDER BY ...)."""
        con = mini_sqlite.connect(":memory:")
        con.execute("CREATE TABLE t (x INTEGER)")
        con.execute("INSERT INTO t VALUES (7)")
        rows = con.execute(
            "SELECT NTH_VALUE(x, 1) OVER (ORDER BY x) FROM t"
        ).fetchall()
        assert rows == [(7,)]
