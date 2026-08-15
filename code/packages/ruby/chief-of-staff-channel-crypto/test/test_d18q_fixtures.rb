# frozen_string_literal: true

require "base64"
require "json"
require "minitest/autorun"
require "coding_adventures_chief_of_staff_channel_crypto"

class TestD18QFixtures < Minitest::Test
  Crypto = CodingAdventures::ChiefOfStaffChannelCrypto
  FIXTURE_PATH = File.expand_path(
    "../../../../fixtures/chief-of-staff-channel-key-grant/v1/manifest.json", __dir__
  )
  FIXTURE = JSON.parse(File.read(FIXTURE_PATH, encoding: "UTF-8"))
  SIGNER = Crypto::OriginatorSigningKey.from_seed([FIXTURE.dig("test_signing_key", "seed_hex")].pack("H*"))
  ORIGINATOR_PUBLIC_KEY = [FIXTURE.dig("test_signing_key", "public_key_hex")].pack("H*")
  CHANNEL_ID = [FIXTURE.dig("positive_cases", 0, "channel_id_hex")].pack("H*")

  def decode(value) = Base64.strict_decode64(value)
  def from_hex(value) = [value].pack("H*")

  def assert_grant_error(code)
    error = assert_raises(Crypto::KeyGrantProfileError) { yield }
    assert_equal code, error.code
    assert_equal code, error.message
  end

  def assert_roster(cases, names, fields)
    assert_equal names, cases.map { |item| item.fetch("name") }
    cases.each { |item| assert_equal fields, item.keys }
  end

  def test_manifest_topology_error_vocabulary_and_erasure_are_closed
    assert_equal %w[
      fixture_format spec generator_blob_sha1 warning constants test_signing_key positive_cases
      structural_negative_cases truncated_prefix_recipe oversize_recipes field_negative_cases
      seal_negative_cases opening_negative_cases receiver_state_trace rotation_case
      secret_erasure_capabilities rust_secret_erasure_capability stable_error_codes
    ], FIXTURE.keys
    assert_equal "D18Q-channel-key-grant-fixtures-v1", FIXTURE["fixture_format"]
    assert_equal "code/specs/D18Q-chief-of-staff-channel-key-grant-profile.md", FIXTURE["spec"]
    assert_equal 40, FIXTURE["generator_blob_sha1"].length
    assert_includes FIXTURE["warning"], "test-only"
    assert_includes FIXTURE["warning"], "Never log"
    assert_equal({
      "key_grant_context_ascii" => "chief-channel-key-grant-v1",
      "key_wrap_context_ascii" => "chief-channel-key-wrap-v1",
      "max_identity_bytes" => "4096",
      "wire_magic_ascii" => "D18G",
      "wire_version" => "1"
    }, FIXTURE["constants"])
    assert_equal FIXTURE["stable_error_codes"], Crypto::KEY_GRANT_ERROR_CODES
    assert_equal %w[guaranteed best_effort not_enforceable], FIXTURE["secret_erasure_capabilities"]
    assert_equal "best_effort", Crypto.secret_erasure_capability
    assert_equal "guaranteed", FIXTURE["rust_secret_erasure_capability"]
    assert_equal ORIGINATOR_PUBLIC_KEY, SIGNER.public_key

    positives = FIXTURE["positive_cases"]
    assert_equal %w[epoch-zero-receiver-a epoch-zero-receiver-b maximum-epoch-receiver-a],
                 positives.map { |item| item["name"] }
    positive_fields = %w[
      name originator_id_b64 receiver_id_b64 channel_id_hex key_epoch cmk_hex receiver_private_key_hex
      receiver_public_key_hex ephemeral_private_key_hex ephemeral_public_key_hex shared_secret_hex
      hkdf_salt_b64 hkdf_info_b64 wrapping_key_hex wrapping_nonce_hex grant_aad_b64 wrapped_cmk_hex
      signature_input_b64 signature_hex d18g_b64 expected_opened_cmk_hex
    ]
    positives.each { |item| assert_equal positive_fields, item.keys }
    assert_roster(
      FIXTURE["structural_negative_cases"],
      ["wrong-magic", "unsupported-version", "trailing-byte"],
      %w[name d18g_b64 expected_error]
    )
    assert_roster(
      FIXTURE["field_negative_cases"],
      %w[empty-originator empty-receiver invalid-uuid-version invalid-uuid-variant oversized-originator
         oversized-receiver],
      %w[name expected_error]
    )
    assert_roster(
      FIXTURE["seal_negative_cases"], ["low-order-receiver-public-key"], %w[name expected_error]
    )
    assert_roster(
      FIXTURE["opening_negative_cases"],
      %w[unexpected-originator unexpected-receiver unexpected-channel invalid-signature
         invalid-signature-before-key-agreement low-order-ephemeral-public-key wrong-receiver-private-key
         wrong-wrapping-nonce mutated-wrapped-cmk mutated-tag epoch-derivation-binding
         receiver-derivation-binding channel-aad-binding originator-aad-binding],
      %w[name d18g_b64 expected_originator_id_b64 expected_receiver_id_b64 expected_channel_id_hex
         receiver_private_key_hex expected_error]
    )
    assert_equal %w[source_case first_length last_length_exclusive expected_error],
                 FIXTURE["truncated_prefix_recipe"].keys
    FIXTURE["oversize_recipes"].each do |recipe|
      assert_equal %w[field length_offset declared_length expected_error], recipe.keys
    end
    trace = FIXTURE["receiver_state_trace"]
    assert_equal %w[grants steps missing_epoch missing_epoch_error], trace.keys
    assert_equal %w[install-epoch-zero retry-epoch-zero same-epoch-conflict failed-higher-open
                    install-skipped-epoch-three decreasing-epoch], trace["steps"].map { |step| step["name"] }
    trace["steps"].each do |step|
      assert_equal %w[name grant expected latest_epoch retained_epochs], step.keys
    end
    assert_equal %w[name current_epoch new_epoch new_cmk_hex authorized_receiver_ids_b64 new_grants_b64
                    receiver_a_retains_epochs receiver_b_retains_epochs receiver_a_new_grant],
                 FIXTURE["rotation_case"].keys
  end

  def test_positive_cases_lock_every_intermediate_and_d18g_byte
    FIXTURE["positive_cases"].each do |test_case|
      originator_id = decode(test_case["originator_id_b64"])
      receiver_id = decode(test_case["receiver_id_b64"])
      channel_id = from_hex(test_case["channel_id_hex"])
      epoch = Integer(test_case["key_epoch"], 10)
      receiver = Crypto::ReceiverKeyPair.from_private_key(from_hex(test_case["receiver_private_key_hex"]))
      assert_equal from_hex(test_case["receiver_public_key_hex"]), receiver.public_key, test_case["name"]
      ephemeral_private = from_hex(test_case["ephemeral_private_key_hex"])
      ephemeral_public = CodingAdventures::X25519.generate_keypair(ephemeral_private.bytes).pack("C*")
      assert_equal from_hex(test_case["ephemeral_public_key_hex"]), ephemeral_public, test_case["name"]
      shared_secret = CodingAdventures::X25519.x25519(ephemeral_private.bytes, receiver.public_key.bytes).pack("C*")
      assert_equal from_hex(test_case["shared_secret_hex"]), shared_secret, test_case["name"]
      assert_equal decode(test_case["hkdf_salt_b64"]), Crypto.key_grant_hkdf_salt(channel_id, epoch)
      assert_equal decode(test_case["hkdf_info_b64"]), Crypto.key_grant_hkdf_info(receiver_id)
      assert_equal from_hex(test_case["wrapping_key_hex"]),
                   Crypto.key_grant_wrapping_key(shared_secret, channel_id, epoch, receiver_id)
      fields = Crypto::KeyGrantFields.new(originator_id, receiver_id, channel_id, epoch)
      cmk = Crypto::ChannelMasterKey.from_bytes(from_hex(test_case["cmk_hex"]))
      grant = Crypto.seal_channel_key_with_material(
        fields, cmk, receiver.public_key, SIGNER, ephemeral_private, from_hex(test_case["wrapping_nonce_hex"])
      )
      record = decode(test_case["d18g_b64"])
      assert_equal record, Crypto.grant_serialize(grant), test_case["name"]
      assert_equal from_hex(test_case["wrapped_cmk_hex"]), grant.wrapped_cmk
      assert_equal from_hex(test_case["signature_hex"]), grant.originator_signature
      assert_equal decode(test_case["grant_aad_b64"]), Crypto.key_grant_aad(grant)
      assert_equal decode(test_case["signature_input_b64"]), Crypto.key_grant_signature_input(grant)
      decoded = Crypto.grant_deserialize(record)
      assert_equal record, Crypto.grant_serialize(decoded)
      opened = Crypto.open_channel_key_grant(
        decoded, originator_id, receiver_id, channel_id, receiver, ORIGINATOR_PUBLIC_KEY
      )
      assert_equal from_hex(test_case["expected_opened_cmk_hex"]), opened.bytes
      opened.destroy
      cmk.destroy
      receiver.destroy
    end
  end

  def test_structural_field_and_seal_failures_use_declared_codes
    base = decode(FIXTURE["positive_cases"][0]["d18g_b64"])
    FIXTURE["structural_negative_cases"].each do |test_case|
      assert_grant_error(test_case["expected_error"]) { Crypto.grant_deserialize(decode(test_case["d18g_b64"])) }
    end
    recipe = FIXTURE["truncated_prefix_recipe"]
    first = Integer(recipe["first_length"], 10)
    last = Integer(recipe["last_length_exclusive"], 10)
    assert_equal base.bytesize, last
    (first...last).each do |finish|
      assert_grant_error(recipe["expected_error"]) { Crypto.grant_deserialize(base.byteslice(0, finish)) }
    end
    FIXTURE["oversize_recipes"].each do |oversize|
      changed = base.dup
      offset = Integer(oversize["length_offset"], 10)
      changed[offset, 4] = [Integer(oversize["declared_length"], 10)].pack("N")
      assert_grant_error(oversize["expected_error"]) { Crypto.grant_deserialize(changed) }
    end
    FIXTURE["field_negative_cases"].each do |test_case|
      originator_id = "originator".b
      receiver_id = "receiver".b
      channel_id = CHANNEL_ID.dup
      case test_case["name"]
      when "empty-originator" then originator_id = "".b
      when "empty-receiver" then receiver_id = "".b
      when "invalid-uuid-version" then channel_id.setbyte(6, 0x60)
      when "invalid-uuid-variant" then channel_id.setbyte(8, 0x10)
      when "oversized-originator" then originator_id = "\0" * 4097
      when "oversized-receiver" then receiver_id = "\0" * 4097
      end
      assert_grant_error(test_case["expected_error"]) do
        Crypto::KeyGrantFields.new(originator_id, receiver_id, channel_id, 0)
      end
    end
    fields = Crypto::KeyGrantFields.new("originator".b, "receiver".b, CHANNEL_ID, 0)
    cmk = Crypto::ChannelMasterKey.from_bytes("\x22" * 32)
    assert_grant_error(FIXTURE["seal_negative_cases"][0]["expected_error"]) do
      Crypto.seal_channel_key_with_material(fields, cmk, "\0" * 32, SIGNER, "\x51" * 32, "\x61" * 24)
    end
    cmk.destroy
  end

  def test_opening_failures_follow_normative_validation_order
    FIXTURE["opening_negative_cases"].each do |test_case|
      receiver = Crypto::ReceiverKeyPair.from_private_key(from_hex(test_case["receiver_private_key_hex"]))
      assert_grant_error(test_case["expected_error"]) do
        Crypto.open_channel_key_grant(
          Crypto.grant_deserialize(decode(test_case["d18g_b64"])),
          decode(test_case["expected_originator_id_b64"]),
          decode(test_case["expected_receiver_id_b64"]),
          from_hex(test_case["expected_channel_id_hex"]), receiver, ORIGINATOR_PUBLIC_KEY
        )
      end
      receiver.destroy
    end
  end

  def retained_epochs(state, maximum)
    (0..maximum).filter_map do |epoch|
      begin
        key = state.key(epoch)
        key.destroy
        epoch.to_s
      rescue Crypto::KeyGrantProfileError
        nil
      end
    end
  end

  def test_receiver_trace_is_atomic_monotonic_and_allows_skipped_epochs
    first = FIXTURE["positive_cases"][0]
    original_receiver = Crypto::ReceiverKeyPair.from_private_key(from_hex(first["receiver_private_key_hex"]))
    state = Crypto::ReceiverEpochKeys.new(
      decode(first["originator_id_b64"]), decode(first["receiver_id_b64"]), CHANNEL_ID, original_receiver,
      ORIGINATOR_PUBLIC_KEY
    )
    trace = FIXTURE["receiver_state_trace"]
    trace["steps"].each do |step|
      grant = Crypto.grant_deserialize(decode(trace["grants"][step["grant"]]))
      actual = begin
        state.install_grant(grant)
      rescue Crypto::KeyGrantProfileError => error
        error.code
      end
      assert_equal step["expected"], actual, step["name"]
      assert_equal step["latest_epoch"], state.latest_epoch.to_s
      assert_equal step["retained_epochs"], retained_epochs(state, 3)
    end
    assert_grant_error(trace["missing_epoch_error"]) { state.key(Integer(trace["missing_epoch"], 10)) }
    malformed = Crypto::PortableKeyGrant.new(
      originator_id: "".b, receiver_id: "".b, channel_id: "\0" * 16, key_epoch: state.latest_epoch,
      ephemeral_public_key: "\0" * 32, wrapping_nonce: "\0" * 24, wrapped_cmk: "\0" * 48,
      originator_signature: "\0" * 64
    )
    assert_grant_error("conflicting_grant") { state.install_grant(malformed) }
    assert_equal original_receiver.public_key, state.receiver_public_key
    state.destroy
    original_receiver.destroy
  end

  def test_rotation_reproduces_prospective_revocation_fixture
    first, second = FIXTURE["positive_cases"].first(2)
    receiver_a = Crypto::ReceiverKeyPair.from_private_key(from_hex(first["receiver_private_key_hex"]))
    receiver_b = Crypto::ReceiverKeyPair.from_private_key(from_hex(second["receiver_private_key_hex"]))
    state_a = Crypto::ReceiverEpochKeys.new(
      decode(first["originator_id_b64"]), decode(first["receiver_id_b64"]), CHANNEL_ID, receiver_a,
      ORIGINATOR_PUBLIC_KEY
    )
    state_b = Crypto::ReceiverEpochKeys.new(
      decode(second["originator_id_b64"]), decode(second["receiver_id_b64"]), CHANNEL_ID, receiver_b,
      ORIGINATOR_PUBLIC_KEY
    )
    state_a.install_grant(Crypto.grant_deserialize(decode(first["d18g_b64"])))
    state_b.install_grant(Crypto.grant_deserialize(decode(second["d18g_b64"])))
    rotation = FIXTURE["rotation_case"]
    new_cmk = Crypto::ChannelMasterKey.from_bytes(from_hex(rotation["new_cmk_hex"]))
    plan = Crypto.plan_rotation(
      decode(first["originator_id_b64"]), CHANNEL_ID, Integer(rotation["current_epoch"], 10), new_cmk,
      [Crypto::RotationReceiver.with_material(
        decode(second["receiver_id_b64"]), receiver_b.public_key, "\x71" * 32, "\x81" * 24
      )], SIGNER
    )
    assert_equal Integer(rotation["new_epoch"], 10), plan.new_epoch
    assert_equal rotation["new_grants_b64"], plan.grants.map { |grant| Base64.strict_encode64(Crypto.grant_serialize(grant)) }
    assert_equal rotation["authorized_receiver_ids_b64"],
                 plan.grants.map { |grant| Base64.strict_encode64(grant.receiver_id) }
    state_b.install_grant(plan.grants[0])
    assert_equal rotation["receiver_a_retains_epochs"], retained_epochs(state_a, 1)
    assert_equal rotation["receiver_b_retains_epochs"], retained_epochs(state_b, 1)
    assert_nil rotation["receiver_a_new_grant"]
    planned_cmk = plan.new_cmk
    installed_cmk = state_b.key(1)
    assert_equal planned_cmk.bytes, installed_cmk.bytes
    installed_cmk.destroy
    planned_cmk.destroy
    plan.destroy
    new_cmk.destroy
    state_a.destroy
    state_b.destroy
    receiver_a.destroy
    receiver_b.destroy
  end

  class QueuedRandom
    def initialize(chunks) = @chunks = chunks.each

    def random_bytes(length)
      value = @chunks.next
      raise "unexpected request" unless value.bytesize == length
      value
    end
  end

  class ShortRandom
    def random_bytes(length) = "\0" * (length - 1)
  end

  class FailingRandom
    def random_bytes(length) = raise("secret request length #{length}")
  end

  def test_entropy_lifecycle_immutability_and_rotation_edges
    first = FIXTURE["positive_cases"][0]
    fields = Crypto::KeyGrantFields.new(
      decode(first["originator_id_b64"]), decode(first["receiver_id_b64"]), CHANNEL_ID,
      Integer(first["key_epoch"], 10)
    )
    cmk = Crypto::ChannelMasterKey.from_bytes(from_hex(first["cmk_hex"]))
    receiver_public = from_hex(first["receiver_public_key_hex"])
    grant = Crypto.seal_channel_key(
      fields, cmk, receiver_public, SIGNER,
      QueuedRandom.new([from_hex(first["ephemeral_private_key_hex"]), from_hex(first["wrapping_nonce_hex"])])
    )
    assert_equal decode(first["d18g_b64"]), Crypto.grant_serialize(grant)
    assert grant.frozen?
    mutable_originator = "mutable-originator".b
    copied_fields = Crypto::KeyGrantFields.new(mutable_originator, "receiver".b, CHANNEL_ID, 0)
    mutable_originator.replace("\0" * mutable_originator.bytesize)
    refute_equal mutable_originator, copied_fields.originator_id
    grant.originator_id.replace("\0" * grant.originator_id.bytesize)
    assert_equal decode(first["originator_id_b64"]), grant.originator_id

    generated_cmk = Crypto::ChannelMasterKey.generate(QueuedRandom.new(["\x09" * 32]))
    generated_receiver = Crypto::ReceiverKeyPair.generate(QueuedRandom.new(["\x0a" * 32]))
    generated_signer = Crypto::OriginatorSigningKey.generate(QueuedRandom.new(["\x0b" * 32]))
    assert_equal "\x09" * 32, generated_cmk.bytes
    assert_equal 32, generated_receiver.public_key.bytesize
    assert_equal 32, generated_signer.public_key.bytesize
    refute_includes generated_cmk.inspect, "0909"
    refute_includes generated_receiver.inspect, "0a0a"
    refute_includes generated_signer.inspect, "0b0b"
    generated_cmk.destroy
    generated_receiver.destroy
    generated_signer.destroy
    assert_grant_error("invalid_field") { generated_cmk.bytes }
    assert_grant_error("invalid_field") { generated_receiver.public_key }
    assert_grant_error("invalid_field") { generated_signer.public_key }

    assert_grant_error("randomness_unavailable") { Crypto::ChannelMasterKey.generate(ShortRandom.new) }
    assert_grant_error("randomness_unavailable") { Crypto::ReceiverKeyPair.generate(FailingRandom.new) }
    assert_grant_error("randomness_unavailable") { Crypto::OriginatorSigningKey.generate(ShortRandom.new) }
    assert_grant_error("randomness_unavailable") do
      Crypto.seal_channel_key(fields, cmk, receiver_public, SIGNER, ShortRandom.new)
    end
    assert_grant_error("randomness_unavailable") do
      Crypto::RotationReceiver.generate("receiver".b, receiver_public, ShortRandom.new)
    end
    exhausted = Crypto::RotationReceiver.with_material(
      "receiver".b, receiver_public, "\x03" * 32, "\x04" * 24
    )
    assert_grant_error("epoch_exhausted") do
      Crypto.plan_rotation("originator".b, CHANNEL_ID, Crypto::MAX_U64, cmk, [exhausted], SIGNER)
    end
    assert_grant_error("invalid_field") do
      Crypto.plan_rotation("originator".b, CHANNEL_ID, 0, cmk, [], SIGNER)
    end
    duplicate_a = Crypto::RotationReceiver.with_material(
      "duplicate".b, receiver_public, "\x05" * 32, "\x06" * 24
    )
    duplicate_b = Crypto::RotationReceiver.with_material(
      "duplicate".b, receiver_public, "\x07" * 32, "\x08" * 24
    )
    assert_grant_error("invalid_field") do
      Crypto.plan_rotation("originator".b, CHANNEL_ID, 0, cmk, [duplicate_b, duplicate_a], SIGNER)
    end
    assert_grant_error("invalid_field") { duplicate_a.seal(fields, cmk, SIGNER) }
    sorted_plan = Crypto.plan_rotation(
      "originator".b, CHANNEL_ID, 0, cmk,
      [
        Crypto::RotationReceiver.with_material("receiver-b".b, receiver_public, "\x0c" * 32, "\x0d" * 24),
        Crypto::RotationReceiver.with_material("receiver-a".b, receiver_public, "\x0e" * 32, "\x0f" * 24)
      ], SIGNER
    )
    assert_equal ["receiver-a".b, "receiver-b".b], sorted_plan.grants.map(&:receiver_id)
    sorted_plan.destroy
    cmk.destroy
  end

  def test_public_constructor_shapes_and_high_level_encoder_are_fail_closed
    assert_grant_error("invalid_field") { Crypto::ChannelMasterKey.from_bytes("\0" * 31) }
    assert_grant_error("invalid_field") { Crypto::ReceiverKeyPair.from_private_key("\0" * 31) }
    assert_grant_error("invalid_field") { Crypto::OriginatorSigningKey.from_seed("\0" * 31) }
    assert_grant_error("invalid_field") do
      Crypto::PortableKeyGrant.new(
        originator_id: "originator".b, receiver_id: "receiver".b, channel_id: "\0" * 15, key_epoch: 0,
        ephemeral_public_key: "\0" * 32, wrapping_nonce: "\0" * 24, wrapped_cmk: "\0" * 48,
        originator_signature: "\0" * 64
      )
    end
    structurally_decodable = Crypto::PortableKeyGrant.new(
      originator_id: "".b, receiver_id: "".b, channel_id: "\0" * 16, key_epoch: 0,
      ephemeral_public_key: "\0" * 32, wrapping_nonce: "\0" * 24, wrapped_cmk: "\0" * 48,
      originator_signature: "\0" * 64
    )
    assert_grant_error("invalid_field") { Crypto.grant_serialize(structurally_decodable) }
    assert_grant_error("invalid_magic") { Crypto.grant_deserialize("NOPE" + "\0" * 200) }
    assert_grant_error("invalid_field") { Crypto.key_grant_hkdf_salt("\0" * 15, 0) }
    assert_grant_error("length_limit_exceeded") { Crypto.key_grant_hkdf_info("\0" * 4097) }
    assert_grant_error("invalid_field") { Crypto.key_grant_wrapping_key("\0" * 31, CHANNEL_ID, 0, "r".b) }
  end
end
