# frozen_string_literal: true

require_relative "test_helper"
require "zlib"

class TestCveResilience < Minitest::Test
  # Helper to construct valid minimal PNG bytes
  def make_minimal_png(width = 1, height = 1, filter_type = 0, rgba = [255, 0, 0, 255])
    ihdr = [width, height, 8, 6, 0, 0, 0].pack("NNCCCCC")
    scanline = [filter_type, *rgba].pack("CCCCC")
    adler = ImageCodecPng.adler32(scanline)
    deflated = CodingAdventures::Zip.raw_deflate(scanline)
    idat = [0x78, 0x9C].pack("CC") + deflated + [adler].pack("N")

    ihdr_chunk = make_chunk("IHDR", ihdr)
    idat_chunk = make_chunk("IDAT", idat)
    iend_chunk = make_chunk("IEND", "")

    "\x89PNG\r\n\x1a\n".b + ihdr_chunk + idat_chunk + iend_chunk
  end

  def make_chunk(type, payload)
    payload = payload.b
    crc = CodingAdventures::Zip.crc32(type.b)
    crc = CodingAdventures::Zip.crc32(payload, initial: crc) unless payload.empty?
    [payload.bytesize].pack("N") + type.b + payload + [crc].pack("N")
  end

  # 1. Dimension & Integer Overflow Bombs (CVE-2004-0597, CVE-2019-7317)
  def test_cve_dimension_and_integer_overflow_bombs
    # Edge limit exceeded (16,385)
    bad_edge = make_chunk("IHDR", [16_385, 1, 8, 6, 0, 0, 0].pack("NNCCCCC"))
    png = "\x89PNG\r\n\x1a\n".b + bad_edge + make_chunk("IEND", "")
    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png(png) }
    assert_equal "dimension-limit", err.code

    # Pixel product limit exceeded (e.g. 10,000 x 4,000 = 40,000,000 > 33,554,432)
    bad_product = make_chunk("IHDR", [10_000, 4_000, 8, 6, 0, 0, 0].pack("NNCCCCC"))
    png = "\x89PNG\r\n\x1a\n".b + bad_product + make_chunk("IEND", "")
    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png(png) }
    assert_equal "pixel-limit", err.code

    # Caller pixel ceiling validations
    invalid_limits = [true, false, Float::NAN, Float::INFINITY, -Float::INFINITY, "500", 100.5, 0, -5, 33_554_433]
    invalid_limits.each do |val|
      # Validated before file-too-short on empty input
      err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png("".b, max_pixels: val) }
      assert_equal "invalid-max-pixels", err.code

      # Validated on PngCodec constructor
      err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng::PngCodec.new(max_pixels: val) }
      assert_equal "invalid-max-pixels", err.code
    end
  end

  # 2. Decompression & Zip Bombs (CVE-2002-1363, CVE-2023-4863)
  def test_cve_decompression_zip_bombs
    # Decompression bounded strictly by promised scanline length (height * (stride + 1))
    # Create an IDAT that inflates to more bytes than the 1x1 image requires (which is 5 bytes)
    huge_scanline = "\x00" * 1000
    huge_deflated = CodingAdventures::Zip.raw_deflate(huge_scanline)
    bad_idat = [0x78, 0x9C].pack("CC") + huge_deflated + [ImageCodecPng.adler32(huge_scanline)].pack("N")

    ihdr_chunk = make_chunk("IHDR", [1, 1, 8, 6, 0, 0, 0].pack("NNCCCCC"))
    idat_chunk = make_chunk("IDAT", bad_idat)
    iend_chunk = make_chunk("IEND", "")
    png = "\x89PNG\r\n\x1a\n".b + ihdr_chunk + idat_chunk + iend_chunk

    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png(png) }
    assert_equal "inflated-length-mismatch", err.code
  end

  # 3. Chunk-Length Truncation & Pre-Allocation OOM (CVE-2018-14550)
  def test_cve_chunk_length_truncation_oom
    # Declares 2 GB chunk length with only 20 bytes of file
    fake_header = [0x7FFF_FFFF].pack("N") + "IDAT".b
    png = "\x89PNG\r\n\x1a\n".b + fake_header
    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png(png) }
    assert_equal "truncated-chunk", err.code
  end

  # 4. Paeth / Filter Signedness & Modulo Arithmetic Traps (CVE-2015-8126)
  def test_cve_filter_arithmetic_and_validation
    # Invalid filter byte > 4 (e.g. filter 5)
    ihdr = [1, 1, 8, 6, 0, 0, 0].pack("NNCCCCC")
    scanline = [5, 10, 20, 30, 255].pack("CCCCC") # filter 5 is invalid
    adler = ImageCodecPng.adler32(scanline)
    deflated = CodingAdventures::Zip.raw_deflate(scanline)
    idat = [0x78, 0x9C].pack("CC") + deflated + [adler].pack("N")

    png = "\x89PNG\r\n\x1a\n".b + make_chunk("IHDR", ihdr) + make_chunk("IDAT", idat) + make_chunk("IEND", "")
    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png(png) }
    assert_equal "invalid-filter", err.code
  end

  # 5. Checksum & Integrity Tampering, IDAT Cavities (CVE-2010-1205)
  def test_cve_checksum_tampering_and_cavities
    # Tampered chunk CRC
    base = make_minimal_png
    corrupt_crc = base.dup
    corrupt_crc.setbyte(28, corrupt_crc.getbyte(28) ^ 0xFF)
    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png(corrupt_crc) }
    assert_equal "chunk-crc-mismatch", err.code

    # Tampered Adler-32
    ihdr = [1, 1, 8, 6, 0, 0, 0].pack("NNCCCCC")
    scanline = [0, 100, 150, 200, 255].pack("CCCCC")
    adler = ImageCodecPng.adler32(scanline) ^ 0x1234_5678
    deflated = CodingAdventures::Zip.raw_deflate(scanline)
    idat = [0x78, 0x9C].pack("CC") + deflated + [adler].pack("N")
    png = "\x89PNG\r\n\x1a\n".b + make_chunk("IHDR", ihdr) + make_chunk("IDAT", idat) + make_chunk("IEND", "")
    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png(png) }
    assert_equal "adler-mismatch", err.code

    # IDAT cavity (extra uncompressed bytes between DEFLATE stream and Adler-32)
    cavity_idat = [0x78, 0x9C].pack("CC") + deflated + "HIDDEN_PAYLOAD".b + [ImageCodecPng.adler32(scanline)].pack("N")
    png_cavity = "\x89PNG\r\n\x1a\n".b + make_chunk("IHDR", ihdr) + make_chunk("IDAT", cavity_idat) + make_chunk("IEND", "")
    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png(png_cavity) }
    assert_equal "idat-cavity", err.code
  end

  # 6. Chunk Ordering, Duplication & APNG Spoofing (CVE-2011-3045, CVE-2021-30860)
  def test_cve_chunk_ordering_and_apng_spoofing
    # APNG acTL chunk rejection with valid CRC
    actl_chunk = make_chunk("acTL", [10, 0].pack("NN"))
    png_actl = "\x89PNG\r\n\x1a\n".b + make_chunk("IHDR", [1, 1, 8, 6, 0, 0, 0].pack("NNCCCCC")) + actl_chunk + make_chunk("IEND", "")
    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png(png_actl) }
    assert_equal "unsupported-feature", err.code

    # APNG precedence: corrupt-CRC acTL -> chunk-crc-mismatch
    corrupt_actl = actl_chunk.dup
    corrupt_actl.setbyte(corrupt_actl.bytesize - 1, corrupt_actl.getbyte(corrupt_actl.bytesize - 1) ^ 0xFF)
    png_corrupt_actl = "\x89PNG\r\n\x1a\n".b + make_chunk("IHDR", [1, 1, 8, 6, 0, 0, 0].pack("NNCCCCC")) + corrupt_actl + make_chunk("IEND", "")
    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png(png_corrupt_actl) }
    assert_equal "chunk-crc-mismatch", err.code

    # APNG precedence: pre-IHDR acTL -> chunk-before-ihdr
    png_pre_actl = "\x89PNG\r\n\x1a\n".b + actl_chunk + make_chunk("IHDR", [1, 1, 8, 6, 0, 0, 0].pack("NNCCCCC")) + make_chunk("IEND", "")
    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png(png_pre_actl) }
    assert_equal "chunk-before-ihdr", err.code

    # APNG fcTL and fdAT -> unsupported-feature
    fctl_chunk = make_chunk("fcTL", [0, 1, 1, 0, 0, 1, 1, 0, 0].pack("NNNNNnnCC"))
    png_fctl = "\x89PNG\r\n\x1a\n".b + make_chunk("IHDR", [1, 1, 8, 6, 0, 0, 0].pack("NNCCCCC")) + fctl_chunk + make_chunk("IEND", "")
    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png(png_fctl) }
    assert_equal "unsupported-feature", err.code

    fdat_chunk = make_chunk("fdAT", [0].pack("N") + "extra")
    png_fdat = "\x89PNG\r\n\x1a\n".b + make_chunk("IHDR", [1, 1, 8, 6, 0, 0, 0].pack("NNCCCCC")) + fdat_chunk + make_chunk("IEND", "")
    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png(png_fdat) }
    assert_equal "unsupported-feature", err.code

    # Chunk before IHDR
    png_before = "\x89PNG\r\n\x1a\n".b + make_chunk("tEXt", "Author=Adhithya") + make_chunk("IHDR", [1, 1, 8, 6, 0, 0, 0].pack("NNCCCCC"))
    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png(png_before) }
    assert_equal "chunk-before-ihdr", err.code

    # Duplicate IHDR
    ihdr = make_chunk("IHDR", [1, 1, 8, 6, 0, 0, 0].pack("NNCCCCC"))
    png_dup = "\x89PNG\r\n\x1a\n".b + ihdr + ihdr + make_chunk("IEND", "")
    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png(png_dup) }
    assert_equal "duplicate-ihdr", err.code

    # Trailing data after IEND
    base = make_minimal_png
    png_trailing = base + "TRAILING_GARBAGE".b
    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png(png_trailing) }
    assert_equal "trailing-data", err.code
  end

  # 7. Payload-Blind Error Diagnostics & Standing Rule #20
  def test_cve_payload_blind_error_diagnostics
    # Verify that error messages do NOT leak input values
    codes = ImageCodecPng::PNG_ERROR_CODES
    codes.each do |code|
      err = ImageCodecPng::PngError.new(code)
      assert_equal code, err.code
      msg = err.message
      refute_empty msg
      assert msg.start_with?("image-codec-png:")
      refute_includes msg, "eval"
      refute_includes msg, "puts"
    end

    # Unknown code falls back safely
    err_unknown = ImageCodecPng::PngError.new("completely-arbitrary-error")
    assert_equal ImageCodecPng::Constants::ERROR_UNKNOWN, err_unknown.code
    assert_equal ImageCodecPng::Messages::UNKNOWN_ERROR_MESSAGE, err_unknown.message

    # Redacted and invalid placeholders
    assert_equal "<redacted>", ImageCodecPng::Messages::REDACTED_PLACEHOLDER
    assert_equal "<invalid>", ImageCodecPng::Messages::INVALID_PLACEHOLDER
  end
end
