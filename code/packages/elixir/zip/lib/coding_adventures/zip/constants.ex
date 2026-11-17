defmodule CodingAdventures.Zip.Constants do
  @moduledoc CodingAdventures.Zip.Messages.constants_moduledoc()
  import Bitwise

  # Wire signatures
  def local_sig, do: 0x04034B50
  def cd_sig, do: 0x02014B50
  def eocd_sig, do: 0x06054B50

  # Bit flags and versions
  def flags, do: 0x0800
  def ver_deflate, do: 20
  def ver_stored, do: 10
  def ver_made_by, do: 0x031E
  def dos_epoch, do: 0x00210000

  # DOS datetime encoding
  def dos_base_year, do: 1980
  def dos_year_shift, do: 25
  def dos_month_shift, do: 21
  def dos_day_shift, do: 16
  def dos_hour_shift, do: 11
  def dos_minute_shift, do: 5
  def dos_second_div, do: 2

  # Compression methods
  def method_stored, do: 0
  def method_deflate, do: 8

  # Output limits
  def raw_inflate_max_output, do: 268_435_456
  def png_ceiling_output, do: 134_234_112

  # DEFLATE block types (RFC 1951)
  def btype_stored, do: 0
  def btype_fixed, do: 1
  def btype_dynamic, do: 2
  def btype_reserved, do: 3

  # Engine constants
  def min_match_len, do: 3
  def max_match_len, do: 258
  def max_distance, do: 32768
  def window_size, do: 32768
  def hash_table_size, do: 8192
  def hash_mask, do: 8191
  def eob_symbol, do: 256
  def max_ll_codes, do: 286

  # Fast Huffman peek table
  def fast_table_size, do: 512
  def fast_table_bits, do: 9
  def fast_table_mask, do: 0x1FF
  def max_pending, do: 512

  # RFC 1951 permutation for code lengths
  def code_length_order, do: [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15]

  # Paged buffer & streaming chunk constants
  def page_size, do: 4096
  def chunk_buffer_size, do: 65536
  def stored_header_size, do: 4

  # ZIP structural sizes & offsets
  def eocd_min_size, do: 22
  def eocd_max_comment, do: 65535
  def max_entry_count, do: 65535
  def local_header_fixed_size, do: 30
  def cd_entry_fixed_size, do: 46

  # Header field offsets
  def local_offset_method, do: 8
  def local_offset_datetime, do: 10
  def local_offset_crc32, do: 14
  def local_offset_comp_size, do: 18
  def local_offset_uncomp_size, do: 22
  def local_offset_name_len, do: 26
  def local_offset_extra_len, do: 28

  def cd_offset_version_made_by, do: 4
  def cd_offset_version_needed, do: 6
  def cd_offset_flags, do: 8
  def cd_offset_method, do: 10
  def cd_offset_datetime, do: 12
  def cd_offset_crc32, do: 16
  def cd_offset_comp_size, do: 20
  def cd_offset_uncomp_size, do: 24
  def cd_offset_name_len, do: 28
  def cd_offset_extra_len, do: 30
  def cd_offset_comment_len, do: 32
  def cd_offset_disk_start, do: 34
  def cd_offset_internal_attr, do: 36
  def cd_offset_external_attr, do: 38
  def cd_offset_local_header_offset, do: 42

  def eocd_offset_disk, do: 4
  def eocd_offset_start_disk, do: 6
  def eocd_offset_total_entries_disk, do: 8
  def eocd_offset_total_entries, do: 10
  def eocd_offset_cd_size, do: 12
  def eocd_offset_cd_offset, do: 16
  def eocd_offset_comment_len, do: 20

  # CRC-32 and Bitwise masks
  def crc32_poly, do: 0xEDB88320
  def crc32_initial, do: 0xFFFFFFFF
  def mask_8, do: 0xFF
  def mask_16, do: 0xFFFF
  def mask_32, do: 0xFFFFFFFF
  def mask_64, do: 0xFFFFFFFFFFFFFFFF
  def bits_64, do: 64
  def bits_32, do: 32
  def bits_16, do: 16
  def bits_8, do: 8

  def fixed_ll_lengths do
    for i <- 0..287 do
      cond do
        i <= 143 -> 8
        i <= 255 -> 9
        i <= 279 -> 7
        true -> 8
      end
    end
  end

  def fixed_dist_lengths, do: for(_ <- 0..31, do: 5)

  def fixed_ll_decoder do
    {:ok, dec} = build_huffman_decoder(fixed_ll_lengths())
    dec
  end

  def fixed_dist_decoder do
    {:ok, dec} = build_huffman_decoder(fixed_dist_lengths())
    dec
  end

  # Huffman decoder construction
  def build_huffman_decoder(lengths) do
    try do
      counts =
        Enum.reduce(lengths, List.duplicate(0, 16), fn length, acc ->
          if length > 15, do: throw({:error, :invalid_literal_length_symbol})
          if length == 0, do: acc, else: List.update_at(acc, length, &(&1 + 1))
        end)

      left =
        Enum.reduce(1..15, 1, fn length, remaining ->
          next = remaining * 2 - Enum.at(counts, length)
          if next < 0, do: throw({:error, :huffman_oversubscribed})
          next
        end)

      {next_codes, _} =
        Enum.reduce(1..15, {%{}, 0}, fn length, {codes, code} ->
          next = (code + Enum.at(counts, length - 1)) <<< 1
          {Map.put(codes, length, next), next}
        end)

      {table, _, symbol_count, triples} =
        Enum.reduce(Enum.with_index(lengths), {%{}, next_codes, 0, []}, fn
          {0, _symbol}, acc ->
            acc

          {length, symbol}, {table, codes, symbol_count, acc_triples} ->
            code = Map.fetch!(codes, length)

            {
              Map.put(table, {length, code}, symbol),
              Map.put(codes, length, code + 1),
              symbol_count + 1,
              [{length, symbol, code} | acc_triples]
            }
        end)

      fast_table = build_fast_table(triples)

      {:ok,
       %{
         table: table,
         fast_table: fast_table,
         complete: left == 0,
         symbol_count: symbol_count,
         one_bit_count: Enum.at(counts, 1)
       }}
    catch
      {:error, reason} -> {:error, reason}
    end
  end

  defp build_fast_table(triples) do
    initial_map =
      for i <- 0..511, into: %{} do
        {i, :slow}
      end

    populated_map =
      Enum.reduce(triples, initial_map, fn {length, symbol, code}, acc ->
        if length <= 9 do
          rev_code = reverse_bits(code, length)
          step_count = bsl(1, 9 - length)

          Enum.reduce(0..(step_count - 1)//1, acc, fn step, step_acc ->
            idx = bor(rev_code, bsl(step, length))
            Map.put(step_acc, idx, {:hit, symbol, length})
          end)
        else
          acc
        end
      end)

    for i <- 0..511 do
      Map.fetch!(populated_map, i)
    end
    |> List.to_tuple()
  end

  defp reverse_bits(_val, 0), do: 0

  defp reverse_bits(val, len) when len > 0 do
    Enum.reduce(0..(len - 1)//1, 0, fn i, acc ->
      bit = band(bsr(val, i), 1)
      bor(acc, bsl(bit, len - 1 - i))
    end)
  end
end
