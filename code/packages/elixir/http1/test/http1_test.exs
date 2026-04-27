defmodule CodingAdventures.Http1Test do
  use ExUnit.Case

  alias CodingAdventures.Http1
  alias CodingAdventures.HttpCore.BodyKind
  alias CodingAdventures.HttpCore.HttpVersion

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.Http1)
  end

  test "parses a simple request" do
    assert {:ok, parsed} = Http1.parse_request_head("GET / HTTP/1.0\r\nHost: example.com\r\n\r\n")
    assert parsed.head.method == "GET"
    assert parsed.head.target == "/"
    assert parsed.head.version == %HttpVersion{major: 1, minor: 0}
    assert parsed.body_kind == BodyKind.none()
  end

  test "parses a request with content length" do
    assert {:ok, parsed} =
             Http1.parse_request_head("POST /submit HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello")

    assert parsed.body_kind == BodyKind.content_length(5)
  end

  test "parses a response head" do
    assert {:ok, parsed} =
             Http1.parse_response_head("HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nbody")

    assert parsed.head.status == 200
    assert parsed.head.reason == "OK"
    assert parsed.body_kind == BodyKind.content_length(4)
  end

  test "uses until eof when no content length is present" do
    assert {:ok, parsed} = Http1.parse_response_head("HTTP/1.0 200 OK\r\nServer: Venture\r\n\r\n")
    assert parsed.body_kind == BodyKind.until_eof()
  end

  test "treats bodyless statuses as bodyless" do
    assert {:ok, parsed} =
             Http1.parse_response_head("HTTP/1.1 204 No Content\r\nContent-Length: 12\r\n\r\n")

    assert parsed.body_kind == BodyKind.none()
  end

  test "accepts lf-only input and preserves duplicate headers" do
    assert {:ok, parsed} =
             Http1.parse_response_head(
               "\nHTTP/1.1 200 OK\nSet-Cookie: a=1\nSet-Cookie: b=2\n\npayload"
             )

    assert Enum.map(parsed.head.headers, & &1.value) == ["a=1", "b=2"]
  end

  test "rejects invalid headers" do
    assert {:error, {:invalid_header, _line}} =
             Http1.parse_request_head("GET / HTTP/1.1\r\nHost example.com\r\n\r\n")
  end

  test "rejects invalid content length" do
    assert {:error, {:invalid_content_length, "nope"}} =
             Http1.parse_response_head("HTTP/1.1 200 OK\r\nContent-Length: nope\r\n\r\n")
  end
end
