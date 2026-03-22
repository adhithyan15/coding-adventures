"""Tests for the HTTP layer — request/response serialization and client."""

from network_stack.dns import DNSResolver
from network_stack.http import HTTPClient, HTTPRequest, HTTPResponse


class TestHTTPRequest:
    """Tests for HTTP request serialization."""

    def test_serialize_get_request(self) -> None:
        """A GET request should serialize to proper HTTP format."""
        req = HTTPRequest(
            method="GET",
            path="/index.html",
            headers={"Host": "example.com"},
        )
        raw = req.serialize()
        text = raw.decode()

        assert text.startswith("GET /index.html HTTP/1.1\r\n")
        assert "Host: example.com\r\n" in text
        assert text.endswith("\r\n\r\n")

    def test_serialize_post_with_body(self) -> None:
        """A POST request with a body should include Content-Length."""
        req = HTTPRequest(
            method="POST",
            path="/api/data",
            headers={"Host": "api.example.com", "Content-Type": "application/json"},
            body=b'{"key": "value"}',
        )
        raw = req.serialize()
        text = raw.decode()

        assert text.startswith("POST /api/data HTTP/1.1\r\n")
        assert "Content-Length: 16\r\n" in text
        assert text.endswith('{"key": "value"}')

    def test_serialize_no_headers(self) -> None:
        """A request with no headers should still have the blank line."""
        req = HTTPRequest(method="GET", path="/")
        raw = req.serialize()
        text = raw.decode()
        assert "GET / HTTP/1.1\r\n\r\n" in text

    def test_serialize_explicit_content_length(self) -> None:
        """Explicit Content-Length should not be duplicated."""
        req = HTTPRequest(
            method="POST",
            path="/",
            headers={"Content-Length": "5"},
            body=b"hello",
        )
        raw = req.serialize()
        text = raw.decode()
        # Should only appear once
        assert text.count("Content-Length") == 1


class TestHTTPResponse:
    """Tests for HTTP response serialization and deserialization."""

    def test_serialize_response(self) -> None:
        """serialize() should produce valid HTTP response bytes."""
        resp = HTTPResponse(
            status_code=200,
            status_text="OK",
            headers={"Content-Type": "text/html"},
            body=b"<h1>Hello</h1>",
        )
        raw = resp.serialize()
        text = raw.decode()

        assert text.startswith("HTTP/1.1 200 OK\r\n")
        assert "Content-Type: text/html\r\n" in text
        assert text.endswith("<h1>Hello</h1>")

    def test_deserialize_response(self) -> None:
        """deserialize() should parse valid HTTP response bytes."""
        raw = b"HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\n\r\nPage not found"
        resp = HTTPResponse.deserialize(raw)

        assert resp.status_code == 404
        assert resp.status_text == "Not Found"
        assert resp.headers["Content-Type"] == "text/plain"
        assert resp.body == b"Page not found"

    def test_deserialize_no_body(self) -> None:
        """A response with no body should parse correctly."""
        raw = b"HTTP/1.1 204 No Content\r\n\r\n"
        resp = HTTPResponse.deserialize(raw)
        assert resp.status_code == 204
        assert resp.body == b""

    def test_serialize_deserialize_roundtrip(self) -> None:
        """A response should survive serialize -> deserialize roundtrip."""
        original = HTTPResponse(
            status_code=200,
            status_text="OK",
            headers={"Server": "test"},
            body=b"hello",
        )
        raw = original.serialize()
        recovered = HTTPResponse.deserialize(raw)

        assert recovered.status_code == 200
        assert recovered.status_text == "OK"
        assert recovered.body == b"hello"

    def test_deserialize_empty_raises(self) -> None:
        """Empty or invalid data should raise ValueError."""
        try:
            HTTPResponse.deserialize(b"")
            assert False, "Expected ValueError"  # noqa: B011
        except ValueError:
            pass

    def test_response_auto_content_length(self) -> None:
        """serialize() should add Content-Length if body is present."""
        resp = HTTPResponse(body=b"test")
        raw = resp.serialize()
        assert b"Content-Length: 4" in raw


class TestHTTPClient:
    """Tests for the HTTP client URL parsing and request building."""

    def test_build_simple_get(self) -> None:
        """build_request for a simple GET should parse URL correctly."""
        resolver = DNSResolver()
        resolver.add_static("example.com", 0x5DB8D822)
        client = HTTPClient(resolver)

        host, port, req = client.build_request("GET", "http://example.com/page")
        assert host == "example.com"
        assert port == 80
        assert req.method == "GET"
        assert req.path == "/page"
        assert req.headers["Host"] == "example.com"

    def test_build_request_with_port(self) -> None:
        """URLs with explicit ports should be parsed correctly."""
        resolver = DNSResolver()
        client = HTTPClient(resolver)

        host, port, req = client.build_request("GET", "http://localhost:8080/api")
        assert host == "localhost"
        assert port == 8080
        assert req.path == "/api"

    def test_build_request_no_path(self) -> None:
        """URLs without a path should default to /."""
        resolver = DNSResolver()
        client = HTTPClient(resolver)

        host, port, req = client.build_request("GET", "http://example.com")
        assert req.path == "/"

    def test_build_post_with_body(self) -> None:
        """POST requests should include the body."""
        resolver = DNSResolver()
        client = HTTPClient(resolver)

        host, port, req = client.build_request(
            "POST", "http://api.local/submit", body=b"data"
        )
        assert req.method == "POST"
        assert req.body == b"data"

    def test_parse_response(self) -> None:
        """parse_response should delegate to HTTPResponse.deserialize."""
        resolver = DNSResolver()
        client = HTTPClient(resolver)

        raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html></html>"
        resp = client.parse_response(raw)
        assert resp.status_code == 200
        assert resp.body == b"<html></html>"

    def test_https_url_stripped(self) -> None:
        """https:// scheme should be stripped (we don't implement TLS)."""
        resolver = DNSResolver()
        client = HTTPClient(resolver)

        host, port, req = client.build_request("GET", "https://secure.com/path")
        assert host == "secure.com"
        assert req.path == "/path"
