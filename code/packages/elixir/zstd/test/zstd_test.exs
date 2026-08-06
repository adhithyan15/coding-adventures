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

  # The pure-Elixir compressor walks 300 KB through match-finding and block
  # encoding; on a heavily loaded CI runner this can exceed ExUnit's 60 s
  # default. The work is bounded (single pass over a fixed 300 KB buffer), so a
  # generous cap keeps the test meaningful without turning transient runner load
  # into a red build.
  @tag timeout: 300_000
  test "TC-8: 300 KB repetitive text round-trip with compression" do
    input = String.duplicate("the quick brown fox jumps over the lazy dog\n", 7000)
      |> binary_part(0, 300 * 1024)
    compressed = Zstd.compress(input)
    assert {:ok, ^input} = Zstd.decompress(compressed)
    threshold = div(byte_size(input), 2)
    assert byte_size(compressed) < threshold,
      "300KB repetitive text: compressed #{byte_size(compressed)}, expected < #{threshold}"
  end

  # ── TC-9: Cross-language / interoperability (real `zstd` CLI) ──────────────
  #
  # code/specs/CMP07-zstd.md designates TC-9 as cross-implementation
  # round-trip against the real `zstd` CLI, in BOTH directions:
  #
  #   1. Compress with `Zstd.compress/1`, decompress with `zstd -d`.
  #   2. Compress with `zstd`, decompress with `Zstd.decompress/1`.
  #
  # This is the ONLY kind of test that can catch a systematic, symmetric
  # protocol deviation: a same-codebase round-trip test (`rt/1` above) is
  # necessary but never sufficient, because if the encoder and decoder agree
  # with each other on a WRONG convention, every internal round-trip still
  # passes. That is exactly what happened here — three compounding bugs in
  # the FSE sequences-section codec (a fabricated two-pass symbol-table
  # spread, a wrong per-sequence field order, and an unconditional final
  # state-transition flush that should have been skipped for the block's
  # last sequence) survived undetected because this package had no
  # independent, spec-conformant interop check. See lessons.md Lesson 96.
  #
  # `previously_broken_fse_repro/0` below is the minimal case from that bug
  # report (`compress("ababababab" * 3)`, one sequence: ll=2, ml=28,
  # offset=2) — it alone is enough to reproduce the "Data corruption
  # detected" failure against a pre-fix build of this codec.
  #
  # Skipped (not failed) when the `zstd` binary isn't on PATH, since CI/dev
  # environments vary — mirrors the approach taken in the java/zstd and
  # rust/zstd ports of this same test.
  #
  # Repeat-offset inputs (where the SAME 8-byte pattern recurs at a FIXED
  # distance, or a long constant-byte run) are now INCLUDED below — see the
  # dedicated "repeat-offset" test group further down. They used to be
  # deliberately avoided here on the theory that this codec's decoder
  # couldn't resolve the real zstd Repeated_Offset (R1/R2/R3) mechanism
  # (RFC 8878 §3.1.1.3.2.1.1). That theory was correct as a description of
  # a real, independent gap — but it was NOT documented in
  # code/specs/CMP07-zstd.md's "Educational Simplification" table (which
  # lists no repeat-offset carve-out), and the decoder now implements the
  # mechanism in full. See lessons.md Lesson 98 for how this class of gap
  # was found (in `code/packages/c/zstd`, via PR #9941) and audited into
  # every other language port, including this one.

  defp zstd_cli_available? do
    case System.cmd("zstd", ["--version"], stderr_to_stdout: true) do
      {_, 0} -> true
      _ -> false
    end
  rescue
    ErlangError -> false
  end

  defp previously_broken_fse_repro, do: String.duplicate("ababababab", 3)

  # Unique, hard-to-predict temp-file name: mixes a monotonic counter (for
  # human-readable ordering) with 8 bytes of CSPRNG output, so a co-resident
  # local user can't pre-guess the path and race it with a symlink between
  # our File.write!/2 and the `zstd` CLI's read (or vice versa). This is
  # belt-and-suspenders for test-only temp files with no sensitive content —
  # not a defense against a privileged attacker.
  defp unique_tmp_name(prefix) do
    counter = :erlang.unique_integer([:positive])
    rand = :crypto.strong_rand_bytes(8) |> Base.encode16(case: :lower)
    "#{prefix}-#{counter}-#{rand}"
  end

  @tag :cli_interop
  test "TC-9: CLI interoperability — compress ours, decompress with real `zstd -d`" do
    if zstd_cli_available?() do
      for input <- [
            previously_broken_fse_repro(),
            String.duplicate("the quick brown fox jumps over the lazy dog ", 25),
            # ~9 KB of a repeating 6-byte cycle: comfortably pushes the
            # single-block sequence count past 128, the exact boundary where
            # Number_of_Sequences switches from its 1-byte to 2-byte wire
            # form (RFC 8878 §3.1.1.3.1) — regression coverage for the
            # sequence-count byte-order bug noted elsewhere in this file.
            String.duplicate("ABCDEF", 1500) |> binary_part(0, 9000)
          ] do
        compressed = Zstd.compress(input)
        tmp_zst = Path.join(System.tmp_dir!(), "#{unique_tmp_name("ex-zstd-tc9")}.zst")
        File.write!(tmp_zst, compressed)

        try do
          {output, exit_code} = System.cmd("zstd", ["-d", "-q", "-c", tmp_zst], stderr_to_stdout: true)
          assert exit_code == 0,
            "real `zstd -d` failed to decode our compressed output (#{byte_size(input)} bytes): #{output}"
          assert output == input,
            "real `zstd -d` decoded our output but got different bytes back"
        after
          File.rm(tmp_zst)
        end
      end
    else
      IO.puts(:stderr, "zstd CLI not found on PATH — skipping TC-9 interop test")
    end
  end

  @tag :cli_interop
  test "TC-9: CLI interoperability — compress with real `zstd`, decompress ours" do
    if zstd_cli_available?() do
      for input <- [
            previously_broken_fse_repro(),
            String.duplicate("the quick brown fox jumps over the lazy dog ", 25),
            String.duplicate("ABCDEF", 1500) |> binary_part(0, 9000)
          ] do
        unique = unique_tmp_name("ex-zstd-tc9")
        tmp_txt = Path.join(System.tmp_dir!(), "#{unique}.txt")
        tmp_zst = Path.join(System.tmp_dir!(), "#{unique}.zst")
        File.write!(tmp_txt, input)

        try do
          {_output, exit_code} = System.cmd("zstd", ["-q", "-f", "-o", tmp_zst, tmp_txt], stderr_to_stdout: true)
          assert exit_code == 0, "real `zstd` CLI failed to compress the test input"

          cli_compressed = File.read!(tmp_zst)
          assert {:ok, ^input} = Zstd.decompress(cli_compressed),
            "our decompress/1 failed to decode real `zstd`'s compressed output (#{byte_size(input)} bytes)"
        after
          File.rm(tmp_txt)
          File.rm(tmp_zst)
        end
      end
    else
      IO.puts(:stderr, "zstd CLI not found on PATH — skipping TC-9 interop test")
    end
  end

  # ── Repeat-Offset (R1/R2/R3) interoperability (lessons.md Lesson 98) ───────
  #
  # RFC 8878 §3.1.1.3.2.1.1 defines a Repeated-Offset mechanism: Offset_Value
  # <= 3 is not a literal distance but a reference into a 3-slot history of
  # recently-used offsets (R1/R2/R3, default 1/4/8), letting the real `zstd`
  # encoder spend ~2 bits instead of a full offset code whenever a match
  # reuses a recent distance. Real `zstd` encoders use this CONSTANTLY —
  # it's one of their principal entropy wins, especially for periodic or
  # highly repetitive data (exactly what a compression test suite tends to
  # contain).
  #
  # This package's OWN encoder never emits repeat-offset codes (an
  # intentional educational simplification — see encode_sequences_section's
  # module doc), so this codec's own compress()/decompress() round trip
  # (`rt/1`, and TC-1..TC-8 above) NEVER exercises the decode-side
  # Repeated-Offset path — nor did TC-9's original fixed corpus, which
  # apparently never produced a real-`zstd`-encoded sequence with
  # Offset_Value <= 3. That gap was invisible to every test in this suite
  # until audited in via lessons.md Lesson 98 (the same audit that first
  # found and fixed it in `code/packages/c/zstd`, PR #9941): a decoder that
  # only understood the explicit-offset path would either crash (an
  # out-of-bounds binary access from a negative `offset = of_raw - 3` when
  # of_raw is 1..3) or emit wrong bytes, on any real-world `.zst` frame that
  # happens to use repeat offsets.
  #
  # These inputs deliberately target that path: a fixed pattern recurring at
  # a CONSTANT distance (spec CMP07-zstd.md's own TC-8 shape) and a long
  # constant-byte run (the Lesson-98 minimal repro: real `zstd` compresses
  # 4713 bytes of `'Z'` as a Compressed block with one sequence whose
  # Offset_Value is 1 — "reuse Repeated_Offset1" — rather than the RLE block
  # type this port's own encoder would choose for the same input).
  @tag :cli_interop
  test "repeat-offset: real `zstd`-compressed repeat-offset frames decode correctly" do
    if zstd_cli_available?() do
      pattern = "ABCDEFGH"
      fixed_distance_repeat = pattern <> String.duplicate(String.duplicate("X", 128) <> pattern, 10)

      for {name, input} <- [
            {"constant-byte run (Lesson 98 minimal repro)", String.duplicate("Z", 4713)},
            {"pattern at fixed distance (spec TC-8 shape)", fixed_distance_repeat},
            {"short constant run", String.duplicate("q", 50)},
            {"periodic 3-byte cycle", String.duplicate("xyz", 4000)}
          ] do
        unique = unique_tmp_name("ex-zstd-repoff")
        tmp_txt = Path.join(System.tmp_dir!(), "#{unique}.txt")
        tmp_zst = Path.join(System.tmp_dir!(), "#{unique}.zst")
        File.write!(tmp_txt, input)

        try do
          {_output, exit_code} = System.cmd("zstd", ["-q", "-f", "-o", tmp_zst, tmp_txt], stderr_to_stdout: true)
          assert exit_code == 0, "real `zstd` CLI failed to compress #{name}"

          cli_compressed = File.read!(tmp_zst)
          assert {:ok, ^input} = Zstd.decompress(cli_compressed),
            "our decompress/1 failed to decode real `zstd`'s repeat-offset output for #{name} (#{byte_size(input)} bytes)"
        after
          File.rm(tmp_txt)
          File.rm(tmp_zst)
        end
      end
    else
      IO.puts(:stderr, "zstd CLI not found on PATH — skipping repeat-offset interop test")
    end
  end

  # Same set of inputs, but round-tripped purely through THIS package's own
  # compress/1 followed by decompress/1. This package's encoder never emits
  # repeat-offset codes, so this direction alone would never have caught
  # Lesson 98 — it's included for completeness/regression coverage of the
  # ordinary explicit-offset path on the same inputs, not as evidence the
  # repeat-offset decode path works (the CLI-interop test above is the one
  # that actually proves that).
  test "repeat-offset-shaped inputs still round-trip through our own encoder" do
    pattern = "ABCDEFGH"
    fixed_distance_repeat = pattern <> String.duplicate(String.duplicate("X", 128) <> pattern, 10)

    for input <- [
          String.duplicate("Z", 4713),
          fixed_distance_repeat,
          String.duplicate("q", 50),
          String.duplicate("xyz", 4000)
        ] do
      assert rt(input) == input
    end
  end

  # ── Error handling: bad magic → {:error, _} ─────────────────────────────────
  #
  # Any input that does not start with the ZStd magic 0xFD2FB528 must return
  # {:error, _}. We test several common cases.
  #
  # (Not one of the spec's 10 mandatory TCs — TC-9 is Cross-language /
  # interoperability, above. An earlier revision of this file mislabelled
  # this test "TC-9", which collided with the spec's numbering.)

  test "bad magic returns error" do
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

  # ── Regression: seq_count endianness bug ────────────────────────────────────
  #
  # The 2-byte seq_count form must place the format-flag byte (bit 7 set) FIRST.
  # An earlier broken pattern in TS+Go wrote `[count & 0xFF, (count >> 8) | 0x80]`
  # — low byte first. For any count ≥ 128 whose low byte happened to be < 128
  # (e.g. 515 → byte0 = 0x03), the decoder mis-took the 1-byte path and silently
  # returned a tiny garbage count, mis-aligning every byte downstream.
  #
  # 200 KB of long-period repetitive text reliably yields ≥ 128 sequences in a
  # single block (LZSS finds ~one match per pattern repetition). This is the
  # canonical regression — the same shape as the TS/Go regression in PR #1448.
  test "seq_count: 200 KB repetitive text — endianness regression" do
    pattern = "hello world and more text for compression testing!\n"
    input = pattern |> List.duplicate(4000) |> IO.iodata_to_binary()
    assert rt(input) == input
  end
end
