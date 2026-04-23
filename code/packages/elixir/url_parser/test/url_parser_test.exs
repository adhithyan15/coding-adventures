defmodule CodingAdventures.UrlParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.UrlParser

  # ══════════════════════════════════════════════════════════════════════
  # Basic Parsing Tests
  # ══════════════════════════════════════════════════════════════════════

  describe "parse/1 - basic URLs" do
    test "simple HTTP URL" do
      assert {:ok, url} = UrlParser.parse("http://example.com")
      assert url.scheme == "http"
      assert url.host == "example.com"
      assert url.path == "/"
      assert url.port == nil
    end

    test "HTTPS URL with path" do
      assert {:ok, url} = UrlParser.parse("https://example.com/path/to/resource")
      assert url.scheme == "https"
      assert url.host == "example.com"
      assert url.path == "/path/to/resource"
    end

    test "URL with trailing slash" do
      assert {:ok, url} = UrlParser.parse("http://example.com/")
      assert url.path == "/"
    end

    test "FTP URL" do
      assert {:ok, url} = UrlParser.parse("ftp://files.example.com/pub/readme.txt")
      assert url.scheme == "ftp"
      assert url.host == "files.example.com"
      assert url.path == "/pub/readme.txt"
    end
  end

  # ══════════════════════════════════════════════════════════════════════
  # All Components Tests
  # ══════════════════════════════════════════════════════════════════════

  describe "parse/1 - all components" do
    test "URL with every component" do
      input = "http://user:pass@example.com:8080/path?query=1#frag"
      assert {:ok, url} = UrlParser.parse(input)
      assert url.scheme == "http"
      assert url.userinfo == "user:pass"
      assert url.host == "example.com"
      assert url.port == 8080
      assert url.path == "/path"
      assert url.query == "query=1"
      assert url.fragment == "frag"
      assert url.raw == input
    end

    test "URL with query only" do
      assert {:ok, url} = UrlParser.parse("http://example.com/search?q=hello")
      assert url.query == "q=hello"
      assert url.fragment == nil
    end

    test "URL with fragment only" do
      assert {:ok, url} = UrlParser.parse("http://example.com/page#section")
      assert url.fragment == "section"
      assert url.query == nil
    end

    test "URL with query and fragment" do
      assert {:ok, url} = UrlParser.parse("http://example.com/p?a=1&b=2#top")
      assert url.query == "a=1&b=2"
      assert url.fragment == "top"
    end

    test "URL with userinfo (no password)" do
      assert {:ok, url} = UrlParser.parse("http://admin@example.com/")
      assert url.userinfo == "admin"
      assert url.host == "example.com"
    end

    test "URL with port" do
      assert {:ok, url} = UrlParser.parse("http://example.com:3000/app")
      assert url.port == 3000
      assert url.host == "example.com"
    end

    test "URL with empty query" do
      assert {:ok, url} = UrlParser.parse("http://example.com/path?")
      assert url.query == ""
    end

    test "URL with empty fragment" do
      assert {:ok, url} = UrlParser.parse("http://example.com/path#")
      assert url.fragment == ""
    end
  end

  # ══════════════════════════════════════════════════════════════════════
  # Case Normalization Tests
  # ══════════════════════════════════════════════════════════════════════

  describe "parse/1 - case normalization" do
    test "scheme is lowercased" do
      assert {:ok, url} = UrlParser.parse("HTTP://example.com")
      assert url.scheme == "http"
    end

    test "host is lowercased" do
      assert {:ok, url} = UrlParser.parse("http://EXAMPLE.COM")
      assert url.host == "example.com"
    end

    test "mixed case scheme and host" do
      assert {:ok, url} = UrlParser.parse("HtTpS://ExAmPlE.CoM/Path")
      assert url.scheme == "https"
      assert url.host == "example.com"
      # Path case is preserved
      assert url.path == "/Path"
    end
  end

  # ══════════════════════════════════════════════════════════════════════
  # Effective Port Tests
  # ══════════════════════════════════════════════════════════════════════

  describe "effective_port/1" do
    test "returns explicit port when present" do
      assert {:ok, url} = UrlParser.parse("http://example.com:8080/")
      assert UrlParser.effective_port(url) == 8080
    end

    test "returns default port for HTTP" do
      assert {:ok, url} = UrlParser.parse("http://example.com/")
      assert UrlParser.effective_port(url) == 80
    end

    test "returns default port for HTTPS" do
      assert {:ok, url} = UrlParser.parse("https://example.com/")
      assert UrlParser.effective_port(url) == 443
    end

    test "returns default port for FTP" do
      assert {:ok, url} = UrlParser.parse("ftp://files.example.com/")
      assert UrlParser.effective_port(url) == 21
    end

    test "returns nil for unknown scheme" do
      assert {:ok, url} = UrlParser.parse("gopher://example.com/")
      assert UrlParser.effective_port(url) == nil
    end
  end

  # ══════════════════════════════════════════════════════════════════════
  # Authority Tests
  # ══════════════════════════════════════════════════════════════════════

  describe "authority/1" do
    test "host only" do
      assert {:ok, url} = UrlParser.parse("http://example.com/")
      assert UrlParser.authority(url) == "example.com"
    end

    test "host and port" do
      assert {:ok, url} = UrlParser.parse("http://example.com:8080/")
      assert UrlParser.authority(url) == "example.com:8080"
    end

    test "userinfo, host, and port" do
      assert {:ok, url} = UrlParser.parse("http://user:pass@example.com:8080/")
      assert UrlParser.authority(url) == "user:pass@example.com:8080"
    end

    test "userinfo and host (no port)" do
      assert {:ok, url} = UrlParser.parse("http://user@example.com/")
      assert UrlParser.authority(url) == "user@example.com"
    end
  end

  # ══════════════════════════════════════════════════════════════════════
  # Invalid Input Tests
  # ══════════════════════════════════════════════════════════════════════

  describe "parse/1 - invalid inputs" do
    test "missing scheme" do
      assert {:error, :missing_scheme} = UrlParser.parse("example.com")
    end

    test "empty string" do
      assert {:error, :missing_scheme} = UrlParser.parse("")
    end

    test "just a path" do
      assert {:error, :missing_scheme} = UrlParser.parse("/path/to/thing")
    end

    test "invalid scheme starting with digit" do
      assert {:error, :invalid_scheme} = UrlParser.parse("123://example.com")
    end

    test "invalid scheme with spaces" do
      assert {:error, :invalid_scheme} = UrlParser.parse("ht tp://example.com")
    end
  end

  # ══════════════════════════════════════════════════════════════════════
  # Percent-Encoding Tests
  # ══════════════════════════════════════════════════════════════════════

  describe "percent_encode/1" do
    test "encodes spaces" do
      assert UrlParser.percent_encode("hello world") == "hello%20world"
    end

    test "preserves unreserved characters" do
      assert UrlParser.percent_encode("abc-._~") == "abc-._~"
    end

    test "preserves slashes" do
      assert UrlParser.percent_encode("a/b/c") == "a/b/c"
    end

    test "encodes special characters" do
      assert UrlParser.percent_encode("a&b=c") == "a%26b%3Dc"
    end

    test "encodes unicode characters" do
      encoded = UrlParser.percent_encode("caf\u00e9")
      assert encoded == "caf%C3%A9"
    end

    test "empty string" do
      assert UrlParser.percent_encode("") == ""
    end

    test "all unreserved pass through" do
      unreserved = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~/"
      assert UrlParser.percent_encode(unreserved) == unreserved
    end
  end

  describe "percent_decode/1" do
    test "decodes spaces" do
      assert {:ok, "hello world"} = UrlParser.percent_decode("hello%20world")
    end

    test "passes through regular characters" do
      assert {:ok, "hello"} = UrlParser.percent_decode("hello")
    end

    test "decodes multiple sequences" do
      assert {:ok, "a&b=c"} = UrlParser.percent_decode("a%26b%3Dc")
    end

    test "handles lowercase hex" do
      assert {:ok, "hello world"} = UrlParser.percent_decode("hello%20world")
    end

    test "decodes UTF-8 sequences" do
      assert {:ok, "caf\u00e9"} = UrlParser.percent_decode("caf%C3%A9")
    end

    test "invalid percent encoding - bad hex" do
      assert {:error, :invalid_percent_encoding} = UrlParser.percent_decode("%ZZ")
    end

    test "incomplete percent encoding" do
      assert {:error, :invalid_percent_encoding} = UrlParser.percent_decode("%2")
    end

    test "percent at end" do
      assert {:error, :invalid_percent_encoding} = UrlParser.percent_decode("abc%")
    end

    test "empty string" do
      assert {:ok, ""} = UrlParser.percent_decode("")
    end
  end

  # ══════════════════════════════════════════════════════════════════════
  # Relative Resolution Tests
  # ══════════════════════════════════════════════════════════════════════

  describe "resolve/2" do
    setup do
      {:ok, base} = UrlParser.parse("http://a.com/b/c/d")
      %{base: base}
    end

    test "empty reference returns base without fragment", %{base: base} do
      assert {:ok, resolved} = UrlParser.resolve(base, "")
      assert resolved.scheme == "http"
      assert resolved.host == "a.com"
      assert resolved.path == "/b/c/d"
      assert resolved.fragment == nil
    end

    test "fragment-only reference", %{base: base} do
      assert {:ok, resolved} = UrlParser.resolve(base, "#sec")
      assert resolved.path == "/b/c/d"
      assert resolved.fragment == "sec"
    end

    test "absolute URL reference", %{base: _base} do
      {:ok, base} = UrlParser.parse("http://a.com/b")
      assert {:ok, resolved} = UrlParser.resolve(base, "https://other.com/x")
      assert resolved.scheme == "https"
      assert resolved.host == "other.com"
      assert resolved.path == "/x"
    end

    test "scheme-relative reference", %{base: base} do
      assert {:ok, resolved} = UrlParser.resolve(base, "//other.com/x")
      assert resolved.scheme == "http"
      assert resolved.host == "other.com"
      assert resolved.path == "/x"
    end

    test "absolute path reference", %{base: base} do
      assert {:ok, resolved} = UrlParser.resolve(base, "/new/path")
      assert resolved.scheme == "http"
      assert resolved.host == "a.com"
      assert resolved.path == "/new/path"
    end

    test "relative path reference", %{base: base} do
      assert {:ok, resolved} = UrlParser.resolve(base, "e")
      assert resolved.path == "/b/c/e"
    end

    test "relative path with ..", %{base: base} do
      assert {:ok, resolved} = UrlParser.resolve(base, "../e")
      assert resolved.path == "/b/e"
    end

    test "relative path with query and fragment", %{base: base} do
      assert {:ok, resolved} = UrlParser.resolve(base, "e?q=1#f")
      assert resolved.path == "/b/c/e"
      assert resolved.query == "q=1"
      assert resolved.fragment == "f"
    end

    test "nil base returns error" do
      assert {:error, :relative_without_base} = UrlParser.resolve(nil, "foo")
    end
  end

  # ══════════════════════════════════════════════════════════════════════
  # Dot Segment Removal Tests
  # ══════════════════════════════════════════════════════════════════════

  describe "remove_dot_segments/1" do
    test "removes single dot" do
      assert UrlParser.remove_dot_segments("/a/./b") == "/a/b"
    end

    test "removes double dot" do
      assert UrlParser.remove_dot_segments("/a/b/../c") == "/a/c"
    end

    test "multiple dot segments" do
      assert UrlParser.remove_dot_segments("/a/b/c/./../../g") == "/a/g"
    end

    test "dots beyond root" do
      assert UrlParser.remove_dot_segments("/a/../../c") == "/c"
    end

    test "just a slash" do
      assert UrlParser.remove_dot_segments("/") == "/"
    end

    test "trailing dot" do
      assert UrlParser.remove_dot_segments("/a/b/.") == "/a/b"
    end

    test "trailing double dot" do
      assert UrlParser.remove_dot_segments("/a/b/..") == "/a"
    end
  end

  # ══════════════════════════════════════════════════════════════════════
  # Round-Trip Tests
  # ══════════════════════════════════════════════════════════════════════

  describe "to_url_string/1 - round trip" do
    test "simple URL round-trips" do
      input = "http://example.com/"
      assert {:ok, url} = UrlParser.parse(input)
      assert UrlParser.to_url_string(url) == input
    end

    test "full URL round-trips" do
      input = "http://user:pass@example.com:8080/path?query=1#frag"
      assert {:ok, url} = UrlParser.parse(input)
      assert UrlParser.to_url_string(url) == input
    end

    test "URL with query round-trips" do
      input = "https://example.com/search?q=hello"
      assert {:ok, url} = UrlParser.parse(input)
      assert UrlParser.to_url_string(url) == input
    end
  end

  # ══════════════════════════════════════════════════════════════════════
  # Historical / Mailto-Style Tests
  # ══════════════════════════════════════════════════════════════════════

  describe "parse/1 - scheme:path style" do
    test "mailto URL" do
      assert {:ok, url} = UrlParser.parse("mailto:user@example.com")
      assert url.scheme == "mailto"
    end
  end

  # ══════════════════════════════════════════════════════════════════════
  # IPv6 Tests
  # ══════════════════════════════════════════════════════════════════════

  describe "parse/1 - IPv6" do
    test "IPv6 host" do
      assert {:ok, url} = UrlParser.parse("http://[::1]/path")
      assert url.host == "::1"
      assert url.path == "/path"
    end

    test "IPv6 host with port" do
      assert {:ok, url} = UrlParser.parse("http://[::1]:8080/path")
      assert url.host == "::1"
      assert url.port == 8080
      assert url.path == "/path"
    end

    test "IPv6 full address" do
      assert {:ok, url} = UrlParser.parse("http://[2001:db8::1]/")
      assert url.host == "2001:db8::1"
    end

    test "IPv6 with userinfo" do
      assert {:ok, url} = UrlParser.parse("http://user@[::1]:80/")
      assert url.userinfo == "user"
      assert url.host == "::1"
      assert url.port == 80
    end
  end

  # ══════════════════════════════════════════════════════════════════════
  # Edge Cases
  # ══════════════════════════════════════════════════════════════════════

  describe "parse/1 - edge cases" do
    test "URL with whitespace trimming" do
      assert {:ok, url} = UrlParser.parse("  http://example.com  ")
      assert url.host == "example.com"
    end

    test "deeply nested path" do
      assert {:ok, url} = UrlParser.parse("http://example.com/a/b/c/d/e/f")
      assert url.path == "/a/b/c/d/e/f"
    end

    test "custom scheme" do
      assert {:ok, url} = UrlParser.parse("myapp://deep/link")
      assert url.scheme == "myapp"
    end

    test "scheme with valid special characters" do
      assert {:ok, url} = UrlParser.parse("coap+tcp://example.com/")
      assert url.scheme == "coap+tcp"
    end

    test "URL with multiple query parameters" do
      assert {:ok, url} = UrlParser.parse("http://example.com/?a=1&b=2&c=3")
      assert url.query == "a=1&b=2&c=3"
    end

    test "port zero" do
      assert {:ok, url} = UrlParser.parse("http://example.com:0/")
      assert url.port == 0
    end

    test "port 65535" do
      assert {:ok, url} = UrlParser.parse("http://example.com:65535/")
      assert url.port == 65535
    end

    test "fragment with special characters" do
      assert {:ok, url} = UrlParser.parse("http://example.com/#/route/path")
      assert url.fragment == "/route/path"
    end

    test "query with fragment-like content" do
      assert {:ok, url} = UrlParser.parse("http://example.com/?color=#red")
      # Fragment splits on first #, so query is "color=" and fragment is "red"
      assert url.query == "color="
      assert url.fragment == "red"
    end

    test "preserves raw input" do
      input = "HTTP://Example.COM/Path"
      assert {:ok, url} = UrlParser.parse(input)
      assert url.raw == input
    end
  end
end
