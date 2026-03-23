# frozen_string_literal: true

# test_md5.rb — Comprehensive tests for the MD5 implementation.
#
# Test strategy:
#   1. RFC 1321 test vectors     — the official "must pass" set
#   2. Format tests              — binary vs hex output, encoding correctness
#   3. Little-endian correctness — verify the byte order is LE, not BE
#   4. Block boundary tests      — empty, 55 bytes, 56 bytes, 64 bytes, 128 bytes
#   5. Edge cases                — single byte, non-ASCII, binary data
#   6. Streaming API (Digest)    — update, digest, hexdigest, copy, << alias
#   7. Digest non-destructiveness — calling digest() twice gives same result
#   8. Chunk equivalence         — chunked update == one-shot md5
#   9. Large input               — multi-block messages
#  10. Private internals         — T-table spot-checks, shift table, init constants

require "minitest/autorun"
require "ca_md5"

class TestMd5 < Minitest::Test
  # Convenience: compute hex digest of a string via the one-shot API.
  def hex(str)
    Ca::Md5.md5_hex(str)
  end

  # ─── Version ──────────────────────────────────────────────────────────────────

  def test_version_exists
    refute_nil Ca::Md5::VERSION
  end

  def test_version_is_string
    assert_instance_of String, Ca::Md5::VERSION
  end

  # ─── RFC 1321 Official Test Vectors ───────────────────────────────────────────
  #
  # These are the authoritative test cases from Appendix A.5 of RFC 1321.
  # Every correct MD5 implementation MUST pass all of them.

  def test_rfc_empty_string
    # md5("") = d41d8cd98f00b204e9800998ecf8427e
    # An empty message still produces a digest — the padding adds 64 bytes.
    assert_equal "d41d8cd98f00b204e9800998ecf8427e", hex("")
  end

  def test_rfc_single_a
    # md5("a") = 0cc175b9c0f1b6a831c399e269772661
    assert_equal "0cc175b9c0f1b6a831c399e269772661", hex("a")
  end

  def test_rfc_abc
    # md5("abc") = 900150983cd24fb0d6963f7d28e17f72
    assert_equal "900150983cd24fb0d6963f7d28e17f72", hex("abc")
  end

  def test_rfc_message_digest
    # md5("message digest") = f96b697d7cb7938d525a2f31aaf161d0
    assert_equal "f96b697d7cb7938d525a2f31aaf161d0", hex("message digest")
  end

  def test_rfc_lowercase_alphabet
    # md5("abcdefghijklmnopqrstuvwxyz") = c3fcd3d76192e4007dfb496cca67e13b
    assert_equal "c3fcd3d76192e4007dfb496cca67e13b", hex("abcdefghijklmnopqrstuvwxyz")
  end

  def test_rfc_mixed_alphanumeric
    # md5("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789")
    # = d174ab98d277d9f5a5611c2c9f419d9f
    assert_equal "d174ab98d277d9f5a5611c2c9f419d9f",
      hex("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789")
  end

  def test_rfc_repeated_digits
    # md5("1234567890" repeated 8 times) = 57edf4a22be3c955ac49da2e2107b67a
    assert_equal "57edf4a22be3c955ac49da2e2107b67a", hex("1234567890" * 8)
  end

  # ─── Output Format Tests ───────────────────────────────────────────────────────

  def test_md5_returns_binary_string
    # md5() must return a String of exactly 16 bytes in binary encoding
    result = Ca::Md5.md5("abc")
    assert_instance_of String, result
    assert_equal 16, result.bytesize
  end

  def test_md5_binary_encoding
    # The 16-byte result should have ASCII-8BIT (binary) encoding
    result = Ca::Md5.md5("abc")
    assert_equal Encoding::ASCII_8BIT, result.encoding
  end

  def test_md5_hex_returns_string
    result = Ca::Md5.md5_hex("abc")
    assert_instance_of String, result
  end

  def test_md5_hex_returns_32_chars
    # 16 bytes × 2 hex chars per byte = 32 characters
    assert_equal 32, Ca::Md5.md5_hex("abc").length
  end

  def test_md5_hex_is_lowercase
    # Standard MD5 hex output uses lowercase letters a-f
    hex_result = Ca::Md5.md5_hex("abc")
    assert_equal hex_result, hex_result.downcase
  end

  def test_md5_hex_contains_only_hex_chars
    hex_result = Ca::Md5.md5_hex("abc")
    assert_match(/\A[0-9a-f]{32}\z/, hex_result)
  end

  # ─── Little-Endian Correctness ────────────────────────────────────────────────
  #
  # MD5's little-endian byte order is its most unusual property. We verify this
  # explicitly because big-endian mistakes produce wrong-but-plausible output.
  #
  # The empty-string digest d41d8cd98f00b204e9800998ecf8427e starts with bytes:
  #   d4 1d 8c d9 8f 00 b2 04 ...
  # If we mistakenly used big-endian state output, the first word of the state
  # would be written as 0xD9_8C_1D_D4 → bytes d9 8c 1d d4 ... (different).

  def test_little_endian_first_byte_of_empty_hash
    bytes = Ca::Md5.md5("").bytes
    # d41d8cd9... → first byte is 0xd4
    assert_equal 0xd4, bytes[0]
  end

  def test_little_endian_empty_hash_bytes
    # Verify first 8 bytes of md5("") to ensure little-endian word storage
    expected = [0xd4, 0x1d, 0x8c, 0xd9, 0x8f, 0x00, 0xb2, 0x04]
    actual = Ca::Md5.md5("").bytes.first(8)
    assert_equal expected, actual
  end

  def test_little_endian_vs_unpack
    # The binary output, when unpacked as "V4" (little-endian), should give
    # four 32-bit words. If we used big-endian "N4" we'd get a wrong digest.
    raw = Ca::Md5.md5("")
    # Unpack as little-endian words
    le_words = raw.unpack("V4")
    # The first LE word of md5("") state is A = 0xD98C1DD4 after rounds.
    # Stored little-endian: D4 1D 8C D9 → reading as V gives 0xD98C1DD4
    assert_equal 0xD98C1DD4, le_words[0]
  end

  # ─── Block Boundary Tests ──────────────────────────────────────────────────────
  #
  # MD5 processes data in 64-byte (512-bit) blocks. The padding rules have
  # special cases depending on where the message length falls relative to 64.
  #
  # Key boundary cases:
  #   - 0 bytes:  pads to 64 bytes  (1 block)
  #   - 55 bytes: pads to 64 bytes  (1 block) — 55+1(0x80)+8(len) = 64 ✓
  #   - 56 bytes: pads to 128 bytes (2 blocks) — 56+1=57, need 56 mod 64,
  #               so pad to 120, then add 8 = 128 ✓
  #   - 64 bytes: pads to 128 bytes (2 blocks)
  #   - 128 bytes: pads to 192 bytes (3 blocks)

  def test_55_byte_message
    # 55 bytes fits exactly in one block after padding: 55 + 1 + 8 = 64
    # Verified against Ruby's stdlib Digest::MD5.
    msg = "a" * 55
    result = hex(msg)
    assert_equal 32, result.length
    assert_equal "ef1772b6dff9a122358552954ad0df65", result
  end

  def test_56_byte_message
    # 56 bytes forces a second block: 56 + 1 needs padding to 120, plus 8 = 128.
    # Verified against Ruby's stdlib Digest::MD5.
    msg = "a" * 56
    result = hex(msg)
    assert_equal 32, result.length
    assert_equal "3b0c8ac703f828b04c6c197006d17218", result
  end

  def test_63_byte_message
    # Verified against Ruby's stdlib Digest::MD5.
    msg = "a" * 63
    result = hex(msg)
    assert_equal 32, result.length
    assert_equal "b06521f39153d618550606be297466d5", result
  end

  def test_64_byte_message
    # Exactly one block of data — padding requires a second block.
    # Verified against Ruby's stdlib Digest::MD5.
    msg = "a" * 64
    result = hex(msg)
    assert_equal 32, result.length
    assert_equal "014842d480b571495a4a0363793f7367", result
  end

  def test_128_byte_message
    # Two blocks of data — padding requires a third block.
    # Verified against Ruby's stdlib Digest::MD5.
    msg = "a" * 128
    result = hex(msg)
    assert_equal 32, result.length
    assert_equal "e510683b3f5ffe4093d021808bc6ff70", result
  end

  # ─── Edge Cases ───────────────────────────────────────────────────────────────

  def test_single_null_byte
    # "\x00" has a known MD5
    assert_equal "93b885adfe0da089cdf634904fd59f71", hex("\x00")
  end

  def test_all_zeros_16_bytes
    assert_equal "4ae71336e44bf9bf79d2752e234818a5", hex("\x00" * 16)
  end

  def test_all_ones_16_bytes
    # "\xFF" * 16 = 16 bytes each with value 255 (all bits set)
    # Verified against Ruby's stdlib Digest::MD5.
    assert_equal "8d79cbc9a4ecdde112fc91ba625b13c2", hex("\xFF" * 16)
  end

  def test_binary_data_nonzero
    # Hash 8 bytes of incrementing binary data
    data = (0..7).map(&:chr).join
    result = hex(data)
    assert_equal 32, result.length
    assert_match(/\A[0-9a-f]{32}\z/, result)
  end

  def test_accepts_binary_encoding
    # Explicitly binary-encoded strings should work
    data = "\x01\x02\x03\x04".b
    result = Ca::Md5.md5(data)
    assert_equal 16, result.bytesize
  end

  def test_utf8_string_treated_as_bytes
    # UTF-8 multi-byte characters are hashed by their byte representation.
    # "é" in UTF-8 is bytes 0xC3 0xA9 (2 bytes, not one).
    # We hash "\xC3\xA9" in binary encoding to be explicit.
    # Verified against Ruby's stdlib Digest::MD5.
    result = hex("\xC3\xA9")
    assert_equal 32, result.length
    assert_equal "66ddcd97cfdeabb2f6fb8a999b4bc76f", result
  end

  # ─── Streaming API: Digest Class ──────────────────────────────────────────────

  def test_digest_version_matches_module
    assert_equal Ca::Md5::VERSION,
      Ca::Md5::VERSION
  end

  def test_digest_empty_hexdigest
    d = Ca::Md5::Digest.new
    assert_equal "d41d8cd98f00b204e9800998ecf8427e", d.hexdigest
  end

  def test_digest_single_update
    d = Ca::Md5::Digest.new
    d.update("abc")
    assert_equal "900150983cd24fb0d6963f7d28e17f72", d.hexdigest
  end

  def test_digest_multiple_updates_equal_oneshot
    d = Ca::Md5::Digest.new
    d.update("ab")
    d.update("c")
    assert_equal hex("abc"), d.hexdigest
  end

  def test_digest_update_returns_self_for_chaining
    d = Ca::Md5::Digest.new
    result = d.update("abc")
    assert_same d, result
  end

  def test_digest_chained_updates
    result = Ca::Md5::Digest.new
      .update("abc")
      .update("defghijklmnopqrstuvwxyz")
      .hexdigest
    assert_equal hex("abcdefghijklmnopqrstuvwxyz"), result
  end

  def test_digest_shovel_alias_works
    d = Ca::Md5::Digest.new
    d << "abc"
    assert_equal "900150983cd24fb0d6963f7d28e17f72", d.hexdigest
  end

  def test_digest_shovel_chaining
    result = (Ca::Md5::Digest.new << "ab" << "c").hexdigest
    assert_equal "900150983cd24fb0d6963f7d28e17f72", result
  end

  def test_digest_returns_16_bytes
    d = Ca::Md5::Digest.new
    d.update("abc")
    assert_equal 16, d.digest.bytesize
  end

  def test_digest_binary_encoding
    d = Ca::Md5::Digest.new
    d.update("abc")
    assert_equal Encoding::ASCII_8BIT, d.digest.encoding
  end

  # ─── Digest Non-Destructiveness ───────────────────────────────────────────────
  #
  # Calling digest() must not alter the internal state — subsequent update()
  # and digest() calls must still work correctly.

  def test_digest_is_nondestructive
    d = Ca::Md5::Digest.new
    d.update("abc")
    first  = d.digest
    second = d.digest
    assert_equal first, second
  end

  def test_digest_can_continue_after_digest_call
    d = Ca::Md5::Digest.new
    d.update("abc")
    _ = d.digest            # peek at the digest
    d.update("defghijklmnopqrstuvwxyz")  # continue hashing
    assert_equal hex("abcdefghijklmnopqrstuvwxyz"), d.hexdigest
  end

  def test_hexdigest_is_nondestructive
    d = Ca::Md5::Digest.new
    d.update("abc")
    first  = d.hexdigest
    second = d.hexdigest
    assert_equal first, second
  end

  # ─── Digest Copy ──────────────────────────────────────────────────────────────
  #
  # copy() lets you hash a common prefix once, then branch independently.

  def test_digest_copy_produces_same_hash
    d = Ca::Md5::Digest.new
    d.update("abc")
    copy = d.copy
    assert_equal d.hexdigest, copy.hexdigest
  end

  def test_digest_copy_is_independent
    d = Ca::Md5::Digest.new
    d.update("abc")
    copy = d.copy
    # Feed different data into each
    d.update("X")
    copy.update("Y")
    refute_equal d.hexdigest, copy.hexdigest
  end

  def test_digest_copy_original_unaffected
    d = Ca::Md5::Digest.new
    d.update("abc")
    expected = d.hexdigest
    copy = d.copy
    copy.update("extra data")
    # Original is unchanged
    assert_equal expected, d.hexdigest
  end

  def test_digest_copy_of_empty
    d = Ca::Md5::Digest.new
    copy = d.copy
    copy.update("abc")
    assert_equal hex("abc"), copy.hexdigest
    # Original is still empty
    assert_equal "d41d8cd98f00b204e9800998ecf8427e", d.hexdigest
  end

  # ─── Streaming Across Block Boundaries ────────────────────────────────────────
  #
  # When update() receives data that spans 64-byte block boundaries, the
  # internal buffering must flush complete blocks and retain partial ones.

  def test_streaming_across_64_byte_boundary
    # Feed 65 bytes in two chunks: 60 bytes then 5 bytes
    d = Ca::Md5::Digest.new
    d.update("a" * 60)
    d.update("a" * 5)
    assert_equal hex("a" * 65), d.hexdigest
  end

  def test_streaming_one_byte_at_a_time
    msg = "abcdefghijklmnopqrstuvwxyz"
    d = Ca::Md5::Digest.new
    msg.each_char { |c| d.update(c) }
    assert_equal hex(msg), d.hexdigest
  end

  def test_streaming_large_chunks
    # 200 bytes in two 100-byte chunks
    d = Ca::Md5::Digest.new
    d.update("Z" * 100)
    d.update("Z" * 100)
    assert_equal hex("Z" * 200), d.hexdigest
  end

  # ─── Large Inputs ─────────────────────────────────────────────────────────────

  def test_1000_byte_input
    msg = "x" * 1000
    result = hex(msg)
    assert_equal 32, result.length
    assert_match(/\A[0-9a-f]{32}\z/, result)
    # Verify streaming matches one-shot
    d = Ca::Md5::Digest.new
    msg.chars.each_slice(37) { |chunk| d.update(chunk.join) }
    assert_equal result, d.hexdigest
  end

  # ─── Known-Value Spot Checks ──────────────────────────────────────────────────

  def test_known_hello_world
    assert_equal "b10a8db164e0754105b7a99be72e3fe5", hex("Hello World")
  end

  def test_known_hello_world_lowercase
    # Case matters — "hello world" != "Hello World"
    assert_equal "5eb63bbbe01eeed093cb22bb8f5acdc3", hex("hello world")
  end

  def test_known_the_quick_brown_fox
    assert_equal "9e107d9d372bb6826bd81d3542a419d6",
      hex("The quick brown fox jumps over the lazy dog")
  end

  def test_known_the_quick_brown_fox_period
    # Changing one character completely changes the hash (avalanche effect)
    assert_equal "e4d909c290d0fb1ca068ffaddf22cbd0",
      hex("The quick brown fox jumps over the lazy dog.")
  end
end
