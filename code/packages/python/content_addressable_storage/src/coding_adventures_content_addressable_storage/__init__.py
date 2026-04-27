"""cas — Generic Content-Addressable Storage (CAS)

What Is Content-Addressable Storage?
=====================================
Content-addressable storage (CAS) maps *the hash of content* to the content
itself.  The hash is simultaneously the address and an integrity check: if the
bytes returned by the store don't hash to the key you requested, the data is
corrupt.  No separate checksum file or trust anchor is needed.

Mental model
------------
Imagine a library where every book's call number *is* a fingerprint of the
book's text.  You can't file a different book under that number — the number
would immediately be wrong.  And if someone swaps pages, the fingerprint
changes and the librarian knows before you even open the cover.

    Traditional storage:  name ──► content   (name can lie; content can change)
    Content-addressed:    hash ──► content   (hash is derived from content, cannot lie)

How Git Uses CAS
----------------
Git's entire history is built on this principle.  Every blob (file snapshot),
tree (directory listing), commit, and tag is stored by the SHA-1 hash of its
serialized bytes.  Two identical files share one object.  Renaming a file
creates zero new storage.  History is an immutable DAG of hashes pointing to
hashes.

This package provides the CAS layer only — hashing and storage.  The Git object
format (``"blob N\\0content"``), compression, and pack files are handled by
layers above and below.

Architecture
------------
::

    ┌──────────────────────────────────────────────────────────┐
    │  ContentAddressableStore(blob_store: BlobStore)           │
    │  · put(data)          → SHA-1 key, delegate to store     │
    │  · get(key)           → fetch from store, verify hash    │
    │  · find_by_prefix(hex)→ prefix search via store          │
    └─────────────────┬────────────────────────────────────────┘
                      │ BlobStore (abstract base class)
             ┌────────┴──────────────────────┐
             │                               │
      LocalDiskStore               (S3, mem, custom, …)
      root/XX/XXXXXX…

Quick start
-----------
::

    from coding_adventures_content_addressable_storage import ContentAddressableStore, LocalDiskStore
    import tempfile, pathlib

    with tempfile.TemporaryDirectory() as tmp:
        store = LocalDiskStore(pathlib.Path(tmp))
        cas = ContentAddressableStore(store)

        key = cas.put(b"hello, world")
        data = cas.get(key)
        assert data == b"hello, world"
"""

# ─── Standard Library Imports ─────────────────────────────────────────────────

import abc
import contextlib
import os
import pathlib
import time
from typing import Final

from coding_adventures_sha1 import sha1 as _sha1_impl

# ─── Public Re-exports ────────────────────────────────────────────────────────

__all__: Final[list[str]] = [
    # Utilities
    "key_to_hex",
    "hex_to_key",
    # Abstract base class
    "BlobStore",
    # Exceptions
    "CasError",
    "CasStoreError",
    "CasNotFoundError",
    "CasCorruptedError",
    "CasAmbiguousPrefixError",
    "CasPrefixNotFoundError",
    "CasInvalidPrefixError",
    # Core classes
    "ContentAddressableStore",
    "LocalDiskStore",
]

# ─── Type Alias ───────────────────────────────────────────────────────────────

# A SHA-1 key is exactly 20 bytes.  We represent it as `bytes` throughout
# Python (unlike Rust's [u8; 20] fixed-size array).  Using `bytes` is idiomatic
# Python: immutable, hashable, 20 bytes produced by coding_adventures_sha1.
Key = bytes  # always exactly 20 bytes

# ─── Hex Utilities ────────────────────────────────────────────────────────────
#
# Keys are 20-byte strings, but humans interact with them as 40-char lowercase
# hex strings (e.g., "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5").
#
# key_to_hex  — converts 20-byte Key → 40-char hex string
# hex_to_key  — parses a 40-char hex string → 20-byte Key, raises on bad input


def key_to_hex(key: Key) -> str:
    """Convert a 20-byte SHA-1 key to a 40-character lowercase hex string.

    >>> key_to_hex(bytes.fromhex("a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5"))
    'a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5'
    """
    if len(key) != 20:
        raise ValueError(f"key must be exactly 20 bytes, got {len(key)}")
    return key.hex()


def hex_to_key(hex_str: str) -> Key:
    """Parse a 40-character lowercase hex string into a 20-byte key.

    Raises ``ValueError`` if the string is not exactly 40 hex characters.

    >>> key = hex_to_key("a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5")
    >>> key_to_hex(key)
    'a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5'
    """
    if len(hex_str) != 40:
        raise ValueError(f"expected 40 hex chars, got {len(hex_str)}")
    try:
        return bytes.fromhex(hex_str)
    except ValueError as exc:
        raise ValueError(f"invalid hex string: {hex_str!r}") from exc


def _decode_hex_prefix(hex_prefix: str) -> bytes:
    """Decode an arbitrary-length hex string (1–40 chars) to a byte prefix.

    Odd-length strings are right-padded with '0' before decoding, because a
    nibble prefix like ``"a3f"`` means "starts with 0xa3, 0xf0" — the trailing
    nibble is the high nibble of the next byte.

    Raises ``ValueError`` if the string is empty or contains non-hex chars.

    Why pad odd lengths with '0' on the *right*?
    Consider the prefix "a3f":
      - It represents the nibbles a, 3, f.
      - In memory, the second byte of the key must start with nibble 'f'.
      - So the byte prefix is [0xa3, 0xf0] (the low nibble is don't-care).
      - Padding right with '0' gives "a3f0" → [0xa3, 0xf0]. Correct.
      - Padding left would give "0a3f" → [0x0a, 0x3f]. Wrong.
    """
    if not hex_prefix:
        raise ValueError("prefix cannot be empty")
    # Validate all characters are valid hex.
    try:
        # int(hex_prefix, 16) alone won't catch leading '0x' or spaces; we
        # validate character-by-character instead.
        for ch in hex_prefix:
            if ch not in "0123456789abcdefABCDEF":
                raise ValueError(f"invalid hex character: {ch!r}")
    except ValueError:
        raise
    # Pad to even length on the right (see docstring for why).
    padded = hex_prefix if len(hex_prefix) % 2 == 0 else hex_prefix + "0"
    return bytes.fromhex(padded)


# ─── SHA-1 Helper ─────────────────────────────────────────────────────────────
#
# We use the repo's own SHA-1 implementation (coding-adventures-sha1) rather
# than Python's stdlib hashlib.  This keeps the entire stack self-contained and
# ensures every language port uses the same hand-rolled algorithm, which is the
# educational point of this monorepo.


def _sha1(data: bytes) -> Key:
    """Return the 20-byte SHA-1 digest of *data* using coding-adventures-sha1."""
    return _sha1_impl(data)


# ─── Error Hierarchy ──────────────────────────────────────────────────────────
#
# We use a class hierarchy rather than a single enum so callers can catch
# specific errors with ``except CasNotFoundError:`` without matching on an
# enum variant — idiomatic Python.


class CasError(Exception):
    """Base class for all errors raised by this package.

    Catching ``CasError`` catches everything from this library.
    """


class CasStoreError(CasError):
    """The underlying :class:`BlobStore` raised an exception.

    The original exception is available via ``__cause__``.
    """

    def __init__(self, message: str, cause: Exception) -> None:
        super().__init__(message)
        self.__cause__ = cause


class CasNotFoundError(CasError):
    """A blob was requested by key but no such key exists in the store."""

    def __init__(self, key: Key) -> None:
        super().__init__(f"object not found: {key_to_hex(key)}")
        self.key: Key = key


class CasCorruptedError(CasError):
    """The stored bytes do not hash to the requested key.

    This indicates data corruption: the stored bytes have been modified since
    they were first written.  The ``key`` attribute is the *requested* key.
    """

    def __init__(self, key: Key) -> None:
        super().__init__(f"object corrupted: {key_to_hex(key)}")
        self.key: Key = key


class CasAmbiguousPrefixError(CasError):
    """A hex prefix matched two or more objects."""

    def __init__(self, prefix: str) -> None:
        super().__init__(f"ambiguous prefix: {prefix}")
        self.prefix: str = prefix


class CasPrefixNotFoundError(CasError):
    """A hex prefix matched zero objects."""

    def __init__(self, prefix: str) -> None:
        super().__init__(f"object not found for prefix: {prefix}")
        self.prefix: str = prefix


class CasInvalidPrefixError(CasError):
    """The supplied hex string is not valid hexadecimal, or is empty."""

    def __init__(self, prefix: str) -> None:
        super().__init__(f"invalid hex prefix: {prefix!r}")
        self.prefix: str = prefix


# ─── BlobStore Abstract Base Class ────────────────────────────────────────────
#
# The single abstraction that separates the CAS logic from persistence.
# Any class that can store and retrieve byte blobs by a 20-byte key qualifies.
#
# We use Python's ``abc.ABC`` / ``abc.abstractmethod`` pattern.  The associated
# error type from Rust's trait (``type Error``) collapses in Python: all methods
# simply raise exceptions.  Implementors should raise ``IOError`` (or a subclass)
# for I/O failures, and may raise their own exceptions for other failures.


class BlobStore(abc.ABC):
    """Abstract base class for a pluggable key-value store of raw byte blobs.

    Subclass this to implement a new storage backend.  The key is always a
    20-byte SHA-1 digest produced by :class:`ContentAddressableStore`;
    implementations should treat it as an opaque identifier.

    All methods operate on ``bytes`` keys of exactly 20 bytes.  Implementors
    do NOT need to verify hashes — that is the CAS layer's responsibility.

    Example custom in-memory backend::

        class MemoryStore(BlobStore):
            def __init__(self):
                self._data: dict[bytes, bytes] = {}

            def put(self, key: bytes, data: bytes) -> None:
                self._data[key] = data

            def get(self, key: bytes) -> bytes:
                if key not in self._data:
                    raise KeyError(key)
                return self._data[key]

            def exists(self, key: bytes) -> bool:
                return key in self._data

            def keys_with_prefix(self, prefix: bytes) -> list[bytes]:
                return [k for k in self._data if k[:len(prefix)] == prefix]
    """

    @abc.abstractmethod
    def put(self, key: Key, data: bytes) -> None:
        """Persist *data* under *key*.

        Implementations must be idempotent: storing the same key twice with the
        same bytes is not an error.  Storing a different blob under an existing
        key is undefined behaviour (the CAS layer prevents this by construction,
        since the same content always produces the same key).

        :param key: 20-byte SHA-1 digest.
        :param data: Raw bytes to store.
        :raises IOError: On I/O failure.
        """

    @abc.abstractmethod
    def get(self, key: Key) -> bytes:
        """Retrieve the blob stored under *key*.

        Implementations do NOT need to verify the hash.

        :param key: 20-byte SHA-1 digest.
        :returns: The stored bytes.
        :raises IOError: If the key is not present or I/O fails.
        """

    @abc.abstractmethod
    def exists(self, key: Key) -> bool:
        """Check whether *key* is present without fetching the blob.

        :param key: 20-byte SHA-1 digest.
        :returns: ``True`` if the key exists, ``False`` otherwise.
        :raises IOError: On I/O failure.
        """

    @abc.abstractmethod
    def keys_with_prefix(self, prefix: bytes) -> list[Key]:
        """Return all stored keys whose first ``len(prefix)`` bytes equal *prefix*.

        Used for abbreviated-hash lookup.  The CAS layer checks for uniqueness
        and reports ambiguity; the store just needs to return all matching keys.

        :param prefix: Byte prefix to match (1–20 bytes).
        :returns: List of matching 20-byte keys.
        :raises IOError: On I/O failure.
        """


# ─── ContentAddressableStore ──────────────────────────────────────────────────
#
# The CAS class owns one BlobStore instance and adds:
#
#   1. Automatic keying  — callers pass content; SHA-1 is computed internally.
#   2. Integrity check   — on every get, SHA-1(returned bytes) must equal the key.
#   3. Prefix resolution — converts abbreviated hex (like ``a3f4b2``) to a full key.


class ContentAddressableStore:
    """Content-addressable store that wraps a :class:`BlobStore` backend.

    All objects are keyed by their SHA-1 hash.  The same content always maps to
    the same key (deduplication), and the stored bytes are verified against the
    key on every read (integrity).

    :param store: Any :class:`BlobStore` implementation.

    Example::

        import tempfile, pathlib
        from coding_adventures_content_addressable_storage import ContentAddressableStore, LocalDiskStore

        with tempfile.TemporaryDirectory() as tmp:
            cas = ContentAddressableStore(LocalDiskStore(pathlib.Path(tmp)))
            key = cas.put(b"hello")
            assert cas.get(key) == b"hello"
    """

    def __init__(self, store: BlobStore) -> None:
        self._store: BlobStore = store

    # ------------------------------------------------------------------
    # Core operations
    # ------------------------------------------------------------------

    def put(self, data: bytes) -> Key:
        """Hash *data* with SHA-1, store it in the backend, and return the key.

        Idempotent: if the same content has already been stored, the existing
        key is returned and no write is performed (the backend handles this).

        :param data: Raw bytes to store.
        :returns: 20-byte SHA-1 key.
        :raises CasStoreError: If the backend raises an exception.

        >>> import tempfile, pathlib
        >>> with tempfile.TemporaryDirectory() as tmp:
        ...     cas = ContentAddressableStore(LocalDiskStore(pathlib.Path(tmp)))
        ...     k1 = cas.put(b"foo")
        ...     k2 = cas.put(b"foo")  # idempotent — same key returned
        ...     k1 == k2
        True
        """
        key = _sha1(data)
        # Delegate directly to the store.  BlobStore.put is required to be
        # idempotent, so no pre-check is needed here.  Skipping the
        # exists() → put() two-step eliminates a TOCTOU window and keeps
        # the CAS layer free of redundant round-trips.
        try:
            self._store.put(key, data)
        except Exception as exc:
            raise CasStoreError(str(exc), exc) from exc
        return key

    def get(self, key: Key) -> bytes:
        """Retrieve the blob stored under *key* and verify its integrity.

        The returned bytes are guaranteed to hash to *key* — if the store
        returns anything else, :class:`CasCorruptedError` is raised instead.

        :param key: 20-byte SHA-1 key.
        :returns: The stored bytes, verified by re-hashing.
        :raises CasNotFoundError: If the key is not present.
        :raises CasCorruptedError: If the stored bytes don't hash to *key*.
        :raises CasStoreError: If the backend raises an unexpected exception.
        """
        try:
            data = self._store.get(key)
        except (FileNotFoundError, KeyError) as exc:
            # Translate common "not found" errors from backends into our typed error.
            raise CasNotFoundError(key) from exc
        except Exception as exc:
            raise CasStoreError(str(exc), exc) from exc

        # Integrity check: re-hash the returned bytes.
        # If the stored content was tampered with, the hash won't match.
        actual = _sha1(data)
        if actual != key:
            raise CasCorruptedError(key)
        return data

    def exists(self, key: Key) -> bool:
        """Check whether a key is present in the store.

        :param key: 20-byte SHA-1 key.
        :returns: ``True`` if present, ``False`` otherwise.
        :raises CasStoreError: If the backend raises an exception.
        """
        try:
            return self._store.exists(key)
        except Exception as exc:
            raise CasStoreError(str(exc), exc) from exc

    def find_by_prefix(self, hex_prefix: str) -> Key:
        """Resolve an abbreviated hex string to a full 20-byte key.

        Accepts any non-empty hex string of 1–40 characters.  Odd-length
        strings are treated as nibble prefixes (e.g., ``"a3f"`` matches any
        key starting with ``0xa3 0xf_``).

        This mirrors ``git show <short-hash>``: you don't need to type all 40
        characters of a SHA-1 hash — just enough to be unambiguous.

        :param hex_prefix: 1–40 character hexadecimal prefix string.
        :returns: The unique matching 20-byte key.
        :raises CasInvalidPrefixError: Empty string or non-hex characters.
        :raises CasPrefixNotFoundError: No keys match.
        :raises CasAmbiguousPrefixError: Two or more keys match.
        :raises CasStoreError: If the backend raises an exception.
        """
        # Decode the prefix — raises ValueError on empty / invalid hex.
        try:
            prefix_bytes = _decode_hex_prefix(hex_prefix)
        except ValueError as exc:
            raise CasInvalidPrefixError(hex_prefix) from exc

        try:
            matches = self._store.keys_with_prefix(prefix_bytes)
        except Exception as exc:
            raise CasStoreError(str(exc), exc) from exc

        # Sort for deterministic behaviour (consistent with the Rust impl).
        matches.sort()

        match len(matches):
            case 0:
                raise CasPrefixNotFoundError(hex_prefix)
            case 1:
                return matches[0]
            case _:
                raise CasAmbiguousPrefixError(hex_prefix)

    @property
    def store(self) -> BlobStore:
        """Access the underlying :class:`BlobStore` directly.

        Useful for backend-specific operations not exposed by the CAS interface
        (e.g., listing all keys for garbage collection, or querying storage
        statistics).
        """
        return self._store


# ─── LocalDiskStore ───────────────────────────────────────────────────────────
#
# Filesystem backend using the Git 2/38 fanout layout.
#
# Why 2/38?  A repository with 100,000 objects would put 100,000 files in a
# single directory if we stored objects as root/<40-hex-hash>.  Most filesystems
# slow down dramatically at that scale.  Splitting on the first byte creates up
# to 256 sub-directories — each with at most ~390 entries for a 100k object
# repo.  Git has used this layout since its initial release (2005).
#
# Object path:  root/<xx>/<remaining-38-hex-chars>
#   key = bytes.fromhex("a3f4b2...")
#   dir  = "a3/"
#   file = "f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5"
#
# Atomic writes: we write to a temp file then call os.rename().  os.rename is
# atomic on POSIX (guaranteed by POSIX.1).  On Windows, it can fail if the
# destination already exists; we detect this and treat it as a successful
# idempotent write (another writer stored the same object first).


class LocalDiskStore(BlobStore):
    """Filesystem-backed :class:`BlobStore` using Git-style 2/38 fanout layout.

    Objects are stored at ``<root>/<xx>/<38-hex-chars>`` where ``xx`` is the
    first byte of the SHA-1 hash encoded as two lowercase hex digits.

    Writes are atomic: content is written to a temp file, then ``os.rename()``
    is called to move it into its final position.  The temp file name includes
    the PID and a nanosecond timestamp to avoid collisions and make it
    infeasible for a local attacker to pre-create a symlink at the temp path.

    :param root: Root directory for the object store.  Created if absent.

    Example::

        from coding_adventures_content_addressable_storage import ContentAddressableStore, LocalDiskStore
        import pathlib, tempfile

        with tempfile.TemporaryDirectory() as tmp:
            store = LocalDiskStore(pathlib.Path(tmp))
            cas = ContentAddressableStore(store)
            key = cas.put(b"data")
            assert cas.get(key) == b"data"
    """

    def __init__(self, root: pathlib.Path) -> None:
        self._root: pathlib.Path = root
        # Create the root directory if it does not exist.  exist_ok=True avoids
        # a TOCTOU race between checking and creating.
        self._root.mkdir(parents=True, exist_ok=True)

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _object_path(self, key: Key) -> pathlib.Path:
        """Compute the storage path for a given key.

        The first byte of the key encodes as a two-char directory name.
        The remaining 19 bytes encode as the 38-char filename.

          key  = bytes.fromhex("a3f4b2...")
          dir  = root/a3/
          file = root/a3/f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5
        """
        hex_str = key.hex()          # 40-char lowercase hex
        dir_name = hex_str[:2]       # first byte → 2-char directory
        file_name = hex_str[2:]      # remaining 38 chars → filename
        return self._root / dir_name / file_name

    # ------------------------------------------------------------------
    # BlobStore implementation
    # ------------------------------------------------------------------

    def put(self, key: Key, data: bytes) -> None:
        """Write *data* to disk under the 2/38 fanout path for *key*.

        The write is atomic: data is flushed to a temp file, then the temp file
        is renamed into its final position.  If the final file already exists,
        we skip the write entirely — the content is identical by construction.

        :param key: 20-byte SHA-1 digest.
        :param data: Raw bytes to write.
        :raises IOError: On filesystem failure.
        """
        final_path = self._object_path(key)

        # Short-circuit: if the file already exists, the object is already stored.
        # Because the key is a hash of the content, the stored bytes are
        # guaranteed to be identical — no need to overwrite.
        if final_path.exists():
            return

        # Create the two-char fanout directory (e.g., "a3/") if needed.
        # exist_ok=True prevents a race with a concurrent writer doing the same.
        final_path.parent.mkdir(parents=True, exist_ok=True)

        # Build a safe temp filename.
        #
        # Security: use an unpredictable temp filename (PID + nanosecond
        # timestamp) rather than a deterministic ``.tmp`` suffix.  A fixed path
        # like ``a3/f4b2....tmp`` could be pre-targeted by a local attacker who
        # places a symlink there before our write, redirecting the file write to
        # an arbitrary path.  Mixing the process ID and a high-resolution
        # nanosecond timestamp makes the name infeasible to predict without
        # privileged access to the process.
        #
        # Note: os.urandom is available in Python but we follow the note in the
        # task spec and use os.getpid() + time.time_ns() as in the Rust impl.
        pid = os.getpid()
        ts_ns = time.time_ns()
        tmp_name = f"{final_path.name}.{pid}.{ts_ns}.tmp"
        tmp_path = final_path.parent / tmp_name

        try:
            # Write the content to the temp file.
            tmp_path.write_bytes(data)

            # Rename into place.  On POSIX this is atomic (POSIX.1 guarantee).
            # On Windows, os.replace is used to overwrite atomically if the
            # dest already exists (concurrent writer race).
            try:
                # os.rename raises FileExistsError on Windows if the destination
                # exists.  Use os.replace for cross-platform atomic rename.
                # os.replace is atomic on POSIX and best-effort on Windows.
                os.replace(str(tmp_path), str(final_path))
            except OSError:
                # On Windows, if the final path already exists and the OS refused
                # the rename, we treat it as success — another writer stored the
                # same object concurrently.  The content is identical by
                # construction (same key → same hash → same data).
                if not final_path.exists():
                    raise
                # Clean up the orphaned temp file.
                with contextlib.suppress(OSError):
                    tmp_path.unlink()
        except Exception:
            # Ensure no temp file orphan is left on unexpected failure.
            with contextlib.suppress(OSError):
                tmp_path.unlink(missing_ok=True)
            raise

    def get(self, key: Key) -> bytes:
        """Read and return the raw bytes stored at the key's fanout path.

        :param key: 20-byte SHA-1 digest.
        :returns: Stored bytes.
        :raises FileNotFoundError: If the key is not present.
        :raises IOError: On other filesystem failures.
        """
        path = self._object_path(key)
        # read_bytes raises FileNotFoundError if the file is absent — the CAS
        # layer translates this to CasNotFoundError.
        return path.read_bytes()

    def exists(self, key: Key) -> bool:
        """Return ``True`` if the key's fanout path exists on disk.

        :param key: 20-byte SHA-1 digest.
        :returns: ``True`` if present, ``False`` otherwise.
        """
        return self._object_path(key).exists()

    def keys_with_prefix(self, prefix: bytes) -> list[Key]:
        """Scan the relevant fanout bucket and return all keys matching *prefix*.

        The first byte of *prefix* selects which two-char subdirectory to scan.
        We then read every 38-char filename in that directory, reconstruct the
        full 40-char hex key, parse it, and filter by full prefix match.

        Temp files (those not exactly 38 chars long, or containing non-hex
        chars) are silently skipped.

        :param prefix: 1–20 byte prefix to match.
        :returns: List of matching 20-byte keys.
        :raises IOError: On filesystem failure.
        """
        # A zero-length prefix is a degenerate case — reject defensively.
        # (The CAS layer already rejects empty hex strings, so this is defence-
        # in-depth.)
        if not prefix:
            return []

        # The first byte of the prefix tells us which fanout bucket to scan.
        # format with :02x to get exactly two lowercase hex digits.
        first_byte_hex = f"{prefix[0]:02x}"

        bucket = self._root / first_byte_hex

        if not bucket.exists():
            return []

        keys: list[Key] = []

        for entry in bucket.iterdir():
            if not entry.is_file():
                continue

            name = entry.name

            # Each valid object file is a 38-char hex string (the latter 38 chars
            # of the 40-char hash).  Skip temp files and other artifacts.
            if len(name) != 38:
                continue

            # Reconstruct the full 40-char hex and parse it.
            # `first_byte_hex` is always 2 chars and `name` is always 38 chars
            # (checked above), so `full_hex` is always exactly 40 chars and
            # `bytes.fromhex` will always return 20 bytes — but it may still
            # raise ValueError if `name` contains non-hex characters (e.g.,
            # temp files whose names happen to be 38 chars long).
            full_hex = first_byte_hex + name
            try:
                key = bytes.fromhex(full_hex)
            except ValueError:
                continue  # skip 38-char filenames with non-hex characters

            # Filter: the key must actually start with the full prefix bytes.
            # This matters when the prefix is longer than one byte.
            if key[: len(prefix)] == prefix:
                keys.append(key)

        return keys
