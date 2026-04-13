defmodule CodingAdventures.AesModesTest do
  @moduledoc """
  Tests for AES modes of operation: ECB, CBC, CTR, GCM.
  Test vectors from NIST SP 800-38A and the GCM specification.
  """

  use ExUnit.Case, async: true
  import Bitwise
  alias CodingAdventures.AesModes

  defp h(hex), do: Base.decode16!(hex, case: :mixed)
  defp to_hex(bin), do: Base.encode16(bin, case: :lower)

  # ─────────────────────────────────────────────────────────────────────────────
  # PKCS#7 Padding
  # ─────────────────────────────────────────────────────────────────────────────

  describe "PKCS#7 padding" do
    test "pads empty input to 16 bytes of 0x10" do
      padded = AesModes.pkcs7_pad(<<>>)
      assert byte_size(padded) == 16
      assert padded == :binary.copy(<<16>>, 16)
    end

    test "pads 5-byte input with 11 bytes of 0x0B" do
      padded = AesModes.pkcs7_pad("HELLO")
      assert byte_size(padded) == 16
      assert binary_part(padded, 0, 5) == "HELLO"
      assert binary_part(padded, 5, 11) == :binary.copy(<<11>>, 11)
    end

    test "pads aligned input with full block" do
      padded = AesModes.pkcs7_pad(:binary.copy("A", 16))
      assert byte_size(padded) == 32
    end

    test "round-trips through pad/unpad" do
      for len <- 0..48 do
        data = :binary.copy("X", len)
        assert AesModes.pkcs7_unpad(AesModes.pkcs7_pad(data)) == data
      end
    end

    test "rejects invalid padding value" do
      assert_raise ArgumentError, fn ->
        AesModes.pkcs7_unpad(:binary.copy(<<0>>, 16))
      end
    end

    test "rejects inconsistent padding bytes" do
      bad = :binary.copy("A", 13) <> <<1, 1, 3>>
      assert_raise ArgumentError, fn ->
        AesModes.pkcs7_unpad(bad)
      end
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # ECB Mode — NIST SP 800-38A
  # ─────────────────────────────────────────────────────────────────────────────

  describe "ECB mode" do
    setup do
      %{key: h("2b7e151628aed2a6abf7158809cf4f3c")}
    end

    test "encrypts single block (NIST F.1.1 block 1)", %{key: key} do
      pt = h("6bc1bee22e409f96e93d7e117393172a")
      ct = AesModes.ecb_encrypt(pt, key)
      assert byte_size(ct) == 32
      assert to_hex(binary_part(ct, 0, 16)) == "3ad77bb40d7a3660a89ecaf32466ef97"
    end

    test "round-trips single block", %{key: key} do
      pt = h("6bc1bee22e409f96e93d7e117393172a")
      ct = AesModes.ecb_encrypt(pt, key)
      assert AesModes.ecb_decrypt(ct, key) == pt
    end

    test "round-trips arbitrary lengths", %{key: key} do
      for len <- [0, 1, 15, 16, 17, 31, 32, 100] do
        pt = :binary.copy("Z", len)
        ct = AesModes.ecb_encrypt(pt, key)
        assert AesModes.ecb_decrypt(ct, key) == pt
      end
    end

    test "identical blocks produce identical ciphertext", %{key: key} do
      block = h("6bc1bee22e409f96e93d7e117393172a")
      ct = AesModes.ecb_encrypt(block <> block, key)
      assert binary_part(ct, 0, 16) == binary_part(ct, 16, 16)
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # CBC Mode — NIST SP 800-38A
  # ─────────────────────────────────────────────────────────────────────────────

  describe "CBC mode" do
    setup do
      %{
        key: h("2b7e151628aed2a6abf7158809cf4f3c"),
        iv: h("000102030405060708090a0b0c0d0e0f")
      }
    end

    test "encrypts single block (NIST F.2.1 block 1)", %{key: key, iv: iv} do
      pt = h("6bc1bee22e409f96e93d7e117393172a")
      ct = AesModes.cbc_encrypt(pt, key, iv)
      assert byte_size(ct) == 32
      assert to_hex(binary_part(ct, 0, 16)) == "7649abac8119b246cee98e9b12e9197d"
    end

    test "round-trips multi-block", %{key: key, iv: iv} do
      pt = h("6bc1bee22e409f96e93d7e117393172a"
          <> "ae2d8a571e03ac9c9eb76fac45af8e51"
          <> "30c81c46a35ce411e5fbc1191a0a52ef"
          <> "f69f2445df4f9b17ad2b417be66c3710")
      ct = AesModes.cbc_encrypt(pt, key, iv)
      assert to_hex(binary_part(ct, 0, 16)) == "7649abac8119b246cee98e9b12e9197d"
      assert AesModes.cbc_decrypt(ct, key, iv) == pt
    end

    test "round-trips arbitrary lengths", %{key: key, iv: iv} do
      for len <- [0, 1, 15, 16, 17, 31, 32, 100] do
        pt = :binary.copy("Q", len)
        ct = AesModes.cbc_encrypt(pt, key, iv)
        assert AesModes.cbc_decrypt(ct, key, iv) == pt
      end
    end

    test "identical blocks produce different ciphertext", %{key: key, iv: iv} do
      block = h("6bc1bee22e409f96e93d7e117393172a")
      ct = AesModes.cbc_encrypt(block <> block, key, iv)
      assert binary_part(ct, 0, 16) != binary_part(ct, 16, 16)
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # CTR Mode
  # ─────────────────────────────────────────────────────────────────────────────

  describe "CTR mode" do
    setup do
      %{key: h("2b7e151628aed2a6abf7158809cf4f3c")}
    end

    test "round-trips single block", %{key: key} do
      nonce = h("f0f1f2f3f4f5f6f7f8f9fafb")
      pt = h("6bc1bee22e409f96e93d7e117393172a")
      ct = AesModes.ctr_encrypt(pt, key, nonce)
      assert byte_size(ct) == 16
      assert AesModes.ctr_decrypt(ct, key, nonce) == pt
    end

    test "handles partial blocks (no padding)", %{key: key} do
      nonce = h("aabbccddeeff00112233aabb")
      pt = "Short"
      ct = AesModes.ctr_encrypt(pt, key, nonce)
      assert byte_size(ct) == 5
      assert AesModes.ctr_decrypt(ct, key, nonce) == pt
    end

    test "round-trips arbitrary lengths", %{key: key} do
      nonce = h("112233445566778899aabbcc")
      for len <- [0, 1, 15, 16, 17, 31, 32, 100] do
        pt = :binary.copy("C", len)
        ct = AesModes.ctr_encrypt(pt, key, nonce)
        assert byte_size(ct) == len
        assert AesModes.ctr_decrypt(ct, key, nonce) == pt
      end
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # GCM Mode — NIST test vectors
  # ─────────────────────────────────────────────────────────────────────────────

  describe "GCM mode" do
    test "Test Case 2: empty plaintext" do
      key = h("00000000000000000000000000000000")
      iv = h("000000000000000000000000")
      {ct, tag} = AesModes.gcm_encrypt(<<>>, key, iv, <<>>)
      assert byte_size(ct) == 0
      assert to_hex(tag) == "58e2fccefa7e3061367f1d57a4e7455a"
      assert {:ok, <<>>} == AesModes.gcm_decrypt(ct, key, iv, <<>>, tag)
    end

    test "Test Case 3: 64-byte plaintext, no AAD" do
      key = h("feffe9928665731c6d6a8f9467308308")
      iv = h("cafebabefacedbaddecaf888")
      pt = h("d9313225f88406e5a55909c5aff5269a"
          <> "86a7a9531534f7da2e4c303d8a318a72"
          <> "1c3c0c95956809532fcf0e2449a6b525"
          <> "b16aedf5aa0de657ba637b391aafd255")
      {ct, tag} = AesModes.gcm_encrypt(pt, key, iv, <<>>)
      assert to_hex(ct) ==
        "42831ec2217774244b7221b784d0d49c" <>
        "e3aa212f2c02a4e035c17e2329aca12e" <>
        "21d514b25466931c7d8f6a5aac84aa05" <>
        "1ba30b396a0aac973d58e091473f5985"
      assert to_hex(tag) == "4d5c2af327cd64a62cf35abd2ba6fab4"
      assert {:ok, ^pt} = AesModes.gcm_decrypt(ct, key, iv, <<>>, tag)
    end

    test "Test Case 4: plaintext with AAD" do
      key = h("feffe9928665731c6d6a8f9467308308")
      iv = h("cafebabefacedbaddecaf888")
      pt = h("d9313225f88406e5a55909c5aff5269a"
          <> "86a7a9531534f7da2e4c303d8a318a72"
          <> "1c3c0c95956809532fcf0e2449a6b525"
          <> "b16aedf5aa0de657ba637b39")
      aad_data = h("feedfacedeadbeeffeedfacedeadbeef"
               <> "abaddad2")
      {ct, tag} = AesModes.gcm_encrypt(pt, key, iv, aad_data)
      assert to_hex(ct) ==
        "42831ec2217774244b7221b784d0d49c" <>
        "e3aa212f2c02a4e035c17e2329aca12e" <>
        "21d514b25466931c7d8f6a5aac84aa05" <>
        "1ba30b396a0aac973d58e091"
      assert to_hex(tag) == "5bc94fbc3221a5db94fae95ae7121a47"
      assert {:ok, ^pt} = AesModes.gcm_decrypt(ct, key, iv, aad_data, tag)
    end

    test "rejects tampered ciphertext" do
      key = h("feffe9928665731c6d6a8f9467308308")
      iv = h("cafebabefacedbaddecaf888")
      pt = h("d9313225f88406e5a55909c5aff5269a")
      {ct, tag} = AesModes.gcm_encrypt(pt, key, iv, <<>>)
      <<first_byte, rest::binary>> = ct
      tampered = <<bxor(first_byte, 1)>> <> rest
      assert {:error, _} = AesModes.gcm_decrypt(tampered, key, iv, <<>>, tag)
    end

    test "rejects tampered tag" do
      key = h("feffe9928665731c6d6a8f9467308308")
      iv = h("cafebabefacedbaddecaf888")
      pt = h("d9313225f88406e5a55909c5aff5269a")
      {ct, tag} = AesModes.gcm_encrypt(pt, key, iv, <<>>)
      <<first_byte, rest::binary>> = tag
      bad_tag = <<bxor(first_byte, 1)>> <> rest
      assert {:error, _} = AesModes.gcm_decrypt(ct, key, iv, <<>>, bad_tag)
    end

    test "rejects tampered AAD" do
      key = h("feffe9928665731c6d6a8f9467308308")
      iv = h("cafebabefacedbaddecaf888")
      {ct, tag} = AesModes.gcm_encrypt("test", key, iv, "authentic")
      assert {:error, _} = AesModes.gcm_decrypt(ct, key, iv, "tampered", tag)
    end

    test "round-trips empty plaintext with AAD" do
      key = h("feffe9928665731c6d6a8f9467308308")
      iv = h("cafebabefacedbaddecaf888")
      aad_data = "authenticate this"
      {ct, tag} = AesModes.gcm_encrypt(<<>>, key, iv, aad_data)
      assert byte_size(ct) == 0
      assert {:ok, <<>>} == AesModes.gcm_decrypt(ct, key, iv, aad_data, tag)
    end

    test "round-trips various lengths" do
      key = h("feffe9928665731c6d6a8f9467308308")
      iv = h("cafebabefacedbaddecaf888")
      for len <- [1, 15, 16, 17, 31, 32, 100] do
        pt = :binary.copy("G", len)
        {ct, tag} = AesModes.gcm_encrypt(pt, key, iv, "aad")
        assert {:ok, ^pt} = AesModes.gcm_decrypt(ct, key, iv, "aad", tag)
      end
    end
  end
end
