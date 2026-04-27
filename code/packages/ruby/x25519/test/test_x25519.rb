# frozen_string_literal: true

# ============================================================================
# test_x25519.rb — Test suite for X25519 (RFC 7748)
# ============================================================================
#
# These tests verify the X25519 implementation against the official test
# vectors from RFC 7748 Section 6.1, plus the iterated test and the
# full Diffie-Hellman key exchange test.
# ============================================================================

require "minitest/autorun"
require_relative "../lib/coding_adventures_x25519"

class TestX25519 < Minitest::Test
  # -------------------------------------------------------------------------
  # Helper: convert hex string to byte array
  # -------------------------------------------------------------------------

  def hex_to_bytes(hex)
    [hex].pack("H*").bytes
  end

  # -------------------------------------------------------------------------
  # Helper: convert byte array to hex string
  # -------------------------------------------------------------------------

  def bytes_to_hex(bytes)
    bytes.map { |b| b.to_s(16).rjust(2, "0") }.join
  end

  # =========================================================================
  # RFC 7748 Section 6.1 — Test Vectors
  # =========================================================================

  # Test Vector 1 — generic scalar multiplication
  def test_rfc7748_vector_1
    scalar = hex_to_bytes("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4")
    u = hex_to_bytes("e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c")
    expected = "c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552"

    result = CodingAdventures::X25519.x25519(scalar, u)
    assert_equal expected, bytes_to_hex(result)
  end

  # Test Vector 2 — generic scalar multiplication
  def test_rfc7748_vector_2
    scalar = hex_to_bytes("4b66e9d4d1b4673c5ad22691957d6af5c11b6421e0ea01d42ca4169e7918ba0d")
    u = hex_to_bytes("e5210f12786811d3f4b7959d0538ae2c31dbe7106fc03c3efc4cd549c715a493")
    expected = "95cbde9476e8907d7aade45cb4b873f88b595a68799fa152e6f8f7647aac7957"

    result = CodingAdventures::X25519.x25519(scalar, u)
    assert_equal expected, bytes_to_hex(result)
  end

  # Alice's public key from base point multiplication
  def test_alice_public_key
    alice_private = hex_to_bytes("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
    expected = "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a"

    result = CodingAdventures::X25519.x25519_base(alice_private)
    assert_equal expected, bytes_to_hex(result)
  end

  # Bob's public key from base point multiplication
  def test_bob_public_key
    bob_private = hex_to_bytes("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb")
    expected = "de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f"

    result = CodingAdventures::X25519.x25519_base(bob_private)
    assert_equal expected, bytes_to_hex(result)
  end

  # Diffie-Hellman shared secret: both parties compute the same value
  def test_shared_secret
    alice_private = hex_to_bytes("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
    bob_private = hex_to_bytes("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb")

    alice_public = CodingAdventures::X25519.x25519_base(alice_private)
    bob_public = CodingAdventures::X25519.x25519_base(bob_private)

    alice_shared = CodingAdventures::X25519.x25519(alice_private, bob_public)
    bob_shared = CodingAdventures::X25519.x25519(bob_private, alice_public)

    expected = "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742"

    assert_equal expected, bytes_to_hex(alice_shared)
    assert_equal expected, bytes_to_hex(bob_shared)
    assert_equal bytes_to_hex(alice_shared), bytes_to_hex(bob_shared)
  end

  # generate_keypair is an alias for x25519_base
  def test_generate_keypair
    private_key = hex_to_bytes("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
    from_base = CodingAdventures::X25519.x25519_base(private_key)
    from_keypair = CodingAdventures::X25519.generate_keypair(private_key)
    assert_equal bytes_to_hex(from_base), bytes_to_hex(from_keypair)
  end

  # Iterated test: 1 iteration
  def test_iterated_1
    k = [9] + [0] * 31
    u = [9] + [0] * 31

    old_k = k.dup
    k = CodingAdventures::X25519.x25519(k, u)
    u = old_k

    assert_equal "422c8e7a6227d7bca1350b3e2bb7279f7897b87bb6854b783c60e80311ae3079",
      bytes_to_hex(k)
  end

  # Iterated test: 1000 iterations
  def test_iterated_1000
    k = [9] + [0] * 31
    u = [9] + [0] * 31

    1000.times do
      old_k = k.dup
      k = CodingAdventures::X25519.x25519(k, u)
      u = old_k
    end

    assert_equal "684cf59ba83309552800ef566f2f4d3c1c3887c49360e3875f2eb94d99532c51",
      bytes_to_hex(k)
  end

  # Input validation: wrong scalar length
  def test_invalid_scalar_length
    assert_raises(ArgumentError) do
      CodingAdventures::X25519.x25519(Array.new(16, 0), Array.new(32, 0))
    end
  end

  # Input validation: wrong u-coordinate length
  def test_invalid_u_length
    assert_raises(ArgumentError) do
      CodingAdventures::X25519.x25519(Array.new(32, 0), Array.new(16, 0))
    end
  end

  # Edge case: u = 0 produces all-zero output (should raise)
  def test_low_order_point
    scalar = hex_to_bytes("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4")
    u = Array.new(32, 0)

    assert_raises(RuntimeError) do
      CodingAdventures::X25519.x25519(scalar, u)
    end
  end

  # Edge case: u = 1 is a low-order point (produces all zeros, should raise)
  def test_u_equals_1_low_order
    scalar = hex_to_bytes("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4")
    u = [1] + [0] * 31

    assert_raises(RuntimeError) do
      CodingAdventures::X25519.x25519(scalar, u)
    end
  end
end
