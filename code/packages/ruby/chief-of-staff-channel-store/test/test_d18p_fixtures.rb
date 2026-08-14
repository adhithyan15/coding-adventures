# frozen_string_literal: true

require "base64"
require "json"
require "minitest/autorun"
require "coding_adventures_chief_of_staff_channel_store"

class TestD18PFixtures < Minitest::Test
  Store = CodingAdventures::ChiefOfStaffChannelStore
  Crypto = CodingAdventures::ChiefOfStaffChannelCrypto
  FIXTURE_PATH = File.expand_path("../../../../fixtures/chief-of-staff-channel/v1/manifest.json", __dir__)
  FIXTURE = JSON.parse(File.read(FIXTURE_PATH, encoding: "UTF-8"))
  ACTIVE_BYTES = Base64.strict_decode64(FIXTURE["definition_cases"][0]["d18c_b64"])
  DEFINITION = Store::ChannelProfile.definition_deserialize(ACTIVE_BYTES)
  CHANNEL_ID = DEFINITION.channel_id
  ORIGINATOR_ID = DEFINITION.originator.agent_id
  BINARY_RECEIVER_ID = Base64.strict_decode64(FIXTURE["definition_cases"][0]["canonical_receiver_ids_b64"][0])
  TEXT_RECEIVER_ID = Base64.strict_decode64(FIXTURE["definition_cases"][0]["canonical_receiver_ids_b64"][1])
  SIGNING_SEED = [FIXTURE.dig("test_keys", "originator_signing_seed_hex")].pack("H*")
  PUBLIC_KEY, SIGNING_SECRET_KEY = CodingAdventures::Ed25519.generate_keypair(SIGNING_SEED)
  MASTER_KEY = [FIXTURE.dig("test_keys", "channel_master_key_hex")].pack("H*")

  def decode(value) = Base64.strict_decode64(value)

  def assert_profile_error(code)
    error = assert_raises(Store::ChannelProfileError) { yield }
    assert_equal code, error.code
    assert_equal code, error.message
  end

  def operation(name) = FIXTURE["operation_cases"].find { |item| item["name"] == name }

  def test_fixture_provenance_constants_and_closed_error_roster
    assert_equal "D18P-durable-channel-fixtures-v1", FIXTURE["fixture_format"]
    assert_equal 40, FIXTURE["generator_blob_sha1"].length
    assert_equal Store::STORAGE_NAMESPACE, FIXTURE.dig("constants", "storage_namespace")
    assert_equal Store::DEFINITION_CONTENT_TYPE, FIXTURE.dig("constants", "content_types", "definition")
    assert_equal Store::STATE_CONTENT_TYPE, FIXTURE.dig("constants", "content_types", "state")
    assert_equal Store::MESSAGE_CONTENT_TYPE, FIXTURE.dig("constants", "content_types", "message")
    assert_equal Store::GRANT_CONTENT_TYPE, FIXTURE.dig("constants", "content_types", "grant")
    assert_equal Store::ACK_CONTENT_TYPE, FIXTURE.dig("constants", "content_types", "ack")
    assert_equal Store::MAX_RECEIVERS.to_s, FIXTURE.dig("constants", "max_receivers")
    assert_equal Store::MAX_PENDING_HEADER_BYTES.to_s, FIXTURE.dig("constants", "max_pending_header_bytes")
    assert_equal Store::MAX_STORE_CAS_ATTEMPTS.to_s, FIXTURE.dig("constants", "max_store_cas_attempts")
    assert_equal Store::MAX_DEFINITION_CAS_ATTEMPTS.to_s, FIXTURE.dig("constants", "max_definition_cas_attempts")
    assert_equal Store::ERROR_CODES, FIXTURE["stable_error_codes"]
    expected_operations = {
      "conflicting-definition" => "conflicting_definition", "session-delivery-enforcement" => "unknown_message_id",
      "unauthorized-originator" => "unauthorized_originator", "unauthorized-receiver" => "unauthorized_receiver",
      "receiver-public-key-mismatch" => "public_key_mismatch", "channel-destroyed" => "channel_destroyed",
      "missing-key-grant" => "missing_key_grant", "pending-append" => "pending_append",
      "acknowledgement-pending" => "acknowledgement_pending", "pending-header-mismatch" => "pending_header_mismatch",
      "no-pending-append" => "no_pending_append", "invalid-page-size" => "invalid_page_size",
      "invalid-receiver-id" => "invalid_receiver_id", "acknowledgement-ahead" => "acknowledgement_ahead",
      "acknowledgement-regression" => "acknowledgement_regression", "message-key-body-mismatch" => "corrupt_record",
      "message-content-type-mismatch" => "corrupt_record"
    }
    assert_equal expected_operations, FIXTURE["operation_negative_cases"].to_h { |item| [item["name"], item["expected_error"]] }
    assert_equal PUBLIC_KEY, [FIXTURE.dig("test_keys", "originator_public_key_hex")].pack("H*")
  end

  def test_all_codec_and_storage_key_cases_are_exact
    FIXTURE["definition_cases"].each do |item|
      encoded = decode(item["d18c_b64"])
      definition = Store::ChannelProfile.definition_deserialize(encoded)
      assert_equal item["lifecycle"], definition.lifecycle
      assert_equal encoded, Store::ChannelProfile.definition_serialize(definition)
    end
    assert_equal FIXTURE["definition_cases"][0]["canonical_receiver_ids_b64"], DEFINITION.receivers.map { |receiver| Base64.strict_encode64(receiver.agent_id) }
    FIXTURE["state_cases"].each do |item|
      encoded = decode(item["d18s_b64"])
      state = Store::ChannelProfile.state_deserialize(encoded, CHANNEL_ID)
      assert_equal Integer(item["next_sequence"], 10), state.next_sequence
      assert_equal item["pending"], !state.pending_header.nil?
      assert_equal encoded, Store::ChannelProfile.state_serialize(state)
    end
    FIXTURE["cursor_cases"].each do |item|
      encoded = decode(item["d18a_b64"])
      cursor = Store::ChannelProfile.cursor_deserialize(encoded)
      assert_equal Integer(item["first_unread_sequence"], 10), cursor
      assert_equal encoded, Store::ChannelProfile.cursor_serialize(cursor)
    end
    keys = {
      "definition" => Store::ChannelProfile.definition_key(CHANNEL_ID),
      "state" => Store::ChannelProfile.state_key(CHANNEL_ID),
      "message-zero" => Store::ChannelProfile.message_key(CHANNEL_ID, 0),
      "message-max" => Store::ChannelProfile.message_key(CHANNEL_ID, Store::MAX_U64),
      "message-prefix" => Store::ChannelProfile.message_prefix(CHANNEL_ID),
      "grant" => Store::ChannelProfile.grant_key(CHANNEL_ID, 7, BINARY_RECEIVER_ID),
      "ack-binary-receiver" => Store::ChannelProfile.ack_key(CHANNEL_ID, BINARY_RECEIVER_ID)
    }
    FIXTURE["storage_key_cases"].each { |item| assert_equal item["expected_key"], keys[item["name"]], item["name"] }
  end

  def test_every_malformed_codec_case_maps_to_its_stable_error
    FIXTURE["codec_negative_cases"].each do |item|
      assert_profile_error(item["expected_error"]) do
        value = decode(item["record_b64"])
        case item["kind"]
        when "definition" then Store::ChannelProfile.definition_deserialize(value)
        when "state" then Store::ChannelProfile.state_deserialize(value, CHANNEL_ID)
        else Store::ChannelProfile.cursor_deserialize(value)
        end
      end
    end
  end

  def test_compact_oversize_recipes_are_enforced
    oversized_originator = Store::OriginatorIdentity.new(agent_id: "\0" * (Store::MAX_IDENTITY_BYTES + 1), public_key: DEFINITION.originator.public_key)
    assert_profile_error("invalid_definition") do
      Store::ChannelDefinition.new(channel_id: CHANNEL_ID, originator: oversized_originator, receivers: DEFINITION.receivers, created_at_ns: 0, key_epoch: 0)
    end
    receivers = Array.new(Store::MAX_RECEIVERS + 1) do |index|
      Store::ReceiverIdentity.new(agent_id: [index].pack("n"), public_key: "\0" * 32)
    end
    assert_profile_error("invalid_definition") do
      Store::ChannelDefinition.new(channel_id: CHANNEL_ID, originator: DEFINITION.originator, receivers: receivers, created_at_ns: 0, key_epoch: 0)
    end
    oversized_state = [68, 49, 56, 83, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 64, 1].pack("C*")
    assert_profile_error("corrupt_record") { Store::ChannelProfile.state_deserialize(oversized_state, CHANNEL_ID) }
  end

  def test_definition_create_is_idempotent_and_conflicting
    backend = Store::MemoryChannelStorage.new
    definitions = Store::ChannelDefinitionStore.new(backend)
    assert_equal DEFINITION, definitions.create(DEFINITION)
    assert_equal DEFINITION, definitions.create(DEFINITION)
    assert_equal 0, Store::ChannelStore.new(backend, CHANNEL_ID).state.next_sequence
    conflict = Store::ChannelDefinition.new(channel_id: CHANNEL_ID, originator: DEFINITION.originator, receivers: DEFINITION.receivers, created_at_ns: DEFINITION.created_at_ns + 1, key_epoch: DEFINITION.key_epoch)
    assert_profile_error("conflicting_definition") { definitions.create(conflict) }
  end

  def test_recovery_retry_abandon_gap_paging_and_acknowledgement_trace
    backend = Store::MemoryChannelStorage.new
    store = Store::ChannelStore.new(backend, CHANNEL_ID)
    store.initialize_store
    header = store.reserve_append(request(20, 20_000_000_020), "recoverable".b)
    recovered = Store::ChannelStore.new(backend, CHANNEL_ID)
    assert_equal header, recovered.initialize_store.pending_header
    assert_profile_error("pending_append") { store.reserve_append(request(21, 20_000_000_021), "pending".b) }
    assert_profile_error("acknowledgement_pending") { store.acknowledge(BINARY_RECEIVER_ID, 0) }
    mismatch = Store::MessageHeader.new(message_id: uuid7(22), timestamp_ns: 20_000_000_022, originator_id: ORIGINATOR_ID, channel_id: CHANNEL_ID, sequence: 0, key_epoch: 0, content_type: "text/plain", plaintext_hash: header.plaintext_hash)
    assert_profile_error("pending_header_mismatch") { recovered.commit_reserved(mismatch, "recoverable".b, MASTER_KEY, SIGNING_SECRET_KEY) }
    first = recovered.commit_reserved(header, "recoverable".b, MASTER_KEY, SIGNING_SECRET_KEY)
    retry_message = recovered.commit_reserved(header, "recoverable".b, MASTER_KEY, SIGNING_SECRET_KEY)
    expected = operation("reserve-recover-complete-retry-abandon-gap")
    assert_equal decode(expected["first_d18m_b64"]), Crypto.message_serialize(first)
    assert_equal Crypto.message_serialize(first), Crypto.message_serialize(retry_message)
    abandoned = recovered.reserve_append(request(23, 20_000_000_023), "abandoned".b)
    assert_equal 1, recovered.abandon_pending.sequence
    assert_profile_error("no_pending_append") { recovered.commit_reserved(abandoned, "abandoned".b, MASTER_KEY, SIGNING_SECRET_KEY) }
    assert_equal 2, recovered.append(request(24, 20_000_000_024), "after gap".b, MASTER_KEY, SIGNING_SECRET_KEY).sequence
    assert_equal [0, 2], recovered.read_messages(0, 10).messages.map(&:sequence)
    page = recovered.read_messages(0, 1)
    assert_equal [0], page.messages.map(&:sequence)
    assert_equal 1, page.next_start
    assert_equal [2], recovered.read_messages(page.next_start, 1).messages.map(&:sequence)
    assert_equal [2], recovered.read_messages(2, 10).messages.map(&:sequence)
    assert_empty recovered.read_messages(3, 10).messages
    assert_profile_error("invalid_page_size") { recovered.read_messages(0, 0) }
    assert_profile_error("acknowledgement_ahead") { recovered.acknowledge(BINARY_RECEIVER_ID, 3) }
    assert_equal 1, recovered.acknowledge(BINARY_RECEIVER_ID, 0)
    assert_equal 3, recovered.acknowledge(BINARY_RECEIVER_ID, 2)
    assert_profile_error("acknowledgement_regression") { recovered.acknowledge(BINARY_RECEIVER_ID, 0) }
    assert_profile_error("invalid_receiver_id") { recovered.receiver_cursor("".b) }
  end

  class MetadataSource
    def initialize(values) = @values = values.dup
    def next = @values.shift || raise("metadata exhausted")
  end

  class KeyProvider
    attr_reader :public_key

    def initialize(public_key, fail_open: false)
      @public_key = public_key
      @fail_open = fail_open
    end

    def open_grant(_epoch, _body)
      raise "provider details must not escape" if @fail_open
      MASTER_KEY
    end
  end

  def provider(receiver_id, fail_open: false)
    KeyProvider.new(DEFINITION.receiver(receiver_id).public_key, fail_open: fail_open)
  end

  def test_encrypted_endpoints_independent_cursors_sessions_and_destruction
    backend = Store::MemoryChannelStorage.new
    definitions = Store::ChannelDefinitionStore.new(backend)
    definitions.create(DEFINITION)
    metadata = MetadataSource.new([
      Store::MessageMetadata.new(message_id: uuid7(1), timestamp_ns: 10_000_000_001),
      Store::MessageMetadata.new(message_id: uuid7(2), timestamp_ns: 10_000_000_002)
    ])
    originator = Store::DurableOriginator.open(backend: backend, channel_id: CHANNEL_ID, agent_id: ORIGINATOR_ID, signing_secret_key: SIGNING_SECRET_KEY, channel_master_key: MASTER_KEY, metadata_source: metadata)
    assert_equal ORIGINATOR_ID, originator.id
    assert_equal CHANNEL_ID, originator.channel_id
    assert_equal PUBLIC_KEY, originator.public_key
    originator.save_receiver_grant(BINARY_RECEIVER_ID, "\1".b)
    originator.save_receiver_grant(TEXT_RECEIVER_ID, "\2".b)
    first = originator.publish("message zero".b, "text/plain")
    second = originator.publish("message one".b, "application/octet-stream")
    assert_equal [0, 1], [first.sequence, second.sequence]
    assert_profile_error("metadata_error") { originator.publish("exhausted".b, "text/plain") }

    binary = Store::DurableReceiver.open(backend: backend, channel_id: CHANNEL_ID, receiver_id: BINARY_RECEIVER_ID, key_provider: provider(BINARY_RECEIVER_ID))
    assert_equal BINARY_RECEIVER_ID, binary.id
    assert_equal CHANNEL_ID, binary.channel_id
    assert_equal DEFINITION.receiver(BINARY_RECEIVER_ID).public_key, binary.public_key
    zero = binary.receive(1)
    assert_equal [0], zero.map(&:sequence)
    assert_equal "message zero".b, zero.first.payload
    assert_equal 1, binary.acknowledge(zero.first.message_id)
    one = binary.receive(10)
    assert_equal [1], one.map(&:sequence)
    assert_equal 2, binary.acknowledge(one.first.message_id)
    assert_equal 2, binary.acknowledge(one.first.message_id)
    assert_empty binary.receive(10)

    text = Store::DurableReceiver.open(backend: backend, channel_id: CHANNEL_ID, receiver_id: TEXT_RECEIVER_ID, key_provider: provider(TEXT_RECEIVER_ID))
    text_messages = text.receive(10)
    assert_equal [0, 1], text_messages.map(&:sequence)
    assert_equal 1, text.acknowledge(text_messages.first.message_id)
    store = Store::ChannelStore.new(backend, CHANNEL_ID)
    assert_equal 2, store.receiver_cursor(BINARY_RECEIVER_ID)
    assert_equal 1, store.receiver_cursor(TEXT_RECEIVER_ID)

    failing = Store::DurableReceiver.open(backend: backend, channel_id: CHANNEL_ID, receiver_id: TEXT_RECEIVER_ID, key_provider: provider(TEXT_RECEIVER_ID, fail_open: true))
    assert_profile_error("crypto_error") { failing.receive(1) }
    fresh = Store::DurableReceiver.open(backend: backend, channel_id: CHANNEL_ID, receiver_id: BINARY_RECEIVER_ID, key_provider: provider(BINARY_RECEIVER_ID))
    assert_profile_error("unknown_message_id") { fresh.acknowledge(first.message_id) }
    assert_profile_error("unauthorized_originator") do
      Store::DurableOriginator.open(backend: backend, channel_id: CHANNEL_ID, agent_id: "intruder".b, signing_secret_key: SIGNING_SECRET_KEY, channel_master_key: MASTER_KEY, metadata_source: metadata)
    end
    assert_profile_error("unauthorized_receiver") do
      Store::DurableReceiver.open(backend: backend, channel_id: CHANNEL_ID, receiver_id: "intruder".b, key_provider: provider(BINARY_RECEIVER_ID))
    end
    assert_profile_error("public_key_mismatch") do
      Store::DurableReceiver.open(backend: backend, channel_id: CHANNEL_ID, receiver_id: BINARY_RECEIVER_ID, key_provider: KeyProvider.new("\0" * 32))
    end

    destroyed = definitions.destroy(CHANNEL_ID)
    assert_equal "destroyed", destroyed.lifecycle
    assert_equal destroyed, definitions.destroy(CHANNEL_ID)
    assert_equal 2, store.read_messages(0, 10).messages.length
    assert_profile_error("channel_destroyed") do
      originator.publish_with_metadata(Store::MessageMetadata.new(message_id: uuid7(9), timestamp_ns: 9), "denied".b, "text/plain")
    end
  end

  def test_missing_grants_corrupt_envelopes_and_backend_conditions_fail_closed
    backend = Store::MemoryChannelStorage.new
    Store::ChannelDefinitionStore.new(backend).create(DEFINITION)
    originator = Store::DurableOriginator.open(
      backend: backend, channel_id: CHANNEL_ID, agent_id: ORIGINATOR_ID,
      signing_secret_key: SIGNING_SECRET_KEY, channel_master_key: MASTER_KEY,
      metadata_source: MetadataSource.new([Store::MessageMetadata.new(message_id: uuid7(9), timestamp_ns: 9)])
    )
    originator.publish("no grant".b, "text/plain")
    receiver = Store::DurableReceiver.open(backend: backend, channel_id: CHANNEL_ID, receiver_id: BINARY_RECEIVER_ID, key_provider: provider(BINARY_RECEIVER_ID))
    assert_profile_error("missing_key_grant") { receiver.receive(1) }
    assert_profile_error("unauthorized_receiver") { originator.save_receiver_grant("intruder".b, "x".b) }

    key_backend = backend_with_message
    zero_key = Store::ChannelProfile.message_key(CHANNEL_ID, 0)
    record = key_backend.get(Store::STORAGE_NAMESPACE, zero_key)
    key_backend.corrupt(Store::StorageRecord.new(namespace: record.namespace, key: Store::ChannelProfile.message_key(CHANNEL_ID, 1), content_type: record.content_type, body: record.body, revision: record.revision))
    assert_profile_error("corrupt_record") { Store::ChannelStore.new(key_backend, CHANNEL_ID).read_messages(0, 10) }

    type_backend = backend_with_message
    record = type_backend.get(Store::STORAGE_NAMESPACE, zero_key)
    type_backend.corrupt(Store::StorageRecord.new(namespace: record.namespace, key: record.key, content_type: "application/octet-stream", body: record.body, revision: record.revision))
    assert_profile_error("corrupt_record") { Store::ChannelStore.new(type_backend, CHANNEL_ID).read_messages(0, 10) }

    condition_backend = Store::MemoryChannelStorage.new
    assert_raises(ArgumentError) { condition_backend.put(Store::StoragePut.new(namespace: "n", key: "k", content_type: "c", body: "".b)) }
    body = "a".b
    stored = condition_backend.put(Store::StoragePut.new(namespace: "n", key: "k", content_type: "c", body: body, if_absent: true))
    body.setbyte(0, 98)
    assert_equal "a".b, condition_backend.get("n", "k").body
    assert_raises(Store::StorageConflictError) { condition_backend.put(Store::StoragePut.new(namespace: "n", key: "k", content_type: "c", body: "".b, if_absent: true)) }
    changed = condition_backend.put(Store::StoragePut.new(namespace: "n", key: "k", content_type: "c", body: "b".b, if_revision: stored.revision))
    refute_equal stored.revision, changed.revision
  end

  def backend_with_message
    backend = Store::MemoryChannelStorage.new
    store = Store::ChannelStore.new(backend, CHANNEL_ID)
    store.initialize_store
    store.append(request(30, 30), "record".b, MASTER_KEY, SIGNING_SECRET_KEY)
    backend
  end

  def request(value, timestamp)
    Store::AppendRequest.new(message_id: uuid7(value), timestamp_ns: timestamp, originator_id: ORIGINATOR_ID, key_epoch: 0, content_type: "text/plain")
  end

  def uuid7(value)
    result = ([value].pack("C") * 16).b
    result.setbyte(6, 0x70 | value & 0x0f)
    result.setbyte(8, 0x80 | value & 0x3f)
    result
  end
end
