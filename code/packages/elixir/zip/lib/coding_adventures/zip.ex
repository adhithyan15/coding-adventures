defmodule CodingAdventures.Zip.RawInflateError do
  @moduledoc false
  defexception [:code]

  @impl true
  def message(%__MODULE__{code: code}) do
    CodingAdventures.Zip.Messages.inflate_message(code)
  end
end

defmodule CodingAdventures.Zip.RawInflateResult do
  @moduledoc false
  @enforce_keys [:output, :bytes_consumed]
  defstruct [:output, :bytes_consumed]
end

defmodule CodingAdventures.Zip do
  @moduledoc CodingAdventures.Zip.Messages.zip_moduledoc()
  import Bitwise

  alias CodingAdventures.Zip.Constants
  alias CodingAdventures.Zip.Messages
  alias CodingAdventures.Zip.RawInflateError
  alias CodingAdventures.Zip.RawInflateResult

  # ─── Compile-Time Attributes Bound from Constants ────────────────────────────

  @btype_stored Constants.btype_stored()
  @btype_fixed Constants.btype_fixed()
  @btype_dynamic Constants.btype_dynamic()
  @btype_reserved Constants.btype_reserved()

  @method_stored Constants.method_stored()
  @method_deflate Constants.method_deflate()

  @page_size Constants.page_size()
  @chunk_buffer_size Constants.chunk_buffer_size()
  @stored_header_size Constants.stored_header_size()
  @max_pending Constants.max_pending()

  @fast_table_bits Constants.fast_table_bits()
  @fast_table_mask Constants.fast_table_mask()

  @bits_64 Constants.bits_64()
  @bits_8 Constants.bits_8()

  @mask_64 Constants.mask_64()
  @mask_32 Constants.mask_32()
  @mask_16 Constants.mask_16()
  @mask_8 Constants.mask_8()

  @min_match_len Constants.min_match_len()
  @max_match_len Constants.max_match_len()
  @max_distance Constants.max_distance()
  @hash_table_size Constants.hash_table_size()
  @hash_mask Constants.hash_mask()
  @eob_symbol Constants.eob_symbol()
  @max_ll_codes Constants.max_ll_codes()

  @dos_base_year Constants.dos_base_year()
  @dos_year_shift Constants.dos_year_shift()
  @dos_month_shift Constants.dos_month_shift()
  @dos_day_shift Constants.dos_day_shift()
  @dos_hour_shift Constants.dos_hour_shift()
  @dos_minute_shift Constants.dos_minute_shift()
  @dos_second_div Constants.dos_second_div()

  # ─── Compile-Time Precomputed Lookup Tables ──────────────────────────────────

  @crc32_table (for i <- 0..255 do
                  Enum.reduce(0..7, i, fn _, acc ->
                    if (acc &&& 1) == 1 do
                      bxor(Constants.crc32_poly(), acc >>> 1)
                    else
                      acc >>> 1
                    end
                  end)
                end)
               |> List.to_tuple()

  @fixed_ll_encode_table (for sym <- 0..287 do
                            {code, bits} =
                              cond do
                                sym in 0..143 -> {48 + sym, 8}
                                sym in 144..255 -> {400 + (sym - 144), 9}
                                sym in 256..279 -> {sym - 256, 7}
                                sym in 280..287 -> {192 + (sym - 280), 8}
                              end

                            rev =
                              Enum.reduce(0..(bits - 1)//1, 0, fn i, acc ->
                                bit = code >>> i &&& 1
                                acc ||| bit <<< (bits - 1 - i)
                              end)

                            {rev, bits}
                          end)
                         |> List.to_tuple()

  @dist_encode_table (for code <- 0..31 do
                        rev =
                          Enum.reduce(0..4, 0, fn i, acc ->
                            bit = code >>> i &&& 1
                            acc ||| bit <<< (4 - i)
                          end)

                        {rev, 5}
                      end)
                     |> List.to_tuple()

  @length_table [
    {257, 3, 0},
    {258, 4, 0},
    {259, 5, 0},
    {260, 6, 0},
    {261, 7, 0},
    {262, 8, 0},
    {263, 9, 0},
    {264, 10, 0},
    {265, 11, 1},
    {266, 13, 1},
    {267, 15, 1},
    {268, 17, 1},
    {269, 19, 2},
    {270, 23, 2},
    {271, 27, 2},
    {272, 31, 2},
    {273, 35, 3},
    {274, 43, 3},
    {275, 51, 3},
    {276, 59, 3},
    {277, 67, 4},
    {278, 83, 4},
    {279, 99, 4},
    {280, 115, 4},
    {281, 131, 5},
    {282, 163, 5},
    {283, 195, 5},
    {284, 227, 5},
    {285, 258, 0}
  ]

  @length_lookup_table (for len <- 0..258 do
                          match =
                            Enum.find(@length_table, fn {_sym, base, extra} ->
                              max_len = base + (1 <<< extra) - 1
                              len >= base and len <= max_len
                            end)

                          case match do
                            {sym, base, extra} -> {sym, base, extra}
                            nil -> {0, 0, 0}
                          end
                        end)
                       |> List.to_tuple()

  @fixed_ll_decoder Constants.fixed_ll_decoder()
  @fixed_dist_decoder Constants.fixed_dist_decoder()

  # ─── Public API ──────────────────────────────────────────────────────────────

  def dos_datetime(year, month, day, hour \\ 0, minute \\ 0, second \\ 0) do
    date =
      (year - @dos_base_year) <<< @dos_year_shift ||| month <<< @dos_month_shift |||
        day <<< @dos_day_shift

    time =
      hour <<< @dos_hour_shift ||| minute <<< @dos_minute_shift |||
        div(second, @dos_second_div)

    time ||| date
  end

  def dos_epoch, do: Constants.dos_epoch()
  def raw_inflate_max_output, do: Constants.raw_inflate_max_output()
  def raw_inflate_error_codes, do: Messages.raw_inflate_error_codes()

  def crc32(data, initial \\ 0) when is_binary(data) do
    crc = bnot(initial) &&& @mask_32
    do_crc32(data, crc)
  end

  defp do_crc32(<<>>, crc), do: bxor(crc, @mask_32)

  defp do_crc32(<<b, rest::binary>>, crc) do
    idx = bxor(crc &&& Constants.mask_8(), b)
    next_crc = bxor(crc >>> 8, elem(@crc32_table, idx))
    do_crc32(rest, next_crc)
  end

  def new_writer do
    %{
      entries: [],
      buffer: []
    }
  end

  def add_file(writer, name, data, opts \\ []) when is_binary(name) and is_binary(data) do
    compress = Keyword.get(opts, :compress, true)
    add_entry(writer, name, data, compress)
  end

  def add_directory(writer, name) when is_binary(name) do
    add_entry(writer, name, <<>>, false)
  end

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

    IO.iodata_to_binary([local_data, cd_data, eocd])
  end

  def new_reader(data) when is_binary(data) do
    %{
      data: data,
      entries: parse_archive(data)
    }
  end

  def reader_entries(reader), do: reader.entries

  def reader_read(reader, entry) do
    data = reader.data
    local_offset = entry.local_offset

    if local_offset + Constants.local_header_fixed_size() > byte_size(data) do
      raise Messages.err_local_header_out_of_bounds()
    end

    lh_sig = read_le32(data, local_offset)

    unless lh_sig == Constants.local_sig() do
      raise Messages.err_bad_local_header_sig()
    end

    lh_name_len = read_le16(data, local_offset + Constants.local_offset_name_len())
    lh_extra_len = read_le16(data, local_offset + Constants.local_offset_extra_len())

    if is_nil(lh_name_len) or is_nil(lh_extra_len) do
      raise Messages.err_local_header_fields_out_of_bounds()
    end

    data_start = local_offset + Constants.local_header_fixed_size() + lh_name_len + lh_extra_len

    if data_start + entry.compressed_size > byte_size(data) do
      raise Messages.err_compressed_data_out_of_bounds()
    end

    compressed = binary_part(data, data_start, entry.compressed_size)

    decompressed =
      case entry.method do
        @method_stored ->
          compressed

        @method_deflate ->
          result = raw_inflate_counted(compressed, entry.size)

          if result.bytes_consumed != byte_size(compressed) do
            raise Messages.err_trailing_bytes()
          end

          result.output

        _other ->
          raise Messages.err_unsupported_compression_method()
      end

    if byte_size(decompressed) != entry.size do
      raise Messages.err_uncompressed_size_mismatch()
    end

    actual_crc = crc32(decompressed)

    unless actual_crc == entry.crc32 do
      raise Messages.err_crc32_mismatch()
    end

    decompressed
  end

  def read_by_name(reader, name) do
    entry = Enum.find(reader_entries(reader), fn e -> e.name == name end)
    unless entry, do: raise(Messages.err_entry_not_found())
    reader_read(reader, entry)
  end

  # ─── Writer Internals ────────────────────────────────────────────────────────

  defp add_entry(writer, name, data, compress) do
    validate_entry_name!(name)
    {method, compressed, crc} = compress_entry(data, compress)
    datetime = Constants.dos_epoch()

    version =
      if method == @method_deflate, do: Constants.ver_deflate(), else: Constants.ver_stored()

    name_bin = :unicode.characters_to_binary(name, :utf8)
    name_len = byte_size(name_bin)

    local_offset = writer.buffer |> IO.iodata_length()

    local_header = [
      <<Constants.local_sig()::little-32>>,
      <<version::little-16>>,
      <<Constants.flags()::little-16>>,
      <<method::little-16>>,
      <<datetime::little-32>>,
      <<crc::little-32>>,
      <<byte_size(compressed)::little-32>>,
      <<byte_size(data)::little-32>>,
      <<name_len::little-16>>,
      <<0::little-16>>,
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

    %{writer | entries: writer.entries ++ [entry], buffer: [writer.buffer | local_header]}
  end

  defp compress_entry(data, false), do: {@method_stored, data, crc32(data)}

  defp compress_entry(data, true) do
    crc = crc32(data)
    compressed = raw_deflate(data)

    if byte_size(compressed) < byte_size(data) do
      {@method_deflate, compressed, crc}
    else
      {@method_stored, data, crc}
    end
  end

  defp build_cd_record(e) do
    name_bin = :unicode.characters_to_binary(e.name, :utf8)
    name_len = byte_size(name_bin)

    [
      <<Constants.cd_sig()::little-32>>,
      <<Constants.ver_made_by()::little-16>>,
      <<e.version::little-16>>,
      <<Constants.flags()::little-16>>,
      <<e.method::little-16>>,
      <<e.datetime::little-32>>,
      <<e.crc32::little-32>>,
      <<e.compressed_size::little-32>>,
      <<e.size::little-32>>,
      <<name_len::little-16>>,
      <<0::little-16>>,
      <<0::little-16>>,
      <<0::little-16>>,
      <<0::little-16>>,
      <<0::little-32>>,
      <<e.local_offset::little-32>>,
      name_bin
    ]
  end

  defp build_eocd(entry_count, cd_size, cd_offset) do
    <<
      Constants.eocd_sig()::little-32,
      0::little-16,
      0::little-16,
      entry_count::little-16,
      entry_count::little-16,
      cd_size::little-32,
      cd_offset::little-32,
      0::little-16
    >>
  end

  # ─── Reader Internals ────────────────────────────────────────────────────────

  def zip(entries, opts \\ []) do
    w =
      Enum.reduce(entries, new_writer(), fn
        {name, data}, acc ->
          add_file(acc, name, data, opts)

        {name, data, entry_opts}, acc ->
          add_file(acc, name, data, entry_opts)
      end)

    finish(w)
  end

  def unzip(data) when is_binary(data) do
    reader = new_reader(data)

    Enum.reduce(reader_entries(reader), %{}, fn entry, acc ->
      content = reader_read(reader, entry)
      Map.put(acc, entry.name, content)
    end)
  end

  defp parse_archive(data) do
    eocd_offset = find_eocd(data)

    unless eocd_offset do
      raise Messages.err_eocd_not_found()
    end

    if eocd_offset + Constants.eocd_min_size() > byte_size(data) do
      raise Messages.err_eocd_truncated()
    end

    entry_count = read_le16(data, eocd_offset + Constants.eocd_offset_total_entries_disk())
    cd_size = read_le32(data, eocd_offset + Constants.eocd_offset_cd_size())
    cd_offset = read_le32(data, eocd_offset + Constants.eocd_offset_cd_offset())

    if entry_count > Constants.max_entry_count() do
      raise Messages.err_entry_count_exceeded()
    end

    if cd_offset > eocd_offset do
      raise Messages.err_cd_offset_overlaps_eocd()
    end

    if cd_offset + cd_size > eocd_offset do
      raise Messages.err_cd_region_extends_into_eocd()
    end

    if cd_offset + cd_size > byte_size(data) do
      raise Messages.err_cd_region_extends_beyond_archive()
    end

    parse_cd_entries(data, cd_offset, cd_size, entry_count)
  end

  defp find_eocd(data) do
    size = byte_size(data)
    start = max(0, size - Constants.eocd_min_size() - Constants.eocd_max_comment())
    find_eocd_loop(data, size - Constants.eocd_min_size(), start)
  end

  defp find_eocd_loop(_data, pos, _start) when pos < 0, do: nil

  defp find_eocd_loop(data, pos, start) do
    sig = read_le32(data, pos)

    cond do
      sig == Constants.eocd_sig() -> pos
      pos <= start -> nil
      true -> find_eocd_loop(data, pos - 1, start)
    end
  end

  defp parse_cd_entries(data, cd_offset, cd_size, entry_count) do
    do_parse_cd(data, cd_offset, cd_size, entry_count, 0, [], MapSet.new(), cd_offset)
  end

  defp do_parse_cd(_data, _pos, _cd_size, expected, count, entries, _seen_names, _cd_base)
       when count == expected do
    Enum.reverse(entries)
  end

  defp do_parse_cd(data, pos, cd_size, expected, count, entries, seen_names, cd_base) do
    data_size = byte_size(data)

    if pos + Constants.cd_entry_fixed_size() > data_size do
      raise Messages.err_cd_entry_header_out_of_bounds()
    end

    sig = read_le32(data, pos)

    unless sig == Constants.cd_sig() do
      raise Messages.err_expected_cd_sig()
    end

    method = read_le16(data, pos + Constants.cd_offset_method())
    crc32v = read_le32(data, pos + Constants.cd_offset_crc32())
    compressed_size = read_le32(data, pos + Constants.cd_offset_comp_size())
    size = read_le32(data, pos + Constants.cd_offset_uncomp_size())
    name_len = read_le16(data, pos + Constants.cd_offset_name_len())
    extra_len = read_le16(data, pos + Constants.cd_offset_extra_len())
    comment_len = read_le16(data, pos + Constants.cd_offset_comment_len())
    local_offset = read_le32(data, pos + Constants.cd_offset_local_header_offset())

    if Enum.any?(
         [method, crc32v, compressed_size, size, name_len, extra_len, comment_len, local_offset],
         &is_nil/1
       ) do
      raise Messages.err_cd_entry_fields_truncated()
    end

    name_start = pos + Constants.cd_entry_fixed_size()

    if name_start + name_len > data_size do
      raise Messages.err_cd_entry_name_extends_beyond_archive()
    end

    name_bin = binary_part(data, name_start, name_len)
    name = scrub_utf8(name_bin)
    validate_entry_name!(name)

    next_pos = pos + Constants.cd_entry_fixed_size() + name_len + extra_len + comment_len

    if next_pos > cd_base + cd_size do
      raise Messages.err_cd_entry_advance_out_of_bounds()
    end

    entry = %{
      name: name,
      method: method,
      crc32: crc32v,
      compressed_size: compressed_size,
      size: size,
      datetime: read_le32(data, pos + Constants.cd_offset_datetime()),
      version: read_le16(data, pos + Constants.cd_offset_version_needed()),
      local_offset: local_offset
    }

    if MapSet.member?(seen_names, name) do
      raise Messages.err_duplicate_entry()
    end

    next_seen = MapSet.put(seen_names, name)

    do_parse_cd(
      data,
      next_pos,
      cd_size,
      expected,
      count + 1,
      [entry | entries],
      next_seen,
      cd_base
    )
  end

  defp scrub_utf8(bin) when is_binary(bin) do
    case :unicode.characters_to_binary(bin, :utf8, :utf8) do
      result when is_binary(result) ->
        result

      _error ->
        case :unicode.characters_to_binary(bin, :latin1, :utf8) do
          result when is_binary(result) -> result
          _ -> raise Messages.err_entry_name_invalid_encoding()
        end
    end
  end

  defp validate_entry_name!(name) do
    cond do
      :binary.match(name, <<0>>) != :nomatch ->
        raise Messages.err_entry_name_null_byte()

      :binary.match(name, <<?\\>>) != :nomatch ->
        raise Messages.err_entry_name_backslash()

      match?(<<?/, _::binary>>, name) ->
        raise Messages.err_entry_name_absolute()

      match?(<<drive, ?:, _::binary>> when drive in ?A..?Z or drive in ?a..?z, name) ->
        raise Messages.err_entry_name_absolute()

      has_dotdot_segment?(name) ->
        raise Messages.err_entry_name_path_traversal()

      true ->
        :ok
    end
  end

  defp has_dotdot_segment?(<<?., ?., ?/, _::binary>>), do: true
  defp has_dotdot_segment?(<<?., ?.>>), do: true
  defp has_dotdot_segment?(<<?/, ?., ?., ?/, _::binary>>), do: true
  defp has_dotdot_segment?(<<?/, ?., ?.>>), do: true
  defp has_dotdot_segment?(<<_byte, rest::binary>>), do: has_dotdot_segment?(rest)
  defp has_dotdot_segment?(<<>>), do: false

  # ─── DEFLATE RFC 1951 Encoder ───────────────────────────────────────────────

  def raw_deflate(data) when is_binary(data) do
    deflate_compress(data)
  end

  defp deflate_compress(<<>>) do
    # BFINAL=1, BTYPE=00 (stored), LEN=0, NLEN=0xFFFF
    <<0x01, 0x00, 0x00, 0xFF, 0xFF>>
  end

  defp deflate_compress(data) when is_binary(data) do
    bw = bw_new()
    # BFINAL=1 (bit 0), BTYPE=01 (bits 1-2) = 3 bits: 3 (0b011)
    bw = bw_write(bw, 3, 3)

    table = :atomics.new(@hash_table_size, signed: false)
    bw = deflate_loop(data, 0, byte_size(data), table, bw)

    # End of block symbol 256: 7 bits, code 0
    bw = bw_write_symbol(bw, @eob_symbol)
    bw_finish(bw)
  end

  defp deflate_loop(data, pos, len, table, bw) when pos + 3 <= len do
    <<b0, b1, b2>> = binary_part(data, pos, 3)
    h = bxor(b0 <<< 10, bxor(b1 <<< 5, b2)) &&& @hash_mask
    table_idx = h + 1
    stored = :atomics.get(table, table_idx)
    :atomics.put(table, table_idx, pos + 1)

    match =
      if stored > 0 do
        prev_pos = stored - 1
        distance = pos - prev_pos

        if distance <= @max_distance and binary_part(data, prev_pos, 3) == <<b0, b1, b2>> do
          max_len = min(@max_match_len, len - pos)
          s1 = binary_part(data, prev_pos, max_len)
          s2 = binary_part(data, pos, max_len)
          match_len = :binary.longest_common_prefix([s1, s2])
          if match_len >= @min_match_len, do: {match_len, distance}, else: nil
        else
          nil
        end
      else
        nil
      end

    case match do
      {match_len, distance} ->
        bw = bw_write_match(bw, match_len, distance)
        deflate_loop(data, pos + match_len, len, table, bw)

      nil ->
        bw = bw_write_symbol(bw, b0)
        deflate_loop(data, pos + 1, len, table, bw)
    end
  end

  defp deflate_loop(data, pos, len, table, bw) when pos < len do
    b = :binary.at(data, pos)
    bw = bw_write_symbol(bw, b)
    deflate_loop(data, pos + 1, len, table, bw)
  end

  defp deflate_loop(_data, _pos, _len, _table, bw), do: bw

  defp find_dist_code(off) do
    cond do
      off <= 4 -> {off - 1, off, 0}
      off <= 6 -> {4, 5, 1}
      off <= 8 -> {5, 7, 1}
      off <= 12 -> {6, 9, 2}
      off <= 16 -> {7, 13, 2}
      off <= 24 -> {8, 17, 3}
      off <= 32 -> {9, 25, 3}
      off <= 48 -> {10, 33, 4}
      off <= 64 -> {11, 49, 4}
      off <= 96 -> {12, 65, 5}
      off <= 128 -> {13, 97, 5}
      off <= 192 -> {14, 129, 6}
      off <= 256 -> {15, 193, 6}
      off <= 384 -> {16, 257, 7}
      off <= 512 -> {17, 385, 7}
      off <= 768 -> {18, 513, 8}
      off <= 1024 -> {19, 769, 8}
      off <= 1536 -> {20, 1025, 9}
      off <= 2048 -> {21, 1537, 9}
      off <= 3072 -> {22, 2049, 10}
      off <= 4096 -> {23, 3073, 10}
      off <= 6144 -> {24, 4097, 11}
      off <= 8192 -> {25, 6145, 11}
      off <= 12288 -> {26, 8193, 12}
      off <= 16384 -> {27, 12289, 12}
      off <= 24576 -> {28, 16385, 13}
      off <= 32768 -> {29, 24577, 13}
      true -> raise Messages.err_distance_out_of_range()
    end
  end

  # ─── DEFLATE RFC 1951 Decoder ───────────────────────────────────────────────

  def raw_inflate(data, max_output \\ Constants.raw_inflate_max_output()) when is_binary(data) do
    raw_inflate_counted(data, max_output).output
  end

  def raw_inflate_counted(data, max_output \\ Constants.raw_inflate_max_output())
      when is_binary(data) do
    unless is_integer(max_output) and max_output >= 0 and
             max_output <= Constants.raw_inflate_max_output() do
      inflate_error!(:invalid_output_limit)
    end

    {reader, output} = decode_blocks(br_new(data), output_new(), max_output)

    %RawInflateResult{
      output: output_to_binary(output),
      bytes_consumed: reader.pos - div(reader.nbits, @bits_8)
    }
  end

  defp inflate_error!(reason) when is_atom(reason) do
    raise RawInflateError, code: Messages.inflate_error_code(reason)
  end

  defp decode_blocks(reader, output, max_output) do
    {final, reader} = strict_br_read(reader, 1)
    {block_type, reader} = strict_br_read(reader, 2)

    {reader, output} =
      case block_type do
        @btype_stored ->
          decode_stored_block(reader, output, max_output)

        @btype_fixed ->
          decode_compressed_block(
            @fixed_ll_decoder,
            @fixed_dist_decoder,
            reader,
            output,
            max_output
          )

        @btype_dynamic ->
          decode_dynamic_block(reader, output, max_output)

        @btype_reserved ->
          inflate_error!(:reserved_block_type)
      end

    if final == 1, do: {reader, output}, else: decode_blocks(reader, output, max_output)
  end

  defp decode_stored_block(reader, output, max_output) do
    reader = br_align(reader)

    if reader.pos + @stored_header_size > byte_size(reader.data) do
      inflate_error!(:unexpected_eof)
    end

    <<length::16-little, complement::16-little, _rest::binary>> =
      binary_part(reader.data, reader.pos, byte_size(reader.data) - reader.pos)

    if bxor(length, @mask_16) != complement do
      inflate_error!(:stored_length_mismatch)
    end

    if length > max_output - output.size do
      inflate_error!(:output_limit_exceeded)
    end

    if reader.pos + @stored_header_size + length > byte_size(reader.data) do
      inflate_error!(:unexpected_eof)
    end

    stored_data = binary_part(reader.data, reader.pos + @stored_header_size, length)
    next_output = output_append_bin(output, stored_data)
    next_reader = %{reader | pos: reader.pos + @stored_header_size + length, buf: 0, nbits: 0}

    {next_reader, next_output}
  end

  defp decode_dynamic_block(reader, output, max_output) do
    {hlit_value, reader} = strict_br_read(reader, 5)
    {hdist_value, reader} = strict_br_read(reader, 5)
    {hclen_value, reader} = strict_br_read(reader, 4)
    hlit = hlit_value + 257
    hdist = hdist_value + 1
    hclen = hclen_value + 4

    if hlit > @max_ll_codes, do: inflate_error!(:invalid_literal_length_symbol)

    code_order = Constants.code_length_order()

    {code_length_lengths, reader} =
      Enum.reduce(0..(hclen - 1)//1, {List.duplicate(0, 19), reader}, fn index,
                                                                         {lengths, next_reader} ->
        {value, next_reader} = strict_br_read(next_reader, 3)
        {List.replace_at(lengths, Enum.at(code_order, index), value), next_reader}
      end)

    code_length_decoder = build_huffman_decoder!(code_length_lengths)

    unless code_length_decoder.complete do
      inflate_error!(:incomplete_code_length_tree)
    end

    {lengths, reader} = decode_code_lengths(code_length_decoder, reader, hlit + hdist, 0, [])
    {literal_lengths, distance_lengths} = Enum.split(lengths, hlit)
    literal_decoder = build_huffman_decoder!(literal_lengths)

    unless literal_decoder.complete do
      inflate_error!(:incomplete_literal_length_tree)
    end

    distance_decoder = build_huffman_decoder!(distance_lengths)

    permitted_distance =
      distance_decoder.complete or
        (distance_decoder.symbol_count == 1 and distance_decoder.one_bit_count == 1) or
        distance_decoder.symbol_count == 0

    unless permitted_distance, do: inflate_error!(:incomplete_distance_tree)

    decode_compressed_block(
      literal_decoder,
      distance_decoder,
      reader,
      output,
      max_output
    )
  end

  defp decode_code_lengths(_decoder, reader, total, count, acc_rev) when count >= total do
    {:lists.reverse(acc_rev), reader}
  end

  defp decode_code_lengths(decoder, reader, total, count, acc_rev) do
    {symbol, reader} = decode_symbol(decoder, reader, :invalid_literal_length_symbol)

    cond do
      symbol <= 15 ->
        decode_code_lengths(decoder, reader, total, count + 1, [symbol | acc_rev])

      symbol == 16 ->
        if acc_rev == [], do: inflate_error!(:repeat_without_previous)
        {extra, reader} = strict_br_read(reader, 2)
        repeat = extra + 3
        if count + repeat > total, do: inflate_error!(:repeat_overrun)
        last_val = hd(acc_rev)
        new_acc = prepend_n(acc_rev, last_val, repeat)
        decode_code_lengths(decoder, reader, total, count + repeat, new_acc)

      symbol in [17, 18] ->
        {bits, base} = if symbol == 17, do: {3, 3}, else: {7, 11}
        {extra, reader} = strict_br_read(reader, bits)
        repeat = extra + base
        if count + repeat > total, do: inflate_error!(:repeat_overrun)
        new_acc = prepend_n(acc_rev, 0, repeat)
        decode_code_lengths(decoder, reader, total, count + repeat, new_acc)

      true ->
        inflate_error!(:invalid_literal_length_symbol)
    end
  end

  defp prepend_n(acc, _val, 0), do: acc
  defp prepend_n(acc, val, n) when n > 0, do: prepend_n([val | acc], val, n - 1)

  defp build_huffman_decoder!(lengths) do
    case Constants.build_huffman_decoder(lengths) do
      {:ok, decoder} -> decoder
      {:error, reason} -> inflate_error!(reason)
    end
  end

  defp decode_symbol(decoder, reader, err_atom) do
    case refill_br_safe(reader, @fast_table_bits) do
      :eof ->
        decode_symbol_slow(decoder, reader, err_atom, 1, 0)

      %{buf: buf} = filled ->
        peek = band(buf, @fast_table_mask)

        case elem(decoder.fast_table, peek) do
          {:hit, symbol, len} ->
            {symbol, br_drop(filled, len)}

          :slow ->
            decode_symbol_slow(decoder, reader, err_atom, 1, 0)
        end
    end
  end

  defp decode_symbol_slow(_decoder, _reader, err_atom, 16, _code) do
    inflate_error!(err_atom)
  end

  defp decode_symbol_slow(decoder, reader, err_atom, length, code) do
    {bit, reader} = strict_br_read(reader, 1)
    code = code <<< 1 ||| bit

    case Map.fetch(decoder.table, {length, code}) do
      {:ok, symbol} -> {symbol, reader}
      :error -> decode_symbol_slow(decoder, reader, err_atom, length + 1, code)
    end
  end

  defp decode_compressed_block(ll_decoder, distance_decoder, reader, output, max_output) do
    {symbol, reader} = decode_symbol(ll_decoder, reader, :invalid_literal_length_symbol)

    cond do
      symbol in 0..255 ->
        if output.size >= max_output, do: inflate_error!(:output_limit_exceeded)

        decode_compressed_block(
          ll_decoder,
          distance_decoder,
          reader,
          output_append(output, symbol),
          max_output
        )

      symbol == @eob_symbol ->
        {reader, output}

      symbol in 257..285 ->
        {base_length, extra_bits} = decode_length_parameters(symbol)
        {extra, reader} = strict_br_read(reader, extra_bits)
        length = base_length + extra

        {distance_symbol, reader} =
          decode_symbol(distance_decoder, reader, :reserved_distance_symbol)

        if distance_symbol > 29, do: inflate_error!(:reserved_distance_symbol)
        {base_distance, distance_extra_bits} = decode_distance_parameters(distance_symbol)
        {distance_extra, reader} = strict_br_read(reader, distance_extra_bits)
        distance = base_distance + distance_extra

        decode_compressed_block(
          ll_decoder,
          distance_decoder,
          reader,
          output_copy(output, distance, length, max_output),
          max_output
        )

      true ->
        inflate_error!(:invalid_literal_length_symbol)
    end
  end

  defp decode_length_parameters(symbol) do
    cond do
      symbol <= 264 -> {symbol - 254, 0}
      symbol <= 268 -> {11 + (symbol - 265) * 2, 1}
      symbol <= 272 -> {19 + (symbol - 269) * 4, 2}
      symbol <= 276 -> {35 + (symbol - 273) * 8, 3}
      symbol <= 280 -> {67 + (symbol - 277) * 16, 4}
      symbol <= 284 -> {131 + (symbol - 281) * 32, 5}
      symbol == 285 -> {258, 0}
    end
  end

  defp decode_distance_parameters(symbol) do
    case symbol do
      s when s in 0..3 ->
        {s + 1, 0}

      s ->
        extra_bits = div(s, 2) - 1
        base = (1 <<< (extra_bits + 1)) + 1 + (rem(s, 2) <<< extra_bits)
        {base, extra_bits}
    end
  end

  # ─── Paged OutputBuffer ──────────────────────────────────────────────────────

  defp output_new do
    %{
      size: 0,
      pages: %{},
      tail_bin: <<>>,
      pending_rev: [],
      pending_len: 0
    }
  end

  defp output_append(%{pending_len: plen, pending_rev: prev} = out, byte)
       when plen < @max_pending do
    %{out | size: out.size + 1, pending_rev: [byte | prev], pending_len: plen + 1}
  end

  defp output_append(out, byte) do
    out = flush_pending(out)
    %{out | size: out.size + 1, pending_rev: [byte], pending_len: 1}
  end

  defp flush_pending(%{pending_len: 0} = out), do: out

  defp flush_pending(%{pending_rev: prev} = out) do
    flushed_bin = :erlang.list_to_binary(:lists.reverse(prev))
    new_tail = <<out.tail_bin::binary, flushed_bin::binary>>
    {new_pages, final_tail} = roll_pages(out.pages, map_size(out.pages), new_tail)

    %{
      out
      | pages: new_pages,
        tail_bin: final_tail,
        pending_rev: [],
        pending_len: 0
    }
  end

  defp roll_pages(pages, next_page_idx, tail) when byte_size(tail) >= @page_size do
    <<page::binary-size(@page_size), rest::binary>> = tail
    roll_pages(Map.put(pages, next_page_idx, page), next_page_idx + 1, rest)
  end

  defp roll_pages(pages, _next_page_idx, tail) do
    {pages, tail}
  end

  defp output_append_bin(out, <<>>), do: out

  defp output_append_bin(out, bin) when is_binary(bin) do
    out = flush_pending(out)
    new_tail = <<out.tail_bin::binary, bin::binary>>
    {new_pages, final_tail} = roll_pages(out.pages, map_size(out.pages), new_tail)
    %{out | size: out.size + byte_size(bin), pages: new_pages, tail_bin: final_tail}
  end

  defp output_copy(output, distance, length, max_output) do
    if distance == 0 or distance > output.size or distance > @max_distance do
      inflate_error!(:invalid_back_reference)
    end

    if length > max_output - output.size do
      inflate_error!(:output_limit_exceeded)
    end

    output = flush_pending(output)

    if distance == 1 do
      last_byte = get_byte_at(output, output.size - 1)
      copied = :binary.copy(<<last_byte>>, length)
      output_append_bin(output, copied)
    else
      if distance >= length do
        slice = slice_output(output, output.size - distance, length)
        output_append_bin(output, slice)
      else
        pattern = slice_output(output, output.size - distance, distance)
        reps = div(length, distance)
        rem_len = rem(length, distance)

        copied =
          if rem_len == 0 do
            :binary.copy(pattern, reps)
          else
            <<:binary.copy(pattern, reps)::binary, binary_part(pattern, 0, rem_len)::binary>>
          end

        output_append_bin(output, copied)
      end
    end
  end

  defp get_byte_at(output, pos) do
    page_idx = div(pos, @page_size)
    offset = rem(pos, @page_size)

    if page_idx == map_size(output.pages) do
      :binary.at(output.tail_bin, offset)
    else
      page = Map.fetch!(output.pages, page_idx)
      :binary.at(page, offset)
    end
  end

  defp slice_output(_output, _start_pos, 0), do: <<>>

  defp slice_output(output, start_pos, len) do
    page_idx = div(start_pos, @page_size)
    offset = rem(start_pos, @page_size)
    page_bytes_left = @page_size - offset

    if len <= page_bytes_left do
      page = get_page_or_tail(output, page_idx)
      binary_part(page, offset, len)
    else
      part1_len = page_bytes_left
      page1 = get_page_or_tail(output, page_idx)
      part1 = binary_part(page1, offset, part1_len)

      part2_len = len - part1_len
      part2 = slice_output(output, start_pos + part1_len, part2_len)

      <<part1::binary, part2::binary>>
    end
  end

  defp get_page_or_tail(output, page_idx) do
    if page_idx == map_size(output.pages) do
      output.tail_bin
    else
      Map.fetch!(output.pages, page_idx)
    end
  end

  defp output_to_binary(output) do
    output = flush_pending(output)
    total_pages = map_size(output.pages)

    if total_pages == 0 do
      output.tail_bin
    else
      page_list = for p <- 0..(total_pages - 1)//1, do: Map.fetch!(output.pages, p)
      :erlang.iolist_to_binary([page_list, output.tail_bin])
    end
  end

  # ─── BitReader ───────────────────────────────────────────────────────────────

  defp br_new(data), do: %{data: data, pos: 0, buf: 0, nbits: 0}

  defp br_align(%{pos: pos, nbits: nb} = br) do
    rewound_pos = pos - div(nb, @bits_8)
    %{br | pos: rewound_pos, buf: 0, nbits: 0}
  end

  defp br_drop(%{buf: buf, nbits: nbits} = reader, count) do
    %{reader | buf: buf >>> count, nbits: nbits - count}
  end

  defp strict_br_read(reader, 0), do: {0, reader}

  defp strict_br_read(reader, count) do
    case br_try_read(reader, count) do
      :eof -> inflate_error!(:unexpected_eof)
      result -> result
    end
  end

  defp br_try_read(reader, count) do
    case refill_br_safe(reader, count) do
      :eof ->
        :eof

      %{buf: buffer, nbits: bits} = filled ->
        value = buffer &&& (1 <<< count) - 1
        {value, %{filled | buf: buffer >>> count, nbits: bits - count}}
    end
  end

  defp refill_br_safe(%{data: data, pos: pos, buf: buffer, nbits: bits} = reader, count)
       when bits < count do
    if pos >= byte_size(data) do
      :eof
    else
      byte = :binary.at(data, pos)

      refill_br_safe(
        %{reader | pos: pos + 1, buf: buffer ||| byte <<< bits, nbits: bits + @bits_8},
        count
      )
    end
  end

  defp refill_br_safe(reader, _count), do: reader

  # ─── BitWriter (Chunked) ─────────────────────────────────────────────────────

  defp bw_new do
    %{
      buf: 0,
      nbits: 0,
      current_chunk: <<>>,
      chunks: []
    }
  end

  defp bw_write(%{buf: buf, nbits: nbits, current_chunk: chunk, chunks: chunks}, val, n) do
    new_buf = buf ||| (val &&& (1 <<< n) - 1) <<< nbits
    new_nbits = nbits + n

    if new_nbits >= @bits_64 do
      flush_bw_64(new_buf, new_nbits, chunk, chunks)
    else
      %{buf: new_buf, nbits: new_nbits, current_chunk: chunk, chunks: chunks}
    end
  end

  defp flush_bw_64(buf, nbits, chunk, chunks) when nbits >= @bits_64 do
    word = buf &&& @mask_64
    new_buf = buf >>> @bits_64
    new_nbits = nbits - @bits_64
    new_chunk = <<chunk::binary, word::64-little>>

    if byte_size(new_chunk) >= @chunk_buffer_size do
      flush_bw_64(new_buf, new_nbits, <<>>, [new_chunk | chunks])
    else
      flush_bw_64(new_buf, new_nbits, new_chunk, chunks)
    end
  end

  defp flush_bw_64(buf, nbits, chunk, chunks) do
    %{buf: buf, nbits: nbits, current_chunk: chunk, chunks: chunks}
  end

  defp bw_write_symbol(bw, symbol) do
    {code_rev, bits} = elem(@fixed_ll_encode_table, symbol)
    bw_write(bw, code_rev, bits)
  end

  defp bw_write_match(bw, length, distance) do
    {ll_sym, base_len, extra_bits} = elem(@length_lookup_table, length)
    {ll_rev, ll_bits} = elem(@fixed_ll_encode_table, ll_sym)
    bw = bw_write(bw, ll_rev, ll_bits)

    bw =
      if extra_bits > 0 do
        bw_write(bw, length - base_len, extra_bits)
      else
        bw
      end

    {dist_code, base_dist, dist_extra} = find_dist_code(distance)
    {dist_rev, dist_bits} = elem(@dist_encode_table, dist_code)
    bw = bw_write(bw, dist_rev, dist_bits)

    if dist_extra > 0 do
      bw_write(bw, distance - base_dist, dist_extra)
    else
      bw
    end
  end

  defp bw_finish(bw) do
    %{buf: buf, nbits: nbits, current_chunk: chunk, chunks: chunks} = bw
    bytes_count = div(nbits + 7, @bits_8)

    final_chunk =
      if bytes_count == 0 do
        chunk
      else
        tail_bytes =
          for i <- 0..(bytes_count - 1)//1 do
            buf >>> (i * @bits_8) &&& @mask_8
          end
          |> :erlang.list_to_binary()

        <<chunk::binary, tail_bytes::binary>>
      end

    all_chunks = :lists.reverse([final_chunk | chunks])
    :erlang.iolist_to_binary(all_chunks)
  end

  # ─── Byte Reading Helpers ────────────────────────────────────────────────────

  defp read_le16(data, offset) when offset + 2 <= byte_size(data) do
    <<val::little-16>> = binary_part(data, offset, 2)
    val
  end

  defp read_le16(_data, _offset), do: nil

  defp read_le32(data, offset) when offset + 4 <= byte_size(data) do
    <<val::little-32>> = binary_part(data, offset, 4)
    val
  end

  defp read_le32(_data, _offset), do: nil
end
