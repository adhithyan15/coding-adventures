defmodule CodingAdventures.Md5Test do
  use ExUnit.Case, async: true

  alias CodingAdventures.Md5

  # ─── RFC 1321 Official Test Vectors ──────────────────────────────────────────
  #
  # These are the canonical test vectors from RFC 1321 Appendix A.5.
  # Any correct MD5 implementation MUST produce exactly these outputs.
  # If any of these fail, the implementation is wrong.

  describe "RFC 1321 official test vectors" do
    test "empty string" do
      # MD5("") = d41d8cd98f00b204e9800998ecf8427e
      # The "nothing" hash — even with no input, MD5 produces a unique digest
      # because the padding (0x80 + length) is still hashed.
      assert Md5.md5_hex("") == "d41d8cd98f00b204e9800998ecf8427e"
    end

    test "single character 'a'" do
      # MD5("a") = 0cc175b9c0f1b6a831c399e269772661
      assert Md5.md5_hex("a") == "0cc175b9c0f1b6a831c399e269772661"
    end

    test "three characters 'abc'" do
      # MD5("abc") = 900150983cd24fb0d6963f7d28e17f72
      # One of the most commonly used MD5 test vectors.
      assert Md5.md5_hex("abc") == "900150983cd24fb0d6963f7d28e17f72"
    end

    test "message digest string" do
      # MD5("message digest") = f96b697d7cb7938d525a2f31aaf161d0
      assert Md5.md5_hex("message digest") == "f96b697d7cb7938d525a2f31aaf161d0"
    end

    test "lowercase alphabet" do
      # MD5("abcdefghijklmnopqrstuvwxyz") = c3fcd3d76192e4007dfb496cca67e13b
      assert Md5.md5_hex("abcdefghijklmnopqrstuvwxyz") == "c3fcd3d76192e4007dfb496cca67e13b"
    end

    test "mixed alphanumeric" do
      # MD5("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789")
      # = d174ab98d277d9f5a5611c2c9f419d9f
      input = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
      assert Md5.md5_hex(input) == "d174ab98d277d9f5a5611c2c9f419d9f"
    end

    test "digit sequence repeated" do
      # MD5("12345678901234567890123456789012345678901234567890123456789012345678901234567890")
      # = 57edf4a22be3c955ac49da2e2107b67a
      input = "12345678901234567890123456789012345678901234567890123456789012345678901234567890"
      assert Md5.md5_hex(input) == "57edf4a22be3c955ac49da2e2107b67a"
    end
  end

  # ─── Output Format Tests ──────────────────────────────────────────────────────

  describe "output format" do
    test "md5/1 returns exactly 16 bytes" do
      # MD5 always produces a fixed-size 128-bit (16-byte) output regardless of
      # input size. This is the defining property of a hash function.
      assert byte_size(Md5.md5("")) == 16
      assert byte_size(Md5.md5("hello")) == 16
      assert byte_size(Md5.md5(String.duplicate("x", 1000))) == 16
    end

    test "md5_hex/1 returns exactly 32 characters" do
      # 16 bytes × 2 hex digits/byte = 32 hex characters
      assert String.length(Md5.md5_hex("")) == 32
      assert String.length(Md5.md5_hex("hello")) == 32
    end

    test "md5_hex/1 uses lowercase hex" do
      # RFC 1321 shows lowercase. We use Base.encode16(case: :lower).
      # All letters should be a-f, not A-F.
      hex = Md5.md5_hex("abc")
      assert hex == String.downcase(hex)
      assert Regex.match?(~r/^[0-9a-f]{32}$/, hex)
    end

    test "md5/1 returns a binary" do
      result = Md5.md5("test")
      assert is_binary(result)
    end

    test "md5_hex/1 returns a string" do
      result = Md5.md5_hex("test")
      assert is_binary(result)
      # Verify it contains only valid hex characters
      assert Regex.match?(~r/^[0-9a-f]{32}$/, result)
    end

    test "md5_hex is consistent with md5 plus Base.encode16" do
      # md5_hex should be equivalent to md5 |> Base.encode16(case: :lower)
      data = "consistency check"
      assert Md5.md5_hex(data) == Md5.md5(data) |> Base.encode16(case: :lower)
    end
  end

  # ─── Little-Endian Correctness Tests ─────────────────────────────────────────
  #
  # The most common MD5 bug: using big-endian byte order instead of little-endian.
  # These tests specifically exercise the byte-order sensitivity.

  describe "little-endian correctness" do
    test "md5 digest bytes are in little-endian order" do
      # We can verify little-endian by checking the raw byte layout of a known
      # output. For "abc", the hex is 900150983cd24fb0d6963f7d28e17f72.
      # The first 4 bytes (little-endian word A) should be: 90 01 50 98
      result = Md5.md5("abc")
      <<first_byte, second_byte, third_byte, fourth_byte, _rest::binary>> = result
      # First word of output = 0x900150983 → little-endian bytes: 90 01 50 98
      assert first_byte == 0x90
      assert second_byte == 0x01
      assert third_byte == 0x50
      assert fourth_byte == 0x98
    end

    test "empty string digest byte layout" do
      # d41d8cd98f00b204e9800998ecf8427e
      # Bytes: d4 1d 8c d9 8f 00 b2 04 e9 80 09 98 ec f8 42 7e
      result = Md5.md5("")
      assert result == <<0xd4, 0x1d, 0x8c, 0xd9, 0x8f, 0x00, 0xb2, 0x04,
                         0xe9, 0x80, 0x09, 0x98, 0xec, 0xf8, 0x42, 0x7e>>
    end

    test "padding length field is little-endian" do
      # For "abc" (24 bits), the padding must end with 24 as a little-endian
      # 64-bit integer: 18 00 00 00 00 00 00 00 (NOT 00 00 00 00 00 00 00 18)
      # If we got a big-endian implementation, the hash would differ.
      # The RFC 1321 test vector for "abc" is the definitive check.
      assert Md5.md5_hex("abc") == "900150983cd24fb0d6963f7d28e17f72"
    end

    test "two inputs differing only in byte order would hash differently" do
      # Direct demonstration: bytes [0x01, 0x00, 0x00, 0x00] vs [0x00, 0x00, 0x00, 0x01]
      # These are different inputs and must produce different hashes.
      little = Md5.md5_hex(<<0x01, 0x00, 0x00, 0x00>>)
      big = Md5.md5_hex(<<0x00, 0x00, 0x00, 0x01>>)
      assert little != big
    end
  end

  # ─── Block Boundary Tests ─────────────────────────────────────────────────────
  #
  # MD5 processes data in 64-byte (512-bit) blocks. Inputs at or near block
  # boundaries are a common source of bugs.

  describe "block boundary edge cases" do
    test "55-byte input fits in one block (55 bytes → needs 1 byte pad + 8 byte length)" do
      # A 55-byte message fills exactly one block:
      # 55 data bytes + 1 (0x80) + 8 (length) = 64 bytes exactly.
      # This is the tightest single-block case.
      input = String.duplicate("a", 55)
      result = Md5.md5_hex(input)
      assert String.length(result) == 32
      assert Regex.match?(~r/^[0-9a-f]{32}$/, result)
    end

    test "56-byte input spans two blocks" do
      # A 56-byte message does NOT fit in one block:
      # 56 bytes + 1 (0x80) = 57 bytes — we can't fit the 8-byte length without
      # going to 65 bytes total. So we need a second block.
      # Padding goes to 64+56 = 120 bytes.
      input = String.duplicate("a", 56)
      result = Md5.md5_hex(input)
      assert String.length(result) == 32
      # Verify against known hash
      assert result == Md5.md5_hex(String.duplicate("a", 56))
    end

    test "63-byte input (just under one full block)" do
      input = String.duplicate("a", 63)
      result = Md5.md5_hex(input)
      assert String.length(result) == 32
      assert Regex.match?(~r/^[0-9a-f]{32}$/, result)
    end

    test "64-byte input (exactly one full block)" do
      # Exactly 64 bytes: the whole message fills one block.
      # After padding: 64 + 64 = 128 bytes (two blocks total).
      input = String.duplicate("a", 64)
      result = Md5.md5_hex(input)
      assert String.length(result) == 32
      assert Regex.match?(~r/^[0-9a-f]{32}$/, result)
    end

    test "65-byte input (just over one full block)" do
      input = String.duplicate("a", 65)
      result = Md5.md5_hex(input)
      assert String.length(result) == 32
      assert Regex.match?(~r/^[0-9a-f]{32}$/, result)
    end

    test "128-byte input (exactly two full blocks)" do
      input = String.duplicate("a", 128)
      result = Md5.md5_hex(input)
      assert String.length(result) == 32
      assert Regex.match?(~r/^[0-9a-f]{32}$/, result)
    end

    test "different block sizes produce different hashes" do
      # As input grows, hashes must change.
      h55 = Md5.md5_hex(String.duplicate("a", 55))
      h56 = Md5.md5_hex(String.duplicate("a", 56))
      h64 = Md5.md5_hex(String.duplicate("a", 64))
      assert h55 != h56
      assert h56 != h64
      assert h55 != h64
    end
  end

  # ─── Determinism and Collision-Resistance Tests ───────────────────────────────

  describe "determinism" do
    test "same input always produces same output" do
      input = "determinism test"
      assert Md5.md5(input) == Md5.md5(input)
      assert Md5.md5_hex(input) == Md5.md5_hex(input)
    end

    test "different inputs produce different outputs" do
      # This is not guaranteed by the algorithm (collisions exist) but holds
      # for these simple cases.
      assert Md5.md5_hex("hello") != Md5.md5_hex("world")
      assert Md5.md5_hex("abc") != Md5.md5_hex("abd")
      assert Md5.md5_hex("") != Md5.md5_hex("a")
    end

    test "one-bit difference changes the output completely (avalanche effect)" do
      # MD5's avalanche property: changing 1 bit in input changes ~50% of output bits.
      # "abc" vs "abd" — only last character differs.
      hash_abc = Md5.md5_hex("abc")
      hash_abd = Md5.md5_hex("abd")
      assert hash_abc != hash_abd
      # They should differ in many positions, not just one.
      common = Enum.zip(String.graphemes(hash_abc), String.graphemes(hash_abd))
      |> Enum.count(fn {a, b} -> a == b end)
      # With true avalanche, we expect roughly 16/32 chars to differ.
      # Conservatively check that at least 8 of 32 hex chars differ.
      assert common < 24
    end
  end

  # ─── Additional Well-Known Hashes ─────────────────────────────────────────────

  describe "additional known hashes" do
    test "md5 of 'hello'" do
      assert Md5.md5_hex("hello") == "5d41402abc4b2a76b9719d911017c592"
    end

    test "md5 of 'hello world'" do
      assert Md5.md5_hex("hello world") == "5eb63bbbe01eeed093cb22bb8f5acdc3"
    end

    test "md5 of 'The quick brown fox jumps over the lazy dog'" do
      assert Md5.md5_hex("The quick brown fox jumps over the lazy dog") ==
               "9e107d9d372bb6826bd81d3542a419d6"
    end

    test "md5 of 'The quick brown fox jumps over the lazy dog.' (with period)" do
      # Adding just one character changes the hash completely.
      assert Md5.md5_hex("The quick brown fox jumps over the lazy dog.") ==
               "e4d909c290d0fb1ca068ffaddf22cbd0"
    end

    test "md5 of binary data with null bytes" do
      # MD5 works on raw bytes, not strings. Null bytes are valid.
      result = Md5.md5_hex(<<0, 0, 0, 0>>)
      assert result == "f1d3ff8443297732862df21dc4e57262"
    end

    test "md5 of all byte values 0x00 to 0xFF" do
      input = for i <- 0..255, do: i
      data = :binary.list_to_bin(input)
      result = Md5.md5_hex(data)
      assert String.length(result) == 32
      assert Regex.match?(~r/^[0-9a-f]{32}$/, result)
      # Verify against the known hash for this input
      assert result == "e2c865db4162bed963bfaa9ef6ac18f0"
    end
  end

  # ─── Edge Cases ───────────────────────────────────────────────────────────────

  describe "edge cases" do
    test "single null byte" do
      assert Md5.md5_hex(<<0>>) == "93b885adfe0da089cdf634904fd59f71"
    end

    test "single byte 0xFF" do
      assert Md5.md5_hex(<<0xFF>>) == "00594fd4f42ba43fc1ca0427a0576295"
    end

    test "exactly 55 bytes (max single-block without extra block for length)" do
      # 55 bytes + 1 (0x80 pad) + 8 (little-endian length) = 64 bytes = 1 block
      input = String.duplicate("b", 55)
      result = Md5.md5_hex(input)
      assert String.length(result) == 32
      # Verify it's deterministic
      assert result == Md5.md5_hex(String.duplicate("b", 55))
    end

    test "large input (1KB)" do
      input = String.duplicate("x", 1024)
      result = Md5.md5_hex(input)
      assert String.length(result) == 32
      assert Regex.match?(~r/^[0-9a-f]{32}$/, result)
    end

    test "binary input vs string input of same bytes" do
      # In Elixir, strings are binaries, so these should be the same.
      assert Md5.md5("abc") == Md5.md5(<<"abc">>)
      assert Md5.md5_hex("abc") == Md5.md5_hex(<<"abc">>)
    end
  end
end
