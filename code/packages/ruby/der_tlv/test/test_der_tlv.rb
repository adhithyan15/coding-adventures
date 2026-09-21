# frozen_string_literal: true

require "simplecov"
SimpleCov.start {
  add_filter "/test/"
  minimum_coverage 95
}

require "json"
require "minitest/autorun"
require "coding_adventures_der_tlv"

class TestDerTlv < Minitest::Test
  DerTlv = CodingAdventures::DerTlv
  FIXTURE = JSON.parse(
    File.read(File.expand_path("../../../../specs/fixtures/der-tlv-v1/cases.json", __dir__), encoding: "UTF-8")
  )

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
    values = FIXTURE.fetch("defaults").merge(test_case.fetch("limits", {}))
    values["max_value_len"] = DerTlv::HOST_MAX if values["max_value_len"] == "host-max"
    values.transform_keys(&:to_sym)
  end

  def element_projection(element, offset)
    {
      "outcome" => "element",
      "element_offset" => offset,
      "tag" => {
        "class" => element.tag.tag_class,
        "constructed" => element.tag.constructed,
        "number" => element.tag.number
      },
      "header_len" => element.header.bytesize,
      "encoded_len" => element.encoded.bytesize,
      "remainder_offset" => offset + element.encoded.bytesize
    }
  end

  def error_projection(error)
    {"outcome" => "error", "error_id" => error.kind, "offset" => error.offset}
  end

  def run_decode(test_case, input, configured)
    if test_case.fetch("operation") == "decode-one"
      element, remainder = DerTlv.decode_one(input, configured)
      projection = element_projection(element, 0)
      assert_equal projection.fetch("remainder_offset"), input.bytesize - remainder.bytesize
      projection
    else
      element_projection(DerTlv.decode_exact(input, configured), 0)
    end
  rescue DerTlv::Error => error
    error_projection(error)
  end

  def run_cursor(test_case, input, configured)
    cursor = DerTlv::Cursor.new(input, configured)
    events = test_case.fetch("actions").map do |action|
      if action == "finish"
        begin
          cursor.finish
          {"outcome" => "finished"}
        rescue DerTlv::Error => error
          error_projection(error)
        end
      else
        offset = input.bytesize - cursor.remaining.bytesize
        begin
          element = cursor.read
          element ? element_projection(element, offset) : {"outcome" => "end"}
        rescue DerTlv::Error => error
          error_projection(error)
        end
      end
    end
    {
      "events" => events,
      "elements_read" => cursor.elements_read,
      "remaining_offset" => input.bytesize - cursor.remaining.bytesize
    }
  end

  FIXTURE.fetch("cases").each do |test_case|
    define_method("test_portable_#{test_case.fetch("id").tr("-", "_")}") do
      input = materialize(test_case.fetch("input"))
      actual = if test_case.fetch("operation") == "cursor"
        run_cursor(test_case, input, limits(test_case))
      else
        run_decode(test_case, input, limits(test_case))
      end
      assert_equal test_case.fetch("expected"), actual
      refute_includes JSON.generate(actual), test_case["redacted_input_hex"] if test_case["redacted_input_hex"]
    end
  end

  def test_views_read_from_the_callers_string
    input = "\x04\x01\x2a".b
    element = DerTlv.decode_exact(input)
    input.setbyte(2, 0x7f)
    assert_equal "\x7f".b, element.value
  end

  def test_limit_validation_and_input_type
    assert_raises(ArgumentError) { DerTlv.decode_exact("".b, max_elements: -1) }
    assert_raises(ArgumentError) { DerTlv.decode_exact("".b, max_tag_number: 0x1_0000_0000) }
    assert_raises(ArgumentError) { DerTlv.decode_exact([]) }
  end

  def test_error_message_is_stable_and_payload_blind
    error = DerTlv::Error.new("truncated-value", 2)
    assert_equal "DER framing error truncated-value at byte 2", error.message
  end
end
