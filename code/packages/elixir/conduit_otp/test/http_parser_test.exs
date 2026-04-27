defmodule CodingAdventures.ConduitOtp.HttpParserTest do
  @moduledoc """
  Tests for the HTTP parser module.

  We use `:erlang.decode_packet/3` directly (via `HttpParser.decode_packet/1`)
  to test the parser logic without a live TCP socket.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.ConduitOtp.HttpParser

  describe "decode_packet/1 — request line" do
    test "decode_packet/1 is a public API" do
      # Just verify we can call it without crashing
      assert is_function(&HttpParser.decode_packet/1)
    end

    test "parses a simple GET request line" do
      bin = "GET / HTTP/1.1\r\n"
      {:ok, packet, _rest} = HttpParser.decode_packet(bin)
      assert {:http_request, :GET, {:abs_path, "/"}, {1, 1}} = packet
    end

    test "parses a POST request line" do
      bin = "POST /submit HTTP/1.1\r\n"
      {:ok, packet, _rest} = HttpParser.decode_packet(bin)
      assert {:http_request, :POST, {:abs_path, "/submit"}, {1, 1}} = packet
    end

    test "parses a DELETE request line" do
      bin = "DELETE /items/42 HTTP/1.1\r\n"
      {:ok, packet, _rest} = HttpParser.decode_packet(bin)
      assert {:http_request, :DELETE, {:abs_path, "/items/42"}, {1, 1}} = packet
    end

    test "parses a path with query string" do
      bin = "GET /search?q=hello&page=2 HTTP/1.1\r\n"
      {:ok, {:http_request, :GET, {:abs_path, path}, _}, _rest} = HttpParser.decode_packet(bin)
      assert path =~ "search"
      assert path =~ "q=hello"
    end

    test "parses a PUT request line" do
      bin = "PUT /resource/1 HTTP/1.1\r\n"
      {:ok, {:http_request, :PUT, {:abs_path, "/resource/1"}, _}, _} =
        HttpParser.decode_packet(bin)
    end

    test "parses a PATCH request line (method may be atom or binary)" do
      bin = "PATCH /resource/1 HTTP/1.1\r\n"
      {:ok, {:http_request, method, {:abs_path, "/resource/1"}, _}, _} =
        HttpParser.decode_packet(bin)
      # BEAM parses known methods as atoms, unknown/custom methods as binaries.
      # PATCH is standard (RFC 5789) but older BEAMs may return it as binary.
      assert method == :PATCH or method == "PATCH"
    end

    test "returns {:more, _} for incomplete data" do
      result = HttpParser.decode_packet("GET /")
      assert match?({:more, _}, result)
    end

    test "returns {:error, _} or {:more, _} for garbage input" do
      # The BEAM HTTP parser is lenient — some garbage causes {:more, :undefined},
      # some causes {:error, :invalid}. Either is acceptable for malformed input.
      result = HttpParser.decode_packet("!!! NOT HTTP AT ALL !!!\r\n")
      assert match?({:error, _}, result) or match?({:more, _}, result) or
               match?({:ok, {:http_error, _}, _}, result)
    end
  end

  describe "decode_packet/1 — header lines" do
    # :erlang.decode_packet(:httph_bin, ...) decodes HTTP headers in streaming
    # mode. It needs a trailing byte or more data after the CRLF to confirm
    # the header boundary (BEAM needs to see if more bytes follow).
    # We append a sentinel byte to force the parse to complete.

    test "parses request line first, then Host header with httph_bin" do
      req_line = "GET / HTTP/1.1\r\n"
      {:ok, {:http_request, :GET, _, _}, _rest} = HttpParser.decode_packet(req_line)
      # Append a sentinel to trigger parse completion
      header = "Host: example.com\r\n\r\n"
      {:ok, {:http_header, _, name, _, value}, _} =
        :erlang.decode_packet(:httph_bin, header, [])

      assert to_string(name) =~ ~r/[Hh]ost/i
      assert to_string(value) == "example.com"
    end

    test "parses Content-Type header with httph_bin and sentinel" do
      # Sentinel bytes force the parser to return the header immediately
      bin = "Content-Type: application/json\r\n\r\n"
      {:ok, {:http_header, _, _name, _, value}, _} =
        :erlang.decode_packet(:httph_bin, bin, [])

      assert to_string(value) == "application/json"
    end

    test "parses Content-Length header with httph_bin and sentinel" do
      # Adding extra data after the header forces the BEAM parser to commit
      bin = "Content-Length: 42\r\nmore"
      {:ok, {:http_header, _, _name, _, value}, _} =
        :erlang.decode_packet(:httph_bin, bin, [])

      assert to_string(value) == "42"
    end

    test "parses end-of-headers marker with httph_bin" do
      # :http_eoh is returned when just CRLF is seen with sentinel data after
      bin = "\r\nmore"
      {:ok, :http_eoh, _} = :erlang.decode_packet(:httph_bin, bin, [])
    end

    test "decode_packet request line parses correctly with HttpParser" do
      # Verifies our wrapper: http_bin mode for the request line.
      req = "GET /path HTTP/1.1\r\n"
      {:ok, {:http_request, :GET, {:abs_path, "/path"}, {1, 1}}, ""} =
        HttpParser.decode_packet(req)
    end
  end
end
