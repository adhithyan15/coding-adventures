defmodule CodingAdventures.ChiefOfStaffChannelCrypto.VerifyGrantSignatureTest do
  @moduledoc """
  Tests for the receiver-key-free D18G authenticity check.

  D18T plan validation requires verifying the originator signature on every
  receiver's grant using only the channel definition's public key. An originator
  holds no receiver private keys, so it cannot open the grants it just sealed —
  which is why this weaker check has to exist separately.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.ChiefOfStaffChannelCrypto.KeyGrantProfile, as: Grants
  alias Grants.ProfileError

  @fixture_path Path.expand(
                  "../../../../fixtures/chief-of-staff-channel-key-grant/v1/manifest.json",
                  __DIR__
                )
  @fixture @fixture_path |> File.read!() |> Jason.decode!()
  @public_key Base.decode16!(@fixture["test_signing_key"]["public_key_hex"], case: :lower)

  # The failures reachable without a receiver key. Everything else in the
  # manifest's opening_negative_cases — invalid_key_agreement and the whole
  # authentication_failed family — happens strictly AFTER signature
  # verification, in the X25519 agreement or the AEAD open, and must therefore
  # VERIFY SUCCESSFULLY here.
  #
  # That is the sharpest statement these tests make: the receiver-key-free entry
  # point stops at exactly the boundary where the receiver key becomes
  # necessary, no earlier and no later.
  @pre_unwrap_codes ~w(unexpected_originator unexpected_receiver unexpected_channel invalid_signature)

  defp decode(value), do: Base.decode64!(value)
  defp from_hex(value), do: Base.decode16!(value, case: :lower)

  defp assert_grant_error(code, operation) do
    error = assert_raise ProfileError, operation
    assert error.code == code
  end

  test "accepts every positive fixture using only public inputs" do
    cases = @fixture["positive_cases"]
    refute Enum.empty?(cases)

    Enum.each(cases, fn testcase ->
      grant = Grants.grant_deserialize(decode(testcase["d18g_b64"]))

      # Note what is absent: receiver_private_key_hex is never read.
      assert :ok ==
               Grants.verify_grant_signature(
                 grant,
                 decode(testcase["originator_id_b64"]),
                 decode(testcase["receiver_id_b64"]),
                 from_hex(testcase["channel_id_hex"]),
                 @public_key
               ),
             "#{testcase["name"]} must verify from public inputs alone"
    end)
  end

  test "stops exactly at the receiver-key boundary" do
    {seen_pre, seen_post} =
      Enum.reduce(@fixture["opening_negative_cases"], {0, 0}, fn testcase, {pre, post} ->
        grant = Grants.grant_deserialize(decode(testcase["d18g_b64"]))
        originator = decode(testcase["expected_originator_id_b64"])
        receiver = decode(testcase["expected_receiver_id_b64"])
        channel = from_hex(testcase["expected_channel_id_hex"])
        expected = testcase["expected_error"]

        if expected in @pre_unwrap_codes do
          assert_grant_error(expected, fn ->
            Grants.verify_grant_signature(grant, originator, receiver, channel, @public_key)
          end)

          {pre + 1, post}
        else
          assert :ok ==
                   Grants.verify_grant_signature(
                     grant,
                     originator,
                     receiver,
                     channel,
                     @public_key
                   ),
                 "#{testcase["name"]} fails only while unwrapping, so signature " <>
                   "verification must succeed (expected #{expected})"

          {pre, post + 1}
        end
      end)

    # Guard against a manifest that loses one side of the split and leaves half
    # this test vacuous.
    assert seen_pre > 0
    assert seen_post > 0
  end

  test "agrees with open_channel_key_grant on every pre-unwrap fixture" do
    @fixture["opening_negative_cases"]
    |> Enum.filter(&(&1["expected_error"] in @pre_unwrap_codes))
    |> Enum.each(fn testcase ->
      grant = Grants.grant_deserialize(decode(testcase["d18g_b64"]))
      originator = decode(testcase["expected_originator_id_b64"])
      receiver = decode(testcase["expected_receiver_id_b64"])
      channel = from_hex(testcase["expected_channel_id_hex"])
      expected = testcase["expected_error"]

      receiver_key =
        testcase["receiver_private_key_hex"]
        |> from_hex()
        |> Grants.receiver_key_pair_from_private_key()

      # Both entry points share verify_grant_bindings!, so they can never
      # disagree on a pre-unwrap failure. This makes that structural claim
      # observable rather than merely asserted in a comment.
      assert_grant_error(expected, fn ->
        Grants.verify_grant_signature(grant, originator, receiver, channel, @public_key)
      end)

      assert_grant_error(expected, fn ->
        Grants.open_channel_key_grant(
          grant,
          originator,
          receiver,
          channel,
          receiver_key,
          @public_key
        )
      end)

      Grants.destroy_receiver_key_pair(receiver_key)
    end)
  end

  test "rejects malformed public inputs" do
    testcase = hd(@fixture["positive_cases"])
    grant = Grants.grant_deserialize(decode(testcase["d18g_b64"]))
    originator = decode(testcase["originator_id_b64"])
    receiver = decode(testcase["receiver_id_b64"])
    channel = from_hex(testcase["channel_id_hex"])

    [
      {"short-channel-id", binary_part(channel, 0, 15), @public_key},
      {"long-channel-id", channel <> <<0>>, @public_key},
      {"empty-channel-id", <<>>, @public_key},
      {"short-public-key", channel, binary_part(@public_key, 0, 31)},
      {"long-public-key", channel, @public_key <> <<0>>},
      {"empty-public-key", channel, <<>>}
    ]
    |> Enum.each(fn {name, bad_channel, bad_key} ->
      error =
        assert_raise ProfileError, fn ->
          Grants.verify_grant_signature(grant, originator, receiver, bad_channel, bad_key)
        end

      assert error.code in Grants.error_codes(),
             "#{name} must fail closed with a stable code, got #{error.code}"
    end)
  end

  test "rejects another originator's public key" do
    testcase = hd(@fixture["positive_cases"])
    grant = Grants.grant_deserialize(decode(testcase["d18g_b64"]))

    # A well-formed key belonging to somebody else must not verify. Without
    # this, a caller could be fooled by any 32 valid bytes.
    other = Grants.originator_signing_key_from_seed(<<0::256>>)

    assert_grant_error("invalid_signature", fn ->
      Grants.verify_grant_signature(
        grant,
        decode(testcase["originator_id_b64"]),
        decode(testcase["receiver_id_b64"]),
        from_hex(testcase["channel_id_hex"]),
        Grants.originator_public_key(other)
      )
    end)

    Grants.destroy_originator_signing_key(other)
  end

  test "rejects a non-grant first argument" do
    assert_grant_error("invalid_field", fn ->
      Grants.verify_grant_signature(%{}, <<>>, <<>>, <<0::128>>, @public_key)
    end)
  end
end
