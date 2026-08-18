# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_chief_of_staff_channel_epoch_activation"
require_relative "test_orchestration"

# Invariant 3: "all grants before visibility".
#
# Writing a grant successfully is not the same as being able to read it back.
# The record a put echoes sits on the same trust boundary as the write, so
# against a write-behind or eventually-consistent backend an echoed success
# proves nothing. Activation must re-READ every grant and byte-compare before
# advancing the epoch -- otherwise it can make E+1 current while a receiver's
# grant is unretrievable, locking that receiver out of a channel it was
# authorized for.
class TestInvariantThree < Minitest::Test
  Epoch = CodingAdventures::ChiefOfStaffChannelEpochActivation
  Crypto = CodingAdventures::ChiefOfStaffChannelCrypto
  Store = CodingAdventures::ChiefOfStaffChannelStore

  CHANNEL_ID = TestOrchestration::CHANNEL_ID
  CURRENT_CMK = TestOrchestration::CURRENT_CMK
  NEXT_CMK = TestOrchestration::NEXT_CMK

  # Accepts writes normally; diverges only on reads of one grant key.
  #
  # Corrupting an already-written record would NOT test this. Activation
  # replays through put_immutable first, whose if-absent put conflicts, re-gets,
  # and returns before control ever reaches the phase-6 loop -- so such a test
  # passes even with the entire invariant-3 check deleted. That is exactly how
  # the Go port's first attempt at this test went wrong. Skipping one read aims
  # the fault at phase 6.
  class GrantFaultBackend
    attr_accessor :fault

    def initialize(inner, grant_key)
      @inner = inner
      @grant_key = grant_key
      @fault = :healthy
      @skip_reads = 1
    end

    def initialize_backend = @inner.initialize_backend

    def get(namespace, key)
      record = @inner.get(namespace, key)
      return record if record.nil? || key != @grant_key

      if @skip_reads.positive?
        @skip_reads -= 1
        return record
      end
      case @fault
      when :vanishes then nil
      when :mutated_body
        Store::StorageRecord.new(namespace: record.namespace, key: record.key,
          content_type: record.content_type, body: "#{record.body}\x00".b,
          revision: record.revision)
      when :mutated_content_type
        Store::StorageRecord.new(namespace: record.namespace, key: record.key,
          content_type: "application/vnd.wrong", body: record.body, revision: record.revision)
      else record
      end
    end

    def put(value) = @inner.put(value)
    def list(namespace, **options) = @inner.list(namespace, **options)
    def corrupt(record) = @inner.corrupt(record)
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
    @store.create_epoch_channel(@definition, Crypto::ChannelMasterKey.from_bytes(CURRENT_CMK))
    @store.prepare_rotation(@definition, [@receiver_b], rotation)
  end

  def teardown
    @signer.destroy
    @receiver_a_key.destroy
    @receiver_b_key.destroy
  end

  def rotation
    Crypto.plan_rotation(
      "originator", CHANNEL_ID, 0, Crypto::ChannelMasterKey.from_bytes(NEXT_CMK),
      [Crypto::RotationReceiver.with_material(@receiver_b.agent_id, @receiver_b.public_key,
        "\x51".b * 32, "\x61".b * 24)],
      @signer
    )
  end

  def grant_key = Store::ChannelProfile.grant_key(CHANNEL_ID, 1, @receiver_b.agent_id)

  def test_activation_refuses_when_a_grant_is_not_retrievable
    %i[vanishes mutated_body mutated_content_type].each do |fault|
      backend = Store::MemoryChannelStorage.new
      custody = Epoch::InMemoryKeyCustody.new
      store = Epoch::EpochActivationStore.open_for_testing(backend, custody, CHANNEL_ID)
      store.create_epoch_channel(@definition, Crypto::ChannelMasterKey.from_bytes(CURRENT_CMK))
      store.prepare_rotation(@definition, [@receiver_b], rotation)

      faulty = GrantFaultBackend.new(backend, grant_key)
      faulty_store = Epoch::EpochActivationStore.open_for_testing(faulty, custody, CHANNEL_ID)
      # Arm the fault only AFTER prepare has written the grant cleanly, so the
      # write genuinely succeeds and only the read-back diverges.
      faulty.fault = fault

      error = assert_raises(Epoch::EpochActivationError, fault.to_s) do
        faulty_store.activate_prepared_epoch(@definition, 1)
      end
      assert_equal "corrupt_record", error.code, fault.to_s

      # The epoch must not have advanced.
      faulty.fault = :healthy
      assert_equal 0, faulty_store.state.active_epoch, fault.to_s
    end
  end

  # Guard against the phase-6 loop being skipped entirely. Without this, a
  # refactor that drops the read-back would leave the tests above passing via
  # some earlier check.
  def test_grant_read_back_is_reached_at_all
    reads = []
    counting = Class.new(GrantFaultBackend) do
      define_method(:get) do |namespace, key|
        reads << key if key == instance_variable_get(:@grant_key)
        instance_variable_get(:@inner).get(namespace, key)
      end
    end.new(@backend, grant_key)

    store = Epoch::EpochActivationStore.open_for_testing(counting, @custody, CHANNEL_ID)
    store.activate_prepared_epoch(@definition, 1)

    # One read from put_immutable's conflict path, one from phase 6.
    assert_operator reads.length, :>=, 2,
      "expected the grant to be re-read during replay, saw #{reads.length} read(s)"
  end

  # Tampered custody bundles are rejected, which is why validate_public_preparation
  # recomputes the plan from the grants instead of trusting the stored commitment.
  def test_tampered_custody_bundles_are_rejected
    genuine = @custody.load_preparation(CHANNEL_ID, 1)
    refute_nil genuine

    [
      [Epoch::PublicPreparation.new(channel_id: CHANNEL_ID, base_epoch: 0, new_epoch: 1,
        plan_bytes: "not a plan", grants: genuine.grants), "corrupt_record"],
      [Epoch::PublicPreparation.new(channel_id: CHANNEL_ID, base_epoch: 5, new_epoch: 6,
        plan_bytes: genuine.plan_bytes, grants: genuine.grants), "invalid_plan"],
      [Epoch::PublicPreparation.new(channel_id: CHANNEL_ID, base_epoch: 0, new_epoch: 1,
        plan_bytes: genuine.plan_bytes, grants: []), "invalid_plan"],
      [Epoch::PublicPreparation.new(channel_id: CHANNEL_ID, base_epoch: 0, new_epoch: 1,
        plan_bytes: genuine.plan_bytes, grants: ["not a grant"]), "crypto_error"]
    ].each do |bundle, code|
      error = assert_raises(Epoch::EpochActivationError) do
        Epoch::Support.validate_public_preparation(@definition, bundle)
      end
      assert_equal code, error.code
    end
  end

  # Proves the D18T layer actually checks signatures rather than trusting
  # whatever custody produced. This is the test that would fail if
  # verify_grant_signature were dropped from the validation path.
  def test_grant_signed_by_another_originator_is_rejected
    impostor = Crypto::OriginatorSigningKey.from_seed("\x12".b * 32)
    forged = Crypto.plan_rotation(
      "originator", CHANNEL_ID, 0, Crypto::ChannelMasterKey.from_bytes(NEXT_CMK),
      [Crypto::RotationReceiver.with_material(@receiver_b.agent_id, @receiver_b.public_key,
        "\x57".b * 32, "\x67".b * 24)],
      impostor
    )
    error = assert_raises(Epoch::EpochActivationError) do
      Epoch::EpochActivationStore.prepare_rotation_candidate(@definition, 0, [@receiver_b], forged)
    end
    assert_equal "crypto_error", error.code
    impostor.destroy
  end
end
