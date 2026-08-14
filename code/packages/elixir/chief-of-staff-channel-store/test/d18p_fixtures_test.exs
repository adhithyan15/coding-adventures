defmodule CodingAdventures.ChiefOfStaffChannelStore.D18PFixturesTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.ChiefOfStaffChannelCrypto, as: Crypto
  alias CodingAdventures.ChiefOfStaffChannelStore, as: Profile

  alias CodingAdventures.ChiefOfStaffChannelStore.{
    AppendRequest,
    ChannelDefinitionStore,
    ChannelState,
    ChannelStore,
    DurableOriginator,
    DurableReceiver,
    MemoryChannelStorage,
    MessageMetadata,
    OpaqueKeyGrant,
    ProfileError,
    StorageConflictError,
    StoragePut,
    StorageRecord
  }

  @fixture_path Path.expand(
                  "../../../../fixtures/chief-of-staff-channel/v1/manifest.json",
                  __DIR__
                )
  @fixture @fixture_path |> File.read!() |> Jason.decode!()
  @active_bytes @fixture["definition_cases"] |> hd() |> Map.fetch!("d18c_b64") |> Base.decode64!()
  @definition Profile.definition_deserialize(@active_bytes)
  @channel_id @definition.channel_id
  @originator_id @definition.originator.agent_id
  @binary_receiver_id @fixture["definition_cases"]
                      |> hd()
                      |> Map.fetch!("canonical_receiver_ids_b64")
                      |> hd()
                      |> Base.decode64!()
  @text_receiver_id @fixture["definition_cases"]
                    |> hd()
                    |> Map.fetch!("canonical_receiver_ids_b64")
                    |> Enum.at(1)
                    |> Base.decode64!()
  @signing_seed Base.decode16!(@fixture["test_keys"]["originator_signing_seed_hex"],
                  case: :lower
                )
  @signing_keypair CodingAdventures.Ed25519.generate_keypair(@signing_seed)
  @public_key elem(@signing_keypair, 0)
  @signing_secret_key elem(@signing_keypair, 1)
  @master_key Base.decode16!(@fixture["test_keys"]["channel_master_key_hex"], case: :lower)

  defp decode(value), do: Base.decode64!(value)

  defp assert_profile_error(code, operation) do
    error = assert_raise ProfileError, operation
    assert error.code == code
    assert Exception.message(error) == code
  end

  defp operation(name), do: Enum.find(@fixture["operation_cases"], &(&1["name"] == name))

  test "fixture provenance constants closed failures and role structure are locked" do
    assert @fixture["fixture_format"] == "D18P-durable-channel-fixtures-v1"
    assert String.length(@fixture["generator_blob_sha1"]) == 40
    assert Profile.storage_namespace() == @fixture["constants"]["storage_namespace"]

    assert Profile.definition_content_type() ==
             @fixture["constants"]["content_types"]["definition"]

    assert Profile.state_content_type() == @fixture["constants"]["content_types"]["state"]
    assert Profile.message_content_type() == @fixture["constants"]["content_types"]["message"]
    assert Profile.grant_content_type() == @fixture["constants"]["content_types"]["grant"]
    assert Profile.ack_content_type() == @fixture["constants"]["content_types"]["ack"]
    assert Integer.to_string(Profile.max_receivers()) == @fixture["constants"]["max_receivers"]

    assert Integer.to_string(Profile.max_pending_header_bytes()) ==
             @fixture["constants"]["max_pending_header_bytes"]

    assert Integer.to_string(Profile.max_store_cas_attempts()) ==
             @fixture["constants"]["max_store_cas_attempts"]

    assert Integer.to_string(Profile.max_definition_cas_attempts()) ==
             @fixture["constants"]["max_definition_cas_attempts"]

    assert Profile.error_codes() == @fixture["stable_error_codes"]

    assert @public_key ==
             Base.decode16!(@fixture["test_keys"]["originator_public_key_hex"], case: :lower)

    expected = %{
      "conflicting-definition" => "conflicting_definition",
      "session-delivery-enforcement" => "unknown_message_id",
      "unauthorized-originator" => "unauthorized_originator",
      "unauthorized-receiver" => "unauthorized_receiver",
      "receiver-public-key-mismatch" => "public_key_mismatch",
      "channel-destroyed" => "channel_destroyed",
      "missing-key-grant" => "missing_key_grant",
      "pending-append" => "pending_append",
      "acknowledgement-pending" => "acknowledgement_pending",
      "pending-header-mismatch" => "pending_header_mismatch",
      "no-pending-append" => "no_pending_append",
      "invalid-page-size" => "invalid_page_size",
      "invalid-receiver-id" => "invalid_receiver_id",
      "acknowledgement-ahead" => "acknowledgement_ahead",
      "acknowledgement-regression" => "acknowledgement_regression",
      "message-key-body-mismatch" => "corrupt_record",
      "message-content-type-mismatch" => "corrupt_record"
    }

    assert Map.new(@fixture["operation_negative_cases"], &{&1["name"], &1["expected_error"]}) ==
             expected

    assert function_exported?(DurableOriginator, :publish!, 3)
    refute function_exported?(DurableOriginator, :receive!, 2)
    assert function_exported?(DurableReceiver, :receive!, 2)
    refute function_exported?(DurableReceiver, :publish!, 3)
  end

  test "all codec and deterministic storage key cases are exact" do
    for item <- @fixture["definition_cases"] do
      encoded = decode(item["d18c_b64"])
      definition = Profile.definition_deserialize(encoded)
      assert definition.lifecycle == item["lifecycle"]
      assert Profile.definition_serialize(definition) == encoded
    end

    canonical_ids = Enum.map(@definition.receivers, &Base.encode64(&1.agent_id))
    assert canonical_ids == hd(@fixture["definition_cases"])["canonical_receiver_ids_b64"]

    for item <- @fixture["state_cases"] do
      encoded = decode(item["d18s_b64"])
      state = Profile.state_deserialize(encoded, @channel_id)
      assert state.next_sequence == String.to_integer(item["next_sequence"])
      assert not is_nil(state.pending_header) == item["pending"]
      assert Profile.state_serialize(state) == encoded
    end

    for item <- @fixture["cursor_cases"] do
      encoded = decode(item["d18a_b64"])
      cursor = Profile.cursor_deserialize(encoded)
      assert cursor == String.to_integer(item["first_unread_sequence"])
      assert Profile.cursor_serialize(cursor) == encoded
    end

    keys = %{
      "definition" => Profile.definition_key(@channel_id),
      "state" => Profile.state_key(@channel_id),
      "message-zero" => Profile.message_key(@channel_id, 0),
      "message-max" => Profile.message_key(@channel_id, Profile.max_u64()),
      "message-prefix" => Profile.message_prefix(@channel_id),
      "grant" => Profile.grant_key(@channel_id, 7, @binary_receiver_id),
      "ack-binary-receiver" => Profile.ack_key(@channel_id, @binary_receiver_id)
    }

    for item <- @fixture["storage_key_cases"] do
      assert keys[item["name"]] == item["expected_key"], item["name"]
    end
  end

  test "every malformed codec case and compact oversize recipe fails closed" do
    for item <- @fixture["codec_negative_cases"] do
      assert_profile_error(item["expected_error"], fn ->
        value = decode(item["record_b64"])

        case item["kind"] do
          "definition" -> Profile.definition_deserialize(value)
          "state" -> Profile.state_deserialize(value, @channel_id)
          "cursor" -> Profile.cursor_deserialize(value)
        end
      end)
    end

    oversized_originator =
      %Profile.OriginatorIdentity{
        agent_id: :binary.copy(<<0>>, Profile.max_identity_bytes() + 1),
        public_key: @definition.originator.public_key
      }

    assert_profile_error("invalid_definition", fn ->
      Profile.new_definition!(
        @channel_id,
        oversized_originator,
        @definition.receivers,
        0,
        0
      )
    end)

    receivers =
      Enum.map(0..Profile.max_receivers(), fn index ->
        Profile.new_receiver!(<<index::unsigned-big-16>>, <<0::256>>)
      end)

    assert_profile_error("invalid_definition", fn ->
      Profile.new_definition!(@channel_id, @definition.originator, receivers, 0, 0)
    end)

    oversized_state = <<68, 49, 56, 83, 1, 0::64, 1, 0, 0, 64, 1>>

    assert_profile_error("corrupt_record", fn ->
      Profile.state_deserialize(oversized_state, @channel_id)
    end)
  end

  test "definition create is idempotent and conflicting definitions fail" do
    backend = MemoryChannelStorage.new!()
    definitions = ChannelDefinitionStore.new(backend)
    assert ChannelDefinitionStore.create!(definitions, @definition) == @definition
    assert ChannelDefinitionStore.create!(definitions, @definition) == @definition

    assert ChannelStore.new!(backend, @channel_id) |> ChannelStore.state!() == %ChannelState{
             next_sequence: 0
           }

    conflict =
      Profile.new_definition!(
        @channel_id,
        @definition.originator,
        @definition.receivers,
        @definition.created_at_ns + 1,
        @definition.key_epoch
      )

    assert_profile_error("conflicting_definition", fn ->
      ChannelDefinitionStore.create!(definitions, conflict)
    end)
  end

  test "reserve recovery retry abandon permanent gap paging and acknowledgement trace" do
    backend = MemoryChannelStorage.new!()
    store = ChannelStore.new!(backend, @channel_id)
    assert ChannelStore.initialize!(store) == %ChannelState{next_sequence: 0}
    header = ChannelStore.reserve_append!(store, request(20, 20_000_000_020), "recoverable")
    recovered = ChannelStore.new!(backend, @channel_id)
    assert ChannelStore.initialize!(recovered).pending_header == header

    assert_profile_error("pending_append", fn ->
      ChannelStore.reserve_append!(store, request(21, 20_000_000_021), "pending")
    end)

    assert_profile_error("acknowledgement_pending", fn ->
      ChannelStore.acknowledge!(store, @binary_receiver_id, 0)
    end)

    mismatch =
      Profile.new_header!(%{
        message_id: uuid7(22),
        timestamp_ns: 20_000_000_022,
        originator_id: @originator_id,
        channel_id: @channel_id,
        sequence: 0,
        key_epoch: 0,
        content_type: "text/plain",
        plaintext_hash: header.plaintext_hash
      })

    assert_profile_error("pending_header_mismatch", fn ->
      ChannelStore.commit_reserved!(
        recovered,
        mismatch,
        "recoverable",
        @master_key,
        @signing_secret_key
      )
    end)

    first =
      ChannelStore.commit_reserved!(
        recovered,
        header,
        "recoverable",
        @master_key,
        @signing_secret_key
      )

    retry_message =
      ChannelStore.commit_reserved!(
        recovered,
        header,
        "recoverable",
        @master_key,
        @signing_secret_key
      )

    expected = operation("reserve-recover-complete-retry-abandon-gap")
    assert Crypto.message_serialize(first) == decode(expected["first_d18m_b64"])
    assert Crypto.message_serialize(retry_message) == Crypto.message_serialize(first)
    abandoned = ChannelStore.reserve_append!(recovered, request(23, 20_000_000_023), "abandoned")
    assert ChannelStore.abandon_pending!(recovered).sequence == 1

    assert_profile_error("no_pending_append", fn ->
      ChannelStore.commit_reserved!(
        recovered,
        abandoned,
        "abandoned",
        @master_key,
        @signing_secret_key
      )
    end)

    after_gap =
      ChannelStore.append!(
        recovered,
        request(24, 20_000_000_024),
        "after gap",
        @master_key,
        @signing_secret_key
      )

    assert after_gap.sequence == 2

    assert Enum.map(ChannelStore.read_messages!(recovered, 0, 10).messages, & &1.sequence) == [
             0,
             2
           ]

    page = ChannelStore.read_messages!(recovered, 0, 1)
    assert Enum.map(page.messages, & &1.sequence) == [0]
    assert page.next_start == 1

    assert Enum.map(
             ChannelStore.read_messages!(recovered, page.next_start, 1).messages,
             & &1.sequence
           ) == [2]

    assert Enum.map(ChannelStore.read_messages!(recovered, 2, 10).messages, & &1.sequence) == [2]
    assert ChannelStore.read_messages!(recovered, 3, 10).messages == []

    assert_profile_error("invalid_page_size", fn ->
      ChannelStore.read_messages!(recovered, 0, 0)
    end)

    assert_profile_error("acknowledgement_ahead", fn ->
      ChannelStore.acknowledge!(recovered, @binary_receiver_id, 3)
    end)

    assert ChannelStore.acknowledge!(recovered, @binary_receiver_id, 0) == 1
    assert ChannelStore.acknowledge!(recovered, @binary_receiver_id, 2) == 3

    assert_profile_error("acknowledgement_regression", fn ->
      ChannelStore.acknowledge!(recovered, @binary_receiver_id, 0)
    end)

    assert_profile_error("invalid_receiver_id", fn ->
      ChannelStore.receiver_cursor!(recovered, <<>>)
    end)
  end

  test "encrypted endpoints have independent cursors session acknowledgement and destruction" do
    backend = MemoryChannelStorage.new!()
    definitions = ChannelDefinitionStore.new(backend)
    ChannelDefinitionStore.create!(definitions, @definition)

    source =
      metadata_source([
        %MessageMetadata{message_id: uuid7(1), timestamp_ns: 10_000_000_001},
        %MessageMetadata{message_id: uuid7(2), timestamp_ns: 10_000_000_002}
      ])

    originator =
      DurableOriginator.open!(
        backend,
        @channel_id,
        @originator_id,
        @signing_secret_key,
        @master_key,
        source
      )

    assert DurableOriginator.id(originator) == @originator_id
    assert DurableOriginator.channel_id(originator) == @channel_id
    assert DurableOriginator.public_key(originator) == @public_key
    DurableOriginator.save_receiver_grant!(originator, @binary_receiver_id, <<1>>)
    DurableOriginator.save_receiver_grant!(originator, @text_receiver_id, <<2>>)
    first = DurableOriginator.publish!(originator, "message zero", "text/plain")
    second = DurableOriginator.publish!(originator, "message one", "application/octet-stream")
    assert [first.sequence, second.sequence] == [0, 1]

    assert_profile_error("metadata_error", fn ->
      DurableOriginator.publish!(originator, "exhausted", "text/plain")
    end)

    binary =
      DurableReceiver.open!(
        backend,
        @channel_id,
        @binary_receiver_id,
        provider(@binary_receiver_id)
      )

    assert DurableReceiver.id(binary) == @binary_receiver_id
    assert DurableReceiver.channel_id(binary) == @channel_id

    assert DurableReceiver.public_key(binary) ==
             Profile.receiver(@definition, @binary_receiver_id).public_key

    [zero] = DurableReceiver.receive!(binary, 1)
    assert zero.sequence == 0
    assert zero.payload == "message zero"
    assert DurableReceiver.acknowledge!(binary, zero.message_id) == 1
    [one] = DurableReceiver.receive!(binary, 10)
    assert one.sequence == 1
    assert DurableReceiver.acknowledge!(binary, one.message_id) == 2
    assert DurableReceiver.acknowledge!(binary, one.message_id) == 2
    assert DurableReceiver.receive!(binary, 10) == []

    text =
      DurableReceiver.open!(backend, @channel_id, @text_receiver_id, provider(@text_receiver_id))

    text_messages = DurableReceiver.receive!(text, 10)
    assert Enum.map(text_messages, & &1.sequence) == [0, 1]
    assert DurableReceiver.acknowledge!(text, hd(text_messages).message_id) == 1
    store = ChannelStore.new!(backend, @channel_id)
    assert ChannelStore.receiver_cursor!(store, @binary_receiver_id) == 2
    assert ChannelStore.receiver_cursor!(store, @text_receiver_id) == 1

    failing =
      DurableReceiver.open!(
        backend,
        @channel_id,
        @text_receiver_id,
        provider(@text_receiver_id, true)
      )

    assert_profile_error("crypto_error", fn -> DurableReceiver.receive!(failing, 1) end)

    fresh =
      DurableReceiver.open!(
        backend,
        @channel_id,
        @binary_receiver_id,
        provider(@binary_receiver_id)
      )

    assert_profile_error("unknown_message_id", fn ->
      DurableReceiver.acknowledge!(fresh, first.message_id)
    end)

    assert_profile_error("unauthorized_originator", fn ->
      DurableOriginator.open!(
        backend,
        @channel_id,
        "intruder",
        @signing_secret_key,
        @master_key,
        source
      )
    end)

    assert_profile_error("unauthorized_receiver", fn ->
      DurableReceiver.open!(backend, @channel_id, "intruder", provider(@binary_receiver_id))
    end)

    assert_profile_error("public_key_mismatch", fn ->
      DurableReceiver.open!(backend, @channel_id, @binary_receiver_id, %{
        public_key: <<0::256>>,
        open_grant: fn _, _ -> @master_key end
      })
    end)

    destroyed = ChannelDefinitionStore.destroy!(definitions, @channel_id)
    assert destroyed.lifecycle == "destroyed"
    assert ChannelDefinitionStore.destroy!(definitions, @channel_id) == destroyed
    assert length(ChannelStore.read_messages!(store, 0, 10).messages) == 2

    assert_profile_error("channel_destroyed", fn ->
      DurableOriginator.publish_with_metadata!(
        originator,
        %MessageMetadata{message_id: uuid7(9), timestamp_ns: 9},
        "denied",
        "text/plain"
      )
    end)
  end

  test "opaque grants corrupt records and backend conditions fail closed" do
    backend = MemoryChannelStorage.new!()
    ChannelDefinitionStore.create!(ChannelDefinitionStore.new(backend), @definition)

    originator =
      DurableOriginator.open!(
        backend,
        @channel_id,
        @originator_id,
        @signing_secret_key,
        @master_key,
        metadata_source([%MessageMetadata{message_id: uuid7(9), timestamp_ns: 9}])
      )

    DurableOriginator.publish!(originator, "no grant", "text/plain")

    receiver =
      DurableReceiver.open!(
        backend,
        @channel_id,
        @binary_receiver_id,
        provider(@binary_receiver_id)
      )

    assert_profile_error("missing_key_grant", fn -> DurableReceiver.receive!(receiver, 1) end)

    assert_profile_error("unauthorized_receiver", fn ->
      DurableOriginator.save_receiver_grant!(originator, "intruder", "x")
    end)

    assert_profile_error("corrupt_record", fn ->
      ChannelStore.save_key_grant!(ChannelStore.new!(backend, @channel_id), %OpaqueKeyGrant{
        channel_id: uuid7(99),
        key_epoch: 0,
        receiver_id: @binary_receiver_id,
        body: "x"
      })
    end)

    key_backend = backend_with_message()
    zero_key = Profile.message_key(@channel_id, 0)
    record = MemoryChannelStorage.get(key_backend, Profile.storage_namespace(), zero_key)

    MemoryChannelStorage.corrupt(key_backend, %{record | key: Profile.message_key(@channel_id, 1)})

    assert_profile_error("corrupt_record", fn ->
      ChannelStore.read_messages!(ChannelStore.new!(key_backend, @channel_id), 0, 10)
    end)

    type_backend = backend_with_message()
    record = MemoryChannelStorage.get(type_backend, Profile.storage_namespace(), zero_key)

    MemoryChannelStorage.corrupt(type_backend, %{
      record
      | content_type: "application/octet-stream"
    })

    assert_profile_error("corrupt_record", fn ->
      ChannelStore.read_messages!(ChannelStore.new!(type_backend, @channel_id), 0, 10)
    end)

    conditions = MemoryChannelStorage.new!()

    assert_raise ArgumentError, fn ->
      MemoryChannelStorage.put(conditions, %StoragePut{
        namespace: "n",
        key: "k",
        content_type: "c",
        body: <<>>
      })
    end

    stored =
      MemoryChannelStorage.put(conditions, %StoragePut{
        namespace: "n",
        key: "k",
        content_type: "c",
        body: "a",
        if_absent: true
      })

    assert MemoryChannelStorage.get(conditions, "n", "k").body == "a"

    assert_raise StorageConflictError, fn ->
      MemoryChannelStorage.put(conditions, %StoragePut{
        namespace: "n",
        key: "k",
        content_type: "c",
        body: <<>>,
        if_absent: true
      })
    end

    changed =
      MemoryChannelStorage.put(conditions, %StoragePut{
        namespace: "n",
        key: "k",
        content_type: "c",
        body: "b",
        if_revision: stored.revision
      })

    refute changed.revision == stored.revision

    assert %StorageRecord{} =
             MemoryChannelStorage.get(conditions, "n", "k")
  end

  defp backend_with_message do
    backend = MemoryChannelStorage.new!()
    store = ChannelStore.new!(backend, @channel_id)
    ChannelStore.initialize!(store)
    ChannelStore.append!(store, request(30, 30), "record", @master_key, @signing_secret_key)
    backend
  end

  defp request(value, timestamp) do
    %AppendRequest{
      message_id: uuid7(value),
      timestamp_ns: timestamp,
      originator_id: @originator_id,
      key_epoch: 0,
      content_type: "text/plain"
    }
  end

  defp uuid7(value) do
    base = :binary.copy(<<value>>, 16)

    <<prefix::binary-size(6), _old_version, middle::binary-size(1), _old_variant, suffix::binary>> =
      base

    <<prefix::binary, Bitwise.bor(0x70, Bitwise.band(value, 0x0F)), middle::binary,
      Bitwise.bor(0x80, Bitwise.band(value, 0x3F)), suffix::binary>>
  end

  defp metadata_source(values) do
    {:ok, pid} = Agent.start_link(fn -> values end)

    fn ->
      case Agent.get_and_update(pid, fn
             [value | rest] -> {{:ok, value}, rest}
             [] -> {:error, []}
           end) do
        {:ok, value} -> value
        :error -> raise "metadata exhausted"
      end
    end
  end

  defp provider(receiver_id, fail_open \\ false) do
    %{
      public_key: Profile.receiver(@definition, receiver_id).public_key,
      open_grant: fn _, _ ->
        if fail_open, do: raise("provider details must not escape"), else: @master_key
      end
    }
  end
end
