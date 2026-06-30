"""
Shared conformance test runner for the mini-sqlite-conformance fixture suite.
=============================================================================

Reads ``code/specs/mini-sqlite-conformance/manifest.json``, loads every listed
fixture, and executes each step against a fresh in-memory mini-sqlite
connection.  Failures are reported per-step so the exact assertion that broke
is visible.

Running
-------

From the ``code/packages/python/mini-sqlite`` directory::

    pytest tests/test_conformance.py -v

All fixtures in ``manifest.json`` are discovered automatically — adding a new
``fixtures/NN-*.json`` file and listing it in the manifest is enough to have it
exercised by this runner.

Fixture format (brief)
----------------------

Each fixture JSON file has the shape::

    {
      "id": "17-null-aggregate-semantics",
      "description": "...",
      "level": 1,
      "steps": [
        {"op": "execute", "sql": "CREATE TABLE ..."},
        {"op": "query", "sql": "SELECT ...",
         "expected_columns": [...], "expected_rows": [[...], ...]},
        {"op": "expect_error", "sql": "BAD SQL", "error_type": "ProgrammingError"},
        ...
      ]
    }

Supported ops: execute, query, executemany, expect_error, commit, rollback,
fetchone_test, fetchmany_test, fetchall_test, fetchall_empty_test.

NULL handling
-------------

JSON ``null`` values in ``expected_rows`` are compared against Python
``None``, which is the language-native NULL representation.  Column name
comparison is case-insensitive.
"""

from __future__ import annotations

import json
import pathlib
from typing import Any

import pytest

import mini_sqlite
from mini_sqlite.errors import (
    NotSupportedError,
    OperationalError,
    ProgrammingError,
)

# ---------------------------------------------------------------------------
# Resolve paths relative to this file so the test works regardless of cwd.
# ---------------------------------------------------------------------------

_HERE = pathlib.Path(__file__).parent
_REPO_ROOT = _HERE.parent.parent.parent.parent.parent  # …/coding-adventures
_SPEC_DIR = _REPO_ROOT / "code" / "specs" / "mini-sqlite-conformance"
_MANIFEST = _SPEC_DIR / "manifest.json"

# ---------------------------------------------------------------------------
# Error type mapping
# ---------------------------------------------------------------------------

_ERROR_MAP: dict[str, type[Exception]] = {
    "ProgrammingError": ProgrammingError,
    "OperationalError": OperationalError,
    "NotSupportedError": NotSupportedError,
}

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _normalise_row(row: tuple[Any, ...]) -> list[Any]:
    """Coerce a DB row tuple to a list matching JSON expected_rows format.

    - Python booleans that slipped through are converted to int (True→1).
    - Python floats that equal an integer value stay as float (fixtures
      use 4.0 not 4 for ROUND results).
    - None stays None.
    """
    out = []
    for v in row:
        if isinstance(v, bool):
            out.append(int(v))
        else:
            out.append(v)
    return out


def _run_fixture(fixture_path: pathlib.Path) -> None:
    """Execute all steps of a single fixture against a fresh connection.

    Fixtures that use ``connect_steps`` (top-level key) instead of ``steps``
    exercise the connection-creation layer rather than statement execution.
    Each ``connect_expect_error`` step in ``connect_steps`` calls
    :func:`mini_sqlite.connect` with a given database path and asserts that
    the expected exception is raised.  These fixtures create no persistent
    connection — each step is fully self-contained.

    Fixtures that use ``steps`` (the common case) open a single in-memory
    connection and run all steps against it in sequence.
    """
    data = json.loads(fixture_path.read_text(encoding="utf-8"))
    fixture_id = data["id"]

    # ── connect_steps fixtures ────────────────────────────────────────────────
    # These fixtures test connection-creation behaviour (e.g. rejecting file
    # paths at Level 0).  They do NOT use a shared connection object.
    if "connect_steps" in data:
        connect_steps = data["connect_steps"]
        for i, step in enumerate(connect_steps):
            op = step["op"]
            step_label = f"fixture={fixture_id!r} step={i} op={op!r}"
            if op == "connect_expect_error":
                db = step.get("database", ":memory:")
                error_type_name = step["error_type"]
                exc_class = _ERROR_MAP.get(error_type_name)
                assert exc_class is not None, (
                    f"{step_label}: unknown error_type {error_type_name!r}"
                )
                with pytest.raises(exc_class):
                    mini_sqlite.connect(db)
            else:
                pytest.fail(f"{step_label}: unknown op in connect_steps {op!r}")
        return  # connect_steps fixtures are fully handled above

    # ── regular steps fixtures ────────────────────────────────────────────────
    steps = data["steps"]

    conn = mini_sqlite.connect(":memory:")

    for i, step in enumerate(steps):
        op = step["op"]
        sql = step.get("sql", "")
        params = step.get("params", ())
        # JSON arrays → tuples for the DB-API layer
        if isinstance(params, list):
            params = tuple(params)

        step_label = f"fixture={fixture_id!r} step={i} op={op!r}"

        if op == "execute":
            conn.execute(sql, params)

        elif op == "executemany":
            param_seq = step.get("param_seq", [])
            conn.executemany(sql, [tuple(r) for r in param_seq])

        elif op == "query":
            expected_columns: list[str] = [c.lower() for c in step["expected_columns"]]
            expected_rows: list[list[Any]] = step["expected_rows"]

            cur = conn.execute(sql, params)
            actual_cols = [desc[0].lower() for desc in cur.description]
            actual_rows = [_normalise_row(r) for r in cur.fetchall()]

            assert actual_cols == expected_columns, (
                f"{step_label}: column mismatch\n"
                f"  expected: {expected_columns}\n"
                f"  actual:   {actual_cols}"
            )
            assert actual_rows == expected_rows, (
                f"{step_label}: row mismatch\n"
                f"  expected: {expected_rows}\n"
                f"  actual:   {actual_rows}"
            )

        elif op == "expect_error":
            error_type_name: str = step["error_type"]
            exc_class = _ERROR_MAP.get(error_type_name)
            assert exc_class is not None, (
                f"{step_label}: unknown error_type {error_type_name!r}"
            )
            with pytest.raises(exc_class):
                conn.execute(sql, params)

        elif op == "commit":
            conn.commit()

        elif op == "rollback":
            conn.rollback()

        elif op == "fetchone_test":
            expected_first = step.get("expected_first")
            expected_second = step.get("expected_second")
            cur = conn.execute(sql, params)
            first = cur.fetchone()
            second = cur.fetchone()
            if expected_first is not None:
                assert _normalise_row(first) == expected_first, (
                    f"{step_label}: fetchone first row mismatch\n"
                    f"  expected: {expected_first}\n"
                    f"  actual:   {_normalise_row(first)}"
                )
            if expected_second is not None:
                assert (
                    _normalise_row(second) if second is not None else None
                ) == expected_second, (
                    f"{step_label}: fetchone second row mismatch\n"
                    f"  expected: {expected_second}\n"
                    f"  actual:   {second}"
                )

        elif op == "fetchmany_test":
            size = step.get("size", 1)
            expected_first_batch = step.get("expected_first_batch")
            expected_second_batch = step.get("expected_second_batch")
            cur = conn.execute(sql, params)
            batch1 = [_normalise_row(r) for r in cur.fetchmany(size)]
            batch2 = [_normalise_row(r) for r in cur.fetchmany(size)]
            if expected_first_batch is not None:
                assert batch1 == expected_first_batch, (
                    f"{step_label}: fetchmany batch 1 mismatch"
                )
            if expected_second_batch is not None:
                assert batch2 == expected_second_batch, (
                    f"{step_label}: fetchmany batch 2 mismatch"
                )

        elif op in ("fetchall_test", "fetchall_empty_test"):
            expected_rows = step.get("expected_rows", [])
            cur = conn.execute(sql, params)
            actual = [_normalise_row(r) for r in cur.fetchall()]
            assert actual == expected_rows, (
                f"{step_label}: fetchall row mismatch\n"
                f"  expected: {expected_rows}\n"
                f"  actual:   {actual}"
            )

        elif op == "connect_expect_error":
            db = step.get("database", ":memory:")
            error_type_name = step["error_type"]
            exc_class = _ERROR_MAP.get(error_type_name)
            assert exc_class is not None, (
                f"{step_label}: unknown error_type {error_type_name!r}"
            )
            with pytest.raises(exc_class):
                mini_sqlite.connect(db)

        else:
            pytest.fail(f"{step_label}: unknown op {op!r}")

    conn.close()


# ---------------------------------------------------------------------------
# Test parametrisation — one test per fixture
# ---------------------------------------------------------------------------


def _load_fixtures() -> list[tuple[str, pathlib.Path]]:
    """Return list of (fixture_id, path) pairs from manifest.json."""
    if not _MANIFEST.exists():
        return []
    manifest = json.loads(_MANIFEST.read_text(encoding="utf-8"))
    result = []
    for rel in manifest["fixtures"]:
        p = _SPEC_DIR / rel
        if p.exists():
            fixture_id = json.loads(p.read_text(encoding="utf-8"))["id"]
            result.append((fixture_id, p))
    return result


_FIXTURES = _load_fixtures()


@pytest.mark.parametrize("fixture_id,fixture_path", _FIXTURES, ids=[f[0] for f in _FIXTURES])
def test_conformance_fixture(fixture_id: str, fixture_path: pathlib.Path) -> None:
    """Run a single conformance fixture end-to-end.

    Each fixture is an isolated test: it gets a fresh in-memory connection
    and runs all steps.  A failure in step N is reported with the fixture
    ID and step index so the root cause is immediately visible.
    """
    _run_fixture(fixture_path)
