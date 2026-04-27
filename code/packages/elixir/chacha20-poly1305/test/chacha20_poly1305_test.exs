defmodule CodingAdventures.ChaCha20Poly1305Test do
  @moduledoc """
  Tests for the ChaCha20-Poly1305 implementation.

  Test vectors from RFC 8439 Sections 2.4.2, 2.5.2, and 2.8.2.
  """

  use ExUnit.Case, async: true
  alias CodingAdventures.ChaCha20Poly1305, as: CC

  defp h(hex_str), do: Base.decode16!(hex_str, case: :mixed)
  defp to_hex(bin), do: Base.encode16(bin, case: :lower)

  # =========================================================================
  # ChaCha20 -- RFC 8439 Section 2.4.2
  # =========================================================================

  describe "ChaCha20 stream cipher" do
    test "RFC 8439 Section 2.4.2 -- Sunscreen test vector" do
      key = h(
        "000102030405060708090a0b0c0d0e0f" <>
        "101112131415161718191a1b1c1d1e1f"
      )
      nonce_val = h("000000000000004a00000000")
      counter = 1

      plaintext =
        "Ladies and Gentlemen of the class of '99: " <>
        "If I could offer you only one tip for the future, " <>
        "sunscreen would be it."

      expected_ct = h(
        "6e2e359a2568f98041ba0728dd0d6981" <>
        "e97e7aec1d4360c20a27afccfd9fae0b" <>
        "f91b65c5524733ab8f593dabcd62b357" <>
        "1639d624e65152ab8f530c359f0861d8" <>
        "07ca0dbf500d6a6156a38e088a22b65e" <>
        "52bc514d16ccf806818ce91ab7793736" <>
        "5af90bbf74a35be6b40b8eedf2785e42" <>
        "874d"
      )

      ct = CC.chacha20_encrypt(plaintext, key, nonce_val, counter)
      assert to_hex(ct) == to_hex(expected_ct)
    end

    test "encrypt then decrypt (round-trip)" do
      key = h(
        "000102030405060708090a0b0c0d0e0f" <>
        "101112131415161718191a1b1c1d1e1f"
      )
      nonce_val = h("000000000000004a00000000")
      plaintext = "Hello, ChaCha20!"

      ct = CC.chacha20_encrypt(plaintext, key, nonce_val, 0)
      recovered = CC.chacha20_encrypt(ct, key, nonce_val, 0)
      assert recovered == plaintext
    end

    test "empty plaintext" do
      key = h(
        "000102030405060708090a0b0c0d0e0f" <>
        "101112131415161718191a1b1c1d1e1f"
      )
      nonce_val = h("000000000000000000000000")
      ct = CC.chacha20_encrypt(<<>>, key, nonce_val, 0)
      assert ct == <<>>
    end

    test "single byte" do
      key = h(
        "000102030405060708090a0b0c0d0e0f" <>
        "101112131415161718191a1b1c1d1e1f"
      )
      nonce_val = h("000000000000000000000000")
      ct = CC.chacha20_encrypt("X", key, nonce_val, 0)
      assert byte_size(ct) == 1
      pt = CC.chacha20_encrypt(ct, key, nonce_val, 0)
      assert pt == "X"
    end

    test "multi-block (> 64 bytes)" do
      key = h(
        "000102030405060708090a0b0c0d0e0f" <>
        "101112131415161718191a1b1c1d1e1f"
      )
      nonce_val = h("000000000000000000000000")
      plaintext = String.duplicate("A", 200)
      ct = CC.chacha20_encrypt(plaintext, key, nonce_val, 0)
      assert byte_size(ct) == 200
      recovered = CC.chacha20_encrypt(ct, key, nonce_val, 0)
      assert recovered == plaintext
    end
  end

  # =========================================================================
  # Poly1305 -- RFC 8439 Section 2.5.2
  # =========================================================================

  describe "Poly1305 MAC" do
    test "RFC 8439 Section 2.5.2 -- CFRG test vector" do
      key = h(
        "85d6be7857556d337f4452fe42d506a8" <>
        "0103808afb0db2fd4abff6af4149f51b"
      )
      message = "Cryptographic Forum Research Group"
      expected_tag = h("a8061dc1305136c6c22b8baf0c0127a9")

      tag = CC.poly1305_mac(message, key)
      assert to_hex(tag) == to_hex(expected_tag)
    end

    test "empty message" do
      key = h(
        "00000000000000000000000000000000" <>
        "01020304050607080910111213141516"
      )
      tag = CC.poly1305_mac(<<>>, key)
      assert byte_size(tag) == 16
      # With no blocks, tag = s
      assert to_hex(tag) == "01020304050607080910111213141516"
    end

    test "single byte" do
      key = h(
        "85d6be7857556d337f4452fe42d506a8" <>
        "0103808afb0db2fd4abff6af4149f51b"
      )
      tag = CC.poly1305_mac("A", key)
      assert byte_size(tag) == 16
    end

    test "exactly 16-byte message" do
      key = h(
        "85d6be7857556d337f4452fe42d506a8" <>
        "0103808afb0db2fd4abff6af4149f51b"
      )
      tag = CC.poly1305_mac("0123456789abcdef", key)
      assert byte_size(tag) == 16
    end

    test "17-byte message (two blocks)" do
      key = h(
        "85d6be7857556d337f4452fe42d506a8" <>
        "0103808afb0db2fd4abff6af4149f51b"
      )
      tag = CC.poly1305_mac("0123456789abcdefg", key)
      assert byte_size(tag) == 16
    end
  end

  # =========================================================================
  # AEAD -- RFC 8439 Section 2.8.2
  # =========================================================================

  describe "AEAD ChaCha20-Poly1305" do
    test "RFC 8439 Section 2.8.2 -- encryption" do
      key = h(
        "808182838485868788898a8b8c8d8e8f" <>
        "909192939495969798999a9b9c9d9e9f"
      )
      nonce_val = h("070000004041424344454647")
      aad = h("50515253c0c1c2c3c4c5c6c7")

      plaintext =
        "Ladies and Gentlemen of the class of '99: " <>
        "If I could offer you only one tip for the future, " <>
        "sunscreen would be it."

      expected_ct = h(
        "d31a8d34648e60db7b86afbc53ef7ec2" <>
        "a4aded51296e08fea9e2b5a736ee62d6" <>
        "3dbea45e8ca9671282fafb69da92728b" <>
        "1a71de0a9e060b2905d6a5b67ecd3b36" <>
        "92ddbd7f2d778b8c9803aee328091b58" <>
        "fab324e4fad675945585808b4831d7bc" <>
        "3ff4def08e4b7a9de576d26586cec64b" <>
        "6116"
      )
      expected_tag = h("1ae10b594f09e26a7e902ecbd0600691")

      {ct, tag} = CC.aead_encrypt(plaintext, key, nonce_val, aad)
      assert to_hex(ct) == to_hex(expected_ct)
      assert to_hex(tag) == to_hex(expected_tag)
    end

    test "RFC 8439 Section 2.8.2 -- decryption" do
      key = h(
        "808182838485868788898a8b8c8d8e8f" <>
        "909192939495969798999a9b9c9d9e9f"
      )
      nonce_val = h("070000004041424344454647")
      aad = h("50515253c0c1c2c3c4c5c6c7")

      ct = h(
        "d31a8d34648e60db7b86afbc53ef7ec2" <>
        "a4aded51296e08fea9e2b5a736ee62d6" <>
        "3dbea45e8ca9671282fafb69da92728b" <>
        "1a71de0a9e060b2905d6a5b67ecd3b36" <>
        "92ddbd7f2d778b8c9803aee328091b58" <>
        "fab324e4fad675945585808b4831d7bc" <>
        "3ff4def08e4b7a9de576d26586cec64b" <>
        "6116"
      )
      tag = h("1ae10b594f09e26a7e902ecbd0600691")

      expected_pt =
        "Ladies and Gentlemen of the class of '99: " <>
        "If I could offer you only one tip for the future, " <>
        "sunscreen would be it."

      assert {:ok, ^expected_pt} = CC.aead_decrypt(ct, key, nonce_val, aad, tag)
    end

    test "round-trip" do
      key = h(
        "000102030405060708090a0b0c0d0e0f" <>
        "101112131415161718191a1b1c1d1e1f"
      )
      nonce_val = h("000000000000000000000000")
      aad = "some metadata"
      plaintext = "secret message!"

      {ct, tag} = CC.aead_encrypt(plaintext, key, nonce_val, aad)
      assert {:ok, ^plaintext} = CC.aead_decrypt(ct, key, nonce_val, aad, tag)
    end

    test "authentication failure -- wrong tag" do
      key = h(
        "000102030405060708090a0b0c0d0e0f" <>
        "101112131415161718191a1b1c1d1e1f"
      )
      nonce_val = h("000000000000000000000000")
      {ct, _tag} = CC.aead_encrypt("secret", key, nonce_val, "metadata")
      bad_tag = :binary.copy(<<0>>, 16)
      assert {:error, :authentication_failed} = CC.aead_decrypt(ct, key, nonce_val, "metadata", bad_tag)
    end

    test "authentication failure -- tampered ciphertext" do
      key = h(
        "000102030405060708090a0b0c0d0e0f" <>
        "101112131415161718191a1b1c1d1e1f"
      )
      nonce_val = h("000000000000000000000000")
      {ct, tag} = CC.aead_encrypt("secret", key, nonce_val, "metadata")
      <<first_byte, rest_bytes::binary>> = ct
      tampered = <<Bitwise.bxor(first_byte, 1)>> <> rest_bytes
      assert {:error, :authentication_failed} = CC.aead_decrypt(tampered, key, nonce_val, "metadata", tag)
    end

    test "authentication failure -- wrong AAD" do
      key = h(
        "000102030405060708090a0b0c0d0e0f" <>
        "101112131415161718191a1b1c1d1e1f"
      )
      nonce_val = h("000000000000000000000000")
      {ct, tag} = CC.aead_encrypt("secret", key, nonce_val, "correct aad")
      assert {:error, :authentication_failed} = CC.aead_decrypt(ct, key, nonce_val, "wrong aad", tag)
    end

    test "empty plaintext with AAD" do
      key = h(
        "000102030405060708090a0b0c0d0e0f" <>
        "101112131415161718191a1b1c1d1e1f"
      )
      nonce_val = h("000000000000000000000000")
      aad = "authenticate this"
      {ct, tag} = CC.aead_encrypt(<<>>, key, nonce_val, aad)
      assert ct == <<>>
      assert byte_size(tag) == 16
      assert {:ok, <<>>} = CC.aead_decrypt(ct, key, nonce_val, aad, tag)
    end

    test "empty AAD" do
      key = h(
        "000102030405060708090a0b0c0d0e0f" <>
        "101112131415161718191a1b1c1d1e1f"
      )
      nonce_val = h("000000000000000000000000")
      {ct, tag} = CC.aead_encrypt("hello", key, nonce_val, <<>>)
      assert {:ok, "hello"} = CC.aead_decrypt(ct, key, nonce_val, <<>>, tag)
    end
  end
end
