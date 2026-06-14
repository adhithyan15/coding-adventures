"""Shared test fixtures — a pre-populated in-memory backend for SELECT tests."""

from __future__ import annotations

import pytest
from sql_backend.in_memory import InMemoryBackend
from sql_backend.schema import ColumnDef

collect_ignore_glob = ["* 2.py"]


@pytest.fixture
def employees() -> InMemoryBackend:
    be = InMemoryBackend()
    be.create_table(
        "employees",
        [
            ColumnDef(name="id", type_name="INTEGER", primary_key=True),
            ColumnDef(name="name", type_name="TEXT"),
            ColumnDef(name="dept", type_name="TEXT"),
            ColumnDef(name="salary", type_name="INTEGER"),
            ColumnDef(name="active", type_name="BOOLEAN"),
        ],
        False,
    )
    rows = [
        {"id": 1, "name": "Alice", "dept": "eng", "salary": 90000, "active": True},
        {"id": 2, "name": "Bob", "dept": "eng", "salary": 80000, "active": True},
        {"id": 3, "name": "Carol", "dept": "sales", "salary": 70000, "active": False},
        {"id": 4, "name": "Dave", "dept": "sales", "salary": 75000, "active": True},
        {"id": 5, "name": "Eve", "dept": "eng", "salary": 85000, "active": True},
    ]
    for r in rows:
        be.insert("employees", r)
    return be


@pytest.fixture
def empty_backend() -> InMemoryBackend:
    be = InMemoryBackend()
    be.create_table(
        "t",
        [
            ColumnDef(name="x", type_name="INTEGER"),
            ColumnDef(name="y", type_name="INTEGER"),
        ],
        False,
    )
    return be
