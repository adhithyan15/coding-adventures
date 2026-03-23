# frozen_string_literal: true

require_relative "test_helper"
require "tmpdir"

# ═══════════════════════════════════════════════════════════════
# Channel Tests
# ═══════════════════════════════════════════════════════════════
#
# These tests verify that Channel:
#   - Creates with correct id and name
#   - Appends messages and tracks sequence numbers
#   - Reads with offset and limit correctly
#   - Slices the log
#   - Persists to disk in binary format
#   - Recovers from disk files, including crash-truncated files
#
class TestChannel < Minitest::Test
  Message = CodingAdventures::Actor::Message
  Channel = CodingAdventures::Actor::Channel

  # Helper: create a simple text message
  def make_msg(text, sender: "test")
    Message.text(sender_id: sender, payload: text)
  end

  # ─── Test 20: Create channel ────────────────────────────────
  def test_create_channel
    ch = Channel.new(channel_id: "ch_001", name: "greetings")

    assert_equal "ch_001", ch.id
    assert_equal "greetings", ch.name
    assert_kind_of Integer, ch.created_at
    assert_equal 0, ch.length
  end

  # ─── Test 21: Append and length ─────────────────────────────
  def test_append_and_length
    ch = Channel.new(channel_id: "ch_001", name: "test")

    ch.append(make_msg("one"))
    ch.append(make_msg("two"))
    ch.append(make_msg("three"))

    assert_equal 3, ch.length
  end

  # ─── Test 22: Append returns sequence number ────────────────
  #
  # Sequence numbers are 0-indexed and monotonically increasing.
  def test_append_returns_sequence_number
    ch = Channel.new(channel_id: "ch_001", name: "test")

    assert_equal 0, ch.append(make_msg("first"))
    assert_equal 1, ch.append(make_msg("second"))
    assert_equal 2, ch.append(make_msg("third"))
  end

  # ─── Test 23: Read from beginning ──────────────────────────
  def test_read_from_beginning
    ch = Channel.new(channel_id: "ch_001", name: "test")
    5.times { |i| ch.append(make_msg("msg_#{i}")) }

    messages = ch.read(offset: 0, limit: 5)

    assert_equal 5, messages.length
    messages.each_with_index do |msg, i|
      assert_equal "msg_#{i}", msg.payload_text
    end
  end

  # ─── Test 24: Read with offset ──────────────────────────────
  def test_read_with_offset
    ch = Channel.new(channel_id: "ch_001", name: "test")
    5.times { |i| ch.append(make_msg("msg_#{i}")) }

    messages = ch.read(offset: 2, limit: 3)

    assert_equal 3, messages.length
    assert_equal "msg_2", messages[0].payload_text
    assert_equal "msg_3", messages[1].payload_text
    assert_equal "msg_4", messages[2].payload_text
  end

  # ─── Test 25: Read past end ────────────────────────────────
  #
  # Reading past the end of the log returns an empty array —
  # the consumer is caught up.
  def test_read_past_end
    ch = Channel.new(channel_id: "ch_001", name: "test")
    3.times { |i| ch.append(make_msg("msg_#{i}")) }

    messages = ch.read(offset: 5, limit: 10)
    assert_equal [], messages
  end

  # ─── Test 26: Read with limit ──────────────────────────────
  def test_read_with_limit
    ch = Channel.new(channel_id: "ch_001", name: "test")
    10.times { |i| ch.append(make_msg("msg_#{i}")) }

    messages = ch.read(offset: 0, limit: 3)
    assert_equal 3, messages.length
  end

  # ─── Test 27: Slice ─────────────────────────────────────────
  def test_slice
    ch = Channel.new(channel_id: "ch_001", name: "test")
    5.times { |i| ch.append(make_msg("msg_#{i}")) }

    sliced = ch.slice(1, 4)

    assert_equal 3, sliced.length
    assert_equal "msg_1", sliced[0].payload_text
    assert_equal "msg_2", sliced[1].payload_text
    assert_equal "msg_3", sliced[2].payload_text
  end

  # ─── Test 28: Independent readers ──────────────────────────
  #
  # Two consumers reading the same channel at different offsets
  # get different views of the log.
  def test_independent_readers
    ch = Channel.new(channel_id: "ch_001", name: "test")
    5.times { |i| ch.append(make_msg("msg_#{i}")) }

    # Consumer A reads from offset 3
    batch_a = ch.read(offset: 3, limit: 10)
    assert_equal 2, batch_a.length
    assert_equal "msg_3", batch_a[0].payload_text

    # Consumer B reads from offset 0
    batch_b = ch.read(offset: 0, limit: 2)
    assert_equal 2, batch_b.length
    assert_equal "msg_0", batch_b[0].payload_text
  end

  # ─── Test 29: Append-only ──────────────────────────────────
  #
  # Verify there is no method to delete or modify messages.
  def test_append_only
    ch = Channel.new(channel_id: "ch_001", name: "test")

    refute ch.respond_to?(:delete)
    refute ch.respond_to?(:remove)
    refute ch.respond_to?(:update)
    refute ch.respond_to?(:insert)
  end

  # ─── Test 30: Binary persistence ───────────────────────────
  #
  # Persist a channel to disk, verify the file starts with "ACTM"
  # magic bytes.
  def test_binary_persistence
    Dir.mktmpdir do |dir|
      ch = Channel.new(channel_id: "ch_001", name: "test-channel")
      ch.append(make_msg("hello"))
      ch.append(Message.binary(
        sender_id: "cam",
        content_type: "image/png",
        payload: "\x89PNG\r\n\x1a\n".b
      ))

      ch.persist(dir)

      path = File.join(dir, "test-channel.log")
      assert File.exist?(path), "Persisted log file should exist"

      data = File.binread(path)
      assert_equal "ACTM", data[0, 4], "File should start with ACTM magic"
    end
  end

  # ─── Test 31: Recovery ─────────────────────────────────────
  #
  # Persist a channel, recover it from disk, verify all messages
  # are restored.
  def test_recovery
    Dir.mktmpdir do |dir|
      ch = Channel.new(channel_id: "ch_001", name: "recovery-test")
      ch.append(make_msg("first"))
      ch.append(make_msg("second"))
      ch.append(Message.binary(
        sender_id: "bin",
        content_type: "application/octet-stream",
        payload: "\x00\x01\x02\xFF".b
      ))

      ch.persist(dir)

      recovered = Channel.recover(dir, "recovery-test")
      assert_equal 3, recovered.length
      assert_equal "first", recovered.read[0].payload_text
      assert_equal "second", recovered.read[1].payload_text
      assert_equal "\x00\x01\x02\xFF".b, recovered.read[2].payload
    end
  end

  # ─── Test 32: Recovery preserves order ──────────────────────
  def test_recovery_preserves_order
    Dir.mktmpdir do |dir|
      ch = Channel.new(channel_id: "ch_001", name: "order-test")
      100.times { |i| ch.append(make_msg("msg_#{i}")) }

      ch.persist(dir)

      recovered = Channel.recover(dir, "order-test")
      assert_equal 100, recovered.length

      recovered.read(offset: 0, limit: 100).each_with_index do |msg, i|
        assert_equal "msg_#{i}", msg.payload_text
      end
    end
  end

  # ─── Test 33: Empty channel recovery ────────────────────────
  #
  # Recovering from a non-existent file returns an empty channel.
  def test_empty_channel_recovery
    Dir.mktmpdir do |dir|
      recovered = Channel.recover(dir, "nonexistent")
      assert_equal 0, recovered.length
    end
  end

  # ─── Test 34: Mixed content recovery ────────────────────────
  def test_mixed_content_recovery
    Dir.mktmpdir do |dir|
      ch = Channel.new(channel_id: "ch_001", name: "mixed")

      ch.append(Message.text(sender_id: "a", payload: "hello"))
      ch.append(Message.json(sender_id: "b", payload: {"key" => "value"}))
      ch.append(Message.binary(
        sender_id: "c",
        content_type: "image/png",
        payload: "\x89PNG\r\n\x1a\n".b
      ))

      ch.persist(dir)

      recovered = Channel.recover(dir, "mixed")
      assert_equal 3, recovered.length

      msgs = recovered.read(offset: 0, limit: 3)
      assert_equal "text/plain", msgs[0].content_type
      assert_equal "hello", msgs[0].payload_text

      assert_equal "application/json", msgs[1].content_type
      assert_equal({"key" => "value"}, msgs[1].payload_json)

      assert_equal "image/png", msgs[2].content_type
      assert_equal "\x89PNG\r\n\x1a\n".b, msgs[2].payload
    end
  end

  # ─── Test 35: Truncated write recovery ─────────────────────
  #
  # Simulate a crash mid-write by truncating the file in the
  # middle of the last message. Recovery should restore all
  # complete messages and discard the partial one.
  def test_truncated_write_recovery
    Dir.mktmpdir do |dir|
      ch = Channel.new(channel_id: "ch_001", name: "truncated")
      ch.append(make_msg("complete_1"))
      ch.append(make_msg("complete_2"))
      ch.append(make_msg("will_be_truncated"))

      ch.persist(dir)

      path = File.join(dir, "truncated.log")

      # To simulate a crash mid-write, we need to figure out where
      # the third message starts, then truncate partway through it.
      # We serialize the first two messages to find the boundary.
      msg1_bytes = ch.read(offset: 0, limit: 1)[0].to_bytes
      msg2_bytes = ch.read(offset: 1, limit: 1)[0].to_bytes
      boundary = msg1_bytes.bytesize + msg2_bytes.bytesize

      # Truncate a few bytes into the third message's header
      File.truncate(path, boundary + 5)

      recovered = Channel.recover(dir, "truncated")
      # Should recover only the first two complete messages
      assert_equal 2, recovered.length
      assert_equal "complete_1", recovered.read[0].payload_text
      assert_equal "complete_2", recovered.read[1].payload_text
    end
  end

  # ─── Test 36: Slice edge cases ─────────────────────────────
  #
  # Slicing past the end or with start >= length returns empty.
  def test_slice_edge_cases
    ch = Channel.new(channel_id: "ch_001", name: "test")
    3.times { |i| ch.append(make_msg("msg_#{i}")) }

    assert_equal [], ch.slice(5, 10)
    assert_equal 3, ch.slice(0, 10).length
  end
end
