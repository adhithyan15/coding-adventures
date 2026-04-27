# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_http1"

class TestHttp1 < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::Http1::VERSION
  end

  def test_parses_simple_request
    parsed = CodingAdventures::Http1.parse_request_head("GET / HTTP/1.0\r\nHost: example.com\r\n\r\n")

    assert_equal "GET", parsed.head.method
    assert_equal "/", parsed.head.target
    assert_equal CodingAdventures::HttpCore::BodyKind.none, parsed.body_kind
  end

  def test_parses_post_request_with_content_length
    parsed = CodingAdventures::Http1.parse_request_head("POST /submit HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello")

    assert_equal CodingAdventures::HttpCore::BodyKind.content_length(5), parsed.body_kind
  end

  def test_parses_response_head
    parsed = CodingAdventures::Http1.parse_response_head("HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nbody")

    assert_equal 200, parsed.head.status
    assert_equal "OK", parsed.head.reason
    assert_equal CodingAdventures::HttpCore::BodyKind.content_length(4), parsed.body_kind
  end

  def test_uses_until_eof_without_content_length
    parsed = CodingAdventures::Http1.parse_response_head("HTTP/1.0 200 OK\r\nServer: Venture\r\n\r\n")

    assert_equal CodingAdventures::HttpCore::BodyKind.until_eof, parsed.body_kind
  end

  def test_treats_bodyless_statuses_as_bodyless
    parsed = CodingAdventures::Http1.parse_response_head("HTTP/1.1 204 No Content\r\nContent-Length: 12\r\n\r\n")

    assert_equal CodingAdventures::HttpCore::BodyKind.none, parsed.body_kind
  end

  def test_accepts_lf_only_input_and_preserves_duplicate_headers
    parsed = CodingAdventures::Http1.parse_response_head("\nHTTP/1.1 200 OK\nSet-Cookie: a=1\nSet-Cookie: b=2\n\npayload")

    assert_equal %w[a=1 b=2], parsed.head.headers.map(&:value)
  end

  def test_rejects_invalid_header
    error = assert_raises(CodingAdventures::Http1::ParseError) do
      CodingAdventures::Http1.parse_request_head("GET / HTTP/1.1\r\nHost example.com\r\n\r\n")
    end

    assert_match(/invalid HTTP\/1 header/, error.message)
  end

  def test_rejects_invalid_content_length
    error = assert_raises(CodingAdventures::Http1::ParseError) do
      CodingAdventures::Http1.parse_response_head("HTTP/1.1 200 OK\r\nContent-Length: nope\r\n\r\n")
    end

    assert_match(/invalid Content-Length/, error.message)
  end
end
