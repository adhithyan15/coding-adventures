# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  enable_coverage :branch
  add_filter "/test/"
  minimum_coverage line: 95, branch: 90
end

require "json"
require "minitest/autorun"
require "coding_adventures_x509_extension"

class TestX509Extension < Minitest::Test
  DerAsn1 = CodingAdventures::DerAsn1
  DerTlv = CodingAdventures::DerTlv
  X509 = CodingAdventures::X509Extension
  ROOT = File.expand_path("../../../../specs/fixtures", __dir__)
  FIXTURE = JSON.parse(File.read(File.join(ROOT, "x509-extension-v1/cases.json"), encoding: "UTF-8"))
  UPSTREAM = JSON.parse(File.read(File.join(ROOT, "der-asn1-v1/cases.json"), encoding: "UTF-8"))

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
    defaults = UPSTREAM.fetch("defaults")
    override = test_case.fetch("limits", {})
    der = defaults.fetch("der").merge(override.fetch("der", {}))
    der["max_value_len"] = DerTlv::HOST_MAX if der["max_value_len"] == "host-max"
    defaults.merge(override).merge("der" => der).transform_keys(&:to_sym).tap do |values|
      values[:der] = values[:der].transform_keys(&:to_sym)
    end
  end

  def attempt(decoder, root)
    extension = X509.decode_extension(decoder, root)
    {
      "outcome" => "value",
      "extension_id_arcs_decimal" => extension.extension_id.arcs.map(&:to_s),
      "critical" => extension.critical,
      "extension_value_hex" => extension.extension_value.unpack1("H*"),
      "elements_read" => decoder.elements_read
    }
  rescue X509::Error => error
    result = {
      "outcome" => "error",
      "error_id" => error.kind,
      "offset" => error.offset,
      "offset_scope" => "extension-element",
      "elements_read" => decoder.elements_read
    }
    result["asn1_error_id"] = error.asn1_kind if error.asn1_kind
    result["framing_error_id"] = error.framing_kind if error.framing_kind
    result
  end

  def run_case(test_case)
    decoder = DerAsn1::Decoder.new(limits(test_case))
    root = decoder.decode_exact(materialize(test_case.fetch("input")))
    return attempt(decoder, root) unless test_case.fetch("operation") == "extension-script"

    {"outcome" => "script", "events" => test_case.fetch("actions").map { attempt(decoder, root) }}
  end

  def test_closed_portable_fixture
    assert_equal 48, FIXTURE.fetch("cases").length
    assert_equal 8, FIXTURE.fetch("error_ids").length
    FIXTURE.fetch("cases").each do |test_case|
      actual = run_case(test_case)
      assert_equal test_case.fetch("expected"), actual, test_case.fetch("id")
      next unless test_case["redacted_input_hex"]

      refute_includes JSON.generate(actual), test_case.fetch("redacted_input_hex")
      decoder = DerAsn1::Decoder.new(limits(test_case))
      root = decoder.decode_exact(materialize(test_case.fetch("input")))
      error = assert_raises(X509::Error) { X509.decode_extension(decoder, root) }
      refute_includes error.message.downcase, test_case.fetch("redacted_input_hex").downcase
    end
  end

  def test_values_are_private_immutable_and_payload_blind
    input = ["30090603551d1104023000"].pack("H*")
    decoder = DerAsn1::Decoder.new
    root = decoder.decode_exact(input)
    extension = X509.decode_extension(decoder, root)
    input.setbyte(-1, 0xff)
    assert_equal "3000", extension.extension_value.unpack1("H*")
    assert_predicate extension.extension_value, :frozen?
    assert_predicate extension.extension_id.arcs, :frozen?
    refute X509::Extension.respond_to?(:new)
    refute X509::Error.respond_to?(:new)
    forged = X509::Extension.allocate
    assert_raises(ArgumentError) { forged.extension_id }
    assert_raises(ArgumentError) { forged.critical }
    assert_raises(ArgumentError) { forged.extension_value }
    assert_raises(ArgumentError) { X509::Extension.__send__(:new, Object.new, nil, false, "secret") }
    assert_raises(ArgumentError) { X509::Error.__send__(:new, Object.new, "bad", 0) }
    assert_raises(ArgumentError) { X509.__send__(:read_child, Object.new, nil, nil, 0, 0) }
    assert_raises(ArgumentError) { X509.__send__(:decode_typed, Object.new, "bad", 0) }
    assert_raises(ArgumentError) { X509.__send__(:fail_with, Object.new, "bad", 0) }
    refute X509.respond_to?(:fail_with)
    refute X509.respond_to?(:read_child)

    hostile_decoder = DerAsn1::Decoder.new
    hostile = hostile_decoder.decode_exact(["30080601800403deadbe"].pack("H*"))
    error = assert_raises(X509::Error) { X509.decode_extension(hostile_decoder, hostile) }
    assert_equal ["invalid-extension-id", "non-minimal-object-identifier", 4],
      [error.kind, error.asn1_kind, error.offset]
    refute_includes error.message.downcase, "deadbe"
  end
end
