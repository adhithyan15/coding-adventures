defmodule CodingAdventures.HttpCoreTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.HttpCore
  alias CodingAdventures.HttpCore.{BodyKind, Header, HttpVersion, RequestHead, ResponseHead}

  test "version round trip" do
    assert {:ok, version} = HttpVersion.parse("HTTP/1.1")
    assert version.major == 1
    assert version.minor == 1
    assert to_string(version) == "HTTP/1.1"
  end

  test "rejects invalid versions" do
    assert {:error, {:invalid_version, "HTTP/one.one"}} = HttpVersion.parse("HTTP/one.one")
    assert {:error, {:invalid_version, "not-http"}} = HttpVersion.parse("not-http")
  end

  test "find_header is case insensitive" do
    headers = [%Header{name: "Content-Type", value: "text/plain"}]
    assert HttpCore.find_header(headers, "content-type") == "text/plain"
    assert HttpCore.find_header(headers, "content-length") == nil
  end

  test "content helpers" do
    headers = [
      %Header{name: "Content-Length", value: "42"},
      %Header{name: "Content-Type", value: "text/html; charset=utf-8"}
    ]

    assert HttpCore.parse_content_length(headers) == 42
    assert HttpCore.parse_content_type(headers) == {"text/html", "utf-8"}
  end

  test "content helpers reject malformed values" do
    assert HttpCore.parse_content_length([%Header{name: "Content-Length", value: "nope"}]) == nil
    assert HttpCore.parse_content_type([%Header{name: "Content-Type", value: ""}]) == nil
    assert HttpCore.parse_content_type([]) == nil
  end

  test "heads delegate to helpers" do
    request = %RequestHead{
      method: "POST",
      target: "/submit",
      version: %HttpVersion{major: 1, minor: 1},
      headers: [%Header{name: "Content-Length", value: "5"}]
    }

    response = %ResponseHead{
      version: %HttpVersion{major: 1, minor: 0},
      status: 200,
      reason: "OK",
      headers: [%Header{name: "Content-Type", value: "application/json"}]
    }

    assert RequestHead.content_length(request) == 5
    assert RequestHead.header(request, "content-length") == "5"
    assert RequestHead.content_type(request) == nil
    assert ResponseHead.header(response, "content-type") == "application/json"
    assert ResponseHead.content_length(response) == nil
    assert ResponseHead.content_type(response) == {"application/json", nil}
  end

  test "body kind constructors" do
    assert BodyKind.none() == %BodyKind{mode: :none, length: nil}
    assert BodyKind.content_length(7) == %BodyKind{mode: :content_length, length: 7}
    assert BodyKind.until_eof() == %BodyKind{mode: :until_eof, length: nil}
    assert BodyKind.chunked() == %BodyKind{mode: :chunked, length: nil}
  end
end
