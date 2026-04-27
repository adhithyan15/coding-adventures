defmodule CodingAdventures.IrcFramingTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.IrcFraming.Framer

  describe "new/0" do
    test "creates framer with empty buffer" do
      framer = Framer.new()
      assert framer.buf == <<>>
      assert Framer.buffer_size(framer) == 0
    end
  end

  describe "feed/2" do
    test "appends bytes to buffer" do
      framer = Framer.new()
      framer = Framer.feed(framer, "NICK alice")
      assert framer.buf == "NICK alice"
      assert Framer.buffer_size(framer) == 10
    end

    test "appends empty bytes is a no-op" do
      framer = Framer.new()
      framer = Framer.feed(framer, "hello")
      framer = Framer.feed(framer, "")
      assert framer.buf == "hello"
    end

    test "multiple feeds concatenate" do
      framer = Framer.new()
      framer = Framer.feed(framer, "NICK ")
      framer = Framer.feed(framer, "alice")
      assert framer.buf == "NICK alice"
    end
  end

  describe "frames/1 — basic extraction" do
    test "extracts one complete CRLF-terminated line" do
      framer = Framer.new()
      framer = Framer.feed(framer, "NICK alice\r\n")
      {framer, lines} = Framer.frames(framer)
      assert lines == ["NICK alice"]
      assert framer.buf == ""
    end

    test "extracts one LF-only terminated line" do
      framer = Framer.new()
      framer = Framer.feed(framer, "NICK alice\n")
      {_framer, lines} = Framer.frames(framer)
      assert lines == ["NICK alice"]
    end

    test "extracts multiple complete lines" do
      framer = Framer.new()
      framer = Framer.feed(framer, "NICK alice\r\nJOIN #general\r\n")
      {_framer, lines} = Framer.frames(framer)
      assert lines == ["NICK alice", "JOIN #general"]
    end

    test "leaves partial line in buffer" do
      framer = Framer.new()
      framer = Framer.feed(framer, "NICK ali")
      {framer, lines} = Framer.frames(framer)
      assert lines == []
      assert framer.buf == "NICK ali"
    end

    test "returns empty list when buffer is empty" do
      framer = Framer.new()
      {_framer, lines} = Framer.frames(framer)
      assert lines == []
    end

    test "handles three messages in one feed" do
      framer = Framer.new()
      data = "NICK alice\r\nUSER alice 0 * :Alice\r\nJOIN #chan\r\n"
      framer = Framer.feed(framer, data)
      {_framer, lines} = Framer.frames(framer)
      assert lines == ["NICK alice", "USER alice 0 * :Alice", "JOIN #chan"]
    end
  end

  describe "frames/1 — partial data accumulation" do
    test "assembles a line from two partial feeds" do
      framer = Framer.new()
      framer = Framer.feed(framer, "NICK ali")
      {framer, lines1} = Framer.frames(framer)
      assert lines1 == []

      framer = Framer.feed(framer, "ce\r\n")
      {_framer, lines2} = Framer.frames(framer)
      assert lines2 == ["NICK alice"]
    end

    test "handles complete line followed by partial next line" do
      framer = Framer.new()
      framer = Framer.feed(framer, "NICK alice\r\nJOIN #g")
      {framer, lines} = Framer.frames(framer)
      assert lines == ["NICK alice"]
      assert Framer.buffer_size(framer) == byte_size("JOIN #g")
    end
  end

  describe "frames/1 — overlong line rejection" do
    test "discards lines longer than 510 bytes" do
      overlong = String.duplicate("x", 511)
      framer = Framer.new()
      framer = Framer.feed(framer, overlong <> "\r\n")
      {_framer, lines} = Framer.frames(framer)
      assert lines == []
    end

    test "accepts lines exactly at 510 bytes" do
      at_limit = String.duplicate("x", 510)
      framer = Framer.new()
      framer = Framer.feed(framer, at_limit <> "\r\n")
      {_framer, lines} = Framer.frames(framer)
      assert lines == [at_limit]
    end

    test "overlong line discarded but subsequent valid lines still extracted" do
      overlong = String.duplicate("x", 600)
      framer = Framer.new()
      framer = Framer.feed(framer, overlong <> "\r\nNICK alice\r\n")
      {_framer, lines} = Framer.frames(framer)
      assert lines == ["NICK alice"]
    end
  end

  describe "reset/1" do
    test "clears the buffer" do
      framer = Framer.new()
      framer = Framer.feed(framer, "partial data")
      framer = Framer.reset(framer)
      assert framer.buf == <<>>
      assert Framer.buffer_size(framer) == 0
    end

    test "reset framer can receive new data" do
      framer = Framer.new()
      framer = Framer.feed(framer, "stale data")
      framer = Framer.reset(framer)
      framer = Framer.feed(framer, "NICK bob\r\n")
      {_framer, lines} = Framer.frames(framer)
      assert lines == ["NICK bob"]
    end
  end

  describe "buffer_size/1" do
    test "returns 0 for empty buffer" do
      framer = Framer.new()
      assert Framer.buffer_size(framer) == 0
    end

    test "decreases after frames extracted" do
      framer = Framer.new()
      framer = Framer.feed(framer, "NICK alice\r\npart")
      {framer, _} = Framer.frames(framer)
      assert Framer.buffer_size(framer) == byte_size("part")
    end
  end

  describe "CodingAdventures.IrcFraming facade" do
    test "new/0 delegates to Framer.new/0" do
      framer = CodingAdventures.IrcFraming.new()
      assert framer.buf == <<>>
    end

    test "feed/2 delegates to Framer.feed/2" do
      framer = CodingAdventures.IrcFraming.new()
      framer = CodingAdventures.IrcFraming.feed(framer, "NICK alice\r\n")
      assert framer.buf == "NICK alice\r\n"
    end

    test "frames/1 delegates to Framer.frames/1" do
      framer = CodingAdventures.IrcFraming.new()
      framer = CodingAdventures.IrcFraming.feed(framer, "NICK alice\r\n")
      {_framer, lines} = CodingAdventures.IrcFraming.frames(framer)
      assert lines == ["NICK alice"]
    end

    test "reset/1 delegates to Framer.reset/1" do
      framer = CodingAdventures.IrcFraming.new()
      framer = CodingAdventures.IrcFraming.feed(framer, "data")
      framer = CodingAdventures.IrcFraming.reset(framer)
      assert framer.buf == <<>>
    end

    test "buffer_size/1 delegates to Framer.buffer_size/1" do
      framer = CodingAdventures.IrcFraming.new()
      framer = CodingAdventures.IrcFraming.feed(framer, "hello")
      assert CodingAdventures.IrcFraming.buffer_size(framer) == 5
    end
  end

  describe "integration with irc_proto" do
    test "framed lines can be parsed as IRC messages" do
      alias CodingAdventures.IrcProto

      framer = Framer.new()
      framer = Framer.feed(framer, "NICK alice\r\nUSER alice 0 * :Alice Smith\r\n")
      {_framer, lines} = Framer.frames(framer)

      msgs = Enum.map(lines, fn line ->
        {:ok, msg} = IrcProto.parse(line)
        msg
      end)

      assert length(msgs) == 2
      assert Enum.at(msgs, 0).command == "NICK"
      assert Enum.at(msgs, 1).command == "USER"
      assert Enum.at(msgs, 1).params == ["alice", "0", "*", "Alice Smith"]
    end
  end
end
