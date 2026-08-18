# frozen_string_literal: true

require "base64"
require "json"
require "minitest/autorun"
require "coding_adventures_chief_of_staff_channel_epoch_activation"

# Direct consumers of the canonical Rust-authored D18T manifest.
#
# These never regenerate expected bytes locally and never shell out to another
# language -- that is the whole point of a shared fixture. If Ruby disagrees
# with Rust about a single octet, they fail.
class TestD18TFixtures < Minitest::Test
  Epoch = CodingAdventures::ChiefOfStaffChannelEpochActivation
  Crypto = CodingAdventures::ChiefOfStaffChannelCrypto
  Store = CodingAdventures::ChiefOfStaffChannelStore

  FIXTURE_PATH = File.expand_path(
    "../../../../fixtures/chief-of-staff-channel-epoch-activation/v1/manifest.json", __dir__
  )
  FIXTURE_TEXT = File.read(FIXTURE_PATH, encoding: "UTF-8")
  FIXTURE = JSON.parse(FIXTURE_TEXT)
  CHANNEL_ID = ["018f47a09b6c7def923456789abcdef0"].pack("H*")

  def decode(value) = Base64.strict_decode64(value)
  def from_hex(value) = [value].pack("H*")

  def test_manifest_contract_roster_and_secret_boundary
    assert_equal "D18T-durable-epoch-activation-fixtures-v1", FIXTURE["fixture_format"]
    assert_equal "code/specs/D18T-chief-of-staff-durable-epoch-activation-profile.md", FIXTURE["spec"]
    assert_includes FIXTURE["warning"], "Never log"
    assert_equal({
      "state_magic_ascii" => "D18S",
      "state_version" => "2",
      "plan_magic_ascii" => "D18T",
      "plan_version" => "1",
      "state_content_type" => Epoch::EPOCH_STATE_CONTENT_TYPE,
      "plan_content_type" => Epoch::ACTIVATION_PLAN_CONTENT_TYPE,
      "max_cas_attempts" => Epoch::MAX_EPOCH_CAS_ATTEMPTS.to_s
    }, FIXTURE["constants"])

    # The error roster is closed AND ordered. A gate that only checked
    # membership would not notice a reordering, and six languages index it.
    assert_equal FIXTURE["stable_error_codes"], Epoch::ERROR_CODES

    assert_equal %w[after-custody-selection after-plan-write after-first-grant
      after-all-grants after-activation-cas],
      FIXTURE["crash_replay_traces"].map { |item| item["name"] }
    assert_equal 4, FIXTURE["race_traces"].length
    assert_equal 6, FIXTURE["negative_scenarios"].length

    # Rust guarantees erasure; Ruby honestly cannot. The fixture records Rust's
    # claim, and Ruby must report its own rather than echo the manifest.
    assert_equal "guaranteed", FIXTURE["secret_erasure_capability"]
    assert_equal "best_effort", Epoch.secret_erasure_capability

    # Every labelled test-only secret must appear exactly once in the whole
    # manifest. A second occurrence would mean a secret leaked into a summary,
    # a public record, or an expected-error string.
    FIXTURE["test_only_secrets"].each do |name, secret|
      assert_equal 1, FIXTURE_TEXT.scan(secret).length,
        "secret #{name} must appear exactly once in the manifest"
    end
  end

  def test_exact_v1_to_v2_state_migrations
    assert_equal %w[no-pending pending-d18h], FIXTURE["state_migrations"].map { |item| item["name"] }
    FIXTURE["state_migrations"].each do |vector|
      v1 = Store::ChannelProfile.state_deserialize(decode(vector["d18s_v1_b64"]), CHANNEL_ID)
      expected = decode(vector["d18s_v2_b64"])
      v2 = Epoch::Wire.epoch_state_deserialize(expected, CHANNEL_ID)

      assert_equal vector["active_epoch"].to_i, v2.active_epoch
      assert_equal vector["next_sequence"].to_i, v2.next_sequence
      assert_equal v1.next_sequence, v2.next_sequence
      # Migration preserves the in-flight reservation exactly; it never clears a
      # publish that was already reserved, and never invents one.
      if v1.pending_header.nil?
        assert_nil v2.pending_header
      else
        assert_equal v1.pending_header, v2.pending_header
      end
      assert_equal expected, Epoch::Wire.epoch_state_serialize(v2)
    end
  end

  def test_consumes_and_reencodes_canonical_activation_plan
    activation = FIXTURE["activation_case"]
    expected = decode(activation["plan_b64"])
    plan = Epoch::Wire.activation_plan_deserialize(expected)

    assert_equal CHANNEL_ID, plan.channel_id
    assert_equal [0, 1, 1], [plan.base_epoch, plan.new_epoch, plan.receivers.length]
    assert_equal expected, Epoch::Wire.activation_plan_serialize(plan)
    assert_equal activation["plan_record_key"], Epoch::Wire.activation_plan_record_key(CHANNEL_ID, 1)
    assert_equal Epoch::ACTIVATION_PLAN_CONTENT_TYPE, activation["plan_content_type"]

    # Prospective revocation, stated as data: A is rotated out at epoch 1, so A
    # gets no new grant and keeps only epoch 0, while B keeps both.
    assert_equal 1, activation["grant_b64"].length
    assert_nil activation["receiver_a_new_grant"]
    assert_equal ["0"], activation["receiver_a_retains_epochs"]
    assert_equal %w[0 1], activation["receiver_b_retains_epochs"]
  end

  # The strongest fixture test here. It rebuilds the candidate from the labelled
  # test-only secrets using Ruby's own D18Q and D18T code and requires the
  # result to equal the bytes Rust authored -- plan and grant alike.
  def test_reproduces_rust_authored_plan_and_grant_bytes
    secrets = FIXTURE["test_only_secrets"]
    signer = Crypto::OriginatorSigningKey.from_seed(from_hex(secrets["originator_signing_seed_hex"]))
    receiver_a_key = Crypto::ReceiverKeyPair.from_private_key(from_hex(secrets["receiver_a_private_key_hex"]))
    receiver_b_key = Crypto::ReceiverKeyPair.from_private_key(from_hex(secrets["receiver_b_private_key_hex"]))

    receiver_a = Store::ReceiverIdentity.new(agent_id: "receiver-a", public_key: receiver_a_key.public_key)
    receiver_b = Store::ReceiverIdentity.new(agent_id: "receiver-b", public_key: receiver_b_key.public_key)
    definition = Store::ChannelDefinition.new(
      channel_id: CHANNEL_ID,
      originator: Store::OriginatorIdentity.new(agent_id: "originator", public_key: signer.public_key),
      receivers: [receiver_a, receiver_b],
      created_at_ns: 1_725_000_000_000_000_000,
      key_epoch: 0
    )
    rotation = Crypto.plan_rotation(
      "originator", CHANNEL_ID, 0,
      Crypto::ChannelMasterKey.from_bytes(from_hex(secrets["next_cmk_hex"])),
      [Crypto::RotationReceiver.with_material(
        receiver_b.agent_id, receiver_b.public_key,
        from_hex(secrets["ephemeral_private_key_hex"]),
        from_hex(secrets["wrapping_nonce_hex"])
      )],
      signer
    )

    prepared = Epoch::EpochActivationStore.prepare_rotation_candidate(
      definition, 0, [receiver_b], rotation
    )
    public_preparation = prepared.public_preparation

    assert_equal decode(FIXTURE["activation_case"]["plan_b64"]), public_preparation.plan_bytes,
      "Ruby produced different D18T plan bytes than the canonical Rust manifest"
    assert_equal FIXTURE["activation_case"]["grant_b64"].map { |value| decode(value) },
      public_preparation.grants,
      "Ruby produced different D18G bytes than the canonical Rust manifest"
    assert_equal "PreparedEpoch(<redacted>)", prepared.inspect

    prepared.destroy
    signer.destroy
    receiver_a_key.destroy
    receiver_b_key.destroy
  end

  def test_rejects_malformed_state_records
    canonical = decode(FIXTURE["state_migrations"][0]["d18s_v2_b64"])
    {
      "truncated" => canonical.byteslice(0, canonical.bytesize - 1),
      "trailing-byte" => "#{canonical}\x00".b,
      "wrong-version" => canonical.dup.b.tap { |data| data.setbyte(4, 3) },
      "unknown-pending-flag" => canonical.dup.b.tap { |data| data.setbyte(data.bytesize - 1, 2) },
      "wrong-magic" => canonical.dup.b.tap { |data| data.setbyte(0, 88) }
    }.each do |name, mutated|
      error = assert_raises(Epoch::EpochActivationError, name) do
        Epoch::Wire.epoch_state_deserialize(mutated, CHANNEL_ID)
      end
      assert_equal "corrupt_record", error.code, name
    end
  end

  def test_rejects_non_canonical_plans
    canonical = decode(FIXTURE["activation_case"]["plan_b64"])

    error = assert_raises(Epoch::EpochActivationError) do
      Epoch::Wire.activation_plan_deserialize("#{canonical}\x00".b)
    end
    assert_equal "corrupt_record", error.code

    # A two-receiver plan whose entries descend by receiver hash. The decoder
    # must reject it rather than silently canonicalize -- which is exactly what
    # ActivationPlan alone would have done, since it sorts its input.
    descending = [
      canonical.byteslice(0, 37), [2].pack("N"),
      "\x04".b * 32, "\x03".b * 32, "\x02".b * 32, "\x01".b * 32
    ].join.b
    error = assert_raises(Epoch::EpochActivationError) do
      Epoch::Wire.activation_plan_deserialize(descending)
    end
    assert_equal "corrupt_record", error.code

    # Two distinct receivers hashing to the same value is a collision, and D18T
    # treats a collision as invalid input rather than equal authorization.
    duplicate = Epoch::ActivationPlanEntry.new(receiver_id_hash: "\x01".b * 32, grant_hash: "\x02".b * 32)
    other = Epoch::ActivationPlanEntry.new(receiver_id_hash: "\x01".b * 32, grant_hash: "\x03".b * 32)
    assert_raises(Epoch::EpochActivationError) do
      Epoch::ActivationPlan.new(channel_id: CHANNEL_ID, base_epoch: 0, new_epoch: 1,
        receivers: [duplicate, other])
    end

    # A 16-octet channel id that is not a real UUID v7 is rejected, matching
    # Rust and Python. Accepting it would mean two conforming implementations
    # disagreed about whether the same plan record is valid.
    entry = Epoch::ActivationPlanEntry.new(receiver_id_hash: "\x01".b * 32, grant_hash: "\x02".b * 32)
    {
      "wrong-version-nibble" => CHANNEL_ID.dup.b.tap { |id| id.setbyte(6, 0x4f) },
      "wrong-variant-bits" => CHANNEL_ID.dup.b.tap { |id| id.setbyte(8, 0x1f) }
    }.each do |name, bad_channel|
      error = assert_raises(Epoch::EpochActivationError, name) do
        Epoch::ActivationPlan.new(channel_id: bad_channel, base_epoch: 0, new_epoch: 1,
          receivers: [entry])
      end
      assert_equal "corrupt_record", error.code, name
    end

    # Empty receiver set and a non-successor epoch are both rejected.
    assert_raises(Epoch::EpochActivationError) do
      Epoch::ActivationPlan.new(channel_id: CHANNEL_ID, base_epoch: 0, new_epoch: 1, receivers: [])
    end
    assert_raises(Epoch::EpochActivationError) do
      Epoch::ActivationPlan.new(channel_id: CHANNEL_ID, base_epoch: 0, new_epoch: 2,
        receivers: [duplicate])
    end
  end
end
