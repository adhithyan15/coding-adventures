# frozen_string_literal: true

# Tests for the SHA-512 implementation.
#
# Test vectors come from FIPS 180-4 (the official SHA-2 standard). Any correct
# SHA-512 implementation must produce exactly these digests for these inputs.

require "minitest/autorun"
require "coding_adventures_sha512"

SHA512 = CodingAdventures::Sha512

class TestVersion < Minitest::Test
  def test_version_exists
    refute_nil SHA512::VERSION
    assert_equal "0.1.0", SHA512::VERSION
  end
end

# ---- FIPS 180-4 Test Vectors ----

class TestFIPSVectors < Minitest::Test
  def test_empty_string
    assert_equal(
      "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce" \
      "47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e",
      SHA512.sha512_hex("")
    )
  end

  def test_abc
    assert_equal(
      "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a" \
      "2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
      SHA512.sha512_hex("abc")
    )
  end

  def test_896_bit_message
    msg = "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmno" \
          "ijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"
    assert_equal 112, msg.bytesize
    assert_equal(
      "8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018" \
      "501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909",
      SHA512.sha512_hex(msg)
    )
  end

  def test_million_a
    data = "a" * 1_000_000
    assert_equal(
      "e718483d0ce769644e2e42c7bc15b4638e1f98b13b2044285632a803afa973eb" \
      "de0ff244877ea60a4cb0432ce577c31beb009c5c2c49aa2e4eadb217ad8cc09b",
      SHA512.sha512_hex(data)
    )
  end
end

# ---- Return Type and Format ----

class TestReturnType < Minitest::Test
  def test_returns_binary_string
    result = SHA512.sha512("test")
    assert_equal Encoding::BINARY, result.encoding
  end

  def test_digest_length_is_64
    assert_equal 64, SHA512.sha512("").bytesize
    assert_equal 64, SHA512.sha512("hello world").bytesize
    assert_equal 64, SHA512.sha512("x" * 1000).bytesize
  end

  def test_hex_length_is_128
    assert_equal 128, SHA512.sha512_hex("").length
    assert_equal 128, SHA512.sha512_hex("hello").length
  end

  def test_hex_is_lowercase
    result = SHA512.sha512_hex("abc")
    assert_equal result.downcase, result
    assert_match(/\A[0-9a-f]+\z/, result)
  end

  def test_deterministic
    assert_equal SHA512.sha512("hello"), SHA512.sha512("hello")
  end

  def test_avalanche_effect
    h1 = SHA512.sha512("hello")
    h2 = SHA512.sha512("helo")
    refute_equal h1, h2
    xor_bytes = h1.bytes.zip(h2.bytes).map { |a, b| a ^ b }
    bits_different = xor_bytes.sum { |b| b.to_s(2).count("1") }
    assert_operator bits_different, :>, 100
  end
end

# ---- Block Boundary Tests ----
# SHA-512 processes 128-byte blocks.

class TestBlockBoundaries < Minitest::Test
  def test_111_bytes
    result = SHA512.sha512("\x00" * 111)
    assert_equal 64, result.bytesize
    assert_equal result, SHA512.sha512("\x00" * 111)
  end

  def test_112_bytes
    assert_equal 64, SHA512.sha512("\x00" * 112).bytesize
  end

  def test_111_and_112_differ
    refute_equal SHA512.sha512("\x00" * 111), SHA512.sha512("\x00" * 112)
  end

  def test_128_bytes
    assert_equal 64, SHA512.sha512("\x00" * 128).bytesize
  end

  def test_255_bytes
    assert_equal 64, SHA512.sha512("\x00" * 255).bytesize
  end

  def test_256_bytes
    assert_equal 64, SHA512.sha512("\x00" * 256).bytesize
  end

  def test_all_boundary_sizes_distinct
    sizes = [111, 112, 127, 128, 255, 256]
    digests = sizes.map { |n| SHA512.sha512_hex("\x00" * n) }
    assert_equal 6, digests.uniq.length
  end
end

# ---- Edge Cases ----

class TestEdgeCases < Minitest::Test
  def test_single_null_byte
    result = SHA512.sha512("\x00")
    assert_equal 64, result.bytesize
    refute_equal result, SHA512.sha512("")
  end

  def test_single_ff_byte
    assert_equal 64, SHA512.sha512("\xFF".b).bytesize
  end

  def test_all_byte_values
    data = (0..255).map(&:chr).join.b
    assert_equal 64, SHA512.sha512(data).bytesize
  end

  def test_every_single_byte_unique
    digests = (0..255).map { |i| SHA512.sha512_hex(i.chr.b) }
    assert_equal 256, digests.uniq.length
  end

  def test_utf8_text
    text = "Hello, \u4e16\u754c!".encode("UTF-8")
    assert_equal 64, SHA512.sha512(text).bytesize
  end

  def test_binary_zeros
    assert_equal 64, SHA512.sha512("\x00" * 1000).bytesize
  end

  def test_sha512_hex_matches_unpack
    data = "hello"
    assert_equal SHA512.sha512(data).unpack1("H*"), SHA512.sha512_hex(data)
  end
end

# ---- Streaming API ----

class TestStreaming < Minitest::Test
  def test_single_update_matches_oneshot
    h = SHA512::Digest.new
    h.update("abc")
    assert_equal SHA512.sha512_hex("abc"), h.hexdigest
  end

  def test_two_updates_split_at_byte
    h = SHA512::Digest.new
    h.update("ab")
    h.update("c")
    assert_equal SHA512.sha512_hex("abc"), h.hexdigest
  end

  def test_split_at_block_boundary
    data = "\x00" * 256
    h = SHA512::Digest.new
    h.update(data[0, 128])
    h.update(data[128..])
    assert_equal SHA512.sha512(data), h.digest
  end

  def test_byte_at_a_time
    data = (0...200).map(&:chr).join.b
    h = SHA512::Digest.new
    data.each_byte { |b| h.update(b.chr.b) }
    assert_equal SHA512.sha512(data), h.digest
  end

  def test_empty_input
    h = SHA512::Digest.new
    assert_equal SHA512.sha512(""), h.digest
  end

  def test_digest_is_nondestructive
    h = SHA512::Digest.new
    h.update("abc")
    d1 = h.digest
    d2 = h.digest
    assert_equal d1, d2
  end

  def test_update_after_digest
    h = SHA512::Digest.new
    h.update("ab")
    h.digest # snapshot -- must not mutate state
    h.update("c")
    assert_equal SHA512.sha512("abc"), h.digest
  end

  def test_hexdigest_fips_vector
    h = SHA512::Digest.new
    h.update("abc")
    expected = "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a" \
               "2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
    assert_equal expected, h.hexdigest
  end

  def test_shovel_operator_alias
    h = SHA512::Digest.new
    h << "ab"
    h << "c"
    assert_equal SHA512.sha512_hex("abc"), h.hexdigest
  end

  def test_copy_is_independent
    h = SHA512::Digest.new
    h.update("ab")
    h2 = h.copy
    h2.update("c")
    h.update("x")
    assert_equal SHA512.sha512("abc"), h2.digest
    assert_equal SHA512.sha512("abx"), h.digest
  end

  def test_copy_same_result
    h = SHA512::Digest.new
    h.update("abc")
    h2 = h.copy
    assert_equal h.digest, h2.digest
  end

  def test_streaming_million_a
    data = "a" * 1_000_000
    h = SHA512::Digest.new
    h.update(data[0, 500_000])
    h.update(data[500_000..])
    assert_equal SHA512.sha512(data), h.digest
  end
end
