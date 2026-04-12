defmodule CodingAdventures.LZ78Test do
  use ExUnit.Case, async: true

  alias CodingAdventures.LZ78
  alias CodingAdventures.LZ78.TrieCursor

  # ─── TrieCursor tests ──────────────────────────────────────────────────────

  describe "TrieCursor" do
    test "new cursor starts at root with dict_id 0" do
      cursor = TrieCursor.new()
      assert TrieCursor.at_root?(cursor)
      assert TrieCursor.dict_id(cursor) == 0
    end

    test "step returns :miss on empty trie" do
      cursor = TrieCursor.new()
      assert TrieCursor.step(cursor, ?A) == :miss
    end

    test "insert then step finds the child" do
      cursor = TrieCursor.new() |> TrieCursor.insert(?A, 1)
      assert {:ok, advanced} = TrieCursor.step(cursor, ?A)
      assert TrieCursor.dict_id(advanced) == 1
      refute TrieCursor.at_root?(advanced)
    end

    test "insert does not advance cursor" do
      cursor = TrieCursor.new() |> TrieCursor.insert(?A, 1)
      assert TrieCursor.at_root?(cursor)
      assert TrieCursor.dict_id(cursor) == 0
    end

    test "reset returns cursor to root" do
      cursor = TrieCursor.new() |> TrieCursor.insert(?A, 1)
      {:ok, advanced} = TrieCursor.step(cursor, ?A)
      assert TrieCursor.at_root?(TrieCursor.reset(advanced))
    end

    test "step misses on unknown byte after insert" do
      cursor = TrieCursor.new() |> TrieCursor.insert(?A, 1)
      assert TrieCursor.step(cursor, ?B) == :miss
    end

    test "multi-level insert and step" do
      cursor =
        TrieCursor.new()
        |> TrieCursor.insert(?A, 1)

      {:ok, at_a} = TrieCursor.step(cursor, ?A)
      cursor2 = at_a |> TrieCursor.insert(?B, 2)
      {:ok, at_b} = TrieCursor.step(cursor2, ?B)
      assert TrieCursor.dict_id(at_b) == 2
    end

    test "to_list returns all entries in id order" do
      cursor =
        TrieCursor.new()
        |> TrieCursor.insert(?A, 1)
        |> TrieCursor.insert(?B, 2)
      entries = TrieCursor.to_list(cursor)
      assert length(entries) == 2
      ids = Enum.map(entries, fn {_path, id} -> id end)
      assert ids == [1, 2]
    end

    test "to_list returns correct paths" do
      cursor = TrieCursor.new() |> TrieCursor.insert(?A, 1)
      [{path, 1}] = TrieCursor.to_list(cursor)
      assert path == [?A]
    end

    test "LZ78 simulation produces correct tokens for AABCBBABC" do
      cursor = TrieCursor.new()
      data   = ~c"AABCBBABC"
      {tokens, _cursor} =
        Enum.reduce(data, {[], cursor, 1}, fn byte, {toks, cur, next_id} ->
          case TrieCursor.step(cur, byte) do
            {:ok, new_cur} -> {toks, new_cur, next_id}
            :miss ->
              tok = {TrieCursor.dict_id(cur), byte}
              new_cur = TrieCursor.insert(cur, byte, next_id) |> TrieCursor.reset()
              {[tok | toks], new_cur, next_id + 1}
          end
        end)
        |> then(fn {toks, cur, _id} -> {Enum.reverse(toks), cur} end)

      assert tokens == [
        {0, ?A}, {1, ?B}, {0, ?C}, {0, ?B}, {4, ?A}, {4, ?C}
      ]
    end
  end

  # ─── Encoder spec vectors ──────────────────────────────────────────────────

  describe "encode/1 spec vectors" do
    test "empty input produces no tokens" do
      assert LZ78.encode("") == []
    end

    test "single byte produces one literal token" do
      assert LZ78.encode("A") == [LZ78.token(0, ?A)]
    end

    test "no repetition: all literals" do
      tokens = LZ78.encode("ABCDE")
      assert length(tokens) == 5
      assert Enum.all?(tokens, fn t -> t.dict_index == 0 end)
    end

    test "AABCBBABC produces correct token sequence" do
      want = [
        LZ78.token(0, ?A),
        LZ78.token(1, ?B),
        LZ78.token(0, ?C),
        LZ78.token(0, ?B),
        LZ78.token(4, ?A),
        LZ78.token(4, ?C),
      ]
      assert LZ78.encode("AABCBBABC") == want
    end

    test "ABABAB produces flush sentinel at end" do
      want = [
        LZ78.token(0, ?A),
        LZ78.token(0, ?B),
        LZ78.token(1, ?B),
        LZ78.token(3, 0),
      ]
      assert LZ78.encode("ABABAB") == want
    end

    test "all identical bytes compresses to few tokens" do
      tokens = LZ78.encode("AAAAAAA")
      assert length(tokens) == 4
    end
  end

  # ─── Decoder ───────────────────────────────────────────────────────────────

  describe "decode/2" do
    test "empty tokens returns empty binary" do
      assert LZ78.decode([]) == ""
    end

    test "single literal token" do
      assert LZ78.decode([LZ78.token(0, ?A)], 1) == "A"
    end

    test "decode AABCBBABC" do
      tokens = LZ78.encode("AABCBBABC")
      assert LZ78.decode(tokens, 9) == "AABCBBABC"
    end

    test "decode ABABAB with original_length strips flush byte" do
      tokens = LZ78.encode("ABABAB")
      assert LZ78.decode(tokens, 6) == "ABABAB"
    end

    test "decode :all includes flush byte" do
      tokens = LZ78.encode("ABABAB")
      result = LZ78.decode(tokens, :all)
      assert byte_size(result) == 7
      assert binary_part(result, 0, 6) == "ABABAB"
    end
  end

  # ─── Round-trip ────────────────────────────────────────────────────────────

  describe "compress/decompress round-trip" do
    for {label, data} <- [
      {"empty", ""},
      {"single byte", "A"},
      {"no repetition", "ABCDE"},
      {"all identical", "AAAAAAA"},
      {"AABCBBABC", "AABCBBABC"},
      {"ABABAB", "ABABAB"},
      {"hello world", "hello world"},
      {"multi word", "ababababab"},
      {"long repetitive", String.duplicate("ABC", 100)},
    ] do
      test "round-trip #{label}" do
        data = unquote(data)
        assert LZ78.decompress(LZ78.compress(data)) == data
      end
    end

    test "round-trip binary with null bytes" do
      data = <<0, 0, 0, 255, 255>>
      assert LZ78.decompress(LZ78.compress(data)) == data
    end

    test "round-trip full byte range" do
      data = :binary.list_to_bin(Enum.to_list(0..255))
      assert LZ78.decompress(LZ78.compress(data)) == data
    end

    test "round-trip repeated pattern" do
      data = :binary.list_to_bin(List.duplicate([0, 1, 2], 100) |> List.flatten())
      assert LZ78.decompress(LZ78.compress(data)) == data
    end
  end

  # ─── Parameters ────────────────────────────────────────────────────────────

  describe "max_dict_size parameter" do
    test "dict indices never exceed max_dict_size" do
      tokens = LZ78.encode("ABCABCABCABCABC", 10)
      assert Enum.all?(tokens, fn t -> t.dict_index < 10 end)
    end

    test "max_dict_size=1 means no dictionary entries are added" do
      tokens = LZ78.encode("AAAA", 1)
      assert Enum.all?(tokens, fn t -> t.dict_index == 0 end)
    end
  end

  # ─── Wire format ───────────────────────────────────────────────────────────

  describe "serialise/deserialise" do
    test "header contains original length and token count" do
      data = "AB"
      compressed = LZ78.compress(data)
      <<orig_len::unsigned-big-integer-32, tok_count::unsigned-big-integer-32, _rest::binary>> = compressed
      assert orig_len == 2
      tokens = LZ78.encode(data)
      assert tok_count == length(tokens)
    end

    test "compressed size matches expected format" do
      data = "AB"
      compressed = LZ78.compress(data)
      tokens = LZ78.encode(data)
      assert byte_size(compressed) == 8 + length(tokens) * 4
    end

    test "compress is deterministic" do
      data = "hello world test"
      assert LZ78.compress(data) == LZ78.compress(data)
    end
  end

  # ─── Compression ratio ────────────────────────────────────────────────────

  describe "compression effectiveness" do
    test "repetitive data compresses smaller than original" do
      data = String.duplicate("ABC", 1000)
      compressed = LZ78.compress(data)
      assert byte_size(compressed) < byte_size(data)
    end

    test "all same byte compresses significantly" do
      data = :binary.copy(<<65>>, 10_000)
      compressed = LZ78.compress(data)
      assert byte_size(compressed) < byte_size(data)
      assert LZ78.decompress(compressed) == data
    end
  end
end
