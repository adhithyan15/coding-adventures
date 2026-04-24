defmodule CodingAdventures.Zip do
  import Bitwise

  @moduledoc """
  ZIP archive format (PKZIP 1989) — CMP09.

  Creates and reads `.zip` files byte-compatible with standard tools
  (macOS Archive Utility, Info-ZIP, Python's `zipfile`, etc.). Each entry
  is compressed with RFC 1951 DEFLATE (method 8) or stored verbatim
  (method 0) if compression does not help.

  ## Where it fits

      CMP02 (LZSS,    1982) — LZ77 + flag bits       ← dependency
      CMP05 (DEFLATE, 1996) — LZ77 + Huffman         ← inlined here
      CMP09 (ZIP,     1989) — DEFLATE container      ← this module

  ## Quick usage

      iex> archive = CodingAdventures.Zip.zip([{"hello.txt", "Hello, ZIP!"}])
      iex> %{"hello.txt" => data} = CodingAdventures.Zip.unzip(archive)
      iex> data
      "Hello, ZIP!"

  ## Architecture

  ZIP has a dual-header design:

  1. **Local File Headers** — one per entry, immediately before compressed data.
  2. **Central Directory** — at the end, lists all entries with offsets.
  3. **End of Central Directory (EOCD)** — the fixed-size trailer the reader
     finds first (searches from the end), then uses to locate the CD.

  This means a ZIP reader always starts at the back (`EOCD_SIG`), reads the
  CD, and random-accesses only the entries it needs. We never need to scan
  from the front.

  ## DEFLATE inlined (RFC 1951, fixed Huffman BTYPE=01)

  The existing `CodingAdventures.Deflate` package uses a custom wire format
  that does NOT conform to RFC 1951. ZIP embeds raw RFC 1951 bit streams, so
  we inline our own encoder and decoder here.

  Fixed Huffman tree (RFC 1951 §3.2.6):

      Literal/Length codes:
        0-143   → 8-bit codes  0x30-0xBF
        144-255 → 9-bit codes  0x190-0x1FF
        256-279 → 7-bit codes  0x00-0x17
        280-287 → 8-bit codes  0xC0-0xC7

      Distance codes 0-29: always 5 bits.
  """

  # ─── Wire constants ──────────────────────────────────────────────────────────

  @local_sig  0x04034B50
  @cd_sig     0x02014B50
  @eocd_sig   0x06054B50

  # UTF-8 general-purpose bit flag
  @flags      0x0800

  # Minimum version needed to extract: 2.0 for DEFLATE, 1.0 for Stored
  @ver_deflate 20
  @ver_stored  10

  # Version made by: Unix (0x03) × 256 + spec version 30 (3.0)
  @ver_made_by 0x031E

  # MS-DOS epoch: 1980-01-01 00:00:00
  @dos_epoch   0x00210000

  # Maximum decompressed output (zip-bomb guard): 256 MiB
  @max_output  268_435_456

  # ─── Public API ──────────────────────────────────────────────────────────────

  @doc "MS-DOS timestamp for the given date/time components."
  def dos_datetime(year, month, day, hour \\ 0, minute \\ 0, second \\ 0) do
    date = (year - 1980) <<< 9 ||| month <<< 5 ||| day
    time = hour <<< 11 ||| minute <<< 5 ||| div(second, 2)
    time ||| date <<< 16
  end

  @doc "MS-DOS epoch constant (1980-01-01 00:00:00)."
  def dos_epoch, do: @dos_epoch

  @doc """
  CRC-32 of `data` (polynomial 0xEDB88320, LSB-first).

  ## Examples

      iex> CodingAdventures.Zip.crc32("hello world")
      0x0D4A1185
  """
  def crc32(data, initial \\ 0) when is_binary(data) do
    table = crc32_table()
    bytes = :binary.bin_to_list(data)
    crc = bnot(initial) &&& 0xFFFFFFFF

    Enum.reduce(bytes, crc, fn byte, acc ->
      idx = bxor(acc, byte) &&& 0xFF
      bxor(acc >>> 8, elem(table, idx))
    end)
    |> then(fn c -> bnot(c) &&& 0xFFFFFFFF end)
  end

  @doc """
  One-shot compress: returns a ZIP archive binary.

  `entries` is a list of `{name, data}` or `{name, data, compress: bool}` tuples.
  """
  def zip(entries, opts \\ []) do
    compress = Keyword.get(opts, :compress, true)
    w = new_writer()
    w = Enum.reduce(entries, w, fn entry, acc ->
      case entry do
        {name, data} -> add_file(acc, name, data, compress: compress)
        {name, data, entry_opts} -> add_file(acc, name, data, entry_opts)
      end
    end)
    finish(w)
  end

  @doc "One-shot decompress: returns a map of `name => data`."
  def unzip(data) when is_binary(data) do
    reader = new_reader(data)
    entries = reader_entries(reader)
    Enum.reduce(entries, %{}, fn entry, acc ->
      if Map.has_key?(acc, entry.name) do
        raise "zip: duplicate entry name '#{entry.name}'"
      end
      Map.put(acc, entry.name, reader_read(reader, entry))
    end)
  end

  # ─── ZipWriter (pure-function style) ─────────────────────────────────────────
  #
  # We use plain maps instead of structs so the module stays self-contained.
  # A writer is %{entries: [...], buffer: iodata}.

  @doc "Create a new ZipWriter."
  def new_writer, do: %{entries: [], buffer: []}

  @doc "Add a file entry to the writer."
  def add_file(writer, name, data, opts \\ []) when is_binary(name) and is_binary(data) do
    compress = Keyword.get(opts, :compress, true)
    add_entry(writer, name, data, compress)
  end

  @doc "Add a directory entry (name should end with `/`)."
  def add_directory(writer, name) when is_binary(name) do
    add_entry(writer, name, "", false)
  end

  @doc "Finish writing; returns the complete archive as a binary."
  def finish(writer) do
    %{entries: entries, buffer: buf} = writer
    local_data = IO.iodata_to_binary(buf)
    cd_offset = byte_size(local_data)

    cd_records =
      Enum.map(entries, fn e ->
        build_cd_record(e)
      end)

    cd_data = IO.iodata_to_binary(cd_records)
    cd_size = byte_size(cd_data)
    entry_count = length(entries)

    eocd = build_eocd(entry_count, cd_size, cd_offset)

    local_data <> cd_data <> eocd
  end

  # ─── ZipReader (pure-function style) ─────────────────────────────────────────

  @doc "Create a new ZipReader for the given binary."
  def new_reader(data) when is_binary(data) do
    %{data: data, entries: parse_entries(data)}
  end

  @doc "Return list of all ZipEntry maps in the archive."
  def reader_entries(reader), do: reader.entries

  @doc "Decompress and CRC-validate one entry."
  def reader_read(reader, entry) do
    %{data: data} = reader
    local_offset = entry.local_offset

    # Parse local file header to find data start
    if local_offset + 30 > byte_size(data) do
      raise "zip: local header out of bounds for '#{entry.name}'"
    end

    lh_sig = read_le32(data, local_offset)
    unless lh_sig == @local_sig do
      raise "zip: bad local header signature at offset #{local_offset}"
    end

    lh_name_len  = read_le16(data, local_offset + 26)
    lh_extra_len = read_le16(data, local_offset + 28)

    if is_nil(lh_name_len) or is_nil(lh_extra_len) do
      raise "zip: local header fields out of bounds for '#{entry.name}'"
    end

    data_start = local_offset + 30 + lh_name_len + lh_extra_len

    if data_start + entry.compressed_size > byte_size(data) do
      raise "zip: compressed data out of bounds for '#{entry.name}'"
    end

    compressed = binary_part(data, data_start, entry.compressed_size)

    decompressed =
      case entry.method do
        0 -> compressed
        8 -> deflate_decompress(compressed, entry.size)
        m -> raise "zip: unsupported compression method #{m}"
      end

    actual_crc = crc32(decompressed)
    unless actual_crc == entry.crc32 do
      raise "zip: CRC-32 mismatch for '#{entry.name}' (expected #{entry.crc32}, got #{actual_crc})"
    end

    decompressed
  end

  @doc "Read an entry by name; raises if not found."
  def read_by_name(reader, name) do
    entry = Enum.find(reader_entries(reader), fn e -> e.name == name end)
    unless entry, do: raise("zip: entry '#{name}' not found")
    reader_read(reader, entry)
  end

  # ─── Writer internals ────────────────────────────────────────────────────────

  defp add_entry(writer, name, data, compress) do
    {method, compressed, crc} = compress_entry(data, compress)
    datetime = @dos_epoch
    version  = if method == 8, do: @ver_deflate, else: @ver_stored
    name_bin = :unicode.characters_to_binary(name, :utf8)
    name_len = byte_size(name_bin)

    local_offset = writer.buffer |> IO.iodata_length()

    local_header = [
      <<@local_sig   :: little-32>>,
      <<version      :: little-16>>,
      <<@flags       :: little-16>>,
      <<method       :: little-16>>,
      <<datetime     :: little-32>>,
      <<crc          :: little-32>>,
      <<byte_size(compressed) :: little-32>>,
      <<byte_size(data)       :: little-32>>,
      <<name_len     :: little-16>>,
      <<0            :: little-16>>,   # extra field length
      name_bin,
      compressed
    ]

    entry = %{
      name: name,
      method: method,
      crc32: crc,
      compressed_size: byte_size(compressed),
      size: byte_size(data),
      datetime: datetime,
      version: version,
      local_offset: local_offset
    }

    %{writer |
      entries: writer.entries ++ [entry],
      buffer: [writer.buffer | local_header]
    }
  end

  defp compress_entry(data, false), do: {0, data, crc32(data)}
  defp compress_entry(data, true) do
    crc = crc32(data)
    compressed = deflate_compress(data)
    if byte_size(compressed) < byte_size(data) do
      {8, compressed, crc}
    else
      {0, data, crc}
    end
  end

  defp build_cd_record(e) do
    name_bin = :unicode.characters_to_binary(e.name, :utf8)
    name_len = byte_size(name_bin)
    [
      <<@cd_sig          :: little-32>>,
      <<@ver_made_by     :: little-16>>,
      <<e.version        :: little-16>>,
      <<@flags           :: little-16>>,
      <<e.method         :: little-16>>,
      <<e.datetime       :: little-32>>,
      <<e.crc32          :: little-32>>,
      <<e.compressed_size :: little-32>>,
      <<e.size           :: little-32>>,
      <<name_len         :: little-16>>,
      <<0                :: little-16>>,  # extra field length
      <<0                :: little-16>>,  # comment length
      <<0                :: little-16>>,  # disk start
      <<0                :: little-16>>,  # internal attributes
      <<0                :: little-32>>,  # external attributes
      <<e.local_offset   :: little-32>>,
      name_bin
    ]
  end

  defp build_eocd(entry_count, cd_size, cd_offset) do
    <<
      @eocd_sig   :: little-32,
      0           :: little-16,   # disk number
      0           :: little-16,   # disk with CD start
      entry_count :: little-16,   # entries on this disk
      entry_count :: little-16,   # total entries
      cd_size     :: little-32,
      cd_offset   :: little-32,
      0           :: little-16    # comment length
    >>
  end

  # ─── Reader internals ────────────────────────────────────────────────────────

  defp parse_entries(data) do
    size = byte_size(data)

    # Search for EOCD from end, allowing up to 65535-byte comment
    eocd_offset = find_eocd(data, size)

    unless eocd_offset do
      raise "zip: EOCD signature not found"
    end

    unless eocd_offset + 22 <= size do
      raise "zip: EOCD truncated"
    end

    entry_count = read_le16(data, eocd_offset + 8)
    cd_size     = read_le32(data, eocd_offset + 12)
    cd_offset   = read_le32(data, eocd_offset + 16)

    if entry_count > 65535 do
      raise "zip: entry count #{entry_count} exceeds ZIP limit of 65535"
    end

    # CD must precede the EOCD and fit within the archive.
    if cd_offset > eocd_offset do
      raise "zip: CD offset #{cd_offset} overlaps EOCD at #{eocd_offset}"
    end
    if cd_offset + cd_size > eocd_offset do
      raise "zip: CD region extends into EOCD"
    end
    if cd_offset + cd_size > size do
      raise "zip: CD region extends beyond archive"
    end

    read_cd_entries(data, cd_offset, cd_size, entry_count)
  end

  # Scan backward from (size - 22) to find the EOCD signature.
  defp find_eocd(data, size) do
    start = max(0, size - 22 - 65535)
    find_eocd_loop(data, size - 22, start)
  end

  defp find_eocd_loop(_data, pos, _start) when pos < 0, do: nil

  defp find_eocd_loop(data, pos, start) do
    sig = read_le32(data, pos)
    cond do
      sig == @eocd_sig -> pos
      pos <= start     -> nil
      true             -> find_eocd_loop(data, pos - 1, start)
    end
  end

  defp read_cd_entries(data, cd_offset, cd_size, entry_count) do
    read_cd_loop(data, cd_offset, cd_offset, cd_size, entry_count, [])
  end

  defp read_cd_loop(_data, _pos, _cd_base, _cd_size, 0, acc) do
    Enum.reverse(acc)
  end

  defp read_cd_loop(data, pos, cd_base, cd_size, remaining, acc) do
    data_size = byte_size(data)

    if pos + 46 > data_size do
      raise "zip: CD entry header out of bounds at pos #{pos}"
    end

    sig = read_le32(data, pos)
    unless sig == @cd_sig do
      raise "zip: expected CD signature at #{pos}, got #{sig}"
    end

    method           = read_le16(data, pos + 10)
    crc32v           = read_le32(data, pos + 16)
    compressed_size  = read_le32(data, pos + 20)
    size             = read_le32(data, pos + 24)
    name_len         = read_le16(data, pos + 28)
    extra_len        = read_le16(data, pos + 30)
    comment_len      = read_le16(data, pos + 32)
    local_offset     = read_le32(data, pos + 42)

    if Enum.any?([method, crc32v, compressed_size, size, name_len, extra_len,
                  comment_len, local_offset], &is_nil/1) do
      raise "zip: CD entry fields truncated at pos #{pos}"
    end

    name_start = pos + 46

    if name_start + name_len > data_size do
      raise "zip: CD entry name extends beyond archive data at pos #{pos}"
    end

    name_bin   = binary_part(data, name_start, name_len)
    # scrub_utf8 handles the UTF-8 conversion internally — do not pre-pipe through
    # :unicode.characters_to_binary as it returns a tuple on error, not a binary,
    # which would cause a FunctionClauseError in scrub_utf8.
    name       = scrub_utf8(name_bin)
    validate_entry_name!(name)

    next_pos = pos + 46 + name_len + extra_len + comment_len

    if next_pos > cd_base + cd_size do
      raise "zip: CD entry advance out of bounds at pos #{pos}"
    end

    entry = %{
      name: name,
      method: method,
      crc32: crc32v,
      compressed_size: compressed_size,
      size: size,
      local_offset: local_offset
    }

    read_cd_loop(data, next_pos, cd_base, cd_size, remaining - 1, [entry | acc])
  end

  # Replace invalid UTF-8 sequences with the Unicode replacement character.
  # Uses :latin1 as the source encoding in the fallback so that multi-byte
  # sequences are handled at the codepoint level, not byte-by-byte.
  defp scrub_utf8(bin) when is_binary(bin) do
    case :unicode.characters_to_binary(bin, :utf8, :utf8) do
      result when is_binary(result) -> result
      _error ->
        case :unicode.characters_to_binary(bin, :latin1, :utf8) do
          result when is_binary(result) -> result
          _ -> raise "zip: entry name is not valid UTF-8 or Latin-1"
        end
    end
  end

  # Reject entry names with path traversal, absolute paths, null bytes, or backslashes.
  # Backslashes are rejected entirely — they are not valid ZIP path separators and
  # are used in Windows path traversal (e.g., `..\etc\passwd`).
  defp validate_entry_name!(name) do
    segments = String.split(name, "/")
    cond do
      String.contains?(name, "\0") ->
        raise "zip: entry name contains null byte"
      String.contains?(name, "\\") ->
        raise "zip: entry name contains backslash"
      String.starts_with?(name, "/") ->
        raise "zip: entry name is an absolute path"
      Enum.any?(segments, &(&1 == "..")) ->
        raise "zip: entry name contains path traversal (..)"
      true -> :ok
    end
  end

  # ─── DEFLATE (RFC 1951, fixed Huffman BTYPE=01) ───────────────────────────────
  #
  # We cannot reuse `CodingAdventures.Deflate` — it uses a custom 4-byte header
  # wire format, not raw RFC 1951 bit streams. ZIP requires raw RFC 1951.

  # Length code table: {symbol, base_len, extra_bits}
  @length_table [
    {257,   3, 0}, {258,   4, 0}, {259,   5, 0}, {260,   6, 0},
    {261,   7, 0}, {262,   8, 0}, {263,   9, 0}, {264,  10, 0},
    {265,  11, 1}, {266,  13, 1}, {267,  15, 1}, {268,  17, 1},
    {269,  19, 2}, {270,  23, 2}, {271,  27, 2}, {272,  31, 2},
    {273,  35, 3}, {274,  43, 3}, {275,  51, 3}, {276,  59, 3},
    {277,  67, 4}, {278,  83, 4}, {279,  99, 4}, {280, 115, 4},
    {281, 131, 5}, {282, 163, 5}, {283, 195, 5}, {284, 227, 5},
    # RFC 1951 §3.2.5: symbol 285 = length 258, 0 extra bits (special case)
    {285, 258, 0}
  ]

  # Distance code table: {code, base_dist, extra_bits}
  @dist_table [
    { 0,     1, 0}, { 1,     2, 0}, { 2,     3, 0}, { 3,     4, 0},
    { 4,     5, 1}, { 5,     7, 1}, { 6,     9, 2}, { 7,    13, 2},
    { 8,    17, 3}, { 9,    25, 3}, {10,    33, 4}, {11,    49, 4},
    {12,    65, 5}, {13,    97, 5}, {14,   129, 6}, {15,   193, 6},
    {16,   257, 7}, {17,   385, 7}, {18,   513, 8}, {19,   769, 8},
    {20,  1025, 9}, {21,  1537, 9}, {22,  2049,10}, {23,  3073,10},
    {24,  4097,11}, {25,  6145,11}, {26,  8193,12}, {27, 12289,12},
    {28, 16385,13}, {29, 24577,13}
  ]

  # Build compile-time lookup maps.
  @length_by_sym @length_table |> Enum.map(fn {s,b,e} -> {s, {b,e}} end) |> Map.new()
  @dist_by_code  @dist_table   |> Enum.map(fn {c,b,e} -> {c, {b,e}} end) |> Map.new()

  # ── Fixed Huffman encode (symbol → {code_bits, nbits}) ───────────────────────

  defp fixed_ll_encode(sym) when sym in 0..143,   do: {0x30 + sym,           8}
  defp fixed_ll_encode(sym) when sym in 144..255,  do: {0x190 + sym - 144,   9}
  defp fixed_ll_encode(sym) when sym in 256..279,  do: {sym - 256,            7}
  defp fixed_ll_encode(sym) when sym in 280..287,  do: {0xC0 + sym - 280,    8}

  # ── Fixed Huffman decode ─────────────────────────────────────────────────────
  #
  # Read bits LSB-first from the BitReader state, then reverse them to get the
  # Huffman code (Huffman is MSB-first in the canonical sense).

  defp fixed_ll_decode(br) do
    # Peek 9 bits to handle 9-bit codes.
    {bits9, br2} = br_peek(br, 9)
    bits7 = bits9 &&& 0x7F
    bits8 = bits9 &&& 0xFF

    cond do
      # 7-bit: 256-279 → reversed codes 0x00-0x17 (binary 0000000-0010111)
      reverse_bits(bits7, 7) < 24 ->
        {256 + reverse_bits(bits7, 7), br_consume(br2, 7)}

      # 8-bit: 0-143 → 0x30-0xBF (reversed: 0x0C-0x7D in 8 bits)
      reverse_bits(bits8, 8) in 0x30..0xBF ->
        {reverse_bits(bits8, 8) - 0x30, br_consume(br2, 8)}

      # 8-bit: 280-287 → 0xC0-0xC7
      reverse_bits(bits8, 8) in 0xC0..0xC7 ->
        {280 + reverse_bits(bits8, 8) - 0xC0, br_consume(br2, 8)}

      # 9-bit: 144-255 → 0x190-0x1FF
      reverse_bits(bits9, 9) in 0x190..0x1FF ->
        {144 + reverse_bits(bits9, 9) - 0x190, br_consume(br2, 9)}

      true ->
        raise "zip: invalid fixed Huffman code #{inspect(bits9)} in stream"
    end
  end

  defp fixed_dist_decode(br) do
    {bits5, br2} = br_read(br, 5)
    dist_code = reverse_bits(bits5, 5)
    {br2, dist_code}
  end

  # ── DEFLATE compress ─────────────────────────────────────────────────────────

  defp deflate_compress(<<>>) do
    # Empty input → stored block.
    # First byte: BFINAL=1 (bit 0), BTYPE=00 (bits 1-2), padding zeros (bits 3-7) = 0x01.
    # Then LEN=0 (LE16) and NLEN=0xFFFF (LE16).
    <<0x01, 0x00, 0x00, 0xFF, 0xFF>>
  end

  defp deflate_compress(data) when is_binary(data) do
    tokens = CodingAdventures.LZSS.encode(data, 32768, 255, 3)
    bw = bw_new()
    # BFINAL=1, BTYPE=01 (fixed Huffman) — 3 bits
    bw = bw_write(bw, 1, 1)
    bw = bw_write(bw, 1, 1)
    bw = bw_write(bw, 0, 1)

    bw = Enum.reduce(tokens, bw, fn token, acc ->
      case token do
        %{kind: :literal, byte: b} ->
          {code, nbits} = fixed_ll_encode(b)
          bw_write_huffman(acc, code, nbits)

        %{kind: :match, length: len, offset: off} ->
          # encode length
          {ll_sym, {base_len, len_extra_bits}} = find_length_sym(len)
          {code, nbits} = fixed_ll_encode(ll_sym)
          acc = bw_write_huffman(acc, code, nbits)
          acc = if len_extra_bits > 0, do: bw_write(acc, len - base_len, len_extra_bits), else: acc
          # encode distance
          {dist_code, {base_dist, dist_extra_bits}} = find_dist_code(off)
          acc = bw_write_huffman(acc, dist_code, 5)
          if dist_extra_bits > 0, do: bw_write(acc, off - base_dist, dist_extra_bits), else: acc
      end
    end)

    # End-of-block symbol 256
    {eob_code, eob_bits} = fixed_ll_encode(256)
    bw = bw_write_huffman(bw, eob_code, eob_bits)
    bw_finish(bw)
  end

  # Find the length symbol and {base, extra_bits} for a given match length.
  defp find_length_sym(len) do
    Enum.reduce_while(@length_table, nil, fn {sym, base, extra}, _acc ->
      max_len = base + (1 <<< extra) - 1
      if len >= base and len <= max_len do
        {:halt, {sym, {base, extra}}}
      else
        {:cont, nil}
      end
    end)
    |> then(fn
      nil -> raise "zip: match length #{len} out of range"
      result -> result
    end)
  end

  # Find the distance code and {base, extra_bits} for a given offset.
  defp find_dist_code(off) do
    Enum.reduce_while(@dist_table, nil, fn {code, base, extra}, _acc ->
      max_dist = base + (1 <<< extra) - 1
      if off >= base and off <= max_dist do
        {:halt, {code, {base, extra}}}
      else
        {:cont, nil}
      end
    end)
    |> then(fn
      nil -> raise "zip: distance #{off} out of range"
      result -> result
    end)
  end

  # ── DEFLATE decompress ───────────────────────────────────────────────────────

  defp deflate_decompress(data, expected_size) when is_binary(data) do
    br = br_new(data)
    decompress_blocks(br, [], 0, expected_size)
  end

  # Track `total` as a running integer to avoid O(n²) Enum.sum on every block.
  defp decompress_blocks(br, acc, total, exp) do
    {bfinal, br} = br_read(br, 1)
    {btype,  br} = br_read(br, 2)

    # Pass remaining budget so the per-token guard inside decompress_fixed
    # accounts for bytes already produced by previous blocks.
    budget = @max_output - total

    {chunk, br} =
      case btype do
        0 -> decompress_stored(br)
        1 -> decompress_fixed(br, budget)
        _ -> raise "zip: unsupported DEFLATE BTYPE #{btype} (only stored=0 and fixed=1 supported)"
      end

    new_total = total + byte_size(chunk)

    if new_total > @max_output do
      raise "zip: decompressed output exceeds #{@max_output} bytes (zip bomb guard)"
    end

    if bfinal == 1 do
      IO.iodata_to_binary([acc | [chunk]])
    else
      decompress_blocks(br, [acc | [chunk]], new_total, exp)
    end
  end

  defp decompress_stored(br) do
    # Align to byte boundary, then read LEN and NLEN (RFC 1951 §3.2.4).
    br = br_align(br)
    {len_bin,  br} = br_read_bytes(br, 2)
    {nlen_bin, br} = br_read_bytes(br, 2)
    len_val  = :binary.decode_unsigned(len_bin,  :little)
    nlen_val = :binary.decode_unsigned(nlen_bin, :little)
    unless bxor(len_val, nlen_val) == 0xFFFF do
      raise "zip: stored block LEN/NLEN complement check failed"
    end
    {chunk, br} = br_read_raw(br, len_val)
    {chunk, br}
  end

  defp decompress_fixed(br, budget) do
    decompress_fixed_loop(br, [], budget)
  end

  # `budget` = remaining bytes allowed before hitting @max_output.
  # Passed from decompress_blocks so it accounts for ALL previous blocks,
  # preventing a multi-block zip-bomb bypass.
  defp decompress_fixed_loop(br, acc, budget) do
    {sym, br} = fixed_ll_decode(br)

    cond do
      sym == 256 ->
        {IO.iodata_to_binary(acc), br}

      sym < 256 ->
        if budget < 1 do
          raise "zip: decompressed output exceeds #{@max_output} bytes (zip bomb guard)"
        end
        decompress_fixed_loop(br, [acc | [<<sym>>]], budget - 1)

      sym in 257..285 ->
        {base_len, extra_len_bits} = Map.fetch!(@length_by_sym, sym)
        {extra_len, br} = if extra_len_bits > 0, do: br_read(br, extra_len_bits), else: {0, br}
        length = base_len + extra_len

        {br, dist_code} = fixed_dist_decode(br)
        unless Map.has_key?(@dist_by_code, dist_code) do
          raise "zip: reserved or invalid distance code #{dist_code}"
        end
        {base_dist, extra_dist_bits} = Map.fetch!(@dist_by_code, dist_code)
        {extra_dist, br} = if extra_dist_bits > 0, do: br_read(br, extra_dist_bits), else: {0, br}
        distance = base_dist + extra_dist

        current = IO.iodata_to_binary(acc)
        cur_len = byte_size(current)

        if distance > cur_len do
          raise "zip: back-reference distance #{distance} > current output #{cur_len}"
        end

        if length > budget do
          raise "zip: decompressed output exceeds #{@max_output} bytes (zip bomb guard)"
        end

        start = cur_len - distance
        chunk = copy_with_overlap(current, start, length)
        decompress_fixed_loop(br, [current | [chunk]], budget - length)

      true ->
        raise "zip: invalid LL symbol #{sym}"
    end
  end

  # Copy `length` bytes from `buf` starting at `start`, allowing overlap.
  #
  # RFC 1951 allows the copy to reference bytes not yet in the source buffer
  # (run-length expansion, e.g. distance=1 length=10 repeats one byte 10×).
  # We handle this without growing `buf` by using modular index arithmetic:
  # once we advance past the end of the original buffer, the byte at position
  # `pos` is the same as the byte at `(buf_size - distance) + rem(pos - buf_size, distance)`,
  # i.e. we repeat the window of `distance` bytes starting at `start`.
  defp copy_with_overlap(buf, start, length) do
    buf_size = byte_size(buf)
    distance = buf_size - start
    do_copy_no_grow(buf, start, buf_size, distance, length, [])
  end

  defp do_copy_no_grow(_buf, _pos, _buf_size, _dist, 0, acc) do
    acc |> Enum.reverse() |> IO.iodata_to_binary()
  end

  defp do_copy_no_grow(buf, pos, buf_size, dist, remaining, acc) do
    effective = if pos < buf_size, do: pos,
                  else: (buf_size - dist) + rem(pos - buf_size, dist)
    byte = :binary.at(buf, effective)
    do_copy_no_grow(buf, pos + 1, buf_size, dist, remaining - 1, [<<byte>> | acc])
  end


  # ─── BitWriter ───────────────────────────────────────────────────────────────
  #
  # State: %{buf: bits accumulated as integer, nbits: count, bytes: iodata}
  # Bits are packed LSB-first into bytes.

  defp bw_new, do: %{buf: 0, nbits: 0, bytes: []}

  defp bw_write(%{buf: b, nbits: n, bytes: out} = bw, val, bits) do
    b2 = b ||| ((val &&& ((1 <<< bits) - 1)) <<< n)
    n2 = n + bits
    flush_bw(%{bw | buf: b2, nbits: n2, bytes: out})
  end

  defp flush_bw(%{buf: b, nbits: n, bytes: out} = bw) when n >= 8 do
    byte = b &&& 0xFF
    flush_bw(%{bw | buf: b >>> 8, nbits: n - 8, bytes: [out | [<<byte>>]]})
  end
  defp flush_bw(bw), do: bw

  # Write a Huffman code MSB-first (reverse the bits, then write LSB-first).
  defp bw_write_huffman(bw, code, nbits) do
    reversed = reverse_bits(code, nbits)
    bw_write(bw, reversed, nbits)
  end

  defp bw_finish(%{buf: b, nbits: n, bytes: out}) do
    if n > 0 do
      IO.iodata_to_binary([out | [<<b &&& 0xFF>>]])
    else
      IO.iodata_to_binary(out)
    end
  end

  # ─── BitReader ────────────────────────────────────────────────────────────────
  #
  # State: %{data: binary, pos: byte_pos, buf: integer, nbits: count}

  defp br_new(data), do: %{data: data, pos: 0, buf: 0, nbits: 0}

  # Read `n` bits (LSB-first).
  defp br_read(br, 0), do: {0, br}
  defp br_read(br, n) do
    br = refill_br(br, n)
    %{buf: b, nbits: nb} = br
    val = b &&& ((1 <<< n) - 1)
    {val, %{br | buf: b >>> n, nbits: nb - n}}
  end

  # Peek `n` bits without consuming.
  defp br_peek(br, n) do
    br2 = refill_br(br, n)
    %{buf: b} = br2
    {b &&& ((1 <<< n) - 1), br2}
  end

  defp br_consume(%{buf: b, nbits: nb} = br, n) do
    %{br | buf: b >>> n, nbits: nb - n}
  end

  defp refill_br(%{data: data, pos: pos, buf: b, nbits: nb} = br, n) when nb < n do
    if pos >= byte_size(data) do
      raise "zip: unexpected end of DEFLATE stream (need #{n} bits, have #{nb})"
    end
    byte = :binary.at(data, pos)
    refill_br(%{br | pos: pos + 1, buf: b ||| (byte <<< nb), nbits: nb + 8}, n)
  end
  defp refill_br(br, _n), do: br

  # Align to next byte boundary (discard partial byte).
  defp br_align(%{nbits: nb} = br) do
    skip = rem(nb, 8)
    if skip > 0, do: br_consume(br, skip), else: br
  end

  # Read `n` bytes as a binary.
  defp br_read_bytes(br, n) do
    br = br_align(br)
    %{data: data, pos: pos} = br
    if pos + n > byte_size(data) do
      raise "zip: DEFLATE stream truncated (need #{n} bytes at pos #{pos})"
    end
    chunk = binary_part(data, pos, n)
    {chunk, %{br | pos: pos + n}}
  end

  # Read exactly `n` bytes from the aligned stream.
  defp br_read_raw(br, n) do
    {chunk, br} = br_read_bytes(br, n)
    {chunk, br}
  end

  # ─── Bit reversal ────────────────────────────────────────────────────────────
  #
  # Huffman codes in RFC 1951 are MSB-first but the bit stream is LSB-first.
  # When writing: reverse before inserting LSB-first.
  # When reading: peek bits LSB-first, then reverse to get canonical code.

  defp reverse_bits(val, nbits) do
    do_reverse(val, nbits, 0)
  end

  defp do_reverse(_val, 0, acc), do: acc
  defp do_reverse(val, n, acc) do
    do_reverse(val >>> 1, n - 1, (acc <<< 1) ||| (val &&& 1))
  end

  # ─── Little-endian helpers ───────────────────────────────────────────────────

  defp read_le16(data, offset) do
    case data do
      <<_::binary-size(offset), lo, hi, _::binary>> -> lo ||| hi <<< 8
      _ -> nil
    end
  end

  defp read_le32(data, offset) do
    case data do
      <<_::binary-size(offset), b0, b1, b2, b3, _::binary>> ->
        b0 ||| b1 <<< 8 ||| b2 <<< 16 ||| b3 <<< 24
      _ -> nil
    end
  end

  # ─── CRC-32 table (compile-time) ─────────────────────────────────────────────

  defp crc32_table do
    entries =
      Enum.map(0..255, fn i ->
        crc32_table_entry(i, 0, 0xEDB88320)
      end)
    List.to_tuple(entries)
  end

  defp crc32_table_entry(crc, 8, _poly), do: crc

  defp crc32_table_entry(crc, k, poly) do
    if (crc &&& 1) == 1 do
      crc32_table_entry(bxor(crc >>> 1, poly), k + 1, poly)
    else
      crc32_table_entry(crc >>> 1, k + 1, poly)
    end
  end
end
