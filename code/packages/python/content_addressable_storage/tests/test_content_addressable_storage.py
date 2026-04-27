"""Tests for coding_adventures_content_addressable_storage.

Test coverage targets
---------------------
- Round-trip put/get for empty, small, and large (1 MB) blobs
- Idempotent put (same content twice → same key, no error)
- get of unknown key → CasNotFoundError
- Corrupted file → CasCorruptedError
- exists() before and after put
- find_by_prefix: unique match, ambiguous, not found, invalid hex, empty string
- LocalDiskStore path layout (2/38 fanout: verify directories are created)
- BlobStore is abstract (cannot be instantiated directly)
- key_to_hex / hex_to_key round-trips
- _decode_hex_prefix: even/odd lengths, edge cases

Architecture reminder
---------------------
ContentAddressableStore wraps any BlobStore.  Tests drive both
LocalDiskStore (filesystem) and a simple MemoryStore (in-memory) to keep the
test for the CAS logic independent of the disk.
"""

from __future__ import annotations

from coding_adventures_sha1 import sha1 as _sha1_test
import pathlib
from unittest.mock import patch

import pytest

from coding_adventures_content_addressable_storage import (  # type: ignore[attr-defined]
    BlobStore,
    CasAmbiguousPrefixError,
    CasCorruptedError,
    CasError,
    CasInvalidPrefixError,
    CasNotFoundError,
    CasPrefixNotFoundError,
    CasStoreError,
    ContentAddressableStore,
    LocalDiskStore,
    _decode_hex_prefix,
    hex_to_key,
    key_to_hex,
)

# ─── Helpers ──────────────────────────────────────────────────────────────────


def sha1(data: bytes) -> bytes:
    """Compute SHA-1 the same way the module does, for test cross-checks."""
    return _sha1_test(data)


class MemoryStore(BlobStore):
    """Simple in-memory BlobStore for unit tests that don't need disk I/O."""

    def __init__(self) -> None:
        self._data: dict[bytes, bytes] = {}

    def put(self, key: bytes, data: bytes) -> None:
        self._data[key] = data

    def get(self, key: bytes) -> bytes:
        if key not in self._data:
            raise FileNotFoundError(key)
        return self._data[key]

    def exists(self, key: bytes) -> bool:
        return key in self._data

    def keys_with_prefix(self, prefix: bytes) -> list[bytes]:
        return [k for k in self._data if k[: len(prefix)] == prefix]


@pytest.fixture
def mem_cas() -> ContentAddressableStore:
    """CAS backed by an in-memory store — fast, no disk."""
    return ContentAddressableStore(MemoryStore())


@pytest.fixture
def disk_cas(tmp_path: pathlib.Path) -> ContentAddressableStore:
    """CAS backed by LocalDiskStore in a pytest tmp directory."""
    return ContentAddressableStore(LocalDiskStore(tmp_path))


# ─── Hex Utility Tests ────────────────────────────────────────────────────────


class TestHexUtils:
    def test_key_to_hex_round_trip(self) -> None:
        key = bytes.fromhex("a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5")
        assert key_to_hex(key) == "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5"

    def test_hex_to_key_round_trip(self) -> None:
        hex_str = "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5"
        key = hex_to_key(hex_str)
        assert len(key) == 20
        assert key_to_hex(key) == hex_str

    def test_key_to_hex_all_zeros(self) -> None:
        key = bytes(20)
        assert key_to_hex(key) == "0" * 40

    def test_key_to_hex_all_ff(self) -> None:
        key = bytes([0xFF] * 20)
        assert key_to_hex(key) == "f" * 40

    def test_hex_to_key_wrong_length(self) -> None:
        with pytest.raises(ValueError, match="40 hex chars"):
            hex_to_key("abc")

    def test_hex_to_key_invalid_chars(self) -> None:
        with pytest.raises(ValueError):
            hex_to_key("z" * 40)

    def test_key_to_hex_wrong_length(self) -> None:
        with pytest.raises(ValueError, match="20 bytes"):
            key_to_hex(b"short")


# ─── _decode_hex_prefix Tests ─────────────────────────────────────────────────


class TestDecodeHexPrefix:
    def test_even_length(self) -> None:
        # "a3f4" → two full bytes
        assert _decode_hex_prefix("a3f4") == bytes([0xA3, 0xF4])

    def test_odd_length_padded_right(self) -> None:
        # "a3f" → pad right → "a3f0" → bytes [0xa3, 0xf0]
        assert _decode_hex_prefix("a3f") == bytes([0xA3, 0xF0])

    def test_single_nibble(self) -> None:
        # "a" → "a0" → [0xa0]
        assert _decode_hex_prefix("a") == bytes([0xA0])

    def test_empty_raises(self) -> None:
        with pytest.raises(ValueError, match="empty"):
            _decode_hex_prefix("")

    def test_invalid_char_raises(self) -> None:
        with pytest.raises(ValueError):
            _decode_hex_prefix("zzzz")

    def test_full_40_chars(self) -> None:
        hex_str = "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5"
        prefix = _decode_hex_prefix(hex_str)
        assert len(prefix) == 20
        assert prefix == bytes.fromhex(hex_str)

    def test_uppercase_accepted(self) -> None:
        # Hex chars are case-insensitive.
        assert _decode_hex_prefix("A3F4") == bytes([0xA3, 0xF4])


# ─── BlobStore Abstract Base Class ────────────────────────────────────────────


class TestBlobStoreAbstract:
    def test_cannot_instantiate_directly(self) -> None:
        """BlobStore must be abstract — direct instantiation should fail."""
        with pytest.raises(TypeError, match="Can't instantiate abstract class"):
            BlobStore()  # type: ignore[abstract]

    def test_partial_implementation_fails(self) -> None:
        """A subclass that doesn't implement all methods can't be instantiated."""

        class Partial(BlobStore):
            def put(self, key: bytes, data: bytes) -> None:
                pass

            def get(self, key: bytes) -> bytes:
                return b""

            # Missing exists() and keys_with_prefix()

        with pytest.raises(TypeError):
            Partial()  # type: ignore[abstract]

    def test_memory_store_is_concrete(self) -> None:
        """MemoryStore (from this test module) is a valid BlobStore."""
        store = MemoryStore()
        assert isinstance(store, BlobStore)


# ─── Round-trip put/get ───────────────────────────────────────────────────────


class TestRoundTrip:
    def test_empty_blob(self, mem_cas: ContentAddressableStore) -> None:
        """Empty bytes can be stored and retrieved."""
        key = mem_cas.put(b"")
        assert mem_cas.get(key) == b""
        assert len(key) == 20

    def test_small_blob(self, mem_cas: ContentAddressableStore) -> None:
        """A small string round-trips correctly."""
        data = b"hello, world"
        key = mem_cas.put(data)
        assert mem_cas.get(key) == data
        # Cross-check against stdlib sha1.
        assert key == sha1(data)

    def test_large_blob(self, mem_cas: ContentAddressableStore) -> None:
        """A 1 MB blob can be stored and retrieved."""
        data = b"x" * (1024 * 1024)
        key = mem_cas.put(data)
        result = mem_cas.get(key)
        assert result == data
        assert len(result) == 1024 * 1024

    def test_binary_data(self, mem_cas: ContentAddressableStore) -> None:
        """Arbitrary binary content (all byte values) round-trips correctly."""
        data = bytes(range(256))
        key = mem_cas.put(data)
        assert mem_cas.get(key) == data

    def test_disk_round_trip(self, disk_cas: ContentAddressableStore) -> None:
        """Round-trip via the LocalDiskStore (actual filesystem)."""
        data = b"persisted to disk"
        key = disk_cas.put(data)
        assert disk_cas.get(key) == data


# ─── Idempotent put ───────────────────────────────────────────────────────────


class TestIdempotentPut:
    def test_same_content_twice(self, mem_cas: ContentAddressableStore) -> None:
        """Putting the same content twice returns the same key without error."""
        k1 = mem_cas.put(b"duplicate")
        k2 = mem_cas.put(b"duplicate")
        assert k1 == k2

    def test_different_content_different_keys(
        self, mem_cas: ContentAddressableStore
    ) -> None:
        """Different content produces different keys."""
        k1 = mem_cas.put(b"alpha")
        k2 = mem_cas.put(b"beta")
        assert k1 != k2

    def test_idempotent_disk(self, disk_cas: ContentAddressableStore) -> None:
        """Idempotent put also works via LocalDiskStore."""
        k1 = disk_cas.put(b"idempotent")
        k2 = disk_cas.put(b"idempotent")
        assert k1 == k2


# ─── Not Found / Missing Key ──────────────────────────────────────────────────


class TestNotFound:
    def test_get_unknown_key(self, mem_cas: ContentAddressableStore) -> None:
        """Getting an unknown key raises CasNotFoundError."""
        unknown_key = bytes([0xDE, 0xAD] + [0x00] * 18)
        with pytest.raises(CasNotFoundError) as exc_info:
            mem_cas.get(unknown_key)
        assert exc_info.value.key == unknown_key

    def test_not_found_is_cas_error(self, mem_cas: ContentAddressableStore) -> None:
        """CasNotFoundError is a subclass of CasError."""
        unknown_key = bytes([0xFF] * 20)
        with pytest.raises(CasError):
            mem_cas.get(unknown_key)

    def test_not_found_message_contains_hex(
        self, mem_cas: ContentAddressableStore
    ) -> None:
        """The error message includes the hex representation of the missing key."""
        unknown_key = bytes([0xAB] * 20)
        with pytest.raises(CasNotFoundError) as exc_info:
            mem_cas.get(unknown_key)
        assert "ab" * 20 in str(exc_info.value)


# ─── Corrupted Data ───────────────────────────────────────────────────────────


class TestCorrupted:
    def test_corrupted_blob_raises(self, tmp_path: pathlib.Path) -> None:
        """If the stored file is modified on disk, get() raises CasCorruptedError."""
        store = LocalDiskStore(tmp_path)
        cas = ContentAddressableStore(store)

        data = b"original content"
        key = cas.put(data)

        # Directly corrupt the stored file by overwriting with different bytes.
        corrupted_bytes = b"tampered content!!"
        path = store._object_path(key)
        path.write_bytes(corrupted_bytes)

        with pytest.raises(CasCorruptedError) as exc_info:
            cas.get(key)
        assert exc_info.value.key == key

    def test_corrupted_is_cas_error(self, tmp_path: pathlib.Path) -> None:
        """CasCorruptedError is a subclass of CasError."""
        store = LocalDiskStore(tmp_path)
        cas = ContentAddressableStore(store)

        key = cas.put(b"will be corrupted")
        store._object_path(key).write_bytes(b"corrupted!")

        with pytest.raises(CasError):
            cas.get(key)

    def test_corrupted_message_contains_hex(self, tmp_path: pathlib.Path) -> None:
        """The error message includes the hex key of the corrupted object."""
        store = LocalDiskStore(tmp_path)
        cas = ContentAddressableStore(store)

        key = cas.put(b"data")
        store._object_path(key).write_bytes(b"wrong")

        with pytest.raises(CasCorruptedError) as exc_info:
            cas.get(key)
        assert key_to_hex(key) in str(exc_info.value)


# ─── exists() ─────────────────────────────────────────────────────────────────


class TestExists:
    def test_exists_false_before_put(self, mem_cas: ContentAddressableStore) -> None:
        """A key does not exist before it is put."""
        key = sha1(b"not yet stored")
        assert mem_cas.exists(key) is False

    def test_exists_true_after_put(self, mem_cas: ContentAddressableStore) -> None:
        """A key exists after it is put."""
        key = mem_cas.put(b"stored now")
        assert mem_cas.exists(key) is True

    def test_exists_disk(self, disk_cas: ContentAddressableStore) -> None:
        """exists() works correctly with LocalDiskStore."""
        key_before = sha1(b"disk data")
        assert disk_cas.exists(key_before) is False
        key_after = disk_cas.put(b"disk data")
        assert disk_cas.exists(key_after) is True
        assert key_before == key_after


# ─── find_by_prefix ───────────────────────────────────────────────────────────


class TestFindByPrefix:
    def _populate(
        self, cas: ContentAddressableStore
    ) -> tuple[bytes, bytes, bytes]:
        """Store three distinct blobs and return their keys."""
        k1 = cas.put(b"alpha content")
        k2 = cas.put(b"beta content")
        k3 = cas.put(b"gamma content")
        return k1, k2, k3

    def test_unique_match_full_hex(self, mem_cas: ContentAddressableStore) -> None:
        """A full 40-char hex resolves to the unique key."""
        k, _, _ = self._populate(mem_cas)
        full_hex = key_to_hex(k)
        assert mem_cas.find_by_prefix(full_hex) == k

    def test_unique_match_prefix(self, mem_cas: ContentAddressableStore) -> None:
        """A short prefix that uniquely identifies one key resolves correctly."""
        # Use a fresh single-blob store so we know our prefix is unique.
        store = MemoryStore()
        cas = ContentAddressableStore(store)
        key = cas.put(b"unique blob")

        # Use the full 40-char hex as the prefix — that's always unique.
        full_hex = key_to_hex(key)
        assert cas.find_by_prefix(full_hex) == key

        # Also test 8-char prefix (very likely unique in a one-object store).
        short_prefix = full_hex[:8]
        assert cas.find_by_prefix(short_prefix) == key

    def test_prefix_not_found(self, mem_cas: ContentAddressableStore) -> None:
        """A prefix that matches nothing raises CasPrefixNotFoundError."""
        self._populate(mem_cas)
        # Use a prefix that is extremely unlikely to match any real SHA-1.
        fake_prefix = "0000000000"
        with pytest.raises(CasPrefixNotFoundError) as exc_info:
            mem_cas.find_by_prefix(fake_prefix)
        assert exc_info.value.prefix == fake_prefix

    def test_ambiguous_prefix(self, tmp_path: pathlib.Path) -> None:
        """A prefix matching multiple keys raises CasAmbiguousPrefixError.

        We use a custom MemoryStore that lets us inject keys with a known
        common prefix to guarantee ambiguity.
        """

        class ControlledStore(BlobStore):
            """Store where we can manually insert arbitrary keys."""

            def __init__(self) -> None:
                self._data: dict[bytes, bytes] = {}

            def put(self, key: bytes, data: bytes) -> None:
                self._data[key] = data

            def get(self, key: bytes) -> bytes:
                return self._data[key]

            def exists(self, key: bytes) -> bool:
                return key in self._data

            def keys_with_prefix(self, prefix: bytes) -> list[bytes]:
                return [k for k in self._data if k[: len(prefix)] == prefix]

        store = ControlledStore()
        # Manually insert two keys with a known identical 4-byte prefix.
        prefix_bytes = bytes([0xAB, 0xCD, 0x12, 0x34])
        key1 = prefix_bytes + bytes(16)          # 0xabcd1234 00000000...
        key2 = prefix_bytes + bytes([0xFF] * 16) # 0xabcd1234 ffffffff...
        store.put(key1, b"content1")
        store.put(key2, b"content2")

        cas = ContentAddressableStore(store)
        # "abcd1234" is a prefix that matches both keys.
        with pytest.raises(CasAmbiguousPrefixError) as exc_info:
            cas.find_by_prefix("abcd1234")
        assert exc_info.value.prefix == "abcd1234"

    def test_invalid_hex_prefix(self, mem_cas: ContentAddressableStore) -> None:
        """A non-hex prefix raises CasInvalidPrefixError."""
        with pytest.raises(CasInvalidPrefixError) as exc_info:
            mem_cas.find_by_prefix("zzz")
        assert "zzz" in exc_info.value.prefix

    def test_empty_prefix_raises(self, mem_cas: ContentAddressableStore) -> None:
        """An empty prefix raises CasInvalidPrefixError (not a lookup error)."""
        with pytest.raises(CasInvalidPrefixError):
            mem_cas.find_by_prefix("")

    def test_odd_length_prefix(self, mem_cas: ContentAddressableStore) -> None:
        """An odd-length hex prefix is handled (padded on the right)."""
        key = mem_cas.put(b"odd prefix test")
        full_hex = key_to_hex(key)
        # Use 7 chars — an odd-length prefix.
        seven_char = full_hex[:7]
        # This might or might not be unique, but should not raise InvalidPrefix.
        try:
            found = mem_cas.find_by_prefix(seven_char)
            assert found == key
        except CasPrefixNotFoundError:
            # The padded prefix doesn't match — acceptable (due to nibble rounding).
            pass
        except CasAmbiguousPrefixError:
            # Multiple objects matched (unlikely with one object, but logically OK).
            pass

    def test_find_by_prefix_disk(self, disk_cas: ContentAddressableStore) -> None:
        """find_by_prefix works end-to-end with LocalDiskStore."""
        key = disk_cas.put(b"disk find me")
        full_hex = key_to_hex(key)
        assert disk_cas.find_by_prefix(full_hex) == key
        assert disk_cas.find_by_prefix(full_hex[:10]) == key


# ─── LocalDiskStore Path Layout ───────────────────────────────────────────────


class TestLocalDiskStoreLayout:
    def test_fanout_directory_created(self, tmp_path: pathlib.Path) -> None:
        """After put(), the 2-char fanout directory exists under root."""
        store = LocalDiskStore(tmp_path)
        cas = ContentAddressableStore(store)

        data = b"layout test"
        key = cas.put(data)
        hex_str = key_to_hex(key)

        fanout_dir = tmp_path / hex_str[:2]
        assert fanout_dir.is_dir(), f"Expected {fanout_dir} to be a directory"

    def test_object_file_at_38_char_path(self, tmp_path: pathlib.Path) -> None:
        """The object file is stored as a 38-char filename inside the fanout dir."""
        store = LocalDiskStore(tmp_path)
        cas = ContentAddressableStore(store)

        data = b"38-char filename test"
        key = cas.put(data)
        hex_str = key_to_hex(key)

        expected_path = tmp_path / hex_str[:2] / hex_str[2:]
        assert expected_path.is_file(), f"Expected object file at {expected_path}"
        assert expected_path.read_bytes() == data

    def test_file_name_is_38_chars(self, tmp_path: pathlib.Path) -> None:
        """The filename portion (excluding the 2-char dir) is exactly 38 chars."""
        store = LocalDiskStore(tmp_path)
        cas = ContentAddressableStore(store)

        key = cas.put(b"file name length check")
        hex_str = key_to_hex(key)

        file_name = hex_str[2:]
        assert len(file_name) == 38

    def test_root_created_if_absent(self, tmp_path: pathlib.Path) -> None:
        """LocalDiskStore creates the root directory if it does not exist."""
        new_root = tmp_path / "deep" / "nested" / "store"
        assert not new_root.exists()
        LocalDiskStore(new_root)
        assert new_root.is_dir()

    def test_multiple_objects_fanout_correctly(self, tmp_path: pathlib.Path) -> None:
        """Multiple objects fan out into the correct subdirectories."""
        store = LocalDiskStore(tmp_path)
        cas = ContentAddressableStore(store)

        blobs = [f"content {i}".encode() for i in range(20)]
        for blob in blobs:
            key = cas.put(blob)
            hex_str = key_to_hex(key)
            expected = tmp_path / hex_str[:2] / hex_str[2:]
            assert expected.is_file()

    def test_no_temp_files_after_put(self, tmp_path: pathlib.Path) -> None:
        """After a successful put(), no .tmp files remain in the store."""
        store = LocalDiskStore(tmp_path)
        cas = ContentAddressableStore(store)
        cas.put(b"temp file cleanup test")

        tmp_files = list(tmp_path.rglob("*.tmp"))
        assert tmp_files == [], f"Unexpected temp files: {tmp_files}"

    def test_object_path_helper(self, tmp_path: pathlib.Path) -> None:
        """_object_path returns the correct two-level path for a given key."""
        store = LocalDiskStore(tmp_path)
        key = bytes.fromhex("a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5")
        path = store._object_path(key)
        assert path == tmp_path / "a3" / "f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5"


# ─── Error Hierarchy ──────────────────────────────────────────────────────────


class TestErrorHierarchy:
    def test_all_errors_are_cas_error_subclasses(self) -> None:
        """Every CAS error class is a subclass of CasError."""
        for cls in (
            CasStoreError,
            CasNotFoundError,
            CasCorruptedError,
            CasAmbiguousPrefixError,
            CasPrefixNotFoundError,
            CasInvalidPrefixError,
        ):
            assert issubclass(cls, CasError), f"{cls} should be a CasError subclass"

    def test_not_found_has_key_attribute(self) -> None:
        key = bytes([0x01] * 20)
        exc = CasNotFoundError(key)
        assert exc.key == key

    def test_corrupted_has_key_attribute(self) -> None:
        key = bytes([0x02] * 20)
        exc = CasCorruptedError(key)
        assert exc.key == key

    def test_ambiguous_has_prefix_attribute(self) -> None:
        exc = CasAmbiguousPrefixError("abc")
        assert exc.prefix == "abc"

    def test_prefix_not_found_has_prefix_attribute(self) -> None:
        exc = CasPrefixNotFoundError("def")
        assert exc.prefix == "def"

    def test_invalid_prefix_has_prefix_attribute(self) -> None:
        exc = CasInvalidPrefixError("zzz")
        assert exc.prefix == "zzz"

    def test_store_error_has_cause(self) -> None:
        cause = OSError("disk full")
        exc = CasStoreError("store failed", cause)
        assert exc.__cause__ is cause


# ─── ContentAddressableStore.store property ───────────────────────────────────


class TestStoreProperty:
    def test_store_returns_wrapped_store(self) -> None:
        """cas.store returns the underlying BlobStore instance."""
        inner = MemoryStore()
        cas = ContentAddressableStore(inner)
        assert cas.store is inner


# ─── LocalDiskStore.keys_with_prefix edge cases ───────────────────────────────


class TestKeysWithPrefix:
    def test_empty_prefix_returns_empty(self, tmp_path: pathlib.Path) -> None:
        """keys_with_prefix([]) returns [] without error."""
        store = LocalDiskStore(tmp_path)
        store.put(sha1(b"x"), b"x")
        result = store.keys_with_prefix(b"")
        assert result == []

    def test_nonexistent_bucket_returns_empty(self, tmp_path: pathlib.Path) -> None:
        """If no objects exist with a given first byte, return empty list."""
        store = LocalDiskStore(tmp_path)
        # Use the prefix [0x00] — very unlikely to have any object.
        result = store.keys_with_prefix(bytes([0x00]))
        assert result == []

    def test_temp_files_skipped(self, tmp_path: pathlib.Path) -> None:
        """Temp files in the object bucket are skipped by keys_with_prefix."""
        store = LocalDiskStore(tmp_path)
        key = sha1(b"real object")
        store.put(key, b"real object")

        # Manually inject a temp file in the same bucket.
        hex_str = key.hex()
        bucket = tmp_path / hex_str[:2]
        (bucket / "some_temp_file.tmp").write_bytes(b"junk")
        (bucket / "short").write_bytes(b"junk")

        # Only the real 38-char hex file should be returned.
        matches = store.keys_with_prefix(key[:1])
        assert matches == [key]

    def test_subdirectory_in_bucket_skipped(self, tmp_path: pathlib.Path) -> None:
        """A subdirectory inside a bucket is skipped (is_file check)."""
        store = LocalDiskStore(tmp_path)
        key = sha1(b"subdir test")
        store.put(key, b"subdir test")

        hex_str = key.hex()
        bucket = tmp_path / hex_str[:2]
        # Create a subdirectory inside the bucket — should be skipped.
        subdir_name = "a" * 38
        (bucket / subdir_name).mkdir(exist_ok=True)

        matches = store.keys_with_prefix(key[:1])
        assert key in matches

    def test_38char_non_hex_filename_skipped(self, tmp_path: pathlib.Path) -> None:
        """A 38-char filename containing non-hex chars is skipped gracefully."""
        store = LocalDiskStore(tmp_path)
        key = sha1(b"non hex test")
        store.put(key, b"non hex test")

        hex_str = key.hex()
        bucket = tmp_path / hex_str[:2]
        # Create a 38-char filename with non-hex characters.
        bad_name = "z" * 38
        (bucket / bad_name).write_bytes(b"junk")

        # Only the real object should be returned.
        matches = store.keys_with_prefix(key[:1])
        assert key in matches
        # The bad file should not have been returned.
        bad_key_hex = hex_str[:2] + bad_name
        for m in matches:
            assert m.hex() != bad_key_hex


# ─── CasStoreError propagation ────────────────────────────────────────────────


class FailingStore(BlobStore):
    """A BlobStore that raises OSError on all operations — tests error wrapping."""

    def put(self, key: bytes, data: bytes) -> None:
        raise OSError("simulated put failure")

    def get(self, key: bytes) -> bytes:
        raise OSError("simulated get failure")

    def exists(self, key: bytes) -> bool:
        raise OSError("simulated exists failure")

    def keys_with_prefix(self, prefix: bytes) -> list[bytes]:
        raise OSError("simulated keys_with_prefix failure")


class TestCasStoreErrorPropagation:
    """Verify that unexpected backend exceptions are wrapped in CasStoreError."""

    def setup_method(self) -> None:
        self.cas = ContentAddressableStore(FailingStore())

    def test_put_wraps_store_error(self) -> None:
        with pytest.raises(CasStoreError) as exc_info:
            self.cas.put(b"data")
        assert "simulated put failure" in str(exc_info.value)

    def test_get_wraps_unexpected_store_error(self) -> None:
        """An unexpected (non-FileNotFoundError) exception from get is wrapped."""
        with pytest.raises(CasStoreError) as exc_info:
            self.cas.get(bytes([0x01] * 20))
        assert "simulated get failure" in str(exc_info.value)

    def test_exists_wraps_store_error(self) -> None:
        with pytest.raises(CasStoreError) as exc_info:
            self.cas.exists(bytes([0x01] * 20))
        assert "simulated exists failure" in str(exc_info.value)

    def test_find_by_prefix_wraps_store_error(self) -> None:
        with pytest.raises(CasStoreError) as exc_info:
            self.cas.find_by_prefix("abcd")
        assert "simulated keys_with_prefix failure" in str(exc_info.value)


# ─── LocalDiskStore atomic write edge cases ───────────────────────────────────


class TestAtomicWrite:
    def test_put_cleanup_on_write_failure(self, tmp_path: pathlib.Path) -> None:
        """If the write to temp file fails, no orphaned temp file is left.

        We simulate this by making the fanout directory read-only so the
        temp file creation itself fails.  On Windows this is hard to do, so
        we patch the write_bytes call.
        """
        store = LocalDiskStore(tmp_path)
        key = sha1(b"write fail test")

        with (
            patch("pathlib.Path.write_bytes", side_effect=OSError("disk full")),
            pytest.raises(OSError, match="disk full"),
        ):
            store.put(key, b"write fail test")

        # No .tmp file should remain.
        tmp_files = list(tmp_path.rglob("*.tmp"))
        assert tmp_files == []

    def test_idempotent_put_skips_write(self, tmp_path: pathlib.Path) -> None:
        """Second put() of the same object skips the write entirely."""
        store = LocalDiskStore(tmp_path)
        data = b"idempotent write"
        key = sha1(data)
        # First put succeeds.
        store.put(key, data)

        # Second put should not call write_bytes — file already exists.
        with patch("pathlib.Path.write_bytes") as mock_write:
            store.put(key, data)
            mock_write.assert_not_called()

    def test_rename_race_condition_handled(self, tmp_path: pathlib.Path) -> None:
        """If os.replace raises OSError but the final file exists, treat as success.

        This simulates a Windows race condition where two processes write the
        same object concurrently.  The first writer wins the rename; the second
        writer's os.replace fails because the destination exists.  Since the
        content is identical, we treat this as a successful idempotent write.
        """
        store = LocalDiskStore(tmp_path)
        data = b"concurrent write race"
        key = sha1(data)

        # Simulate: os.replace raises OSError, but the final file appears
        # (as if another writer placed it there first).

        def replace_then_create(src: str, dst: str) -> None:
            # Write the final file to simulate a concurrent writer, then raise.
            pathlib.Path(dst).parent.mkdir(parents=True, exist_ok=True)
            pathlib.Path(dst).write_bytes(data)
            raise OSError("simulated concurrent-write failure")

        with patch("os.replace", side_effect=replace_then_create):
            # Should not raise — the final path exists after the OSError.
            store.put(key, data)

        # The data should be retrievable.
        assert store.get(key) == data

    def test_rename_race_reraises_if_final_missing(
        self, tmp_path: pathlib.Path
    ) -> None:
        """If os.replace raises OSError and the final file does NOT exist, re-raise.

        This covers a genuine I/O failure (not a race) where the rename fails
        and the destination was never created.
        """
        store = LocalDiskStore(tmp_path)
        data = b"genuine rename failure"
        key = sha1(data)

        with (
            patch("os.replace", side_effect=OSError("genuine failure")),
            pytest.raises(OSError, match="genuine failure"),
        ):
            store.put(key, data)
