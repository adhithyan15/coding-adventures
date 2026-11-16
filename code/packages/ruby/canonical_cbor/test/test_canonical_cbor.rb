# frozen_string_literal: true

require_relative "test_helper"

class TestCanonicalCbor < Minitest::Test
  include CodingAdventures::CanonicalCbor

  # Deterministic Linear Congruential Generator (LCG) for zero-dependency property testing.
  class DeterministicPrng
    def initialize(seed)
      @state = seed & 0xFFFFFFFF
    end

    def next_u32
      @state = (1664525 * @state + 1013904223) & 0xFFFFFFFF
      @state
    end

    def next_int(max)
      return 0 if max <= 0
      next_u32 % max
    end

    def next_u64
      high = next_u32
      low = next_u32
      (high << 32) | low
    end

    def next_bytes(len)
      String.new(encoding: Encoding::BINARY).tap do |buf|
        len.times { buf << (next_u32 & 0xFF) }
      end
    end
  end

  def hex_to_bytes(hex)
    [hex.gsub(/\s+/, "")].pack("H*")
  end

  def bytes_to_hex(bytes)
    bytes.unpack1("H*")
  end

  def build_nested_array(depth)
    cur = CborNull.new
    depth.times do
      cur = CborArray.new([cur])
    end
    cur
  end

  def build_nested_array_wire(depth)
    String.new(encoding: Encoding::BINARY).tap do |wire|
      depth.times { wire << 0x81 }
      wire << 0xF6
    end
  end

  # =========================================================================
  # 1. Value Model, Constructors, and Defensive Slicing / Immutability
  # =========================================================================

  def test_unsigned_constructor_and_immutability
    u = CborUnsigned.new(42)
    assert_equal :unsigned, u.kind
    assert_equal 42, u.value
    assert_equal "CborUnsigned(42)", u.to_s
    assert_equal "CborUnsigned(42)", u.inspect
    assert u.frozen?

    # Equality and hash
    u2 = CborUnsigned.new(42)
    assert_equal u, u2
    assert u.eql?(u2)
    assert_equal u.hash, u2.hash
    refute_equal u, CborUnsigned.new(43)

    # Immutability
    assert_raises(FrozenError) do
      u.instance_variable_set(:@value, 99)
    end

    # Range limits
    assert_raises(CborError) { CborUnsigned.new(-1) }
    assert_raises(CborError) { CborUnsigned.new(Constants::MAX_UINT64 + 1) }
    assert_raises(CborError) { CborUnsigned.new("42") }
    assert_raises(CborError) { CborUnsigned.new(nil) }
  end

  def test_negative_constructor_and_immutability
    n = CborNegative.new(41) # represents -42
    assert_equal :negative, n.kind
    assert_equal 41, n.value
    assert_equal(-42, n.to_i)
    assert_equal "CborNegative(41)", n.to_s
    assert n.frozen?

    n0 = CborNegative.new(0) # represents -1
    assert_equal(-1, n0.to_i)

    # Equality and hash
    n2 = CborNegative.new(41)
    assert_equal n, n2
    assert n.eql?(n2)
    assert_equal n.hash, n2.hash
    refute_equal n, CborNegative.new(42)

    # Range limits
    assert_raises(CborError) { CborNegative.new(-1) }
    assert_raises(CborError) { CborNegative.new(Constants::MAX_UINT64 + 1) }
    assert_raises(CborError) { CborNegative.new(nil) }
  end

  def test_bytes_constructor_and_immutability
    b = CborBytes.new("\x01\x02\x03".b)
    assert_equal :bytes, b.kind
    assert_equal "\x01\x02\x03".b, b.bytes
    assert_equal 3, b.bytesize
    assert_equal 3, b.length
    assert_includes b.to_s, "CborBytes"
    assert_includes b.inspect, "CborBytes"
    assert b.frozen?
    assert b.bytes.frozen?

    # From integer array
    b_arr = CborBytes.new([1, 2, 3])
    assert_equal b, b_arr

    # Equality and hash
    b2 = CborBytes.new("\x01\x02\x03".b)
    assert_equal b, b2
    assert b.eql?(b2)
    assert_equal b.hash, b2.hash
    refute_equal b, CborBytes.new("\x01\x02".b)

    # Rejections
    assert_raises(CborError) { CborBytes.new(123) }
    assert_raises(CborError) { CborBytes.new([-1]) }
    assert_raises(CborError) { CborBytes.new([256]) }
  end

  def test_text_constructor_and_utf8_validation
    t = CborText.new("hello world")
    assert_equal :text, t.kind
    assert_equal "hello world", t.text
    assert_equal 11, t.bytesize
    assert_equal 11, t.length
    assert_includes t.to_s, "CborText"
    assert_includes t.inspect, "CborText"
    assert t.frozen?
    assert t.text.frozen?

    # Equality and hash
    t2 = CborText.new("hello world")
    assert_equal t, t2
    assert t.eql?(t2)
    assert_equal t.hash, t2.hash
    refute_equal t, CborText.new("hello")

    # Multi-byte UTF-8
    t_multi = CborText.new("日本語")
    assert_equal 3, t_multi.length
    assert_equal 9, t_multi.bytesize

    # Non-string rejection
    assert_raises(CborError) { CborText.new(123) }
    assert_raises(CborError) { CborText.new(nil) }

    # Non-UTF-8 encoding rejection (CVE-5 defense)
    ascii_8bit = "test".b
    assert_raises(CborError) { CborText.new(ascii_8bit) }

    # Invalid UTF-8 bytes rejection
    invalid_utf8 = [0xFF, 0xFE].pack("C*").force_encoding(Encoding::UTF_8)
    assert_raises(CborError) { CborText.new(invalid_utf8) }

    # Lone surrogate rejection (CESU-8)
    surrogate = [0xED, 0xA0, 0x80].pack("C*").force_encoding(Encoding::UTF_8)
    assert_raises(CborError) { CborText.new(surrogate) }
  end

  def test_array_constructor_and_immutability
    arr = CborArray.new([CborUnsigned.new(1), CborBool.new(true)])
    assert_equal :array, arr.kind
    assert_equal 2, arr.length
    assert_includes arr.to_s, "CborArray"
    assert_includes arr.inspect, "CborArray"
    assert arr.frozen?
    assert arr.items.frozen?

    # Equality and hash
    arr2 = CborArray.new([CborUnsigned.new(1), CborBool.new(true)])
    assert_equal arr, arr2
    assert arr.eql?(arr2)
    assert_equal arr.hash, arr2.hash
    refute_equal arr, CborArray.new([CborUnsigned.new(1)])

    assert_raises(CborError) { CborArray.new("not-an-array") }
  end

  def test_map_and_entry_constructors_and_immutability
    entry1 = CborMapEntry.new(CborText.new("a"), CborUnsigned.new(1))
    entry2 = CborMapEntry.new(CborText.new("b"), CborUnsigned.new(2))
    assert entry1.frozen?
    assert_includes entry1.to_s, "CborMapEntry"
    assert_includes entry1.inspect, "CborMapEntry"

    assert_equal CborText.new("a"), entry1.key
    assert_equal CborUnsigned.new(1), entry1.value
    assert_equal entry1, CborMapEntry.new(CborText.new("a"), CborUnsigned.new(1))
    assert entry1.eql?(CborMapEntry.new(CborText.new("a"), CborUnsigned.new(1)))
    assert_equal entry1.hash, CborMapEntry.new(CborText.new("a"), CborUnsigned.new(1)).hash
    refute_equal entry1, entry2

    map = CborMap.new([entry1, entry2])
    assert_equal :map, map.kind
    assert_equal 2, map.length
    assert_includes map.to_s, "CborMap"
    assert_includes map.inspect, "CborMap"
    assert map.frozen?
    assert map.entries.frozen?

    map2 = CborMap.new([entry1, entry2])
    assert_equal map, map2
    assert map.eql?(map2)
    assert_equal map.hash, map2.hash
    refute_equal map, CborMap.new([entry1])

    assert_raises(CborError) { CborMap.new("not-an-array") }
  end

  def test_tag_constructor_and_immutability
    tag = CborTag.new(1, CborUnsigned.new(100))
    assert_equal :tag, tag.kind
    assert_equal 1, tag.tag
    assert_equal CborUnsigned.new(100), tag.value
    assert_includes tag.to_s, "CborTag"
    assert_includes tag.inspect, "CborTag"
    assert tag.frozen?

    tag2 = CborTag.new(1, CborUnsigned.new(100))
    assert_equal tag, tag2
    assert tag.eql?(tag2)
    assert_equal tag.hash, tag2.hash
    refute_equal tag, CborTag.new(2, CborUnsigned.new(100))

    assert_raises(CborError) { CborTag.new(-1, CborNull.new) }
    assert_raises(CborError) { CborTag.new(Constants::MAX_UINT64 + 1, CborNull.new) }
  end

  def test_bool_and_null_constructors
    b_true = CborBool.new(true)
    b_false = CborBool.new(false)
    assert_equal :boolean, b_true.kind
    assert_equal true, b_true.value
    assert_equal false, b_false.value
    assert_includes b_true.to_s, "CborBool"
    assert_includes b_true.inspect, "CborBool"
    assert b_true.frozen?
    assert b_false.frozen?

    assert_equal b_true, CborBool.new(true)
    assert b_true.eql?(CborBool.new(true))
    assert_equal b_true.hash, CborBool.new(true).hash
    refute_equal b_true, b_false

    assert_raises(CborError) { CborBool.new(1) }
    assert_raises(CborError) { CborBool.new(nil) }

    c_null = CborNull.new
    assert_equal :null, c_null.kind
    assert c_null.frozen?
    assert_equal c_null, CborNull.new
    assert c_null.eql?(CborNull.new)
    assert_equal c_null.hash, CborNull.new.hash
    refute_equal c_null, b_false
    assert_equal "CborNull()", c_null.to_s
  end

  def test_span_and_spanned_map_entry_redaction
    span = Span.new(5, 15)
    assert_equal 5, span.start
    assert_equal 15, span.end
    assert_equal 10, span.length
    assert_equal "[5, 15)", span.to_s
    assert_equal "[5, 15)", span.inspect
    assert span.frozen?

    span2 = Span.new(5, 15)
    assert_equal span, span2
    assert span.eql?(span2)
    assert_equal span.hash, span2.hash
    refute_equal span, Span.new(5, 16)

    secret_key = CborText.new("SECRET_API_KEY")
    secret_val = CborText.new("CONFIDENTIAL_PASSWORD")
    spanned_entry = SpannedMapEntry.new(key: secret_key, value: secret_val, value_span: span)
    assert spanned_entry.frozen?
    assert_equal secret_key, spanned_entry.key
    assert_equal secret_val, spanned_entry.value
    assert_equal span, spanned_entry.value_span

    # Redaction requirement: inspect and to_s MUST redact key and value
    entry_str = spanned_entry.to_s
    entry_inspect = spanned_entry.inspect
    assert_includes entry_str, "<redacted>"
    assert_includes entry_inspect, "<redacted>"
    refute_includes entry_str, "SECRET_API_KEY"
    refute_includes entry_str, "CONFIDENTIAL_PASSWORD"
    refute_includes entry_inspect, "SECRET_API_KEY"
    refute_includes entry_inspect, "CONFIDENTIAL_PASSWORD"

    spanned2 = SpannedMapEntry.new(key: secret_key, value: secret_val, value_span: span)
    assert_equal spanned_entry, spanned2
    assert spanned_entry.eql?(spanned2)
    assert_equal spanned_entry.hash, spanned2.hash
  end

  def test_decode_map_spans_result
    entry = SpannedMapEntry.new(
      key: CborText.new("k"),
      value: CborUnsigned.new(1),
      value_span: Span.new(2, 3)
    )
    res_map = DecodeMapSpansResult.map([entry])
    assert res_map.map?
    assert_equal [entry], res_map.entries
    assert res_map.frozen?
    assert res_map.entries.frozen?

    res_map2 = DecodeMapSpansResult.map([entry])
    assert_equal res_map, res_map2
    assert res_map.eql?(res_map2)
    assert_equal res_map.hash, res_map2.hash

    res_non_map = DecodeMapSpansResult.non_map
    refute res_non_map.map?
    assert_nil res_non_map.entries
    assert res_non_map.frozen?

    refute_equal res_map, res_non_map

    # Constructor encapsulation and type validation
    assert_raises(NoMethodError) { DecodeMapSpansResult.new(nil) }
    assert_raises(CborError) { DecodeMapSpansResult.map(["not-spanned-entry"]) }
    assert_raises(CborError) { DecodeMapSpansResult.map("not-an-array") }
  end

  def test_container_and_tag_type_safety
    # CborArray
    assert_raises(CborError) { CborArray.new([123]) }
    assert_raises(CborError) { CborArray.new(["not-cbor"]) }

    # CborMapEntry
    assert_raises(CborError) { CborMapEntry.new("k", CborUnsigned.new(1)) }
    assert_raises(CborError) { CborMapEntry.new(CborText.new("k"), "v") }

    # CborMap
    assert_raises(CborError) { CborMap.new([123]) }
    assert_raises(CborError) { CborMap.new([CborNull.new]) }

    # CborTag
    assert_raises(CborError) { CborTag.new(1, 123) }
    assert_raises(CborError) { CborTag.new(1, "raw") }

    # Root encode_checked
    assert_raises(CborError) { CanonicalCbor.encode_checked(123) }
    assert_raises(CborError) { CanonicalCbor.encode_checked("raw") }
    assert_raises(CborError) { CanonicalCbor.encode_checked(nil) }

    # Root encode_into_checked dst validation
    assert_raises(CborError) { CanonicalCbor.encode_into_checked(CborNull.new, 123) }
    assert_raises(CborError) { CanonicalCbor.encode_into_checked(CborNull.new, (+"frozen".b).freeze) }
    assert_raises(CborError) { CanonicalCbor.encode_into_checked(CborNull.new, +"utf8".encode(Encoding::UTF_8)) }
  end

  # =========================================================================
  # 2. Error Diagnostics & Clean Code
  # =========================================================================

  def test_all_14_closed_error_messages
    expected_ids = [
      :unexpected_eof,
      :trailing_bytes,
      :reserved,
      :indefinite,
      :non_minimal_integer,
      :invalid_utf8,
      :non_canonical_map_order,
      :unsupported_simple,
      :float_not_supported,
      :too_deep,
      :length_too_large,
      :duplicate_map_key,
      :encode_too_deep,
      :encode_too_large
    ]

    assert_equal expected_ids.sort, Messages::ERROR_MESSAGES.keys.sort
    assert_equal "<redacted>", Messages::REDACTED_PLACEHOLDER

    expected_ids.each do |id|
      msg = Messages.message_for(id)
      assert msg.start_with?("canonical-cbor: "), "#{id} must start with 'canonical-cbor: '"

      err = CborError.new(id)
      assert_equal id, err.id
      assert_equal msg, err.message
      assert_equal msg, err.to_s
      assert_equal "#<CodingAdventures::CanonicalCbor::CborError: #{msg}>", err.inspect

      err_same = CborError.new(id)
      assert_equal err, err_same
      assert err.eql?(err_same)
      assert_equal err.hash, err_same.hash
    end
  end

  # =========================================================================
  # 3. Convenience Wrappers & Atomic Rollback
  # =========================================================================

  def test_convenience_wrappers
    val = CborUnsigned.new(123)
    enc1 = CanonicalCbor.encode_checked(val)
    enc2 = CanonicalCbor.encode(val)
    assert_equal enc1, enc2

    dst1 = String.new(encoding: Encoding::BINARY)
    CanonicalCbor.encode_into_checked(val, dst1)
    assert_equal enc1, dst1

    dst2 = String.new(encoding: Encoding::BINARY)
    CanonicalCbor.encode_into(val, dst2)
    assert_equal enc1, dst2
  end

  def test_atomic_rollback_on_encoding_error
    buf = +"prefix_payload_bytes".b
    initial_len = buf.bytesize
    initial_content = buf.dup

    bad_map = CborMap.new([
      CborMapEntry.new(CborText.new("key"), CborUnsigned.new(1)),
      CborMapEntry.new(CborText.new("key"), CborUnsigned.new(2))
    ])

    err = assert_raises(CborError) do
      CanonicalCbor.encode_into_checked(bad_map, buf)
    end
    assert_equal :duplicate_map_key, err.id
    assert_equal initial_len, buf.bytesize
    assert_equal initial_content, buf

    # Test rollback when deep nesting triggers encode_too_deep
    bad_nest = build_nested_array(129)
    err2 = assert_raises(CborError) do
      CanonicalCbor.encode_into_checked(bad_nest, buf)
    end
    assert_equal :encode_too_deep, err2.id
    assert_equal initial_len, buf.bytesize
    assert_equal initial_content, buf

    # Test mid-map rollback when second entry triggers encode_too_large
    big_payload = "\x00".b * Constants::MAX_ENCODED_BYTES
    too_large_map = CborMap.new([
      CborMapEntry.new(CborText.new("key1"), CborText.new("val1")),
      CborMapEntry.new(CborText.new("key2"), CborBytes.new(big_payload))
    ])
    err3 = assert_raises(CborError) do
      CanonicalCbor.encode_into_checked(too_large_map, buf)
    end
    assert_equal :encode_too_large, err3.id
    assert_equal initial_len, buf.bytesize
    assert_equal initial_content, buf
  end

  # =========================================================================
  # 4. Integer and Argument Boundary Transitions
  # =========================================================================

  def test_unsigned_boundary_transitions
    cases = [
      [0, "00"],
      [23, "17"],
      [24, "1818"],
      [255, "18ff"],
      [256, "190100"],
      [65535, "19ffff"],
      [65536, "1a00010000"],
      [4294967295, "1affffffff"],
      [4294967296, "1b0000000100000000"],
      [18446744073709551615, "1bffffffffffffffff"]
    ]

    cases.each do |val, expected_hex|
      u = CborUnsigned.new(val)
      encoded = CanonicalCbor.encode_checked(u)
      assert_equal expected_hex, bytes_to_hex(encoded), "Failed for unsigned #{val}"
      decoded = CanonicalCbor.decode(encoded)
      assert_equal u, decoded
    end
  end

  def test_negative_boundary_transitions
    cases = [
      [0, -1, "20"],
      [23, -24, "37"],
      [24, -25, "3818"],
      [255, -256, "38ff"],
      [256, -257, "390100"],
      [65535, -65536, "39ffff"],
      [65536, -65537, "3a00010000"],
      [4294967295, -4294967296, "3affffffff"],
      [4294967296, -4294967297, "3b0000000100000000"],
      [18446744073709551615, -18446744073709551616, "3bffffffffffffffff"]
    ]

    cases.each do |raw_val, signed_val, expected_hex|
      n = CborNegative.new(raw_val)
      assert_equal signed_val, n.to_i
      encoded = CanonicalCbor.encode_checked(n)
      assert_equal expected_hex, bytes_to_hex(encoded), "Failed for negative #{signed_val}"
      decoded = CanonicalCbor.decode(encoded)
      assert_equal n, decoded
    end
  end

  # =========================================================================
  # 5. Nesting Depth and Size Transitions
  # =========================================================================

  def test_nesting_depth_transitions
    # Depth 128 is valid
    val_128 = build_nested_array(128)
    wire_128 = build_nested_array_wire(128)
    encoded_128 = CanonicalCbor.encode_checked(val_128)
    assert_equal wire_128, encoded_128
    assert_equal val_128, CanonicalCbor.decode(encoded_128)

    # Depth 129 decode rejected
    wire_129 = build_nested_array_wire(129)
    err_dec = assert_raises(CborError) { CanonicalCbor.decode(wire_129) }
    assert_equal :too_deep, err_dec.id

    # Depth 129 encode rejected
    val_129 = build_nested_array(129)
    err_enc = assert_raises(CborError) { CanonicalCbor.encode_checked(val_129) }
    assert_equal :encode_too_deep, err_enc.id
  end

  def test_size_transitions
    # Exactly 1,048,576 bytes: 5 byte header (0x5A 0x00 0x10 0x00 0x00 minus header = 1048571 payload)
    payload_exact = ("\xAA" * 1048571).b
    val_exact = CborBytes.new(payload_exact)
    encoded_exact = CanonicalCbor.encode_checked(val_exact)
    assert_equal Constants::MAX_ENCODED_BYTES, encoded_exact.bytesize
    assert_equal val_exact, CanonicalCbor.decode(encoded_exact)

    # 1 byte over: 1,048,577 bytes encoded rejected
    payload_over = ("\xAA" * 1048572).b
    val_over = CborBytes.new(payload_over)
    err = assert_raises(CborError) { CanonicalCbor.encode_checked(val_over) }
    assert_equal :encode_too_large, err.id
  end

  # =========================================================================
  # 6. DecodeMapSpans & Pass-Through Invariant
  # =========================================================================

  def test_decode_map_spans_accuracy
    wire = hex_to_bytes("a26161016162820304")
    # {"a": 1, "b": [3, 4]}
    # 'a' wire: 61 61 (2 bytes), value 1: 01 (span 3..4)
    # 'b' wire: 61 62 (2 bytes), value [3, 4]: 82 03 04 (span 7..10)

    res = CanonicalCbor.decode_map_spans(wire)
    assert res.map?
    entries = res.entries
    assert_equal 2, entries.length

    e1 = entries[0]
    assert_equal CborText.new("a"), e1.key
    assert_equal CborUnsigned.new(1), e1.value
    assert_equal 3, e1.value_span.start
    assert_equal 4, e1.value_span.end
    assert_equal wire.byteslice(e1.value_span.start, e1.value_span.length), CanonicalCbor.encode_checked(e1.value)

    e2 = entries[1]
    assert_equal CborText.new("b"), e2.key
    assert_equal CborArray.new([CborUnsigned.new(3), CborUnsigned.new(4)]), e2.value
    assert_equal 6, e2.value_span.start
    assert_equal 9, e2.value_span.end
    assert_equal 3, e2.value_span.length
    assert_equal wire.byteslice(e2.value_span.start, e2.value_span.length), CanonicalCbor.encode_checked(e2.value)
  end

  def test_decode_map_spans_non_map
    res1 = CanonicalCbor.decode_map_spans(hex_to_bytes("01"))
    refute res1.map?

    res2 = CanonicalCbor.decode_map_spans(hex_to_bytes("820102"))
    refute res2.map?
  end

  def test_decode_map_spans_invalid_headers
    # Empty wire
    assert_raises(CborError) { CanonicalCbor.decode_map_spans("".b) }

    # Non-canonical map key ordering
    assert_raises(CborError) do
      CanonicalCbor.decode_map_spans(hex_to_bytes("a2616202616101"))
    end

    # Trailing bytes
    assert_raises(CborError) do
      CanonicalCbor.decode_map_spans(hex_to_bytes("a1616101ff"))
    end

    # Indefinite map (0xBF)
    err_indef = assert_raises(CborError) do
      CanonicalCbor.decode_map_spans(hex_to_bytes("bf616101ff"))
    end
    assert_equal :indefinite, err_indef.id

    # Non-minimal integer header (0xB800)
    err_non_min = assert_raises(CborError) do
      CanonicalCbor.decode_map_spans(hex_to_bytes("b800"))
    end
    assert_equal :non_minimal_integer, err_non_min.id

    # Reserved additional info (0xBC)
    err_res = assert_raises(CborError) do
      CanonicalCbor.decode_map_spans(hex_to_bytes("bc"))
    end
    assert_equal :reserved, err_res.id
  end

  def test_decode_map_spans_cbr01_passthrough_invariant
    # CBR01 pass-through: large wire bytes (> 1 MiB) can be decoded by decode_map_spans without encode_too_large error
    # Create valid wire representation with 2 entries: key "a" -> 1 MiB bytes
    big_bytes = "\x55" * 1_048_580
    header = [0x5A, 0x00, 0x10, 0x00, 0x04].pack("C*") # 5 bytes header for 1048580
    val_wire = header + big_bytes

    key_wire = "\x61\x61".b # "a"
    map_header = [0xA1].pack("C*") # map(1)
    full_wire = map_header + key_wire + val_wire

    res = CanonicalCbor.decode_map_spans(full_wire)
    assert res.map?
    assert_equal 1, res.entries.length
    assert_equal full_wire.bytesize - val_wire.bytesize, res.entries.first.value_span.start
    assert_equal full_wire.bytesize, res.entries.first.value_span.end
  end

  # =========================================================================
  # 7. Non-Minimal Header Widening Rejection Across Major Types
  # =========================================================================

  def test_non_minimal_header_widening_rejection
    # Across major types 0..6, widened headers must be rejected with :non_minimal_integer
    (0..6).each do |major|
      major_shift = major << 5

      # 1-byte widening for value 23 (0x17)
      wire_1 = [(major_shift | 24), 23].pack("C*")
      # Note: major 2, 3, 4, 5 with length 23 need 23 payload bytes, otherwise unexpected_eof fires first
      payload = if [2, 3].include?(major)
        ("\x00" * 23).b
      elsif major == 4
        # 23 null elements
        ("\xF6" * 23).b
      elsif major == 5
        # 23 map entries: 23 * 2 null elements
        ("\xF6" * 46).b
      elsif major == 6
        # tagged value: 1 byte unsigned 0
        "\x00".b
      else
        "".b
      end

      err = assert_raises(CborError, "Major #{major} 1-byte widening must reject") do
        CanonicalCbor.decode(wire_1 + payload)
      end
      assert_equal :non_minimal_integer, err.id

      # 2-byte widening for value 255 (0x00FF)
      wire_2 = [(major_shift | 25), 0x00, 0xFF].pack("C*")
      payload_2 = if [2, 3].include?(major)
        ("\x00" * 255).b
      elsif major == 4
        ("\xF6" * 255).b
      elsif major == 5
        ("\xF6" * 510).b
      elsif major == 6
        "\x00".b
      else
        "".b
      end
      err2 = assert_raises(CborError, "Major #{major} 2-byte widening must reject") do
        CanonicalCbor.decode(wire_2 + payload_2)
      end
      assert_equal :non_minimal_integer, err2.id
    end
  end

  # =========================================================================
  # 8. CVE Resilience Showcase
  # =========================================================================

  def test_cve_1_integer_bounds_and_overflow_defense
    # Ruby transparently promotes to Bignum; ensure all uint bounds strictly enforce [0, MAX_UINT64]
    overflow_wires = [
      hex_to_bytes("5bffffffffffffffff"), # Major 2 length 2^64-1
      hex_to_bytes("7bffffffffffffffff"), # Major 3 length 2^64-1
      hex_to_bytes("9bffffffffffffffff"), # Major 4 count 2^64-1
      hex_to_bytes("bbffffffffffffffff"), # Major 5 count 2^64-1
      hex_to_bytes("ba40000001")          # Major 5 count > max / 2
    ]

    overflow_wires.each do |vec|
      err = assert_raises(CborError) { CanonicalCbor.decode(vec) }
      assert_equal :length_too_large, err.id
    end

    # Value bounds
    assert_raises(CborError) { CborUnsigned.new(1 << 64) }
    assert_raises(CborError) { CborNegative.new(1 << 64) }
    assert_raises(CborError) { CborTag.new(1 << 64, CborNull.new) }
  end

  def test_cve_2_preallocation_oom_guard
    oom_vectors = [
      hex_to_bytes("9a3b9aca00"), # 1 billion element array in 5 bytes
      hex_to_bytes("ba3b9aca00"), # 1 billion entry map in 5 bytes
      hex_to_bytes("5a05f5e100"), # 100 million byte string in 5 bytes
      hex_to_bytes("7a05f5e100")  # 100 million char text in 5 bytes
    ]

    oom_vectors.each do |vec|
      err = assert_raises(CborError) { CanonicalCbor.decode(vec) }
      assert_equal :length_too_large, err.id
    end
  end

  def test_cve_3_deep_nesting_stack_defense
    attack_wire = build_nested_array_wire(129)
    err1 = assert_raises(CborError) { CanonicalCbor.decode(attack_wire) }
    assert_equal :too_deep, err1.id

    attack_val = build_nested_array(129)
    err2 = assert_raises(CborError) { CanonicalCbor.encode_checked(attack_val) }
    assert_equal :encode_too_deep, err2.id
  end

  def test_cve_4_signature_bypass_via_non_canonical_keys
    # Non-minimal integer
    err_non_min = assert_raises(CborError) { CanonicalCbor.decode(hex_to_bytes("1800")) }
    assert_equal :non_minimal_integer, err_non_min.id

    # Out of order keys {"b": 1, "a": 2}
    err_order = assert_raises(CborError) { CanonicalCbor.decode(hex_to_bytes("a2616201616102")) }
    assert_equal :non_canonical_map_order, err_order.id

    # Duplicate keys {"a": 1, "a": 2}
    err_dup = assert_raises(CborError) { CanonicalCbor.decode(hex_to_bytes("a2616101616102")) }
    assert_equal :non_canonical_map_order, err_dup.id

    # Duplicate keys rejected on encode
    dup_map = CborMap.new([
      CborMapEntry.new(CborText.new("role"), CborText.new("guest")),
      CborMapEntry.new(CborText.new("role"), CborText.new("admin"))
    ])
    err_enc_dup = assert_raises(CborError) { CanonicalCbor.encode_checked(dup_map) }
    assert_equal :duplicate_map_key, err_enc_dup.id
  end

  def test_cve_5_encoding_confusion_and_surrogate_pair_defense
    # Decode-side smuggling vectors
    smuggling_vectors = [
      hex_to_bytes("62c080"),     # Overlong 2-byte NUL
      hex_to_bytes("63e080af"),   # Overlong 3-byte slash
      hex_to_bytes("64f08080af"), # Overlong 4-byte slash
      hex_to_bytes("63eda080"),   # Lone surrogate U+D800
      hex_to_bytes("63edbfbf"),   # Lone surrogate U+DFFF
      hex_to_bytes("61ff")        # Invalid 0xFF byte
    ]

    smuggling_vectors.each do |vec|
      err = assert_raises(CborError) { CanonicalCbor.decode(vec) }
      assert_equal :invalid_utf8, err.id
    end

    # Encode-side Ruby encoding confusion defense
    iso_string = "caf\xE9".b.force_encoding(Encoding::ISO_8859_1)
    err_iso = assert_raises(CborError) { CborText.new(iso_string) }
    assert_equal :invalid_utf8, err_iso.id

    ascii_8bit = "raw_ascii".b
    err_ascii = assert_raises(CborError) { CborText.new(ascii_8bit) }
    assert_equal :invalid_utf8, err_ascii.id

    cesu8_surrogate = [0xED, 0xA0, 0x80].pack("C*").force_encoding(Encoding::UTF_8)
    err_cesu8 = assert_raises(CborError) { CborText.new(cesu8_surrogate) }
    assert_equal :invalid_utf8, err_cesu8.id

    # Multi-byte emoji bytesize vs length round-trip verification
    emoji = "🚀"
    assert_equal 1, emoji.length
    assert_equal 4, emoji.bytesize
    t_emoji = CborText.new(emoji)
    enc_emoji = CanonicalCbor.encode_checked(t_emoji)
    assert_equal t_emoji, CanonicalCbor.decode(enc_emoji)
  end

  def test_cve_6_trailing_byte_smuggling_defense
    valid_wire = hex_to_bytes("01") # unsigned 1
    smuggled = valid_wire + "\xDE\xAD\xBE\xEF".b
    err = assert_raises(CborError) { CanonicalCbor.decode(smuggled) }
    assert_equal :trailing_bytes, err.id
  end

  def test_cve_7_secret_payload_leak_defense
    confidential_token = "CONFIDENTIAL_API_KEY_99999_DO_NOT_LEAK"
    bad_map = CborMap.new([
      CborMapEntry.new(CborText.new(confidential_token), CborUnsigned.new(1)),
      CborMapEntry.new(CborText.new(confidential_token), CborUnsigned.new(2))
    ])

    err = assert_raises(CborError) { CanonicalCbor.encode_checked(bad_map) }
    msg = err.message
    refute_includes msg, confidential_token
    assert msg.start_with?("canonical-cbor: ")

    spanned = SpannedMapEntry.new(
      key: CborText.new(confidential_token),
      value: CborText.new("CONFIDENTIAL_PASSWORD_SECRET"),
      value_span: Span.new(0, 10)
    )
    inspected = spanned.inspect
    refute_includes inspected, confidential_token
    refute_includes inspected, "CONFIDENTIAL_PASSWORD_SECRET"
    assert_includes inspected, "<redacted>"
  end

  # =========================================================================
  # 9. Property & Model Equivalence Tests
  # =========================================================================

  def test_map_permutation_invariance
    prng = DeterministicPrng.new(12345)
    entries = [
      CborMapEntry.new(CborText.new("delta"), CborUnsigned.new(4)),
      CborMapEntry.new(CborText.new("alpha"), CborUnsigned.new(1)),
      CborMapEntry.new(CborText.new("charlie"), CborUnsigned.new(3)),
      CborMapEntry.new(CborText.new("bravo"), CborUnsigned.new(2)),
      CborMapEntry.new(CborUnsigned.new(10), CborText.new("num10")),
      CborMapEntry.new(CborUnsigned.new(1), CborText.new("num1"))
    ]

    canonical_bytes = CanonicalCbor.encode_checked(CborMap.new(entries))

    100.times do
      shuffled = entries.dup
      (shuffled.length - 1).downto(1) do |i|
        j = prng.next_int(i + 1)
        shuffled[i], shuffled[j] = shuffled[j], shuffled[i]
      end
      shuffled_bytes = CanonicalCbor.encode_checked(CborMap.new(shuffled))
      assert_equal canonical_bytes, shuffled_bytes
    end
  end

  def test_round_trip_bijection_across_random_values
    prng = DeterministicPrng.new(54321)

    500.times do
      type_choice = prng.next_int(7)
      val = case type_choice
      when 0
        CborUnsigned.new(prng.next_u64)
      when 1
        CborNegative.new(prng.next_u64)
      when 2
        CborBytes.new(prng.next_bytes(prng.next_int(32)))
      when 3
        CborText.new("text_#{prng.next_u32}")
      when 4
        CborArray.new([
          CborUnsigned.new(prng.next_int(100)),
          CborBool.new(prng.next_int(2) == 1),
          CborNull.new
        ])
      when 5
        CborTag.new(prng.next_int(100), CborUnsigned.new(prng.next_int(1000)))
      else
        CborBool.new(prng.next_int(2) == 0)
      end

      encoded = CanonicalCbor.encode_checked(val)
      assert encoded.bytesize <= Constants::MAX_ENCODED_BYTES
      decoded = CanonicalCbor.decode(encoded)
      assert_equal val, decoded
      re_encoded = CanonicalCbor.encode_checked(decoded)
      assert_equal encoded, re_encoded
    end
  end

  def test_push_byte_checked_natural_overflow
    # Array of 2 items: 1 byte header (0x82)
    # First item: CborBytes with 5 bytes header (0x5A + 4 bytes length) + payload
    # Total before second item: 1 + 5 + payload = MAX_ENCODED_BYTES
    payload = "\x00".b * (Constants::MAX_ENCODED_BYTES - 6)
    val = CborArray.new([
      CborBytes.new(payload),
      CborNull.new
    ])
    err = assert_raises(CborError) { CanonicalCbor.encode_checked(val) }
    assert_equal :encode_too_large, err.id
  end

  def test_decode_input_normalization_edge_cases
    dec_arr = CanonicalCbor.decode([0x01])
    assert_equal CborUnsigned.new(1), dec_arr

    err_type = assert_raises(CborError) { CanonicalCbor.decode(12345) }
    assert_equal :unexpected_eof, err_type.id

    err_arr_range = assert_raises(CborError) { CanonicalCbor.decode([256]) }
    assert_equal :unexpected_eof, err_arr_range.id

    # Unknown value kind in encode_into_buffer
    bad_val = Class.new(CborValue) {
      def kind
        :unsupported_kind
      end
    }.new
    err_kind = assert_raises(CborError) { CanonicalCbor.encode_checked(bad_val) }
    assert_equal :unsupported_simple, err_kind.id
  end

  def test_adversarial_byte_mutation_fuzz_loop
    prng = DeterministicPrng.new(99999)

    10_000.times do
      len = prng.next_int(64)
      bytes = prng.next_bytes(len)

      begin
        val = CanonicalCbor.decode(bytes)
        if bytes.bytesize > Constants::MAX_ENCODED_BYTES
          err = assert_raises(CborError) { CanonicalCbor.encode_checked(val) }
          assert_equal :encode_too_large, err.id
        else
          re_encoded = CanonicalCbor.encode_checked(val)
          assert_equal bytes, re_encoded
          val2 = CanonicalCbor.decode(re_encoded)
          assert_equal val, val2
        end
      rescue CborError => e
        assert e.message.start_with?("canonical-cbor: ")
        assert Messages::ERROR_MESSAGES.key?(e.id)
      end
    end
  end
end
