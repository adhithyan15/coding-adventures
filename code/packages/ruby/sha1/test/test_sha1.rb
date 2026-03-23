# frozen_string_literal: true

# Tests for the SHA-1 implementation.
#
# Test vectors come from FIPS 180-4 (the official SHA-1 standard). Any correct
# SHA-1 implementation must produce exactly these digests for these inputs.
#
# We also test the streaming API (CodingAdventures::Sha1::Digest) to verify it
# produces the same results as the one-shot sha1() function, and test edge cases
# like empty input, exact block boundaries, and very long inputs.

require "minitest/autorun"
require "coding_adventures_sha1"

SHA1 = CodingAdventures::Sha1

class TestVersion < Minitest::Test
  def test_version_exists
    refute_nil SHA1::VERSION
    assert_equal "0.1.0", SHA1::VERSION
  end
end

# ─── FIPS 180-4 Test Vectors ─────────────────────────────────────────────────

class TestFIPSVectors < Minitest::Test
  def test_empty_string
    assert_equal "da39a3ee5e6b4b0d3255bfef95601890afd80709", SHA1.sha1_hex("")
  end

  def test_abc
    assert_equal "a9993e364706816aba3e25717850c26c9cd0d89d", SHA1.sha1_hex("abc")
  end

  def test_448_bit_message
    msg = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
    assert_equal 56, msg.bytesize
    assert_equal "84983e441c3bd26ebaae4aa1f95129e5e54670f1", SHA1.sha1_hex(msg)
  end

  def test_million_a
    data = "a" * 1_000_000
    assert_equal "34aa973cd4c4daa4f61eeb2bdbad27316534016f", SHA1.sha1_hex(data)
  end
end

# ─── Return Type and Format ───────────────────────────────────────────────────

class TestReturnType < Minitest::Test
  def test_returns_binary_string
    result = SHA1.sha1("test")
    assert_equal Encoding::BINARY, result.encoding
  end

  def test_digest_length_is_20
    assert_equal 20, SHA1.sha1("").bytesize
    assert_equal 20, SHA1.sha1("hello world").bytesize
    assert_equal 20, SHA1.sha1("x" * 1000).bytesize
  end

  def test_hex_length_is_40
    assert_equal 40, SHA1.sha1_hex("").length
    assert_equal 40, SHA1.sha1_hex("hello").length
  end

  def test_hex_is_lowercase
    result = SHA1.sha1_hex("abc")
    assert_equal result.downcase, result
    assert_match(/\A[0-9a-f]+\z/, result)
  end

  def test_deterministic
    assert_equal SHA1.sha1("hello"), SHA1.sha1("hello")
  end

  def test_avalanche_effect
    h1 = SHA1.sha1("hello")
    h2 = SHA1.sha1("helo") # one character different
    refute_equal h1, h2
    # XOR the bytes — at least some should differ significantly
    xor_bytes = h1.bytes.zip(h2.bytes).map { |a, b| a ^ b }
    bits_different = xor_bytes.sum { |b| b.to_s(2).count("1") }
    assert_operator bits_different, :>, 20
  end
end

# ─── Block Boundary Tests ─────────────────────────────────────────────────────
#
# SHA-1 processes 64-byte blocks. Block boundaries are the most common source
# of bugs because padding behaves differently near them:
#
#   55 bytes: fits in one block (55 + 1 + 8 = 64)
#   56 bytes: overflows into a second block
#   64 bytes: one data block + a full padding block
#   128 bytes: two data blocks + one full padding block

class TestBlockBoundaries < Minitest::Test
  def test_55_bytes
    result = SHA1.sha1("\x00" * 55)
    assert_equal 20, result.bytesize
    assert_equal result, SHA1.sha1("\x00" * 55) # deterministic
  end

  def test_56_bytes
    assert_equal 20, SHA1.sha1("\x00" * 56).bytesize
  end

  def test_55_and_56_differ
    refute_equal SHA1.sha1("\x00" * 55), SHA1.sha1("\x00" * 56)
  end

  def test_64_bytes
    assert_equal 20, SHA1.sha1("\x00" * 64).bytesize
  end

  def test_127_bytes
    assert_equal 20, SHA1.sha1("\x00" * 127).bytesize
  end

  def test_128_bytes
    assert_equal 20, SHA1.sha1("\x00" * 128).bytesize
  end

  def test_all_boundary_sizes_distinct
    sizes = [55, 56, 63, 64, 127, 128]
    digests = sizes.map { |n| SHA1.sha1_hex("\x00" * n) }
    assert_equal 6, digests.uniq.length
  end
end

# ─── Edge Cases ───────────────────────────────────────────────────────────────

class TestEdgeCases < Minitest::Test
  def test_single_null_byte
    result = SHA1.sha1("\x00")
    assert_equal 20, result.bytesize
    refute_equal result, SHA1.sha1("") # null byte ≠ empty string
  end

  def test_single_ff_byte
    assert_equal 20, SHA1.sha1("\xFF".b).bytesize
  end

  def test_all_byte_values
    data = (0..255).map(&:chr).join.b
    assert_equal 20, SHA1.sha1(data).bytesize
  end

  def test_every_single_byte_unique
    digests = (0..255).map { |i| SHA1.sha1_hex(i.chr.b) }
    assert_equal 256, digests.uniq.length
  end

  def test_utf8_text
    text = "Hello, 世界!".encode("UTF-8")
    assert_equal 20, SHA1.sha1(text).bytesize
  end

  def test_binary_zeros
    assert_equal 20, SHA1.sha1("\x00" * 1000).bytesize
  end

  def test_sha1_hex_matches_unpack
    data = "hello"
    assert_equal SHA1.sha1(data).unpack1("H*"), SHA1.sha1_hex(data)
  end
end

# ─── Streaming API ────────────────────────────────────────────────────────────

class TestStreaming < Minitest::Test
  def test_single_update_matches_oneshot
    h = SHA1::Digest.new
    h.update("abc")
    assert_equal SHA1.sha1_hex("abc"), h.hexdigest
  end

  def test_two_updates_split_at_byte
    h = SHA1::Digest.new
    h.update("ab")
    h.update("c")
    assert_equal SHA1.sha1_hex("abc"), h.hexdigest
  end

  def test_split_at_block_boundary
    data = "\x00" * 128
    h = SHA1::Digest.new
    h.update(data[0, 64])
    h.update(data[64..])
    assert_equal SHA1.sha1(data), h.digest
  end

  def test_byte_at_a_time
    data = (0...100).map(&:chr).join.b
    h = SHA1::Digest.new
    data.each_byte { |b| h.update(b.chr.b) }
    assert_equal SHA1.sha1(data), h.digest
  end

  def test_empty_input
    h = SHA1::Digest.new
    assert_equal SHA1.sha1(""), h.digest
  end

  def test_digest_is_nondestructive
    h = SHA1::Digest.new
    h.update("abc")
    d1 = h.digest
    d2 = h.digest
    assert_equal d1, d2
  end

  def test_update_after_digest
    h = SHA1::Digest.new
    h.update("ab")
    h.digest # snapshot — must not mutate state
    h.update("c")
    assert_equal SHA1.sha1("abc"), h.digest
  end

  def test_hexdigest_fips_vector
    h = SHA1::Digest.new
    h.update("abc")
    assert_equal "a9993e364706816aba3e25717850c26c9cd0d89d", h.hexdigest
  end

  def test_shovel_operator_alias
    h = SHA1::Digest.new
    h << "ab"
    h << "c"
    assert_equal SHA1.sha1_hex("abc"), h.hexdigest
  end

  def test_copy_is_independent
    h = SHA1::Digest.new
    h.update("ab")
    h2 = h.copy
    h2.update("c")
    h.update("x") # different suffix on original
    assert_equal SHA1.sha1("abc"), h2.digest
    assert_equal SHA1.sha1("abx"), h.digest
  end

  def test_copy_same_result
    h = SHA1::Digest.new
    h.update("abc")
    h2 = h.copy
    assert_equal h.digest, h2.digest
  end

  def test_streaming_million_a
    data = "a" * 1_000_000
    h = SHA1::Digest.new
    h.update(data[0, 500_000])
    h.update(data[500_000..])
    assert_equal SHA1.sha1(data), h.digest
  end
end
