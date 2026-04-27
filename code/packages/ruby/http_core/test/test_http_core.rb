# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_http_core"

HC = CodingAdventures::HttpCore

class HttpCoreTest < Minitest::Test
  def test_version_round_trip
    version = HC::HttpVersion.parse("HTTP/1.1")
    assert_equal 1, version.major
    assert_equal 1, version.minor
    assert_equal "HTTP/1.1", version.to_s
  end

  def test_case_insensitive_header_lookup
    headers = [HC::Header.new("Content-Type", "text/plain")]
    assert_equal "text/plain", HC.find_header(headers, "content-type")
  end

  def test_content_helpers
    headers = [
      HC::Header.new("Content-Length", "42"),
      HC::Header.new("Content-Type", "text/html; charset=utf-8")
    ]
    assert_equal 42, HC.parse_content_length(headers)
    assert_equal ["text/html", "utf-8"], HC.parse_content_type(headers)
  end

  def test_heads_delegate_to_helpers
    request = HC::RequestHead.new("POST", "/submit", HC::HttpVersion.new(1, 1), [HC::Header.new("Content-Length", "5")])
    response = HC::ResponseHead.new(HC::HttpVersion.new(1, 0), 200, "OK", [HC::Header.new("Content-Type", "application/json")])

    assert_equal 5, request.content_length
    assert_equal ["application/json", nil], response.content_type
  end

  def test_body_kind_constructors
    assert_equal HC::BodyKind.new("none", nil), HC::BodyKind.none
    assert_equal HC::BodyKind.new("content-length", 7), HC::BodyKind.content_length(7)
    assert_equal HC::BodyKind.new("until-eof", nil), HC::BodyKind.until_eof
    assert_equal HC::BodyKind.new("chunked", nil), HC::BodyKind.chunked
  end
end
