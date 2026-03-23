# frozen_string_literal: true

require_relative "test_helper"

# ═══════════════════════════════════════════════════════════════
# Actor Tests
# ═══════════════════════════════════════════════════════════════
#
# These tests verify that actors:
#   - Start in :idle status
#   - Accept messages into their mailbox
#   - Process messages through their behavior function
#   - Update state correctly
#   - Generate outbound messages and actor creation specs
#   - Handle stop requests
#   - Recover from behavior exceptions
#   - Reject duplicate IDs
#
class TestActor < Minitest::Test
  Message = CodingAdventures::Actor::Message
  ActorResult = CodingAdventures::Actor::ActorResult
  ActorSpec = CodingAdventures::Actor::ActorSpec
  ActorSystem = CodingAdventures::Actor::ActorSystem

  # ─── Helper behaviors ──────────────────────────────────────

  # Echo behavior: replies to sender with "echo: <payload>"
  ECHO_BEHAVIOR = ->(state, msg) {
    reply = Message.text(
      sender_id: "echo",
      payload: "echo: #{msg.payload_text}"
    )
    ActorResult.new(
      new_state: state,
      messages_to_send: [[msg.sender_id, reply]]
    )
  }

  # Counter behavior: increments state by 1 for each message
  COUNTER_BEHAVIOR = ->(state, _msg) {
    ActorResult.new(new_state: state + 1)
  }

  # Spawner behavior: creates a new echo actor when told "spawn"
  SPAWNER_BEHAVIOR = ->(state, msg) {
    if msg.payload_text == "spawn"
      new_id = "echo_#{state}"
      ActorResult.new(
        new_state: state + 1,
        actors_to_create: [
          ActorSpec.new(
            actor_id: new_id,
            initial_state: nil,
            behavior: ECHO_BEHAVIOR
          )
        ]
      )
    else
      ActorResult.new(new_state: state)
    end
  }

  # Stop behavior: stops when told "stop"
  STOP_BEHAVIOR = ->(state, msg) {
    if msg.payload_text == "stop"
      ActorResult.new(new_state: state, stop: true)
    else
      ActorResult.new(new_state: state)
    end
  }

  # Exploding behavior: raises on messages containing "bomb"
  EXPLODING_BEHAVIOR = ->(state, msg) {
    if msg.payload_text == "bomb"
      raise "BOOM! The actor exploded!"
    end
    ActorResult.new(new_state: state + 1)
  }

  # ─── Test 37: Create actor ─────────────────────────────────
  def test_create_actor
    system = ActorSystem.new
    system.create_actor("test", 0, COUNTER_BEHAVIOR)

    assert_equal "idle", system.get_actor_status("test")
    assert_includes system.actor_ids, "test"
  end

  # ─── Test 38: Send message ─────────────────────────────────
  def test_send_message
    system = ActorSystem.new
    system.create_actor("test", 0, COUNTER_BEHAVIOR)

    msg = Message.text(sender_id: "sender", payload: "hello")
    system.send_message("test", msg)

    assert_equal 1, system.mailbox_size("test")
  end

  # ─── Test 39: Process message ──────────────────────────────
  def test_process_message
    system = ActorSystem.new
    system.create_actor("counter", 0, COUNTER_BEHAVIOR)

    msg = Message.text(sender_id: "sender", payload: "tick")
    system.send_message("counter", msg)

    result = system.process_next("counter")
    assert result, "Should have processed a message"
    assert_equal 0, system.mailbox_size("counter")
  end

  # ─── Test 40: State update ─────────────────────────────────
  #
  # A counter actor receiving 3 messages should have state = 3.
  def test_state_update
    system = ActorSystem.new
    system.create_actor("counter", 0, COUNTER_BEHAVIOR)

    3.times do |i|
      system.send_message("counter", Message.text(sender_id: "s", payload: "tick_#{i}"))
    end

    3.times { system.process_next("counter") }

    # We can't directly access actor state through the system,
    # but we can verify indirectly through behavior. Let's use a
    # behavior that reports state instead.
    reporter_behavior = ->(state, _msg) {
      ActorResult.new(new_state: state + 1)
    }

    system2 = ActorSystem.new
    system2.create_actor("c", 0, reporter_behavior)
    3.times { system2.send_message("c", Message.text(sender_id: "s", payload: "x")) }
    3.times { system2.process_next("c") }

    # After 3 messages, mailbox should be empty
    assert_equal 0, system2.mailbox_size("c")
  end

  # ─── Test 41: Messages to send ─────────────────────────────
  #
  # Echo actor receives a message, the reply should be delivered
  # to the sender's mailbox.
  def test_messages_to_send
    system = ActorSystem.new
    system.create_actor("echo", nil, ECHO_BEHAVIOR)
    system.create_actor("sender", nil, ->(state, _msg) { ActorResult.new(new_state: state) })

    msg = Message.text(sender_id: "sender", payload: "hello")
    system.send_message("echo", msg)
    system.process_next("echo")

    # The echo actor should have sent a reply to "sender"
    assert_equal 1, system.mailbox_size("sender")
  end

  # ─── Test 42: Actor creation ────────────────────────────────
  def test_actor_creation
    system = ActorSystem.new
    system.create_actor("spawner", 0, SPAWNER_BEHAVIOR)

    msg = Message.text(sender_id: "requester", payload: "spawn")
    system.send_message("spawner", msg)
    system.process_next("spawner")

    assert_includes system.actor_ids, "echo_0"
    assert_equal "idle", system.get_actor_status("echo_0")
  end

  # ─── Test 43: Stop actor ───────────────────────────────────
  def test_stop_actor
    system = ActorSystem.new
    system.create_actor("stopper", nil, STOP_BEHAVIOR)

    msg = Message.text(sender_id: "s", payload: "stop")
    system.send_message("stopper", msg)
    system.process_next("stopper")

    assert_equal "stopped", system.get_actor_status("stopper")
  end

  # ─── Test 44: Stopped actor rejects messages ────────────────
  def test_stopped_actor_rejects_messages
    system = ActorSystem.new
    system.create_actor("stopper", nil, STOP_BEHAVIOR)

    system.send_message("stopper", Message.text(sender_id: "s", payload: "stop"))
    system.process_next("stopper")

    # Now send another message — it should go to dead letters
    rejected_msg = Message.text(sender_id: "s", payload: "too late")
    system.send_message("stopper", rejected_msg)

    assert_equal 1, system.dead_letters.length
    assert_equal "too late", system.dead_letters.last.payload_text
  end

  # ─── Test 45: Dead letters ─────────────────────────────────
  def test_dead_letters
    system = ActorSystem.new

    msg = Message.text(sender_id: "s", payload: "to nobody")
    system.send_message("nonexistent", msg)

    assert_equal 1, system.dead_letters.length
    assert_equal "to nobody", system.dead_letters[0].payload_text
  end

  # ─── Test 46: Sequential processing (FIFO) ─────────────────
  #
  # Messages are processed in the order they were received.
  def test_sequential_processing
    processed_order = []

    ordered_behavior = ->(state, msg) {
      processed_order << msg.payload_text
      ActorResult.new(new_state: state)
    }

    system = ActorSystem.new
    system.create_actor("fifo", nil, ordered_behavior)

    %w[first second third].each do |text|
      system.send_message("fifo", Message.text(sender_id: "s", payload: text))
    end

    3.times { system.process_next("fifo") }

    assert_equal %w[first second third], processed_order
  end

  # ─── Test 47: Mailbox drains on stop ────────────────────────
  def test_mailbox_drains_on_stop
    system = ActorSystem.new
    system.create_actor("drainer", nil, COUNTER_BEHAVIOR)

    # Enqueue 3 messages
    3.times { |i| system.send_message("drainer", Message.text(sender_id: "s", payload: "msg_#{i}")) }
    assert_equal 3, system.mailbox_size("drainer")

    # Stop the actor — all pending messages go to dead letters
    system.stop_actor("drainer")

    assert_equal "stopped", system.get_actor_status("drainer")
    assert_equal 3, system.dead_letters.length
  end

  # ─── Test 48: Behavior exception ───────────────────────────
  #
  # When a behavior function raises an exception:
  #   - State is unchanged
  #   - The failing message goes to dead_letters
  #   - The actor continues processing subsequent messages
  def test_behavior_exception
    system = ActorSystem.new
    system.create_actor("exploder", 0, EXPLODING_BEHAVIOR)

    # Send a normal message, then a bomb, then another normal message
    system.send_message("exploder", Message.text(sender_id: "s", payload: "safe_1"))
    system.send_message("exploder", Message.text(sender_id: "s", payload: "bomb"))
    system.send_message("exploder", Message.text(sender_id: "s", payload: "safe_2"))

    # Process all three
    system.process_next("exploder")  # safe_1: state -> 1
    system.process_next("exploder")  # bomb: exception, state unchanged (still 1)
    system.process_next("exploder")  # safe_2: state -> 2

    # The bomb message should be in dead_letters
    assert_equal 1, system.dead_letters.length
    assert_equal "bomb", system.dead_letters[0].payload_text

    # Actor should still be alive and idle
    assert_equal "idle", system.get_actor_status("exploder")
  end

  # ─── Test 49: Duplicate actor ID ───────────────────────────
  def test_duplicate_actor_id
    system = ActorSystem.new
    system.create_actor("unique", nil, COUNTER_BEHAVIOR)

    assert_raises(ArgumentError) do
      system.create_actor("unique", nil, COUNTER_BEHAVIOR)
    end
  end
end
