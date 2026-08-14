# frozen_string_literal: true

require "json"
require "zlib"
require "test_helper"

class TestPortableRawRFC1951Conformance < Minitest::Test
  FIXTURE_PATH = File.expand_path(
    "../../../../specs/fixtures/zip-raw-rfc1951-v1/cases.json",
    __dir__
  )
  FIXTURE = JSON.parse(File.binread(FIXTURE_PATH))

  def from_hex(value)
    [value].pack("H*").b
  end

  def expected_bytes(test_case)
    output = test_case.fetch("expected").fetch("output")
    return from_hex(output.fetch("hex")) if output.key?("hex")

    from_hex(output.fetch("repeat_hex")) * output.fetch("count")
  end

  def inflate_limit(test_case)
    test_case.fetch("max_output", CodingAdventures::Zip::RAW_INFLATE_MAX_OUTPUT)
  end

  def test_closed_fixture_metadata
    assert_equal 34, FIXTURE.fetch("cases").length
    assert_equal(
      {
        "default_max_output" => CodingAdventures::Zip::RAW_INFLATE_MAX_OUTPUT,
        "hard_max_output" => CodingAdventures::Zip::RAW_INFLATE_MAX_OUTPUT
      },
      FIXTURE.fetch("limits")
    )
    assert_equal CodingAdventures::Zip::RAW_INFLATE_ERROR_CODES, FIXTURE.fetch("error_ids")
  end

  FIXTURE.fetch("cases").each do |test_case|
    define_method("test_#{test_case.fetch("id").tr("-", "_")}") do
      expected = test_case.fetch("expected")
      case test_case.fetch("operation")
      when "inflate"
        input = from_hex(test_case.fetch("input_hex"))
        result = CodingAdventures::Zip.raw_inflate_counted(input, max_output: inflate_limit(test_case))
        assert_equal expected_bytes(test_case), result.output
        assert_equal expected.fetch("bytes_consumed"), result.bytes_consumed
        assert_equal expected_bytes(test_case),
          CodingAdventures::Zip.raw_inflate(input, max_output: inflate_limit(test_case))
      when "inflate-error"
        error = assert_raises(CodingAdventures::Zip::RawInflateError) do
          CodingAdventures::Zip.raw_inflate_counted(
            from_hex(test_case.fetch("input_hex")),
            max_output: inflate_limit(test_case)
          )
        end
        assert_equal expected.fetch("error_id"), error.code
        assert_equal expected.fetch("error_id"), error.message
      when "deflate-interoperability"
        compressed = CodingAdventures::Zip.raw_deflate(from_hex(test_case.fetch("input_hex")))
        inflater = Zlib::Inflate.new(-Zlib::MAX_WBITS)
        decoded = inflater.inflate(compressed) + inflater.finish
        inflater.close
        assert_equal expected_bytes(test_case), decoded
      when "crc32"
        checksum = test_case.fetch("initial_crc32_hex", "00000000").to_i(16)
        test_case.fetch("chunks_hex").each do |chunk|
          checksum = CodingAdventures::Zip.crc32(from_hex(chunk), initial: checksum)
        end
        assert_equal expected.fetch("crc32_hex"), format("%08x", checksum)
      else
        flunk "unknown operation #{test_case.fetch("operation")}"
      end
    end
  end
end

class TestStrictRawInflateZipBoundary < Minitest::Test
  DYNAMIC = [
    "0dc28911c0200c03b0d8f97028ec3f6ed129cab7dd96a0c2445bdb93809663a5d303f6b265e20c2b79ea03379d227e"
  ].pack("H*").b
  DYNAMIC_OUTPUT = [
    "0406030b000e070909010906010a04070007000000000501010908030108050302030401000401000207090009020a0a020605020d060c01020b020302090201"
  ].pack("H*").b

  def raw_zip(name, compressed, uncompressed, declared_size: uncompressed.bytesize)
    name_bytes = name.b
    checksum = CodingAdventures::Zip.crc32(uncompressed)
    local = [
      0x04034B50, 20, 0x0800, 8, 0, 0, checksum,
      compressed.bytesize, declared_size, name_bytes.bytesize, 0
    ].pack("VvvvvvVVVvv") + name_bytes + compressed
    central_offset = local.bytesize
    central = [
      0x02014B50, 0x031E, 20, 0x0800, 8, 0, 0, checksum,
      compressed.bytesize, declared_size, name_bytes.bytesize,
      0, 0, 0, 0, 0, 0
    ].pack("VvvvvvvVVVvvvvvVV") + name_bytes
    eocd = [0x06054B50, 0, 0, 1, 1, central.bytesize, central_offset, 0].pack("VvvvvVVv")
    local + central + eocd
  end

  def test_zip_reader_accepts_dynamic_raw_payload
    reader = CodingAdventures::Zip::ZipReader.new(raw_zip("dynamic.bin", DYNAMIC, DYNAMIC_OUTPUT))
    assert_equal DYNAMIC_OUTPUT, reader.read(reader.entries.first)
  end

  def test_zip_reader_rejects_suffix_cavity
    archive = raw_zip("cavity.bin", DYNAMIC + "\xDE\xAD".b, DYNAMIC_OUTPUT)
    reader = CodingAdventures::Zip::ZipReader.new(archive)
    error = assert_raises(RuntimeError) { reader.read(reader.entries.first) }
    assert_equal "zip: compressed payload contains trailing bytes", error.message
  end

  def test_zip_reader_rejects_declared_size_mismatch
    archive = raw_zip(
      "size.bin",
      DYNAMIC,
      DYNAMIC_OUTPUT,
      declared_size: DYNAMIC_OUTPUT.bytesize + 1
    )
    reader = CodingAdventures::Zip::ZipReader.new(archive)
    error = assert_raises(RuntimeError) { reader.read(reader.entries.first) }
    assert_equal "zip: uncompressed size does not match the directory", error.message
  end

  def test_rejects_invalid_output_limit_before_decode
    [-1, CodingAdventures::Zip::RAW_INFLATE_MAX_OUTPUT + 1, 1.5].each do |limit|
      error = assert_raises(CodingAdventures::Zip::RawInflateError) do
        CodingAdventures::Zip.raw_inflate_counted("\x01\x00\x00\xFF\xFF".b, max_output: limit)
      end
      assert_equal "invalid-output-limit", error.code
    end
  end

  def test_historical_deflate_wrappers_remain_compatible
    expected = "wrapper compatibility".b * 16
    compressed = CodingAdventures::Zip.deflate_compress(expected)

    assert_equal expected, CodingAdventures::Zip.deflate_decompress(compressed)
  end

  def test_full_window_foreign_stream
    prefix = Array.new(32_768) { |i| ((i * 73) + (i / 251)) & 0xFF }.pack("C*")
    expected = prefix + prefix
    deflater = Zlib::Deflate.new(Zlib::BEST_COMPRESSION, -Zlib::MAX_WBITS)
    compressed = deflater.deflate(expected, Zlib::FINISH)
    deflater.close

    assert_equal expected,
      CodingAdventures::Zip.raw_inflate(compressed, max_output: expected.bytesize)
  end
end
