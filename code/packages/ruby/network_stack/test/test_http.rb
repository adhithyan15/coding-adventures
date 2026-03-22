# frozen_string_literal: true

require_relative "test_helper"

class TestHTTPRequest < Minitest::Test
  include CodingAdventures::NetworkStack

  def test_serialize_get_request
    req = HTTPRequest.new(
      method: "GET",
      path: "/index.html",
      headers: {"Host" => "example.com"}
    )

    text = req.serialize
    assert_includes text, "GET /index.html HTTP/1.1\r\n"
    assert_includes text, "Host: example.com\r\n"
    assert text.end_with?("\r\n\r\n")
  end

  def test_serialize_post_request_with_body
    req = HTTPRequest.new(
      method: "POST",
      path: "/api/data",
      headers: {"Content-Type" => "application/json", "Content-Length" => "13"},
      body: '{"key":"val"}'
    )

    text = req.serialize
    assert_includes text, "POST /api/data HTTP/1.1\r\n"
    assert_includes text, '{"key":"val"}'
  end

  def test_deserialize_get_request
    text = "GET /page HTTP/1.1\r\nHost: example.com\r\n\r\n"
    req = HTTPRequest.deserialize(text)

    refute_nil req
    assert_equal "GET", req.method
    assert_equal "/page", req.path
    assert_equal "example.com", req.headers["Host"]
    assert_equal "", req.body
  end

  def test_deserialize_post_with_body
    text = "POST /submit HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello"
    req = HTTPRequest.deserialize(text)

    assert_equal "POST", req.method
    assert_equal "/submit", req.path
    assert_equal "hello", req.body
  end

  def test_round_trip
    original = HTTPRequest.new(
      method: "GET",
      path: "/test",
      headers: {"Host" => "localhost", "Accept" => "text/html"}
    )

    restored = HTTPRequest.deserialize(original.serialize)
    assert_equal "GET", restored.method
    assert_equal "/test", restored.path
    assert_equal "localhost", restored.headers["Host"]
  end

  def test_deserialize_nil_returns_nil
    assert_nil HTTPRequest.deserialize(nil)
    assert_nil HTTPRequest.deserialize("")
  end

  def test_serialize_empty_body
    req = HTTPRequest.new(method: "GET", path: "/")
    text = req.serialize
    assert text.end_with?("\r\n\r\n")
  end
end

class TestHTTPResponse < Minitest::Test
  include CodingAdventures::NetworkStack

  def test_serialize_200_ok
    resp = HTTPResponse.new(
      status_code: 200,
      status_text: "OK",
      headers: {"Content-Type" => "text/html", "Content-Length" => "13"},
      body: "Hello, World!"
    )

    text = resp.serialize
    assert_includes text, "HTTP/1.1 200 OK\r\n"
    assert_includes text, "Content-Type: text/html\r\n"
    assert_includes text, "Hello, World!"
  end

  def test_serialize_404
    resp = HTTPResponse.new(
      status_code: 404,
      status_text: "Not Found",
      body: "Page not found"
    )

    text = resp.serialize
    assert_includes text, "HTTP/1.1 404 Not Found\r\n"
    assert_includes text, "Page not found"
  end

  def test_deserialize_200_ok
    text = "HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello"
    resp = HTTPResponse.deserialize(text)

    refute_nil resp
    assert_equal 200, resp.status_code
    assert_equal "OK", resp.status_text
    assert_equal "5", resp.headers["Content-Length"]
    assert_equal "hello", resp.body
  end

  def test_deserialize_404
    text = "HTTP/1.1 404 Not Found\r\n\r\n"
    resp = HTTPResponse.deserialize(text)

    assert_equal 404, resp.status_code
    assert_equal "Not Found", resp.status_text
  end

  def test_round_trip
    original = HTTPResponse.new(
      status_code: 200,
      status_text: "OK",
      headers: {"Server" => "coding-adventures"},
      body: "test body"
    )

    restored = HTTPResponse.deserialize(original.serialize)
    assert_equal 200, restored.status_code
    assert_equal "OK", restored.status_text
    assert_equal "coding-adventures", restored.headers["Server"]
    assert_equal "test body", restored.body
  end

  def test_deserialize_nil_returns_nil
    assert_nil HTTPResponse.deserialize(nil)
    assert_nil HTTPResponse.deserialize("")
  end

  def test_serialize_empty_body
    resp = HTTPResponse.new(status_code: 204, status_text: "No Content")
    text = resp.serialize
    assert text.end_with?("\r\n\r\n")
  end
end

class TestHTTPClient < Minitest::Test
  include CodingAdventures::NetworkStack

  def test_build_get_request
    client = HTTPClient.new
    hostname, port, request = client.build_request("http://example.com/page")

    assert_equal "example.com", hostname
    assert_equal 80, port
    assert_equal "GET", request.method
    assert_equal "/page", request.path
    assert_equal "example.com", request.headers["Host"]
  end

  def test_build_request_with_port
    client = HTTPClient.new
    hostname, port, request = client.build_request("http://localhost:8080/api")

    assert_equal "localhost", hostname
    assert_equal 8080, port
    assert_equal "/api", request.path
  end

  def test_build_request_root_path
    client = HTTPClient.new
    hostname, port, request = client.build_request("http://example.com")

    assert_equal "example.com", hostname
    assert_equal 80, port
    assert_equal "/", request.path
  end

  def test_build_post_request
    client = HTTPClient.new
    hostname, port, request = client.build_request(
      "http://example.com/api",
      method: "POST",
      body: '{"data":true}',
      content_type: "application/json"
    )

    assert_equal "POST", request.method
    assert_equal '{"data":true}', request.body
    assert_equal "application/json", request.headers["Content-Type"]
    assert_equal "13", request.headers["Content-Length"]
  end

  def test_parse_response
    client = HTTPClient.new
    text = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html>hi</html>"

    resp = client.parse_response(text)
    assert_equal 200, resp.status_code
    assert_equal "<html>hi</html>", resp.body
  end

  def test_dns_resolver_default
    client = HTTPClient.new
    refute_nil client.dns_resolver
    assert_equal [127, 0, 0, 1], client.dns_resolver.resolve("localhost")
  end

  def test_dns_resolver_custom
    resolver = DNSResolver.new
    resolver.add_static("test.com", [1, 2, 3, 4])
    client = HTTPClient.new(dns_resolver: resolver)

    assert_equal [1, 2, 3, 4], client.dns_resolver.resolve("test.com")
  end
end
