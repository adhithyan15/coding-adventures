# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_sha256"

# Tests for the SHA-256 implementation.
#
# Test vectors come from FIPS 180-4 (the official SHA-2 standard). We also test
# the streaming API, block boundaries, edge cases, and the copy/branching API.

class TestFIPSVectors < Minitest::Test
  def test_empty_string
    assert_equal(
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      CodingAdventures::Sha256.sha256_hex("")
    )
  end

  def test_abc
    assert_equal(
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
      CodingAdventures::Sha256.sha256_hex("abc")
    )
  end

  def test_448_bit_message
    msg = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
    assert_equal 56, msg.bytesize
    assert_equal(
      "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
      CodingAdventures::Sha256.sha256_hex(msg)
    )
  end

  def test_million_a
    assert_equal(
      "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
      CodingAdventures::Sha256.sha256_hex("a" * 1_000_000)
    )
  end
end

class TestReturnType < Minitest::Test
  def test_returns_binary_string
    result = CodingAdventures::Sha256.sha256("test")
    assert_instance_of String, result
    assert_equal Encoding::ASCII_8BIT, result.encoding
  end

  def test_length_is_32
    assert_equal 32, CodingAdventures::Sha256.sha256("").bytesize
    assert_equal 32, CodingAdventures::Sha256.sha256("hello world").bytesize
    assert_equal 32, CodingAdventures::Sha256.sha256("x" * 1000).bytesize
  end

  def test_deterministic
    assert_equal(
      CodingAdventures::Sha256.sha256("hello"),
      CodingAdventures::Sha256.sha256("hello")
    )
  end

  def test_avalanche_effect
    h1 = CodingAdventures::Sha256.sha256("hello")
    h2 = CodingAdventures::Sha256.sha256("helo")
    refute_equal h1, h2
    # Count differing bits
    xor_bytes = h1.bytes.zip(h2.bytes).map { |a, b| a ^ b }
    bits_different = xor_bytes.sum { |b| b.to_s(2).count("1") }
    assert bits_different > 50, "Only #{bits_different} bits differ (expected ~128 of 256)"
  end
end

class TestBlockBoundaries < Minitest::Test
  def test_55_bytes
    result = CodingAdventures::Sha256.sha256("x" * 55)
    assert_equal 32, result.bytesize
    assert_equal result, CodingAdventures::Sha256.sha256("x" * 55)
  end

  def test_56_bytes
    assert_equal 32, CodingAdventures::Sha256.sha256("x" * 56).bytesize
  end

  def test_63_bytes
    assert_equal 32, CodingAdventures::Sha256.sha256("x" * 63).bytesize
  end

  def test_64_bytes
    assert_equal 32, CodingAdventures::Sha256.sha256("x" * 64).bytesize
  end

  def test_119_bytes
    assert_equal 32, CodingAdventures::Sha256.sha256("x" * 119).bytesize
  end

  def test_120_bytes
    assert_equal 32, CodingAdventures::Sha256.sha256("x" * 120).bytesize
  end

  def test_127_bytes
    assert_equal 32, CodingAdventures::Sha256.sha256("x" * 127).bytesize
  end

  def test_128_bytes
    assert_equal 32, CodingAdventures::Sha256.sha256("x" * 128).bytesize
  end

  def test_boundary_lengths_all_distinct
    lengths = [55, 56, 63, 64, 119, 120, 127, 128]
    digests = lengths.map { |n| CodingAdventures::Sha256.sha256("x" * n) }
    assert_equal lengths.size, digests.uniq.size, "Not all boundary digests are unique"
  end
end

class TestEdgeCases < Minitest::Test
  def test_single_zero_byte
    result = CodingAdventures::Sha256.sha256("\x00")
    assert_equal 32, result.bytesize
    refute_equal CodingAdventures::Sha256.sha256(""), result
  end

  def test_single_ff_byte
    assert_equal 32, CodingAdventures::Sha256.sha256("\xFF").bytesize
  end

  def test_all_byte_values
    data = (0..255).map(&:chr).join.b
    assert_equal 32, CodingAdventures::Sha256.sha256(data).bytesize
  end

  def test_binary_zeros
    assert_equal 32, CodingAdventures::Sha256.sha256("\x00" * 1000).bytesize
  end

  def test_different_single_bytes
    digests = (0..255).map { |i| CodingAdventures::Sha256.sha256(i.chr.b) }
    assert_equal 256, digests.uniq.size, "Not all single-byte digests are unique"
  end
end

class TestSha256Hex < Minitest::Test
  def test_returns_string
    assert_instance_of String, CodingAdventures::Sha256.sha256_hex("")
  end

  def test_length_is_64
    assert_equal 64, CodingAdventures::Sha256.sha256_hex("").length
    assert_equal 64, CodingAdventures::Sha256.sha256_hex("hello").length
  end

  def test_lowercase
    result = CodingAdventures::Sha256.sha256_hex("abc")
    assert_equal result, result.downcase
  end

  def test_matches_digest_hex
    ["", "abc", "hello world"].each do |msg|
      assert_equal(
        CodingAdventures::Sha256.sha256(msg).unpack1("H*"),
        CodingAdventures::Sha256.sha256_hex(msg)
      )
    end
  end

  def test_fips_vector
    assert_equal(
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
      CodingAdventures::Sha256.sha256_hex("abc")
    )
  end
end

class TestStreaming < Minitest::Test
  def test_single_update_equals_oneshot
    h = CodingAdventures::Sha256::Digest.new
    h.update("abc")
    assert_equal CodingAdventures::Sha256.sha256("abc"), h.digest
  end

  def test_split_at_byte_boundary
    h = CodingAdventures::Sha256::Digest.new
    h.update("ab")
    h.update("c")
    assert_equal CodingAdventures::Sha256.sha256("abc"), h.digest
  end

  def test_split_at_block_boundary
    data = "x" * 128
    h = CodingAdventures::Sha256::Digest.new
    h.update(data[0, 64])
    h.update(data[64..])
    assert_equal CodingAdventures::Sha256.sha256(data), h.digest
  end

  def test_many_tiny_updates
    data = (0...100).map(&:chr).join.b
    h = CodingAdventures::Sha256::Digest.new
    data.each_byte { |b| h.update(b.chr.b) }
    assert_equal CodingAdventures::Sha256.sha256(data), h.digest
  end

  def test_empty_input
    h = CodingAdventures::Sha256::Digest.new
    assert_equal CodingAdventures::Sha256.sha256(""), h.digest
  end

  def test_digest_is_nondestructive
    h = CodingAdventures::Sha256::Digest.new
    h.update("abc")
    d1 = h.digest
    d2 = h.digest
    assert_equal d1, d2
  end

  def test_update_after_digest
    h = CodingAdventures::Sha256::Digest.new
    h.update("ab")
    h.digest # snapshot
    h.update("c")
    assert_equal CodingAdventures::Sha256.sha256("abc"), h.digest
  end

  def test_hexdigest
    h = CodingAdventures::Sha256::Digest.new
    h.update("abc")
    assert_equal(
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
      h.hexdigest
    )
  end

  def test_chaining
    h = CodingAdventures::Sha256::Digest.new
    result = h.update("a").update("b").update("c").hexdigest
    assert_equal(
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
      result
    )
  end

  def test_copy_is_independent
    h = CodingAdventures::Sha256::Digest.new
    h.update("ab")
    h2 = h.copy
    h2.update("c")
    h.update("x")
    assert_equal CodingAdventures::Sha256.sha256("abc"), h2.digest
    assert_equal CodingAdventures::Sha256.sha256("abx"), h.digest
  end

  def test_copy_same_result
    h = CodingAdventures::Sha256::Digest.new
    h.update("abc")
    h2 = h.copy
    assert_equal h.digest, h2.digest
  end

  def test_fips_vector_streaming
    h = CodingAdventures::Sha256::Digest.new
    h.update("a" * 500_000)
    h.update("a" * 500_000)
    assert_equal CodingAdventures::Sha256.sha256("a" * 1_000_000), h.digest
  end

  def test_streaming_various_chunk_sizes
    data = "a" * 200
    expected = CodingAdventures::Sha256.sha256(data)
    [1, 7, 13, 32, 63, 64, 65, 100, 200].each do |chunk_size|
      h = CodingAdventures::Sha256::Digest.new
      (0...data.bytesize).step(chunk_size) do |i|
        h.update(data[i, chunk_size])
      end
      assert_equal expected, h.digest, "Failed with chunk_size=#{chunk_size}"
    end
  end
end
