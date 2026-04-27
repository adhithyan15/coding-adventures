"""
The pager: the bottom of the storage stack.

The pager is the only thing that touches the database file. Everything
above — B-trees, records, ``sqlite_schema`` — reads and writes pages
through this object and never seeks raw bytes on the file.

What the pager guarantees
-------------------------

**Page I/O.** Fixed-size 4096-byte pages, 1-based numbering (page 1 at
byte offset 0, page 2 at byte offset 4096, …). Reads return exactly
4096 bytes; writes take exactly 4096 bytes. Page 0 is illegal.

**Caching.** A small LRU cache (default 32 pages) holds recently-read
clean pages. Writes go first to an in-memory dirty-page table; they
don't touch the file until :meth:`commit`.

**Atomic commit via a rollback journal.** A transaction's writes are
staged in memory. :meth:`commit` does this sequence:

    1. Build a journal of *original* contents for every page being
       modified that existed before this transaction.
    2. Write the journal header with a sentinel record-count and fsync
       the journal.
    3. Finalise the journal header with the real record-count, fsync
       the journal.
    4. Apply every dirty page to the main file, fsync the main file.
    5. Delete the journal.

A crash anywhere in steps 1–3 leaves a non-finalised journal that
recovery discards (main file untouched). A crash in step 4 leaves a
finalised journal that recovery replays, rolling the main file back to
its pre-transaction state (no partial commit). A crash in step 5 also
replays — the user hadn't received a "commit succeeded" signal yet so
the rollback is legitimate. This is the standard SQLite undo-log
protocol.

:meth:`rollback` just drops the in-memory dirty-page table. Nothing has
been written to disk mid-transaction, so there is no journal file to
clean up on the mid-transaction path.

:meth:`open` performs crash recovery before returning: if a journal
file is present, it is replayed (if finalised) or deleted (if not) so
the main file is consistent before any caller sees it.

**v1 limitations** (deferred per ``code/specs/storage-sqlite.md``):

- Page size is pinned at 4096. Other sizes are valid SQLite but not yet
  supported here.
- Large transactions hold every dirty page in memory until commit. Real
  SQLite spills to disk; we don't (yet).
- Single-process, single-writer. No POSIX advisory locking.
- Journal format is our own simple layout, not the real SQLite rollback
  journal byte layout. That's fine because the journal file is transient
  and only relevant to *our* crash recovery — the *main* file is what
  must be byte-compatible with SQLite, and this module doesn't impose a
  format on the main file at all.
"""

from __future__ import annotations

import contextlib
import os
import struct
from collections import OrderedDict
from pathlib import Path
from types import TracebackType

from storage_sqlite.errors import CorruptDatabaseError, JournalError

PAGE_SIZE: int = 4096
"""Fixed page size for v1. SQLite supports 512..65536 in powers of two;
we'll add the rest in v2."""

_DEFAULT_CACHE_PAGES: int = 32

# Our rollback journal format. See module docstring for the protocol.
#
#   [20-byte header]
#     0..8   magic      b"RJRNL\\0\\1\\0"   (8 bytes, arbitrary — not SQLite's)
#     8..12  page_size  u32 BE
#     12..16 record_cnt u32 BE  (sentinel 0xFFFFFFFF while mid-flight;
#                                 real count after finalisation)
#     16..20 initial_size_pages u32 BE  (file size at txn start — used
#                                        to truncate the main file back
#                                        on replay)
#   [each record]
#     0..4   page_no    u32 BE
#     4..4 + PAGE_SIZE  original page contents
_JOURNAL_MAGIC: bytes = b"RJRNL\x00\x01\x00"
_JOURNAL_HEADER_FMT: str = ">8sIII"
_JOURNAL_HEADER_SIZE: int = 20
_JOURNAL_SENTINEL: int = 0xFFFFFFFF
_RECORD_PREFIX_FMT: str = ">I"
_RECORD_PREFIX_SIZE: int = 4


class Pager:
    """Page-at-a-time I/O with LRU cache and rollback journal.

    Lifecycle: :meth:`create` for a brand-new file, :meth:`open` for an
    existing one. Both return an instance usable as a context manager;
    :meth:`close` is idempotent.
    """

    __slots__ = (
        "_cache",
        "_cache_pages",
        "_closed",
        "_dirty",
        "_f",
        "_initial_size_pages",
        "_journal_path",
        "_journaled",
        "_originals",
        "_path",
        "_size_pages",
    )

    def __init__(
        self,
        path: str | os.PathLike[str],
        *,
        cache_pages: int = _DEFAULT_CACHE_PAGES,
    ) -> None:
        self._path: str = str(path)
        self._journal_path: str = self._path + "-journal"
        self._cache_pages: int = cache_pages
        # Opened lazily in ``create`` / ``open``.
        self._f = None  # type: ignore[assignment]
        self._size_pages: int = 0
        self._initial_size_pages: int = 0
        self._cache: OrderedDict[int, bytes] = OrderedDict()
        self._dirty: dict[int, bytes] = {}
        self._journaled: set[int] = set()
        self._originals: dict[int, bytes] = {}
        self._closed: bool = False

    # ------------------------------------------------------------------
    # Public surface properties.
    # ------------------------------------------------------------------

    @property
    def page_size(self) -> int:
        return PAGE_SIZE

    @property
    def size_pages(self) -> int:
        """Current logical size of the database in pages, including any
        pages allocated (but not yet committed) in the current transaction.
        """
        return self._size_pages

    # ------------------------------------------------------------------
    # Construction.
    # ------------------------------------------------------------------

    @classmethod
    def create(
        cls,
        path: str | os.PathLike[str],
        *,
        cache_pages: int = _DEFAULT_CACHE_PAGES,
    ) -> Pager:
        """Create a fresh zero-page file and return a pager over it.

        Raises :class:`FileExistsError` if the path already exists — the
        caller should explicitly delete first if they want to overwrite.
        """
        p = Path(path)
        if p.exists():
            raise FileExistsError(f"refusing to overwrite existing file: {p}")
        p.touch()
        pager = cls(p, cache_pages=cache_pages)
        pager._open_file()
        return pager

    @classmethod
    def open(
        cls,
        path: str | os.PathLike[str],
        *,
        cache_pages: int = _DEFAULT_CACHE_PAGES,
    ) -> Pager:
        """Open an existing file. Replays a hot journal if present."""
        p = Path(path)
        if not p.exists():
            raise FileNotFoundError(str(p))
        pager = cls(p, cache_pages=cache_pages)
        pager._open_file()
        return pager

    def _open_file(self) -> None:
        self._f = open(self._path, "r+b")  # noqa: SIM115 — lifetime is the pager
        file_size = os.path.getsize(self._path)
        if file_size % PAGE_SIZE != 0:
            self._f.close()
            raise CorruptDatabaseError(
                f"file size {file_size} is not a multiple of page size {PAGE_SIZE}"
            )
        self._size_pages = file_size // PAGE_SIZE
        self._initial_size_pages = self._size_pages

        # Recover before returning: a hot journal must be resolved before
        # any caller reads a page, or they might see a partially-committed
        # main file.
        if os.path.exists(self._journal_path):
            self._recover()

    # ------------------------------------------------------------------
    # Transaction-level operations.
    # ------------------------------------------------------------------

    def read(self, page_no: int) -> bytes:
        """Return the 4096-byte contents of ``page_no``.

        Looks in the dirty-page table first (so writes are visible within
        the same transaction), then the LRU cache, then falls through to
        the main file.
        """
        self._assert_open()
        self._check_page_no(page_no)

        if page_no in self._dirty:
            return self._dirty[page_no]

        if page_no in self._cache:
            self._cache.move_to_end(page_no)
            return self._cache[page_no]

        data = self._read_from_main(page_no)
        self._cache_put(page_no, data)
        return data

    def write(self, page_no: int, data: bytes) -> None:
        """Stage a write to ``page_no``.

        The write lands on disk only at :meth:`commit`. If this is the
        first write to ``page_no`` in the current transaction *and*
        ``page_no`` existed before the transaction, the original contents
        are snapshotted so the journal can restore them on a crashed
        commit.
        """
        self._assert_open()
        self._check_page_no(page_no)
        if len(data) != PAGE_SIZE:
            raise ValueError(f"data must be exactly {PAGE_SIZE} bytes, got {len(data)}")

        # Snapshot the original once, on the first write in this txn,
        # *if* the page existed pre-txn. Pages newly allocated this txn
        # have no "original" — on rollback they simply cease to exist.
        if page_no <= self._initial_size_pages and page_no not in self._journaled:
            self._originals[page_no] = self._read_from_main(page_no)
            self._journaled.add(page_no)

        # ``data`` is stored by reference — callers must not mutate a
        # bytearray they handed in. We copy to ``bytes`` defensively.
        self._dirty[page_no] = bytes(data)

    def allocate(self) -> int:
        """Reserve the next page number.

        The freshly-allocated page is initialised to zeros in the
        dirty-page table, so subsequent reads see a defined state. The
        page number is one past the current size.
        """
        self._assert_open()
        self._size_pages += 1
        new_no = self._size_pages
        self._dirty[new_no] = b"\x00" * PAGE_SIZE
        return new_no

    def commit(self) -> None:
        """Atomically apply every dirty page to the main file.

        No-op if nothing is dirty — no journal file is created in that
        case. Callers can call ``commit`` freely at transaction
        boundaries regardless of whether anything was actually written.
        """
        self._assert_open()
        if not self._dirty:
            return

        try:
            self._write_journal()
            self._apply_dirty_to_main()
            # The journal becoming unlinked is the "this transaction
            # committed" signal. A crash before this point → recovery
            # replays. A crash after → nothing to replay.
            os.remove(self._journal_path)
        except Exception:
            # Leave state as-is; the caller must rollback or close. We
            # don't attempt a partial cleanup because the journal /
            # main-file state could be anywhere between "untouched" and
            # "fully applied" and guessing wrong would corrupt the file.
            raise

        # Success: the txn boundary moves forward.
        #
        # Promote every committed page into the LRU cache so that reads
        # *within the same Pager session* immediately after commit see the
        # newly-written data without going back to disk.  Without this
        # promotion the cache can hold stale pre-txn values for pages that
        # were written during the transaction (the cache is populated on
        # first *read*, not on write, so a page that was only written in
        # this txn would not be in the cache yet, but a page that was read
        # before being modified *would* be in the cache with its old value).
        for page_no, data in self._dirty.items():
            self._cache_put(page_no, data)
        self._initial_size_pages = self._size_pages
        self._dirty.clear()
        self._originals.clear()
        self._journaled.clear()

    def rollback(self) -> None:
        """Discard every pending write. No disk I/O."""
        self._assert_open()
        # Evict any dirty pages from the cache before clearing dirty, so
        # subsequent reads see the *committed* state (from the last commit
        # or the initial file contents) rather than the discarded dirty
        # values.  For pages that existed before this transaction the cache
        # will be repopulated from the main file on the next read; for
        # freshly-allocated pages the cache entry is simply gone (which is
        # correct — those page numbers cease to exist after rollback).
        for page_no in self._dirty:
            self._cache.pop(page_no, None)
        self._dirty.clear()
        self._originals.clear()
        self._journaled.clear()
        # Drop any pages allocated this txn by shrinking back to the
        # pre-txn size. The file itself wasn't extended yet, so there's
        # nothing to truncate — the extension only happens in ``commit``.
        self._size_pages = self._initial_size_pages

    # ------------------------------------------------------------------
    # Lifecycle.
    # ------------------------------------------------------------------

    def close(self) -> None:
        """Release the file handle.

        Pending dirty writes are discarded silently — for durability the
        caller must ``commit`` first. Idempotent.
        """
        if self._closed:
            return
        self._closed = True
        self._dirty.clear()
        self._originals.clear()
        self._journaled.clear()
        self._cache.clear()
        if self._f is not None:
            self._f.close()
            self._f = None  # type: ignore[assignment]

    def __enter__(self) -> Pager:
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> None:
        self.close()

    # ------------------------------------------------------------------
    # Internals.
    # ------------------------------------------------------------------

    def _check_page_no(self, page_no: int) -> None:
        if page_no < 1:
            raise ValueError(f"page number must be >= 1, got {page_no}")
        if page_no > self._size_pages and page_no not in self._dirty:
            raise ValueError(
                f"page {page_no} out of range (size={self._size_pages})"
            )

    def _read_from_main(self, page_no: int) -> bytes:
        # Pages allocated this txn but not yet committed are served from
        # ``_dirty`` by the caller — we should never be asked to read them
        # from main. Defensively return zeros if asked.
        if page_no > self._initial_size_pages:
            return b"\x00" * PAGE_SIZE
        assert self._f is not None
        self._f.seek((page_no - 1) * PAGE_SIZE)
        data = self._f.read(PAGE_SIZE)
        if len(data) != PAGE_SIZE:
            raise CorruptDatabaseError(
                f"short read: page {page_no} returned {len(data)} bytes"
            )
        return data

    def _cache_put(self, page_no: int, data: bytes) -> None:
        self._cache[page_no] = data
        self._cache.move_to_end(page_no)
        while len(self._cache) > self._cache_pages:
            self._cache.popitem(last=False)

    def _assert_open(self) -> None:
        if self._closed:
            raise RuntimeError("pager is closed")

    # ---- Journal write path (commit side). ----

    def _write_journal(self) -> None:
        """Write every ``_originals`` entry to the journal, then finalise.

        Two fsyncs: once after the records are written (so we can finalise
        on top of durable data), once after the finalised header (so a
        crash after this returns is recoverable).
        """
        # If nothing pre-existing changed (e.g. txn only allocated new
        # pages), the journal has zero records but we still write a
        # finalised header — recovery for such a journal is a no-op but
        # the presence of *any* journal file on disk during the apply
        # phase is what makes the commit atomic.
        with open(self._journal_path, "wb") as j:
            j.write(
                struct.pack(
                    _JOURNAL_HEADER_FMT,
                    _JOURNAL_MAGIC,
                    PAGE_SIZE,
                    _JOURNAL_SENTINEL,
                    self._initial_size_pages,
                )
            )
            for page_no in sorted(self._originals):
                j.write(struct.pack(_RECORD_PREFIX_FMT, page_no))
                j.write(self._originals[page_no])
            j.flush()
            os.fsync(j.fileno())
            # Finalise: rewrite header with the real record count.
            j.seek(0)
            j.write(
                struct.pack(
                    _JOURNAL_HEADER_FMT,
                    _JOURNAL_MAGIC,
                    PAGE_SIZE,
                    len(self._originals),
                    self._initial_size_pages,
                )
            )
            j.flush()
            os.fsync(j.fileno())

    def _apply_dirty_to_main(self) -> None:
        """Write every dirty page to the main file and fsync."""
        assert self._f is not None
        # Sort for deterministic on-disk write order — makes tests and
        # golden-file diffs behave predictably.
        for page_no in sorted(self._dirty):
            self._f.seek((page_no - 1) * PAGE_SIZE)
            self._f.write(self._dirty[page_no])
        # Shrink if this txn allocated pages that rollback later undid —
        # _size_pages at this point is the *committed* size.
        self._f.truncate(self._size_pages * PAGE_SIZE)
        self._f.flush()
        os.fsync(self._f.fileno())

    # ---- Crash recovery (open path). ----

    def _recover(self) -> None:
        """Replay or discard a hot journal left by a crashed writer."""
        try:
            with open(self._journal_path, "rb") as j:
                header = j.read(_JOURNAL_HEADER_SIZE)
                if len(header) < _JOURNAL_HEADER_SIZE:
                    # Partial header → commit never began → discard.
                    self._drop_journal()
                    return
                magic, page_size, record_count, initial_size = struct.unpack(
                    _JOURNAL_HEADER_FMT, header
                )
                if magic != _JOURNAL_MAGIC:
                    raise JournalError(f"bad journal magic: {magic!r}")
                if page_size != PAGE_SIZE:
                    raise JournalError(
                        f"journal page size {page_size} != main {PAGE_SIZE}"
                    )
                if record_count == _JOURNAL_SENTINEL:
                    # Not finalised → commit aborted mid-flight → main is
                    # still pre-txn → no replay needed.
                    self._drop_journal()
                    return

                # Finalised: replay every record to undo the partial
                # application on main.
                assert self._f is not None
                for _ in range(record_count):
                    prefix = j.read(_RECORD_PREFIX_SIZE)
                    if len(prefix) < _RECORD_PREFIX_SIZE:
                        raise JournalError("journal record prefix truncated")
                    (page_no,) = struct.unpack(_RECORD_PREFIX_FMT, prefix)
                    payload = j.read(PAGE_SIZE)
                    if len(payload) < PAGE_SIZE:
                        raise JournalError("journal record payload truncated")
                    self._f.seek((page_no - 1) * PAGE_SIZE)
                    self._f.write(payload)
                # Truncate main back to the pre-txn size so any pages
                # allocated during the failed txn disappear.
                self._f.truncate(initial_size * PAGE_SIZE)
                self._f.flush()
                os.fsync(self._f.fileno())
                self._size_pages = initial_size
                self._initial_size_pages = initial_size
        except JournalError:
            raise
        except OSError as e:
            raise JournalError(f"journal recovery failed: {e}") from e

        self._drop_journal()

    def _drop_journal(self) -> None:
        with contextlib.suppress(FileNotFoundError):
            os.remove(self._journal_path)
