"""Tests for the Pager — page I/O, LRU cache, rollback journal, recovery."""

from __future__ import annotations

import os
import struct
from pathlib import Path

import pytest

from storage_sqlite.errors import CorruptDatabaseError, JournalError
from storage_sqlite.pager import (
    _JOURNAL_HEADER_FMT,
    _JOURNAL_HEADER_SIZE,
    _JOURNAL_MAGIC,
    _JOURNAL_SENTINEL,
    PAGE_SIZE,
    Pager,
)


def _page(byte: int) -> bytes:
    return bytes([byte]) * PAGE_SIZE


# ------------------------------------------------------------------
# Create / open / close basics.
# ------------------------------------------------------------------


def test_create_then_open(tmp_path: Path) -> None:
    p = tmp_path / "db"
    pager = Pager.create(p)
    assert pager.page_size == PAGE_SIZE
    assert pager.size_pages == 0
    pager.close()

    pager2 = Pager.open(p)
    assert pager2.size_pages == 0
    pager2.close()


def test_create_refuses_to_overwrite(tmp_path: Path) -> None:
    p = tmp_path / "db"
    p.touch()
    with pytest.raises(FileExistsError):
        Pager.create(p)


def test_open_refuses_missing_file(tmp_path: Path) -> None:
    with pytest.raises(FileNotFoundError):
        Pager.open(tmp_path / "missing")


def test_open_rejects_unaligned_file(tmp_path: Path) -> None:
    p = tmp_path / "db"
    p.write_bytes(b"\x00" * (PAGE_SIZE + 5))
    with pytest.raises(CorruptDatabaseError, match="multiple of page size"):
        Pager.open(p)


def test_close_is_idempotent(tmp_path: Path) -> None:
    p = tmp_path / "db"
    pager = Pager.create(p)
    pager.close()
    pager.close()  # no-op the second time


def test_context_manager_closes(tmp_path: Path) -> None:
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        assert pager.size_pages == 0
    # After exit, calls should fail.
    with pytest.raises(RuntimeError, match="closed"):
        pager.read(1)


# ------------------------------------------------------------------
# allocate / read / write.
# ------------------------------------------------------------------


def test_allocate_returns_sequential_page_numbers(tmp_path: Path) -> None:
    with Pager.create(tmp_path / "db") as pager:
        assert pager.allocate() == 1
        assert pager.allocate() == 2
        assert pager.allocate() == 3
        assert pager.size_pages == 3


def test_newly_allocated_page_reads_as_zeros(tmp_path: Path) -> None:
    with Pager.create(tmp_path / "db") as pager:
        n = pager.allocate()
        assert pager.read(n) == b"\x00" * PAGE_SIZE


def test_write_then_read_within_txn(tmp_path: Path) -> None:
    with Pager.create(tmp_path / "db") as pager:
        n = pager.allocate()
        pager.write(n, _page(0xAB))
        assert pager.read(n) == _page(0xAB)


def test_write_rejects_unallocated_page(tmp_path: Path) -> None:
    with Pager.create(tmp_path / "db") as pager, pytest.raises(ValueError, match="out of range"):
        pager.write(1, _page(0))


def test_write_rejects_wrong_size(tmp_path: Path) -> None:
    with Pager.create(tmp_path / "db") as pager:
        pager.allocate()
        with pytest.raises(ValueError, match="exactly"):
            pager.write(1, b"\x00" * 10)


def test_read_rejects_page_zero(tmp_path: Path) -> None:
    with Pager.create(tmp_path / "db") as pager, pytest.raises(ValueError, match=">= 1"):
        pager.read(0)


def test_read_rejects_out_of_range(tmp_path: Path) -> None:
    with Pager.create(tmp_path / "db") as pager, pytest.raises(ValueError, match="out of range"):
        pager.read(5)


# ------------------------------------------------------------------
# Commit / rollback.
# ------------------------------------------------------------------


def test_commit_persists_writes(tmp_path: Path) -> None:
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        n = pager.allocate()
        pager.write(n, _page(0xCD))
        pager.commit()

    with Pager.open(p) as pager:
        assert pager.size_pages == 1
        assert pager.read(1) == _page(0xCD)


def test_commit_is_noop_without_dirty(tmp_path: Path) -> None:
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        pager.commit()  # nothing staged
    assert not (tmp_path / "db-journal").exists()


def test_commit_deletes_journal(tmp_path: Path) -> None:
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        pager.allocate()
        pager.write(1, _page(1))
        pager.commit()
    assert not (tmp_path / "db-journal").exists()


def test_rollback_discards_writes(tmp_path: Path) -> None:
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        n1 = pager.allocate()
        pager.write(n1, _page(0xAA))
        pager.commit()

        # Now modify & rollback.
        pager.write(n1, _page(0xBB))
        pager.allocate()  # page 2
        assert pager.size_pages == 2
        pager.rollback()
        assert pager.size_pages == 1
        assert pager.read(1) == _page(0xAA)


def test_rollback_before_any_commit(tmp_path: Path) -> None:
    with Pager.create(tmp_path / "db") as pager:
        pager.allocate()
        pager.allocate()
        pager.rollback()
        assert pager.size_pages == 0


def test_commit_then_rollback_does_nothing_dangerous(tmp_path: Path) -> None:
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        pager.allocate()
        pager.write(1, _page(0x11))
        pager.commit()
        pager.rollback()
        assert pager.read(1) == _page(0x11)


def test_second_write_to_same_page_does_not_journal_twice(tmp_path: Path) -> None:
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        pager.allocate()
        pager.write(1, _page(0x01))
        pager.commit()

        # Snapshot original, overwrite twice.
        pager.write(1, _page(0x02))
        pager.write(1, _page(0x03))
        pager.commit()

    with Pager.open(p) as pager:
        assert pager.read(1) == _page(0x03)


# ------------------------------------------------------------------
# LRU cache behaviour.
# ------------------------------------------------------------------


def test_lru_evicts_least_recent(tmp_path: Path) -> None:
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        for i in range(1, 6):
            pager.allocate()
            pager.write(i, _page(i))
        pager.commit()

    with Pager.open(p, cache_pages=2) as pager:
        pager.read(1)
        pager.read(2)
        pager.read(3)  # evicts page 1
        # Access page 2 to bump it — page 3 is now LRU.
        pager.read(2)
        pager.read(4)  # evicts page 3
        # Re-reading still works (cache miss falls through to main).
        assert pager.read(3) == _page(3)


def test_cached_read_returns_same_bytes(tmp_path: Path) -> None:
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        pager.allocate()
        pager.write(1, _page(0x77))
        pager.commit()

    with Pager.open(p) as pager:
        a = pager.read(1)
        b = pager.read(1)  # cache hit
        assert a == b == _page(0x77)


# ------------------------------------------------------------------
# Crash recovery.
# ------------------------------------------------------------------


def test_recover_finalised_journal_restores_original(tmp_path: Path) -> None:
    """A finalised journal means we crashed mid-apply — replay restores."""
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        pager.allocate()
        pager.write(1, _page(0xAA))
        pager.commit()

    # Simulate a crashed commit: partially-applied main file + finalised
    # journal. We do this by running a transaction, then hand-crafting the
    # post-crash state.
    original = (tmp_path / "db").read_bytes()

    with Pager.open(p) as pager:
        pager.write(1, _page(0xBB))  # snapshot original to _originals
        pager._write_journal()
        pager._apply_dirty_to_main()
        # Crash before os.remove(journal) — leave journal on disk.

    # Main file now has 0xBB; journal is finalised with 0xAA as the original.
    assert (tmp_path / "db").read_bytes() != original
    assert (tmp_path / "db-journal").exists()

    with Pager.open(p) as pager:
        assert pager.read(1) == _page(0xAA)
    assert not (tmp_path / "db-journal").exists()


def test_recover_non_finalised_journal_discards(tmp_path: Path) -> None:
    """Sentinel record count ⇒ commit aborted pre-apply ⇒ discard."""
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        pager.allocate()
        pager.write(1, _page(0xAA))
        pager.commit()

    # Write a non-finalised journal by hand.
    (tmp_path / "db-journal").write_bytes(
        struct.pack(
            _JOURNAL_HEADER_FMT,
            _JOURNAL_MAGIC,
            PAGE_SIZE,
            _JOURNAL_SENTINEL,
            1,
        )
    )
    with Pager.open(p) as pager:
        assert pager.read(1) == _page(0xAA)
    assert not (tmp_path / "db-journal").exists()


def test_recover_truncated_journal_discards(tmp_path: Path) -> None:
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        pager.allocate()
        pager.write(1, _page(0xAA))
        pager.commit()

    (tmp_path / "db-journal").write_bytes(b"\x00" * 4)  # partial header
    with Pager.open(p) as pager:
        assert pager.read(1) == _page(0xAA)
    assert not (tmp_path / "db-journal").exists()


def test_recover_rejects_bad_journal_magic(tmp_path: Path) -> None:
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        pager.allocate()
        pager.write(1, _page(0xAA))
        pager.commit()

    (tmp_path / "db-journal").write_bytes(
        struct.pack(_JOURNAL_HEADER_FMT, b"BADMAGIC", PAGE_SIZE, 0, 1)
    )
    with pytest.raises(JournalError, match="bad journal magic"):
        Pager.open(p)


def test_recover_rejects_wrong_page_size_journal(tmp_path: Path) -> None:
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        pager.allocate()
        pager.write(1, _page(0xAA))
        pager.commit()

    (tmp_path / "db-journal").write_bytes(
        struct.pack(_JOURNAL_HEADER_FMT, _JOURNAL_MAGIC, 8192, 0, 1)
    )
    with pytest.raises(JournalError, match="page size"):
        Pager.open(p)


def test_recover_rejects_truncated_record_prefix(tmp_path: Path) -> None:
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        pager.allocate()
        pager.write(1, _page(0xAA))
        pager.commit()

    # Finalised header claiming one record but no record bytes follow.
    (tmp_path / "db-journal").write_bytes(
        struct.pack(_JOURNAL_HEADER_FMT, _JOURNAL_MAGIC, PAGE_SIZE, 1, 1)
    )
    with pytest.raises(JournalError, match="prefix truncated"):
        Pager.open(p)


def test_recover_rejects_truncated_record_payload(tmp_path: Path) -> None:
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        pager.allocate()
        pager.write(1, _page(0xAA))
        pager.commit()

    header = struct.pack(_JOURNAL_HEADER_FMT, _JOURNAL_MAGIC, PAGE_SIZE, 1, 1)
    prefix = struct.pack(">I", 1)
    # Only 10 bytes of payload instead of 4096.
    (tmp_path / "db-journal").write_bytes(header + prefix + b"\x00" * 10)
    with pytest.raises(JournalError, match="payload truncated"):
        Pager.open(p)


def test_recover_finalised_truncates_new_pages(tmp_path: Path) -> None:
    """Finalised journal: pages allocated during the crashed txn get chopped."""
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        pager.allocate()
        pager.write(1, _page(0x11))
        pager.commit()

    with Pager.open(p) as pager:
        pager.allocate()  # page 2
        pager.write(2, _page(0x22))
        pager._write_journal()
        pager._apply_dirty_to_main()
        # Crash: leave journal.

    # Main file now has 2 pages; journal's initial_size=1 so replay truncates.
    assert os.path.getsize(p) == 2 * PAGE_SIZE

    with Pager.open(p) as pager:
        assert pager.size_pages == 1
        assert pager.read(1) == _page(0x11)
    assert os.path.getsize(p) == PAGE_SIZE


# ------------------------------------------------------------------
# Journal error paths on open helpers.
# ------------------------------------------------------------------


def test_recover_wraps_os_error(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        pager.allocate()
        pager.write(1, _page(0xAA))
        pager.commit()

    # Create a finalised journal pointing at page 1.
    with Pager.open(p) as pager:
        pager.write(1, _page(0xBB))
        pager._write_journal()
        pager._apply_dirty_to_main()

    # Force a write failure during replay.
    real_open = open

    def bad_open(path, *args, **kwargs):  # type: ignore[no-untyped-def]
        f = real_open(path, *args, **kwargs)
        if str(path).endswith("-journal"):
            real_read = f.read

            def boom(*_a, **_k):  # type: ignore[no-untyped-def]
                raise OSError("simulated")

            # Read header OK; read record-prefix blows up.
            state = {"call": 0}

            def read(*a, **k):  # type: ignore[no-untyped-def]
                state["call"] += 1
                if state["call"] == 1:
                    return real_read(*a, **k)
                boom()
                return b""

            f.read = read  # type: ignore[method-assign]
        return f

    monkeypatch.setattr("builtins.open", bad_open)
    with pytest.raises(JournalError, match="recovery failed"):
        Pager.open(p)


# ------------------------------------------------------------------
# Misc.
# ------------------------------------------------------------------


def test_operations_after_close_fail(tmp_path: Path) -> None:
    pager = Pager.create(tmp_path / "db")
    pager.close()
    with pytest.raises(RuntimeError, match="closed"):
        pager.read(1)
    with pytest.raises(RuntimeError, match="closed"):
        pager.write(1, _page(0))
    with pytest.raises(RuntimeError, match="closed"):
        pager.allocate()
    with pytest.raises(RuntimeError, match="closed"):
        pager.commit()
    with pytest.raises(RuntimeError, match="closed"):
        pager.rollback()


def test_short_read_raises_corrupt(tmp_path: Path) -> None:
    """If file shrinks out from under us, a read should refuse to lie."""
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        pager.allocate()
        pager.write(1, _page(0x42))
        pager.commit()

    pager = Pager.open(p)
    # Sabotage: truncate the underlying file mid-life.
    with open(p, "r+b") as f:
        f.truncate(10)
    # Reset initial_size so _read_from_main actually hits disk.
    pager._initial_size_pages = 1
    with pytest.raises(CorruptDatabaseError, match="short read"):
        pager.read(1)
    pager.close()


def test_journal_header_size_constant() -> None:
    assert struct.calcsize(_JOURNAL_HEADER_FMT) == _JOURNAL_HEADER_SIZE


def test_drop_journal_tolerates_missing_file(tmp_path: Path) -> None:
    """``_drop_journal`` should swallow FileNotFoundError."""
    with Pager.create(tmp_path / "db") as pager:
        pager._drop_journal()  # no journal exists


def test_write_on_pre_existing_page_journals_original(tmp_path: Path) -> None:
    """First write to a pre-txn page must snapshot to originals."""
    p = tmp_path / "db"
    with Pager.create(p) as pager:
        pager.allocate()
        pager.write(1, _page(0xAA))
        pager.commit()

    with Pager.open(p) as pager:
        pager.write(1, _page(0xBB))
        assert 1 in pager._originals
        assert pager._originals[1] == _page(0xAA)


def test_newly_allocated_page_not_in_originals(tmp_path: Path) -> None:
    with Pager.create(tmp_path / "db") as pager:
        pager.allocate()
        pager.write(1, _page(0xCC))
        assert 1 not in pager._originals
