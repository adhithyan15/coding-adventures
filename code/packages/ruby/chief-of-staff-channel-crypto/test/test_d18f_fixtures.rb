# frozen_string_literal: true

require "base64"
require "json"
require "minitest/autorun"
require "coding_adventures_chief_of_staff_channel_crypto"

class TestD18FFixtures < Minitest::Test
  Crypto = CodingAdventures::ChiefOfStaffChannelCrypto
  FIXTURE_PATH = File.expand_path("../../../../fixtures/chief-of-staff-message/v1/manifest.json", __dir__)
  FIXTURE = JSON.parse(File.read(FIXTURE_PATH, encoding: "UTF-8"))
  SIGNING_SEED = [FIXTURE.dig("keys", "originator_signing_seed_hex")].pack("H*")
  PUBLIC_KEY, SIGNING_SECRET_KEY = CodingAdventures::Ed25519.generate_keypair(SIGNING_SEED)
  EXPECTED_PUBLIC_KEY = [FIXTURE.dig("keys", "originator_public_key_hex")].pack("H*")
  EPOCH_KEYS = FIXTURE.dig("keys", "channel_master_keys").to_h do |item|
    [Integer(item["key_epoch"], 10), [item["key_hex"]].pack("H*")]
  end.freeze

  def decode(value) = Base64.strict_decode64(value)

  def assert_profile_error(code)
    error = assert_raises(Crypto::MessageProfileError) { yield }
    assert_equal code, error.code
  end

  def test_fixture_provenance_and_public_material_are_locked
    assert_equal "D18F-message-fixtures-v1", FIXTURE["fixture_format"]
    assert_equal 40, FIXTURE["generator_blob_sha1"].length
    assert_includes FIXTURE["warning"], "test-only"
    assert_equal 8, FIXTURE["positive_cases"].length
    assert_equal 20, FIXTURE["binary_negative_cases"].length
    assert_equal 11, FIXTURE["json_negative_cases"].length
    assert_equal EXPECTED_PUBLIC_KEY, PUBLIC_KEY
  end

  def test_positive_fixtures_are_reproduced_byte_identically
    FIXTURE["positive_cases"].each do |test_case|
      binary = decode(test_case["d18m_b64"])
      plaintext = decode(test_case["plaintext_b64"])
      message = Crypto.message_deserialize(binary)
      key = EPOCH_KEYS.fetch(message.key_epoch)

      assert_equal binary, Crypto.message_serialize(message), test_case["name"]
      assert_equal decode(test_case["authenticated_header_b64"]), Crypto.message_authenticated_header(message), test_case["name"]
      assert_equal plaintext, Crypto.message_verify_with_key_resolver(message, PUBLIC_KEY, ->(epoch) { EPOCH_KEYS[epoch] }), test_case["name"]
      assert_equal plaintext, Crypto.message_verify(message, PUBLIC_KEY, key), test_case["name"]

      canonical_json = decode(test_case["canonical_json_b64"])
      assert_equal canonical_json, Crypto.message_to_json(message), test_case["name"]
      assert_equal binary, Crypto.message_serialize(Crypto.message_from_json(canonical_json)), test_case["name"]
      recreated = Crypto.message_create(message.fields, plaintext, SIGNING_SECRET_KEY, key)
      assert_equal binary, Crypto.message_serialize(recreated), test_case["name"]
    end
  end

  def test_binary_mutations_map_to_stable_errors
    FIXTURE["binary_negative_cases"].each do |test_case|
      assert_profile_error(test_case["expected_error"]) do
        message = Crypto.message_deserialize(decode(test_case["d18m_b64"]))
        if test_case["phase"] == "verify"
          Crypto.message_verify_with_key_resolver(message, PUBLIC_KEY, ->(epoch) { EPOCH_KEYS[epoch] })
        end
      end
    end
  end

  def test_json_mutations_map_to_stable_errors
    FIXTURE["json_negative_cases"].each do |test_case|
      assert_profile_error(test_case["expected_error"]) do
        Crypto.message_from_json(decode(test_case["json_b64"]))
      end
    end
  end

  def test_json_field_order_is_irrelevant_and_output_is_canonical
    canonical = decode(FIXTURE["positive_cases"][2]["canonical_json_b64"])
    reordered = JSON.generate(JSON.parse(canonical).to_a.reverse.to_h)
    assert_equal canonical, Crypto.message_to_json(Crypto.message_from_json(reordered))
  end

  def test_json_rejects_unpaired_surrogates
    canonical = decode(FIXTURE["positive_cases"][0]["canonical_json_b64"])
    malformed = canonical.sub('"content_type":"application/octet-stream"', '"content_type":"\\ud800"')
    assert_profile_error("invalid_field") { Crypto.message_from_json(malformed) }
  end

  def test_json_field_types_fail_before_magic_semantics
    canonical = decode(FIXTURE["positive_cases"][0]["canonical_json_b64"])
    wrong_type = canonical.sub('"record_type":"D18M"', '"record_type":18')
    assert_profile_error("invalid_json") { Crypto.message_from_json(wrong_type) }
  end

  def test_canonical_json_uses_literal_utf8_instead_of_optional_unicode_escapes
    canonical = decode(FIXTURE["positive_cases"][0]["canonical_json_b64"])
    escaped = canonical.sub('"content_type":"application/octet-stream"', '"content_type":"application/\\u2028"')
    encoded = Crypto.message_to_json(Crypto.message_from_json(escaped))
    assert_includes encoded, "application/\u2028".encode(Encoding::UTF_8)
    refute_includes encoded, "\\u2028"
  end

  def test_compact_oversize_recipes_are_enforced
    baseline = decode(FIXTURE["positive_cases"][0]["d18m_b64"])
    FIXTURE["oversize_recipes"].each do |recipe|
      if recipe["field"] == "json-input"
        assert_profile_error(recipe["expected_error"]) { Crypto.message_from_json("\0" * (Crypto::MAX_MESSAGE_JSON_BYTES + 1)) }
        next
      end
      changed = baseline.dup
      length = Integer(recipe["declared_length"], 10)
      case recipe["field"]
      when "originator-id" then changed[29, 4] = [length].pack("N")
      when "content-type" then changed[83, 4] = [length].pack("N")
      when "ciphertext" then changed[143, 8] = [length].pack("Q>")
      end
      assert_profile_error(recipe["expected_error"]) { Crypto.message_deserialize(changed) }
    end
  end

  def test_messages_copy_mutable_inputs_and_accessor_results
    source = Crypto.message_deserialize(decode(FIXTURE["positive_cases"][1]["d18m_b64"]))
    buffers = {
      message_id: source.message_id,
      originator_id: source.originator_id,
      channel_id: source.channel_id,
      plaintext_hash: source.plaintext_hash,
      ciphertext: source.ciphertext,
      authentication_tag: source.authentication_tag,
      originator_signature: source.originator_signature
    }
    message = Crypto::D18Message.new(
      message_id: buffers[:message_id], timestamp_ns: source.timestamp_ns,
      originator_id: buffers[:originator_id], channel_id: buffers[:channel_id], sequence: source.sequence,
      key_epoch: source.key_epoch, content_type: source.content_type, plaintext_hash: buffers[:plaintext_hash],
      ciphertext: buffers[:ciphertext], authentication_tag: buffers[:authentication_tag],
      originator_signature: buffers[:originator_signature]
    )
    original = Crypto.message_serialize(message)
    buffers.each_value { |value| value.replace("\0" * value.bytesize) }
    [message.message_id, message.originator_id, message.channel_id, message.plaintext_hash, message.ciphertext,
     message.authentication_tag, message.originator_signature].each { |value| value.replace("\0" * value.bytesize) }
    assert message.frozen?
    assert_equal original, Crypto.message_serialize(message)
  end

  class FixedUuidSource
    def initialize(value) = @value = value
    def next_uuid_v7 = @value.dup
  end

  class FixedClock
    def now_nanoseconds = 456
  end

  def test_creation_uses_injected_uuid_and_monotonic_clock_sources
    source = Crypto.message_deserialize(decode(FIXTURE["positive_cases"][0]["d18m_b64"]))
    key = EPOCH_KEYS.fetch(source.key_epoch)
    fields = Crypto::SourcedMessageFields.new(
      originator_id: source.originator_id, channel_id: source.channel_id, sequence: 123,
      key_epoch: source.key_epoch, content_type: source.content_type
    )
    message = Crypto.message_create_with_sources(
      fields, "\x01\x02\x03".b, SIGNING_SECRET_KEY, key, FixedUuidSource.new(source.message_id), FixedClock.new
    )
    assert_equal source.message_id, message.message_id
    assert_equal 456, message.timestamp_ns
    assert_equal "\x01\x02\x03".b, Crypto.message_verify(message, PUBLIC_KEY, key)
  end

  def test_uuid_v7_generator_orders_1000_values_in_one_millisecond
    generator = Crypto::MonotonicUuidV7Generator.new
    previous = nil
    1000.times do
      current = generator.next(1_725_000_000_000, "\x55" * 10)
      assert_equal 7, current.getbyte(6) >> 4
      assert_equal 0x80, current.getbyte(8) & 0xc0
      assert_operator previous, :<, current unless previous.nil?
      previous = current
    end
  end
end
