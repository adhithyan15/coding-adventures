defmodule CodingAdventures.LZ77Test do
  use ExUnit.Case, async: true

  alias CodingAdventures.LZ77

  # ---- Specification Test Vectors ----

  describe "encode/decode spec vectors" do
    test "empty input produces no tokens" do
      assert LZ77.encode("") == []
      assert LZ77.decode([]) == ""
    end

    test "no repetition → all literal tokens" do
      tokens = LZ77.encode("ABCDE")
      assert length(tokens) == 5
      assert Enum.all?(tokens, fn t -> t.offset == 0 and t.length == 0 end)
    end

    test "all identical bytes exploit overlap" do
      # "AAAAAAA" → literal A + backreference (offset=1, length=5, next_char=A)
      tokens = LZ77.encode("AAAAAAA")
      assert length(tokens) == 2
      assert hd(tokens) == LZ77.token(0, 0, 65)
      [_, second] = tokens
      assert second.offset == 1
      assert second.length == 5
      assert second.next_char == 65
      assert LZ77.decode(tokens) == "AAAAAAA"
    end

    test "repeated pair uses backreference" do
      # "ABABABAB" → [A literal, B literal, (offset=2, length=5, next_char='B')]
      tokens = LZ77.encode("ABABABAB")
      assert length(tokens) == 3
      [t0, t1, t2] = tokens
      assert t0 == LZ77.token(0, 0, 65)
      assert t1 == LZ77.token(0, 0, 66)
      assert t2.offset == 2
      assert t2.length == 5
      assert t2.next_char == 66
      assert LZ77.decode(tokens) == "ABABABAB"
    end

    test "AABCBBABC with min_match=3 → all literals" do
      tokens = LZ77.encode("AABCBBABC")
      assert length(tokens) == 9
      assert Enum.all?(tokens, fn t -> t.offset == 0 and t.length == 0 end)
      assert LZ77.decode(tokens) == "AABCBBABC"
    end

    test "AABCBBABC with min_match=2 round-trips" do
      tokens = LZ77.encode("AABCBBABC", 4096, 255, 2)
      assert LZ77.decode(tokens) == "AABCBBABC"
    end
  end

  # ---- Round-Trip Tests ----

  describe "round-trip invariants" do
    test "empty" do
      assert LZ77.decode(LZ77.encode("")) == ""
    end

    test "single byte A" do
      assert LZ77.decode(LZ77.encode("A")) == "A"
    end

    test "ascii strings" do
      for s <- ["hello world", "the quick brown fox", "ababababab", "aaaaaaaaaa"] do
        assert LZ77.decode(LZ77.encode(s)) == s
      end
    end

    test "binary data" do
      for s <- [<<0, 0, 0>>, <<255, 255, 255>>, <<0, 1, 2, 0, 1, 2>>] do
        assert LZ77.decode(LZ77.encode(s)) == s
      end
    end

    test "compress/decompress round-trip" do
      for s <- ["", "A", "ABCDE", "AAAAAAA", "ABABABAB", "hello world"] do
        assert LZ77.decompress(LZ77.compress(s)) == s
      end
    end
  end

  # ---- Parameter Tests ----

  describe "parameters" do
    test "offsets never exceed window_size" do
      data = "X" <> String.duplicate("Y", 5000) <> "X"
      tokens = LZ77.encode(data, 100)
      assert Enum.all?(tokens, fn t -> t.offset <= 100 end)
    end

    test "lengths never exceed max_match" do
      data = String.duplicate("A", 1000)
      tokens = LZ77.encode(data, 4096, 50)
      assert Enum.all?(tokens, fn t -> t.length <= 50 end)
    end

    test "min_match threshold respected" do
      tokens = LZ77.encode("AABAA", 4096, 255, 2)
      assert Enum.all?(tokens, fn t -> t.length == 0 or t.length >= 2 end)
    end
  end

  # ---- Edge Cases ----

  describe "edge cases" do
    test "single byte encodes as literal" do
      tokens = LZ77.encode("X")
      assert tokens == [LZ77.token(0, 0, 88)]
    end

    test "exact window boundary match" do
      data = String.duplicate("X", 11)
      tokens = LZ77.encode(data, 10)
      assert Enum.any?(tokens, fn t -> t.offset > 0 end)
      assert LZ77.decode(tokens) == data
    end

    test "overlapping match decoded byte-by-byte" do
      # [A, B] + (offset=2, length=5, next_char='Z') → ABABABAZ
      tokens = [
        LZ77.token(0, 0, 65),
        LZ77.token(0, 0, 66),
        LZ77.token(2, 5, 90)
      ]
      assert LZ77.decode(tokens) == "ABABABAZ"
    end

    test "binary with nulls" do
      data = <<0, 0, 0, 255, 255>>
      assert LZ77.decode(LZ77.encode(data)) == data
    end

    test "very long input" do
      repeated = String.duplicate("Hello, World! ", 100)
      extra = String.duplicate("X", 500)
      data = repeated <> extra
      assert LZ77.decode(LZ77.encode(data)) == data
    end

    test "long run of identical bytes compresses well" do
      data = String.duplicate("A", 10_000)
      tokens = LZ77.encode(data)
      # ~41 tokens expected: 1 literal + ~39 × 255 + 1 partial
      assert length(tokens) < 50
      assert LZ77.decode(tokens) == data
    end
  end

  # ---- Serialisation Tests ----

  describe "serialisation" do
    test "format is 4 + N*4 bytes" do
      tokens = [LZ77.token(0, 0, 65), LZ77.token(2, 5, 66)]
      serialised = LZ77.serialise_tokens(tokens)
      assert byte_size(serialised) == 4 + 2 * 4
    end

    test "serialise/deserialise is a no-op" do
      tokens = [LZ77.token(0, 0, 65), LZ77.token(1, 3, 66), LZ77.token(2, 5, 67)]
      assert LZ77.deserialise_tokens(LZ77.serialise_tokens(tokens)) == tokens
    end

    test "empty serialisation" do
      compressed = LZ77.compress("")
      assert LZ77.decompress(compressed) == ""
    end

    test "all spec vectors compress/decompress" do
      for s <- ["", "ABCDE", "AAAAAAA", "ABABABAB", "AABCBBABC"] do
        assert LZ77.decompress(LZ77.compress(s)) == s
      end
    end
  end

  # ---- Behaviour Tests ----

  describe "behaviour" do
    test "incompressible data does not expand beyond 4N+10" do
      data = :binary.list_to_bin(Enum.to_list(0..255))
      compressed = LZ77.compress(data)
      assert byte_size(compressed) <= 4 * byte_size(data) + 10
    end

    test "repetitive data compresses significantly" do
      data = String.duplicate("ABC", 100)
      compressed = LZ77.compress(data)
      assert byte_size(compressed) < byte_size(data)
    end

    test "compression is deterministic" do
      data = "hello world test"
      assert LZ77.compress(data) == LZ77.compress(data)
    end
  end
end
