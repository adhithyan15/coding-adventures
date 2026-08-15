defmodule CodingAdventures.ChiefOfStaffChannelCrypto.D18QFixturesTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.X25519
  alias CodingAdventures.ChiefOfStaffChannelCrypto.KeyGrantProfile, as: Grants
  alias Grants.{PortableKeyGrant, ProfileError}

  @fixture_path Path.expand(
                  "../../../../fixtures/chief-of-staff-channel-key-grant/v1/manifest.json",
                  __DIR__
                )
  @fixture @fixture_path |> File.read!() |> Jason.decode!()
  @public_key Base.decode16!(@fixture["test_signing_key"]["public_key_hex"], case: :lower)
  @channel_id Base.decode16!(hd(@fixture["positive_cases"])["channel_id_hex"], case: :lower)

  defp decode(value), do: Base.decode64!(value)
  defp from_hex(value), do: Base.decode16!(value, case: :lower)
  defp integer(value), do: String.to_integer(value)

  defp signer do
    @fixture["test_signing_key"]["seed_hex"]
    |> from_hex()
    |> Grants.originator_signing_key_from_seed()
  end

  defp assert_grant_error(code, operation) do
    error = assert_raise ProfileError, operation
    assert error.code == code
    assert Exception.message(error) == code
  end

  defp assert_roster(cases, names, fields) do
    assert Enum.map(cases, & &1["name"]) == names
    expected = MapSet.new(fields)
    Enum.each(cases, fn item -> assert MapSet.new(Map.keys(item)) == expected end)
  end

  test "manifest topology, error vocabulary, and erasure capabilities are closed" do
    assert MapSet.new(Map.keys(@fixture)) ==
             MapSet.new(~w(
               fixture_format spec generator_blob_sha1 warning constants test_signing_key
               positive_cases structural_negative_cases truncated_prefix_recipe oversize_recipes
               field_negative_cases seal_negative_cases opening_negative_cases receiver_state_trace
               rotation_case secret_erasure_capabilities rust_secret_erasure_capability stable_error_codes
             ))

    assert @fixture["fixture_format"] == "D18Q-channel-key-grant-fixtures-v1"
    assert @fixture["spec"] == "code/specs/D18Q-chief-of-staff-channel-key-grant-profile.md"
    assert String.length(@fixture["generator_blob_sha1"]) == 40
    assert @fixture["warning"] =~ "test-only"
    assert @fixture["warning"] =~ "Never log"

    assert @fixture["constants"] == %{
             "key_grant_context_ascii" => "chief-channel-key-grant-v1",
             "key_wrap_context_ascii" => "chief-channel-key-wrap-v1",
             "max_identity_bytes" => "4096",
             "wire_magic_ascii" => "D18G",
             "wire_version" => "1"
           }

    assert Grants.error_codes() == @fixture["stable_error_codes"]
    assert @fixture["secret_erasure_capabilities"] == ~w(guaranteed best_effort not_enforceable)
    assert Grants.secret_erasure_capability() == "not_enforceable"
    assert @fixture["rust_secret_erasure_capability"] == "guaranteed"
    assert Grants.originator_public_key(signer()) == @public_key

    positives = @fixture["positive_cases"]

    assert Enum.map(positives, & &1["name"]) ==
             ~w(epoch-zero-receiver-a epoch-zero-receiver-b maximum-epoch-receiver-a)

    positive_fields = ~w(
      name originator_id_b64 receiver_id_b64 channel_id_hex key_epoch cmk_hex
      receiver_private_key_hex receiver_public_key_hex ephemeral_private_key_hex
      ephemeral_public_key_hex shared_secret_hex hkdf_salt_b64 hkdf_info_b64
      wrapping_key_hex wrapping_nonce_hex grant_aad_b64 wrapped_cmk_hex
      signature_input_b64 signature_hex d18g_b64 expected_opened_cmk_hex
    )

    Enum.each(positives, fn item ->
      assert MapSet.new(Map.keys(item)) == MapSet.new(positive_fields)
    end)

    assert_roster(
      @fixture["structural_negative_cases"],
      ~w(wrong-magic unsupported-version trailing-byte),
      ~w(name d18g_b64 expected_error)
    )

    assert_roster(
      @fixture["field_negative_cases"],
      ~w(empty-originator empty-receiver invalid-uuid-version invalid-uuid-variant oversized-originator oversized-receiver),
      ~w(name expected_error)
    )

    assert_roster(
      @fixture["seal_negative_cases"],
      ["low-order-receiver-public-key"],
      ~w(name expected_error)
    )

    assert_roster(
      @fixture["opening_negative_cases"],
      ~w(unexpected-originator unexpected-receiver unexpected-channel invalid-signature
         invalid-signature-before-key-agreement low-order-ephemeral-public-key wrong-receiver-private-key
         wrong-wrapping-nonce mutated-wrapped-cmk mutated-tag epoch-derivation-binding
         receiver-derivation-binding channel-aad-binding originator-aad-binding),
      ~w(name d18g_b64 expected_originator_id_b64 expected_receiver_id_b64
         expected_channel_id_hex receiver_private_key_hex expected_error)
    )

    trace = @fixture["receiver_state_trace"]

    assert Enum.map(trace["steps"], & &1["name"]) ==
             ~w(install-epoch-zero retry-epoch-zero same-epoch-conflict failed-higher-open
                install-skipped-epoch-three decreasing-epoch)
  end

  test "positive cases lock every intermediate and D18G byte" do
    Enum.each(@fixture["positive_cases"], fn test_case ->
      originator_id = decode(test_case["originator_id_b64"])
      receiver_id = decode(test_case["receiver_id_b64"])
      channel_id = from_hex(test_case["channel_id_hex"])
      epoch = integer(test_case["key_epoch"])

      receiver =
        Grants.receiver_key_pair_from_private_key(from_hex(test_case["receiver_private_key_hex"]))

      receiver_public = Grants.receiver_public_key(receiver)

      assert receiver_public == from_hex(test_case["receiver_public_key_hex"]), test_case["name"]

      ephemeral_private = from_hex(test_case["ephemeral_private_key_hex"])
      {_private, ephemeral_public} = X25519.generate_keypair(ephemeral_private)

      assert ephemeral_public == from_hex(test_case["ephemeral_public_key_hex"]),
             test_case["name"]

      shared_secret = X25519.x25519(ephemeral_private, receiver_public)
      assert shared_secret == from_hex(test_case["shared_secret_hex"]), test_case["name"]
      assert Grants.key_grant_hkdf_salt(channel_id, epoch) == decode(test_case["hkdf_salt_b64"])
      assert Grants.key_grant_hkdf_info(receiver_id) == decode(test_case["hkdf_info_b64"])

      assert Grants.key_grant_wrapping_key(shared_secret, channel_id, epoch, receiver_id) ==
               from_hex(test_case["wrapping_key_hex"])

      fields = Grants.grant_fields(originator_id, receiver_id, channel_id, epoch)
      cmk = Grants.channel_master_key_from_bytes(from_hex(test_case["cmk_hex"]))

      grant =
        Grants.seal_channel_key_with_material(
          fields,
          cmk,
          receiver_public,
          signer(),
          ephemeral_private,
          from_hex(test_case["wrapping_nonce_hex"])
        )

      record = decode(test_case["d18g_b64"])
      assert Grants.grant_serialize(grant) == record, test_case["name"]
      assert grant.wrapped_cmk == from_hex(test_case["wrapped_cmk_hex"])
      assert grant.originator_signature == from_hex(test_case["signature_hex"])
      assert Grants.key_grant_aad(grant) == decode(test_case["grant_aad_b64"])
      assert Grants.key_grant_signature_input(grant) == decode(test_case["signature_input_b64"])

      decoded = Grants.grant_deserialize(record)
      assert Grants.grant_serialize(decoded) == record

      opened =
        Grants.open_channel_key_grant(
          decoded,
          originator_id,
          receiver_id,
          channel_id,
          receiver,
          @public_key
        )

      assert Grants.channel_master_key_bytes(opened) ==
               from_hex(test_case["expected_opened_cmk_hex"])
    end)
  end

  test "structural, field, and seal failures use declared codes" do
    base = decode(hd(@fixture["positive_cases"])["d18g_b64"])

    Enum.each(@fixture["structural_negative_cases"], fn test_case ->
      assert_grant_error(test_case["expected_error"], fn ->
        test_case["d18g_b64"] |> decode() |> Grants.grant_deserialize()
      end)
    end)

    recipe = @fixture["truncated_prefix_recipe"]
    first = integer(recipe["first_length"])
    last = integer(recipe["last_length_exclusive"])
    assert byte_size(base) == last

    Enum.each(first..(last - 1), fn finish ->
      assert_grant_error(recipe["expected_error"], fn ->
        base |> binary_part(0, finish) |> Grants.grant_deserialize()
      end)
    end)

    Enum.each(@fixture["oversize_recipes"], fn oversize ->
      offset = integer(oversize["length_offset"])
      declared = integer(oversize["declared_length"])
      <<prefix::binary-size(offset), _::binary-size(4), suffix::binary>> = base
      changed = prefix <> <<declared::unsigned-big-32>> <> suffix

      assert_grant_error(oversize["expected_error"], fn ->
        Grants.grant_deserialize(changed)
      end)
    end)

    Enum.each(@fixture["field_negative_cases"], fn test_case ->
      {originator_id, receiver_id, channel_id} =
        case test_case["name"] do
          "empty-originator" -> {<<>>, "receiver", @channel_id}
          "empty-receiver" -> {"originator", <<>>, @channel_id}
          "invalid-uuid-version" -> {"originator", "receiver", replace_byte(@channel_id, 6, 0x60)}
          "invalid-uuid-variant" -> {"originator", "receiver", replace_byte(@channel_id, 8, 0x10)}
          "oversized-originator" -> {:binary.copy(<<0>>, 4097), "receiver", @channel_id}
          "oversized-receiver" -> {"originator", :binary.copy(<<0>>, 4097), @channel_id}
        end

      assert_grant_error(test_case["expected_error"], fn ->
        Grants.grant_fields(originator_id, receiver_id, channel_id, 0)
      end)
    end)

    fields = Grants.grant_fields("originator", "receiver", @channel_id, 0)
    cmk = Grants.channel_master_key_from_bytes(:binary.copy(<<0x22>>, 32))

    assert_grant_error(hd(@fixture["seal_negative_cases"])["expected_error"], fn ->
      Grants.seal_channel_key_with_material(
        fields,
        cmk,
        :binary.copy(<<0>>, 32),
        signer(),
        :binary.copy(<<0x51>>, 32),
        :binary.copy(<<0x61>>, 24)
      )
    end)
  end

  test "opening failures follow the normative validation order" do
    Enum.each(@fixture["opening_negative_cases"], fn test_case ->
      receiver =
        Grants.receiver_key_pair_from_private_key(from_hex(test_case["receiver_private_key_hex"]))

      assert_grant_error(test_case["expected_error"], fn ->
        Grants.open_channel_key_grant(
          test_case["d18g_b64"] |> decode() |> Grants.grant_deserialize(),
          decode(test_case["expected_originator_id_b64"]),
          decode(test_case["expected_receiver_id_b64"]),
          from_hex(test_case["expected_channel_id_hex"]),
          receiver,
          @public_key
        )
      end)
    end)
  end

  test "receiver trace is atomic, monotonic, and permits skipped epochs" do
    first = hd(@fixture["positive_cases"])

    receiver =
      Grants.receiver_key_pair_from_private_key(from_hex(first["receiver_private_key_hex"]))

    initial =
      Grants.receiver_epoch_keys(
        decode(first["originator_id_b64"]),
        decode(first["receiver_id_b64"]),
        @channel_id,
        receiver,
        @public_key
      )

    trace = @fixture["receiver_state_trace"]

    final =
      Enum.reduce(trace["steps"], initial, fn step, state ->
        grant = trace["grants"][step["grant"]] |> decode() |> Grants.grant_deserialize()

        {actual, updated} =
          try do
            Grants.receiver_state_install(state, grant)
          rescue
            error in ProfileError -> {error.code, state}
          end

        assert actual == step["expected"], step["name"]
        assert to_string(Grants.receiver_state_latest_epoch(updated)) == step["latest_epoch"]

        assert Enum.map(Grants.receiver_state_retained_epochs(updated), &to_string/1) ==
                 step["retained_epochs"]

        updated
      end)

    assert_grant_error(trace["missing_epoch_error"], fn ->
      Grants.receiver_state_key(final, integer(trace["missing_epoch"]))
    end)

    malformed = %PortableKeyGrant{
      originator_id: <<>>,
      receiver_id: <<>>,
      channel_id: :binary.copy(<<0>>, 16),
      key_epoch: Grants.receiver_state_latest_epoch(final),
      ephemeral_public_key: :binary.copy(<<0>>, 32),
      wrapping_nonce: :binary.copy(<<0>>, 24),
      wrapped_cmk: :binary.copy(<<0>>, 48),
      originator_signature: :binary.copy(<<0>>, 64)
    }

    assert_grant_error("conflicting_grant", fn ->
      Grants.receiver_state_install(final, malformed)
    end)

    assert Grants.receiver_state_public_key(final) == Grants.receiver_public_key(receiver)

    destroyed = Grants.destroy_receiver_state(final)
    assert Grants.receiver_state_retained_epochs(destroyed) == []
    assert Grants.receiver_state_latest_epoch(destroyed) == nil

    assert_grant_error("invalid_field", fn ->
      Grants.receiver_state_public_key(destroyed)
    end)
  end

  test "rotation reproduces prospective A+B to B-only revocation" do
    [first, second | _] = @fixture["positive_cases"]

    receiver_a =
      Grants.receiver_key_pair_from_private_key(from_hex(first["receiver_private_key_hex"]))

    receiver_b =
      Grants.receiver_key_pair_from_private_key(from_hex(second["receiver_private_key_hex"]))

    state_a =
      Grants.receiver_epoch_keys(
        decode(first["originator_id_b64"]),
        decode(first["receiver_id_b64"]),
        @channel_id,
        receiver_a,
        @public_key
      )

    state_b =
      Grants.receiver_epoch_keys(
        decode(second["originator_id_b64"]),
        decode(second["receiver_id_b64"]),
        @channel_id,
        receiver_b,
        @public_key
      )

    {"installed", state_a} =
      Grants.receiver_state_install(
        state_a,
        first["d18g_b64"] |> decode() |> Grants.grant_deserialize()
      )

    {"installed", state_b} =
      Grants.receiver_state_install(
        state_b,
        second["d18g_b64"] |> decode() |> Grants.grant_deserialize()
      )

    rotation = @fixture["rotation_case"]
    new_cmk = Grants.channel_master_key_from_bytes(from_hex(rotation["new_cmk_hex"]))

    receiver_material =
      Grants.rotation_receiver_with_material(
        decode(second["receiver_id_b64"]),
        Grants.receiver_public_key(receiver_b),
        :binary.copy(<<0x71>>, 32),
        :binary.copy(<<0x81>>, 24)
      )

    plan =
      Grants.plan_rotation(
        decode(first["originator_id_b64"]),
        @channel_id,
        integer(rotation["current_epoch"]),
        new_cmk,
        [receiver_material],
        signer()
      )

    assert plan.new_epoch == integer(rotation["new_epoch"])

    assert Enum.map(plan.grants, &(Grants.grant_serialize(&1) |> Base.encode64())) ==
             rotation["new_grants_b64"]

    assert Enum.map(plan.grants, &Base.encode64(&1.receiver_id)) ==
             rotation["authorized_receiver_ids_b64"]

    {"installed", state_b} = Grants.receiver_state_install(state_b, hd(plan.grants))

    assert Enum.map(Grants.receiver_state_retained_epochs(state_a), &to_string/1) ==
             rotation["receiver_a_retains_epochs"]

    assert Enum.map(Grants.receiver_state_retained_epochs(state_b), &to_string/1) ==
             rotation["receiver_b_retains_epochs"]

    assert rotation["receiver_a_new_grant"] == nil

    assert Grants.channel_master_key_bytes(plan.new_cmk) ==
             Grants.channel_master_key_bytes(Grants.receiver_state_key(state_b, 1))

    destroyed_plan = Grants.destroy_rotation_plan(plan)

    assert_grant_error("invalid_field", fn ->
      Grants.channel_master_key_bytes(destroyed_plan.new_cmk)
    end)
  end

  test "entropy, redaction, immutability, and rotation edges fail closed" do
    first = hd(@fixture["positive_cases"])

    fields =
      Grants.grant_fields(
        decode(first["originator_id_b64"]),
        decode(first["receiver_id_b64"]),
        @channel_id,
        integer(first["key_epoch"])
      )

    cmk = Grants.channel_master_key_from_bytes(from_hex(first["cmk_hex"]))
    receiver_public = from_hex(first["receiver_public_key_hex"])

    queued =
      queued_random([
        from_hex(first["ephemeral_private_key_hex"]),
        from_hex(first["wrapping_nonce_hex"])
      ])

    grant = Grants.seal_channel_key(fields, cmk, receiver_public, signer(), queued)
    assert Grants.grant_serialize(grant) == decode(first["d18g_b64"])

    generated_cmk = Grants.generate_channel_master_key(fn 32 -> :binary.copy(<<0x09>>, 32) end)

    generated_receiver =
      Grants.generate_receiver_key_pair(fn 32 -> :binary.copy(<<0x0A>>, 32) end)

    generated_signer =
      Grants.generate_originator_signing_key(fn 32 -> :binary.copy(<<0x0B>>, 32) end)

    assert Grants.channel_master_key_bytes(generated_cmk) == :binary.copy(<<0x09>>, 32)
    assert byte_size(Grants.receiver_public_key(generated_receiver)) == 32
    assert byte_size(Grants.originator_public_key(generated_signer)) == 32
    refute inspect(generated_cmk) =~ "0909"
    refute inspect(generated_receiver) =~ "0A0A"
    refute inspect(generated_signer) =~ "0B0B"

    destroyed_cmk = Grants.destroy_channel_master_key(generated_cmk)
    destroyed_receiver = Grants.destroy_receiver_key_pair(generated_receiver)
    destroyed_signer = Grants.destroy_originator_signing_key(generated_signer)
    assert_grant_error("invalid_field", fn -> Grants.channel_master_key_bytes(destroyed_cmk) end)
    assert_grant_error("invalid_field", fn -> Grants.receiver_public_key(destroyed_receiver) end)
    assert_grant_error("invalid_field", fn -> Grants.originator_public_key(destroyed_signer) end)

    short_random = fn length -> :binary.copy(<<0>>, length - 1) end
    failing_random = fn _length -> raise "entropy unavailable" end

    assert_grant_error("randomness_unavailable", fn ->
      Grants.generate_channel_master_key(short_random)
    end)

    assert_grant_error("randomness_unavailable", fn ->
      Grants.generate_receiver_key_pair(failing_random)
    end)

    assert_grant_error("randomness_unavailable", fn ->
      Grants.generate_originator_signing_key(short_random)
    end)

    assert_grant_error("randomness_unavailable", fn ->
      Grants.seal_channel_key(fields, cmk, receiver_public, signer(), short_random)
    end)

    assert_grant_error("randomness_unavailable", fn ->
      Grants.generate_rotation_receiver("receiver", receiver_public, short_random)
    end)

    exhausted =
      Grants.rotation_receiver_with_material(
        "receiver",
        receiver_public,
        :binary.copy(<<3>>, 32),
        :binary.copy(<<4>>, 24)
      )

    max_u64 = 18_446_744_073_709_551_615

    assert_grant_error("epoch_exhausted", fn ->
      Grants.plan_rotation("originator", @channel_id, max_u64, cmk, [exhausted], signer())
    end)

    assert_grant_error("invalid_field", fn ->
      Grants.plan_rotation("originator", @channel_id, 0, cmk, [], signer())
    end)

    duplicate_a =
      Grants.rotation_receiver_with_material(
        "duplicate",
        receiver_public,
        :binary.copy(<<5>>, 32),
        :binary.copy(<<6>>, 24)
      )

    duplicate_b =
      Grants.rotation_receiver_with_material(
        "duplicate",
        receiver_public,
        :binary.copy(<<7>>, 32),
        :binary.copy(<<8>>, 24)
      )

    assert_grant_error("invalid_field", fn ->
      Grants.plan_rotation(
        "originator",
        @channel_id,
        0,
        cmk,
        [duplicate_b, duplicate_a],
        signer()
      )
    end)

    sorted_plan =
      Grants.plan_rotation(
        "originator",
        @channel_id,
        0,
        cmk,
        [
          Grants.rotation_receiver_with_material(
            "receiver-b",
            receiver_public,
            :binary.copy(<<12>>, 32),
            :binary.copy(<<13>>, 24)
          ),
          Grants.rotation_receiver_with_material(
            "receiver-a",
            receiver_public,
            :binary.copy(<<14>>, 32),
            :binary.copy(<<15>>, 24)
          )
        ],
        signer()
      )

    assert Enum.map(sorted_plan.grants, & &1.receiver_id) == ["receiver-a", "receiver-b"]
  end

  test "public constructor shapes and high-level encoder are fail closed" do
    assert_grant_error("invalid_field", fn ->
      Grants.channel_master_key_from_bytes(:binary.copy(<<0>>, 31))
    end)

    assert_grant_error("invalid_field", fn ->
      Grants.receiver_key_pair_from_private_key(:binary.copy(<<0>>, 31))
    end)

    assert_grant_error("invalid_field", fn ->
      Grants.originator_signing_key_from_seed(:binary.copy(<<0>>, 31))
    end)

    structurally_decodable = %PortableKeyGrant{
      originator_id: <<>>,
      receiver_id: <<>>,
      channel_id: :binary.copy(<<0>>, 16),
      key_epoch: 0,
      ephemeral_public_key: :binary.copy(<<0>>, 32),
      wrapping_nonce: :binary.copy(<<0>>, 24),
      wrapped_cmk: :binary.copy(<<0>>, 48),
      originator_signature: :binary.copy(<<0>>, 64)
    }

    assert_grant_error("invalid_field", fn -> Grants.grant_serialize(structurally_decodable) end)

    assert_grant_error("invalid_magic", fn ->
      Grants.grant_deserialize("NOPE" <> :binary.copy(<<0>>, 200))
    end)

    assert_grant_error("invalid_field", fn ->
      Grants.key_grant_hkdf_salt(:binary.copy(<<0>>, 15), 0)
    end)

    assert_grant_error("length_limit_exceeded", fn ->
      Grants.key_grant_hkdf_info(:binary.copy(<<0>>, 4097))
    end)

    assert_grant_error("invalid_field", fn ->
      Grants.key_grant_wrapping_key(:binary.copy(<<0>>, 31), @channel_id, 0, "r")
    end)
  end

  defp replace_byte(binary, offset, value) do
    <<prefix::binary-size(offset), _old, suffix::binary>> = binary
    prefix <> <<value>> <> suffix
  end

  defp queued_random(chunks) do
    {:ok, agent} = Agent.start_link(fn -> chunks end)

    fn length ->
      Agent.get_and_update(agent, fn
        [value | rest] when byte_size(value) == length -> {value, rest}
        _ -> raise "unexpected entropy request"
      end)
    end
  end
end
