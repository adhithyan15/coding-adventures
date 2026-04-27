# frozen_string_literal: true

# Ed25519 Test Suite
#
# Tests against RFC 8032 Section 7.1 test vectors, verified against
# Node.js built-in Ed25519 implementation.

require "minitest/autorun"
require "coding_adventures_ed25519"

class TestEd25519 < Minitest::Test
  # ── Helper ─────────────────────────────────────────────────────────────

  def hex_to_bytes(hex)
    [hex].pack("H*")
  end

  def bytes_to_hex(bytes)
    bytes.unpack1("H*")
  end

  # ── RFC 8032 Test Vector 1: Empty message ──────────────────────────────

  def test_vector_1_empty_message
    seed = hex_to_bytes("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
    expected_pubkey = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
    expected_sig = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"

    public_key, secret_key = CodingAdventures::Ed25519.generate_keypair(seed)
    assert_equal expected_pubkey, bytes_to_hex(public_key)

    message = "".b
    signature = CodingAdventures::Ed25519.sign(message, secret_key)
    assert_equal expected_sig, bytes_to_hex(signature)

    assert CodingAdventures::Ed25519.verify(message, signature, public_key)
  end

  # ── RFC 8032 Test Vector 2: One byte (0x72) ────────────────────────────

  def test_vector_2_one_byte
    seed = hex_to_bytes("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb")
    expected_pubkey = "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c"
    expected_sig = "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00"

    public_key, secret_key = CodingAdventures::Ed25519.generate_keypair(seed)
    assert_equal expected_pubkey, bytes_to_hex(public_key)

    message = hex_to_bytes("72")
    signature = CodingAdventures::Ed25519.sign(message, secret_key)
    assert_equal expected_sig, bytes_to_hex(signature)

    assert CodingAdventures::Ed25519.verify(message, signature, public_key)
  end

  # ── RFC 8032 Test Vector 3: Two bytes (0xaf82) ─────────────────────────

  def test_vector_3_two_bytes
    seed = hex_to_bytes("c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7")
    expected_pubkey = "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025"
    expected_sig = "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a"

    public_key, secret_key = CodingAdventures::Ed25519.generate_keypair(seed)
    assert_equal expected_pubkey, bytes_to_hex(public_key)

    message = hex_to_bytes("af82")
    signature = CodingAdventures::Ed25519.sign(message, secret_key)
    assert_equal expected_sig, bytes_to_hex(signature)

    assert CodingAdventures::Ed25519.verify(message, signature, public_key)
  end

  # ── RFC 8032 Test Vector 4: 1023 bytes ─────────────────────────────────

  def test_vector_4_large_message
    seed = hex_to_bytes("f5e5767cf153319517630f226876b86c8160cc583bc013744c6bf255f5cc0ee5")
    expected_pubkey = "278117fc144c72340f67d0f2316e8386ceffbf2b2428c9c51fef7c597f1d426e"
    expected_sig = "d686294b743c6760c6a78a2c4c2fc76115c2600b8f083acde59e7cee32578c0f59ea4219ab9b5896795e4e2b87a30270aa0e3099eee944e9e67a1b22df41ff07"

    message = hex_to_bytes(
      "08b8b2b733424243760fe426a4b54908632110a66c2f6591eabd3345e3e4eb98" \
      "fa6e264bf09efe12ee50f8f54e9f77b1e355f6c50544e23fb1433ddf73be84d8" \
      "79de7c0046dc4996d9e773f4bc9efe5738829adb26c81b37c93a1b270b20329d" \
      "658675fc6ea534e0810a4432826bf58c941efb65d57a338bbd2e26640f89ffbc" \
      "1a858efcb8550ee3a5e1998bd177e93a7363c344fe6b199ee5d02e82d522c4fe" \
      "ba15452f80288a821a579116ec6dad2b3b310da903401aa62100ab5d1a36553e" \
      "06203b33890cc9b832f79ef80560ccb9a39ce767967ed628c6ad573cb116dbef" \
      "fefd75499da96bd68a8a97b928a8bbc103b6621fcde2beca1231d206be6cd9ec" \
      "7aff6f6c94fcd7204ed3455c68c83f4a41da4af2b74ef5c53f1d8ac70bdcb7ed" \
      "185ce81bd84359d44254d95629e9855a94a7c1958d1f8ada5d0532ed8a5aa3fb" \
      "2d17ba70eb6248e594e1a2297acbbb39d502f1a8c6eb6f1ce22b3de1a1f40cc2" \
      "4554119a831a9aad6079cad88425de6bde1a9187ebb6092cf67bf2b13fd65f27" \
      "088d78b7e883c8759d2c4f5c65adb7553878ad575f9fad878e80a0c9ba63bcbc" \
      "c2732e69485bbc9c90bfbd62481d9089beccf80cfe2df16a2cf65bd92dd597b0" \
      "7e0917af48bbb75fed413d238f5555a7a569d80c3414a8d0859dc65a46128bab" \
      "27af87a71314f318c782b23ebfe808b82b0ce26401d2e22f04d83d1255dc51ad" \
      "dd3b75a2b1ae0784504df543af8969be3ea7082ff7fc9888c144da2af58429ec" \
      "96031dbcad3dad9af0dcbaaaf268cb8fcffead94f3c7ca495e056a9b47acdb75" \
      "1fb73e666c6c655ade8297297d07ad1ba5e43f1bca32301651339e22904cc8c4" \
      "2f58c30c04aafdb038dda0847dd988dcda6f3bfd15c4b4c4525004aa06eeff8c" \
      "a61783aacec57fb3d1f92b0fe2fd1a85f6724517b65e614ad6808d6f6ee34dff" \
      "7310fdc82aebfd904b01e1dc54b2927094b2db68d6f903b68401adebf5a7e08d" \
      "78ff4ef5d63653a65040cf9bfd4aca7984a74d37145986780fc0b16ac451649d" \
      "e6188a7dbdf191f64b5fc5e2ab47b57f7f7276cd419c17a3ca8e1b939ae49e48" \
      "8acba6b965610b5480109c8b17b80e1b7b750dfc7598d5d5011fd2dcc5600a32" \
      "ef5b52a1ecc820e308aa342721aac0943bf6686b64b2579376504ccc493d97e6" \
      "aed3fb0f9cd71a43dd497f01f17c0e2cb3797aa2a2f256656168e6c496afc5fb" \
      "93246f6b1116398a346f1a641f3b041e989f7914f90cc2c7fff357876e506b50" \
      "d334ba77c225bc307ba537152f3f1610e4eafe595f6d9d90d11faa933a15ef13" \
      "69546868a7f3a45a96768d40fd9d03412c091c6315cf4fde7cb68606937380db" \
      "2eaaa707b4c4185c32eddcdd306705e4dc1ffc872eeee475a64dfac86aba41c0" \
      "618983f8741c5ef68d3a101e8a3b8cac60c905c15fc910840b94c00a0b9d00"
    )

    public_key, secret_key = CodingAdventures::Ed25519.generate_keypair(seed)
    assert_equal expected_pubkey, bytes_to_hex(public_key)

    signature = CodingAdventures::Ed25519.sign(message, secret_key)
    assert_equal expected_sig, bytes_to_hex(signature)

    assert CodingAdventures::Ed25519.verify(message, signature, public_key)
  end

  # ── Verification Failure Tests ─────────────────────────────────────────

  def test_rejects_tampered_message
    seed = hex_to_bytes("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
    public_key, secret_key = CodingAdventures::Ed25519.generate_keypair(seed)
    message = "Hello".b
    signature = CodingAdventures::Ed25519.sign(message, secret_key)

    refute CodingAdventures::Ed25519.verify("Hello!".b, signature, public_key)
  end

  def test_rejects_wrong_public_key
    seed1 = hex_to_bytes("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
    seed2 = hex_to_bytes("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb")
    pub1, sec1 = CodingAdventures::Ed25519.generate_keypair(seed1)
    pub2, _sec2 = CodingAdventures::Ed25519.generate_keypair(seed2)

    message = "Hello".b
    signature = CodingAdventures::Ed25519.sign(message, sec1)

    refute CodingAdventures::Ed25519.verify(message, signature, pub2)
  end

  def test_rejects_tampered_signature_r
    seed = hex_to_bytes("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
    public_key, secret_key = CodingAdventures::Ed25519.generate_keypair(seed)
    message = "Hello".b
    signature = CodingAdventures::Ed25519.sign(message, secret_key)

    bad_sig = signature.dup
    bad_sig.setbyte(0, bad_sig.getbyte(0) ^ 1)
    refute CodingAdventures::Ed25519.verify(message, bad_sig, public_key)
  end

  def test_rejects_tampered_signature_s
    seed = hex_to_bytes("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
    public_key, secret_key = CodingAdventures::Ed25519.generate_keypair(seed)
    message = "Hello".b
    signature = CodingAdventures::Ed25519.sign(message, secret_key)

    bad_sig = signature.dup
    bad_sig.setbyte(32, bad_sig.getbyte(32) ^ 1)
    refute CodingAdventures::Ed25519.verify(message, bad_sig, public_key)
  end

  def test_rejects_wrong_length_signature
    seed = hex_to_bytes("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
    public_key, _secret_key = CodingAdventures::Ed25519.generate_keypair(seed)
    message = "Hello".b

    refute CodingAdventures::Ed25519.verify(message, "\x00" * 63, public_key)
    refute CodingAdventures::Ed25519.verify(message, "\x00" * 65, public_key)
  end

  def test_rejects_wrong_length_public_key
    seed = hex_to_bytes("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
    _public_key, secret_key = CodingAdventures::Ed25519.generate_keypair(seed)
    message = "Hello".b
    signature = CodingAdventures::Ed25519.sign(message, secret_key)

    refute CodingAdventures::Ed25519.verify(message, signature, "\x00" * 31)
    refute CodingAdventures::Ed25519.verify(message, signature, "\x00" * 33)
  end

  # ── Keypair Generation Tests ───────────────────────────────────────────

  def test_key_sizes
    seed = hex_to_bytes("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
    public_key, secret_key = CodingAdventures::Ed25519.generate_keypair(seed)
    assert_equal 32, public_key.bytesize
    assert_equal 64, secret_key.bytesize
  end

  def test_secret_key_structure
    seed = hex_to_bytes("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
    public_key, secret_key = CodingAdventures::Ed25519.generate_keypair(seed)
    assert_equal seed, secret_key[0, 32]
    assert_equal public_key, secret_key[32, 32]
  end

  def test_deterministic_keypair
    seed = hex_to_bytes("c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7")
    pub1, sec1 = CodingAdventures::Ed25519.generate_keypair(seed)
    pub2, sec2 = CodingAdventures::Ed25519.generate_keypair(seed)
    assert_equal pub1, pub2
    assert_equal sec1, sec2
  end

  # ── Round-Trip Tests ───────────────────────────────────────────────────

  def test_sign_verify_round_trip
    seed = hex_to_bytes("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
    public_key, secret_key = CodingAdventures::Ed25519.generate_keypair(seed)

    [0, 1, 2, 16, 64, 128, 256].each do |len|
      msg = (0...len).map { |i| i & 0xFF }.pack("C*")
      sig = CodingAdventures::Ed25519.sign(msg, secret_key)
      assert CodingAdventures::Ed25519.verify(msg, sig, public_key),
        "Failed to verify message of length #{len}"
    end
  end

  def test_deterministic_signature
    seed = hex_to_bytes("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
    _public_key, secret_key = CodingAdventures::Ed25519.generate_keypair(seed)
    msg = "\x01\x02\x03".b
    sig1 = CodingAdventures::Ed25519.sign(msg, secret_key)
    sig2 = CodingAdventures::Ed25519.sign(msg, secret_key)
    assert_equal sig1, sig2
  end
end
