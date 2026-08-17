# frozen_string_literal: true

require "base64"
require "json"
require "minitest/autorun"
require "coding_adventures_chief_of_staff_channel_crypto"

# Tests for the receiver-key-free D18G authenticity check.
#
# D18T plan validation requires verifying the originator signature on every
# receiver's grant using only the channel definition's public key. An
# originator holds no receiver private keys, so it cannot open the grants it
# just sealed -- which is why this weaker check has to exist separately.
class TestVerifyGrantSignature < Minitest::Test
  Crypto = CodingAdventures::ChiefOfStaffChannelCrypto
  FIXTURE_PATH = File.expand_path(
    "../../../../fixtures/chief-of-staff-channel-key-grant/v1/manifest.json", __dir__
  )
  FIXTURE = JSON.parse(File.read(FIXTURE_PATH, encoding: "UTF-8"))
  ORIGINATOR_PUBLIC_KEY = [FIXTURE.dig("test_signing_key", "public_key_hex")].pack("H*")

  # The failures reachable without a receiver key. Everything else in the
  # manifest's opening_negative_cases -- invalid_key_agreement and the whole
  # authentication_failed family -- happens strictly AFTER signature
  # verification, in the X25519 agreement or the AEAD open, and must therefore
  # VERIFY SUCCESSFULLY here.
  #
  # That is the sharpest statement these tests make: the receiver-key-free
  # entry point stops at exactly the boundary where the receiver key becomes
  # necessary, no earlier and no later.
  PRE_UNWRAP_CODES = %w[
    unexpected_originator unexpected_receiver unexpected_channel invalid_signature
  ].freeze

  def decode(value) = Base64.strict_decode64(value)
  def from_hex(value) = [value].pack("H*")

  def assert_grant_error(code)
    error = assert_raises(Crypto::KeyGrantProfileError) { yield }
    assert_equal code, error.code
  end

  def test_accepts_every_positive_fixture_using_only_public_inputs
    cases = FIXTURE.fetch("positive_cases")
    refute_empty cases
    cases.each do |testcase|
      grant = Crypto.grant_deserialize(decode(testcase.fetch("d18g_b64")))
      # Note what is absent: receiver_private_key_hex is never read.
      assert_nil Crypto.verify_grant_signature(
        grant,
        decode(testcase.fetch("originator_id_b64")),
        decode(testcase.fetch("receiver_id_b64")),
        from_hex(testcase.fetch("channel_id_hex")),
        ORIGINATOR_PUBLIC_KEY
      ), "#{testcase.fetch('name')} must verify from public inputs alone"
    end
  end

  def test_stops_exactly_at_the_receiver_key_boundary
    seen_pre = 0
    seen_post = 0
    FIXTURE.fetch("opening_negative_cases").each do |testcase|
      grant = Crypto.grant_deserialize(decode(testcase.fetch("d18g_b64")))
      arguments = [
        grant,
        decode(testcase.fetch("expected_originator_id_b64")),
        decode(testcase.fetch("expected_receiver_id_b64")),
        from_hex(testcase.fetch("expected_channel_id_hex")),
        ORIGINATOR_PUBLIC_KEY
      ]
      expected = testcase.fetch("expected_error")
      if PRE_UNWRAP_CODES.include?(expected)
        seen_pre += 1
        assert_grant_error(expected) { Crypto.verify_grant_signature(*arguments) }
      else
        seen_post += 1
        assert_nil Crypto.verify_grant_signature(*arguments),
                   "#{testcase.fetch('name')} fails only while unwrapping, so signature " \
                   "verification must succeed (expected #{expected})"
      end
    end
    # Guard against a manifest that loses one side of the split and leaves half
    # this test vacuous.
    refute_equal 0, seen_pre
    refute_equal 0, seen_post
  end

  def test_agrees_with_open_channel_key_grant_on_every_pre_unwrap_fixture
    FIXTURE.fetch("opening_negative_cases").each do |testcase|
      expected = testcase.fetch("expected_error")
      next unless PRE_UNWRAP_CODES.include?(expected)

      grant = Crypto.grant_deserialize(decode(testcase.fetch("d18g_b64")))
      originator = decode(testcase.fetch("expected_originator_id_b64"))
      receiver = decode(testcase.fetch("expected_receiver_id_b64"))
      channel = from_hex(testcase.fetch("expected_channel_id_hex"))
      receiver_key = Crypto::ReceiverKeyPair.from_private_key(
        from_hex(testcase.fetch("receiver_private_key_hex"))
      )

      # Both entry points share verify_grant_bindings, so they can never
      # disagree on a pre-unwrap failure. This makes that structural claim
      # observable rather than merely asserted in a comment.
      assert_grant_error(expected) do
        Crypto.verify_grant_signature(grant, originator, receiver, channel, ORIGINATOR_PUBLIC_KEY)
      end
      assert_grant_error(expected) do
        Crypto.open_channel_key_grant(grant, originator, receiver, channel, receiver_key,
                                      ORIGINATOR_PUBLIC_KEY)
      end
      receiver_key.destroy
    end
  end

  def test_rejects_malformed_public_inputs
    testcase = FIXTURE.fetch("positive_cases").first
    grant = Crypto.grant_deserialize(decode(testcase.fetch("d18g_b64")))
    originator = decode(testcase.fetch("originator_id_b64"))
    receiver = decode(testcase.fetch("receiver_id_b64"))
    channel = from_hex(testcase.fetch("channel_id_hex"))

    [
      ["short-channel-id", channel.byteslice(0, 15), ORIGINATOR_PUBLIC_KEY],
      ["long-channel-id", "#{channel}\0", ORIGINATOR_PUBLIC_KEY],
      ["empty-channel-id", "".b, ORIGINATOR_PUBLIC_KEY],
      ["short-public-key", channel, ORIGINATOR_PUBLIC_KEY.byteslice(0, 31)],
      ["long-public-key", channel, "#{ORIGINATOR_PUBLIC_KEY}\0"],
      ["empty-public-key", channel, "".b]
    ].each do |name, bad_channel, bad_key|
      assert_raises(Crypto::KeyGrantProfileError, "#{name} must fail closed") do
        Crypto.verify_grant_signature(grant, originator, receiver, bad_channel, bad_key)
      end
    end
  end

  def test_rejects_another_originators_public_key
    testcase = FIXTURE.fetch("positive_cases").first
    grant = Crypto.grant_deserialize(decode(testcase.fetch("d18g_b64")))
    # A well-formed key belonging to somebody else must not verify. Without
    # this, a caller could be fooled by any 32 valid bytes.
    other = Crypto::OriginatorSigningKey.from_seed(("\0" * 32).b)
    assert_grant_error("invalid_signature") do
      Crypto.verify_grant_signature(
        grant,
        decode(testcase.fetch("originator_id_b64")),
        decode(testcase.fetch("receiver_id_b64")),
        from_hex(testcase.fetch("channel_id_hex")),
        other.public_key
      )
    end
    other.destroy
  end
end
