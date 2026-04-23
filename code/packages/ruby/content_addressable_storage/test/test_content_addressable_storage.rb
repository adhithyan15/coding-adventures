# frozen_string_literal: true

# Tests for the CodingAdventures::ContentAddressableStorage package.
#
# Test Organization
# =================
#
#   TestHexUtilities          — key_to_hex, hex_to_key round-trips and edge cases
#   TestDecodeHexPrefix       — decode_hex_prefix padding, validation
#   TestCasErrors             — error class hierarchy, attributes, messages
#   TestBlobStoreMixin        — BlobStore as a module mixin
#   TestLocalDiskStoreLayout  — 2/38 fanout path layout
#   TestLocalDiskStorePut     — idempotent write, atomic rename
#   TestLocalDiskStoreGet     — round-trip, not-found
#   TestLocalDiskStoreExists  — exists? before/after put
#   TestLocalDiskStorePrefix  — keys_with_prefix scanning
#   TestContentAddressableStore — end-to-end round-trips, integrity, prefix resolution
#
# All tests use Dir.mktmpdir to create an isolated temporary directory per test
# so that tests do not share state.

require "minitest/autorun"
require "tmpdir"
require "coding_adventures_sha1"
require "coding_adventures_content_addressable_storage"

CAS = CodingAdventures::ContentAddressableStorage

# ─── Helpers ──────────────────────────────────────────────────────────────────

# Returns a fresh temporary directory path.  The block form of Dir.mktmpdir
# auto-deletes on exit; here we use the path form and clean up manually in
# teardown so that individual tests can share the setup.
def tmpdir
  Dir.mktmpdir("cas-test-")
end

# ─── TestHexUtilities ─────────────────────────────────────────────────────────

class TestHexUtilities < Minitest::Test
  # A known 20-byte key — all distinct nibbles for easy visual verification.
  KEY_BYTES = "\xa3\xf4\xb2\xc1\xd0\xe9\xf8\xa7\xb6\xc5\xd4\xe3\xf2\xa1\xb0\xc9\xd8\xe7\xf6\xa5"
  KEY_HEX   = "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5"

  def test_key_to_hex_produces_40_chars
    key = KEY_BYTES.b
    assert_equal 40, CAS.key_to_hex(key).length
  end

  def test_key_to_hex_correct_value
    assert_equal KEY_HEX, CAS.key_to_hex(KEY_BYTES.b)
  end

  def test_hex_to_key_roundtrip
    key = CAS.hex_to_key(KEY_HEX)
    assert_equal 20, key.bytesize
    assert_equal Encoding::BINARY, key.encoding
    assert_equal KEY_HEX, CAS.key_to_hex(key)
  end

  def test_hex_to_key_accepts_uppercase
    upper = KEY_HEX.upcase
    lower = KEY_HEX.downcase
    assert_equal CAS.hex_to_key(lower), CAS.hex_to_key(upper)
  end

  def test_hex_to_key_rejects_short
    assert_raises(ArgumentError) { CAS.hex_to_key("a3f4") }
  end

  def test_hex_to_key_rejects_non_hex
    bad = "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6zz"
    assert_raises(ArgumentError) { CAS.hex_to_key(bad) }
  end

  def test_hex_to_key_rejects_empty
    assert_raises(ArgumentError) { CAS.hex_to_key("") }
  end

  def test_key_is_binary_encoding
    key = CAS.hex_to_key(KEY_HEX)
    assert_equal Encoding::BINARY, key.encoding
  end

  def test_all_zeros_key
    # Force binary (ASCII-8BIT) encoding so assert_equal doesn't fail on
    # encoding mismatch: hex_to_key always returns ASCII-8BIT via pack("H*").
    key = ("\x00" * 20).b
    hex = CAS.key_to_hex(key)
    assert_equal "00" * 20, hex
    assert_equal key, CAS.hex_to_key(hex)
  end

  def test_all_ff_key
    # "\xff" in a UTF-8 source file produces encoding UTF-8 + valid:false, but
    # hex_to_key returns ASCII-8BIT.  Force binary so the encodings match.
    key = ("\xff" * 20).b
    hex = CAS.key_to_hex(key)
    assert_equal "ff" * 20, hex
    assert_equal key, CAS.hex_to_key(hex)
  end
end

# ─── TestDecodeHexPrefix ──────────────────────────────────────────────────────

class TestDecodeHexPrefix < Minitest::Test
  def test_empty_raises
    # An empty prefix would match everything — not useful.
    assert_raises(ArgumentError) { CAS.decode_hex_prefix("") }
  end

  def test_even_length_no_padding
    # "a3f4" → 2 bytes: 0xa3, 0xf4
    result = CAS.decode_hex_prefix("a3f4")
    assert_equal "\xa3\xf4".b, result
  end

  def test_odd_length_pads_right
    # "a3f" → pad to "a3f0" → 0xa3, 0xf0
    result = CAS.decode_hex_prefix("a3f")
    assert_equal "\xa3\xf0".b, result
  end

  def test_single_char_pads_right
    # "a" → pad to "a0" → 0xa0
    result = CAS.decode_hex_prefix("a")
    assert_equal "\xa0".b, result
  end

  def test_invalid_chars_raise
    assert_raises(ArgumentError) { CAS.decode_hex_prefix("xyz") }
    assert_raises(ArgumentError) { CAS.decode_hex_prefix("a3g4") }
    assert_raises(ArgumentError) { CAS.decode_hex_prefix("a3 f4") }
  end

  def test_uppercase_valid
    # Uppercase hex is valid — same bytes as lowercase.
    result = CAS.decode_hex_prefix("A3F4")
    assert_equal "\xa3\xf4".b, result
  end

  def test_full_40_char_prefix
    # A 40-char prefix is the entire key — still valid.
    hex = "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5"
    result = CAS.decode_hex_prefix(hex)
    assert_equal 20, result.bytesize
  end

  def test_result_is_binary_encoding
    result = CAS.decode_hex_prefix("a3f4")
    assert_equal Encoding::BINARY, result.encoding
  end
end

# ─── TestCasErrors ────────────────────────────────────────────────────────────

class TestCasErrors < Minitest::Test
  KEY = "\xde\xad\xbe\xef\x00" * 4

  def test_hierarchy_all_inherit_cas_error
    assert CAS::CasNotFoundError.ancestors.include?(CAS::CasError)
    assert CAS::CasCorruptedError.ancestors.include?(CAS::CasError)
    assert CAS::CasAmbiguousPrefixError.ancestors.include?(CAS::CasError)
    assert CAS::CasPrefixNotFoundError.ancestors.include?(CAS::CasError)
    assert CAS::CasInvalidPrefixError.ancestors.include?(CAS::CasError)
  end

  def test_cas_error_inherits_standard_error
    assert CAS::CasError.ancestors.include?(StandardError)
  end

  def test_not_found_carries_key
    err = CAS::CasNotFoundError.new(KEY)
    assert_equal KEY, err.key
  end

  def test_not_found_message_contains_hex
    err = CAS::CasNotFoundError.new(KEY)
    assert_includes err.message, CAS.key_to_hex(KEY)
  end

  def test_corrupted_carries_key
    err = CAS::CasCorruptedError.new(KEY)
    assert_equal KEY, err.key
  end

  def test_corrupted_message_contains_hex
    err = CAS::CasCorruptedError.new(KEY)
    assert_includes err.message, CAS.key_to_hex(KEY)
  end

  def test_ambiguous_prefix_carries_prefix
    err = CAS::CasAmbiguousPrefixError.new("a3f4")
    assert_equal "a3f4", err.prefix
    assert_includes err.message, "a3f4"
  end

  def test_prefix_not_found_carries_prefix
    err = CAS::CasPrefixNotFoundError.new("deadbeef")
    assert_equal "deadbeef", err.prefix
    assert_includes err.message, "deadbeef"
  end

  def test_invalid_prefix_carries_prefix
    err = CAS::CasInvalidPrefixError.new("xyz!")
    assert_equal "xyz!", err.prefix
    assert_includes err.message, "xyz!"
  end

  def test_rescue_base_catches_all
    exceptions = [
      CAS::CasNotFoundError.new(KEY),
      CAS::CasCorruptedError.new(KEY),
      CAS::CasAmbiguousPrefixError.new("ab"),
      CAS::CasPrefixNotFoundError.new("ab"),
      CAS::CasInvalidPrefixError.new("zz")
    ]
    exceptions.each do |e|
      rescued = false
      begin
        raise e
      rescue CAS::CasError
        rescued = true
      end
      assert rescued, "Expected #{e.class} to be rescued as CasError"
    end
  end
end

# ─── TestBlobStoreMixin ───────────────────────────────────────────────────────

class TestBlobStoreMixin < Minitest::Test
  # A minimal in-memory BlobStore that includes the mixin and implements all
  # four required methods. This verifies that the module-as-interface pattern
  # works correctly in Ruby.
  class MemStore
    include CAS::BlobStore

    def initialize
      @data = {}
    end

    def put(key, data)
      @data[key] = data.b unless @data.key?(key)
    end

    def get(key)
      raise Errno::ENOENT, "not found" unless @data.key?(key)

      @data[key]
    end

    def exists?(key)
      @data.key?(key)
    end

    def keys_with_prefix(prefix)
      @data.keys.select { |k| k.start_with?(prefix) }
    end
  end

  def test_include_works
    # MemStore includes BlobStore — no error at class definition.
    assert_includes MemStore.ancestors, CAS::BlobStore
  end

  def test_mem_store_put_and_get
    store = MemStore.new
    key = CodingAdventures::Sha1.sha1("hello")
    store.put(key, "hello")
    assert_equal "hello".b, store.get(key)
  end

  def test_mem_store_exists
    store = MemStore.new
    key = CodingAdventures::Sha1.sha1("exists-test")
    refute store.exists?(key)
    store.put(key, "exists-test")
    assert store.exists?(key)
  end

  def test_mem_store_keys_with_prefix
    store = MemStore.new
    key1 = CodingAdventures::Sha1.sha1("alpha")
    key2 = CodingAdventures::Sha1.sha1("beta")
    store.put(key1, "alpha")
    store.put(key2, "beta")
    prefix = key1[0, 2]
    result = store.keys_with_prefix(prefix)
    # At minimum key1 should match; key2 may or may not share the prefix.
    assert_includes result, key1
  end

  def test_unimplemented_methods_raise_not_implemented
    # A class that includes BlobStore but implements nothing should raise
    # NotImplementedError when any method is called.
    bare = Class.new { include CAS::BlobStore }.new
    assert_raises(NotImplementedError) { bare.put("x", "y") }
    assert_raises(NotImplementedError) { bare.get("x") }
    assert_raises(NotImplementedError) { bare.exists?("x") }
    assert_raises(NotImplementedError) { bare.keys_with_prefix("x") }
  end

  def test_cas_wraps_mem_store
    # Full end-to-end through ContentAddressableStore with an in-memory backend.
    store = MemStore.new
    cas = CAS::ContentAddressableStore.new(store)
    key = cas.put("via mem store")
    assert_equal "via mem store".b, cas.get(key)
  end
end

# ─── TestLocalDiskStoreLayout ─────────────────────────────────────────────────

class TestLocalDiskStoreLayout < Minitest::Test
  def setup
    @dir = tmpdir
    @store = CAS::LocalDiskStore.new(@dir)
  end

  def teardown
    FileUtils.rm_rf(@dir)
  end

  def test_creates_root_directory
    dir = File.join(@dir, "nested", "cas")
    CAS::LocalDiskStore.new(dir)
    assert Dir.exist?(dir), "root directory should be created"
  end

  def test_object_stored_in_2_char_subdir
    # After putting a blob, a 2-char fanout subdir and 38-char file should exist.
    @store.put(CodingAdventures::Sha1.sha1("layout-test"), "layout-test")
    hex = CAS.key_to_hex(CodingAdventures::Sha1.sha1("layout-test"))
    expected_dir  = File.join(@dir, hex[0, 2])
    expected_file = File.join(expected_dir, hex[2, 38])
    assert Dir.exist?(expected_dir),   "2-char fanout dir should exist"
    assert File.exist?(expected_file), "38-char object file should exist"
  end

  def test_2_char_dir_name_is_first_byte_hex
    data = "layout-dir-name"
    hash = CodingAdventures::Sha1.sha1(data)
    @store.put(hash, data)
    hex  = hash.unpack1("H*")
    dir  = hex[0, 2]
    file = hex[2..]
    assert Dir.exist?(File.join(@dir, dir)),       "first 2 hex chars as dir"
    assert File.exist?(File.join(@dir, dir, file)), "remaining 38 as filename"
  end

  def test_file_contains_raw_bytes
    data = "raw bytes test"
    hash = CodingAdventures::Sha1.sha1(data)
    @store.put(hash, data)
    hex      = hash.unpack1("H*")
    raw_path = File.join(@dir, hex[0, 2], hex[2..])
    assert_equal data.b, File.binread(raw_path)
  end
end

# ─── TestLocalDiskStorePut ────────────────────────────────────────────────────

class TestLocalDiskStorePut < Minitest::Test
  def setup
    @dir   = tmpdir
    @store = CAS::LocalDiskStore.new(@dir)
  end

  def teardown
    FileUtils.rm_rf(@dir)
  end

  def test_put_empty_blob
    hash = CodingAdventures::Sha1.sha1("")
    @store.put(hash, "")
    assert @store.exists?(hash)
  end

  def test_put_small_blob
    data = "hello, CAS"
    hash = CodingAdventures::Sha1.sha1(data)
    @store.put(hash, data)
    assert @store.exists?(hash)
  end

  def test_put_large_blob_1mb
    # 1 MiB of pseudo-random-looking data (repeating pattern for speed)
    data = ("abc" * 350_000)[0, 1_048_576]
    hash = CodingAdventures::Sha1.sha1(data)
    @store.put(hash, data)
    assert @store.exists?(hash), "1 MiB blob should be stored"
    assert_equal data.b, @store.get(hash)
  end

  def test_put_is_idempotent
    data = "idempotent test"
    hash = CodingAdventures::Sha1.sha1(data)
    # First put stores the object.
    @store.put(hash, data)
    # Second put must not raise and must leave the stored bytes unchanged.
    @store.put(hash, data)
    assert_equal data.b, @store.get(hash)
  end

  def test_put_no_temp_files_left_behind
    data = "no temp files"
    hash = CodingAdventures::Sha1.sha1(data)
    @store.put(hash, data)

    hex = hash.unpack1("H*")
    bucket = File.join(@dir, hex[0, 2])
    tmp_files = Dir.glob(File.join(bucket, "*.tmp"))
    assert_empty tmp_files, "no .tmp files should remain after put"
  end
end

# ─── TestLocalDiskStoreGet ────────────────────────────────────────────────────

class TestLocalDiskStoreGet < Minitest::Test
  def setup
    @dir   = tmpdir
    @store = CAS::LocalDiskStore.new(@dir)
  end

  def teardown
    FileUtils.rm_rf(@dir)
  end

  def test_get_returns_stored_bytes
    data = "round trip get"
    hash = CodingAdventures::Sha1.sha1(data)
    @store.put(hash, data)
    assert_equal data.b, @store.get(hash)
  end

  def test_get_returns_binary_encoding
    data = "encoding check"
    hash = CodingAdventures::Sha1.sha1(data)
    @store.put(hash, data)
    result = @store.get(hash)
    assert_equal Encoding::BINARY, result.encoding
  end

  def test_get_unknown_key_raises_enoent
    fake_key = "\x00" * 20
    assert_raises(Errno::ENOENT) { @store.get(fake_key) }
  end
end

# ─── TestLocalDiskStoreExists ─────────────────────────────────────────────────

class TestLocalDiskStoreExists < Minitest::Test
  def setup
    @dir   = tmpdir
    @store = CAS::LocalDiskStore.new(@dir)
  end

  def teardown
    FileUtils.rm_rf(@dir)
  end

  def test_exists_false_before_put
    key = CodingAdventures::Sha1.sha1("not stored yet")
    refute @store.exists?(key)
  end

  def test_exists_true_after_put
    data = "exists after put"
    key  = CodingAdventures::Sha1.sha1(data)
    @store.put(key, data)
    assert @store.exists?(key)
  end

  def test_exists_does_not_change_other_keys
    key_a = CodingAdventures::Sha1.sha1("key-a")
    key_b = CodingAdventures::Sha1.sha1("key-b")
    @store.put(key_a, "key-a")
    refute @store.exists?(key_b), "unrelated key should still not exist"
  end
end

# ─── TestLocalDiskStorePrefix ─────────────────────────────────────────────────

class TestLocalDiskStorePrefix < Minitest::Test
  def setup
    @dir   = tmpdir
    @store = CAS::LocalDiskStore.new(@dir)
  end

  def teardown
    FileUtils.rm_rf(@dir)
  end

  def test_prefix_returns_matching_key
    data = "prefix match"
    key  = CodingAdventures::Sha1.sha1(data)
    @store.put(key, data)

    hex    = key.unpack1("H*")
    prefix = [hex[0, 6]].pack("H*")  # first 3 bytes as prefix
    result = @store.keys_with_prefix(prefix)
    assert_includes result, key
  end

  def test_prefix_empty_returns_empty_array
    result = @store.keys_with_prefix("")
    assert_equal [], result
  end

  def test_prefix_no_match_returns_empty_array
    # A key with all \x00 bytes would be under bucket "00/".
    # With an empty store, any prefix returns empty.
    result = @store.keys_with_prefix("\xfe\xed")
    assert_equal [], result
  end

  def test_prefix_all_matched_keys_returned
    # Store two objects that happen to be in the same bucket (same first byte).
    # This is non-deterministic with SHA-1, so we manufacture keys directly.
    key1 = "\x42\x01" + "\x00" * 18
    key2 = "\x42\x02" + "\x00" * 18

    # Write the raw files to simulate stored objects.
    hex1 = key1.unpack1("H*")
    hex2 = key2.unpack1("H*")
    bucket = File.join(@dir, "42")
    FileUtils.mkdir_p(bucket)
    File.binwrite(File.join(bucket, hex1[2..]), "data1")
    File.binwrite(File.join(bucket, hex2[2..]), "data2")

    prefix = "\x42"
    result = @store.keys_with_prefix(prefix)
    assert_includes result, key1
    assert_includes result, key2
  end
end

# ─── TestContentAddressableStore ──────────────────────────────────────────────

class TestContentAddressableStore < Minitest::Test
  def setup
    @dir   = tmpdir
    store  = CAS::LocalDiskStore.new(@dir)
    @cas   = CAS::ContentAddressableStore.new(store)
  end

  def teardown
    FileUtils.rm_rf(@dir)
  end

  # ── Round-trip tests ──────────────────────────────────────────────────────

  def test_roundtrip_empty_blob
    # An empty blob is a valid CAS object. Git stores empty files this way.
    key  = @cas.put("")
    data = @cas.get(key)
    assert_equal "".b, data
  end

  def test_roundtrip_small_blob
    key  = @cas.put("hello, world")
    data = @cas.get(key)
    assert_equal "hello, world".b, data
  end

  def test_roundtrip_1mb_blob
    # Verify that large blobs survive the round-trip intact.
    big = "x" * 1_048_576
    key  = @cas.put(big)
    data = @cas.get(key)
    assert_equal big.b, data
  end

  def test_put_returns_sha1_key
    data = "sha1 verification"
    key  = @cas.put(data)
    # The key must equal SHA-1(data).
    assert_equal CodingAdventures::Sha1.sha1(data.b), key
    assert_equal 20, key.bytesize
  end

  def test_put_is_idempotent
    key1 = @cas.put("idempotent")
    key2 = @cas.put("idempotent")
    assert_equal key1, key2
    # Only one file should exist (idempotent store skips second write)
    hex    = key1.unpack1("H*")
    bucket = File.join(@dir, hex[0, 2])
    files  = Dir.glob(File.join(bucket, hex[2..]))
    assert_equal 1, files.length
  end

  # ── Not found ─────────────────────────────────────────────────────────────

  def test_get_unknown_key_raises_not_found
    fake_key = "\x00" * 20
    err = assert_raises(CAS::CasNotFoundError) { @cas.get(fake_key) }
    assert_equal fake_key, err.key
  end

  # ── Corruption detection ──────────────────────────────────────────────────

  def test_get_detects_corruption
    # 1. Store a valid object.
    key = @cas.put("original content")

    # 2. Corrupt the raw file on disk by overwriting it with garbage.
    hex      = key.unpack1("H*")
    obj_path = File.join(@dir, hex[0, 2], hex[2..])
    File.binwrite(obj_path, "CORRUPTED BYTES THAT DO NOT HASH TO THE KEY")

    # 3. get should detect the mismatch and raise CasCorruptedError.
    err = assert_raises(CAS::CasCorruptedError) { @cas.get(key) }
    assert_equal key, err.key
  end

  # ── exists? ───────────────────────────────────────────────────────────────

  def test_exists_false_before_put
    key = CodingAdventures::Sha1.sha1("not yet stored")
    refute @cas.exists?(key)
  end

  def test_exists_true_after_put
    key = @cas.put("now stored")
    assert @cas.exists?(key)
  end

  # ── find_by_prefix ────────────────────────────────────────────────────────

  def test_find_by_prefix_unique_match
    key = @cas.put("unique prefix match")
    hex = key.unpack1("H*")
    # Use the first 7 chars (like git log --oneline)
    found = @cas.find_by_prefix(hex[0, 7])
    assert_equal key, found
  end

  def test_find_by_prefix_full_hex
    key   = @cas.put("full hex lookup")
    hex   = key.unpack1("H*")
    found = @cas.find_by_prefix(hex)
    assert_equal key, found
  end

  def test_find_by_prefix_ambiguous_raises
    # Store two blobs whose SHA-1 hashes share the same first byte.
    # We manufacture the situation by writing raw files into the store.
    key1 = "\x7f\x01" + "\x00" * 18
    key2 = "\x7f\x02" + "\x00" * 18
    hex1 = key1.unpack1("H*")
    hex2 = key2.unpack1("H*")

    bucket = File.join(@dir, "7f")
    FileUtils.mkdir_p(bucket)
    File.binwrite(File.join(bucket, hex1[2..]), "data1")
    File.binwrite(File.join(bucket, hex2[2..]), "data2")

    # A prefix that matches both keys ("7f") should raise ambiguous.
    err = assert_raises(CAS::CasAmbiguousPrefixError) do
      @cas.find_by_prefix("7f")
    end
    assert_equal "7f", err.prefix
  end

  def test_find_by_prefix_not_found_raises
    err = assert_raises(CAS::CasPrefixNotFoundError) do
      @cas.find_by_prefix("deadbeef00")
    end
    assert_includes err.message, "deadbeef00"
  end

  def test_find_by_prefix_invalid_hex_raises
    err = assert_raises(CAS::CasInvalidPrefixError) do
      @cas.find_by_prefix("xyz!")
    end
    assert_equal "xyz!", err.prefix
  end

  def test_find_by_prefix_empty_string_raises
    err = assert_raises(CAS::CasInvalidPrefixError) do
      @cas.find_by_prefix("")
    end
    assert_equal "", err.prefix
  end

  def test_find_by_prefix_odd_length
    # "a3f" is an odd-length prefix — should be handled (padded to "a3f0").
    # Since no real object starts with 0xa3 0xf0 in our empty store,
    # we expect PrefixNotFoundError, not InvalidPrefixError.
    assert_raises(CAS::CasPrefixNotFoundError) do
      @cas.find_by_prefix("a3f")
    end
  end

  # ── inner ─────────────────────────────────────────────────────────────────

  def test_inner_returns_wrapped_store
    inner = @cas.inner
    assert_kind_of CAS::LocalDiskStore, inner
  end

  # ── multi-object ──────────────────────────────────────────────────────────

  def test_different_content_produces_different_keys
    key1 = @cas.put("content-alpha")
    key2 = @cas.put("content-beta")
    refute_equal key1, key2
  end

  def test_binary_content_round_trips
    # Arbitrary binary data (all 256 byte values) must survive intact.
    binary = (0..255).map(&:chr).join.b
    key  = @cas.put(binary)
    data = @cas.get(key)
    assert_equal binary, data
  end
end
