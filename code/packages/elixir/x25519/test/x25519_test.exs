defmodule CodingAdventures.X25519Test do
  use ExUnit.Case, async: true

  alias CodingAdventures.X25519

  # Helper: convert a hex string to a 32-byte binary.
  defp hex_to_bytes(hex) do
    Base.decode16!(hex, case: :lower)
  end

  # Helper: convert a 32-byte binary to a lowercase hex string.
  defp bytes_to_hex(bytes) do
    Base.encode16(bytes, case: :lower)
  end

  # ---------------------------------------------------------------------------
  # RFC 7748 Test Vector 1
  # ---------------------------------------------------------------------------
  describe "RFC 7748 test vector 1" do
    test "scalar multiplication produces correct output" do
      scalar = hex_to_bytes("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4")
      u = hex_to_bytes("e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c")
      expected = "c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552"

      result = X25519.x25519(scalar, u)
      assert bytes_to_hex(result) == expected
    end
  end

  # ---------------------------------------------------------------------------
  # RFC 7748 Test Vector 2
  # ---------------------------------------------------------------------------
  describe "RFC 7748 test vector 2" do
    test "scalar multiplication produces correct output" do
      scalar = hex_to_bytes("4b66e9d4d1b4673c5ad22691957d6af5c11b6421e0ea01d42ca4169e7918ba0d")
      u = hex_to_bytes("e5210f12786811d3f4b7959d0538ae2c31dbe7106fc03c3efc4cd549c715a493")
      expected = "95cbde9476e8907d7aade45cb4b873f88b595a68799fa152e6f8f7647aac7957"

      result = X25519.x25519(scalar, u)
      assert bytes_to_hex(result) == expected
    end
  end

  # ---------------------------------------------------------------------------
  # Base Point Multiplication (Alice's public key)
  # ---------------------------------------------------------------------------
  describe "base point multiplication (Alice)" do
    test "generates Alice's public key from her private key" do
      alice_private = hex_to_bytes("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
      expected = "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a"

      result = X25519.x25519_base(alice_private)
      assert bytes_to_hex(result) == expected
    end
  end

  # ---------------------------------------------------------------------------
  # Base Point Multiplication (Bob's public key)
  # ---------------------------------------------------------------------------
  describe "base point multiplication (Bob)" do
    test "generates Bob's public key from his private key" do
      bob_private = hex_to_bytes("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb")
      expected = "de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f"

      result = X25519.x25519_base(bob_private)
      assert bytes_to_hex(result) == expected
    end
  end

  # ---------------------------------------------------------------------------
  # Diffie-Hellman Shared Secret
  # ---------------------------------------------------------------------------
  describe "Diffie-Hellman shared secret" do
    test "Alice and Bob compute the same shared secret" do
      alice_private = hex_to_bytes("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
      bob_private = hex_to_bytes("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb")

      alice_public = X25519.x25519_base(alice_private)
      bob_public = X25519.x25519_base(bob_private)

      # Alice computes shared secret using her private key and Bob's public key
      shared_alice = X25519.x25519(alice_private, bob_public)

      # Bob computes shared secret using his private key and Alice's public key
      shared_bob = X25519.x25519(bob_private, alice_public)

      expected = "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742"

      assert bytes_to_hex(shared_alice) == expected
      assert bytes_to_hex(shared_bob) == expected
      assert shared_alice == shared_bob
    end
  end

  # ---------------------------------------------------------------------------
  # generate_keypair/1
  # ---------------------------------------------------------------------------
  describe "generate_keypair/1" do
    test "returns the original private key and the derived public key" do
      alice_private = hex_to_bytes("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
      expected_public = "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a"

      {priv, pub} = X25519.generate_keypair(alice_private)

      assert priv == alice_private
      assert bytes_to_hex(pub) == expected_public
    end
  end

  # ---------------------------------------------------------------------------
  # Iterated Test (1 iteration)
  # ---------------------------------------------------------------------------
  describe "iterated test" do
    test "1 iteration starting from k = u = 9" do
      # Start with k = u = 9, encoded as 32-byte LE
      nine = <<9>> <> <<0::size(248)>>

      result = X25519.x25519(nine, nine)
      expected = "422c8e7a6227d7bca1350b3e2bb7279f7897b87bb6854b783c60e80311ae3079"

      assert bytes_to_hex(result) == expected
    end

    test "1000 iterations starting from k = u = 9" do
      nine = <<9>> <> <<0::size(248)>>

      # After 1000 iterations, k should be this value.
      # Each iteration: new_k = x25519(k, u), then u becomes old k.
      # So after the reduce, the tuple is {k_1000, k_999}.
      expected = "684cf59ba83309552800ef566f2f4d3c1c3887c49360e3875f2eb94d99532c51"

      {k_1000, _u} =
        Enum.reduce(1..1000, {nine, nine}, fn _i, {k, u} ->
          new_k = X25519.x25519(k, u)
          {new_k, k}
        end)

      assert bytes_to_hex(k_1000) == expected
    end
  end

  # ---------------------------------------------------------------------------
  # Edge Cases
  # ---------------------------------------------------------------------------
  describe "edge cases" do
    test "input validation requires 32-byte scalar" do
      assert_raise FunctionClauseError, fn ->
        X25519.x25519(<<1, 2, 3>>, <<0::size(256)>>)
      end
    end

    test "input validation requires 32-byte u-coordinate" do
      assert_raise FunctionClauseError, fn ->
        X25519.x25519(<<0::size(256)>>, <<1, 2, 3>>)
      end
    end
  end
end
