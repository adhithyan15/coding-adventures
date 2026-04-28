defmodule CodingAdventures.ChaCha20Poly1305Test do
  use ExUnit.Case
  import Bitwise

  alias CodingAdventures.ChaCha20Poly1305, as: CC

  # Convenience: decode an uppercase or lowercase hex string to binary.
  # Note: this is a test-only helper, defined as a function (not a module attribute)
  # because module attributes are evaluated at compile time before functions are in scope.
  defp h(hex), do: Base.decode16!(String.upcase(hex))

  # ─── chacha20_block/3 ──────────────────────────────────────────────────────

  describe "chacha20_block/3" do
    # RFC 8439 §2.1.2 — official test vector for the ChaCha20 block function.
    # Key:   00 01 02 ... 1f  (32 bytes, sequential)
    # Nonce: 00 00 00 09 00 00 00 4a 00 00 00 00
    # Counter: 1
    test "RFC 8439 §2.1.2 test vector" do
      key = :binary.list_to_bin(Enum.to_list(0..31))
      nonce = h("000000090000004a00000000")

      expected =
        h(
          "10f1e7e4d13b5915500fdd1fa32071c4" <>
            "c7d1f4c733c068030422aa9ac3d46c4e" <>
            "d2826446079faa0914c2d705d98b02a2" <>
            "b5129cd1de164eb9cbd083e8a2503c4e"
        )

      assert CC.chacha20_block(key, 1, nonce) == expected
    end

    # A zero key and zero nonce at counter 0 should still produce a non-zero block
    # (the constants ensure the initial state is never all-zeros).
    test "zero key + zero nonce produces non-zero output" do
      key = :binary.copy(<<0>>, 32)
      nonce = :binary.copy(<<0>>, 12)
      blk = CC.chacha20_block(key, 0, nonce)
      assert byte_size(blk) == 64
      refute blk == :binary.copy(<<0>>, 64)
    end

    test "output is always 64 bytes" do
      key = :binary.copy(<<0>>, 32)
      nonce = :binary.copy(<<0>>, 12)
      assert byte_size(CC.chacha20_block(key, 0, nonce)) == 64
      assert byte_size(CC.chacha20_block(key, 1, nonce)) == 64
      assert byte_size(CC.chacha20_block(key, 0xFFFFFFFF, nonce)) == 64
    end

    test "different counters produce different output" do
      key = :binary.copy(<<0>>, 32)
      nonce = :binary.copy(<<0>>, 12)
      refute CC.chacha20_block(key, 0, nonce) == CC.chacha20_block(key, 1, nonce)
    end

    test "different nonces produce different output" do
      key = :binary.copy(<<0>>, 32)
      nonce0 = :binary.copy(<<0>>, 12)
      nonce1 = :binary.copy(<<1>>, 12)
      refute CC.chacha20_block(key, 0, nonce0) == CC.chacha20_block(key, 0, nonce1)
    end

    test "different keys produce different output" do
      key0 = :binary.copy(<<0>>, 32)
      key1 = :binary.copy(<<1>>, 32)
      nonce = :binary.copy(<<0>>, 12)
      refute CC.chacha20_block(key0, 0, nonce) == CC.chacha20_block(key1, 0, nonce)
    end
  end

  # ─── chacha20_encrypt/4 ───────────────────────────────────────────────────

  describe "chacha20_encrypt/4" do
    # RFC 8439 §2.4.2 — encryption test vector
    # Verified against Erlang OTP :crypto.crypto_one_time(:chacha20, ...) implementation.
    # Note: the ciphertext is 114 bytes (same length as the plaintext).
    test "RFC 8439 §2.4.2 encryption vector" do
      key = :binary.list_to_bin(Enum.to_list(0..31))
      nonce = h("000000000000004a00000000")

      plaintext =
        "Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it."

      expected_ct =
        h(
          "6e2e359a2568f98041ba0728dd0d6981" <>
            "e97e7aec1d4360c20a27afccfd9fae0b" <>
            "f91b65c5524733ab8f593dabcd62b357" <>
            "1639d624e65152ab8f530c359f0861d8" <>
            "07ca0dbf500d6a6156a38e088a22b65e" <>
            "52bc514d16ccf806818ce91ab7793736" <>
            "5af90bbf74a35be6b40b8eedf2785e42874d"
        )

      assert CC.chacha20_encrypt(plaintext, key, nonce, 1) == expected_ct
    end

    test "encrypting empty binary returns empty binary" do
      key = :binary.copy(<<0>>, 32)
      nonce = :binary.copy(<<0>>, 12)
      assert CC.chacha20_encrypt(<<>>, key, nonce) == <<>>
    end

    test "encrypt then encrypt again (XOR twice) returns original" do
      key = :crypto.strong_rand_bytes(32)
      nonce = :crypto.strong_rand_bytes(12)
      plaintext = "Hello, World!"
      ciphertext = CC.chacha20_encrypt(plaintext, key, nonce)
      recovered = CC.chacha20_encrypt(ciphertext, key, nonce)
      assert recovered == plaintext
    end

    test "output length equals input length" do
      key = :binary.copy(<<0>>, 32)
      nonce = :binary.copy(<<0>>, 12)

      for len <- [1, 15, 16, 63, 64, 65, 127, 128, 200] do
        pt = :binary.copy(<<0xAB>>, len)
        ct = CC.chacha20_encrypt(pt, key, nonce)
        assert byte_size(ct) == len, "expected #{len} bytes, got #{byte_size(ct)}"
      end
    end

    test "multi-block plaintext round-trips correctly" do
      key = :crypto.strong_rand_bytes(32)
      nonce = :crypto.strong_rand_bytes(12)
      pt = String.duplicate("The quick brown fox jumps over the lazy dog. ", 10)
      ct = CC.chacha20_encrypt(pt, key, nonce)
      assert CC.chacha20_encrypt(ct, key, nonce) == pt
    end

    test "default counter is 1" do
      key = :binary.copy(<<0>>, 32)
      nonce = :binary.copy(<<0>>, 12)
      pt = "test"
      assert CC.chacha20_encrypt(pt, key, nonce) == CC.chacha20_encrypt(pt, key, nonce, 1)
    end

    test "different counters produce different ciphertext" do
      key = :binary.copy(<<0>>, 32)
      nonce = :binary.copy(<<0>>, 12)
      pt = "same plaintext"
      refute CC.chacha20_encrypt(pt, key, nonce, 1) == CC.chacha20_encrypt(pt, key, nonce, 2)
    end
  end

  # ─── poly1305_mac/2 ───────────────────────────────────────────────────────

  describe "poly1305_mac/2" do
    # RFC 8439 §2.5.2 — official Poly1305 test vector
    test "RFC 8439 §2.5.2 test vector" do
      key = h("85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b")
      msg = "Cryptographic Forum Research Group"
      expected = h("a8061dc1305136c6c22b8baf0c0127a9")
      assert CC.poly1305_mac(msg, key) == expected
    end

    test "returns exactly 16 bytes" do
      key = :binary.copy(<<0>>, 32)
      assert byte_size(CC.poly1305_mac("hello", key)) == 16
      assert byte_size(CC.poly1305_mac("", key)) == 16
    end

    test "empty message produces a valid 16-byte tag" do
      key = :crypto.strong_rand_bytes(32)
      tag = CC.poly1305_mac("", key)
      assert byte_size(tag) == 16
    end

    test "different messages produce different tags (with same key)" do
      key = :crypto.strong_rand_bytes(32)
      tag1 = CC.poly1305_mac("message one", key)
      tag2 = CC.poly1305_mac("message two", key)
      refute tag1 == tag2
    end

    test "different keys produce different tags (with same message)" do
      msg = "same message"
      tag1 = CC.poly1305_mac(msg, :binary.copy(<<0x01>>, 32))
      tag2 = CC.poly1305_mac(msg, :binary.copy(<<0x02>>, 32))
      refute tag1 == tag2
    end

    test "message spanning multiple 16-byte chunks" do
      key = :crypto.strong_rand_bytes(32)
      # 33 bytes: two full chunks + 1 byte remainder
      msg = String.duplicate("A", 33)
      tag = CC.poly1305_mac(msg, key)
      assert byte_size(tag) == 16
    end
  end

  # ─── aead_encrypt/4 and aead_decrypt/5 ────────────────────────────────────

  describe "aead_encrypt/4 and aead_decrypt/5" do
    # RFC 8439 §2.8.2 test vectors — defined as functions (not module attributes)
    # to avoid the compile-time restriction on calling local functions from @attrs.
    defp rfc_key,
      do: h("808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f")

    defp rfc_nonce, do: h("070000004041424344454647")
    defp rfc_aad, do: h("50515253c0c1c2c3c4c5c6c7")

    defp rfc_plaintext,
      do:
        "Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it."

    defp rfc_expected_ct,
      do:
        h(
          "d31a8d34648e60db7b86afbc53ef7ec2" <>
            "a4aded51296e08fea9e2b5a736ee62d6" <>
            "3dbea45e8ca9671282fafb69da92728b" <>
            "1a71de0a9e060b2905d6a5b67ecd3b36" <>
            "92ddbd7f2d778b8c9803aee328091b58" <>
            "fab324e4fad675945585808b4831d7bc" <>
            "3ff4def08e4b7a9de576d26586cec64b6116"
        )

    defp rfc_expected_tag, do: h("1ae10b594f09e26a7e902ecbd0600691")

    test "RFC 8439 §2.8.2 — ciphertext matches" do
      {ct, _tag} = CC.aead_encrypt(rfc_plaintext(), rfc_key(), rfc_nonce(), rfc_aad())
      assert ct == rfc_expected_ct()
    end

    test "RFC 8439 §2.8.2 — tag matches" do
      {_ct, tag} = CC.aead_encrypt(rfc_plaintext(), rfc_key(), rfc_nonce(), rfc_aad())
      assert tag == rfc_expected_tag()
    end

    test "RFC 8439 §2.8.2 — full round trip" do
      key = rfc_key()
      nonce = rfc_nonce()
      aad = rfc_aad()
      pt = rfc_plaintext()
      {ct, tag} = CC.aead_encrypt(pt, key, nonce, aad)
      assert {:ok, pt} == CC.aead_decrypt(ct, key, nonce, aad, tag)
    end

    test "wrong tag returns :authentication_failed" do
      key = rfc_key()
      nonce = rfc_nonce()
      aad = rfc_aad()
      pt = rfc_plaintext()
      {ct, tag} = CC.aead_encrypt(pt, key, nonce, aad)
      # Flip the first byte of the tag
      <<first, rest_tag::binary>> = tag
      bad_tag = <<bxor(first, 1)>> <> rest_tag
      assert {:error, :authentication_failed} == CC.aead_decrypt(ct, key, nonce, aad, bad_tag)
    end

    test "tampered ciphertext returns :authentication_failed" do
      key = rfc_key()
      nonce = rfc_nonce()
      aad = rfc_aad()
      pt = rfc_plaintext()
      {ct, tag} = CC.aead_encrypt(pt, key, nonce, aad)
      <<first_ct, rest_ct::binary>> = ct
      bad_ct = <<bxor(first_ct, 0xFF)>> <> rest_ct
      assert {:error, :authentication_failed} == CC.aead_decrypt(bad_ct, key, nonce, aad, tag)
    end

    test "wrong AAD returns :authentication_failed" do
      key = rfc_key()
      nonce = rfc_nonce()
      pt = rfc_plaintext()
      {ct, tag} = CC.aead_encrypt(pt, key, nonce, rfc_aad())
      assert {:error, :authentication_failed} ==
               CC.aead_decrypt(ct, key, nonce, "wrong aad", tag)
    end

    test "empty plaintext round-trips" do
      key = :crypto.strong_rand_bytes(32)
      nonce = :crypto.strong_rand_bytes(12)
      {ct, tag} = CC.aead_encrypt("", key, nonce, "")
      assert {:ok, ""} == CC.aead_decrypt(ct, key, nonce, "", tag)
    end

    test "empty AAD round-trips" do
      key = :crypto.strong_rand_bytes(32)
      nonce = :crypto.strong_rand_bytes(12)
      pt = "secret message"
      {ct, tag} = CC.aead_encrypt(pt, key, nonce, "")
      assert {:ok, pt} == CC.aead_decrypt(ct, key, nonce, "", tag)
    end

    test "multi-block plaintext (>64 bytes) round-trips" do
      key = :crypto.strong_rand_bytes(32)
      nonce = :crypto.strong_rand_bytes(12)
      pt = String.duplicate("a", 200)
      {ct, tag} = CC.aead_encrypt(pt, key, nonce, "")
      assert {:ok, pt} == CC.aead_decrypt(ct, key, nonce, "", tag)
    end

    test "very large plaintext round-trips" do
      key = :crypto.strong_rand_bytes(32)
      nonce = :crypto.strong_rand_bytes(12)
      # 10 KB
      pt = :crypto.strong_rand_bytes(10_240)
      {ct, tag} = CC.aead_encrypt(pt, key, nonce, "header")
      assert {:ok, pt} == CC.aead_decrypt(ct, key, nonce, "header", tag)
    end

    test "ciphertext length equals plaintext length" do
      key = :crypto.strong_rand_bytes(32)
      nonce = :crypto.strong_rand_bytes(12)
      pt = "exactly right length"
      {ct, tag} = CC.aead_encrypt(pt, key, nonce, "")
      assert byte_size(ct) == byte_size(pt)
      assert byte_size(tag) == 16
    end

    test "different nonces produce different ciphertexts" do
      key = :crypto.strong_rand_bytes(32)
      pt = "same plaintext"
      aad = ""
      {ct1, _} = CC.aead_encrypt(pt, key, :crypto.strong_rand_bytes(12), aad)
      {ct2, _} = CC.aead_encrypt(pt, key, :crypto.strong_rand_bytes(12), aad)
      # Extremely unlikely to collide, but theoretically possible with random nonces
      # — in practice this test will always pass
      refute ct1 == ct2
    end

    test "all-zero tag returns :authentication_failed" do
      key = rfc_key()
      nonce = rfc_nonce()
      aad = rfc_aad()
      {ct, _tag} = CC.aead_encrypt(rfc_plaintext(), key, nonce, aad)
      zero_tag = :binary.copy(<<0>>, 16)
      assert {:error, :authentication_failed} ==
               CC.aead_decrypt(ct, key, nonce, aad, zero_tag)
    end

    test "plaintext boundary: exactly 64 bytes (one full keystream block)" do
      key = :crypto.strong_rand_bytes(32)
      nonce = :crypto.strong_rand_bytes(12)
      pt = :crypto.strong_rand_bytes(64)
      {ct, tag} = CC.aead_encrypt(pt, key, nonce, "")
      assert {:ok, pt} == CC.aead_decrypt(ct, key, nonce, "", tag)
    end

    test "plaintext boundary: 65 bytes (one block + one byte)" do
      key = :crypto.strong_rand_bytes(32)
      nonce = :crypto.strong_rand_bytes(12)
      pt = :crypto.strong_rand_bytes(65)
      {ct, tag} = CC.aead_encrypt(pt, key, nonce, "")
      assert {:ok, pt} == CC.aead_decrypt(ct, key, nonce, "", tag)
    end
  end
end
