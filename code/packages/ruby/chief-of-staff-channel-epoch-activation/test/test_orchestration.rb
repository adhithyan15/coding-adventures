# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_chief_of_staff_channel_epoch_activation"

# Crash, retry, activation, publish, race, and destruction orchestration.
class TestOrchestration < Minitest::Test
  Epoch = CodingAdventures::ChiefOfStaffChannelEpochActivation
  Crypto = CodingAdventures::ChiefOfStaffChannelCrypto
  Store = CodingAdventures::ChiefOfStaffChannelStore

  CHANNEL_ID = ["018f47a09b6c7def923456789abcdef0"].pack("H*")
  MESSAGE_ID = ["018f47a09b6c7def923456789abcdef1"].pack("H*")
  CURRENT_CMK = ("\x22".b * 32)
  NEXT_CMK = ("\x33".b * 32)

  # InMemoryKeyCustody that claims durability, so the production constructor
  # accepts it. Only tests may do this; the point of the durable? split is that
  # a real deployment cannot.
  class DurableMemoryCustody < Epoch::InMemoryKeyCustody
    def durable? = true
  end

  def setup
    @signer = Crypto::OriginatorSigningKey.from_seed("\x11".b * 32)
    @receiver_a_key = Crypto::ReceiverKeyPair.from_private_key("\x41".b * 32)
    @receiver_b_key = Crypto::ReceiverKeyPair.from_private_key("\x42".b * 32)
    @receiver_a = Store::ReceiverIdentity.new(agent_id: "receiver-a", public_key: @receiver_a_key.public_key)
    @receiver_b = Store::ReceiverIdentity.new(agent_id: "receiver-b", public_key: @receiver_b_key.public_key)
    @definition = Store::ChannelDefinition.new(
      channel_id: CHANNEL_ID,
      originator: Store::OriginatorIdentity.new(agent_id: "originator", public_key: @signer.public_key),
      receivers: [@receiver_a, @receiver_b],
      created_at_ns: 1_725_000_000_000_000_000,
      key_epoch: 0
    )
    @backend = Store::MemoryChannelStorage.new
    @custody = Epoch::InMemoryKeyCustody.new
    @store = Epoch::EpochActivationStore.open_for_testing(@backend, @custody, CHANNEL_ID)
  end

  def teardown
    @signer.destroy
    @receiver_a_key.destroy
    @receiver_b_key.destroy
  end

  def create
    state = @store.create_epoch_channel(@definition, Crypto::ChannelMasterKey.from_bytes(CURRENT_CMK))
    assert_equal 0, state.active_epoch
    assert_equal 0, state.next_sequence
    assert_nil state.pending_header
    state
  end

  def rotation(cmk_bytes = NEXT_CMK, ephemeral = "\x51".b * 32, nonce = "\x61".b * 24)
    Crypto.plan_rotation(
      "originator", CHANNEL_ID, 0, Crypto::ChannelMasterKey.from_bytes(cmk_bytes),
      [Crypto::RotationReceiver.with_material(@receiver_b.agent_id, @receiver_b.public_key,
        ephemeral, nonce)],
      @signer
    )
  end

  def assert_epoch_error(code)
    error = assert_raises(Epoch::EpochActivationError) { yield }
    assert_equal code, error.code
    # The message is exactly the code -- no channel bytes, no epoch numbers.
    assert_equal code, error.message
    error
  end

  def test_production_rejects_non_durable_custody_and_accepts_durable
    assert_epoch_error("custody_error") do
      Epoch::EpochActivationStore.open(@backend, Epoch::InMemoryKeyCustody.new, CHANNEL_ID)
    end
    # The same custody, wrapped in a type that honestly claims durability, is
    # accepted -- so the gate is on the declaration, not on the type.
    durable = Epoch::EpochActivationStore.open(Store::MemoryChannelStorage.new,
      DurableMemoryCustody.new, CHANNEL_ID)
    refute_nil durable
  end

  def test_custody_first_creation_is_idempotent_and_conflicts_fail_closed
    create
    same = @store.create_epoch_channel(@definition, Crypto::ChannelMasterKey.from_bytes(CURRENT_CMK))
    assert_equal 0, same.active_epoch

    # A different CMK for the same epoch is a fail-closed conflict, and the
    # error does not disclose how the stored secret differed.
    assert_epoch_error("conflicting_active_key") do
      @store.create_epoch_channel(@definition, Crypto::ChannelMasterKey.from_bytes("\x99".b * 32))
    end
  end

  def test_prepare_recover_activate_and_prospective_revocation
    create
    assert_equal "prepared", @store.prepare_rotation(@definition, [@receiver_b], rotation)
    refute_nil @store.activation_plan(1)
    assert_equal "idempotent", @store.recover_preparation(@definition, 1)
    # Preparation must not change the active epoch.
    assert_equal 0, @store.state.active_epoch

    assert_equal "activated", @store.activate_prepared_epoch(@definition, 1)
    assert_equal "idempotent", @store.activate_prepared_epoch(@definition, 1)
    assert_equal 1, @store.state.active_epoch

    # Prospective revocation: the originator retains BOTH epoch keys, because
    # old messages were encrypted under epoch 0 and are never re-encrypted.
    refute_nil @custody.resolve_handle(CHANNEL_ID, 0)
    refute_nil @custody.resolve_handle(CHANNEL_ID, 1)
  end

  # The crash-safety core. Select a candidate in custody and do nothing else --
  # simulating a process that died between phase 2 and phase 4 -- then require
  # recovery to reconstruct every public record from the durable bundle alone.
  def test_crash_after_custody_selection_replays_public_records
    create
    prepared = Epoch::EpochActivationStore.prepare_rotation_candidate(
      @definition, 0, [@receiver_b], rotation
    )
    assert_equal "PreparedEpoch(<redacted>)", prepared.inspect
    assert_equal Epoch::CUSTODY_SELECTED, @custody.prepare_if_absent(prepared)
    # A byte-identical retry is idempotent, not a conflict.
    assert_equal Epoch::CUSTODY_IDEMPOTENT, @custody.prepare_if_absent(prepared)
    prepared.destroy

    # Crash point: custody holds the bundle, nothing is public yet.
    assert_nil @store.activation_plan(1)
    assert_equal "idempotent", @store.recover_preparation(@definition, 1)
    plan = @store.activation_plan(1)
    refute_nil plan
    assert_equal [0, 1], [plan.base_epoch, plan.new_epoch]
  end

  # The race trace of the same name: two candidates compete for E+1, exactly one
  # is selected, and the loser must not write anything public.
  def test_different_candidate_loses_the_custody_slot
    create
    winner = Epoch::EpochActivationStore.prepare_rotation_candidate(
      @definition, 0, [@receiver_b], rotation
    )
    assert_equal Epoch::CUSTODY_SELECTED, @custody.prepare_if_absent(winner)
    winner.destroy

    # A second candidate for the same epoch, differing only in its CMK.
    loser = Epoch::EpochActivationStore.prepare_rotation_candidate(
      @definition, 0, [@receiver_b], rotation("\x77".b * 32, "\x52".b * 32, "\x62".b * 24)
    )
    assert_equal Epoch::CUSTODY_CONFLICT, @custody.prepare_if_absent(loser)
    loser.destroy

    # And the store-level path reports the stable code.
    assert_epoch_error("conflicting_preparation") do
      @store.prepare_rotation(@definition, [@receiver_b],
        rotation("\x88".b * 32, "\x53".b * 32, "\x63".b * 24))
    end
  end

  # Both directions of the shared-CAS race: a reservation in flight blocks
  # activation, and clearing it unblocks.
  def test_pending_publish_serializes_rotation
    create
    request = Epoch::ActiveEpochAppendRequest.new(
      message_id: MESSAGE_ID, timestamp_ns: 1_725_000_000_000_000_001,
      originator_id: "originator", content_type: "application/octet-stream", key_epoch: 0
    )
    reservation = @store.reserve_publish_using_active_epoch(@definition, request, "hello")
    assert_equal 0, reservation.header.sequence
    assert_equal 0, reservation.key_handle.epoch
    assert_equal "EpochKeyHandle(<redacted>)", reservation.key_handle.inspect

    # Publication won the CAS, so activation must yield rather than race it.
    assert_epoch_error("pending_append") do
      @store.prepare_rotation(@definition, [@receiver_b], rotation)
    end

    abandoned = @store.abandon_pending
    assert_equal reservation.header, abandoned
    assert_nil @store.abandon_pending

    # With the reservation cleared, rotation proceeds.
    assert_equal "prepared", @store.prepare_rotation(@definition, [@receiver_b], rotation)
  end

  def test_unactivated_epoch_is_rejected_before_any_state_change
    create
    request = Epoch::ActiveEpochAppendRequest.new(
      message_id: MESSAGE_ID, timestamp_ns: 1, originator_id: "originator",
      content_type: "application/octet-stream", key_epoch: 1
    )
    assert_epoch_error("unactivated_epoch") do
      @store.reserve_publish_using_active_epoch(@definition, request, "hello")
    end
    # The rejection must not have mutated state.
    state = @store.state
    assert_nil state.pending_header
    assert_equal 0, state.next_sequence
  end

  def test_fail_closed_preconditions_and_stable_codes
    create
    assert_epoch_error("preparation_missing") { @store.activate_prepared_epoch(@definition, 1) }
    assert_epoch_error("unexpected_epoch") { @store.recover_preparation(@definition, 2) }

    assert_epoch_error("invalid_plan") do
      @store.reserve_publish_using_active_epoch(@definition, Epoch::ActiveEpochAppendRequest.new(
        message_id: MESSAGE_ID, timestamp_ns: 1, originator_id: "not-originator",
        content_type: "application/octet-stream"
      ), "hello")
    end

    assert_epoch_error("invalid_plan") { @store.prepare_rotation(@definition, [], rotation) }

    isolated = Epoch::EpochActivationStore.open_for_testing(
      @backend, Epoch::InMemoryKeyCustody.new, CHANNEL_ID
    )
    assert_epoch_error("active_key_missing") { isolated.migrate_epoch_state(@definition) }

    assert_epoch_error("custody_error") do
      @custody.with_key(Epoch::EpochKeyHandle.new(channel_id: CHANNEL_ID, epoch: 99)) { |_| nil }
    end
  end

  def test_not_initialized_before_creation
    assert_epoch_error("not_initialized") { @store.state }
    assert_epoch_error("not_initialized") { @store.recover_preparation(@definition, 1) }
  end

  def test_corrupt_public_state_fails_closed
    create
    key = Store::ChannelProfile.state_key(CHANNEL_ID)
    record = @backend.get(Store::STORAGE_NAMESPACE, key)
    refute_nil record
    @backend.corrupt(Store::StorageRecord.new(
      namespace: record.namespace, key: record.key, content_type: record.content_type,
      body: "#{record.body}\x00".b, revision: record.revision
    ))
    assert_epoch_error("corrupt_record") { @store.state }
  end

  # Invariant 6, made observable: destruction erases secrets but leaves the
  # append-only public record exactly where it was.
  def test_destroy_wipes_custody_but_retains_public_history
    create
    @store.prepare_rotation(@definition, [@receiver_b], rotation)
    @store.activate_prepared_epoch(@definition, 1)
    before = @store.activation_plan(1)
    refute_nil before

    destroyed = Store::ChannelDefinitionStore.new(@backend).destroy(CHANNEL_ID)
    @store.apply_destruction(destroyed)
    assert_equal 0, @custody.retained_key_count

    after = @store.activation_plan(1)
    refute_nil after
    assert_equal before, after

    assert_epoch_error("channel_destroyed") do
      @store.reserve_publish_using_active_epoch(destroyed, Epoch::ActiveEpochAppendRequest.new(
        message_id: MESSAGE_ID, timestamp_ns: 1, originator_id: "originator",
        content_type: "application/octet-stream"
      ), "hello")
    end
  end

  def test_apply_destruction_requires_a_destroyed_definition
    create
    assert_epoch_error("invalid_plan") { @store.apply_destruction(@definition) }
    refute_equal 0, @custody.retained_key_count
  end

  def test_decreasing_epoch_is_rejected
    create
    @store.prepare_rotation(@definition, [@receiver_b], rotation)
    @store.activate_prepared_epoch(@definition, 1)
    assert_epoch_error("decreasing_epoch") { @store.recover_preparation(@definition, 0) }
  end

  def test_roster_must_match_grant_receivers
    create
    # One-receiver rotation for B, but a roster naming A.
    assert_epoch_error("invalid_plan") { @store.prepare_rotation(@definition, [@receiver_a], rotation) }
  end
end
