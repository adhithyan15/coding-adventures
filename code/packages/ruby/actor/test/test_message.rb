# frozen_string_literal: true

require_relative "test_helper"

# ═══════════════════════════════════════════════════════════════
# Message Tests
# ═══════════════════════════════════════════════════════════════
#
# These tests verify that Message:
#   - Creates correctly with all fields
#   - Is immutable (frozen after creation)
#   - Generates unique IDs and monotonically increasing timestamps
#   - Serializes/deserializes via the binary wire format
#   - Handles text, JSON, and binary payloads
#   - Rejects invalid/future wire format versions
#
class TestMessage < Minitest::Test
  Message = CodingAdventures::Actor::Message
  VersionError = CodingAdventures::Actor::VersionError
  InvalidFormatError = CodingAdventures::Actor::InvalidFormatError

  # ─── Test 1: Create message with all fields ─────────────────
  #
  # Verify that all properties return the values set at creation time.
  def test_create_message
    msg = Message.new(
      sender_id: "actor_1",
      content_type: "text/plain",
      payload: "hello world",
      metadata: {"key" => "value"}
    )

    assert_kind_of String, msg.id
    refute_empty msg.id
    assert_kind_of Integer, msg.timestamp
    assert_equal "actor_1", msg.sender_id
    assert_equal "text/plain", msg.content_type
    assert_equal "hello world".b, msg.payload
    assert_equal({"key" => "value"}, msg.metadata)
  end

  # ─── Test 2: Immutability ───────────────────────────────────
  #
  # Messages are frozen after creation. Any attempt to modify
  # them should raise a FrozenError (or RuntimeError on older Ruby).
  def test_immutability
    msg = Message.text(sender_id: "a", payload: "hello")

    assert msg.frozen?, "Message should be frozen after creation"

    # Attempting to modify should raise
    assert_raises(FrozenError) { msg.instance_variable_set(:@sender_id, "b") }
  end

  # ─── Test 3: Unique IDs ────────────────────────────────────
  #
  # Every message gets a unique UUID. Creating 1000 messages
  # should produce 1000 distinct IDs.
  def test_unique_ids
    ids = 1000.times.map do
      Message.text(sender_id: "a", payload: "x").id
    end

    assert_equal 1000, ids.uniq.length, "All 1000 message IDs should be unique"
  end

  # ─── Test 4: Timestamp ordering ─────────────────────────────
  #
  # Messages created sequentially should have strictly increasing
  # timestamps (monotonic clock guarantee).
  def test_timestamp_ordering
    messages = 10.times.map do
      Message.text(sender_id: "a", payload: "x")
    end

    messages.each_cons(2) do |earlier, later|
      assert later.timestamp >= earlier.timestamp,
        "Timestamps should be monotonically increasing"
    end
  end

  # ─── Test 5: Wire format round-trip (text) ──────────────────
  #
  # Serialize a text message to bytes, deserialize it back,
  # verify all fields are identical.
  def test_wire_format_round_trip_text
    original = Message.text(
      sender_id: "agent_1",
      payload: "hello world",
      metadata: {"trace" => "abc123"}
    )

    bytes = original.to_bytes
    restored = Message.from_bytes(bytes)

    assert_equal original.id, restored.id
    assert_equal original.timestamp, restored.timestamp
    assert_equal original.sender_id, restored.sender_id
    assert_equal original.content_type, restored.content_type
    assert_equal original.payload, restored.payload
    assert_equal original.metadata, restored.metadata
  end

  # ─── Test 6: Wire format round-trip (binary) ────────────────
  #
  # A PNG file starts with an 8-byte magic signature. Verify
  # that binary data survives serialization without corruption.
  def test_wire_format_round_trip_binary
    # PNG magic header bytes
    png_header = [137, 80, 78, 71, 13, 10, 26, 10].pack("C*")

    original = Message.binary(
      sender_id: "camera",
      content_type: "image/png",
      payload: png_header
    )

    bytes = original.to_bytes
    restored = Message.from_bytes(bytes)

    assert_equal original.payload, restored.payload
    assert_equal "image/png", restored.content_type
  end

  # ─── Test 7: Metadata passthrough ──────────────────────────
  #
  # Metadata key-value pairs must survive serialization intact.
  def test_metadata_passthrough
    meta = {
      "correlation_id" => "req_abc123",
      "priority" => "high",
      "width" => "1920"
    }

    original = Message.text(sender_id: "a", payload: "test", metadata: meta)
    restored = Message.from_bytes(original.to_bytes)

    assert_equal meta, restored.metadata
  end

  # ─── Test 8: Empty payload ─────────────────────────────────
  #
  # A message with zero bytes of payload should work correctly.
  def test_empty_payload
    msg = Message.new(
      sender_id: "a",
      content_type: "text/plain",
      payload: ""
    )

    assert_equal 0, msg.payload.bytesize

    # Round-trip should work too
    restored = Message.from_bytes(msg.to_bytes)
    assert_equal 0, restored.payload.bytesize
  end

  # ─── Test 9: Large payload ─────────────────────────────────
  #
  # A 1MB binary payload should serialize and deserialize correctly.
  def test_large_payload
    large_data = SecureRandom.random_bytes(1_048_576) # 1 MB

    msg = Message.binary(
      sender_id: "uploader",
      content_type: "application/octet-stream",
      payload: large_data
    )

    restored = Message.from_bytes(msg.to_bytes)
    assert_equal large_data, restored.payload
    assert_equal 1_048_576, restored.payload.bytesize
  end

  # ─── Test 10: Content type preserved ────────────────────────
  def test_content_type_preserved
    msg = Message.new(
      sender_id: "a",
      content_type: "video/mp4",
      payload: "fake_video_data"
    )

    restored = Message.from_bytes(msg.to_bytes)
    assert_equal "video/mp4", restored.content_type
  end

  # ─── Test 11: Convenience constructors ──────────────────────
  #
  # Message.text, Message.json, and Message.binary should produce
  # messages with the correct content_type and payload encoding.
  def test_convenience_constructors
    # Text
    text_msg = Message.text(sender_id: "a", payload: "hello")
    assert_equal "text/plain", text_msg.content_type
    assert_equal "hello", text_msg.payload_text

    # JSON
    json_msg = Message.json(sender_id: "a", payload: {"key" => "value"})
    assert_equal "application/json", json_msg.content_type
    assert_equal({"key" => "value"}, json_msg.payload_json)

    # Binary
    bin_msg = Message.binary(
      sender_id: "a",
      content_type: "image/jpeg",
      payload: "\xFF\xD8\xFF".b
    )
    assert_equal "image/jpeg", bin_msg.content_type
    assert_equal "\xFF\xD8\xFF".b, bin_msg.payload
  end

  # ─── Test 12: payload_text ──────────────────────────────────
  def test_payload_text
    msg = Message.text(sender_id: "a", payload: "hello world")
    assert_equal "hello world", msg.payload_text
    assert_equal Encoding::UTF_8, msg.payload_text.encoding
  end

  # ─── Test 13: payload_json ──────────────────────────────────
  def test_payload_json
    data = {"users" => [1, 2, 3], "active" => true}
    msg = Message.json(sender_id: "a", payload: data)
    assert_equal data, msg.payload_json
  end

  # ─── Test 14: Envelope-only serialization ───────────────────
  #
  # envelope_to_json produces JSON without the payload, useful
  # for indexing or logging without touching large binary data.
  def test_envelope_to_json
    msg = Message.text(sender_id: "a", payload: "big data here")
    json_str = msg.envelope_to_json
    parsed = JSON.parse(json_str)

    assert_equal msg.id, parsed["id"]
    assert_equal msg.timestamp, parsed["timestamp"]
    assert_equal "a", parsed["sender_id"]
    assert_equal "text/plain", parsed["content_type"]
    refute parsed.key?("payload"), "Envelope should not contain payload"
  end

  # ─── Test 15: Wire format magic ─────────────────────────────
  #
  # The first 4 bytes of serialized data should be "ACTM".
  def test_wire_format_magic
    msg = Message.text(sender_id: "a", payload: "test")
    bytes = msg.to_bytes

    assert_equal "ACTM", bytes[0, 4]
  end

  # ─── Test 16: Wire format version ──────────────────────────
  #
  # The 5th byte should be the WIRE_VERSION (currently 1).
  def test_wire_format_version
    msg = Message.text(sender_id: "a", payload: "test")
    bytes = msg.to_bytes

    assert_equal Message::WIRE_VERSION, bytes[4].unpack1("C")
  end

  # ─── Test 17: Future version rejection ──────────────────────
  #
  # If we receive data with a version higher than what we support,
  # we must raise VersionError — not crash with a confusing error.
  def test_future_version_rejection
    msg = Message.text(sender_id: "a", payload: "test")
    bytes = msg.to_bytes.dup

    # Tamper with the version byte (byte index 4) to be version 99
    bytes[4] = [99].pack("C")

    error = assert_raises(VersionError) { Message.from_bytes(bytes) }
    assert_match(/99/, error.message)
  end

  # ─── Test 18: Corrupt magic rejection ──────────────────────
  #
  # Data that doesn't start with "ACTM" is not an actor message.
  def test_corrupt_magic_rejection
    bad_data = "XXXX" + ("\x00" * 13) + "some data"

    assert_raises(InvalidFormatError) { Message.from_bytes(bad_data) }
  end

  # ─── Test 19: Stream reading ────────────────────────────────
  #
  # from_io reads exactly one message from a stream and leaves
  # the stream positioned at the start of the next message.
  def test_stream_reading
    msg1 = Message.text(sender_id: "a", payload: "first")
    msg2 = Message.text(sender_id: "b", payload: "second")

    # Concatenate two messages into a single binary stream
    combined = msg1.to_bytes + msg2.to_bytes
    io = StringIO.new(combined)

    # Read first message
    restored1 = Message.from_io(io)
    assert_equal msg1.id, restored1.id
    assert_equal "first", restored1.payload_text

    # Read second message — stream should be positioned correctly
    restored2 = Message.from_io(io)
    assert_equal msg2.id, restored2.id
    assert_equal "second", restored2.payload_text

    # Reading past end should return nil
    assert_nil Message.from_io(io)
  end
end
