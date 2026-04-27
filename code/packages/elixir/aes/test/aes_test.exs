defmodule CodingAdventures.AesTest do
  @moduledoc """
  Tests for the AES block cipher implementation.

  Test vectors come from FIPS 197 Appendix B and C.
  """

  use ExUnit.Case, async: true
  alias CodingAdventures.Aes

  defp h(hex), do: Base.decode16!(hex, case: :mixed)

  # ─────────────────────────────────────────────────────────────────────────────
  # FIPS 197 Known-Answer Tests — AES-128
  # ─────────────────────────────────────────────────────────────────────────────

  describe "AES-128 (16-byte key)" do
    test "FIPS 197 Appendix B encrypt" do
      key   = h("2b7e151628aed2a6abf7158809cf4f3c")
      plain = h("3243f6a8885a308d313198a2e0370734")
      assert Aes.aes_encrypt_block(plain, key) == h("3925841d02dc09fbdc118597196a0b32")
    end

    test "FIPS 197 Appendix B decrypt" do
      key    = h("2b7e151628aed2a6abf7158809cf4f3c")
      cipher = h("3925841d02dc09fbdc118597196a0b32")
      assert Aes.aes_decrypt_block(cipher, key) == h("3243f6a8885a308d313198a2e0370734")
    end

    test "FIPS 197 Appendix C.1 — sequential key encrypt" do
      key   = h("000102030405060708090a0b0c0d0e0f")
      plain = h("00112233445566778899aabbccddeeff")
      assert Aes.aes_encrypt_block(plain, key) == h("69c4e0d86a7b0430d8cdb78070b4c55a")
    end

    test "FIPS 197 Appendix C.1 — sequential key decrypt" do
      key    = h("000102030405060708090a0b0c0d0e0f")
      cipher = h("69c4e0d86a7b0430d8cdb78070b4c55a")
      assert Aes.aes_decrypt_block(cipher, key) == h("00112233445566778899aabbccddeeff")
    end

    test "round-trip multiple blocks" do
      key = h("2b7e151628aed2a6abf7158809cf4f3c")
      for start <- Enum.take_every(0..240, 16) do
        plain = :binary.list_to_bin(Enum.to_list(start..(start + 15)))
        ct = Aes.aes_encrypt_block(plain, key)
        assert Aes.aes_decrypt_block(ct, key) == plain
      end
    end

    test "ciphertext differs from plaintext" do
      key   = h("2b7e151628aed2a6abf7158809cf4f3c")
      plain = h("3243f6a8885a308d313198a2e0370734")
      assert Aes.aes_encrypt_block(plain, key) != plain
    end

    test "encrypt returns 16 bytes" do
      key   = h("000102030405060708090a0b0c0d0e0f")
      plain = h("00112233445566778899aabbccddeeff")
      assert byte_size(Aes.aes_encrypt_block(plain, key)) == 16
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # FIPS 197 Known-Answer Tests — AES-192
  # ─────────────────────────────────────────────────────────────────────────────

  describe "AES-192 (24-byte key)" do
    test "FIPS 197 Appendix C.2 encrypt" do
      key   = h("000102030405060708090a0b0c0d0e0f1011121314151617")
      plain = h("00112233445566778899aabbccddeeff")
      assert Aes.aes_encrypt_block(plain, key) == h("dda97ca4864cdfe06eaf70a0ec0d7191")
    end

    test "FIPS 197 Appendix C.2 decrypt" do
      key    = h("000102030405060708090a0b0c0d0e0f1011121314151617")
      cipher = h("dda97ca4864cdfe06eaf70a0ec0d7191")
      assert Aes.aes_decrypt_block(cipher, key) == h("00112233445566778899aabbccddeeff")
    end

    test "round-trip" do
      key = h("000102030405060708090a0b0c0d0e0f1011121314151617")
      plain = h("6bc1bee22e409f96e93d7e117393172a")
      ct = Aes.aes_encrypt_block(plain, key)
      assert Aes.aes_decrypt_block(ct, key) == plain
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # FIPS 197 Known-Answer Tests — AES-256
  # ─────────────────────────────────────────────────────────────────────────────

  describe "AES-256 (32-byte key)" do
    test "FIPS 197 Appendix C.3 encrypt" do
      key   = h("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
      plain = h("00112233445566778899aabbccddeeff")
      assert Aes.aes_encrypt_block(plain, key) == h("8ea2b7ca516745bfeafc49904b496089")
    end

    test "FIPS 197 Appendix C.3 decrypt" do
      key    = h("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
      cipher = h("8ea2b7ca516745bfeafc49904b496089")
      assert Aes.aes_decrypt_block(cipher, key) == h("00112233445566778899aabbccddeeff")
    end

    test "SP 800-38A AES-256 vector" do
      key   = h("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4")
      plain = h("6bc1bee22e409f96e93d7e117393172a")
      assert Aes.aes_encrypt_block(plain, key) == h("f3eed1bdb5d2a03c064b5a7e3db181f8")
    end

    test "SP 800-38A AES-256 decrypt" do
      key    = h("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4")
      cipher = h("f3eed1bdb5d2a03c064b5a7e3db181f8")
      assert Aes.aes_decrypt_block(cipher, key) == h("6bc1bee22e409f96e93d7e117393172a")
    end

    test "round-trip" do
      key = h("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
      for start <- Enum.take_every(0..240, 16) do
        plain = :binary.list_to_bin(Enum.to_list(start..(start + 15)))
        ct = Aes.aes_encrypt_block(plain, key)
        assert Aes.aes_decrypt_block(ct, key) == plain
      end
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # Key Schedule
  # ─────────────────────────────────────────────────────────────────────────────

  describe "expand_key/1" do
    test "AES-128: 11 round keys" do
      key = h("2b7e151628aed2a6abf7158809cf4f3c")
      assert length(Aes.expand_key(key)) == 11
    end

    test "AES-192: 13 round keys" do
      key = h("000102030405060708090a0b0c0d0e0f1011121314151617")
      assert length(Aes.expand_key(key)) == 13
    end

    test "AES-256: 15 round keys" do
      key = h("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
      assert length(Aes.expand_key(key)) == 15
    end

    test "each round key is 4×4" do
      key = h("2b7e151628aed2a6abf7158809cf4f3c")
      for rk <- Aes.expand_key(key) do
        assert length(rk) == 4
        for row <- rk, do: assert length(row) == 4
      end
    end

    test "raises on wrong key size (15 bytes)" do
      assert_raise ArgumentError, ~r/16, 24, or 32/, fn ->
        Aes.expand_key(<<0::120>>)
      end
    end

    test "raises on wrong key size (33 bytes)" do
      assert_raise ArgumentError, ~r/16, 24, or 32/, fn ->
        Aes.expand_key(<<0::264>>)
      end
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # S-box Properties
  # ─────────────────────────────────────────────────────────────────────────────

  describe "S-box properties" do
    test "S-box has 256 entries" do
      assert length(Aes.sbox()) == 256
    end

    test "inverse S-box has 256 entries" do
      assert length(Aes.inv_sbox()) == 256
    end

    test "S-box and inverse S-box are inverses of each other" do
      sbox = List.to_tuple(Aes.sbox())
      inv  = List.to_tuple(Aes.inv_sbox())
      for b <- 0..255 do
        assert elem(inv, elem(sbox, b)) == b
      end
    end

    test "S-box(0) = 0x63 (FIPS 197)" do
      sbox = Aes.sbox()
      assert Enum.at(sbox, 0) == 0x63
    end

    test "S-box(0x53) = 0xED (FIPS 197 Appendix A)" do
      sbox = Aes.sbox()
      assert Enum.at(sbox, 0x53) == 0xed
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # Invalid Inputs
  # ─────────────────────────────────────────────────────────────────────────────

  describe "invalid inputs" do
    test "aes_encrypt_block raises on wrong block size (15 bytes)" do
      key = h("2b7e151628aed2a6abf7158809cf4f3c")
      assert_raise ArgumentError, ~r/16 bytes/, fn ->
        Aes.aes_encrypt_block(<<0::120>>, key)
      end
    end

    test "aes_encrypt_block raises on wrong block size (17 bytes)" do
      key = h("2b7e151628aed2a6abf7158809cf4f3c")
      assert_raise ArgumentError, ~r/16 bytes/, fn ->
        Aes.aes_encrypt_block(<<0::136>>, key)
      end
    end

    test "aes_encrypt_block raises on wrong key size" do
      plain = h("00112233445566778899aabbccddeeff")
      assert_raise ArgumentError, ~r/16, 24, or 32/, fn ->
        Aes.aes_encrypt_block(plain, <<0::80>>)
      end
    end

    test "aes_decrypt_block raises on wrong block size" do
      key = h("2b7e151628aed2a6abf7158809cf4f3c")
      assert_raise ArgumentError, ~r/16 bytes/, fn ->
        Aes.aes_decrypt_block(<<0::88>>, key)
      end
    end

    test "aes_decrypt_block raises on wrong key size" do
      block = h("00112233445566778899aabbccddeeff")
      assert_raise ArgumentError, ~r/16, 24, or 32/, fn ->
        Aes.aes_decrypt_block(block, <<0::96>>)
      end
    end
  end
end
