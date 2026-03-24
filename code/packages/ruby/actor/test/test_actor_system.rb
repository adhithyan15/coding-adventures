# frozen_string_literal: true

require_relative "test_helper"
require "tmpdir"

# ═══════════════════════════════════════════════════════════════
# ActorSystem Integration Tests
# ═══════════════════════════════════════════════════════════════
#
# These tests exercise the full ActorSystem with multiple actors
# communicating, dynamic actor creation, channel-based messaging,
# and persistence round-trips.
#
class TestActorSystem < Minitest::Test
  Message = CodingAdventures::Actor::Message
  ActorResult = CodingAdventures::Actor::ActorResult
  ActorSpec = CodingAdventures::Actor::ActorSpec
  ActorSystem = CodingAdventures::Actor::ActorSystem
  Channel = CodingAdventures::Actor::Channel

  # ─── Test 50: Ping-pong ────────────────────────────────────
  #
  # Two actors send messages back and forth. Each one replies
  # to every message it receives, up to a maximum count.
  def test_ping_pong
    # Ping behavior: reply with "ping" to the sender, up to 10 times
    ping_behavior = ->(state, msg) {
      count = state + 1
      if count <= 10
        reply = Message.text(sender_id: "pinger", payload: "ping_#{count}")
        ActorResult.new(
          new_state: count,
          messages_to_send: [["ponger", reply]]
        )
      else
        ActorResult.new(new_state: count)
      end
    }

    pong_behavior = ->(state, msg) {
      count = state + 1
      if count <= 10
        reply = Message.text(sender_id: "ponger", payload: "pong_#{count}")
        ActorResult.new(
          new_state: count,
          messages_to_send: [["pinger", reply]]
        )
      else
        ActorResult.new(new_state: count)
      end
    }

    system = ActorSystem.new
    system.create_actor("pinger", 0, ping_behavior)
    system.create_actor("ponger", 0, pong_behavior)

    # Start the rally
    system.send_message("pinger", Message.text(sender_id: "ponger", payload: "start"))
    stats = system.run_until_done

    assert stats[:messages_processed] > 0
    assert_equal 0, system.mailbox_size("pinger")
    assert_equal 0, system.mailbox_size("ponger")
  end

  # ─── Test 51: Pipeline ─────────────────────────────────────
  #
  # Three actors in a chain: A -> B -> C. A sends a message,
  # B transforms it and forwards to C.
  def test_pipeline
    received_by_c = []

    # B transforms: uppercases the text and forwards to C
    transform_behavior = ->(state, msg) {
      transformed = Message.text(
        sender_id: "transformer",
        payload: msg.payload_text.upcase
      )
      ActorResult.new(
        new_state: state,
        messages_to_send: [["consumer", transformed]]
      )
    }

    # C collects received messages
    consumer_behavior = ->(state, msg) {
      received_by_c << msg.payload_text
      ActorResult.new(new_state: state)
    }

    system = ActorSystem.new
    system.create_actor("transformer", nil, transform_behavior)
    system.create_actor("consumer", nil, consumer_behavior)

    # A sends to B
    system.send_message("transformer", Message.text(sender_id: "producer", payload: "hello world"))
    system.run_until_done

    assert_equal ["HELLO WORLD"], received_by_c
  end

  # ─── Test 52: Channel-based pipeline ────────────────────────
  def test_channel_based_pipeline
    system = ActorSystem.new
    channel = system.create_channel("ch_001", "pipeline")

    # Producer writes to channel
    3.times do |i|
      channel.append(Message.text(sender_id: "producer", payload: "item_#{i}"))
    end

    # Consumer reads from channel
    offset = 0
    batch = channel.read(offset: offset, limit: 10)

    assert_equal 3, batch.length
    batch.each_with_index do |msg, i|
      assert_equal "item_#{i}", msg.payload_text
    end
  end

  # ─── Test 53: Fan-out ──────────────────────────────────────
  #
  # One actor sends the same message to 5 different actors.
  def test_fan_out
    received_counts = {}

    system = ActorSystem.new

    # Create 5 receiver actors
    5.times do |i|
      actor_id = "receiver_#{i}"
      received_counts[actor_id] = 0

      receiver_behavior = ->(state, _msg) {
        ActorResult.new(new_state: state + 1)
      }

      system.create_actor(actor_id, 0, receiver_behavior)
    end

    # Fan-out actor sends to all 5 receivers
    fanout_behavior = ->(state, msg) {
      messages = 5.times.map do |i|
        ["receiver_#{i}", Message.text(sender_id: "fanout", payload: "broadcast")]
      end
      ActorResult.new(new_state: state, messages_to_send: messages)
    }

    system.create_actor("fanout", nil, fanout_behavior)

    system.send_message("fanout", Message.text(sender_id: "trigger", payload: "go"))
    system.run_until_done

    # Each receiver should have received 1 message
    5.times do |i|
      assert_equal 0, system.mailbox_size("receiver_#{i}"),
        "receiver_#{i} should have processed its message"
    end
  end

  # ─── Test 54: Dynamic topology ─────────────────────────────
  #
  # Actor A spawns Actor B, sends B a message, B responds.
  def test_dynamic_topology
    response_received = []

    # Spawner creates a worker and sends it a task
    spawner_behavior = ->(state, msg) {
      if msg.payload_text == "create_worker"
        worker_behavior = ->(wstate, wmsg) {
          reply = Message.text(sender_id: "worker", payload: "done: #{wmsg.payload_text}")
          ActorResult.new(
            new_state: wstate,
            messages_to_send: [["collector", reply]]
          )
        }

        ActorResult.new(
          new_state: state,
          actors_to_create: [
            ActorSpec.new(actor_id: "worker", initial_state: nil, behavior: worker_behavior)
          ],
          messages_to_send: [
            ["worker", Message.text(sender_id: "spawner", payload: "task_1")]
          ]
        )
      else
        ActorResult.new(new_state: state)
      end
    }

    collector_behavior = ->(state, msg) {
      response_received << msg.payload_text
      ActorResult.new(new_state: state)
    }

    system = ActorSystem.new
    system.create_actor("spawner", nil, spawner_behavior)
    system.create_actor("collector", nil, collector_behavior)

    system.send_message("spawner", Message.text(sender_id: "main", payload: "create_worker"))
    system.run_until_done

    assert_includes system.actor_ids, "worker"
    assert_equal ["done: task_1"], response_received
  end

  # ─── Test 55: Run until idle ────────────────────────────────
  #
  # Create a network of 5 actors with interconnected messaging.
  # run_until_idle should process everything.
  def test_run_until_idle
    system = ActorSystem.new

    # Create 5 actors, each forwards to the next (circular)
    5.times do |i|
      next_id = "actor_#{(i + 1) % 5}"
      actor_id = "actor_#{i}"

      behavior = ->(state, msg) {
        count = state + 1
        if count <= 2  # Each actor forwards at most twice
          fwd = Message.text(sender_id: actor_id, payload: "hop_#{count}")
          ActorResult.new(
            new_state: count,
            messages_to_send: [[next_id, fwd]]
          )
        else
          ActorResult.new(new_state: count)
        end
      }

      system.create_actor(actor_id, 0, behavior)
    end

    # Kick off with a message to actor_0
    system.send_message("actor_0", Message.text(sender_id: "trigger", payload: "start"))
    stats = system.run_until_idle

    assert stats[:messages_processed] > 0
    # All mailboxes should be empty
    5.times do |i|
      assert_equal 0, system.mailbox_size("actor_#{i}")
    end
  end

  # ─── Test 56: Persistence round-trip ────────────────────────
  def test_persistence_round_trip
    Dir.mktmpdir do |dir|
      system1 = ActorSystem.new
      channel = system1.create_channel("ch_persist", "persist-test")

      # Write messages including binary
      channel.append(Message.text(sender_id: "a", payload: "text message"))
      channel.append(Message.json(sender_id: "b", payload: {"result" => 42}))
      channel.append(Message.binary(
        sender_id: "c",
        content_type: "image/png",
        payload: "\x89PNG\r\n\x1a\n".b
      ))

      channel.persist(dir)

      # Recover in a new system
      recovered = Channel.recover(dir, "persist-test")
      assert_equal 3, recovered.length

      msgs = recovered.read(offset: 0, limit: 3)
      assert_equal "text message", msgs[0].payload_text
      assert_equal({"result" => 42}, msgs[1].payload_json)
      assert_equal "\x89PNG\r\n\x1a\n".b, msgs[2].payload
    end
  end

  # ─── Test 57: Large-scale ──────────────────────────────────
  #
  # Create 100 actors, send 1000 messages randomly, run until done.
  # Verify no messages are lost (all delivered or in dead letters).
  def test_large_scale
    system = ActorSystem.new

    # Create 100 simple actors
    100.times do |i|
      behavior = ->(state, _msg) {
        ActorResult.new(new_state: state + 1)
      }
      system.create_actor("actor_#{i}", 0, behavior)
    end

    # Send 1000 messages to random actors
    rng = Random.new(42)  # Fixed seed for reproducibility
    1000.times do |i|
      target = "actor_#{rng.rand(100)}"
      msg = Message.text(sender_id: "dispatcher", payload: "msg_#{i}")
      system.send_message(target, msg)
    end

    stats = system.run_until_done

    assert_equal 1000, stats[:messages_processed]
    assert_equal 0, system.dead_letters.length

    # All mailboxes should be empty
    100.times do |i|
      assert_equal 0, system.mailbox_size("actor_#{i}")
    end
  end

  # ─── Test 58: Binary message pipeline ──────────────────────
  #
  # Actor A sends a PNG image to Actor B via a channel.
  # Actor B receives and verifies the bytes are identical.
  def test_binary_message_pipeline
    # Simulated PNG data (8-byte header + some "pixel" data)
    png_data = "\x89PNG\r\n\x1a\n".b + SecureRandom.random_bytes(256)

    system = ActorSystem.new
    channel = system.create_channel("ch_images", "images")

    # Actor A appends a binary message to the channel
    img_msg = Message.binary(
      sender_id: "camera",
      content_type: "image/png",
      payload: png_data
    )
    channel.append(img_msg)

    # Actor B reads from the channel and verifies
    received = channel.read(offset: 0, limit: 1)
    assert_equal 1, received.length
    assert_equal "image/png", received[0].content_type
    assert_equal png_data, received[0].payload
    assert_equal png_data.bytesize, received[0].payload.bytesize
  end

  # ─── Additional: Shutdown ──────────────────────────────────
  def test_shutdown
    system = ActorSystem.new

    3.times do |i|
      behavior = ->(state, _msg) { ActorResult.new(new_state: state) }
      system.create_actor("actor_#{i}", nil, behavior)
      system.send_message("actor_#{i}", Message.text(sender_id: "s", payload: "pending"))
    end

    system.shutdown

    3.times do |i|
      assert_equal "stopped", system.get_actor_status("actor_#{i}")
    end

    # All pending messages should be in dead_letters
    assert_equal 3, system.dead_letters.length
  end

  # ─── Additional: Get channel ───────────────────────────────
  def test_get_channel
    system = ActorSystem.new
    system.create_channel("ch_1", "test-channel")

    channel = system.get_channel("ch_1")
    assert_equal "ch_1", channel.id
    assert_equal "test-channel", channel.name
  end

  # ─── Additional: Get nonexistent channel raises ────────────
  def test_get_nonexistent_channel
    system = ActorSystem.new

    assert_raises(ArgumentError) do
      system.get_channel("nonexistent")
    end
  end

  # ─── Additional: Get nonexistent actor status raises ────────
  def test_get_nonexistent_actor_status
    system = ActorSystem.new

    assert_raises(ArgumentError) do
      system.get_actor_status("nonexistent")
    end
  end

  # ─── Additional: Process next on empty mailbox ─────────────
  def test_process_next_empty_mailbox
    system = ActorSystem.new
    behavior = ->(state, _msg) { ActorResult.new(new_state: state) }
    system.create_actor("empty", nil, behavior)

    result = system.process_next("empty")
    refute result, "Should return false when mailbox is empty"
  end

  # ─── Additional: Process next on nonexistent actor ──────────
  def test_process_next_nonexistent
    system = ActorSystem.new
    result = system.process_next("ghost")
    refute result, "Should return false for nonexistent actor"
  end

  # ─── Additional: Mailbox size for nonexistent actor ─────────
  def test_mailbox_size_nonexistent
    system = ActorSystem.new
    assert_equal 0, system.mailbox_size("ghost")
  end
end
