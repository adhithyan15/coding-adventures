defmodule CodingAdventures.ImageCodecPngTest do
  use ExUnit.Case, async: true

  import Bitwise
  alias CodingAdventures.ImageCodecPng
  alias CodingAdventures.ImageCodecPng.Constants
  alias CodingAdventures.ImageCodecPng.Messages
  alias CodingAdventures.ImageCodecPng.PngError
  alias CodingAdventures.PixelContainer

  @fixture_path Path.expand("../../../../specs/fixtures/image-codec-png-v1/cases.json", __DIR__)
  @max_fixture_bytes 262_144
  @prng_seed 0x5EEDC0DE

  # ---------------------------------------------------------------------------
  # Bounded Fixture Reader & Cases Replay
  # ---------------------------------------------------------------------------

  describe "FIPS/IC18 Neutral Conformance Suite (cases.json)" do
    test "replays all 85 fixture cases with bounded reader" do
      # 1. Single-pass bounded read eliminating TOCTOU gap
      raw_bytes =
        File.open!(@fixture_path, [:read, :binary], fn file ->
          IO.binread(file, @max_fixture_bytes + 1)
        end)

      assert is_binary(raw_bytes) and byte_size(raw_bytes) <= @max_fixture_bytes

      # 2. Strict stdlib JSON decode (zero Hex deps)
      fixtures = :json.decode(raw_bytes)
      cases = Map.fetch!(fixtures, "cases")
      limits = Map.fetch!(fixtures, "limits")
      error_ids = Map.fetch!(fixtures, "error_ids")

      assert length(cases) == 85
      assert limits["max_dimension"] == ImageCodecPng.max_dimension()
      assert limits["default_max_pixels"] == ImageCodecPng.max_pixels()
      assert error_ids == ImageCodecPng.error_codes()

      for case_data <- cases do
        _case_id = Map.fetch!(case_data, "id")
        operation = Map.fetch!(case_data, "operation")
        expected = Map.fetch!(case_data, "expected")
        options = Map.get(case_data, "options")

        opts =
          if options && Map.has_key?(options, "max_pixels") do
            [max_pixels: options["max_pixels"]]
          else
            []
          end

        case operation do
          "decode" ->
            png_bytes = Base.decode16!(case_data["png_hex"], case: :lower)
            container = ImageCodecPng.decode_png(png_bytes, opts)
            assert container.width == expected["width"]
            assert container.height == expected["height"]
            actual_hex = Base.encode16(container.data, case: :lower)
            assert actual_hex == expected["rgba_hex"]

            # Also verify ImageCodec behaviour decode/1 callback
            if opts == [] do
              assert {:ok, ^container} = ImageCodecPng.decode(png_bytes)
            end

          "decode-error" ->
            png_bytes = Base.decode16!(case_data["png_hex"], case: :lower)
            expected_id = expected["error_id"]

            err =
              assert_raise PngError, fn ->
                ImageCodecPng.decode_png(png_bytes, opts)
              end

            assert err.code == expected_id

            if opts == [] do
              assert {:error, ^expected_id} = ImageCodecPng.decode(png_bytes)
            end

          "encode" ->
            input = Map.fetch!(case_data, "input")
            rgba_bytes = Base.decode16!(input["rgba_hex"], case: :lower)

            container = %PixelContainer{
              width: input["width"],
              height: input["height"],
              data: rgba_bytes
            }

            encoded = ImageCodecPng.encode_png(container)
            chunks = parse_chunks(encoded)

            chunk_types = Enum.map(chunks, fn {type, _} -> type end)
            assert chunk_types == expected["chunk_types"]

            # Bit depth 8, colour type 6, interlace 0 in IHDR
            <<_w::32, _h::32, depth, col_type, _comp, _filt, interlace>> =
              hd(Enum.filter(chunks, fn {t, _} -> t == "IHDR" end)) |> elem(1)

            assert depth == expected["bit_depth"]
            assert col_type == expected["colour_type"]
            assert interlace == expected["interlace"]

            # Foreign zlib check in test: inflate IDAT
            idat_payloads =
              Enum.filter(chunks, fn {t, _} -> t == "IDAT" end)
              |> Enum.map(&elem(&1, 1))
              |> IO.iodata_to_binary()

            z = :zlib.open()
            :zlib.inflateInit(z)
            decompressed = :zlib.inflate(z, idat_payloads)
            :zlib.inflateEnd(z)
            :zlib.close(z)
            filtered = IO.iodata_to_binary(decompressed)

            stride = input["width"] * 4
            row_size = stride + 1

            actual_filter_types =
              for y <- 0..(input["height"] - 1)//1 do
                :binary.at(filtered, y * row_size)
              end

            assert actual_filter_types == expected["filter_types"]

            # Re-decode encoded PNG to ensure full round-trip fidelity
            decoded_container = ImageCodecPng.decode_png(encoded)
            assert decoded_container.width == input["width"]
            assert decoded_container.height == input["height"]
            assert decoded_container.data == rgba_bytes

          "encode-error" ->
            input = Map.fetch!(case_data, "input")
            rgba_bytes = Base.decode16!(input["rgba_hex"], case: :lower)
            expected_id = expected["error_id"]

            container = %PixelContainer{
              width: input["width"],
              height: input["height"],
              data: rgba_bytes
            }

            err =
              assert_raise PngError, fn ->
                ImageCodecPng.encode_png(container)
              end

            assert err.code == expected_id

          "adler32" ->
            input_bytes = Base.decode16!(case_data["input_hex"], case: :lower)
            actual_adler = ImageCodecPng.adler32(input_bytes)
            expected_adler = String.to_integer(expected["adler32_hex"], 16)
            assert actual_adler == expected_adler

            # Cross check with Erlang :erlang.adler32
            assert actual_adler == :erlang.adler32(input_bytes)
        end
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Behaviour & Module Surface
  # ---------------------------------------------------------------------------

  describe "ImageCodec behaviour and module surface" do
    test "mime_type, limits, and error taxonomy" do
      assert ImageCodecPng.mime_type() == "image/png"
      assert ImageCodecPng.max_dimension() == 16_384
      assert ImageCodecPng.max_pixels() == 33_554_432
      assert length(ImageCodecPng.error_codes()) == 29

      for code <- ImageCodecPng.error_codes() do
        msg = Messages.message_for(code)
        assert is_binary(msg)
        assert String.starts_with?(msg, "image-codec-png:")
        atom = String.replace(code, "-", "_") |> String.to_atom()
        assert Messages.error_code(atom) == code
        assert Messages.message_for(atom) == msg
      end

      # Unknown error codes fallback payload-blindly
      assert Messages.error_code(:something_unrecognized) == "unknown-error"
      assert Messages.error_code("some-random-string") == "unknown-error"
      assert Messages.error_code(nil) == "unknown-error"
      assert Messages.message_for("unknown-code") == "image-codec-png: unknown error code"
      assert Messages.message_for(:invalid_atom) == "image-codec-png: unknown error code"
      assert Messages.message_for(nil) == "image-codec-png: unknown error code"
      assert Messages.redacted_placeholder() == "<redacted>"
      assert Messages.invalid_placeholder() == "<invalid>"

      # PngError formatting and closed security resolution
      e = PngError.exception(:invalid_signature)
      assert e.code == "invalid-signature"
      assert e.message == "image-codec-png: invalid PNG signature"

      # Untrusted caller code falls back to unknown error code
      e_untrusted = PngError.exception(code: "untrusted")
      assert e_untrusted.code == "unknown-error"
      assert e_untrusted.message == "image-codec-png: unknown error code"

      # Non-keyword list or malformed opts gracefully handled
      e_bad = PngError.exception([:bad])
      assert e_bad.code == "unknown-error"
      assert e_bad.message == "image-codec-png: unknown error code"

      e_nil = PngError.exception(nil)
      assert e_nil.code == "unknown-error"

      e_binary = PngError.exception("invalid-signature")
      assert e_binary.code == "invalid-signature"

      e_atom_other = PngError.exception(:unknown_atom_reason)
      assert e_atom_other.code == "unknown-error"

      # ImageCodec behaviour encode/1 and decode/1
      pc = %PixelContainer{width: 1, height: 1, data: <<1, 2, 3, 4>>}
      assert is_binary(ImageCodecPng.encode(pc))
      assert {:error, "file-too-short"} = ImageCodecPng.decode(<<>>)

      # Encoder error validations
      assert_raise PngError, fn -> ImageCodecPng.encode_png(:not_a_container) end

      # Exercise all Constants getters
      assert is_binary(Constants.png_signature())
      assert Constants.signature_length() == 8
      assert Constants.chunk_length_size() == 4
      assert Constants.chunk_type_size() == 4
      assert Constants.chunk_header_size() == 8
      assert Constants.chunk_crc_size() == 4
      assert Constants.chunk_overhead() == 12
      assert Constants.ancillary_bit_mask() == 0x20
      assert Constants.zlib_cinfo_shift() == 4
      assert Constants.average_divisor() == 2
      assert is_binary(Constants.chunk_ihdr())
      assert is_binary(Constants.chunk_plte())
      assert is_binary(Constants.chunk_trns())
      assert is_binary(Constants.chunk_idat())
      assert is_binary(Constants.chunk_iend())
      assert is_list(Constants.apng_chunks())
      assert Constants.png_max_dimension() == 16_384
      assert Constants.png_max_pixels() == 33_554_432
      assert Constants.default_max_pixels() == 33_554_432
      assert Constants.ihdr_length() == 13
      assert Constants.plte_min_length() == 3
      assert Constants.plte_max_length() == 768
      assert Constants.plte_entry_size() == 3
      assert Constants.trns_grey_length() == 2
      assert Constants.trns_rgb_length() == 6
      assert Constants.max_sample_value() == 255
      assert Constants.colour_type_grey() == 0
      assert Constants.colour_type_rgb() == 2
      assert Constants.colour_type_palette() == 3
      assert Constants.colour_type_grey_alpha() == 4
      assert Constants.colour_type_rgba() == 6
      assert is_list(Constants.supported_colour_types())
      assert Constants.channels_grey() == 1
      assert Constants.channels_rgb() == 3
      assert Constants.channels_grey_alpha() == 2
      assert Constants.channels_rgba() == 4
      assert Constants.channels_for_colour_type(0) == 1
      assert Constants.channels_for_colour_type(2) == 3
      assert Constants.channels_for_colour_type(4) == 2
      assert Constants.channels_for_colour_type(6) == 4
      assert is_nil(Constants.channels_for_colour_type(99))
      assert Constants.supported_bit_depth() == 8
      assert Constants.compression_method_deflate() == 0
      assert Constants.filter_method_adaptive() == 0
      assert Constants.interlace_method_none() == 0
      assert Constants.filter_none() == 0
      assert Constants.filter_sub() == 1
      assert Constants.filter_up() == 2
      assert Constants.filter_average() == 3
      assert Constants.filter_paeth() == 4
      assert Constants.filter_count() == 5
      assert Constants.max_filter_type() == 4
      assert Constants.zlib_min_stream_size() == 6
      assert Constants.zlib_cmf_deflate() == 0x78
      assert Constants.zlib_flg_default() == 0x9C
      assert Constants.zlib_method_deflate() == 8
      assert Constants.zlib_cinfo_max() == 7
      assert Constants.zlib_cmf_method_mask() == 0x0F
      assert Constants.zlib_fcheck_modulus() == 31
      assert Constants.zlib_fdict_mask() == 0x20
      assert Constants.zlib_header_size() == 2
      assert Constants.zlib_trailer_size() == 4
      assert Constants.zlib_overhead() == 6
      assert Constants.adler_initial_a() == 1
      assert Constants.adler_initial_b() == 0
      assert Constants.adler_base() == 65_521
      assert Constants.adler_chunk_size() == 5552
      assert Constants.adler_b_shift() == 16
      assert Constants.uint32_mask() == 0xFFFFFFFF
      assert Constants.byte_mask() == 0xFF
      assert Constants.byte_midpoint() == 128
      assert Constants.byte_modulus() == 256
      assert Constants.bits_per_byte() == 8
      assert Constants.bytes_per_rgba_pixel() == 4
      assert Constants.transparent_alpha() == 0
      assert Constants.opaque_alpha() == 255
    end
  end

  # ---------------------------------------------------------------------------
  # 6-Class PNG CVE Resilience Showcase
  # ---------------------------------------------------------------------------

  describe "6-Class PNG CVE Resilience Showcase" do
    # Helper to construct valid minimal PNG bytes
    defp make_minimal_png(w \\ 1, h \\ 1, filter \\ 0, rgba \\ <<255, 0, 0, 255>>) do
      ihdr = <<w::32, h::32, 8, 6, 0, 0, 0>>
      scanline = <<filter, rgba::binary>>
      adler = ImageCodecPng.adler32(scanline)
      deflated = CodingAdventures.Zip.raw_deflate(scanline)
      idat = <<0x78, 0x9C, deflated::binary, adler::32>>

      sig = Constants.png_signature()
      ihdr_chunk = make_raw_chunk(Constants.chunk_ihdr(), ihdr)
      idat_chunk = make_raw_chunk(Constants.chunk_idat(), idat)
      iend_chunk = make_raw_chunk(Constants.chunk_iend(), <<>>)

      <<sig::binary, ihdr_chunk::binary, idat_chunk::binary, iend_chunk::binary>>
    end

    defp make_raw_chunk(type, payload) do
      len = byte_size(payload)
      crc = CodingAdventures.Zip.crc32(<<type::binary-size(4), payload::binary>>)
      <<len::32, type::binary-size(4), payload::binary, crc::32>>
    end

    test "Class 1: Decompression & Zip Bombs (CVE-2002-1363, CVE-2023-4863)" do
      # IDAT decompresses to more data than 1x1 image promises (5 bytes)
      huge_scanline = :binary.copy(<<0>>, 1000)
      huge_deflated = CodingAdventures.Zip.raw_deflate(huge_scanline)
      adler = ImageCodecPng.adler32(huge_scanline)
      bad_idat = <<0x78, 0x9C, huge_deflated::binary, adler::32>>

      sig = Constants.png_signature()
      ihdr_chunk = make_raw_chunk(Constants.chunk_ihdr(), <<1::32, 1::32, 8, 6, 0, 0, 0>>)
      idat_chunk = make_raw_chunk(Constants.chunk_idat(), bad_idat)
      iend_chunk = make_raw_chunk(Constants.chunk_iend(), <<>>)
      png = <<sig::binary, ihdr_chunk::binary, idat_chunk::binary, iend_chunk::binary>>

      err =
        assert_raise PngError, fn ->
          ImageCodecPng.decode_png(png)
        end

      assert err.code == "inflated-length-mismatch"
    end

    test "Class 2: Dimension & Pixel Limits (CVE-2004-0597, CVE-2019-7317)" do
      sig = Constants.png_signature()

      # Edge limit exceeded (16,385)
      bad_edge = make_raw_chunk(Constants.chunk_ihdr(), <<16_385::32, 1::32, 8, 6, 0, 0, 0>>)

      png1 =
        <<sig::binary, bad_edge::binary, make_raw_chunk(Constants.chunk_iend(), <<>>)::binary>>

      err1 = assert_raise PngError, fn -> ImageCodecPng.decode_png(png1) end
      assert err1.code == "dimension-limit"

      # Pixel product limit exceeded (10,000 x 4,000 = 40M > 33.5M)
      bad_product =
        make_raw_chunk(Constants.chunk_ihdr(), <<10_000::32, 4_000::32, 8, 6, 0, 0, 0>>)

      png2 =
        <<sig::binary, bad_product::binary, make_raw_chunk(Constants.chunk_iend(), <<>>)::binary>>

      err2 = assert_raise PngError, fn -> ImageCodecPng.decode_png(png2) end
      assert err2.code == "pixel-limit"

      # Caller pixel ceiling validations
      invalid_ceilings = [nil, 0, -1, 33_554_433, :infinity, "500", 1.5]

      for val <- invalid_ceilings do
        err =
          assert_raise PngError, fn ->
            ImageCodecPng.decode_png(<<>>, max_pixels: val)
          end

        assert err.code == "invalid-max-pixels"
      end

      # Non-keyword options fail closed with invalid-max-pixels
      for bad_opts <- [[:bad], %{}, 123, "invalid"] do
        err = assert_raise PngError, fn -> ImageCodecPng.decode_png(<<>>, bad_opts) end
        assert err.code == "invalid-max-pixels"
      end

      # Non-binary payload to decode/1 returns {:error, "file-too-short"}
      assert ImageCodecPng.decode(nil) == {:error, "file-too-short"}
      assert ImageCodecPng.decode(12345) == {:error, "file-too-short"}
    end

    test "Class 3: Chunk-Length Truncation & Pre-Allocation OOM (CVE-2018-14550)" do
      sig = Constants.png_signature()
      # Declares 2 GB chunk length with only small header bytes
      fake_chunk = <<0x7FFFFFFF::32, "IDAT">>
      png = <<sig::binary, fake_chunk::binary>>

      err = assert_raise PngError, fn -> ImageCodecPng.decode_png(png) end
      assert err.code == "truncated-chunk"
    end

    test "Class 4: Checksum Tampering, IDAT Cavities & Trailing Data (CVE-2010-1205)" do
      base = make_minimal_png()
      sig = Constants.png_signature()

      # 1. Tampered CRC in IEND
      <<head::binary-size(byte_size(base) - 4), crc::32>> = base
      tampered_crc_png = <<head::binary, bxor(crc, 1)::32>>
      err1 = assert_raise PngError, fn -> ImageCodecPng.decode_png(tampered_crc_png) end
      assert err1.code == "chunk-crc-mismatch"

      # 2. Trailing data after IEND
      trailing_png = <<base::binary, "EXTRA_BYTES">>
      err2 = assert_raise PngError, fn -> ImageCodecPng.decode_png(trailing_png) end
      assert err2.code == "trailing-data"

      # 3. IDAT Cavity: unconsumed bytes between deflate stream and Adler
      scanline = <<0, 255, 0, 0, 255>>
      deflated = CodingAdventures.Zip.raw_deflate(scanline)
      adler = ImageCodecPng.adler32(scanline)
      cavity_idat = <<0x78, 0x9C, deflated::binary, "CAVITY_BYTES", adler::32>>
      ihdr_chunk = make_raw_chunk(Constants.chunk_ihdr(), <<1::32, 1::32, 8, 6, 0, 0, 0>>)
      idat_chunk = make_raw_chunk(Constants.chunk_idat(), cavity_idat)
      iend_chunk = make_raw_chunk(Constants.chunk_iend(), <<>>)
      cavity_png = <<sig::binary, ihdr_chunk::binary, idat_chunk::binary, iend_chunk::binary>>

      err3 = assert_raise PngError, fn -> ImageCodecPng.decode_png(cavity_png) end
      assert err3.code == "idat-cavity"

      # 4. Non-consecutive IDAT chunks
      valid_idat = <<0x78, 0x9C, deflated::binary, adler::32>>
      idat1 = make_raw_chunk(Constants.chunk_idat(), valid_idat)
      ancillary = make_raw_chunk(<<?t, ?E, ?X, ?t>>, <<0>>)
      idat2 = make_raw_chunk(Constants.chunk_idat(), valid_idat)

      nonconsec =
        IO.iodata_to_binary([
          sig,
          ihdr_chunk,
          idat1,
          ancillary,
          idat2,
          iend_chunk
        ])

      err4 = assert_raise PngError, fn -> ImageCodecPng.decode_png(nonconsec) end
      assert err4.code == "nonconsecutive-idat"
    end

    test "Class 5: APNG Refusal & Unknown Chunks" do
      sig = Constants.png_signature()
      ihdr = make_raw_chunk(Constants.chunk_ihdr(), <<1::32, 1::32, 8, 6, 0, 0, 0>>)
      iend = make_raw_chunk(Constants.chunk_iend(), <<>>)

      # APNG acTL, fcTL, fdAT must be refused with unsupported-feature
      for apng_tag <- [<<?a, ?c, ?T, ?L>>, <<?f, ?c, ?T, ?L>>, <<?f, ?d, ?A, ?T>>] do
        apng_chunk = make_raw_chunk(apng_tag, <<0, 0, 0, 1>>)
        png = <<sig::binary, ihdr::binary, apng_chunk::binary, iend::binary>>

        err = assert_raise PngError, fn -> ImageCodecPng.decode_png(png) end
        assert err.code == "unsupported-feature"
      end

      # Unknown critical chunk (uppercase first letter)
      unknown_crit = make_raw_chunk(<<?C, ?R, ?I, ?T>>, <<0, 1, 2, 3>>)
      png_crit = <<sig::binary, ihdr::binary, unknown_crit::binary, iend::binary>>
      err_crit = assert_raise PngError, fn -> ImageCodecPng.decode_png(png_crit) end
      assert err_crit.code == "unknown-critical-chunk"

      # Unknown ancillary chunk (lowercase first letter) is skipped safely
      unknown_anc = make_raw_chunk(<<?a, ?N, ?C, ?I>>, <<0, 1, 2, 3>>)
      valid_png = make_minimal_png()
      <<s::binary-size(8), ihdr_c::binary-size(25), rest::binary>> = valid_png
      with_anc = <<s::binary, ihdr_c::binary, unknown_anc::binary, rest::binary>>
      assert %PixelContainer{} = ImageCodecPng.decode_png(with_anc)
    end

    test "Class 6: Filter Arithmetic, PLTE & tRNS Semantics" do
      sig = Constants.png_signature()

      # Invalid filter type > 4
      ihdr = make_raw_chunk(Constants.chunk_ihdr(), <<1::32, 1::32, 8, 6, 0, 0, 0>>)
      scanline = <<5, 10, 20, 30, 255>>
      adler = ImageCodecPng.adler32(scanline)
      deflated = CodingAdventures.Zip.raw_deflate(scanline)
      idat = make_raw_chunk(Constants.chunk_idat(), <<0x78, 0x9C, deflated::binary, adler::32>>)
      iend = make_raw_chunk(Constants.chunk_iend(), <<>>)
      png_filter = <<sig::binary, ihdr::binary, idat::binary, iend::binary>>

      err1 = assert_raise PngError, fn -> ImageCodecPng.decode_png(png_filter) end
      assert err1.code == "invalid-filter"

      # Misplaced PLTE after IDAT
      plte = make_raw_chunk(Constants.chunk_plte(), <<1, 2, 3>>)
      png_plte = <<sig::binary, ihdr::binary, idat::binary, plte::binary, iend::binary>>
      err2 = assert_raise PngError, fn -> ImageCodecPng.decode_png(png_plte) end
      assert err2.code == "invalid-plte"

      # Misplaced tRNS after IDAT
      trns = make_raw_chunk(Constants.chunk_trns(), <<0, 1>>)
      png_trns = <<sig::binary, ihdr::binary, idat::binary, trns::binary, iend::binary>>
      err3 = assert_raise PngError, fn -> ImageCodecPng.decode_png(png_trns) end
      assert err3.code == "invalid-trns"

      # tRNS sample out of 8-bit range for colour type 0
      ihdr_grey = make_raw_chunk(Constants.chunk_ihdr(), <<1::32, 1::32, 8, 0, 0, 0, 0>>)
      bad_trns_sample = make_raw_chunk(Constants.chunk_trns(), <<256::16>>)
      png_trns_range = <<sig::binary, ihdr_grey::binary, bad_trns_sample::binary, iend::binary>>
      err4 = assert_raise PngError, fn -> ImageCodecPng.decode_png(png_trns_range) end
      assert err4.code == "invalid-trns"
    end
  end

  # ---------------------------------------------------------------------------
  # Deterministic PRNG Property Tests and 10,000-Iteration Fuzz Loop
  # ---------------------------------------------------------------------------

  describe "Deterministic PRNG property tests and mutation fuzzing" do
    defmodule Prng do
      import Bitwise

      def next_u32(state) do
        new_state =
          rem(
            state * 6_364_136_223_846_793_005 + 1_442_695_040_888_963_407,
            18_446_744_073_709_551_616
          )

        val = new_state >>> 32 &&& 0xFFFFFFFF
        {val, new_state}
      end

      def rand_int(state, min, max) when min <= max do
        {val, next_state} = next_u32(state)
        range = max - min + 1
        {min + rem(val, range), next_state}
      end

      def mutate(data, state) do
        len = byte_size(data)
        {op, s1} = rand_int(state, 0, 4)

        case op do
          0 ->
            # Bit flip
            {pos, s2} = rand_int(s1, 0, max(len - 1, 0))
            {bit, s3} = rand_int(s2, 0, 7)

            if len == 0 do
              {data, s3}
            else
              byte = :binary.at(data, pos)
              mutated = bxor(byte, 1 <<< bit)
              head = :binary.part(data, 0, pos)
              tail = :binary.part(data, pos + 1, len - pos - 1)
              {<<head::binary, mutated, tail::binary>>, s3}
            end

          1 ->
            # Truncation
            {cut, s2} = rand_int(s1, 0, len)
            {:binary.part(data, 0, cut), s2}

          2 ->
            # Byte overwrite
            {pos, s2} = rand_int(s1, 0, max(len - 1, 0))
            {byte, s3} = rand_int(s2, 0, 255)

            if len == 0 do
              {data, s3}
            else
              head = :binary.part(data, 0, pos)
              tail = :binary.part(data, pos + 1, len - pos - 1)
              {<<head::binary, byte, tail::binary>>, s3}
            end

          3 ->
            # Byte insert
            {pos, s2} = rand_int(s1, 0, len)
            {byte, s3} = rand_int(s2, 0, 255)
            head = :binary.part(data, 0, pos)
            tail = :binary.part(data, pos, len - pos)
            {<<head::binary, byte, tail::binary>>, s3}

          4 ->
            # Byte delete
            if len <= 1 do
              {<<>>, s1}
            else
              {pos, s2} = rand_int(s1, 0, len - 1)
              head = :binary.part(data, 0, pos)
              tail = :binary.part(data, pos + 1, len - pos - 1)
              {<<head::binary, tail::binary>>, s2}
            end
        end
      end
    end

    test "multi-pattern round trip across diverse dimensions" do
      test_dimensions = [
        {1, 1},
        {1, 7},
        {7, 1},
        {2, 3},
        {13, 17},
        {25, 25},
        {64, 4}
      ]

      for {w, h} <- test_dimensions do
        # Pattern 1: Solid color
        rgba1 = :binary.copy(<<200, 100, 50, 255>>, w * h)
        pc1 = %PixelContainer{width: w, height: h, data: rgba1}
        assert ImageCodecPng.decode_png(ImageCodecPng.encode_png(pc1)) == pc1

        # Pattern 2: Gradients
        rgba2 =
          for y <- 0..(h - 1)//1, x <- 0..(w - 1)//1, into: <<>> do
            r = rem(x * 255, 256)
            g = rem(y * 255, 256)
            b = rem((x + y) * 128, 256)
            <<r, g, b, 255>>
          end

        pc2 = %PixelContainer{width: w, height: h, data: rgba2}
        assert ImageCodecPng.decode_png(ImageCodecPng.encode_png(pc2)) == pc2

        # Pattern 3: Checkerboard
        rgba3 =
          for y <- 0..(h - 1)//1, x <- 0..(w - 1)//1, into: <<>> do
            v = if rem(x + y, 2) == 0, do: 255, else: 0
            <<v, v, v, 255>>
          end

        pc3 = %PixelContainer{width: w, height: h, data: rgba3}
        assert ImageCodecPng.decode_png(ImageCodecPng.encode_png(pc3)) == pc3
      end
    end

    test "10,000-iteration deterministic PRNG mutation fuzz loop fails closed" do
      seed1 =
        ImageCodecPng.encode_png(%PixelContainer{width: 1, height: 1, data: <<10, 20, 30, 40>>})

      seed2 =
        ImageCodecPng.encode_png(%PixelContainer{
          width: 4,
          height: 2,
          data: :binary.copy(<<5, 10, 15, 20>>, 8)
        })

      seed3 =
        ImageCodecPng.encode_png(%PixelContainer{
          width: 2,
          height: 5,
          data: :binary.copy(<<100, 150, 200, 255>>, 10)
        })

      seeds = [seed1, seed2, seed3]

      valid_codes = MapSet.new(ImageCodecPng.error_codes())

      Enum.reduce(1..10_000, @prng_seed, fn iter, st ->
        base = Enum.at(seeds, rem(iter, 3))
        {mutated, next_st} = Prng.mutate(base, st)

        try do
          case ImageCodecPng.decode_png(mutated) do
            %PixelContainer{width: w, height: h, data: data} = decoded ->
              assert w > 0 and h > 0
              assert byte_size(data) == w * h * 4
              # Round-trip check
              assert ImageCodecPng.decode_png(ImageCodecPng.encode_png(decoded)).data == data
          end
        rescue
          e in PngError ->
            assert MapSet.member?(valid_codes, e.code),
                   "Iteration #{iter} raised unknown error code: #{e.code}"
        end

        next_st
      end)
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp parse_chunks(png_bytes) do
    sig_len = Constants.signature_length()
    <<_sig::binary-size(sig_len), rest::binary>> = png_bytes
    do_parse_chunks(rest, [])
  end

  defp do_parse_chunks(<<>>, acc), do: Enum.reverse(acc)

  defp do_parse_chunks(<<length::32, type::binary-size(4), rest::binary>>, acc) do
    <<payload::binary-size(length), _crc::32, next_chunks::binary>> = rest
    do_parse_chunks(next_chunks, [{type, payload} | acc])
  end
end
