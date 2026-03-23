defmodule CodingAdventures.ActorTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Actor.{Message, Channel, ActorResult, ActorSpec, ActorSystem}

  # ============================================================================
  # Unit Tests — Message (Tests 1-19)
  # ============================================================================

  describe "Message" do
    # Test 1: Create message with all fields
    test "create message with all fields" do
      msg = Message.new("actor_1", "text/plain", "hello", %{"key" => "val"})
      assert msg.sender_id == "actor_1"
      assert msg.content_type == "text/plain"
      assert msg.payload == "hello"
      assert msg.metadata == %{"key" => "val"}
      assert is_binary(msg.id)
      assert String.starts_with?(msg.id, "msg_")
      assert is_integer(msg.timestamp)
    end

    # Test 2: Immutability — Elixir structs are immutable by nature.
    # There are no setter methods. Attempting to use Map.put on a struct
    # creates a NEW struct; the original is unchanged.
    test "immutability — original struct unchanged after update" do
      msg = Message.text("actor_1", "hello")
      _modified = %{msg | payload: "different"}
      # Original is unchanged
      assert msg.payload == "hello"
    end

    # Test 3: Unique IDs — 1000 messages should all have unique IDs
    test "unique IDs for 1000 messages" do
      ids =
        for _i <- 1..1000 do
          msg = Message.text("actor", "hello")
          msg.id
        end

      assert length(Enum.uniq(ids)) == 1000
    end

    # Test 4: Timestamp ordering — sequential messages have increasing timestamps
    test "timestamp ordering is strictly increasing" do
      msgs = for _i <- 1..100, do: Message.text("actor", "hello")
      timestamps = Enum.map(msgs, & &1.timestamp)

      # Each timestamp should be greater than the previous
      timestamps
      |> Enum.chunk_every(2, 1, :discard)
      |> Enum.each(fn [a, b] -> assert b > a end)
    end

    # Test 5: Wire format round-trip (text)
    test "wire format round-trip for text message" do
      msg = Message.text("agent", "hello world", %{"trace" => "abc"})
      bytes = Message.to_bytes(msg)
      {:ok, decoded} = Message.from_bytes(bytes)

      assert decoded.id == msg.id
      assert decoded.timestamp == msg.timestamp
      assert decoded.sender_id == msg.sender_id
      assert decoded.content_type == msg.content_type
      assert decoded.payload == msg.payload
      assert decoded.metadata == msg.metadata
    end

    # Test 6: Wire format round-trip (binary — PNG header)
    test "wire format round-trip for binary message" do
      png_header = <<137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13>>
      msg = Message.binary("browser", "image/png", png_header)
      bytes = Message.to_bytes(msg)
      {:ok, decoded} = Message.from_bytes(bytes)

      assert decoded.payload == png_header
      assert decoded.content_type == "image/png"
    end

    # Test 7: Metadata passthrough
    test "metadata preserved across serialization" do
      meta = %{"correlation_id" => "req_abc123", "priority" => "high"}
      msg = Message.text("agent", "hello", meta)
      bytes = Message.to_bytes(msg)
      {:ok, decoded} = Message.from_bytes(bytes)

      assert decoded.metadata == meta
    end

    # Test 8: Empty payload
    test "empty payload works" do
      msg = Message.new("actor", "application/octet-stream", "")
      assert msg.payload == ""
      bytes = Message.to_bytes(msg)
      {:ok, decoded} = Message.from_bytes(bytes)
      assert decoded.payload == ""
    end

    # Test 9: Large payload (1MB)
    test "large payload serialization works" do
      large_payload = :crypto.strong_rand_bytes(1_000_000)
      msg = Message.binary("actor", "application/octet-stream", large_payload)
      bytes = Message.to_bytes(msg)
      {:ok, decoded} = Message.from_bytes(bytes)
      assert decoded.payload == large_payload
    end

    # Test 10: Content type preserved
    test "content type preserved across serialization" do
      msg = Message.binary("actor", "video/mp4", "fake_video_data")
      bytes = Message.to_bytes(msg)
      {:ok, decoded} = Message.from_bytes(bytes)
      assert decoded.content_type == "video/mp4"
    end

    # Test 11: Convenience constructors
    test "convenience constructors set correct content types" do
      text_msg = Message.text("a", "hello")
      assert text_msg.content_type == "text/plain"
      assert text_msg.payload == "hello"

      json_msg = Message.json("a", %{"key" => "value"})
      assert json_msg.content_type == "application/json"
      assert is_binary(json_msg.payload)

      bin_msg = Message.binary("a", "image/jpeg", <<0xFF, 0xD8>>)
      assert bin_msg.content_type == "image/jpeg"
      assert bin_msg.payload == <<0xFF, 0xD8>>
    end

    # Test 12: payload_text
    test "payload_text returns decoded string" do
      msg = Message.text("agent", "hello world")
      assert Message.payload_text(msg) == "hello world"
    end

    # Test 13: payload_json
    test "payload_json returns parsed map" do
      msg = Message.json("agent", %{"key" => "value"})
      {:ok, parsed} = Message.payload_json(msg)
      assert parsed == %{"key" => "value"}
    end

    # Test 14: Envelope-only serialization
    test "envelope_to_json produces JSON without payload" do
      msg = Message.text("agent", "big payload here")
      envelope = Message.envelope_to_json(msg)

      assert String.contains?(envelope, "agent")
      assert String.contains?(envelope, msg.id)
      refute String.contains?(envelope, "big payload here")
    end

    # Test 15: Wire format magic
    test "to_bytes starts with ACTM magic bytes" do
      msg = Message.text("a", "hello")
      bytes = Message.to_bytes(msg)
      assert <<magic::binary-size(4), _rest::binary>> = bytes
      assert magic == "ACTM"
    end

    # Test 16: Wire format version
    test "to_bytes contains correct version byte" do
      msg = Message.text("a", "hello")
      bytes = Message.to_bytes(msg)
      assert <<"ACTM", version::8, _rest::binary>> = bytes
      assert version == Message.wire_version()
    end

    # Test 17: Future version rejection
    test "from_bytes rejects future versions" do
      msg = Message.text("a", "hello")
      bytes = Message.to_bytes(msg)

      # Corrupt the version byte to a future version
      <<"ACTM", _version::8, after_version::binary>> = bytes
      corrupted = <<"ACTM", 99::8, after_version::binary>>

      assert {:error, {:unsupported_version, 99}} = Message.from_bytes(corrupted)
    end

    # Test 18: Corrupt magic rejection
    test "from_bytes rejects wrong magic" do
      corrupted = <<"XXXX", 1::8, 0::32-big, 0::64-big>>
      assert {:error, :invalid_format} = Message.from_bytes(corrupted)
    end

    # Test 19: Stream reading (from_io_device)
    test "from_io_device reads one message from a stream" do
      msg1 = Message.text("a", "first")
      msg2 = Message.text("a", "second")
      combined = Message.to_bytes(msg1) <> Message.to_bytes(msg2)

      # Write to a temp file and read back
      path = Path.join(System.tmp_dir!(), "actor_test_stream_#{:rand.uniform(100_000)}")

      try do
        File.write!(path, combined)
        {:ok, device} = File.open(path, [:read, :binary])

        {:ok, read1} = Message.from_io_device(device)
        assert read1.payload == "first"

        {:ok, read2} = Message.from_io_device(device)
        assert read2.payload == "second"

        assert :eof = Message.from_io_device(device)

        File.close(device)
      after
        File.rm(path)
      end
    end
  end

  # ============================================================================
  # Unit Tests — Channel (Tests 20-36)
  # ============================================================================

  describe "Channel" do
    # Test 20: Create channel
    test "create channel with id and name" do
      ch = Channel.new("ch_001", "greetings")
      assert ch.id == "ch_001"
      assert ch.name == "greetings"
      assert Channel.length(ch) == 0
    end

    # Test 21: Append and length
    test "append 3 messages, length returns 3" do
      ch = Channel.new("ch_001", "test")
      msg1 = Message.text("a", "one")
      msg2 = Message.text("a", "two")
      msg3 = Message.text("a", "three")

      {ch, _} = Channel.append(ch, msg1)
      {ch, _} = Channel.append(ch, msg2)
      {ch, _} = Channel.append(ch, msg3)

      assert Channel.length(ch) == 3
    end

    # Test 22: Append returns sequence numbers
    test "append returns sequential sequence numbers" do
      ch = Channel.new("ch_001", "test")
      msg = Message.text("a", "hello")

      {ch, seq0} = Channel.append(ch, msg)
      {ch, seq1} = Channel.append(ch, msg)
      {_ch, seq2} = Channel.append(ch, msg)

      assert seq0 == 0
      assert seq1 == 1
      assert seq2 == 2
    end

    # Test 23: Read from beginning
    test "read all 5 messages from beginning" do
      ch = Channel.new("ch_001", "test")
      msgs = for i <- 1..5, do: Message.text("a", "msg_#{i}")

      ch = Enum.reduce(msgs, ch, fn msg, acc ->
        {new_ch, _} = Channel.append(acc, msg)
        new_ch
      end)

      read_msgs = Channel.read(ch, 0, 5)
      assert length(read_msgs) == 5

      payloads = Enum.map(read_msgs, & &1.payload)
      assert payloads == ["msg_1", "msg_2", "msg_3", "msg_4", "msg_5"]
    end

    # Test 24: Read with offset
    test "read with offset returns correct subset" do
      ch = Channel.new("ch_001", "test")
      msgs = for i <- 1..5, do: Message.text("a", "msg_#{i}")

      ch = Enum.reduce(msgs, ch, fn msg, acc ->
        {new_ch, _} = Channel.append(acc, msg)
        new_ch
      end)

      read_msgs = Channel.read(ch, 2, 3)
      assert length(read_msgs) == 3

      payloads = Enum.map(read_msgs, & &1.payload)
      assert payloads == ["msg_3", "msg_4", "msg_5"]
    end

    # Test 25: Read past end
    test "read past end returns empty list" do
      ch = Channel.new("ch_001", "test")
      msgs = for _i <- 1..3, do: Message.text("a", "hello")

      ch = Enum.reduce(msgs, ch, fn msg, acc ->
        {new_ch, _} = Channel.append(acc, msg)
        new_ch
      end)

      assert Channel.read(ch, 5, 10) == []
    end

    # Test 26: Read with limit
    test "read with limit returns only requested number" do
      ch = Channel.new("ch_001", "test")
      msgs = for i <- 1..10, do: Message.text("a", "msg_#{i}")

      ch = Enum.reduce(msgs, ch, fn msg, acc ->
        {new_ch, _} = Channel.append(acc, msg)
        new_ch
      end)

      read_msgs = Channel.read(ch, 0, 3)
      assert length(read_msgs) == 3
    end

    # Test 27: Slice
    test "channel_slice returns correct range" do
      ch = Channel.new("ch_001", "test")
      msgs = for i <- 1..5, do: Message.text("a", "msg_#{i}")

      ch = Enum.reduce(msgs, ch, fn msg, acc ->
        {new_ch, _} = Channel.append(acc, msg)
        new_ch
      end)

      sliced = Channel.channel_slice(ch, 1, 4)
      assert length(sliced) == 3

      payloads = Enum.map(sliced, & &1.payload)
      assert payloads == ["msg_2", "msg_3", "msg_4"]
    end

    # Test 28: Independent readers
    test "two consumers read independently at different offsets" do
      ch = Channel.new("ch_001", "test")
      msgs = for i <- 1..5, do: Message.text("a", "msg_#{i}")

      ch = Enum.reduce(msgs, ch, fn msg, acc ->
        {new_ch, _} = Channel.append(acc, msg)
        new_ch
      end)

      # Consumer A reads from offset 4
      consumer_a = Channel.read(ch, 4, 10)
      assert length(consumer_a) == 1
      assert hd(consumer_a).payload == "msg_5"

      # Consumer B reads from offset 1
      consumer_b = Channel.read(ch, 1, 2)
      assert length(consumer_b) == 2
      payloads = Enum.map(consumer_b, & &1.payload)
      assert payloads == ["msg_2", "msg_3"]
    end

    # Test 29: Append-only — verify no delete/modify methods exist
    test "channel has no delete or modify operations" do
      functions = Channel.__info__(:functions)
      function_names = Enum.map(functions, fn {name, _arity} -> name end)

      refute :delete in function_names
      refute :remove in function_names
      refute :update in function_names
      refute :modify in function_names
      refute :insert_at in function_names
    end

    # Test 30: Binary persistence
    test "persist writes file starting with ACTM magic" do
      dir = Path.join(System.tmp_dir!(), "actor_test_persist_#{:rand.uniform(100_000)}")

      try do
        ch = Channel.new("ch_001", "test_persist")
        msg = Message.text("agent", "hello world")
        {ch, _} = Channel.append(ch, msg)

        Channel.persist(ch, dir)

        path = Path.join(dir, "test_persist.log")
        assert File.exists?(path)

        {:ok, data} = File.read(path)
        assert <<magic::binary-size(4), _rest::binary>> = data
        assert magic == "ACTM"
      after
        File.rm_rf(dir)
      end
    end

    # Test 31: Recovery
    test "persist and recover restores all messages" do
      dir = Path.join(System.tmp_dir!(), "actor_test_recovery_#{:rand.uniform(100_000)}")

      try do
        ch = Channel.new("ch_001", "test_recovery")
        msg1 = Message.text("a", "hello")
        msg2 = Message.binary("b", "image/png", <<137, 80, 78, 71>>)

        {ch, _} = Channel.append(ch, msg1)
        {ch, _} = Channel.append(ch, msg2)

        Channel.persist(ch, dir)

        recovered = Channel.recover(dir, "test_recovery")
        assert Channel.length(recovered) == 2

        [r1, r2] = Channel.read(recovered, 0, 10)
        assert r1.payload == "hello"
        assert r1.sender_id == "a"
        assert r2.payload == <<137, 80, 78, 71>>
        assert r2.content_type == "image/png"
      after
        File.rm_rf(dir)
      end
    end

    # Test 32: Recovery preserves order
    test "recovery preserves message order for 100 messages" do
      dir = Path.join(System.tmp_dir!(), "actor_test_order_#{:rand.uniform(100_000)}")

      try do
        ch = Channel.new("ch_001", "test_order")

        ch =
          Enum.reduce(1..100, ch, fn i, acc ->
            msg = Message.text("a", "msg_#{i}")
            {new_ch, _} = Channel.append(acc, msg)
            new_ch
          end)

        Channel.persist(ch, dir)
        recovered = Channel.recover(dir, "test_order")
        assert Channel.length(recovered) == 100

        recovered_msgs = Channel.read(recovered, 0, 100)
        payloads = Enum.map(recovered_msgs, & &1.payload)
        expected = for i <- 1..100, do: "msg_#{i}"
        assert payloads == expected
      after
        File.rm_rf(dir)
      end
    end

    # Test 33: Empty channel recovery
    test "recover from non-existent file returns empty channel" do
      dir = Path.join(System.tmp_dir!(), "actor_test_noexist_#{:rand.uniform(100_000)}")
      recovered = Channel.recover(dir, "nonexistent")
      assert Channel.length(recovered) == 0
    end

    # Test 34: Mixed content recovery
    test "mixed content types persist and recover correctly" do
      dir = Path.join(System.tmp_dir!(), "actor_test_mixed_#{:rand.uniform(100_000)}")

      try do
        ch = Channel.new("ch_001", "test_mixed")
        msg_text = Message.text("a", "plain text")
        msg_json = Message.json("b", %{"key" => "value"})
        msg_bin = Message.binary("c", "image/png", <<137, 80, 78, 71, 13, 10, 26, 10>>)

        {ch, _} = Channel.append(ch, msg_text)
        {ch, _} = Channel.append(ch, msg_json)
        {ch, _} = Channel.append(ch, msg_bin)

        Channel.persist(ch, dir)
        recovered = Channel.recover(dir, "test_mixed")

        [r1, r2, r3] = Channel.read(recovered, 0, 10)

        assert r1.content_type == "text/plain"
        assert r1.payload == "plain text"

        assert r2.content_type == "application/json"
        {:ok, parsed} = Message.payload_json(r2)
        assert parsed == %{"key" => "value"}

        assert r3.content_type == "image/png"
        assert r3.payload == <<137, 80, 78, 71, 13, 10, 26, 10>>
      after
        File.rm_rf(dir)
      end
    end

    # Test 35: Truncated write recovery
    test "truncated file recovers complete messages only" do
      dir = Path.join(System.tmp_dir!(), "actor_test_trunc_#{:rand.uniform(100_000)}")

      try do
        File.mkdir_p!(dir)
        ch = Channel.new("ch_001", "test_trunc")
        msg1 = Message.text("a", "complete message one")
        msg2 = Message.text("b", "complete message two")

        {ch, _} = Channel.append(ch, msg1)
        {ch, _} = Channel.append(ch, msg2)

        # Write full messages, then append truncated data
        Channel.persist(ch, dir)
        path = Path.join(dir, "test_trunc.log")

        # Append a partial header (simulating crash mid-write)
        {:ok, file} = File.open(path, [:append, :binary])
        IO.binwrite(file, "ACTM" <> <<1::8, 100::32-big>>)
        File.close(file)

        recovered = Channel.recover(dir, "test_trunc")
        assert Channel.length(recovered) == 2

        [r1, r2] = Channel.read(recovered, 0, 10)
        assert r1.payload == "complete message one"
        assert r2.payload == "complete message two"
      after
        File.rm_rf(dir)
      end
    end

    # Test 36: Mixed version recovery (hypothetical v2)
    # Since we only support v1, we test that v1 messages are correctly recovered.
    # A true mixed-version test would require a v2 writer, which doesn't exist yet.
    # Instead we verify that the recovery code handles the version field correctly.
    test "v1 messages recovered with correct version handling" do
      dir = Path.join(System.tmp_dir!(), "actor_test_version_#{:rand.uniform(100_000)}")

      try do
        ch = Channel.new("ch_001", "test_version")
        msg1 = Message.text("a", "v1 message 1")
        msg2 = Message.text("b", "v1 message 2")

        {ch, _} = Channel.append(ch, msg1)
        {ch, _} = Channel.append(ch, msg2)

        Channel.persist(ch, dir)
        recovered = Channel.recover(dir, "test_version")
        assert Channel.length(recovered) == 2

        # Verify the wire format contains version 1
        bytes = Message.to_bytes(msg1)
        <<"ACTM", version::8, _rest::binary>> = bytes
        assert version == 1
      after
        File.rm_rf(dir)
      end
    end
  end

  # ============================================================================
  # Unit Tests — Actor (Tests 37-49)
  # ============================================================================

  describe "Actor" do
    # Test 37: Create actor
    test "create actor with initial state, status is idle" do
      system = ActorSystem.new()

      noop = fn state, _msg -> %ActorResult{new_state: state} end
      {:ok, system} = ActorSystem.create_actor(system, "test", 0, noop)

      assert ActorSystem.get_actor_status(system, "test") == :idle
    end

    # Test 38: Send message
    test "send message to actor, mailbox_size is 1" do
      system = ActorSystem.new()
      noop = fn state, _msg -> %ActorResult{new_state: state} end
      {:ok, system} = ActorSystem.create_actor(system, "test", nil, noop)

      msg = Message.text("sender", "hello")
      system = ActorSystem.send_message(system, "test", msg)

      assert ActorSystem.mailbox_size(system, "test") == 1
    end

    # Test 39: Process message — behavior was called
    test "process_next calls behavior function" do
      system = ActorSystem.new()

      # Behavior that sets state to the payload text
      behavior = fn _state, msg ->
        %ActorResult{new_state: msg.payload}
      end

      {:ok, system} = ActorSystem.create_actor(system, "test", nil, behavior)
      msg = Message.text("sender", "processed!")
      system = ActorSystem.send_message(system, "test", msg)

      {system, :ok} = ActorSystem.process_next(system, "test")
      assert ActorSystem.get_actor_state(system, "test") == "processed!"
    end

    # Test 40: State update — counter actor
    test "counter actor increments state with each message" do
      system = ActorSystem.new()
      counter = fn state, _msg -> %ActorResult{new_state: state + 1} end
      {:ok, system} = ActorSystem.create_actor(system, "counter", 0, counter)

      system = ActorSystem.send_message(system, "counter", Message.text("a", "1"))
      system = ActorSystem.send_message(system, "counter", Message.text("a", "2"))
      system = ActorSystem.send_message(system, "counter", Message.text("a", "3"))

      {system, :ok} = ActorSystem.process_next(system, "counter")
      {system, :ok} = ActorSystem.process_next(system, "counter")
      {system, :ok} = ActorSystem.process_next(system, "counter")

      assert ActorSystem.get_actor_state(system, "counter") == 3
    end

    # Test 41: Messages to send — echo actor
    test "echo actor delivers reply to sender's mailbox" do
      system = ActorSystem.new()

      echo = fn state, msg ->
        reply = Message.text("echo", "echo: #{msg.payload}")
        %ActorResult{new_state: state, messages_to_send: [{msg.sender_id, reply}]}
      end

      noop = fn state, _msg -> %ActorResult{new_state: state} end

      {:ok, system} = ActorSystem.create_actor(system, "echo", nil, echo)
      {:ok, system} = ActorSystem.create_actor(system, "sender", nil, noop)

      msg = Message.text("sender", "hello")
      system = ActorSystem.send_message(system, "echo", msg)
      {system, :ok} = ActorSystem.process_next(system, "echo")

      # The reply should be in sender's mailbox
      assert ActorSystem.mailbox_size(system, "sender") == 1
    end

    # Test 42: Actor creation — spawner
    test "actor can spawn new actors" do
      system = ActorSystem.new()

      noop = fn state, _msg -> %ActorResult{new_state: state} end

      spawner = fn state, _msg ->
        %ActorResult{
          new_state: state + 1,
          actors_to_create: [
            %ActorSpec{actor_id: "spawned_#{state}", initial_state: nil, behavior: noop}
          ]
        }
      end

      {:ok, system} = ActorSystem.create_actor(system, "spawner", 0, spawner)
      system = ActorSystem.send_message(system, "spawner", Message.text("a", "spawn"))
      {system, :ok} = ActorSystem.process_next(system, "spawner")

      assert "spawned_0" in ActorSystem.actor_ids(system)
      assert ActorSystem.get_actor_status(system, "spawned_0") == :idle
    end

    # Test 43: Stop actor
    test "stop actor sets status to stopped" do
      system = ActorSystem.new()

      stopper = fn state, _msg ->
        %ActorResult{new_state: state, stop_actor: true}
      end

      {:ok, system} = ActorSystem.create_actor(system, "stopper", nil, stopper)
      system = ActorSystem.send_message(system, "stopper", Message.text("a", "stop"))
      {system, :ok} = ActorSystem.process_next(system, "stopper")

      assert ActorSystem.get_actor_status(system, "stopper") == :stopped
    end

    # Test 44: Stopped actor rejects messages (dead letter)
    test "stopped actor sends messages to dead_letters" do
      system = ActorSystem.new()
      stopper = fn state, _msg -> %ActorResult{new_state: state, stop_actor: true} end
      {:ok, system} = ActorSystem.create_actor(system, "actor", nil, stopper)

      system = ActorSystem.send_message(system, "actor", Message.text("a", "stop"))
      {system, :ok} = ActorSystem.process_next(system, "actor")
      assert ActorSystem.get_actor_status(system, "actor") == :stopped

      # Send to stopped actor — should go to dead letters
      msg = Message.text("b", "after stop")
      system = ActorSystem.send_message(system, "actor", msg)
      assert length(system.dead_letters) >= 1
      last_dead = List.last(system.dead_letters)
      assert last_dead.payload == "after stop"
    end

    # Test 45: Dead letters — non-existent actor
    test "message to non-existent actor goes to dead_letters" do
      system = ActorSystem.new()
      msg = Message.text("a", "hello ghost")
      system = ActorSystem.send_message(system, "nonexistent", msg)

      assert length(system.dead_letters) == 1
      assert hd(system.dead_letters).payload == "hello ghost"
    end

    # Test 46: Sequential FIFO processing
    test "messages processed in FIFO order" do
      system = ActorSystem.new()

      # Behavior that appends payload to a list in state
      collector = fn state, msg ->
        %ActorResult{new_state: state ++ [msg.payload]}
      end

      {:ok, system} = ActorSystem.create_actor(system, "collector", [], collector)

      system = ActorSystem.send_message(system, "collector", Message.text("a", "first"))
      system = ActorSystem.send_message(system, "collector", Message.text("a", "second"))
      system = ActorSystem.send_message(system, "collector", Message.text("a", "third"))

      {system, :ok} = ActorSystem.process_next(system, "collector")
      {system, :ok} = ActorSystem.process_next(system, "collector")
      {system, :ok} = ActorSystem.process_next(system, "collector")

      assert ActorSystem.get_actor_state(system, "collector") == ["first", "second", "third"]
    end

    # Test 47: Mailbox drains on stop
    test "stopping actor drains mailbox to dead_letters" do
      system = ActorSystem.new()
      noop = fn state, _msg -> %ActorResult{new_state: state} end
      {:ok, system} = ActorSystem.create_actor(system, "actor", nil, noop)

      # Queue 3 messages
      system = ActorSystem.send_message(system, "actor", Message.text("a", "msg1"))
      system = ActorSystem.send_message(system, "actor", Message.text("a", "msg2"))
      system = ActorSystem.send_message(system, "actor", Message.text("a", "msg3"))

      assert ActorSystem.mailbox_size(system, "actor") == 3

      # Stop the actor
      system = ActorSystem.stop_actor(system, "actor")

      assert ActorSystem.get_actor_status(system, "actor") == :stopped
      assert ActorSystem.mailbox_size(system, "actor") == 0
      assert length(system.dead_letters) == 3
    end

    # Test 48: Behavior exception — state unchanged, message to dead letters
    test "behavior exception preserves state and adds message to dead_letters" do
      system = ActorSystem.new()

      # Behavior that crashes on "crash" payload
      crashy = fn state, msg ->
        if msg.payload == "crash" do
          raise "intentional crash"
        end

        %ActorResult{new_state: state + 1}
      end

      {:ok, system} = ActorSystem.create_actor(system, "crashy", 0, crashy)

      # Process a normal message first
      system = ActorSystem.send_message(system, "crashy", Message.text("a", "normal"))
      {system, :ok} = ActorSystem.process_next(system, "crashy")
      assert ActorSystem.get_actor_state(system, "crashy") == 1

      # Now send a crashing message
      system = ActorSystem.send_message(system, "crashy", Message.text("a", "crash"))
      {system, :ok} = ActorSystem.process_next(system, "crashy")

      # State should be unchanged (still 1, not 2)
      assert ActorSystem.get_actor_state(system, "crashy") == 1
      # The crashed message should be in dead_letters
      assert length(system.dead_letters) == 1

      # Actor should still work — process another normal message
      system = ActorSystem.send_message(system, "crashy", Message.text("a", "normal2"))
      {system, :ok} = ActorSystem.process_next(system, "crashy")
      assert ActorSystem.get_actor_state(system, "crashy") == 2
      assert ActorSystem.get_actor_status(system, "crashy") == :idle
    end

    # Test 49: Duplicate actor ID
    test "duplicate actor ID returns error" do
      system = ActorSystem.new()
      noop = fn state, _msg -> %ActorResult{new_state: state} end

      {:ok, system} = ActorSystem.create_actor(system, "actor", nil, noop)
      assert {:error, :duplicate_id} = ActorSystem.create_actor(system, "actor", nil, noop)
    end
  end

  # ============================================================================
  # Integration Tests (Tests 50-58)
  # ============================================================================

  describe "Integration" do
    # Test 50: Ping-pong — two actors exchange 10 messages
    test "ping-pong: two actors exchange messages" do
      system = ActorSystem.new()

      # Ping behavior: when receiving a message, send "ping" to "pong" actor.
      # Stop after 10 pings. State tracks count.
      ping_behavior = fn state, _msg ->
        new_count = state + 1

        if new_count >= 10 do
          %ActorResult{new_state: new_count}
        else
          reply = Message.text("ping", "ping_#{new_count}")
          %ActorResult{new_state: new_count, messages_to_send: [{"pong", reply}]}
        end
      end

      pong_behavior = fn state, _msg ->
        new_count = state + 1

        if new_count >= 10 do
          %ActorResult{new_state: new_count}
        else
          reply = Message.text("pong", "pong_#{new_count}")
          %ActorResult{new_state: new_count, messages_to_send: [{"ping", reply}]}
        end
      end

      {:ok, system} = ActorSystem.create_actor(system, "ping", 0, ping_behavior)
      {:ok, system} = ActorSystem.create_actor(system, "pong", 0, pong_behavior)

      # Start the ping-pong
      system = ActorSystem.send_message(system, "ping", Message.text("start", "go"))
      {system, stats} = ActorSystem.run_until_done(system)

      # Both should have processed messages
      assert stats.messages_processed > 0
      assert ActorSystem.get_actor_state(system, "ping") >= 1
      assert ActorSystem.get_actor_state(system, "pong") >= 1
    end

    # Test 51: Pipeline — A -> B -> C
    test "three-actor pipeline transforms and forwards messages" do
      system = ActorSystem.new()

      # A just sends to B
      a_behavior = fn state, msg ->
        fwd = Message.text("actor_a", "from_a:#{msg.payload}")
        %ActorResult{new_state: state, messages_to_send: [{"actor_b", fwd}]}
      end

      # B transforms and sends to C
      b_behavior = fn state, msg ->
        fwd = Message.text("actor_b", "from_b:#{msg.payload}")
        %ActorResult{new_state: state, messages_to_send: [{"actor_c", fwd}]}
      end

      # C collects messages
      c_behavior = fn state, msg ->
        %ActorResult{new_state: state ++ [msg.payload]}
      end

      {:ok, system} = ActorSystem.create_actor(system, "actor_a", nil, a_behavior)
      {:ok, system} = ActorSystem.create_actor(system, "actor_b", nil, b_behavior)
      {:ok, system} = ActorSystem.create_actor(system, "actor_c", [], c_behavior)

      system = ActorSystem.send_message(system, "actor_a", Message.text("ext", "hello"))
      {system, _} = ActorSystem.run_until_done(system)

      c_state = ActorSystem.get_actor_state(system, "actor_c")
      assert length(c_state) == 1
      assert hd(c_state) == "from_b:from_a:hello"
    end

    # Test 52: Channel-based pipeline
    test "producer writes to channel, consumer reads all messages in order" do
      system = ActorSystem.new()
      system = ActorSystem.create_channel(system, "ch_001", "data")

      {:ok, channel} = ActorSystem.get_channel(system, "ch_001")

      # Producer writes 5 messages
      {channel, _messages} =
        Enum.reduce(1..5, {channel, []}, fn i, {ch, msgs} ->
          msg = Message.text("producer", "item_#{i}")
          {new_ch, _seq} = Channel.append(ch, msg)
          {new_ch, msgs ++ [msg]}
        end)

      system = ActorSystem.put_channel(system, "ch_001", channel)

      # Consumer reads from channel
      {:ok, updated_channel} = ActorSystem.get_channel(system, "ch_001")
      read_msgs = Channel.read(updated_channel, 0, 10)

      assert length(read_msgs) == 5
      payloads = Enum.map(read_msgs, & &1.payload)
      assert payloads == ["item_1", "item_2", "item_3", "item_4", "item_5"]
    end

    # Test 53: Fan-out — one actor sends to 5 actors
    test "fan-out: one actor sends to 5 recipients" do
      system = ActorSystem.new()

      collector = fn state, msg ->
        %ActorResult{new_state: state ++ [msg.payload]}
      end

      fan_out = fn state, msg ->
        targets =
          for i <- 1..5 do
            {"recv_#{i}", Message.text("fan", "broadcast:#{msg.payload}")}
          end

        %ActorResult{new_state: state, messages_to_send: targets}
      end

      {:ok, system} = ActorSystem.create_actor(system, "fan", nil, fan_out)

      system =
        Enum.reduce(1..5, system, fn i, sys ->
          {:ok, new_sys} = ActorSystem.create_actor(sys, "recv_#{i}", [], collector)
          new_sys
        end)

      system = ActorSystem.send_message(system, "fan", Message.text("ext", "hello"))
      {system, _} = ActorSystem.run_until_done(system)

      for i <- 1..5 do
        recv_state = ActorSystem.get_actor_state(system, "recv_#{i}")
        assert recv_state == ["broadcast:hello"]
      end
    end

    # Test 54: Dynamic topology — A spawns B, sends B a message, B responds
    test "dynamic topology: spawned actor can receive and respond" do
      system = ActorSystem.new()

      # B's behavior: echo back to sender
      responder_behavior = fn state, msg ->
        reply = Message.text("responder", "response:#{msg.payload}")
        %ActorResult{new_state: state, messages_to_send: [{msg.sender_id, reply}]}
      end

      # A's behavior: on first message, spawn B and send it a message
      spawner_a = fn state, msg ->
        if state == :initial do
          spec = %ActorSpec{
            actor_id: "responder",
            initial_state: nil,
            behavior: responder_behavior
          }

          outgoing = Message.text("spawner_a", "hello from A")

          %ActorResult{
            new_state: :spawned,
            actors_to_create: [spec],
            messages_to_send: [{"responder", outgoing}]
          }
        else
          %ActorResult{new_state: {:got_response, msg.payload}}
        end
      end

      {:ok, system} = ActorSystem.create_actor(system, "spawner_a", :initial, spawner_a)
      system = ActorSystem.send_message(system, "spawner_a", Message.text("ext", "go"))
      {system, _} = ActorSystem.run_until_done(system)

      assert "responder" in ActorSystem.actor_ids(system)

      a_state = ActorSystem.get_actor_state(system, "spawner_a")
      assert a_state == {:got_response, "response:hello from A"}
    end

    # Test 55: Run until idle with 5 interconnected actors
    test "run_until_idle processes complex network of actors" do
      system = ActorSystem.new()

      # Each actor forwards to the next, forming a ring: 1->2->3->4->5->done
      make_forwarder = fn next_id ->
        fn state, msg ->
          fwd = Message.text("fwd", "#{msg.payload}->#{next_id}")
          %ActorResult{new_state: state + 1, messages_to_send: [{next_id, fwd}]}
        end
      end

      sink = fn state, _msg -> %ActorResult{new_state: state + 1} end

      {:ok, system} = ActorSystem.create_actor(system, "a1", 0, make_forwarder.("a2"))
      {:ok, system} = ActorSystem.create_actor(system, "a2", 0, make_forwarder.("a3"))
      {:ok, system} = ActorSystem.create_actor(system, "a3", 0, make_forwarder.("a4"))
      {:ok, system} = ActorSystem.create_actor(system, "a4", 0, make_forwarder.("a5"))
      {:ok, system} = ActorSystem.create_actor(system, "a5", 0, sink)

      # Send 3 messages into the pipeline
      system = ActorSystem.send_message(system, "a1", Message.text("ext", "msg1"))
      system = ActorSystem.send_message(system, "a1", Message.text("ext", "msg2"))
      system = ActorSystem.send_message(system, "a1", Message.text("ext", "msg3"))

      {system, stats} = ActorSystem.run_until_idle(system)

      # All messages should be processed: 3 messages * 5 hops = 15
      assert stats.messages_processed == 15

      # All mailboxes should be empty
      for id <- ["a1", "a2", "a3", "a4", "a5"] do
        assert ActorSystem.mailbox_size(system, id) == 0
      end

      # a5 (sink) should have processed 3 messages
      assert ActorSystem.get_actor_state(system, "a5") == 3
    end

    # Test 56: Persistence round-trip
    test "channel persistence round-trip with binary payloads" do
      dir = Path.join(System.tmp_dir!(), "actor_test_roundtrip_#{:rand.uniform(100_000)}")

      try do
        system = ActorSystem.new()
        system = ActorSystem.create_channel(system, "ch_001", "test_roundtrip")

        {:ok, channel} = ActorSystem.get_channel(system, "ch_001")

        # Add mixed messages
        msg_text = Message.text("a", "hello world")
        msg_json = Message.json("b", [1, 2, 3])
        msg_bin = Message.binary("c", "image/png", :crypto.strong_rand_bytes(256))

        {channel, _} = Channel.append(channel, msg_text)
        {channel, _} = Channel.append(channel, msg_json)
        {channel, _} = Channel.append(channel, msg_bin)

        Channel.persist(channel, dir)

        # Recover in a "new system"
        recovered = Channel.recover(dir, "test_roundtrip")
        assert Channel.length(recovered) == 3

        [r1, r2, r3] = Channel.read(recovered, 0, 10)

        assert r1.payload == msg_text.payload
        assert r2.payload == msg_json.payload
        assert r3.payload == msg_bin.payload
      after
        File.rm_rf(dir)
      end
    end

    # Test 57: Large-scale — 100 actors, 1000 messages
    test "100 actors, 1000 messages, all delivered or in dead_letters" do
      system = ActorSystem.new()

      counter = fn state, _msg -> %ActorResult{new_state: state + 1} end

      # Create 100 actors
      system =
        Enum.reduce(1..100, system, fn i, sys ->
          {:ok, new_sys} = ActorSystem.create_actor(sys, "actor_#{i}", 0, counter)
          new_sys
        end)

      # Send 1000 messages to random actors
      system =
        Enum.reduce(1..1000, system, fn _i, sys ->
          target = "actor_#{:rand.uniform(100)}"
          msg = Message.text("source", "random_msg")
          ActorSystem.send_message(sys, target, msg)
        end)

      {system, stats} = ActorSystem.run_until_done(system)

      # All 1000 messages should be processed
      assert stats.messages_processed == 1000

      # Total messages received across all actors should equal 1000
      total_received =
        Enum.sum(
          for i <- 1..100 do
            ActorSystem.get_actor_state(system, "actor_#{i}")
          end
        )

      assert total_received == 1000
      assert length(system.dead_letters) == 0
    end

    # Test 58: Binary message pipeline
    test "binary message sent through channel arrives with identical bytes" do
      system = ActorSystem.new()
      system = ActorSystem.create_channel(system, "img_ch", "images")

      # Simulate a PNG image (first 8 bytes of PNG signature)
      png_data = <<137, 80, 78, 71, 13, 10, 26, 10>> <> :crypto.strong_rand_bytes(1024)

      {:ok, channel} = ActorSystem.get_channel(system, "img_ch")
      msg = Message.binary("actor_a", "image/png", png_data)

      {channel, _seq} = Channel.append(channel, msg)
      system = ActorSystem.put_channel(system, "img_ch", channel)

      # Actor B reads from channel
      {:ok, updated_channel} = ActorSystem.get_channel(system, "img_ch")
      [received] = Channel.read(updated_channel, 0, 1)

      assert received.payload == png_data
      assert received.content_type == "image/png"
      assert byte_size(received.payload) == byte_size(png_data)
    end
  end

  # ============================================================================
  # Additional Coverage Tests
  # ============================================================================

  describe "Edge cases" do
    test "process_next on empty mailbox returns :empty" do
      system = ActorSystem.new()
      noop = fn state, _msg -> %ActorResult{new_state: state} end
      {:ok, system} = ActorSystem.create_actor(system, "test", nil, noop)

      {_system, result} = ActorSystem.process_next(system, "test")
      assert result == :empty
    end

    test "process_next on non-existent actor returns :not_found" do
      system = ActorSystem.new()
      {_system, result} = ActorSystem.process_next(system, "ghost")
      assert result == :not_found
    end

    test "process_next on stopped actor returns :stopped" do
      system = ActorSystem.new()
      noop = fn state, _msg -> %ActorResult{new_state: state} end
      {:ok, system} = ActorSystem.create_actor(system, "test", nil, noop)
      system = ActorSystem.stop_actor(system, "test")

      {_system, result} = ActorSystem.process_next(system, "test")
      assert result == :stopped
    end

    test "get_channel for non-existent channel returns error" do
      system = ActorSystem.new()
      assert {:error, :not_found} = ActorSystem.get_channel(system, "nope")
    end

    test "shutdown stops all actors" do
      system = ActorSystem.new()
      noop = fn state, _msg -> %ActorResult{new_state: state} end
      {:ok, system} = ActorSystem.create_actor(system, "a1", nil, noop)
      {:ok, system} = ActorSystem.create_actor(system, "a2", nil, noop)

      system = ActorSystem.send_message(system, "a1", Message.text("x", "m1"))
      system = ActorSystem.send_message(system, "a2", Message.text("x", "m2"))

      system = ActorSystem.shutdown(system)

      assert ActorSystem.get_actor_status(system, "a1") == :stopped
      assert ActorSystem.get_actor_status(system, "a2") == :stopped
      assert length(system.dead_letters) == 2
    end

    test "stop_actor on non-existent actor is a no-op" do
      system = ActorSystem.new()
      # Should not crash
      system2 = ActorSystem.stop_actor(system, "ghost")
      assert system2 == system
    end

    test "mailbox_size for non-existent actor returns 0" do
      system = ActorSystem.new()
      assert ActorSystem.mailbox_size(system, "ghost") == 0
    end

    test "get_actor_status for non-existent actor returns :not_found" do
      system = ActorSystem.new()
      assert ActorSystem.get_actor_status(system, "ghost") == :not_found
    end

    test "channel_slice with start >= end returns empty" do
      ch = Channel.new("ch", "test")
      msg = Message.text("a", "hello")
      {ch, _} = Channel.append(ch, msg)

      assert Channel.channel_slice(ch, 5, 3) == []
      assert Channel.channel_slice(ch, 2, 2) == []
    end

    test "JSON encoder handles special characters" do
      msg = Message.text("agent", "hello\nworld\t\"quoted\"")
      bytes = Message.to_bytes(msg)
      {:ok, decoded} = Message.from_bytes(bytes)
      assert decoded.payload == "hello\nworld\t\"quoted\""
    end

    test "JSON encoder handles lists in payload_json" do
      msg = Message.json("a", [1, 2, 3])
      {:ok, parsed} = Message.payload_json(msg)
      assert parsed == [1, 2, 3]
    end

    test "JSON encoder handles nested maps" do
      msg = Message.json("a", %{"outer" => %{"inner" => "value"}})
      {:ok, parsed} = Message.payload_json(msg)
      assert parsed == %{"outer" => %{"inner" => "value"}}
    end

    test "JSON encoder handles empty map" do
      msg = Message.json("a", %{})
      {:ok, parsed} = Message.payload_json(msg)
      assert parsed == %{}
    end

    test "JSON encoder handles booleans and null" do
      msg = Message.json("a", %{"flag" => true, "empty" => nil})
      {:ok, parsed} = Message.payload_json(msg)
      assert parsed["flag"] == true
      assert parsed["empty"] == nil
    end

    test "JSON decode error" do
      assert {:error, _} = CodingAdventures.Actor.JSON.decode("not valid json {{{}}")
    end

    test "JSON encode float" do
      result = CodingAdventures.Actor.JSON.encode(3.14)
      assert is_binary(result)
    end

    test "JSON encode list with mixed types" do
      result = CodingAdventures.Actor.JSON.encode([1, "two", true, nil, [3]])
      assert is_binary(result)
      {:ok, parsed} = CodingAdventures.Actor.JSON.decode(result)
      assert parsed == [1, "two", true, nil, [3]]
    end

    test "JSON decode with whitespace" do
      {:ok, result} = CodingAdventures.Actor.JSON.decode("  { \"a\" : 1 }  ")
      assert result == %{"a" => 1}
    end

    test "JSON decode negative number" do
      {:ok, result} = CodingAdventures.Actor.JSON.decode("-42")
      assert result == -42
    end

    test "JSON decode float" do
      {:ok, result} = CodingAdventures.Actor.JSON.decode("3.14")
      assert result == 3.14
    end

    test "JSON decode array" do
      {:ok, result} = CodingAdventures.Actor.JSON.decode("[1, 2, 3]")
      assert result == [1, 2, 3]
    end

    test "JSON decode empty array" do
      {:ok, result} = CodingAdventures.Actor.JSON.decode("[]")
      assert result == []
    end

    test "JSON decode string with escaped slash" do
      {:ok, result} = CodingAdventures.Actor.JSON.decode("\"a\\/b\"")
      assert result == "a/b"
    end

    test "JSON decode trailing data returns error" do
      assert {:error, _} = CodingAdventures.Actor.JSON.decode("42 extra")
    end

    test "from_io_device with truncated envelope" do
      # Write a valid header but truncated envelope
      path = Path.join(System.tmp_dir!(), "actor_test_trunc_env_#{:rand.uniform(100_000)}")

      try do
        # Header says envelope is 100 bytes but we only write 5
        header = <<"ACTM", 1::8, 100::32-big, 0::64-big>>
        File.write!(path, header <> "short")
        {:ok, device} = File.open(path, [:read, :binary])
        result = Message.from_io_device(device)
        assert result == {:error, :truncated}
        File.close(device)
      after
        File.rm(path)
      end
    end

    test "from_io_device with invalid magic" do
      path = Path.join(System.tmp_dir!(), "actor_test_bad_magic_#{:rand.uniform(100_000)}")

      try do
        header = <<"XXXX", 1::8, 0::32-big, 0::64-big>>
        File.write!(path, header)
        {:ok, device} = File.open(path, [:read, :binary])
        result = Message.from_io_device(device)
        assert result == {:error, :invalid_format}
        File.close(device)
      after
        File.rm(path)
      end
    end

    test "from_io_device with truncated header" do
      path = Path.join(System.tmp_dir!(), "actor_test_short_hdr_#{:rand.uniform(100_000)}")

      try do
        File.write!(path, "ACTM" <> <<1::8>>)
        {:ok, device} = File.open(path, [:read, :binary])
        result = Message.from_io_device(device)
        assert result == {:error, :truncated}
        File.close(device)
      after
        File.rm(path)
      end
    end

    test "wire_magic and wire_version accessors" do
      assert Message.wire_magic() == "ACTM"
      assert Message.wire_version() == 1
    end

    test "from_bytes with truncated data returns invalid_format" do
      assert {:error, :invalid_format} = Message.from_bytes(<<"ACTM", 1::8>>)
    end

    test "ActorResult defaults" do
      result = %ActorResult{new_state: 42}
      assert result.messages_to_send == []
      assert result.actors_to_create == []
      assert result.stop_actor == false
    end

    test "actor drains mailbox on stop via behavior" do
      system = ActorSystem.new()

      stopper = fn state, _msg ->
        %ActorResult{new_state: state, stop_actor: true}
      end

      {:ok, system} = ActorSystem.create_actor(system, "s", nil, stopper)
      system = ActorSystem.send_message(system, "s", Message.text("a", "stop"))
      system = ActorSystem.send_message(system, "s", Message.text("a", "after1"))
      system = ActorSystem.send_message(system, "s", Message.text("a", "after2"))

      {system, :ok} = ActorSystem.process_next(system, "s")

      assert ActorSystem.get_actor_status(system, "s") == :stopped
      # The remaining 2 messages should be in dead_letters
      assert length(system.dead_letters) == 2
    end
  end
end
