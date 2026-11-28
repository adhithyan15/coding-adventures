# frozen_string_literal: true

require "json"
require "minitest/autorun"
require "coding_adventures_test_bootstrap"
require "coding_adventures/historical_web"

class TestHistoricalWebAssurance < Minitest::Test
  HW = CodingAdventures::HistoricalWeb
  PBT = CodingAdventures::TestBootstrap::Pbt
  FUZZ = CodingAdventures::TestBootstrap::Fuzz

  def fixture_path
    File.expand_path("../../../../specs/fixtures/historical-web-v1/vectors.json", __dir__)
  end

  def fixture
    parsed = JSON.parse(File.read(fixture_path, encoding: Encoding::UTF_8))
    assert parsed.is_a?(Hash), "fixture root must be an object"
    http = parsed.fetch("http-0.9")
    assert http.fetch("requests").any?, "fixture requests are missing"
    assert http.fetch("responses").any?, "fixture responses are missing"
    assert parsed.fetch("html-tags-1992").any?, "fixture HTML vectors are missing"
    assert parsed.fetch("exchanges").any?, "fixture exchanges are missing"
    parsed
  end

  def hex_bytes(hex)
    assert_kind_of String, hex
    assert_equal 0, hex.length % 2, "odd-length fixture hex #{hex.inspect}"
    assert_match(/\A(?:[0-9a-fA-F]{2})*\z/, hex, "invalid fixture hex #{hex.inspect}")
    [hex].pack("H*").b
  end

  def value_of(result, label)
    assert_predicate result, :ok?, "#{label}: #{result.error.inspect}"
    result.value
  end

  def node_json(document, id)
    node = document.nodes.fetch(id)
    case node.kind
    when :text
      node.text
    when :element
      {
        "tag" => node.tag,
        "attributes" => node.attributes,
        "children" => node.children.map { |child| node_json(document, child) }
      }
    else
      flunk "unknown arena node kind #{node.kind.inspect}"
    end
  end

  def roots_json(document)
    document.roots.map { |root| node_json(document, root) }
  end

  # Traverse the flat arena iteratively: the verifier does not recurse even
  # when the parser returns attacker-controlled nesting depth.
  def assert_arena_well_formed(document)
    nodes = document.nodes
    owners = Array.new(nodes.length, 0)
    seen = Array.new(nodes.length, false)
    pending = document.roots.dup
    document.roots.each do |root|
      assert_operator root, :>=, 0
      assert_operator root, :<, nodes.length
      owners[root] += 1
    end
    cursor = 0
    while cursor < pending.length
      id = pending[cursor]
      cursor += 1
      refute seen[id], "cycle or duplicate reachability at node #{id}"
      seen[id] = true
      node = nodes.fetch(id)
      case node.kind
      when :text
        assert_kind_of String, node.text
      when :element
        refute_empty node.tag
        assert_equal node.tag.downcase, node.tag
        assert_kind_of Hash, node.attributes
        node.attributes.each do |name, value|
          assert_includes %w[href name type], name
          assert value.nil? || value.is_a?(String)
        end
        node.children.each do |child|
          assert_operator child, :>=, 0
          assert_operator child, :<, nodes.length
          owners[child] += 1
          pending << child
        end
      else
        flunk "unknown node kind #{node.kind.inspect}"
      end
    end
    assert seen.all?, "every arena node must be reachable"
    assert owners.all? { |count| count == 1 }, "each arena node must have one owner"
  end

  # Independent small protocol model: frame first, strip the optional CR,
  # then admit only nonempty printable-ASCII address bytes.
  def wire_oracle(wire)
    bytes = wire.bytes
    return nil if bytes.length < 6 || bytes[0, 4] != [71, 69, 84, 32] || bytes[-1] != 10

    finish = bytes.length - 1
    finish -= 1 if finish.positive? && bytes[finish - 1] == 13
    address = bytes[4...finish]
    return nil if address.empty? || address.any? { |byte| byte < 0x21 || byte > 0x7e }

    address.pack("C*").b
  end

  def assert_wire_matches_oracle(wire)
    expected = wire_oracle(wire)
    actual = HW.parse_http09_request(wire)
    if expected.nil?
      refute actual.ok?, "accepted invalid wire #{wire.bytes.inspect}"
      assert_equal :invalid_request, actual.error
    else
      parsed_address = value_of(actual, "valid request")
      assert_equal expected, parsed_address.value
    end
  end

  def test_shared_fixture_replay_is_complete_and_exact
    vectors = fixture
    http = vectors.fetch("http-0.9")
    http.fetch("requests").each do |tc|
      address = value_of(HW::Http09Address.create(tc.fetch("address")), "fixture address")
      encoded = value_of(HW.encode_http09_request(address), "fixture encoding")
      assert_equal "GET #{tc.fetch("address")}\r\n".b, encoded
      wire = hex_bytes(tc.fetch("wire_hex"))
      lf_wire = "GET #{tc.fetch("address")}\n".b
      crlf_wire = "GET #{tc.fetch("address")}\r\n".b
      assert_includes [lf_wire, crlf_wire], wire, "fixture framing for #{tc.fetch("address")}"
      assert_equal tc.fetch("address"), value_of(HW.parse_http09_request(wire), "fixture request").value
    end
    http.fetch("invalid_addresses").each do |address|
      result = HW::Http09Address.create(address)
      refute result.ok?, "accepted invalid address #{address.inspect}"
      assert_equal :invalid_address, result.error
    end
    http.fetch("invalid_request_hex").each do |hex|
      result = HW.parse_http09_request(hex_bytes(hex))
      refute result.ok?, "accepted invalid request #{hex}"
      assert_equal :invalid_request, result.error
    end
    http.fetch("responses").each do |tc|
      wire = hex_bytes(tc.fetch("wire_hex"))
      decoded = value_of(HW.decode_http09_response(wire), "fixture response")
      assert_equal tc.fetch("body").b, decoded.b
      assert_equal Encoding::US_ASCII, decoded.encoding
    end
    http.fetch("invalid_response_hex").each do |hex|
      result = HW.decode_http09_response(hex_bytes(hex))
      refute result.ok?, "accepted non-ASCII response #{hex}"
      assert_equal :non_ascii, result.error
    end

    vectors.fetch("html-tags-1992").each do |tc|
      document = value_of(HW.parse_html_tags_1992(tc.fetch("source")), "HTML #{tc.fetch("id")}")
      assert_arena_well_formed(document)
      assert_equal tc.fetch("tree"), roots_json(document), tc.fetch("id")
    end
    vectors.fetch("exchanges").each do |tc|
      request = value_of(HW.parse_http09_request(hex_bytes(tc.fetch("request_hex"))), "exchange request")
      assert_equal tc.fetch("address"), request.value
      body = value_of(HW.decode_http09_response(hex_bytes(tc.fetch("response_hex"))), "exchange response")
      document = value_of(HW.parse_html_tags_1992(body), "exchange HTML")
      assert_arena_well_formed(document)
      assert_equal tc.fetch("tree"), roots_json(document), tc.fetch("address")
    end
  end

  def test_exhaustive_short_http_payloads_and_both_framings
    cases = 0
    (0..2).each do |length|
      (0...(256**length)).each do |value|
        payload = Array.new(length) { |index| (value >> (8 * index)) & 0xff }.pack("C*").b
        ["\n".b, "\r\n".b].each do |ending|
          assert_wire_matches_oracle("GET ".b + payload + ending)
          cases += 1
        end
      end
    end
    assert_equal 131_586, cases
  end

  def test_seeded_property_engine_checks_printable_address_round_trips
    generator = PBT::Gens.map(PBT::Gens.bytes(1, 32)) do |raw|
      raw.bytes.map { |byte| 0x21 + (byte % 0x5e) }.pack("C*").force_encoding(Encoding::US_ASCII)
    end
    verdict = PBT::Runner.check(0x1992_1103, generator, PBT::Options.new(num_tests: 256, ci: :off)) do |address_text|
      address = HW::Http09Address.create(address_text)
      next false unless address.ok?

      encoded = HW.encode_http09_request(address.value)
      next false unless encoded.ok? && encoded.value == "GET ".b + address_text.b + "\r\n".b

      parsed = HW.parse_http09_request(encoded.value)
      parsed.ok? && parsed.value.value == address_text.b
    end
    assert verdict.passed, verdict.message
    assert_equal 256, verdict.cases_run
  end

  def test_address_and_wire_are_frozen_binary_snapshots_and_forgery_fails_closed
    source = "/snapshot".encode(Encoding::UTF_8)
    address = value_of(HW::Http09Address.create(source), "address snapshot")
    source.replace("/changed")
    assert_equal "/snapshot".b, address.value
    assert_equal Encoding::ASCII_8BIT, address.value.encoding
    assert_predicate address.value, :frozen?
    assert_raises(FrozenError) { address.value.replace("/evil") }

    encoded = value_of(HW.encode_http09_request(address), "request snapshot")
    assert_equal Encoding::ASCII_8BIT, encoded.encoding
    assert_predicate encoded, :frozen?
    assert_equal "GET /snapshot\r\n".b, encoded
    assert_raises(FrozenError) { encoded.setbyte(4, 0x0a) }

    forged = HW::Http09Address.allocate
    result = HW.encode_http09_request(forged)
    refute result.ok?
    assert_equal :invalid_address, result.error
  end

  def test_wire_codec_uses_octets_and_response_decoder_does_not_transcode
    wire = "GET /octets\n".b.force_encoding(Encoding::UTF_16LE)
    parsed = value_of(HW.parse_http09_request(wire), "raw octet request")
    wire.replace("POST /no\n".b.force_encoding(Encoding::UTF_16LE))
    assert_equal "/octets".b, parsed.value
    assert_equal Encoding::ASCII_8BIT, parsed.value.encoding

    utf16_tagged = [0x41, 0x00].pack("C*").force_encoding(Encoding::UTF_16LE)
    decoded = value_of(HW.decode_http09_response(utf16_tagged), "raw octet response")
    refute_same utf16_tagged, decoded
    assert_equal [0x41, 0x00], decoded.bytes
    assert_equal Encoding::US_ASCII, decoded.encoding

    invalid = HW.decode_http09_response("\x80".b)
    refute invalid.ok?
    assert_equal :non_ascii, invalid.error
  end

  def test_html_encoding_domain_and_invalid_sequences
    utf8 = "café".encode(Encoding::UTF_8)
    parsed_utf8 = value_of(HW.parse_html_tags_1992(utf8), "valid UTF-8 source")
    assert_equal ["café"], roots_json(parsed_utf8)

    ascii = "plain ASCII".encode(Encoding::US_ASCII)
    parsed_ascii = value_of(HW.parse_html_tags_1992(ascii), "valid US-ASCII source")
    assert_equal ["plain ASCII"], roots_json(parsed_ascii)

    invalid_utf8 = "\xff".b.force_encoding(Encoding::UTF_8)
    invalid_us_ascii = "\xe9".b.force_encoding(Encoding::US_ASCII)
    unsupported = "plain".encode(Encoding::UTF_16LE)
    [invalid_utf8, invalid_us_ascii, unsupported, "plain".b].each do |source|
      result = HW.parse_html_tags_1992(source)
      refute result.ok?, "accepted source encoding #{source.encoding}"
      assert_equal :invalid_source, result.error
    end
  end

  def test_archived_sgml_empty_marker_raw_text_and_attribute_regressions
    source = "<UL><LI>one<LI>two</UL><P>café &amp; &lt; &unknown;</P>"
    document = value_of(HW.parse_html_tags_1992(source), "SGML omission")
    assert_equal [
      {"tag" => "ul", "attributes" => {}, "children" => [
        {"tag" => "li", "attributes" => {}, "children" => ["one"]},
        {"tag" => "li", "attributes" => {}, "children" => ["two"]}
      ]},
      {"tag" => "p", "attributes" => {}, "children" => []},
      "café & < &unknown;"
    ], roots_json(document)

    anchor_document = value_of(HW.parse_html_tags_1992("<A HREF HREF=/later NAME='first' TYPE=\"UP>DOWN\">x</A>"), "anchor values")
    anchor = anchor_document.nodes.fetch(anchor_document.roots.fetch(0))
    assert_equal :element, anchor.kind
    assert_equal({"href" => nil, "name" => "first", "type" => "UP>DOWN"}, anchor.attributes)

    raw_source = "<XMP>aaaaaé</XMP><P>after</P><LISTING>aaaaaaaé</LISTING><P>end</P>"
    raw = value_of(HW.parse_html_tags_1992(raw_source), "UTF-8 raw text boundaries")
    assert_equal [
      {"tag" => "xmp", "attributes" => {}, "children" => ["aaaaaé"]},
      {"tag" => "p", "attributes" => {}, "children" => []}, "after",
      {"tag" => "listing", "attributes" => {}, "children" => ["aaaaaaaé"]},
      {"tag" => "p", "attributes" => {}, "children" => []}, "end"
    ], roots_json(raw)

    quoted_crlf = value_of(HW.parse_html_tags_1992("<A HREF=\"line1\r\nline2\">x</A>"), "quoted CRLF")
    crlf_anchor = quoted_crlf.nodes.fetch(quoted_crlf.roots.fetch(0))
    assert_equal "line1\nline2", crlf_anchor.attributes.fetch("href")
  end

  def test_deep_nesting_and_large_malformed_text_are_iterative
    depth = 12_000
    source = ("<ADDRESS>" * depth) + ("</A>" * depth) + "tail" + ("</ADDRESS>" * depth)
    document = value_of(HW.parse_html_tags_1992(source), "deep nesting")
    assert_equal depth + 1, document.nodes.length
    assert_arena_well_formed(document)

    malformed = ("<1" * 20_000) + "tail"
    recovered = value_of(HW.parse_html_tags_1992(malformed), "malformed tag-like text")
    assert_equal [malformed], roots_json(recovered)
    assert_arena_well_formed(recovered)
  end

  def test_repository_fuzz_engine_checks_wire_response_and_recoverable_html
    seeds = [
      "GET /\r\n".b, "GET /bad path\n".b, "POST /\n".b,
      "<A HREF='x'>ok</A>".b, "<!--x--><UL><LI>one<LI>two</UL>".b,
      "\xff<A>".b
    ]
    options = FUZZ::Options.new(max_iterations: 2_048, seed: 0x4857_4542_05, max_input_len: 256, base_energy: 10)
    report = FUZZ.run(seeds, options) do |data, handle|
      handle.edge(data.getbyte(0) || 0)
      expected = wire_oracle(data)
      request = HW.parse_http09_request(data)
      wire_matches = if expected.nil?
        !request.ok?
      else
        request.ok? && request.value.value == expected
      end
      next FUZZ::Outcome.crash("wire-oracle-mismatch", ["assurance"]) unless wire_matches

      ascii = data.bytes.all? { |byte| byte <= 0x7f }
      response = HW.decode_http09_response(data)
      response_matches = if ascii
        response.ok? && response.value.bytes == data.bytes && response.value.encoding == Encoding::US_ASCII
      else
        !response.ok? && response.error == :non_ascii
      end
      next FUZZ::Outcome.crash("response-oracle-mismatch", ["assurance"]) unless response_matches

      source = data.dup.force_encoding(Encoding::UTF_8)
      if source.valid_encoding?
        html = HW.parse_html_tags_1992(source)
        next FUZZ::Outcome.crash("valid-utf8-html-error", ["assurance"]) unless html.ok?

        assert_arena_well_formed(html.value)
      end
      FUZZ::Outcome.pass
    end
    assert_equal 2_048, report.iterations
    assert_empty report.crashes
    assert_equal 0, report.timeouts
    assert_equal 0, report.ooms
    assert_predicate report, :clean?
  end
end
