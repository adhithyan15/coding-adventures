# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  enable_coverage :branch
  add_filter "/test/"
  minimum_coverage line: 95, branch: 90
end

require "json"
require "minitest/autorun"
require "coding_adventures_der_asn1"

class TestDerAsn1 < Minitest::Test
  DerAsn1 = CodingAdventures::DerAsn1
  DerTlv = CodingAdventures::DerTlv
  ROOT = File.expand_path("../../../../specs/fixtures", __dir__)
  FIXTURE = JSON.parse(File.read(File.join(ROOT, "der-asn1-v1/cases.json"), encoding: "UTF-8"))
  UPSTREAM = JSON.parse(File.read(File.join(ROOT, "der-tlv-v1/cases.json"), encoding: "UTF-8"))

  def materialize(segments)
    segments.map do |segment|
      if segment.key?("hex")
        [segment.fetch("hex")].pack("H*")
      else
        [segment.fetch("repeat_hex")].pack("H*") * segment.fetch("count")
      end
    end.join.b
  end

  def limits(test_case)
    defaults = FIXTURE.fetch("defaults")
    override = test_case.fetch("limits", {})
    der = defaults.fetch("der").merge(override.fetch("der", {}))
    der["max_value_len"] = DerTlv::HOST_MAX if der["max_value_len"] == "host-max"
    defaults.merge(override).merge("der" => der).transform_keys(&:to_sym).tap do |values|
      values[:der] = values[:der].transform_keys(&:to_sym)
    end
  end

  def tag(element)
    {
      "class" => element.tag.tag_class,
      "constructed" => element.tag.constructed,
      "number" => element.tag.number
    }
  end

  def failure(error, scope = "operation-input")
    result = {
      "outcome" => "error",
      "error_id" => error.kind,
      "offset" => error.offset,
      "offset_scope" => scope
    }
    result["framing_error_id"] = error.framing_kind if error.framing_kind
    result
  end

  def verify_upstream(test_case)
    upstream = UPSTREAM.fetch("cases").find { |item| item.fetch("id") == test_case.fetch("der_tlv_case_id") }
    input = materialize(upstream.fetch("input"))
    configured = UPSTREAM.fetch("defaults").merge(upstream.fetch("limits", {}))
    configured["max_value_len"] = DerTlv::HOST_MAX if configured["max_value_len"] == "host-max"
    begin
      element = DerTlv.decode_exact(input, configured.transform_keys(&:to_sym))
      actual = {
        "outcome" => "element",
        "element_offset" => 0,
        "tag" => {
          "class" => element.tag.tag_class,
          "constructed" => element.tag.constructed,
          "number" => element.tag.number
        },
        "header_len" => element.header.bytesize,
        "encoded_len" => element.encoded.bytesize,
        "remainder_offset" => element.encoded.bytesize
      }
    rescue DerTlv::Error => error
      actual = {"outcome" => "error", "error_id" => error.kind, "offset" => error.offset}
    end
    (actual == upstream.fetch("expected")) ? {"outcome" => "upstream"} : actual
  end

  def primitive_result(operation, element, configured, tag_number)
    case operation
    when "decode-boolean"
      {"outcome" => "value", "boolean" => DerAsn1.decode_boolean(element)}
    when "decode-integer", "integer-to-u64"
      integer = DerAsn1.decode_integer(element)
      result = {"outcome" => "value", "signed_hex" => integer.signed_bytes.unpack1("H*"), "negative" => integer.negative?}
      result["u64_decimal"] = integer.to_u64.to_s if operation == "integer-to-u64"
      result
    when "decode-bit-string"
      bits = DerAsn1.decode_bit_string(element)
      {"outcome" => "value", "bytes_hex" => bits.bytes.unpack1("H*"), "unused_bits" => bits.unused_bits, "bit_length" => bits.bit_length}
    when "decode-octet-string"
      {"outcome" => "value", "bytes_hex" => DerAsn1.decode_octet_string(element).unpack1("H*")}
    when "decode-implicit-octet-string"
      {"outcome" => "value", "bytes_hex" => DerAsn1.decode_implicit_octet_string(element, tag_number).unpack1("H*")}
    when "decode-ia5-string"
      {"outcome" => "value", "text" => DerAsn1.decode_ia5_string(element)}
    when "decode-implicit-ia5-string"
      {"outcome" => "value", "text" => DerAsn1.decode_implicit_ia5_string(element, tag_number)}
    when "decode-null"
      DerAsn1.decode_null(element)
      {"outcome" => "value"}
    when "decode-object-identifier", "decode-implicit-object-identifier"
      oid = if operation == "decode-object-identifier"
        DerAsn1.decode_object_identifier(element, configured)
      else
        DerAsn1.decode_implicit_object_identifier(element, tag_number, configured)
      end
      {"outcome" => "value", "bytes_hex" => oid.encoded.unpack1("H*"), "arcs_decimal" => oid.arcs.map(&:to_s), "arc_count" => oid.arc_count}
    else
      raise "unsupported operation #{operation}"
    end
  end

  def cursor_result(test_case, decoder, root)
    cursor = decoder.sequence(root)
    total = cursor.remaining.bytesize
    events = test_case.fetch("actions").map do |action|
      if action == "finish"
        begin
          cursor.finish
          next({"outcome" => "finished"})
        rescue DerAsn1::Error => error
          next failure(error, "container-value")
        end
      end
      active = if action == "read-with-different-limits"
        DerAsn1::Decoder.new(decoder.limits.merge(max_total_elements: decoder.limits[:max_total_elements] + 1))
      else
        decoder
      end
      begin
        child = cursor.read(active)
        if action == "read-nested-sequence"
          raise "nested child required" unless child
          nested = decoder.sequence(child)
          grandchild = nested.read(decoder)
          raise "nested grandchild required" unless grandchild
          nested.finish
          next({"outcome" => "value", "tag" => tag(grandchild), "depth" => grandchild.depth})
        end
        child ? {"outcome" => "value", "tag" => tag(child), "depth" => child.depth} : {"outcome" => "end"}
      rescue DerAsn1::Error => error
        failure(error, "container-value")
      end
    end
    {"outcome" => "value", "elements_read" => decoder.elements_read, "remaining_offset" => total - cursor.remaining.bytesize, "events" => events}
  end

  def run_case(test_case)
    return verify_upstream(test_case) if test_case.key?("der_tlv_case_id")

    configured = limits(test_case)
    decoder = DerAsn1::Decoder.new(configured)
    operation = test_case.fetch("operation")
    begin
      root = decoder.decode_exact(materialize(test_case.fetch("input")))
      case operation
      when "decode-exact"
        {"outcome" => "value", "tag" => tag(root), "header_hex" => root.header.unpack1("H*"), "value_hex" => root.value.unpack1("H*"), "encoded_hex" => root.encoded.unpack1("H*"), "depth" => root.depth, "elements_read" => decoder.elements_read}
      when "cursor-script"
        cursor_result(test_case, decoder, root)
      when "sequence", "set"
        cursor = (operation == "sequence") ? decoder.sequence(root) : decoder.set(root)
        {"outcome" => "value", "elements_read" => decoder.elements_read, "remaining_offset" => root.value.bytesize - cursor.remaining.bytesize}
      when "explicit"
        child = decoder.explicit(root, test_case.fetch("tag_number"))
        {"outcome" => "value", "tag" => tag(child), "value_hex" => child.value.unpack1("H*"), "depth" => child.depth, "elements_read" => decoder.elements_read}
      else
        result = primitive_result(operation, root, configured, test_case["tag_number"])
        result["elements_read"] = decoder.elements_read if test_case.fetch("expected").key?("elements_read")
        result
      end
    rescue DerAsn1::Error => error
      scope = (operation == "explicit" && error.kind == "framing") ? "container-value" : "operation-input"
      failure(error, scope)
    end
  end

  def test_closed_portable_fixture
    assert_equal 122, FIXTURE.fetch("cases").length
    assert_equal 22, FIXTURE.fetch("error_ids").length
    references = FIXTURE.fetch("cases").filter_map { |item| item["der_tlv_case_id"] }
    assert_equal 46, references.uniq.length
    FIXTURE.fetch("cases").each do |test_case|
      actual = run_case(test_case)
      assert_equal test_case.fetch("expected"), actual, test_case.fetch("id")
      refute_includes JSON.generate(actual), test_case["redacted_input_hex"] if test_case["redacted_input_hex"]
    end
  end

  def test_snapshots_are_immutable_and_wrappers_are_sealed
    input = "\x04\x01\x2a".b
    element = DerAsn1::Decoder.new.decode_exact(input)
    input.setbyte(2, 0x7f)
    assert_equal "\x2a".b, element.value
    assert_predicate element.value, :frozen?
    forged = DerAsn1::Element.allocate
    assert_raises(ArgumentError) { DerAsn1.decode_octet_string(forged) }
    spoof = Class.new(DerAsn1::Element) do
      def valid? = true
      def value = "secret".b
    end.allocate
    assert_raises(ArgumentError) { DerAsn1.decode_octet_string(spoof) }
    refute DerAsn1::Element.respond_to?(:new)
    refute DerAsn1::DerInteger.respond_to?(:new)
    refute DerAsn1::DerBitString.respond_to?(:new)
    refute DerAsn1::ObjectIdentifier.respond_to?(:new)
    refute DerAsn1::Cursor.respond_to?(:new)
    refute DerAsn1.const_defined?(:ELEMENT_SEAL, false)
    refute DerAsn1.const_defined?(:VALUE_SEAL, false)
    refute DerAsn1.respond_to?(:wrap)
    refute DerAsn1.respond_to?(:decode_oid)
    bogus = Object.new
    assert_raises(ArgumentError) do
      DerAsn1::Element.__send__(:new, bogus, tag: nil, header: "", value: "secret", encoded: "secret", depth: 0)
    end
    assert_raises(ArgumentError) { DerAsn1::DerInteger.__send__(:new, bogus, "\x00".b, 2) }
    assert_raises(ArgumentError) { DerAsn1::DerBitString.__send__(:new, bogus, "".b, 0, 0) }
    assert_raises(ArgumentError) { DerAsn1::ObjectIdentifier.__send__(:new, bogus, "\x2a".b, [1, 2]) }
    assert_raises(ArgumentError) { DerAsn1::Cursor.__send__(:new, bogus, "".b, 1, DerAsn1::DEFAULT_LIMITS, DerAsn1::Decoder.new) }
    assert_raises(ArgumentError) { DerAsn1.__send__(:decode_oid, bogus, "\x2a".b, 0, 128) }
  end

  def test_limits_reject_unknown_negative_and_wrong_types
    assert_raises(ArgumentError) { DerAsn1::Decoder.new(max_depth: -1) }
    assert_raises(ArgumentError) { DerAsn1::Decoder.new(max_oid_arcs: "1") }
    assert_raises(ArgumentError) { DerAsn1::Decoder.new(unknown: 1) }
    assert_raises(ArgumentError) { DerAsn1::Decoder.new(der: {unknown: 1}) }
    assert_raises(ArgumentError) { DerAsn1::Decoder.new.decode_exact([]) }
    error = assert_raises(DerAsn1::Error) do
      DerAsn1::Decoder.new(der: DerTlv::DEFAULT_LIMITS.merge(max_input_len: 1)).decode_exact("\x05\x00".b)
    end
    assert_equal ["framing", "input-limit-exceeded"], [error.kind, error.framing_kind]

    depth_first = assert_raises(DerAsn1::Error) do
      DerAsn1::Decoder.new(max_depth: 0, der: DerTlv::DEFAULT_LIMITS.merge(max_input_len: 1)).decode_exact("\x05\x00".b)
    end
    assert_equal "depth-limit-exceeded", depth_first.kind
    total_first = assert_raises(DerAsn1::Error) do
      DerAsn1::Decoder.new(max_total_elements: 0, der: DerTlv::DEFAULT_LIMITS.merge(max_input_len: 1)).decode_exact("\x05\x00".b)
    end
    assert_equal "element-limit-exceeded", total_first.kind
  end

  def test_oid_equality_and_collections_are_exact_and_frozen
    decoder = DerAsn1::Decoder.new
    first = DerAsn1.decode_object_identifier(decoder.decode_exact("\x06\x03\x2a\x03\x04".b))
    second = DerAsn1.decode_object_identifier(DerAsn1::Decoder.new.decode_exact("\x06\x03\x2a\x03\x04".b))
    assert_equal first, second
    assert_equal first.hash, second.hash
    assert first.equals([1, 2, 3, 4])
    refute first.equals([1, 2, 3, 5])
    refute_equal first, Object.new
    zero_first = DerAsn1.decode_object_identifier(DerAsn1::Decoder.new.decode_exact("\x06\x01\x0a".b))
    assert_equal [0, 10], zero_first.arcs
    refute_equal first, zero_first
    assert_predicate first.arcs, :frozen?
    assert_predicate first.encoded, :frozen?
    assert_raises(FrozenError) { first.arcs << 5 }
    element = DerAsn1::Decoder.new.decode_exact("\x06\x01\x0a".b)
    error = assert_raises(DerAsn1::Error) { DerAsn1.decode_object_identifier(element, max_oid_arcs: 1) }
    assert_equal "oid-arc-limit-exceeded", error.kind
    malformed = DerAsn1::Decoder.new.decode_exact("\x06\x02\x2a\x80".b)
    precedence = assert_raises(DerAsn1::Error) { DerAsn1.decode_object_identifier(malformed, max_oid_arcs: 2) }
    assert_equal ["non-minimal-object-identifier", 3], [precedence.kind, precedence.offset]
  end

  def test_cursor_retains_der_element_budget_transactionally
    decoder = DerAsn1::Decoder.new(der: DerTlv::DEFAULT_LIMITS.merge(max_elements: 1))
    root = decoder.decode_exact("\x30\x04\x05\x00\x05\x00".b)
    cursor = decoder.sequence(root)

    refute_nil cursor.read(decoder)
    before = cursor.remaining
    error = assert_raises(DerAsn1::Error) { cursor.read(decoder) }
    assert_equal ["framing", "element-limit-exceeded", 2], [error.kind, error.framing_kind, error.offset]
    assert_equal before, cursor.remaining
  end

  def test_cursor_requires_the_creating_decoder_but_end_is_unconditional
    decoder = DerAsn1::Decoder.new
    root = decoder.decode_exact("\x30\x02\x05\x00".b)
    cursor = decoder.sequence(root)
    other = DerAsn1::Decoder.new

    error = assert_raises(DerAsn1::Error) { cursor.read(other) }
    assert_equal "decoder-limit-mismatch", error.kind
    refute_nil cursor.read(decoder)
    assert_nil cursor.read(other)
  end

  def test_error_message_is_payload_blind
    error = DerAsn1::Error.new("framing", 2, "truncated-value")
    assert_equal "DER ASN.1 error framing (truncated-value) at byte 2", error.message
    refute_includes error.message, "secret"
  end
end
