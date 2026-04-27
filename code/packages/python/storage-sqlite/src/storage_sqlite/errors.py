"""
Storage-level exceptions.

These live at a layer below the SQL pipeline — they describe things that
went wrong with the file, not with the query. The SQL facade translates
them into PEP 249 classes at a higher level.
"""

from __future__ import annotations


class StorageError(Exception):
    """Base class for everything in this package."""


class CorruptDatabaseError(StorageError):
    """The file on disk does not look like a valid SQLite database.

    Examples: wrong magic string, page size not a supported power of two,
    file shorter than a single page, journal out of sync with the main file.
    """


class JournalError(StorageError):
    """Something went wrong with the rollback journal.

    Raised when a recovery replay encounters an inconsistent journal (e.g.
    truncated, bad length prefix). Distinct from :class:`CorruptDatabaseError`
    because a corrupt journal is often recoverable — we can just throw the
    journal away and trust the main file.
    """
