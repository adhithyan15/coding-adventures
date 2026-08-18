defmodule CodingAdventures.ChiefOfStaffChannelEpochActivation.D18TFixturesTest do
  @moduledoc """
  Direct consumers of the canonical Rust-authored D18T manifest.

  These never regenerate expected bytes locally and never shell out to another
  language — that is the whole point of a shared fixture. If Elixir disagrees
  with Rust about a single octet, they fail.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.ChiefOfStaffChannelCrypto.KeyGrantProfile, as: Grants
  alias CodingAdventures.ChiefOfStaffChannelStore, as: Profile
  alias CodingAdventures.ChiefOfStaffChannelEpochActivation, as: Epoch
  alias CodingAdventures.ChiefOfStaffChannelEpochActivation.ActivationError
  alias CodingAdventures.ChiefOfStaffChannelEpochActivation.Store, as: EpochStore

  @fixture_path Path.expand(
                  "../../../../fixtures/chief-of-staff-channel-epoch-activation/v1/manifest.json",
                  __DIR__
                )
  @fixture_text File.read!(@fixture_path)
  @fixture Jason.decode!(@fixture_text)
  @channel_id Base.decode16!("018f47a09b6c7def923456789abcdef0", case: :lower)

  defp decode(value), do: Base.decode64!(value)
  defp from_hex(value), do: Base.decode16!(value, case: :lower)

  defp assert_code(code, operation) do
    error = assert_raise ActivationError, operation
    assert error.code == code
    error
  end

  test "manifest contract, roster, and secret boundary" do
    assert @fixture["fixture_format"] == "D18T-durable-epoch-activation-fixtures-v1"

    assert @fixture["spec"] ==
             "code/specs/D18T-chief-of-staff-durable-epoch-activation-profile.md"

    assert @fixture["warning"] =~ "Never log"

    assert @fixture["constants"] == %{
             "state_magic_ascii" => "D18S",
             "state_version" => "2",
             "plan_magic_ascii" => "D18T",
             "plan_version" => "1",
             "state_content_type" => Epoch.epoch_state_content_type(),
             "plan_content_type" => Epoch.activation_plan_content_type(),
             "max_cas_attempts" => Integer.to_string(Epoch.max_epoch_cas_attempts())
           }

    # The error roster is closed AND ordered. A gate that only checked
    # membership would not notice a reordering, and six languages index it.
    assert @fixture["stable_error_codes"] == Epoch.error_codes()

    assert Enum.map(@fixture["crash_replay_traces"], & &1["name"]) == [
             "after-custody-selection",
             "after-plan-write",
             "after-first-grant",
             "after-all-grants",
             "after-activation-cas"
           ]

    assert length(@fixture["race_traces"]) == 4
    assert length(@fixture["negative_scenarios"]) == 6

    # Rust guarantees erasure; the BEAM honestly cannot. The fixture records
    # Rust's claim, and Elixir must report its own rather than echo it.
    assert @fixture["secret_erasure_capability"] == "guaranteed"
    assert Epoch.secret_erasure_capability() == "not_enforceable"

    # Every labelled test-only secret must appear exactly once in the whole
    # manifest. A second occurrence would mean a secret leaked into a summary, a
    # public record, or an expected-error string.
    Enum.each(@fixture["test_only_secrets"], fn {name, secret} ->
      occurrences = length(String.split(@fixture_text, secret)) - 1
      assert occurrences == 1, "secret #{name} must appear exactly once, saw #{occurrences}"
    end)
  end

  test "exact v1 to v2 state migrations" do
    assert Enum.map(@fixture["state_migrations"], & &1["name"]) == ["no-pending", "pending-d18h"]

    Enum.each(@fixture["state_migrations"], fn vector ->
      v1 = Profile.state_deserialize(decode(vector["d18s_v1_b64"]), @channel_id)
      expected = decode(vector["d18s_v2_b64"])
      v2 = Epoch.epoch_state_deserialize(expected, @channel_id)

      assert v2.active_epoch == String.to_integer(vector["active_epoch"])
      assert v2.next_sequence == String.to_integer(vector["next_sequence"])
      assert v2.next_sequence == v1.next_sequence
      # Migration preserves the in-flight reservation exactly; it never clears a
      # publish that was already reserved, and never invents one.
      assert v2.pending_header == v1.pending_header
      assert Epoch.epoch_state_serialize(v2) == expected
    end)
  end

  test "consumes and re-encodes the canonical activation plan" do
    activation = @fixture["activation_case"]
    expected = decode(activation["plan_b64"])
    plan = Epoch.activation_plan_deserialize(expected)

    assert plan.channel_id == @channel_id
    assert {plan.base_epoch, plan.new_epoch, length(plan.receivers)} == {0, 1, 1}
    assert Epoch.activation_plan_serialize(plan) == expected

    assert Epoch.activation_plan_record_key(@channel_id, 1) == activation["plan_record_key"]
    assert activation["plan_content_type"] == Epoch.activation_plan_content_type()

    # Prospective revocation, stated as data: A is rotated out at epoch 1, so A
    # gets no new grant and keeps only epoch 0, while B keeps both.
    assert length(activation["grant_b64"]) == 1
    assert activation["receiver_a_new_grant"] == nil
    assert activation["receiver_a_retains_epochs"] == ["0"]
    assert activation["receiver_b_retains_epochs"] == ["0", "1"]
  end

  # The strongest fixture test here. It rebuilds the candidate from the labelled
  # test-only secrets using Elixir's own D18Q and D18T code and requires the
  # result to equal the bytes Rust authored — plan and grant alike.
  test "reproduces the Rust-authored plan and grant bytes" do
    secrets = @fixture["test_only_secrets"]
    signer = Grants.originator_signing_key_from_seed(from_hex(secrets["originator_signing_seed_hex"]))

    receiver_a_key =
      Grants.receiver_key_pair_from_private_key(from_hex(secrets["receiver_a_private_key_hex"]))

    receiver_b_key =
      Grants.receiver_key_pair_from_private_key(from_hex(secrets["receiver_b_private_key_hex"]))

    receiver_a = Profile.new_receiver!("receiver-a", Grants.receiver_public_key(receiver_a_key))
    receiver_b = Profile.new_receiver!("receiver-b", Grants.receiver_public_key(receiver_b_key))

    definition =
      Profile.new_definition!(
        @channel_id,
        Profile.new_originator!("originator", Grants.originator_public_key(signer)),
        [receiver_a, receiver_b],
        1_725_000_000_000_000_000,
        0
      )

    rotation =
      Grants.plan_rotation(
        "originator",
        @channel_id,
        0,
        Grants.channel_master_key_from_bytes(from_hex(secrets["next_cmk_hex"])),
        [
          Grants.rotation_receiver_with_material(
            receiver_b.agent_id,
            receiver_b.public_key,
            from_hex(secrets["ephemeral_private_key_hex"]),
            from_hex(secrets["wrapping_nonce_hex"])
          )
        ],
        signer
      )

    prepared = EpochStore.prepare_rotation_candidate(definition, 0, [receiver_b], rotation)
    public = prepared.public_preparation

    assert public.plan_bytes == decode(@fixture["activation_case"]["plan_b64"]),
           "Elixir produced different D18T plan bytes than the canonical Rust manifest"

    assert public.grants == Enum.map(@fixture["activation_case"]["grant_b64"], &decode/1),
           "Elixir produced different D18G bytes than the canonical Rust manifest"

    # The candidate redacts under inspection.
    assert inspect(prepared) =~ "PreparedEpoch"
    refute inspect(prepared) =~ "cmk"
  end

  test "rejects malformed state records" do
    canonical = decode(@fixture["state_migrations"] |> hd() |> Map.fetch!("d18s_v2_b64"))
    size = byte_size(canonical)

    [
      {"truncated", binary_part(canonical, 0, size - 1)},
      {"trailing-byte", canonical <> <<0>>},
      {"wrong-version", mutate(canonical, 4, 3)},
      {"unknown-pending-flag", mutate(canonical, size - 1, 2)},
      {"wrong-magic", mutate(canonical, 0, ?X)}
    ]
    |> Enum.each(fn {name, mutated} ->
      error =
        assert_raise ActivationError, fn -> Epoch.epoch_state_deserialize(mutated, @channel_id) end

      assert error.code == "corrupt_record", name
    end)
  end

  test "rejects non-canonical plans" do
    canonical = decode(@fixture["activation_case"]["plan_b64"])

    assert_code("corrupt_record", fn ->
      Epoch.activation_plan_deserialize(canonical <> <<0>>)
    end)

    # A two-receiver plan whose entries descend by receiver hash. The decoder
    # must reject it rather than silently canonicalize — which is exactly what
    # new_activation_plan!/4 alone would have done, since it sorts its input.
    descending =
      binary_part(canonical, 0, 37) <>
        <<2::unsigned-big-32>> <>
        :binary.copy(<<4>>, 32) <>
        :binary.copy(<<3>>, 32) <> :binary.copy(<<2>>, 32) <> :binary.copy(<<1>>, 32)

    assert_code("corrupt_record", fn -> Epoch.activation_plan_deserialize(descending) end)

    entry = Epoch.new_plan_entry!(:binary.copy(<<1>>, 32), :binary.copy(<<2>>, 32))
    other = Epoch.new_plan_entry!(:binary.copy(<<1>>, 32), :binary.copy(<<3>>, 32))

    # Two distinct receivers hashing to the same value is a collision, and D18T
    # treats a collision as invalid input rather than equal authorization.
    assert_code("corrupt_record", fn ->
      Epoch.new_activation_plan!(@channel_id, 0, 1, [entry, other])
    end)

    assert_code("corrupt_record", fn -> Epoch.new_activation_plan!(@channel_id, 0, 1, []) end)
    assert_code("corrupt_record", fn -> Epoch.new_activation_plan!(@channel_id, 0, 2, [entry]) end)

    # A 16-octet channel id that is not a real UUID v7 is rejected, matching
    # Rust and Python. Accepting it would mean two conforming implementations
    # disagreed about whether the same plan record is valid.
    assert_code("corrupt_record", fn ->
      Epoch.new_activation_plan!(mutate(@channel_id, 6, 0x4F), 0, 1, [entry])
    end)

    assert_code("corrupt_record", fn ->
      Epoch.new_activation_plan!(mutate(@channel_id, 8, 0x1F), 0, 1, [entry])
    end)
  end

  defp mutate(binary, index, byte) do
    binary_part(binary, 0, index) <>
      <<byte>> <> binary_part(binary, index + 1, byte_size(binary) - index - 1)
  end
end
