defmodule CodingAdventures.Brotli do
  import Bitwise

  @moduledoc """
  Brotli Compression — CMP06 wire-format compress/decompress.

  ## What Is Brotli?

  Brotli (2013, RFC 7932) is a lossless compression algorithm developed at Google that
  achieves significantly better compression ratios than DEFLATE, particularly on web
  content (HTML, CSS, JavaScript). It became the dominant algorithm for HTTP
  `Content-Encoding: br` compression.

  Brotli builds on DEFLATE's foundation but adds three major innovations:

  1. **Context-dependent literal trees** — instead of one Huffman tree for all literals,
     Brotli assigns each literal to one of 4 *context buckets* based on the preceding
     byte. Each context bucket gets its own Huffman tree.

  2. **Insert-and-copy commands** — instead of separate literal and back-reference tokens,
     Brotli uses *commands* that bundle an insert run (raw literals) with a copy
     operation. Both lengths are encoded together as a single ICC Huffman symbol.

  3. **Larger sliding window** — 65535 bytes instead of DEFLATE's 4096.

  ## Encoding Order

  Each regular command (copy_length >= 4) in the bit stream:

  ```
  [ICC Huffman code]
  [insert_length extra bits, LSB-first]
  [copy_length extra bits, LSB-first]
  [literal bytes, each encoded via per-context Huffman tree]
  [distance code Huffman code]
  [distance extra bits, LSB-first]
  ```

  End of stream:

  ```
  [ICC=63 sentinel]
  [flush literal bytes, if any — encoded AFTER the sentinel]
  ```

  ## Context Buckets

  Each literal is assigned to a context based on the preceding byte:

  ```
  bucket 0 — space or punctuation (or start-of-stream: no previous byte)
  bucket 1 — digit ('0'–'9')
  bucket 2 — uppercase letter ('A'–'Z')
  bucket 3 — lowercase letter ('a'–'z')
  ```

  ## Flush Literals

  Trailing literals that cannot be bundled into a regular ICC command (because
  there is no LZ match after them) are encoded **after** the sentinel ICC=63.
  The decoder reads them after breaking out of the ICC loop, until `original_length`
  bytes have been produced.

  ## Wire Format (CMP06)

  ```
  Header (10 bytes):
    [4B] original_length       big-endian uint32
    [1B] icc_entry_count       entries in ICC table (1-64)
    [1B] dist_entry_count      entries in dist table (0-32)
    [1B] ctx0_entry_count      entries in literal tree 0
    [1B] ctx1_entry_count      entries in literal tree 1
    [1B] ctx2_entry_count      entries in literal tree 2
    [1B] ctx3_entry_count      entries in literal tree 3

  ICC table (icc_entry_count × 2 bytes): symbol::8, code_length::8
  Dist table (dist_entry_count × 2 bytes): symbol::8, code_length::8
  Literal trees 0-3 (entry_count × 3 bytes each): symbol::16, code_length::8
  Bit stream: LSB-first packed bits, zero-padded to byte boundary.
  ```

  ## Examples

      iex> data = "hello hello hello"
      iex> CodingAdventures.Brotli.decompress(CodingAdventures.Brotli.compress(data)) == data
      true

  """

  alias CodingAdventures.HuffmanTree

  # ---------------------------------------------------------------------------
  # ICC (Insert-Copy Code) Table — 64 codes
  # ---------------------------------------------------------------------------
  #
  # Each ICC code covers a range of insert lengths and a range of copy lengths.
  # Extra bits after the Huffman code select the exact value within the range.
  #
  # Format: {code, insert_base, insert_extra_bits, copy_base, copy_extra_bits}
  #
  # Code 63 is the end-of-data sentinel (insert=0, copy=0).
  #
  # Insert ranges by group (16 codes per group):
  #   Codes  0-15: insert_base=0, insert_extra=0 → exactly 0
  #   Codes 16-23: insert_base=1, insert_extra=0 → exactly 1
  #   Codes 24-31: insert_base=2, insert_extra=0 → exactly 2
  #   Codes 32-39: insert_base=3, insert_extra=1 → 3-4
  #   Codes 40-47: insert_base=5, insert_extra=2 → 5-8
  #   Codes 48-55: insert_base=9, insert_extra=3 → 9-16
  #   Codes 56-62: insert_base=17, insert_extra=4 → 17-32
  #   Code    63: sentinel (insert=0, copy=0)

  @icc_table [
    { 0,  0, 0,   4, 0},
    { 1,  0, 0,   5, 0},
    { 2,  0, 0,   6, 0},
    { 3,  0, 0,   8, 1},
    { 4,  0, 0,  10, 1},
    { 5,  0, 0,  14, 2},
    { 6,  0, 0,  18, 2},
    { 7,  0, 0,  26, 3},
    { 8,  0, 0,  34, 3},
    { 9,  0, 0,  50, 4},
    {10,  0, 0,  66, 4},
    {11,  0, 0,  98, 5},
    {12,  0, 0, 130, 5},
    {13,  0, 0, 194, 6},
    {14,  0, 0, 258, 7},
    {15,  0, 0, 514, 8},
    {16,  1, 0,   4, 0},
    {17,  1, 0,   5, 0},
    {18,  1, 0,   6, 0},
    {19,  1, 0,   8, 1},
    {20,  1, 0,  10, 1},
    {21,  1, 0,  14, 2},
    {22,  1, 0,  18, 2},
    {23,  1, 0,  26, 3},
    {24,  2, 0,   4, 0},
    {25,  2, 0,   5, 0},
    {26,  2, 0,   6, 0},
    {27,  2, 0,   8, 1},
    {28,  2, 0,  10, 1},
    {29,  2, 0,  14, 2},
    {30,  2, 0,  18, 2},
    {31,  2, 0,  26, 3},
    {32,  3, 1,   4, 0},
    {33,  3, 1,   5, 0},
    {34,  3, 1,   6, 0},
    {35,  3, 1,   8, 1},
    {36,  3, 1,  10, 1},
    {37,  3, 1,  14, 2},
    {38,  3, 1,  18, 2},
    {39,  3, 1,  26, 3},
    {40,  5, 2,   4, 0},
    {41,  5, 2,   5, 0},
    {42,  5, 2,   6, 0},
    {43,  5, 2,   8, 1},
    {44,  5, 2,  10, 1},
    {45,  5, 2,  14, 2},
    {46,  5, 2,  18, 2},
    {47,  5, 2,  26, 3},
    {48,  9, 3,   4, 0},
    {49,  9, 3,   5, 0},
    {50,  9, 3,   6, 0},
    {51,  9, 3,   8, 1},
    {52,  9, 3,  10, 1},
    {53,  9, 3,  14, 2},
    {54,  9, 3,  18, 2},
    {55,  9, 3,  26, 3},
    {56, 17, 4,   4, 0},
    {57, 17, 4,   5, 0},
    {58, 17, 4,   6, 0},
    {59, 17, 4,   8, 1},
    {60, 17, 4,  10, 1},
    {61, 17, 4,  14, 2},
    {62, 17, 4,  18, 2},
    {63,  0, 0,   0, 0},  # end-of-data sentinel
  ]

  @icc_insert_base  @icc_table |> Enum.map(fn {c, ib, _, _, _} -> {c, ib} end) |> Map.new()
  @icc_insert_extra @icc_table |> Enum.map(fn {c, _, ie, _, _} -> {c, ie} end) |> Map.new()
  @icc_copy_base    @icc_table |> Enum.map(fn {c, _, _, cb, _} -> {c, cb} end) |> Map.new()
  @icc_copy_extra   @icc_table |> Enum.map(fn {c, _, _, _, ce} -> {c, ce} end) |> Map.new()

  # Maximum insert length representable by a single ICC code.
  # Codes 56-62: insert_base=17, insert_extra=4 → max = 17 + (1<<4) - 1 = 32
  @max_insert_per_icc 32

  # ---------------------------------------------------------------------------
  # Distance code table (32 codes, distances 1-65535)
  # ---------------------------------------------------------------------------

  @dist_table [
    { 0,     1,  0},
    { 1,     2,  0},
    { 2,     3,  0},
    { 3,     4,  0},
    { 4,     5,  1},
    { 5,     7,  1},
    { 6,     9,  2},
    { 7,    13,  2},
    { 8,    17,  3},
    { 9,    25,  3},
    {10,    33,  4},
    {11,    49,  4},
    {12,    65,  5},
    {13,    97,  5},
    {14,   129,  6},
    {15,   193,  6},
    {16,   257,  7},
    {17,   385,  7},
    {18,   513,  8},
    {19,   769,  8},
    {20,  1025,  9},
    {21,  1537,  9},
    {22,  2049, 10},
    {23,  3073, 10},
    {24,  4097, 11},
    {25,  6145, 11},
    {26,  8193, 12},
    {27, 12289, 12},
    {28, 16385, 13},
    {29, 24577, 13},
    {30, 32769, 14},
    {31, 49153, 14},
  ]

  @dist_base  @dist_table |> Enum.map(fn {code, base, _} -> {code, base} end) |> Map.new()
  @dist_extra @dist_table |> Enum.map(fn {code, _, extra} -> {code, extra} end) |> Map.new()

  # ---------------------------------------------------------------------------
  # Context bucket function
  # ---------------------------------------------------------------------------
  #
  # Determines the literal context bucket (0-3) from the last emitted byte.
  #
  # -1 means "no previous byte" (start of stream) → bucket 0.
  # This matches the spec: "at the start of the stream, bucket 0 is used."

  defp literal_context(-1), do: 0
  defp literal_context(p1) do
    cond do
      p1 >= ?a and p1 <= ?z -> 3  # lowercase
      p1 >= ?A and p1 <= ?Z -> 2  # uppercase
      p1 >= ?0 and p1 <= ?9 -> 1  # digit
      true                  -> 0  # space/punctuation/other
    end
  end

  # ---------------------------------------------------------------------------
  # LZ matching — O(n²) sliding window scan
  # ---------------------------------------------------------------------------
  #
  # Scans backwards from `pos` in `data` to find the longest match starting
  # at or after `window_start`. Minimum match length: 4. Maximum: 258.
  #
  # Returns {distance, length} or {0, 0} if no qualifying match is found.
  #
  # Distance is 1-based: distance=1 means the match starts at pos-1.

  @min_match 4
  @max_match 258
  @max_window 65535

  defp find_longest_match(data, pos, window_start) do
    data_len = byte_size(data)
    max_len = min(@max_match, data_len - pos)

    if max_len < @min_match or pos == 0 do
      {0, 0}
    else
      # Scan all candidate start positions from window_start to pos-1.
      # For each candidate, count how many consecutive bytes match.
      Enum.reduce(window_start..(pos - 1)//1, {0, 0}, fn candidate, {best_dist, best_len} ->
        match_len = prefix_match_length(data, candidate, pos, max_len)
        if match_len > best_len do
          {pos - candidate, match_len}
        else
          {best_dist, best_len}
        end
      end)
      |> then(fn {dist, len} ->
        if len >= @min_match, do: {dist, len}, else: {0, 0}
      end)
    end
  end

  # Count matching bytes between data[a..] and data[b..], up to max_len bytes.
  # Handles overlapping matches: when a + i >= b, the pattern "wraps around"
  # and repeats (run-length encoding effect).
  defp prefix_match_length(data, a, b, max_len) do
    data_len = byte_size(data)
    Enum.reduce_while(0..(max_len - 1), 0, fn i, acc ->
      ba = if a + i < data_len, do: :binary.at(data, a + i), else: -1
      bb = if b + i < data_len, do: :binary.at(data, b + i), else: -2
      if ba == bb, do: {:cont, acc + 1}, else: {:halt, acc}
    end)
  end

  # ---------------------------------------------------------------------------
  # ICC code selection
  # ---------------------------------------------------------------------------
  #
  # Given insert_length and copy_length, find the ICC code (0-62) whose
  # insert range contains insert_length AND whose copy range contains copy_length.
  #
  # The ICC table has gaps in copy-length coverage (e.g., copy=7 is not
  # representable for any code). Use find_best_icc_copy/2 to snap the
  # copy_length down to the nearest representable value before calling this.

  defp find_icc_code(insert_len, copy_len) do
    Enum.find_value(0..62, 0, fn code ->
      ib = Map.get(@icc_insert_base, code)
      ie = Map.get(@icc_insert_extra, code)
      cb = Map.get(@icc_copy_base, code)
      ce = Map.get(@icc_copy_extra, code)
      max_ins = ib + (1 <<< ie) - 1
      max_cpy = cb + (1 <<< ce) - 1
      if insert_len >= ib and insert_len <= max_ins and
         copy_len >= cb and copy_len <= max_cpy do
        code
      end
    end)
  end

  # Snap copy_length down to the largest representable value <= requested,
  # for the given insert_length. This handles ICC table gaps.
  defp find_best_icc_copy(insert_len, copy_len) do
    Enum.reduce(0..62, 0, fn code, best ->
      ib = Map.get(@icc_insert_base, code)
      ie = Map.get(@icc_insert_extra, code)
      cb = Map.get(@icc_copy_base, code)
      ce = Map.get(@icc_copy_extra, code)
      max_ins = ib + (1 <<< ie) - 1
      max_cpy = cb + (1 <<< ce) - 1

      if insert_len >= ib and insert_len <= max_ins do
        cond do
          copy_len >= cb and copy_len <= max_cpy -> copy_len  # exact match
          max_cpy <= copy_len and max_cpy > best -> max_cpy   # best below gap
          true -> best
        end
      else
        best
      end
    end)
    |> max(@min_match)
  end

  defp dist_code(distance) do
    Enum.find_value(@dist_table, 31, fn {code, base, extra} ->
      max_dist = base + (1 <<< extra) - 1
      if distance >= base and distance <= max_dist, do: code, else: nil
    end)
  end

  # ---------------------------------------------------------------------------
  # Bit I/O helpers
  # ---------------------------------------------------------------------------

  # Pack a list of bits (0/1 integers) into bytes, LSB first.
  defp pack_bits_lsb_first(bits) when is_list(bits) do
    bits
    |> Enum.chunk_every(8, 8, List.duplicate(0, 7))
    |> Enum.map(fn byte_bits ->
      byte_bits
      |> Enum.with_index()
      |> Enum.reduce(0, fn {bit, pos}, acc -> acc ||| (bit <<< pos) end)
    end)
    |> :erlang.list_to_binary()
  end

  # Unpack bytes into a list of bits (0/1 integers), LSB first.
  defp unpack_bits_lsb_first(data) when is_binary(data) do
    for <<byte::8 <- data>>,
        i <- 0..7,
        do: (byte >>> i) &&& 1
  end

  # Emit `n` bits of `val`, LSB first, as a list of 0/1 integers.
  defp extra_bits_lsb(_val, 0), do: []
  defp extra_bits_lsb(val, n) do
    for i <- 0..(n - 1), do: (val >>> i) &&& 1
  end

  # ---------------------------------------------------------------------------
  # Canonical code reconstruction (for decompression)
  # ---------------------------------------------------------------------------
  #
  # Given a sorted list of {symbol, code_length} pairs (code_length ASC,
  # symbol ASC), reconstruct the canonical Huffman bit strings.
  # Returns a map of bit_string → symbol.
  #
  # Single-symbol trees: code is "0" (length 1).

  defp reconstruct_canonical_codes([]) do
    %{}
  end

  defp reconstruct_canonical_codes([{sym, _len}]) do
    %{"0" => sym}
  end

  defp reconstruct_canonical_codes(lengths) do
    {result, _, _} =
      Enum.reduce(lengths, {%{}, 0, nil}, fn {sym, code_len}, {acc, code, prev_len} ->
        code2 =
          if prev_len != nil and code_len > prev_len do
            code <<< (code_len - prev_len)
          else
            code
          end
        bit_str = Integer.to_string(code2, 2) |> String.pad_leading(code_len, "0")
        {Map.put(acc, bit_str, sym), code2 + 1, code_len}
      end)
    result
  end

  # ---------------------------------------------------------------------------
  # Huffman symbol encoding/decoding helpers
  # ---------------------------------------------------------------------------

  defp encode_symbol(code_table, sym) do
    Map.get(code_table, sym) |> String.graphemes() |> Enum.map(&String.to_integer/1)
  end

  defp next_huffman_symbol(bits, pos, rev_map) do
    do_next_huffman(bits, pos, rev_map, "")
  end

  defp do_next_huffman(bits, pos, rev_map, acc) do
    bit = Enum.at(bits, pos, 0)
    new_acc = acc <> Integer.to_string(bit)
    case Map.get(rev_map, new_acc) do
      nil -> do_next_huffman(bits, pos + 1, rev_map, new_acc)
      sym -> {sym, pos + 1}
    end
  end

  defp read_bits(_bits, pos, 0), do: {0, pos}
  defp read_bits(bits, pos, n) do
    {val, new_pos} =
      Enum.reduce(0..(n - 1), {0, pos}, fn i, {acc, p} ->
        bit = Enum.at(bits, p, 0)
        {acc ||| (bit <<< i), p + 1}
      end)
    {val, new_pos}
  end

  # ---------------------------------------------------------------------------
  # Wire format helpers
  # ---------------------------------------------------------------------------

  defp serialize_code_lengths_1byte(code_table) do
    sorted =
      code_table
      |> Enum.map(fn {sym, code} -> {sym, String.length(code)} end)
      |> Enum.sort(fn {sa, la}, {sb, lb} ->
        if la != lb, do: la < lb, else: sa < sb
      end)

    count = length(sorted)
    bytes = for {sym, len} <- sorted, into: <<>>, do: <<sym::8, len::8>>
    {count, bytes}
  end

  defp serialize_code_lengths_2byte(code_table) do
    sorted =
      code_table
      |> Enum.map(fn {sym, code} -> {sym, String.length(code)} end)
      |> Enum.sort(fn {sa, la}, {sb, lb} ->
        if la != lb, do: la < lb, else: sa < sb
      end)

    count = length(sorted)
    bytes = for {sym, len} <- sorted, into: <<>>, do: <<sym::16, len::8>>
    {count, bytes}
  end

  # ---------------------------------------------------------------------------
  # Public API: compress/1
  # ---------------------------------------------------------------------------

  @doc """
  Compress `data` using Brotli (CMP06) and return wire-format bytes.

  Accepts a binary or a list of byte values.

  ## Algorithm

  Pass 1: LZ matching → regular commands (copy_length >= 4) + flush literals.
  Pass 2a: Frequency tallying (ICC, dist, per-context literals).
  Pass 2b: Huffman tree construction.
  Pass 2c: Bit stream encoding:
    - Per regular command: [ICC][insert extras][copy extras][literals][dist][dist extras]
    - End of stream: [ICC=63][flush literals]
  Wire format: 10-byte header + tables + bit stream.

  ## Flush Literals

  Trailing bytes that have no LZ match after them are emitted AFTER the sentinel
  ICC=63. The decoder reads them until `original_length` bytes are produced.
  """
  def compress(data) when is_binary(data) or is_list(data) do
    data = if is_list(data), do: :binary.list_to_bin(data), else: data
    original_length = byte_size(data)

    if original_length == 0 do
      # Empty input: header + ICC entry (63, length=1) + sentinel bit "0".
      header = <<0::32, 1::8, 0::8, 0::8, 0::8, 0::8, 0::8>>
      header <> <<63::8, 1::8>> <> <<0::8>>
    else
      # ── Pass 1: LZ matching ───────────────────────────────────────────────
      {commands, flush_literals} = build_commands(data)

      # ── Pass 2a: Frequency tallying ───────────────────────────────────────
      {lit_freqs, icc_freq, dist_freq} = tally_frequencies(commands, flush_literals)

      # ── Pass 2b: Build Huffman trees ──────────────────────────────────────
      icc_tree = HuffmanTree.build(Map.to_list(icc_freq))
      icc_code_table = HuffmanTree.canonical_code_table(icc_tree)

      dist_code_table =
        if map_size(dist_freq) > 0 do
          dist_tree = HuffmanTree.build(Map.to_list(dist_freq))
          HuffmanTree.canonical_code_table(dist_tree)
        else
          %{}
        end

      lit_code_tables =
        Enum.map(0..3, fn ctx ->
          freq = Map.get(lit_freqs, ctx, %{})
          if map_size(freq) > 0 do
            tree = HuffmanTree.build(Map.to_list(freq))
            HuffmanTree.canonical_code_table(tree)
          else
            %{}
          end
        end)

      # ── Pass 2c: Encode bit stream ────────────────────────────────────────
      bit_list = encode_commands(
        commands, flush_literals,
        icc_code_table, dist_code_table, lit_code_tables
      )
      bit_stream = pack_bits_lsb_first(bit_list)

      # ── Assemble wire format ──────────────────────────────────────────────
      {icc_count, icc_bytes} = serialize_code_lengths_1byte(icc_code_table)
      {dist_count, dist_bytes} = serialize_code_lengths_1byte(dist_code_table)

      {ctx_counts, ctx_bytes_list} =
        Enum.map(0..3, fn ctx ->
          serialize_code_lengths_2byte(Enum.at(lit_code_tables, ctx))
        end)
        |> Enum.unzip()

      [ctx0_count, ctx1_count, ctx2_count, ctx3_count] = ctx_counts
      ctx_bytes = Enum.reduce(ctx_bytes_list, <<>>, &(&2 <> &1))

      header = <<
        original_length::32,
        icc_count::8,
        dist_count::8,
        ctx0_count::8,
        ctx1_count::8,
        ctx2_count::8,
        ctx3_count::8
      >>

      header <> icc_bytes <> dist_bytes <> ctx_bytes <> bit_stream
    end
  end

  # ---------------------------------------------------------------------------
  # build_commands/1 — LZ pass
  # ---------------------------------------------------------------------------
  #
  # Produces a list of regular commands (each with copy_length >= 4 and a valid
  # ICC code) followed by the sentinel {0, 0, 0, []}, plus a separate list of
  # flush_literals (trailing bytes encoded after the sentinel).
  #
  # We only take an LZ match when insert_buf <= @max_insert_per_icc (32), which
  # ensures every command has a representable ICC code.

  defp build_commands(data) do
    data_len = byte_size(data)
    {commands_rev, insert_buf_rev} = do_lz_scan(data, data_len, 0, [], [])

    flush_literals = Enum.reverse(insert_buf_rev)
    sentinel = {0, 0, 0, []}

    {Enum.reverse([sentinel | commands_rev]), flush_literals}
  end

  defp do_lz_scan(_data, data_len, pos, commands_rev, insert_buf_rev) when pos >= data_len do
    {commands_rev, insert_buf_rev}
  end

  defp do_lz_scan(data, data_len, pos, commands_rev, insert_buf_rev) do
    window_start = max(0, pos - @max_window)
    {distance, match_len} = find_longest_match(data, pos, window_start)
    insert_len = length(insert_buf_rev)

    if match_len >= @min_match and insert_len <= @max_insert_per_icc do
      # Snap copy to nearest representable ICC copy value.
      actual_copy = find_best_icc_copy(insert_len, match_len)
      literals = Enum.reverse(insert_buf_rev)
      cmd = {insert_len, actual_copy, distance, literals}
      do_lz_scan(data, data_len, pos + actual_copy, [cmd | commands_rev], [])
    else
      byte = :binary.at(data, pos)
      do_lz_scan(data, data_len, pos + 1, commands_rev, [byte | insert_buf_rev])
    end
  end

  # ---------------------------------------------------------------------------
  # tally_frequencies/2
  # ---------------------------------------------------------------------------
  #
  # Walk commands + flush_literals and count:
  # - lit_freqs: map of ctx → map of byte → count
  # - icc_freq: map of icc_code → count (always includes sentinel 63)
  # - dist_freq: map of dist_code → count
  #
  # Simulates the output history (p1 tracking) for correct context assignment.

  defp tally_frequencies(commands, flush_literals) do
    {lit_freqs, icc_freq, dist_freq, history} =
      tally_commands(commands, %{}, %{}, %{}, [])

    # Tally flush literals (encoded AFTER the sentinel in the bit stream).
    # The context is the last byte from the regular-command phase.
    p1 = if history == [], do: -1, else: List.last(history)
    {lit_freqs2, _} =
      Enum.reduce(flush_literals, {lit_freqs, p1}, fn byte, {lf, prev} ->
        ctx = literal_context(prev)
        ctx_freq = Map.get(lf, ctx, %{})
        ctx_freq2 = Map.update(ctx_freq, byte, 1, &(&1 + 1))
        {Map.put(lf, ctx, ctx_freq2), byte}
      end)

    {lit_freqs2, icc_freq, dist_freq}
  end

  # Process commands until sentinel {0,0,0,[]}. Returns accumulated state.
  defp tally_commands([], lit_freqs, icc_freq, dist_freq, history) do
    {lit_freqs, icc_freq, dist_freq, history}
  end

  defp tally_commands([cmd | rest], lit_freqs, icc_freq, dist_freq, history) do
    {insert_len, copy_len, copy_dist, literals} = cmd

    if copy_len == 0 do
      # Sentinel: add it to ICC frequencies and stop.
      icc_freq2 = Map.update(icc_freq, 63, 1, &(&1 + 1))
      tally_commands([], lit_freqs, icc_freq2, dist_freq, history)
    else
      # Regular command: tally ICC and dist.
      icc = find_icc_code(insert_len, copy_len)
      dc = dist_code(copy_dist)
      icc_freq2 = Map.update(icc_freq, icc, 1, &(&1 + 1))
      dist_freq2 = Map.update(dist_freq, dc, 1, &(&1 + 1))

      # Tally insert literals with context tracking.
      p1_start = if history == [], do: -1, else: List.last(history)
      {lit_freqs2, history2} =
        Enum.reduce(literals, {lit_freqs, {p1_start, history}}, fn byte, {lf, {p1, hist}} ->
          ctx = literal_context(p1)
          ctx_freq = Map.get(lf, ctx, %{})
          ctx_freq2 = Map.update(ctx_freq, byte, 1, &(&1 + 1))
          {Map.put(lf, ctx, ctx_freq2), {byte, hist ++ [byte]}}
        end)
        |> then(fn {lf, {_p1, hist}} -> {lf, hist} end)

      # Simulate copy for history tracking (byte-by-byte for overlap support).
      copy_start = length(history2) - copy_dist
      history3 =
        Enum.reduce(0..(copy_len - 1), history2, fn i, hist ->
          byte = Enum.at(hist, copy_start + i)
          hist ++ [byte]
        end)

      tally_commands(rest, lit_freqs2, icc_freq2, dist_freq2, history3)
    end
  end

  # ---------------------------------------------------------------------------
  # encode_commands/5
  # ---------------------------------------------------------------------------
  #
  # Encodes the full bit stream:
  #
  # For each regular command:
  #   [ICC][insert extras][copy extras][literals via ctx trees][dist][dist extras]
  #
  # Then: [ICC=63][flush literals via ctx trees]
  #
  # Returns a flat list of 0/1 integers.

  defp encode_commands(commands, flush_literals, icc_code_table, dist_code_table, lit_code_tables) do
    {bit_list, history} =
      encode_cmd_loop(commands, icc_code_table, dist_code_table, lit_code_tables, [], [])

    # Encode flush literals after the sentinel.
    p1_flush = if history == [], do: -1, else: List.last(history)
    flush_bits =
      Enum.reduce(flush_literals, {[], p1_flush}, fn byte, {bits_acc, p1} ->
        ctx = literal_context(p1)
        code_table = Enum.at(lit_code_tables, ctx)
        sym_bits = encode_symbol(code_table, byte)
        {bits_acc ++ sym_bits, byte}
      end)
      |> elem(0)

    bit_list ++ flush_bits
  end

  defp encode_cmd_loop([], _icc_ct, _dist_ct, _lit_cts, bits_acc, history) do
    {bits_acc, history}
  end

  defp encode_cmd_loop([{insert_len, copy_len, copy_dist, literals} | rest],
                       icc_ct, dist_ct, lit_cts, bits_acc, history) do
    if copy_len == 0 do
      # Sentinel.
      sentinel_bits = encode_symbol(icc_ct, 63)
      {bits_acc ++ sentinel_bits, history}
    else
      # Regular command.
      icc = find_icc_code(insert_len, copy_len)
      ins_extra_count = Map.get(@icc_insert_extra, icc, 0)
      copy_extra_count = Map.get(@icc_copy_extra, icc, 0)
      ins_extra_val = insert_len - Map.get(@icc_insert_base, icc, 0)
      copy_extra_val = copy_len - Map.get(@icc_copy_base, icc, 0)

      # 1. ICC Huffman code.
      icc_bits = encode_symbol(icc_ct, icc)
      # 2. Insert length extra bits.
      ins_xbits = extra_bits_lsb(ins_extra_val, ins_extra_count)
      # 3. Copy length extra bits.
      cpy_xbits = extra_bits_lsb(copy_extra_val, copy_extra_count)

      # 4. Insert literals (each via its per-context Huffman tree).
      p1_start = if history == [], do: -1, else: List.last(history)
      {lit_bits, history2} =
        Enum.reduce(literals, {[], {p1_start, history}}, fn byte, {lb, {p1, hist}} ->
          ctx = literal_context(p1)
          sym_bits = encode_symbol(Enum.at(lit_cts, ctx), byte)
          {lb ++ sym_bits, {byte, hist ++ [byte]}}
        end)
        |> then(fn {lb, {_p1, hist}} -> {lb, hist} end)

      # 5. Distance code + extra bits.
      dc = dist_code(copy_dist)
      dist_extra_count = Map.get(@dist_extra, dc, 0)
      dist_extra_val = copy_dist - Map.get(@dist_base, dc, 0)
      dist_bits = encode_symbol(dist_ct, dc)
      dist_xbits = extra_bits_lsb(dist_extra_val, dist_extra_count)

      # Simulate copy for history.
      copy_start = length(history2) - copy_dist
      history3 =
        Enum.reduce(0..(copy_len - 1), history2, fn i, hist ->
          byte = Enum.at(hist, copy_start + i)
          hist ++ [byte]
        end)

      new_bits = bits_acc ++ icc_bits ++ ins_xbits ++ cpy_xbits ++ lit_bits ++ dist_bits ++ dist_xbits
      encode_cmd_loop(rest, icc_ct, dist_ct, lit_cts, new_bits, history3)
    end
  end

  # ---------------------------------------------------------------------------
  # Public API: decompress/1
  # ---------------------------------------------------------------------------

  @doc """
  Decompress CMP06 Brotli wire-format `data` and return the original binary.

  ## Algorithm

  1. Parse 10-byte header.
  2. Parse ICC, dist, and literal code-length tables.
  3. Reconstruct canonical Huffman code maps.
  4. Unpack LSB-first bit stream.
  5. Decode ICC commands until sentinel (ICC 63):
     - For each command: read insert_length literals, then perform copy.
  6. After sentinel: read remaining flush literals until `original_length` bytes.
  """
  def decompress(data) when is_binary(data) and byte_size(data) < 10, do: ""

  def decompress(<<
        original_length::32,
        icc_entry_count::8,
        dist_entry_count::8,
        ctx0_entry_count::8,
        ctx1_entry_count::8,
        ctx2_entry_count::8,
        ctx3_entry_count::8,
        rest::binary
      >>) do
    if original_length == 0 do
      ""
    else
      {icc_lengths, rest2} = parse_table_1byte(rest, icc_entry_count)
      {dist_lengths, rest3} = parse_table_1byte(rest2, dist_entry_count)
      {ctx0_lengths, rest4} = parse_table_2byte(rest3, ctx0_entry_count)
      {ctx1_lengths, rest5} = parse_table_2byte(rest4, ctx1_entry_count)
      {ctx2_lengths, rest6} = parse_table_2byte(rest5, ctx2_entry_count)
      {ctx3_lengths, bit_stream} = parse_table_2byte(rest6, ctx3_entry_count)

      icc_rev_map = reconstruct_canonical_codes(icc_lengths)
      dist_rev_map = reconstruct_canonical_codes(dist_lengths)
      lit_rev_maps = [
        reconstruct_canonical_codes(ctx0_lengths),
        reconstruct_canonical_codes(ctx1_lengths),
        reconstruct_canonical_codes(ctx2_lengths),
        reconstruct_canonical_codes(ctx3_lengths)
      ]

      bits = unpack_bits_lsb_first(bit_stream)

      {output, bit_pos} =
        decode_loop(bits, 0, icc_rev_map, dist_rev_map, lit_rev_maps, [], -1)

      # After sentinel: read flush literals until original_length bytes produced.
      p1 = if output == [], do: -1, else: List.last(output)
      output2 =
        decode_flush_literals(bits, bit_pos, lit_rev_maps, output, p1, original_length)

      :binary.list_to_bin(output2)
    end
  end

  defp parse_table_1byte(data, 0), do: {[], data}
  defp parse_table_1byte(<<sym::8, code_len::8, rest::binary>>, n) do
    {tail, rest2} = parse_table_1byte(rest, n - 1)
    {[{sym, code_len} | tail], rest2}
  end

  defp parse_table_2byte(data, 0), do: {[], data}
  defp parse_table_2byte(<<sym::16, code_len::8, rest::binary>>, n) do
    {tail, rest2} = parse_table_2byte(rest, n - 1)
    {[{sym, code_len} | tail], rest2}
  end

  # ---------------------------------------------------------------------------
  # decode_loop/7 — ICC command decode loop
  # ---------------------------------------------------------------------------
  #
  # Reads ICC symbols repeatedly. For each command:
  # 1. Read insert_length + copy_length from extras.
  # 2. Read insert_length literals via per-context trees.
  # 3. If copy_length > 0: read distance, copy bytes.
  # 4. If ICC == 63: stop (sentinel).
  #
  # Returns {output, bit_pos} where bit_pos is after the sentinel bit.
  # The flush phase then uses bit_pos to continue reading.

  defp decode_loop(bits, bit_pos, icc_rev_map, dist_rev_map, lit_rev_maps, output, p1) do
    {icc, bit_pos2} = next_huffman_symbol(bits, bit_pos, icc_rev_map)

    if icc == 63 do
      # Sentinel — return current state; flush phase continues from here.
      {output, bit_pos2}
    else
      ins_extra_count = Map.get(@icc_insert_extra, icc, 0)
      copy_extra_count = Map.get(@icc_copy_extra, icc, 0)

      {ins_extra_val, bit_pos3} = read_bits(bits, bit_pos2, ins_extra_count)
      insert_length = Map.get(@icc_insert_base, icc, 0) + ins_extra_val

      {copy_extra_val, bit_pos4} = read_bits(bits, bit_pos3, copy_extra_count)
      copy_length = Map.get(@icc_copy_base, icc, 0) + copy_extra_val

      # Read insert_length literal bytes.
      {output2, bit_pos5, p1_2} =
        if insert_length == 0 do
          {output, bit_pos4, p1}
        else
          Enum.reduce(1..insert_length, {output, bit_pos4, p1}, fn _i, {out, bpos, prev} ->
            ctx = literal_context(prev)
            lit_rev_map = Enum.at(lit_rev_maps, ctx)
            {byte, bpos2} = next_huffman_symbol(bits, bpos, lit_rev_map)
            {out ++ [byte], bpos2, byte}
          end)
        end

      # Perform copy.
      {output3, bit_pos6, p1_3} =
        if copy_length > 0 do
          {dc, bpos3} = next_huffman_symbol(bits, bit_pos5, dist_rev_map)
          dist_extra_count = Map.get(@dist_extra, dc, 0)
          {dist_extra_val, bpos4} = read_bits(bits, bpos3, dist_extra_count)
          copy_distance = Map.get(@dist_base, dc, 0) + dist_extra_val

          copy_start = length(output2) - copy_distance
          {output_after_copy, last_byte} =
            Enum.reduce(0..(copy_length - 1), {output2, p1_2}, fn i, {out, _prev} ->
              byte = Enum.at(out, copy_start + i)
              {out ++ [byte], byte}
            end)

          {output_after_copy, bpos4, last_byte}
        else
          {output2, bit_pos5, p1_2}
        end

      decode_loop(bits, bit_pos6, icc_rev_map, dist_rev_map, lit_rev_maps, output3, p1_3)
    end
  end

  # Read flush literals until output reaches original_length.
  defp decode_flush_literals(_bits, _bit_pos, _lit_rev_maps, output, _p1, target)
       when length(output) >= target do
    output
  end

  defp decode_flush_literals(bits, bit_pos, lit_rev_maps, output, p1, target) do
    ctx = literal_context(p1)
    lit_rev_map = Enum.at(lit_rev_maps, ctx)
    {byte, bit_pos2} = next_huffman_symbol(bits, bit_pos, lit_rev_map)
    decode_flush_literals(bits, bit_pos2, lit_rev_maps, output ++ [byte], byte, target)
  end
end
