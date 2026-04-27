"""
Run the conformance suite against InMemoryBackend.

This is a black-box test: the conformance helpers know nothing about the
backend's internals — they just exercise the public interface and assert
the documented contract. If InMemoryBackend passes, we know the helpers
are a correct statement of the contract, and any future backend that passes
them will be a drop-in replacement.
"""

from __future__ import annotations

from sql_backend.conformance import (
    make_in_memory_users,
    run_ddl,
    run_read_write,
    run_required,
    run_transaction,
)


def test_required_tier() -> None:
    run_required(make_in_memory_users)


def test_read_write_tier() -> None:
    run_read_write(make_in_memory_users)


def test_ddl_tier() -> None:
    run_ddl(make_in_memory_users)


def test_transaction_tier() -> None:
    run_transaction(make_in_memory_users)
