# ============================================================================
# Tests for Ed25519 (RFC 8032)
# ============================================================================
# Test vectors verified against libsodium (PyNaCl) and the RFC 8032 appendix
# reference implementation.

defmodule CodingAdventures.Ed25519Test do
  use ExUnit.Case, async: true

  alias CodingAdventures.Ed25519

  # Helper to decode hex strings to binary.
  defp from_hex(hex), do: Ed25519.from_hex(hex)
  defp to_hex(bin), do: Ed25519.to_hex(bin)

  # ==========================================================================
  # RFC 8032 Test Vectors (verified against libsodium)
  # ==========================================================================

  describe "RFC 8032 test vectors" do
    # ----------------------------------------------------------------------
    # Test 1: Empty message
    # ----------------------------------------------------------------------
    test "signs and verifies empty message (test vector 1)" do
      seed = from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4" <>
        "4449c5697b326919703bac031cae7f60"
      )
      expected_pub = "d75a980182b10ab7d54bfed3c964073a" <>
        "0ee172f3daa62325af021a68f707511a"
      expected_sig = "e5564300c360ac729086e2cc806e828a" <>
        "84877f1eb8e5d974d873e06522490155" <>
        "5fb8821590a33bacc61e39701cf9b46b" <>
        "d25bf5f0595bbe24655141438e7a100b"

      {pub, sk} = Ed25519.generate_keypair(seed)
      assert to_hex(pub) == expected_pub

      sig = Ed25519.sign("", sk)
      assert to_hex(sig) == expected_sig

      assert Ed25519.verify("", sig, pub)
    end

    # ----------------------------------------------------------------------
    # Test 2: One byte (0x72)
    # ----------------------------------------------------------------------
    test "signs and verifies one-byte message (test vector 2)" do
      seed = from_hex(
        "4ccd089b28ff96da9db6c346ec114e0f" <>
        "5b8a319f35aba624da8cf6ed4fb8a6fb"
      )
      expected_pub = "3d4017c3e843895a92b70aa74d1b7ebc" <>
        "9c982ccf2ec4968cc0cd55f12af4660c"
      expected_sig = "92a009a9f0d4cab8720e820b5f642540" <>
        "a2b27b5416503f8fb3762223ebdb69da" <>
        "085ac1e43e15996e458f3613d0f11d8c" <>
        "387b2eaeb4302aeeb00d291612bb0c00"
      message = from_hex("72")

      {pub, sk} = Ed25519.generate_keypair(seed)
      assert to_hex(pub) == expected_pub

      sig = Ed25519.sign(message, sk)
      assert to_hex(sig) == expected_sig

      assert Ed25519.verify(message, sig, pub)
    end

    # ----------------------------------------------------------------------
    # Test 3: Two bytes (0xaf82)
    # ----------------------------------------------------------------------
    test "signs and verifies two-byte message (test vector 3)" do
      seed = from_hex(
        "c5aa8df43f9f837bedb7442f31dcb7b1" <>
        "66d38535076f094b85ce3a2e0b4458f7"
      )
      expected_pub = "fc51cd8e6218a1a38da47ed00230f058" <>
        "0816ed13ba3303ac5deb911548908025"
      expected_sig = "6291d657deec24024827e69c3abe01a3" <>
        "0ce548a284743a445e3680d7db5ac3ac" <>
        "18ff9b538d16f290ae67f760984dc659" <>
        "4a7c15e9716ed28dc027beceea1ec40a"
      message = from_hex("af82")

      {pub, sk} = Ed25519.generate_keypair(seed)
      assert to_hex(pub) == expected_pub

      sig = Ed25519.sign(message, sk)
      assert to_hex(sig) == expected_sig

      assert Ed25519.verify(message, sig, pub)
    end

    # ----------------------------------------------------------------------
    # Test 4: 1023 bytes
    # ----------------------------------------------------------------------
    test "signs and verifies 1023-byte message (test vector 4)" do
      seed = from_hex(
        "f5e5767cf153319517630f226876b86c" <>
        "8160cc583bc013744c6bf255f5cc0ee5"
      )
      expected_pub = "278117fc144c72340f67d0f2316e8386" <>
        "ceffbf2b2428c9c51fef7c597f1d426e"
      expected_sig = "d686294b743c6760c6a78a2c4c2fc761" <>
        "15c2600b8f083acde59e7cee32578c0f" <>
        "59ea4219ab9b5896795e4e2b87a30270" <>
        "aa0e3099eee944e9e67a1b22df41ff07"
      message = from_hex(
        "08b8b2b733424243760fe426a4b54908" <>
        "632110a66c2f6591eabd3345e3e4eb98" <>
        "fa6e264bf09efe12ee50f8f54e9f77b1" <>
        "e355f6c50544e23fb1433ddf73be84d8" <>
        "79de7c0046dc4996d9e773f4bc9efe57" <>
        "38829adb26c81b37c93a1b270b20329d" <>
        "658675fc6ea534e0810a4432826bf58c" <>
        "941efb65d57a338bbd2e26640f89ffbc" <>
        "1a858efcb8550ee3a5e1998bd177e93a" <>
        "7363c344fe6b199ee5d02e82d522c4fe" <>
        "ba15452f80288a821a579116ec6dad2b" <>
        "3b310da903401aa62100ab5d1a36553e" <>
        "06203b33890cc9b832f79ef80560ccb9" <>
        "a39ce767967ed628c6ad573cb116dbef" <>
        "fefd75499da96bd68a8a97b928a8bbc1" <>
        "03b6621fcde2beca1231d206be6cd9ec" <>
        "7aff6f6c94fcd7204ed3455c68c83f4a" <>
        "41da4af2b74ef5c53f1d8ac70bdcb7ed" <>
        "185ce81bd84359d44254d95629e9855a" <>
        "94a7c1958d1f8ada5d0532ed8a5aa3fb" <>
        "2d17ba70eb6248e594e1a2297acbbb39" <>
        "d502f1a8c6eb6f1ce22b3de1a1f40cc2" <>
        "4554119a831a9aad6079cad88425de6b" <>
        "de1a9187ebb6092cf67bf2b13fd65f27" <>
        "088d78b7e883c8759d2c4f5c65adb755" <>
        "3878ad575f9fad878e80a0c9ba63bcbc" <>
        "c2732e69485bbc9c90bfbd62481d9089" <>
        "beccf80cfe2df16a2cf65bd92dd597b0" <>
        "7e0917af48bbb75fed413d238f5555a7" <>
        "a569d80c3414a8d0859dc65a46128bab" <>
        "27af87a71314f318c782b23ebfe808b8" <>
        "2b0ce26401d2e22f04d83d1255dc51ad" <>
        "dd3b75a2b1ae0784504df543af8969be" <>
        "3ea7082ff7fc9888c144da2af58429ec" <>
        "96031dbcad3dad9af0dcbaaaf268cb8f" <>
        "cffead94f3c7ca495e056a9b47acdb75" <>
        "1fb73e666c6c655ade8297297d07ad1b" <>
        "a5e43f1bca32301651339e22904cc8c4" <>
        "2f58c30c04aafdb038dda0847dd988dc" <>
        "da6f3bfd15c4b4c4525004aa06eeff8c" <>
        "a61783aacec57fb3d1f92b0fe2fd1a85" <>
        "f6724517b65e614ad6808d6f6ee34dff" <>
        "7310fdc82aebfd904b01e1dc54b29270" <>
        "94b2db68d6f903b68401adebf5a7e08d" <>
        "78ff4ef5d63653a65040cf9bfd4aca79" <>
        "84a74d37145986780fc0b16ac451649d" <>
        "e6188a7dbdf191f64b5fc5e2ab47b57f" <>
        "7f7276cd419c17a3ca8e1b939ae49e48" <>
        "8acba6b965610b5480109c8b17b80e1b" <>
        "7b750dfc7598d5d5011fd2dcc5600a32" <>
        "ef5b52a1ecc820e308aa342721aac094" <>
        "3bf6686b64b2579376504ccc493d97e6" <>
        "aed3fb0f9cd71a43dd497f01f17c0e2c" <>
        "b3797aa2a2f256656168e6c496afc5fb" <>
        "93246f6b1116398a346f1a641f3b041e" <>
        "989f7914f90cc2c7fff357876e506b50" <>
        "d334ba77c225bc307ba537152f3f1610" <>
        "e4eafe595f6d9d90d11faa933a15ef13" <>
        "69546868a7f3a45a96768d40fd9d0341" <>
        "2c091c6315cf4fde7cb68606937380db" <>
        "2eaaa707b4c4185c32eddcdd306705e4" <>
        "dc1ffc872eeee475a64dfac86aba41c0" <>
        "618983f8741c5ef68d3a101e8a3b8cac" <>
        "60c905c15fc910840b94c00a0b9d00"
      )

      {pub, sk} = Ed25519.generate_keypair(seed)
      assert to_hex(pub) == expected_pub

      sig = Ed25519.sign(message, sk)
      assert to_hex(sig) == expected_sig

      assert Ed25519.verify(message, sig, pub)
    end
  end

  # ==========================================================================
  # Verification Failure Tests
  # ==========================================================================

  describe "verification edge cases" do
    test "rejects signature with wrong message" do
      seed = from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4" <>
        "4449c5697b326919703bac031cae7f60"
      )
      {pub, sk} = Ed25519.generate_keypair(seed)
      sig = Ed25519.sign("hello", sk)
      refute Ed25519.verify("world", sig, pub)
    end

    test "rejects signature with wrong public key" do
      seed1 = from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4" <>
        "4449c5697b326919703bac031cae7f60"
      )
      seed2 = from_hex(
        "4ccd089b28ff96da9db6c346ec114e0f" <>
        "5b8a319f35aba624da8cf6ed4fb8a6fb"
      )
      {_pub1, sk1} = Ed25519.generate_keypair(seed1)
      {pub2, _sk2} = Ed25519.generate_keypair(seed2)
      sig = Ed25519.sign("hello", sk1)
      refute Ed25519.verify("hello", sig, pub2)
    end

    test "rejects tampered signature" do
      seed = from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4" <>
        "4449c5697b326919703bac031cae7f60"
      )
      {pub, sk} = Ed25519.generate_keypair(seed)
      sig = Ed25519.sign("hello", sk)
      # Flip a bit in the first byte
      <<first_byte, rest::binary>> = sig
      tampered = <<Bitwise.bxor(first_byte, 1), rest::binary>>
      refute Ed25519.verify("hello", tampered, pub)
    end

    test "rejects invalid signature length" do
      seed = from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4" <>
        "4449c5697b326919703bac031cae7f60"
      )
      {pub, _sk} = Ed25519.generate_keypair(seed)
      refute Ed25519.verify("hello", "short", pub)
    end

    test "rejects invalid public key length" do
      seed = from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4" <>
        "4449c5697b326919703bac031cae7f60"
      )
      {_pub, sk} = Ed25519.generate_keypair(seed)
      sig = Ed25519.sign("hello", sk)
      refute Ed25519.verify("hello", sig, "short")
    end
  end

  # ==========================================================================
  # Key Generation Tests
  # ==========================================================================

  describe "key generation" do
    test "produces 32-byte public key" do
      seed = from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4" <>
        "4449c5697b326919703bac031cae7f60"
      )
      {pub, _sk} = Ed25519.generate_keypair(seed)
      assert byte_size(pub) == 32
    end

    test "produces 64-byte secret key" do
      seed = from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4" <>
        "4449c5697b326919703bac031cae7f60"
      )
      {_pub, sk} = Ed25519.generate_keypair(seed)
      assert byte_size(sk) == 64
    end

    test "secret key starts with seed and ends with public key" do
      seed = from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4" <>
        "4449c5697b326919703bac031cae7f60"
      )
      {pub, sk} = Ed25519.generate_keypair(seed)
      <<sk_seed::binary-size(32), sk_pub::binary-size(32)>> = sk
      assert sk_seed == seed
      assert sk_pub == pub
    end
  end

  # ==========================================================================
  # Sign/Verify Round-Trip Tests
  # ==========================================================================

  describe "round-trip" do
    test "sign and verify with various message lengths" do
      seed = from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4" <>
        "4449c5697b326919703bac031cae7f60"
      )
      {pub, sk} = Ed25519.generate_keypair(seed)

      # Empty message
      sig = Ed25519.sign("", sk)
      assert Ed25519.verify("", sig, pub)

      # Short message
      sig = Ed25519.sign("test", sk)
      assert Ed25519.verify("test", sig, pub)

      # Longer message
      long_msg = String.duplicate("a", 256)
      sig = Ed25519.sign(long_msg, sk)
      assert Ed25519.verify(long_msg, sig, pub)
    end

    test "produces deterministic signatures" do
      seed = from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4" <>
        "4449c5697b326919703bac031cae7f60"
      )
      {_pub, sk} = Ed25519.generate_keypair(seed)

      sig1 = Ed25519.sign("hello", sk)
      sig2 = Ed25519.sign("hello", sk)
      assert sig1 == sig2
    end

    test "produces 64-byte signatures" do
      seed = from_hex(
        "9d61b19deffd5a60ba844af492ec2cc4" <>
        "4449c5697b326919703bac031cae7f60"
      )
      {_pub, sk} = Ed25519.generate_keypair(seed)
      sig = Ed25519.sign("hello", sk)
      assert byte_size(sig) == 64
    end
  end

  # ==========================================================================
  # Hex Utility Tests
  # ==========================================================================

  describe "hex utilities" do
    test "from_hex decodes correctly" do
      assert Ed25519.from_hex("48656c6c6f") == "Hello"
    end

    test "to_hex encodes correctly" do
      assert Ed25519.to_hex("Hello") == "48656c6c6f"
    end

    test "round-trip hex encoding" do
      original = <<0, 1, 127, 128, 255>>
      assert Ed25519.from_hex(Ed25519.to_hex(original)) == original
    end
  end
end
