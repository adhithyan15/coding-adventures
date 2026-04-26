defmodule CodingAdventures.ZstdTest do
  use ExUnit.Case, async: true
  import Bitwise

  alias CodingAdventures.Zstd

  # Helper: round-trip via our own compress/decompress.
  # Asserts that decompressing the result of compressing `data` gives back `data`.
  defp rt(data) when is_binary(data) do
    compressed = Zstd.compress(data)
    assert {:ok, result} = Zstd.decompress(compressed)
    assert result == data, "round-trip failed: got #{inspect(result)}, expected #{inspect(data)}"
    result
  end

  # ── TC-1: empty input ────────────────────────────────────────────────────────
  #
  # An empty binary must produce a valid ZStd frame (header + one empty raw block)
  # and decompress back to empty without panic or error.

  test "TC-1: empty input round-trips correctly" do
    result = rt("")
    assert result == ""
    assert byte_size(Zstd.compress("")) > 0, "compressed empty must still produce a frame"
  end

  # ── TC-2: single byte ────────────────────────────────────────────────────────
  #
  # The smallest non-empty input: one byte. Tests the literal-only path where
  # LZSS finds no matches and the block falls back to raw or compressed.

  test "TC-2: single byte round-trips correctly" do
    assert rt(<<0x42>>) == <<0x42>>
    assert rt(<<0x00>>) == <<0x00>>
    assert rt(<<0xFF>>) == <<0xFF>>
  end

  # ── TC-3: all 256 byte values ────────────────────────────────────────────────
  #
  # Every possible byte value 0x00..=0xFF in order. Exercises literal encoding
  # of non-ASCII and zero bytes, as well as bytes that look like ZStd headers.

  test "TC-3: all 256 byte values round-trip" do
    input = :binary.list_to_bin(Enum.to_list(0..255))
    assert rt(input) == input
  end

  # ── TC-4: RLE — 1024 identical bytes ────────────────────────────────────────
  #
  # 1024 identical bytes should be detected as an RLE block.
  # Expected compressed size:
  #   4 (magic) + 1 (FHD) + 8 (FCS) + 3 (block header) + 1 (RLE byte) = 17 bytes.
  # We assert < 30 bytes to give the implementation some slack.

  test "TC-4: RLE block compression" do
    input = String.duplicate("A", 1024)
    compressed = Zstd.compress(input)
    assert {:ok, ^input} = Zstd.decompress(compressed)
    assert byte_size(compressed) < 30,
      "RLE of 1024 bytes should compress to < 30 bytes, got #{byte_size(compressed)}"
  end

  # ── TC-5: English prose — ≥20% compression ──────────────────────────────────
  #
  # Repeated English text has strong LZ77 matches. We require at least 20%
  # reduction (output ≤ 80% of input size) after 25 repetitions.

  test "TC-5: English prose achieves ≥ 20% compression" do
    input = String.duplicate("the quick brown fox jumps over the lazy dog ", 25)
    compressed = Zstd.compress(input)
    assert {:ok, ^input} = Zstd.decompress(compressed)
    threshold = div(byte_size(input) * 80, 100)
    assert byte_size(compressed) < threshold,
      "prose: compressed #{byte_size(compressed)} bytes (input #{byte_size(input)}), expected < #{threshold} (80%)"
  end

  # ── TC-6: LCG pseudo-random data ────────────────────────────────────────────
  #
  # LCG pseudo-random bytes — no significant compression expected, but round-trip
  # must be byte-perfect regardless of the block type chosen (raw fallback).

  test "TC-6: LCG pseudo-random round-trip" do
    input =
      Enum.reduce(1..512, {42, []}, fn _, {seed, acc} ->
        new_seed = rem(seed * 1_664_525 + 1_013_904_223, 0x100000000)
        byte_val = new_seed &&& 0xFF
        {new_seed, [byte_val | acc]}
      end)
      |> elem(1)
      |> Enum.reverse()
      |> :binary.list_to_bin()

    assert rt(input) == input
  end

  # ── TC-7: 200 KB single-byte run — multi-block ───────────────────────────────
  #
  # 200 KB > MAX_BLOCK_SIZE (128 KB), so this requires at least 2 blocks.
  # Both should be RLE blocks since all bytes are identical.

  test "TC-7: 200 KB single-byte run (multi-block)" do
    input = String.duplicate("x", 200 * 1024)
    compressed = Zstd.compress(input)
    assert {:ok, ^input} = Zstd.decompress(compressed)
    # With RLE blocks, this should compress to well under 100 bytes
    assert byte_size(compressed) < 100,
      "200KB single-byte run should compress tiny, got #{byte_size(compressed)}"
  end

  # ── TC-8: 300 KB repetitive text — high compression ─────────────────────────
  #
  # Repetitive text > 2× MAX_BLOCK_SIZE. Tests multi-block handling with
  # compressed blocks. Expect at least 50% compression.

  test "TC-8: 300 KB repetitive text round-trip with compression" do
    input = String.duplicate("the quick brown fox jumps over the lazy dog\n", 7000)
      |> binary_part(0, 300 * 1024)
    compressed = Zstd.compress(input)
    assert {:ok, ^input} = Zstd.decompress(compressed)
    threshold = div(byte_size(input), 2)
    assert byte_size(compressed) < threshold,
      "300KB repetitive text: compressed #{byte_size(compressed)}, expected < #{threshold}"
  end

  # ── TC-9: bad magic → {:error, _} ───────────────────────────────────────────
  #
  # Any input that does not start with the ZStd magic 0xFD2FB528 must return
  # {:error, _}. We test several common cases.

  test "TC-9: bad magic returns error" do
    assert {:error, msg} = Zstd.decompress("not a zstd frame")
    assert String.contains?(msg, "bad magic") or String.contains?(msg, "frame too short")

    assert {:error, _} = Zstd.decompress(<<0x00, 0x00, 0x00, 0x00, 0x00>>)
    assert {:error, _} = Zstd.decompress("")
    assert {:error, _} = Zstd.decompress("hi")
  end

  # ── Additional round-trip tests ──────────────────────────────────────────────

  test "round-trip: binary data" do
    input = :binary.list_to_bin(Enum.map(0..299, fn i -> rem(i, 256) end))
    assert rt(input) == input
  end

  test "round-trip: all zeros" do
    input = :binary.list_to_bin(List.duplicate(0, 1000))
    assert rt(input) == input
  end

  test "round-trip: all 0xFF bytes" do
    input = :binary.list_to_bin(List.duplicate(255, 1000))
    assert rt(input) == input
  end

  test "round-trip: hello world" do
    assert rt("hello world") == "hello world"
  end

  test "round-trip: repeated pattern" do
    input = String.duplicate("ABCDEF", 500)
    assert rt(input) == input
  end

  test "round-trip: unicode text" do
    input = String.duplicate("héllo wörld — ñoño — 日本語テスト", 50)
    assert rt(input) == input
  end

  # ── Compression ratio checks ─────────────────────────────────────────────────

  test "compress produces output smaller than input for repetitive data" do
    input = String.duplicate("hello hello hello hello hello\n", 100)
    compressed = Zstd.compress(input)
    assert byte_size(compressed) < byte_size(input)
  end

  test "magic number is correct in output" do
    compressed = Zstd.compress("test")
    # ZStd magic LE: 0x28 0xB5 0x2F 0xFD
    assert binary_part(compressed, 0, 4) == <<0x28, 0xB5, 0x2F, 0xFD>>
  end

  test "compress is deterministic (same input → same output)" do
    data = String.duplicate("hello, ZStd world! ", 50)
    assert Zstd.compress(data) == Zstd.compress(data)
  end

  # ── Wire format validation (manual frame) ───────────────────────────────────
  #
  # Manually constructed ZStd frame to verify our decoder reads the wire format
  # correctly without depending on our encoder. This frame uses:
  #   Magic = 0xFD2FB528 LE = [0x28, 0xB5, 0x2F, 0xFD]
  #   FHD = 0x20: FCS_flag=00, Single_Segment=1, no checksum, no dict
  #   FCS = 0x05 (1-byte FCS when Single_Segment=1 and FCS_flag=00)
  #   Block: Last=1, Type=Raw, Size=5 → hdr = (5<<3)|(0<<1)|1 = 41 = [0x29, 0x00, 0x00]
  #   Data: b"hello"

  test "manual minimal raw-block frame decodes correctly" do
    frame = <<
      0x28, 0xB5, 0x2F, 0xFD,  # magic
      0x20,                      # FHD: Single_Segment=1, FCS=1byte
      0x05,                      # FCS = 5
      0x29, 0x00, 0x00,          # block header: last=1, raw, size=5
      ?h, ?e, ?l, ?l, ?o        # data
    >>
    assert {:ok, "hello"} = Zstd.decompress(frame)
  end

  # ── Edge cases ───────────────────────────────────────────────────────────────

  test "single newline round-trips" do
    assert rt("\n") == "\n"
  end

  test "null byte round-trips" do
    assert rt(<<0>>) == <<0>>
  end

  test "binary with null bytes round-trips" do
    input = <<0, 1, 0, 2, 0, 3, 0>> |> :binary.copy(100)
    assert rt(input) == input
  end

  test "long string with varied content round-trips" do
    input = for i <- 1..1000, into: <<>>, do: <<rem(i * 7 + 3, 256)>>
    assert rt(input) == input
  end

  test "truncated frame returns error" do
    valid = Zstd.compress("hello world")
    truncated = binary_part(valid, 0, div(byte_size(valid), 2))
    result = Zstd.decompress(truncated)
    assert match?({:error, _}, result)
  end
end
