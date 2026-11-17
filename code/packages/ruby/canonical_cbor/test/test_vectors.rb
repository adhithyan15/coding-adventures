# frozen_string_literal: true

require_relative "test_helper"
require "json"

class TestVectors < Minitest::Test
  include CodingAdventures::CanonicalCbor

  # standard:disable Style/RedundantHeredocDelimiterQuotes, Layout/HeredocIndentation, Layout/ClosingHeredocIndentation
  CANONICAL_CBOR_CASES_JSON = <<~'JSON'
{
  "schema_version": 1,
  "profile": "rfc8949-section-4.2.3-length-first",
  "limits": {
    "max_nesting_depth": 128,
    "max_encoded_bytes": 1048576
  },
  "error_ids": [
    "unexpected-eof",
    "trailing-bytes",
    "reserved",
    "indefinite",
    "non-minimal-integer",
    "invalid-utf8",
    "non-canonical-map-order",
    "unsupported-simple",
    "float-not-supported",
    "too-deep",
    "length-too-large",
    "duplicate-map-key",
    "encode-too-deep",
    "encode-too-large"
  ],
  "cases": [
    { "id": "cbor-v1-unsigned-zero", "operation": "round-trip", "input": "00", "expected": "00" },
    { "id": "cbor-v1-unsigned-23", "operation": "round-trip", "input": "17", "expected": "17" },
    { "id": "cbor-v1-unsigned-24", "operation": "round-trip", "input": "1818", "expected": "1818" },
    { "id": "cbor-v1-unsigned-255", "operation": "round-trip", "input": "18ff", "expected": "18ff" },
    { "id": "cbor-v1-unsigned-256", "operation": "round-trip", "input": "190100", "expected": "190100" },
    { "id": "cbor-v1-unsigned-65535", "operation": "round-trip", "input": "19ffff", "expected": "19ffff" },
    { "id": "cbor-v1-unsigned-65536", "operation": "round-trip", "input": "1a00010000", "expected": "1a00010000" },
    { "id": "cbor-v1-unsigned-u32max", "operation": "round-trip", "input": "1affffffff", "expected": "1affffffff" },
    { "id": "cbor-v1-unsigned-u32plus1", "operation": "round-trip", "input": "1b0000000100000000", "expected": "1b0000000100000000" },
    { "id": "cbor-v1-unsigned-u64max", "operation": "round-trip", "input": "1bffffffffffffffff", "expected": "1bffffffffffffffff" },
    { "id": "cbor-v1-negative-one", "operation": "round-trip", "input": "20", "expected": "20" },
    { "id": "cbor-v1-negative-24", "operation": "round-trip", "input": "37", "expected": "37" },
    { "id": "cbor-v1-negative-25", "operation": "round-trip", "input": "3818", "expected": "3818" },
    { "id": "cbor-v1-bytes-empty", "operation": "round-trip", "input": "40", "expected": "40" },
    { "id": "cbor-v1-bytes-three", "operation": "round-trip", "input": "430001ff", "expected": "430001ff" },
    { "id": "cbor-v1-text-empty", "operation": "round-trip", "input": "60", "expected": "60" },
    { "id": "cbor-v1-text-utf8", "operation": "round-trip", "input": "63e29883", "expected": "63e29883" },
    { "id": "cbor-v1-array", "operation": "round-trip", "input": "8300f5f6", "expected": "8300f5f6" },
    { "id": "cbor-v1-map-wire", "operation": "round-trip", "input": "a300f460f56161f6", "expected": "a300f460f56161f6" },
    { "id": "cbor-v1-tag-zero", "operation": "round-trip", "input": "c0f6", "expected": "c0f6" },
    { "id": "cbor-v1-tag-24", "operation": "round-trip", "input": "d81800", "expected": "d81800" },
    { "id": "cbor-v1-simple-values", "operation": "round-trip", "input": "83f4f5f6", "expected": "83f4f5f6" },
    { "id": "cbor-v1-map-input-order", "operation": "encode-map", "input": "6162=f6;1818=f5;60=f4;00=01;6161=02", "expected": "a5000160f41818f56161026162f6" },
    { "id": "cbor-v1-depth-128", "operation": "generated-round-trip", "input": "nested-array:128", "expected": "wire:nested-array:128" },
    { "id": "cbor-v1-size-limit", "operation": "generated-round-trip", "input": "bytes-repeat:1048571:00", "expected": "wire:bytes-repeat:1048571:00" },
    { "id": "cbor-v1-empty-input", "operation": "decode-error", "input": "", "expected": "unexpected-eof" },
    { "id": "cbor-v1-truncated-argument", "operation": "decode-error", "input": "1a0001", "expected": "unexpected-eof" },
    { "id": "cbor-v1-trailing", "operation": "decode-error", "input": "0000", "expected": "trailing-bytes" },
    { "id": "cbor-v1-reserved-28", "operation": "decode-error", "input": "1c", "expected": "reserved" },
    { "id": "cbor-v1-reserved-30", "operation": "decode-error", "input": "1e", "expected": "reserved" },
    { "id": "cbor-v1-indefinite-bytes", "operation": "decode-error", "input": "5f", "expected": "indefinite" },
    { "id": "cbor-v1-indefinite-text", "operation": "decode-error", "input": "7f", "expected": "indefinite" },
    { "id": "cbor-v1-indefinite-array", "operation": "decode-error", "input": "9f", "expected": "indefinite" },
    { "id": "cbor-v1-indefinite-map", "operation": "decode-error", "input": "bf", "expected": "indefinite" },
    { "id": "cbor-v1-break", "operation": "decode-error", "input": "ff", "expected": "indefinite" },
    { "id": "cbor-v1-nonminimal-inline", "operation": "decode-error", "input": "1800", "expected": "non-minimal-integer" },
    { "id": "cbor-v1-nonminimal-u8", "operation": "decode-error", "input": "1900ff", "expected": "non-minimal-integer" },
    { "id": "cbor-v1-nonminimal-u16", "operation": "decode-error", "input": "1a0000ffff", "expected": "non-minimal-integer" },
    { "id": "cbor-v1-nonminimal-u32", "operation": "decode-error", "input": "1b00000000ffffffff", "expected": "non-minimal-integer" },
    { "id": "cbor-v1-nonminimal-length", "operation": "decode-error", "input": "5800", "expected": "non-minimal-integer" },
    { "id": "cbor-v1-nonminimal-tag", "operation": "decode-error", "input": "d800f6", "expected": "non-minimal-integer" },
    { "id": "cbor-v1-invalid-utf8", "operation": "decode-error", "input": "61ff", "expected": "invalid-utf8" },
    { "id": "cbor-v1-invalid-utf8-overlong", "operation": "decode-error", "input": "62c080", "expected": "invalid-utf8" },
    { "id": "cbor-v1-map-out-of-order", "operation": "decode-error", "input": "a2616200616101", "expected": "non-canonical-map-order" },
    { "id": "cbor-v1-map-duplicate-wire", "operation": "decode-error", "input": "a2616100616101", "expected": "non-canonical-map-order" },
    { "id": "cbor-v1-unsupported-simple", "operation": "decode-error", "input": "f0", "expected": "unsupported-simple" },
    { "id": "cbor-v1-undefined", "operation": "decode-error", "input": "f7", "expected": "unsupported-simple" },
    { "id": "cbor-v1-float16", "operation": "decode-error", "input": "f90000", "expected": "float-not-supported" },
    { "id": "cbor-v1-float32", "operation": "decode-error", "input": "fa00000000", "expected": "float-not-supported" },
    { "id": "cbor-v1-float64", "operation": "decode-error", "input": "fb0000000000000000", "expected": "float-not-supported" },
    { "id": "cbor-v1-hostile-bytes-length", "operation": "decode-error", "input": "5bffffffffffffffff", "expected": "length-too-large" },
    { "id": "cbor-v1-depth-129-decode", "operation": "decode-error", "input": "nested-array-wire:129", "expected": "too-deep" },
    { "id": "cbor-v1-duplicate-encode", "operation": "encode-error", "input": "duplicate-map-key", "expected": "duplicate-map-key" },
    { "id": "cbor-v1-depth-129-encode", "operation": "encode-error", "input": "nested-array:129", "expected": "encode-too-deep" },
    { "id": "cbor-v1-size-limit-plus-one", "operation": "encode-error", "input": "bytes-repeat:1048572:00", "expected": "encode-too-large" }
  ]
}
JSON
  # standard:enable Style/RedundantHeredocDelimiterQuotes, Layout/HeredocIndentation, Layout/ClosingHeredocIndentation

  def hex_to_bytes(hex)
    return String.new(encoding: Encoding::BINARY) if hex.empty?
    [hex.gsub(/\s+/, "")].pack("H*").b
  end

  def bytes_to_hex(bytes)
    bytes.b.unpack1("H*")
  end

  def build_nested_array(n)
    cur = CborArray.new([])
    n.times { cur = CborArray.new([cur]) }
    cur
  end

  def build_nested_array_wire(n)
    ("\x81" * n + "\x80").b
  end

  def test_fixture_parsing_and_count
    data = JSON.parse(CANONICAL_CBOR_CASES_JSON)
    assert_equal 55, data["cases"].length
  end

  def test_all_canonical_cbor_conformance_vectors
    data = JSON.parse(CANONICAL_CBOR_CASES_JSON)
    cases = data["cases"]

    cases.each do |c|
      id = c["id"]
      op = c["operation"]
      input = c["input"]
      expected = c["expected"]

      case op
      when "round-trip"
        in_bytes = hex_to_bytes(input)
        val = CanonicalCbor.decode(in_bytes)
        out_bytes = CanonicalCbor.encode_checked(val)
        assert_equal expected, bytes_to_hex(out_bytes), "#{id}: encoded mismatch"
        val2 = CanonicalCbor.decode(out_bytes)
        assert_equal val, val2, "#{id}: round-trip value mismatch"

      when "encode-map"
        pairs = input.split(";")
        entries = pairs.map do |pair|
          k_hex, v_hex = pair.split("=")
          k = CanonicalCbor.decode(hex_to_bytes(k_hex))
          v = CanonicalCbor.decode(hex_to_bytes(v_hex))
          CborMapEntry.new(k, v)
        end
        map_val = CborMap.new(entries)
        out_bytes = CanonicalCbor.encode_checked(map_val)
        assert_equal expected, bytes_to_hex(out_bytes), "#{id}: encoded map mismatch"

      when "generated-round-trip"
        if input == "nested-array:128"
          val = build_nested_array(128)
          wire = build_nested_array_wire(128)
          enc = CanonicalCbor.encode_checked(val)
          assert_equal wire, enc, "#{id}: generated wire mismatch"
          dec = CanonicalCbor.decode(wire)
          assert_equal val, dec, "#{id}: generated decode mismatch"
        elsif input == "bytes-repeat:1048571:00"
          payload = ("\x00" * 1048571).b
          val = CborBytes.new(payload)
          enc = CanonicalCbor.encode_checked(val)
          assert_equal 1048576, enc.bytesize, "#{id}: generated size mismatch"
          dec = CanonicalCbor.decode(enc)
          assert_equal val, dec, "#{id}: generated decode mismatch"
        else
          flunk "unknown generated-round-trip input: #{input}"
        end

      when "decode-error"
        wire = if input == "nested-array-wire:129"
          build_nested_array_wire(129)
        else
          hex_to_bytes(input)
        end
        expected_sym = expected.tr("-", "_").to_sym
        err = assert_raises(CborError, "#{id}: expected #{expected}") do
          CanonicalCbor.decode(wire)
        end
        assert_equal expected_sym, err.id, "#{id}: wrong error id"
        assert err.message.start_with?("canonical-cbor: "), "#{id}: bad message prefix"

      when "encode-error"
        val = if input == "duplicate-map-key"
          CborMap.new([
            CborMapEntry.new(CborText.new("dup"), CborUnsigned.new(1)),
            CborMapEntry.new(CborText.new("dup"), CborUnsigned.new(2))
          ])
        elsif input == "nested-array:129"
          build_nested_array(129)
        elsif input == "bytes-repeat:1048572:00"
          CborBytes.new(("\x00" * 1048572).b)
        else
          flunk "unknown encode-error input: #{input}"
        end
        expected_sym = expected.tr("-", "_").to_sym
        err = assert_raises(CborError, "#{id}: expected #{expected}") do
          CanonicalCbor.encode_checked(val)
        end
        assert_equal expected_sym, err.id, "#{id}: wrong error id"
        assert err.message.start_with?("canonical-cbor: "), "#{id}: bad message prefix"

      else
        flunk "unknown operation: #{op}"
      end
    end
  end
end
