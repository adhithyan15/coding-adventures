defmodule CodingAdventures.Zip.PngScaleHardeningTest do
  use ExUnit.Case, async: true
  import Bitwise

  alias CodingAdventures.Zip
  alias CodingAdventures.Zip.Constants
  alias CodingAdventures.Zip.Messages
  alias CodingAdventures.Zip.RawInflateError
  alias CodingAdventures.Zip.RawInflateResult

  # ─── 1. Measured IC18 PNG Ceiling (134,234,112 bytes) ───────────────────────

  describe "IC18 PNG ceiling scale tests (134,234,112 bytes)" do
    @tag :scale
    test "round-trip through raw_deflate, :zlib, raw_inflate_counted, and crc32" do
      # IC18 PNG max dimension: 16,384 x 2,048 pixels = 33,554,432 pixels.
      # RGBA scanline bytes: 33,554,432 * 4 + 16,384 = 134,234,112 bytes.
      png_ceiling = Constants.png_ceiling_output()
      assert png_ceiling == 134_234_112

      # Generate repeating 64-byte scanline pattern
      pattern = for i <- 0..63, into: <<>>, do: <<rem(i * 17 + 1, 256)>>
      assert byte_size(pattern) == 64
      repeat_count = div(png_ceiling, 64)
      large_data = :binary.copy(pattern, repeat_count)
      assert byte_size(large_data) == png_ceiling

      # Initial process memory snapshot
      {:memory, initial_mem} = :erlang.process_info(self(), :memory)

      start_time = System.monotonic_time(:millisecond)

      # 1. Compress with raw_deflate
      compressed = Zip.raw_deflate(large_data)
      assert is_binary(compressed)
      assert byte_size(compressed) < div(png_ceiling, 50)

      # 2. Cross-verify compression against Erlang stdlib :zlib
      zlib_inflated = zlib_raw_inflate(compressed)
      assert byte_size(zlib_inflated) == png_ceiling
      assert zlib_inflated == large_data

      # 3. Inflate with Zip.raw_inflate_counted under ceiling
      result = Zip.raw_inflate_counted(compressed, png_ceiling)
      assert %RawInflateResult{} = result
      assert result.bytes_consumed == byte_size(compressed)
      assert byte_size(result.output) == png_ceiling
      assert result.output == large_data

      # 4. Compute CRC-32 over full 134 MB
      crc_val = Zip.crc32(large_data)
      assert is_integer(crc_val)
      assert crc_val > 0

      # 5. Enforce output limit rejection when ceiling - 1
      err =
        assert_raise RawInflateError, fn ->
          Zip.raw_inflate_counted(compressed, png_ceiling - 1)
        end

      assert err.code == "output-limit-exceeded"

      elapsed_ms = System.monotonic_time(:millisecond) - start_time
      {:memory, final_mem} = :erlang.process_info(self(), :memory)

      # Strict wall-clock assertion: under 10 seconds total on this runner
      assert elapsed_ms < 10_000

      # Memory check: process memory stayed bounded (< 256 MB heap expansion)
      heap_delta = final_mem - initial_mem
      assert heap_delta < 268_435_456
    end
  end

  # ─── 2. Multi-block, Paged Buffers & Back-References ────────────────────────

  describe "paged buffer & back-reference boundary conditions" do
    test "RLE pattern (distance == 1) cross-page expansion" do
      # 50,000 repetitions of byte 42 (spans > 12 pages of 4096 bytes)
      raw = :binary.copy(<<42>>, 50_000)
      compressed = Zip.raw_deflate(raw)
      result = Zip.raw_inflate_counted(compressed)
      assert result.output == raw
      assert result.bytes_consumed == byte_size(compressed)
    end

    test "multi-page slice boundary back-reference (distance >= length across 4 KiB)" do
      header = :binary.copy(<<1, 2, 3, 4>>, 1000)
      repeat = binary_part(header, 3900, 100) <> <<9, 8, 7, 6>>
      raw = header <> repeat
      compressed = Zip.raw_deflate(raw)
      assert Zip.raw_inflate(compressed) == raw
    end

    test "overlapping pattern replication (1 < distance < length)" do
      pattern = "ABCDE"
      raw = :binary.copy(pattern, 100)
      compressed = Zip.raw_deflate(raw)
      assert Zip.raw_inflate(compressed) == raw
    end

    test "full 32 KiB window boundary back-reference" do
      head = :binary.copy(<<0xAA>>, 32_768)
      tail = <<0xAA, 0xBB, 0xCC>>
      raw = head <> tail
      compressed = Zip.raw_deflate(raw)
      assert Zip.raw_inflate(compressed) == raw
    end

    test "exact 64-bit alignment in bw_finish (nbits == 0 regression)" do
      # 3 header bits + 6 * 9 literal bits (codes >= 144) + 7 EOB bits = 64 bits exactly
      raw = <<200, 201, 202, 203, 204, 205>>
      compressed = Zip.raw_deflate(raw)
      assert byte_size(compressed) == 8
      assert Zip.raw_inflate(compressed) == raw
      result = Zip.raw_inflate_counted(compressed)
      assert result.output == raw
      assert result.bytes_consumed == byte_size(compressed)
    end

    test "sweep lengths 0..128 of non-repeating bytes covering all nbits mod-64 alignments" do
      # Test both < 144 (8-bit fixed Huffman codes) and >= 144 (9-bit fixed Huffman codes)
      for len <- 0..128 do
        # Non-repeating bytes < 144
        data_low = for(i <- 0..(len - 1)//1, into: <<>>, do: <<rem(i, 140)>>)
        compressed_low = Zip.raw_deflate(data_low)
        assert Zip.raw_inflate(compressed_low) == data_low

        # Non-repeating bytes >= 144 (range 144..255)
        data_high = for(i <- 0..(len - 1)//1, into: <<>>, do: <<144 + rem(i, 112)>>)
        compressed_high = Zip.raw_deflate(data_high)
        assert Zip.raw_inflate(compressed_high) == data_high
      end
    end
  end

  # ─── 3. 7-Class CVE Resilience Showcase ─────────────────────────────────────

  describe "7-class ZIP and RFC 1951 CVE resilience showcase" do
    test "CVE Class 1: Zip-bomb amplification rejected via output-limit-exceeded" do
      bomb_data = :binary.copy(<<0>>, 100_000)
      compressed = Zip.raw_deflate(bomb_data)

      err =
        assert_raise RawInflateError, fn ->
          Zip.raw_inflate_counted(compressed, 1024)
        end

      assert err.code == "output-limit-exceeded"
      assert Exception.message(err) == "output-limit-exceeded"
    end

    test "CVE Class 2: Huffman oversubscription and incomplete trees fail closed" do
      oversubscribed = <<
        0b00000101,
        0b00000000,
        0b10000000,
        0b00000000,
        0b00000000
      >>

      assert_raise RawInflateError, fn ->
        Zip.raw_inflate(oversubscribed)
      end
    end

    test "CVE Class 3: Back-reference before start of output fails closed" do
      invalid_ref_stream = <<0x03, 0x8E, 0x04, 0x14, 0x00>>

      assert_raise RawInflateError, fn ->
        Zip.raw_inflate(invalid_ref_stream)
      end
    end

    test "CVE Class 4: Stored-block NLEN complement mismatch rejected" do
      corrupt_stored = <<0x01, 0x04, 0x00, 0x04, 0x00, 1, 2, 3, 4>>

      err =
        assert_raise RawInflateError, fn ->
          Zip.raw_inflate(corrupt_stored)
        end

      assert err.code == "stored-length-mismatch"
    end

    test "CVE Class 5: Trailing bytes detected via bytes_consumed" do
      raw = "clean valid payload"
      compressed = Zip.raw_deflate(raw)
      with_cavity = compressed <> <<0xDE, 0xAD, 0xBE, 0xEF>>

      result = Zip.raw_inflate_counted(with_cavity)
      assert result.output == raw
      assert result.bytes_consumed == byte_size(compressed)
      assert result.bytes_consumed < byte_size(with_cavity)
    end

    test "CVE Class 6: Zip-Slip traversal paths rejected fail-closed on write and read" do
      writer = Zip.new_writer()

      # Write-side rejections
      assert_raise RuntimeError, ~r/path traversal/, fn ->
        Zip.add_file(writer, "../../etc/passwd", "evil")
      end

      assert_raise RuntimeError, ~r/path traversal/, fn ->
        Zip.add_file(writer, "safe/../../../shadow", "evil")
      end

      assert_raise RuntimeError, ~r/absolute path/, fn ->
        Zip.add_file(writer, "/root/secret", "evil")
      end

      assert_raise RuntimeError, ~r/absolute path/, fn ->
        Zip.add_file(writer, "C:evil.txt", "evil")
      end

      assert_raise RuntimeError, ~r/absolute path/, fn ->
        Zip.add_file(writer, "C:/Windows/System32", "evil")
      end

      assert_raise RuntimeError, ~r/backslash/, fn ->
        Zip.add_file(writer, "windows\\path\\traversal", "evil")
      end

      assert_raise RuntimeError, ~r/null byte/, fn ->
        Zip.add_file(writer, "safe.txt\0evil.sh", "evil")
      end

      # Read-side rejections on crafted Central Directory entries
      craft_zip = fn entry_name ->
        name_len = byte_size(entry_name)

        local_hdr =
          <<0x50, 0x4B, 0x03, 0x04, 20::little-16, 0::little-16, 0::little-16, 0::little-32,
            0::little-32, 0::little-32, 0::little-32, name_len::little-16, 0::little-16>>

        local = local_hdr <> entry_name

        cd_hdr =
          <<0x50, 0x4B, 0x01, 0x02, 0x031E::little-16, 20::little-16, 0::little-16, 0::little-16,
            0::little-32, 0::little-32, 0::little-32, 0::little-32, name_len::little-16,
            0::little-16, 0::little-16, 0::little-16, 0::little-16, 0::little-32, 0::little-32>>

        cd = cd_hdr <> entry_name

        cd_size = byte_size(cd)
        cd_offset = byte_size(local)

        eocd =
          <<0x50, 0x4B, 0x05, 0x06, 0::little-16, 0::little-16, 1::little-16, 1::little-16,
            cd_size::little-32, cd_offset::little-32, 0::little-16>>

        local <> cd <> eocd
      end

      malicious_read_cases = [
        {"../../etc/passwd", ~r/path traversal/},
        {"safe/../../../shadow", ~r/path traversal/},
        {"/root/secret", ~r/absolute path/},
        {"C:evil.txt", ~r/absolute path/},
        {"C:/Windows/System32", ~r/absolute path/},
        {"windows\\path\\traversal", ~r/backslash/},
        {"safe.txt\0evil.sh", ~r/null byte/}
      ]

      for {malicious_name, expected_pattern} <- malicious_read_cases do
        archive = craft_zip.(malicious_name)

        assert_raise RuntimeError, expected_pattern, fn ->
          Zip.new_reader(archive)
        end

        assert_raise RuntimeError, expected_pattern, fn ->
          Zip.unzip(archive)
        end
      end
    end

    test "CVE Class 7: All 14 RFC 1951 codes and static diagnostics are payload-blind" do
      error_codes = Zip.raw_inflate_error_codes()
      assert length(error_codes) == 14

      for code <- error_codes do
        assert Messages.inflate_message(code) == code
      end

      assert Messages.inflate_message("unknown-nonexistent-code") == "unknown-error"
      assert Messages.redacted_placeholder() == "<redacted>"
      assert Messages.invalid_placeholder() == "<invalid>"
      assert Messages.unknown_error_message() == "zip: unknown error"

      for msg <- Messages.all_diagnostic_messages() do
        assert is_binary(msg)
        assert byte_size(msg) > 0
        refute String.contains?(msg, "password")
        refute String.contains?(msg, "secret")
      end
    end

    test "Standing Rule #20: zero double-quote characters in zip.ex and constants.ex" do
      zip_ex_path = Path.expand("../lib/coding_adventures/zip.ex", __DIR__)
      constants_ex_path = Path.expand("../lib/coding_adventures/zip/constants.ex", __DIR__)

      zip_content = File.read!(zip_ex_path)
      constants_content = File.read!(constants_ex_path)

      refute String.contains?(zip_content, "\"")
      refute String.contains?(constants_content, "\"")
    end
  end

  # ─── 4. Bidirectional PRNG Property Round-Trips against :zlib ───────────────

  describe "bidirectional PRNG property tests vs Erlang :zlib" do
    test "property round-trips across various buffer sizes and patterns" do
      sizes = [1, 2, 3, 7, 15, 31, 64, 127, 255, 512, 1024, 4096, 8192, 16384, 32768, 65536]

      for {size, idx} <- Enum.with_index(sizes) do
        seed = 0x12345678 + idx * 7919
        data = generate_prng_binary(seed, size)

        # 1. Deflate with Zip, Inflate with :zlib
        zip_deflated = Zip.raw_deflate(data)
        zlib_result = zlib_raw_inflate(zip_deflated)
        assert zlib_result == data

        # 2. Deflate with :zlib, Inflate with Zip
        zlib_deflated = zlib_raw_deflate(data)
        zip_result = Zip.raw_inflate_counted(zlib_deflated)
        assert zip_result.output == data
        assert zip_result.bytes_consumed == byte_size(zlib_deflated)

        # 3. Zip self-roundtrip
        self_result = Zip.raw_inflate_counted(zip_deflated)
        assert self_result.output == data
        assert self_result.bytes_consumed == byte_size(zip_deflated)

        # 4. CRC-32 consistency
        assert Zip.crc32(data) == :erlang.crc32(data)
      end
    end

    test "1 MiB pseudo-random incompressible bidirectional round-trip" do
      size = 1024 * 1024
      prng = :rand.seed_s(:exsss, {12345, 67890, 54321})

      {random_bytes, _} =
        Enum.reduce(1..div(size, 4), {<<>>, prng}, fn _, {acc, state} ->
          {val, next_state} = :rand.uniform_s(0xFFFFFFFF, state)
          {<<acc::binary, val::32-little>>, next_state}
        end)

      assert byte_size(random_bytes) == size

      # 1. Compress with Zip.raw_deflate, decompress with :zlib
      zip_deflated = Zip.raw_deflate(random_bytes)
      zlib_out = zlib_raw_inflate(zip_deflated)
      assert zlib_out == random_bytes

      # 2. Decompress with Zip.raw_inflate_counted
      result = Zip.raw_inflate_counted(zip_deflated, size + 1024)
      assert result.output == random_bytes
      assert result.bytes_consumed == byte_size(zip_deflated)

      # 3. Independent compression with :zlib and decompression with Zip.raw_inflate_counted
      zlib_deflated = zlib_raw_deflate(random_bytes)
      z_result = Zip.raw_inflate_counted(zlib_deflated, size + 1024)
      assert z_result.output == random_bytes
      assert z_result.bytes_consumed == byte_size(zlib_deflated)
    end
  end

  # ─── 5. 10,000-Iteration Deterministic PRNG Mutation Fuzz Loop ──────────────

  describe "10,000-iteration deterministic PRNG mutation fuzz loop" do
    test "fuzz raw_inflate_counted with 10,000 adversarial bit/byte mutations" do
      base_inputs = [
        Zip.raw_deflate("Hello, world! This is a test string for fuzzing DEFLATE."),
        Zip.raw_deflate(:binary.copy(<<0x42>>, 200)),
        Zip.raw_deflate(for(i <- 0..60, into: <<>>, do: <<rem(i * 13, 256)>>))
      ]

      initial_seed = 0xCAFEBABEDEAD1234
      total_iterations = 10_000
      valid_codes = MapSet.new(Zip.raw_inflate_error_codes())

      fuzz_loop(0, total_iterations, initial_seed, base_inputs, valid_codes)
    end
  end

  # ─── 6. Constants and Edge Branch Coverage ──────────────────────────────────

  describe "constants and edge branch coverage" do
    test "all constants exposed" do
      assert Constants.local_sig() == 0x04034B50
      assert Constants.cd_sig() == 0x02014B50
      assert Constants.eocd_sig() == 0x06054B50
      assert Constants.flags() == 0x0800
      assert Constants.ver_deflate() == 20
      assert Constants.ver_stored() == 10
      assert Constants.ver_made_by() == 0x031E
      assert Constants.dos_epoch() == 0x00210000
      assert Constants.method_stored() == 0
      assert Constants.method_deflate() == 8
      assert Constants.raw_inflate_max_output() == 268_435_456
      assert Constants.png_ceiling_output() == 134_234_112
      assert Constants.btype_stored() == 0
      assert Constants.btype_fixed() == 1
      assert Constants.btype_dynamic() == 2
      assert Constants.btype_reserved() == 3
      assert Constants.min_match_len() == 3
      assert Constants.max_match_len() == 258
      assert Constants.max_distance() == 32768
      assert Constants.window_size() == 32768
      assert Constants.hash_table_size() == 8192
      assert Constants.hash_mask() == 8191
      assert Constants.eob_symbol() == 256
      assert Constants.max_ll_codes() == 286
      assert length(Constants.code_length_order()) == 19
      assert Constants.page_size() == 4096
      assert Constants.chunk_buffer_size() == 65536
      assert Constants.eocd_min_size() == 22
      assert Constants.eocd_max_comment() == 65535
      assert Constants.max_entry_count() == 65535
      assert Constants.local_header_fixed_size() == 30
      assert Constants.cd_entry_fixed_size() == 46
      assert Constants.crc32_poly() == 0xEDB88320
      assert Constants.crc32_initial() == 0xFFFFFFFF
      assert Constants.mask_8() == 0xFF
      assert Constants.mask_16() == 0xFFFF
      assert Constants.mask_32() == 0xFFFFFFFF
      assert Constants.mask_64() == 0xFFFFFFFFFFFFFFFF
      assert Constants.bits_64() == 64
      assert Constants.bits_32() == 32
      assert Constants.bits_16() == 16
      assert Constants.bits_8() == 8
      assert Constants.dos_base_year() == 1980
      assert Constants.dos_year_shift() == 25
      assert Constants.dos_month_shift() == 21
      assert Constants.dos_day_shift() == 16
      assert Constants.dos_hour_shift() == 11
      assert Constants.dos_minute_shift() == 5
      assert Constants.dos_second_div() == 2
      assert Constants.fast_table_size() == 512
      assert Constants.fast_table_bits() == 9
      assert Constants.fast_table_mask() == 0x1FF
      assert Constants.max_pending() == 512
      assert Constants.stored_header_size() == 4
      assert Constants.local_offset_method() == 8
      assert Constants.local_offset_datetime() == 10
      assert Constants.local_offset_crc32() == 14
      assert Constants.local_offset_comp_size() == 18
      assert Constants.local_offset_uncomp_size() == 22
      assert Constants.local_offset_name_len() == 26
      assert Constants.local_offset_extra_len() == 28
      assert Constants.cd_offset_version_made_by() == 4
      assert Constants.cd_offset_version_needed() == 6
      assert Constants.cd_offset_flags() == 8
      assert Constants.cd_offset_method() == 10
      assert Constants.cd_offset_datetime() == 12
      assert Constants.cd_offset_crc32() == 16
      assert Constants.cd_offset_comp_size() == 20
      assert Constants.cd_offset_uncomp_size() == 24
      assert Constants.cd_offset_name_len() == 28
      assert Constants.cd_offset_extra_len() == 30
      assert Constants.cd_offset_comment_len() == 32
      assert Constants.cd_offset_disk_start() == 34
      assert Constants.cd_offset_internal_attr() == 36
      assert Constants.cd_offset_external_attr() == 38
      assert Constants.cd_offset_local_header_offset() == 42
      assert Constants.eocd_offset_disk() == 4
      assert Constants.eocd_offset_start_disk() == 6
      assert Constants.eocd_offset_total_entries_disk() == 8
      assert Constants.eocd_offset_total_entries() == 10
      assert Constants.eocd_offset_cd_size() == 12
      assert Constants.eocd_offset_cd_offset() == 16
      assert Constants.eocd_offset_comment_len() == 20
      assert is_map(Constants.fixed_ll_decoder())
      assert is_map(Constants.fixed_dist_decoder())

      assert {:error, :invalid_literal_length_symbol} =
               Constants.build_huffman_decoder([16])

      assert {:error, :huffman_oversubscribed} =
               Constants.build_huffman_decoder([1, 1, 1, 1])

      all_atoms = [
        :invalid_output_limit,
        :unexpected_eof,
        :reserved_block_type,
        :stored_length_mismatch,
        :huffman_oversubscribed,
        :incomplete_code_length_tree,
        :incomplete_literal_length_tree,
        :incomplete_distance_tree,
        :repeat_without_previous,
        :repeat_overrun,
        :invalid_literal_length_symbol,
        :reserved_distance_symbol,
        :invalid_back_reference,
        :output_limit_exceeded,
        :unknown_fallback
      ]

      for a <- all_atoms do
        assert is_binary(Messages.inflate_error_code(a))
      end

      assert is_binary(Messages.zip_moduledoc())
      assert is_binary(Messages.constants_moduledoc())
    end

    test "zip with 3-element entry tuple" do
      archive = Zip.zip([{"opt.txt", "uncompressed", [compress: false]}])
      assert Zip.unzip(archive) == %{"opt.txt" => "uncompressed"}
    end

    test "dos_datetime calculation" do
      dt = Zip.dos_datetime(2026, 11, 15, 12, 30, 40)
      assert dt > 0
      assert Zip.dos_epoch() == 0x00210000
    end

    test "corrupted local header out of bounds" do
      archive = Zip.zip([{"a.txt", "hello"}])
      eocd_pos = byte_size(archive) - 22
      cd_offset = :binary.decode_unsigned(binary_part(archive, eocd_pos + 16, 4), :little)
      bogus_offset = byte_size(archive) + 100

      corrupted =
        binary_part(archive, 0, cd_offset + 42) <>
          <<bogus_offset::little-32>> <>
          binary_part(archive, cd_offset + 46, byte_size(archive) - (cd_offset + 46))

      assert_raise RuntimeError, ~r/local header out of bounds/, fn ->
        Zip.unzip(corrupted)
      end
    end

    test "corrupted local header truncated fields" do
      archive = Zip.zip([{"a.txt", "hello"}])
      eocd_pos = byte_size(archive) - 22
      cd_offset = :binary.decode_unsigned(binary_part(archive, eocd_pos + 16, 4), :little)
      near_end_offset = byte_size(archive) - 10

      corrupted =
        binary_part(archive, 0, cd_offset + 42) <>
          <<near_end_offset::little-32>> <>
          binary_part(archive, cd_offset + 46, byte_size(archive) - (cd_offset + 46))

      assert_raise RuntimeError, ~r/local header/, fn ->
        Zip.unzip(corrupted)
      end
    end

    test "corrupted compressed data out of bounds" do
      archive = Zip.zip([{"a.txt", "hello"}])
      eocd_pos = byte_size(archive) - 22
      cd_offset = :binary.decode_unsigned(binary_part(archive, eocd_pos + 16, 4), :little)

      corrupted =
        binary_part(archive, 0, cd_offset + 20) <>
          <<999_999::little-32>> <>
          binary_part(archive, cd_offset + 24, byte_size(archive) - (cd_offset + 24))

      assert_raise RuntimeError, ~r/compressed data out of bounds/, fn ->
        Zip.unzip(corrupted)
      end
    end

    test "EOCD signature not found when archive is truncated" do
      truncated = <<Constants.eocd_sig()::little-32, 0::size(6)-unit(8)>>

      assert_raise RuntimeError, ~r/EOCD signature not found/, fn ->
        Zip.new_reader(truncated)
      end
    end

    test "CD region extends into EOCD" do
      eocd = <<
        Constants.eocd_sig()::little-32,
        0::little-16,
        0::little-16,
        1::little-16,
        1::little-16,
        10::little-32,
        0::little-32,
        0::little-16
      >>

      padded = <<0::size(5)-unit(8)>> <> eocd

      assert_raise RuntimeError, ~r/CD region extends into EOCD/, fn ->
        Zip.new_reader(padded)
      end
    end

    test "CD entry header out of bounds" do
      # Archive with total size 32 bytes (cd 10 bytes + eocd 22 bytes)
      # pos = 0, pos + 46 = 46 > data_size 32
      eocd = <<
        Constants.eocd_sig()::little-32,
        0::little-16,
        0::little-16,
        1::little-16,
        1::little-16,
        10::little-32,
        0::little-32,
        0::little-16
      >>

      cd = <<0::size(10)-unit(8)>>

      assert_raise RuntimeError, ~r/CD entry header out of bounds/, fn ->
        Zip.new_reader(cd <> eocd)
      end
    end

    test "expected CD signature mismatch" do
      eocd = <<
        Constants.eocd_sig()::little-32,
        0::little-16,
        0::little-16,
        1::little-16,
        1::little-16,
        46::little-32,
        0::little-32,
        0::little-16
      >>

      cd = <<0x12345678::little-32, 0::size(42)-unit(8)>>

      assert_raise RuntimeError, ~r/expected CD signature/, fn ->
        Zip.new_reader(cd <> eocd)
      end
    end

    test "CD entry name extends beyond archive" do
      cd_record = <<
        Constants.cd_sig()::little-32,
        0::little-16,
        Constants.ver_stored()::little-16,
        0::little-16,
        0::little-16,
        0::little-32,
        0::little-32,
        0::little-32,
        0::little-32,
        100::little-16,
        0::little-16,
        0::little-16,
        0::little-16,
        0::little-16,
        0::little-32,
        0::little-32
      >>

      eocd = <<
        Constants.eocd_sig()::little-32,
        0::little-16,
        0::little-16,
        1::little-16,
        1::little-16,
        byte_size(cd_record)::little-32,
        0::little-32,
        0::little-16
      >>

      assert_raise RuntimeError, ~r/CD entry name extends beyond archive/, fn ->
        Zip.new_reader(cd_record <> eocd)
      end
    end

    test "CD entry advance out of bounds" do
      # CD size 50, but comment_len is 20 -> next_pos 67 > cd_base + cd_size (50)
      cd_record = <<
        Constants.cd_sig()::little-32,
        0::little-16,
        Constants.ver_stored()::little-16,
        0::little-16,
        0::little-16,
        0::little-32,
        0::little-32,
        0::little-32,
        0::little-32,
        1::little-16,
        0::little-16,
        20::little-16,
        0::little-16,
        0::little-16,
        0::little-32,
        0::little-32,
        ?a
      >>

      padding = <<0::size(3)-unit(8)>>

      eocd = <<
        Constants.eocd_sig()::little-32,
        0::little-16,
        0::little-16,
        1::little-16,
        1::little-16,
        50::little-32,
        0::little-32,
        0::little-16
      >>

      assert_raise RuntimeError, ~r/CD entry advance out of bounds/, fn ->
        Zip.new_reader(cd_record <> padding <> eocd)
      end
    end

    test "Latin-1 encoding fallback in entry name" do
      latin1_name = <<?f, ?i, ?l, ?e, 0xE9, ?., ?t, ?x, ?t>>

      cd_record =
        <<
          Constants.cd_sig()::little-32,
          0::little-16,
          Constants.ver_stored()::little-16,
          0::little-16,
          0::little-16,
          0::little-32,
          0::little-32,
          0::little-32,
          0::little-32,
          byte_size(latin1_name)::little-16,
          0::little-16,
          0::little-16,
          0::little-16,
          0::little-16,
          0::little-32,
          0::little-32
        >> <> latin1_name

      eocd = <<
        Constants.eocd_sig()::little-32,
        0::little-16,
        0::little-16,
        1::little-16,
        1::little-16,
        byte_size(cd_record)::little-32,
        0::little-32,
        0::little-16
      >>

      reader = Zip.new_reader(cd_record <> eocd)
      [entry] = Zip.reader_entries(reader)
      assert String.contains?(entry.name, "file")
    end

    test "dotdot path variations" do
      writer = Zip.new_writer()

      assert_raise RuntimeError, ~r/path traversal/, fn ->
        Zip.add_file(writer, "..", "data")
      end

      assert_raise RuntimeError, ~r/path traversal/, fn ->
        Zip.add_file(writer, "dir/..", "data")
      end
    end

    test "stored block output limit exceeded" do
      stored = <<0x01, 0x05, 0x00, 0xFA, 0xFF, 1, 2, 3, 4, 5>>

      err =
        assert_raise RawInflateError, fn ->
          Zip.raw_inflate_counted(stored, 2)
        end

      assert err.code == "output-limit-exceeded"
    end

    test "stored block unexpected EOF" do
      truncated_stored = <<0x01, 0x05, 0x00, 0xFA, 0xFF, 1, 2>>

      err =
        assert_raise RawInflateError, fn ->
          Zip.raw_inflate_counted(truncated_stored, 100)
        end

      assert err.code == "unexpected-eof"
    end

    test "get_byte_at and slice_output across filled 4096-byte page" do
      block1 = :binary.copy(<<7>>, 4096)
      block2 = :binary.copy(<<7>>, 10)
      raw = block1 <> block2
      compressed = Zip.raw_deflate(raw)
      assert Zip.raw_inflate(compressed) == raw
    end
  end

  # ─── Helpers ─────────────────────────────────────────────────────────────────

  defp fuzz_loop(iter, max_iter, _seed, _inputs, _valid_codes) when iter >= max_iter, do: :ok

  defp fuzz_loop(iter, max_iter, seed, inputs, valid_codes) do
    {rand_val, next_seed} = next_prng(seed)
    base_idx = rem(rand_val, length(inputs))
    base = Enum.at(inputs, base_idx)

    mutation_type = rem(rand_val >>> 8, 5)
    mutated = mutate_binary(base, mutation_type, rand_val)

    # Must never crash with unhandled exception or panic
    try do
      case Zip.raw_inflate_counted(mutated, 65536) do
        %RawInflateResult{output: out, bytes_consumed: consumed} ->
          assert is_binary(out)
          assert is_integer(consumed)
          assert consumed <= byte_size(mutated)

        other ->
          flunk("Unexpected return value from raw_inflate_counted: #{inspect(other)}")
      end
    rescue
      e in RawInflateError ->
        assert MapSet.member?(valid_codes, e.code)
    end

    fuzz_loop(iter + 1, max_iter, next_seed, inputs, valid_codes)
  end

  defp mutate_binary(<<>>, _type, _rand), do: <<0>>

  defp mutate_binary(data, type, rand) do
    len = byte_size(data)
    pos = rem(rand >>> 16, len)
    byte_val = rand &&& 0xFF

    case type do
      0 ->
        # Bit flip
        orig = :binary.at(data, pos)
        bit_idx = rem(rand >>> 24, 8)
        flipped = bxor(orig, 1 <<< bit_idx)
        replace_byte(data, pos, flipped)

      1 ->
        # Byte overwrite
        replace_byte(data, pos, byte_val)

      2 ->
        # Truncation
        binary_part(data, 0, pos)

      3 ->
        # Byte deletion
        binary_part(data, 0, pos) <> binary_part(data, pos + 1, len - pos - 1)

      4 ->
        # Byte insertion
        binary_part(data, 0, pos) <> <<byte_val>> <> binary_part(data, pos, len - pos)
    end
  end

  defp replace_byte(data, pos, val) do
    head = binary_part(data, 0, pos)
    tail = binary_part(data, pos + 1, byte_size(data) - pos - 1)
    <<head::binary, val::8, tail::binary>>
  end

  defp next_prng(seed) do
    next = seed * 6_364_136_223_846_793_005 + 1_442_695_040_888_963_407 &&& 0xFFFFFFFFFFFFFFFF
    {next, next}
  end

  defp generate_prng_binary(seed, size) do
    do_generate_prng(seed, size, <<>>)
  end

  defp do_generate_prng(_seed, 0, acc), do: acc

  defp do_generate_prng(seed, size, acc) do
    {rand_val, next_seed} = next_prng(seed)
    chunk_len = min(size, 8)
    word = rand_val &&& (1 <<< (chunk_len * 8)) - 1
    do_generate_prng(next_seed, size - chunk_len, <<acc::binary, word::size(chunk_len)-unit(8)>>)
  end

  defp zlib_raw_inflate(compressed) do
    z = :zlib.open()

    try do
      :zlib.inflateInit(z, -15)
      decompressed = :zlib.inflate(z, compressed)
      :zlib.inflateEnd(z)
      IO.iodata_to_binary(decompressed)
    after
      :zlib.close(z)
    end
  end

  defp zlib_raw_deflate(data) do
    z = :zlib.open()

    try do
      :zlib.deflateInit(z, :default, :deflated, -15, 8, :default)
      compressed = :zlib.deflate(z, data, :finish)
      :zlib.deflateEnd(z)
      IO.iodata_to_binary(compressed)
    after
      :zlib.close(z)
    end
  end
end
