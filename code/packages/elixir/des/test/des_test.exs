defmodule CodingAdventures.DesTest do
  @moduledoc """
  Tests for the DES and 3DES implementation.

  Test vectors come from:
  - FIPS 46-3 / SP 800-20 Known-Answer Tests
  - NIST SP 800-67 TDEA test vectors
  - Stallings "Cryptography and Network Security" worked examples
  """

  use ExUnit.Case, async: true
  alias CodingAdventures.Des

  # Convenience: decode a hex string (no spaces) to binary
  defp h(hex), do: Base.decode16!(hex, case: :mixed)

  # ─────────────────────────────────────────────────────────────────────────────
  # DES Encrypt Block — FIPS / SP 800-20 Known-Answer Tests
  # ─────────────────────────────────────────────────────────────────────────────

  describe "des_encrypt_block/2 — FIPS known-answer tests" do
    test "Stallings/FIPS 46 worked example" do
      # The classic DES example from most textbooks.
      # Key = 133457799BBCDFF1, Plain = 0123456789ABCDEF → 85E813540F0AB405
      key   = h("133457799BBCDFF1")
      plain = h("0123456789ABCDEF")
      assert Des.des_encrypt_block(plain, key) == h("85E813540F0AB405")
    end

    test "SP 800-20 Table 1 row 0 — plaintext variable, key=0101…01" do
      key = h("0101010101010101")
      assert Des.des_encrypt_block(h("95F8A5E5DD31D900"), key) == h("8000000000000000")
    end

    test "SP 800-20 Table 1 row 1" do
      key = h("0101010101010101")
      assert Des.des_encrypt_block(h("DD7F121CA5015619"), key) == h("4000000000000000")
    end

    test "SP 800-20 Table 2 row 0 — key variable, plain=0000…00" do
      assert Des.des_encrypt_block(h("0000000000000000"), h("8001010101010101")) ==
               h("95A8D72813DAA94D")
    end

    test "SP 800-20 Table 2 row 1" do
      assert Des.des_encrypt_block(h("0000000000000000"), h("4001010101010101")) ==
               h("0EEC1487DD8C26D5")
    end

    test "round-trip for multiple blocks" do
      key = h("FEDCBA9876543210")
      for start <- Enum.take_every(0..248, 8) do
        block = :binary.list_to_bin(Enum.to_list(start..(start + 7)))
        assert Des.des_decrypt_block(Des.des_encrypt_block(block, key), key) == block
      end
    end

    test "returns 8 bytes" do
      key   = h("0101010101010101")
      plain = h("0000000000000000")
      assert byte_size(Des.des_encrypt_block(plain, key)) == 8
    end

    test "deterministic — same input always produces same output" do
      key   = h("FEDCBA9876543210")
      plain = h("0123456789ABCDEF")
      assert Des.des_encrypt_block(plain, key) == Des.des_encrypt_block(plain, key)
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # DES Decrypt Block
  # ─────────────────────────────────────────────────────────────────────────────

  describe "des_decrypt_block/2" do
    test "decrypts FIPS vector 1" do
      key = h("133457799BBCDFF1")
      ct  = h("85E813540F0AB405")
      assert Des.des_decrypt_block(ct, key) == h("0123456789ABCDEF")
    end

    test "round-trip with various keys" do
      plain = h("0123456789ABCDEF")
      keys = [
        h("133457799BBCDFF0"),
        h("FFFFFFFFFFFFFFFF"),
        h("0000000000000000"),
        h("FEDCBA9876543210")
      ]
      for key <- keys do
        ct = Des.des_encrypt_block(plain, key)
        assert Des.des_decrypt_block(ct, key) == plain
      end
    end

    test "returns 8 bytes" do
      key = h("133457799BBCDFF1")
      ct  = h("85E813540F0AB405")
      assert byte_size(Des.des_decrypt_block(ct, key)) == 8
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # Key Schedule
  # ─────────────────────────────────────────────────────────────────────────────

  describe "expand_key/1" do
    test "returns 16 subkeys" do
      key = h("0133457799BBCDFF")
      assert length(Des.expand_key(key)) == 16
    end

    test "each subkey is 6 bytes (48 bits)" do
      key = h("0133457799BBCDFF")
      for sk <- Des.expand_key(key) do
        assert byte_size(sk) == 6
      end
    end

    test "different keys produce different subkeys" do
      sk1 = Des.expand_key(h("0133457799BBCDFF"))
      sk2 = Des.expand_key(h("FEDCBA9876543210"))
      assert sk1 != sk2
    end

    test "subkeys are not all the same" do
      key = h("0133457799BBCDFF")
      subkeys = Des.expand_key(key)
      assert Enum.uniq(subkeys) |> length() > 1
    end

    test "raises on wrong key size (7 bytes)" do
      assert_raise ArgumentError, ~r/8 bytes/, fn ->
        Des.expand_key(<<0::56>>)
      end
    end

    test "raises on wrong key size (9 bytes)" do
      assert_raise ArgumentError, ~r/8 bytes/, fn ->
        Des.expand_key(<<0::72>>)
      end
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # Invalid Inputs
  # ─────────────────────────────────────────────────────────────────────────────

  describe "invalid inputs" do
    test "des_encrypt_block raises on wrong block size (7 bytes)" do
      assert_raise ArgumentError, ~r/8 bytes/, fn ->
        Des.des_encrypt_block(<<0::56>>, h("0133457799BBCDFF"))
      end
    end

    test "des_encrypt_block raises on wrong block size (16 bytes)" do
      assert_raise ArgumentError, ~r/8 bytes/, fn ->
        Des.des_encrypt_block(<<0::128>>, h("0133457799BBCDFF"))
      end
    end

    test "des_encrypt_block raises on wrong key size (4 bytes)" do
      assert_raise ArgumentError, ~r/8 bytes/, fn ->
        Des.des_encrypt_block(h("0123456789ABCDEF"), <<0::32>>)
      end
    end

    test "des_decrypt_block raises on wrong block size (9 bytes)" do
      assert_raise ArgumentError, ~r/8 bytes/, fn ->
        Des.des_decrypt_block(<<0::72>>, h("0133457799BBCDFF"))
      end
    end

    test "des_decrypt_block raises on wrong key size (16 bytes)" do
      assert_raise ArgumentError, ~r/8 bytes/, fn ->
        Des.des_decrypt_block(h("0123456789ABCDEF"), <<0::128>>)
      end
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # ECB Mode
  # ─────────────────────────────────────────────────────────────────────────────

  describe "des_ecb_encrypt/2 and des_ecb_decrypt/2" do
    test "8-byte input → 16 bytes ciphertext (one data block + one padding block)" do
      ecb_key = h("0133457799BBCDFF")
      ct = Des.des_ecb_encrypt(h("0123456789ABCDEF"), ecb_key)
      assert byte_size(ct) == 16
    end

    test "sub-block input → 8 bytes ciphertext" do
      ecb_key = h("0133457799BBCDFF")
      ct = Des.des_ecb_encrypt("hello", ecb_key)
      assert byte_size(ct) == 8
    end

    test "16-byte input → 24 bytes ciphertext" do
      ecb_key = h("0133457799BBCDFF")
      ct = Des.des_ecb_encrypt(:binary.copy(<<0>>, 16), ecb_key)
      assert byte_size(ct) == 24
    end

    test "empty input → 8 bytes ciphertext (full padding block)" do
      ecb_key = h("0133457799BBCDFF")
      ct = Des.des_ecb_encrypt(<<>>, ecb_key)
      assert byte_size(ct) == 8
    end

    test "round-trip short message" do
      ecb_key = h("0133457799BBCDFF")
      plain = "hello"
      assert Des.des_ecb_decrypt(Des.des_ecb_encrypt(plain, ecb_key), ecb_key) == plain
    end

    test "round-trip exact block" do
      ecb_key = h("0133457799BBCDFF")
      plain = "ABCDEFGH"
      assert Des.des_ecb_decrypt(Des.des_ecb_encrypt(plain, ecb_key), ecb_key) == plain
    end

    test "round-trip multi-block" do
      ecb_key = h("0133457799BBCDFF")
      plain = "The quick brown fox jumps"
      assert Des.des_ecb_decrypt(Des.des_ecb_encrypt(plain, ecb_key), ecb_key) == plain
    end

    test "round-trip empty" do
      ecb_key = h("0133457799BBCDFF")
      assert Des.des_ecb_decrypt(Des.des_ecb_encrypt(<<>>, ecb_key), ecb_key) == <<>>
    end

    test "round-trip 256 bytes" do
      ecb_key = h("0133457799BBCDFF")
      plain = :binary.list_to_bin(Enum.to_list(0..255))
      assert Des.des_ecb_decrypt(Des.des_ecb_encrypt(plain, ecb_key), ecb_key) == plain
    end

    test "des_ecb_decrypt raises on empty ciphertext" do
      ecb_key = h("0133457799BBCDFF")
      assert_raise ArgumentError, fn ->
        Des.des_ecb_decrypt(<<>>, ecb_key)
      end
    end

    test "des_ecb_decrypt raises if ciphertext not multiple of 8" do
      ecb_key = h("0133457799BBCDFF")
      assert_raise ArgumentError, ~r/multiple of 8/, fn ->
        Des.des_ecb_decrypt(<<0::56>>, ecb_key)
      end
    end

    test "bad padding raises" do
      ecb_key = h("0133457799BBCDFF")
      ct = Des.des_ecb_encrypt("test data", ecb_key)
      # Flip the last byte to corrupt padding
      len = byte_size(ct)
      last_byte = :binary.last(ct)
      corrupted = binary_part(ct, 0, len - 1) <> <<Bitwise.bxor(last_byte, 0xFF)>>
      assert_raise ArgumentError, fn ->
        Des.des_ecb_decrypt(corrupted, ecb_key)
      end
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # 3DES / TDEA
  # ─────────────────────────────────────────────────────────────────────────────

  describe "tdea_encrypt_block/4 and tdea_decrypt_block/4" do
    test "TDEA encrypt — NIST SP 800-67 EDE vector" do
      k1    = h("0123456789ABCDEF")
      k2    = h("23456789ABCDEF01")
      k3    = h("456789ABCDEF0123")
      plain = h("6BC1BEE22E409F96")
      cipher = h("3B6423D418DEFC23")
      assert Des.tdea_encrypt_block(plain, k1, k2, k3) == cipher
    end

    test "TDEA decrypt" do
      k1     = h("0123456789ABCDEF")
      k2     = h("23456789ABCDEF01")
      k3     = h("456789ABCDEF0123")
      plain  = h("6BC1BEE22E409F96")
      cipher = h("3B6423D418DEFC23")
      assert Des.tdea_decrypt_block(cipher, k1, k2, k3) == plain
    end

    test "TDEA round-trip with random keys" do
      k1 = h("FEDCBA9876543210")
      k2 = h("0F1E2D3C4B5A6978")
      k3 = h("7869584A3B2C1D0E")
      plain = h("0123456789ABCDEF")
      ct = Des.tdea_encrypt_block(plain, k1, k2, k3)
      assert Des.tdea_decrypt_block(ct, k1, k2, k3) == plain
    end

    test "TDEA backward compat: K1=K2=K3 reduces to single DES" do
      key   = h("0133457799BBCDFF")
      plain = h("0123456789ABCDEF")
      assert Des.tdea_encrypt_block(plain, key, key, key) ==
               Des.des_encrypt_block(plain, key)
    end

    test "TDEA decrypt backward compat: K1=K2=K3 reduces to single DES decrypt" do
      key = h("FEDCBA9876543210")
      ct  = h("0123456789ABCDEF")
      assert Des.tdea_decrypt_block(ct, key, key, key) ==
               Des.des_decrypt_block(ct, key)
    end

    test "TDEA round-trip all-same blocks" do
      k1 = h("1234567890ABCDEF")
      k2 = h("FEDCBA0987654321")
      k3 = h("0F0F0F0F0F0F0F0F")
      for val <- [0x00, 0xFF, 0xA5, 0x5A] do
        plain = :binary.copy(<<val>>, 8)
        ct = Des.tdea_encrypt_block(plain, k1, k2, k3)
        assert Des.tdea_decrypt_block(ct, k1, k2, k3) == plain
      end
    end
  end
end
