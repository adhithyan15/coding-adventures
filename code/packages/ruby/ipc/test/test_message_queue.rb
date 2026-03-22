# frozen_string_literal: true

require_relative "test_helper"

# Tests for the MessageQueue class -- typed FIFO message passing.
#
# We test:
#   1. FIFO ordering (messages come out in the order they went in)
#   2. Typed receive (filtering by message type)
#   3. Queue full behavior
#   4. Oversized message rejection
#   5. Invalid message type rejection
#   6. Empty queue behavior
#   7. Count and state tracking
class TestMessageQueue < Minitest::Test
  # -- Basic send/receive --

  def test_send_and_receive_single_message
    mq = CodingAdventures::Ipc::MessageQueue.new
    assert mq.send(1, [65, 66, 67]) # type=1, body="ABC"

    msg = mq.receive
    assert_equal 1, msg.msg_type
    assert_equal [65, 66, 67], msg.body
  end

  def test_fifo_ordering
    mq = CodingAdventures::Ipc::MessageQueue.new
    mq.send(1, [1])
    mq.send(1, [2])
    mq.send(1, [3])

    assert_equal [1], mq.receive.body
    assert_equal [2], mq.receive.body
    assert_equal [3], mq.receive.body
  end

  # -- Typed receive --

  def test_receive_specific_type
    mq = CodingAdventures::Ipc::MessageQueue.new
    mq.send(1, [10])  # type 1
    mq.send(2, [20])  # type 2
    mq.send(1, [30])  # type 1

    # Request type 2 -- should skip the type-1 messages.
    msg = mq.receive(2)
    assert_equal 2, msg.msg_type
    assert_equal [20], msg.body

    # The two type-1 messages should still be in the queue.
    assert_equal 2, mq.count
  end

  def test_receive_type_returns_oldest_matching
    mq = CodingAdventures::Ipc::MessageQueue.new
    mq.send(1, [10])  # oldest type 1
    mq.send(2, [20])
    mq.send(1, [30])  # newer type 1

    msg = mq.receive(1)
    assert_equal [10], msg.body # oldest type-1 message
  end

  def test_receive_type_not_found_returns_nil
    mq = CodingAdventures::Ipc::MessageQueue.new
    mq.send(1, [10])

    msg = mq.receive(99) # no type-99 messages
    assert_nil msg
  end

  def test_receive_any_type_with_zero
    mq = CodingAdventures::Ipc::MessageQueue.new
    mq.send(5, [50])

    # type=0 means "give me any message."
    msg = mq.receive(0)
    assert_equal 5, msg.msg_type
    assert_equal [50], msg.body
  end

  # -- Queue full --

  def test_queue_full_rejects_new_messages
    mq = CodingAdventures::Ipc::MessageQueue.new(max_messages: 3)
    assert mq.send(1, [1])
    assert mq.send(1, [2])
    assert mq.send(1, [3])

    # Queue is now full -- next send should fail.
    refute mq.send(1, [4])
    assert mq.full?
  end

  # -- Oversized message --

  def test_oversized_message_rejected
    mq = CodingAdventures::Ipc::MessageQueue.new(max_message_size: 10)
    big_body = Array.new(11, 0)

    refute mq.send(1, big_body)
    assert_equal 0, mq.count
  end

  def test_message_at_max_size_accepted
    mq = CodingAdventures::Ipc::MessageQueue.new(max_message_size: 10)
    body = Array.new(10, 0)

    assert mq.send(1, body)
    assert_equal 1, mq.count
  end

  # -- Invalid message type --

  def test_invalid_type_zero_rejected
    mq = CodingAdventures::Ipc::MessageQueue.new
    refute mq.send(0, [1]) # type must be positive
  end

  def test_invalid_type_negative_rejected
    mq = CodingAdventures::Ipc::MessageQueue.new
    refute mq.send(-1, [1])
  end

  def test_invalid_type_non_integer_rejected
    mq = CodingAdventures::Ipc::MessageQueue.new
    refute mq.send("hello", [1])
  end

  # -- Empty queue --

  def test_receive_from_empty_queue_returns_nil
    mq = CodingAdventures::Ipc::MessageQueue.new
    assert_nil mq.receive
  end

  # -- State tracking --

  def test_count
    mq = CodingAdventures::Ipc::MessageQueue.new
    assert_equal 0, mq.count

    mq.send(1, [1])
    mq.send(2, [2])
    assert_equal 2, mq.count

    mq.receive
    assert_equal 1, mq.count
  end

  def test_empty_and_full
    mq = CodingAdventures::Ipc::MessageQueue.new(max_messages: 2)
    assert mq.empty?
    refute mq.full?

    mq.send(1, [1])
    refute mq.empty?
    refute mq.full?

    mq.send(1, [2])
    refute mq.empty?
    assert mq.full?
  end

  # -- Body is duplicated (not aliased) --

  def test_body_is_copied_on_send
    mq = CodingAdventures::Ipc::MessageQueue.new
    original = [1, 2, 3]
    mq.send(1, original)

    # Mutating the original should not affect the queued message.
    original[0] = 99
    msg = mq.receive
    assert_equal [1, 2, 3], msg.body
  end

  # -- Default limits --

  def test_default_max_messages
    mq = CodingAdventures::Ipc::MessageQueue.new
    assert_equal 256, mq.max_messages
  end

  def test_default_max_message_size
    mq = CodingAdventures::Ipc::MessageQueue.new
    assert_equal 4096, mq.max_message_size
  end
end
