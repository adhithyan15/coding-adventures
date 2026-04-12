defmodule CodingAdventures.UrlParser do
  @moduledoc """
  RFC 1738 URL parser with relative resolution and percent-encoding.

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.

  ## Architecture

  The parser uses a single-pass, left-to-right algorithm that mirrors RFC 1738's
  grammar. Instead of regular expressions, we manually walk the input string,
  peeling off components in this order:

      input
        |
        v
      scheme://    <-- split on "://" (or "scheme:" for mailto-style)
        |
        v
      fragment     <-- split on first "#"
        |
        v
      query        <-- split on first "?"
        |
        v
      path         <-- split on first "/"
        |
        v
      userinfo     <-- split on last "@" before host
        |
        v
      host:port    <-- IPv6 brackets or last ":" for port

  This ordering matters because later components (fragment, query) can contain
  characters that look like earlier delimiters. By stripping right-to-left
  within the authority section, we avoid ambiguity.

  ## Percent-Encoding

  RFC 3986 defines "unreserved" characters that need no encoding:

      ALPHA / DIGIT / "-" / "." / "_" / "~"

  We also leave "/" alone since it's a path delimiter. Everything else gets
  encoded as %XX where XX is the uppercase hex of the byte value.

  ## Relative Resolution

  Follows RFC 1808 (later superseded by RFC 3986 Section 5). The algorithm
  checks in order:

  1. Empty reference -> base URL without fragment
  2. Fragment-only -> base URL with new fragment
  3. Has scheme -> treat as absolute
  4. Starts with "//" -> scheme-relative
  5. Starts with "/" -> absolute path (keep base scheme + authority)
  6. Otherwise -> merge paths and remove dot segments
  """

  # ── Struct Definition ─────────────────────────────────────────────────
  #
  # Every parsed URL becomes one of these structs. Fields map directly to
  # RFC 3986's five components plus userinfo (split out from authority)
  # and the original raw string for round-trip fidelity.

  defstruct [:scheme, :userinfo, :host, :port, :path, :query, :fragment, :raw]

  @type t :: %__MODULE__{
          scheme: String.t(),
          userinfo: String.t() | nil,
          host: String.t() | nil,
          port: non_neg_integer() | nil,
          path: String.t(),
          query: String.t() | nil,
          fragment: String.t() | nil,
          raw: String.t()
        }

  # ── Well-Known Default Ports ──────────────────────────────────────────
  #
  # When a URL omits the port, these defaults apply. For example,
  # "http://example.com" implicitly means port 80.
  #
  #   Scheme    Port    Protocol
  #   ──────    ────    ────────
  #   http       80     HyperText Transfer Protocol
  #   https     443     HTTP over TLS
  #   ftp        21     File Transfer Protocol

  @default_ports %{
    "http" => 80,
    "https" => 443,
    "ftp" => 21
  }

  # ── Scheme Validation ─────────────────────────────────────────────────
  #
  # RFC 3986 says a scheme must start with a letter, then any mix of
  # letters, digits, "+", "-", or ".". We validate this with a guard
  # on the first character and a check on the remaining characters.

  @valid_scheme_start_chars Enum.concat([?a..?z, ?A..?Z])
  @valid_scheme_chars Enum.concat([?a..?z, ?A..?Z, ?0..?9]) ++ [?+, ?-, ?.]

  # ── Public API ────────────────────────────────────────────────────────

  @doc """
  Parse a URL string into a `%CodingAdventures.UrlParser{}` struct.

  ## Examples

      iex> CodingAdventures.UrlParser.parse("http://example.com/path")
      {:ok, %CodingAdventures.UrlParser{scheme: "http", host: "example.com", path: "/path"}}

      iex> CodingAdventures.UrlParser.parse("not a url")
      {:error, :missing_scheme}
  """
  @spec parse(String.t()) :: {:ok, t()} | {:error, atom()}
  def parse(input) when is_binary(input) do
    trimmed = String.trim(input)

    case extract_scheme(trimmed) do
      {:ok, scheme_raw, remainder} ->
        scheme = String.downcase(scheme_raw)

        if valid_scheme?(scheme) do
          url = parse_after_scheme(scheme, remainder, trimmed)
          {:ok, url}
        else
          {:error, :invalid_scheme}
        end

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Resolve a relative URL against a base URL (RFC 1808).

  ## Algorithm (decision tree)
  ```
  relative empty?          → base without fragment
  relative starts with #?  → base + new fragment
  relative has scheme?     → absolute (parse it standalone)
  relative starts with //? → scheme-relative (borrow base scheme)
  relative starts with /?  → absolute path (keep base authority)
  otherwise                → merge paths, remove dot segments
  ```

  ## Examples

      iex> {:ok, base} = CodingAdventures.UrlParser.parse("http://a.com/b/c")
      iex> CodingAdventures.UrlParser.resolve(base, "../d")
      {:ok, %CodingAdventures.UrlParser{scheme: "http", host: "a.com", path: "/d"}}
  """
  @spec resolve(t(), String.t()) :: {:ok, t()} | {:error, atom()}
  def resolve(%__MODULE__{} = base, relative) when is_binary(relative) do
    trimmed = String.trim(relative)
    do_resolve(base, trimmed)
  end

  def resolve(nil, _relative), do: {:error, :relative_without_base}

  @doc """
  Return the effective port: explicit port if present, otherwise the
  well-known default for the scheme (http→80, https→443, ftp→21).

  ## Truth Table
  ```
  Explicit Port    Scheme    Result
  ─────────────    ──────    ──────
  8080             http      8080
  nil              http      80
  nil              https     443
  nil              ftp       21
  nil              gopher    nil
  ```
  """
  @spec effective_port(t()) :: non_neg_integer() | nil
  def effective_port(%__MODULE__{port: port_val}) when is_integer(port_val), do: port_val
  def effective_port(%__MODULE__{scheme: scheme}), do: Map.get(@default_ports, scheme)

  @doc """
  Reconstruct the authority component: `[userinfo@]host[:port]`

  ## Examples

      iex> {:ok, url} = CodingAdventures.UrlParser.parse("http://user:pass@host.com:8080/")
      iex> CodingAdventures.UrlParser.authority(url)
      "user:pass@host.com:8080"
  """
  @spec authority(t()) :: String.t()
  def authority(%__MODULE__{} = url) do
    parts = []

    parts =
      if url.userinfo do
        parts ++ [url.userinfo, "@"]
      else
        parts
      end

    parts =
      if url.host do
        parts ++ [url.host]
      else
        parts
      end

    parts =
      if url.port do
        parts ++ [":", Integer.to_string(url.port)]
      else
        parts
      end

    Enum.join(parts)
  end

  @doc """
  Serialize a parsed URL struct back into a URL string.

  Reconstructs scheme://authority/path?query#fragment from the struct fields.
  This is useful for round-trip testing: parse → modify → serialize.
  """
  @spec to_url_string(t()) :: String.t()
  def to_url_string(%__MODULE__{} = url) do
    auth = authority(url)

    result = url.scheme <> "://" <> auth <> url.path

    result =
      if url.query do
        result <> "?" <> url.query
      else
        result
      end

    if url.fragment do
      result <> "#" <> url.fragment
    else
      result
    end
  end

  @doc """
  Percent-encode a string according to RFC 3986.

  Unreserved characters (letters, digits, `-`, `.`, `_`, `~`, `/`) pass
  through unchanged. Every other byte becomes `%XX` where XX is uppercase hex.

  ## Examples

      iex> CodingAdventures.UrlParser.percent_encode("hello world")
      "hello%20world"

      iex> CodingAdventures.UrlParser.percent_encode("a/b")
      "a/b"
  """
  @spec percent_encode(String.t()) :: String.t()
  def percent_encode(input) when is_binary(input) do
    input
    |> :binary.bin_to_list()
    |> Enum.map(&encode_byte/1)
    |> Enum.join()
  end

  @doc """
  Decode percent-encoded sequences back into a UTF-8 string.

  `%XX` sequences are converted to the byte they represent. If the decoded
  bytes don't form valid UTF-8 or a `%` is followed by non-hex characters,
  returns `{:error, :invalid_percent_encoding}`.

  ## Examples

      iex> CodingAdventures.UrlParser.percent_decode("hello%20world")
      {:ok, "hello world"}

      iex> CodingAdventures.UrlParser.percent_decode("%ZZ")
      {:error, :invalid_percent_encoding}
  """
  @spec percent_decode(String.t()) :: {:ok, String.t()} | {:error, atom()}
  def percent_decode(input) when is_binary(input) do
    case do_percent_decode(input, <<>>) do
      {:ok, bytes} ->
        if String.valid?(bytes) do
          {:ok, bytes}
        else
          {:error, :invalid_percent_encoding}
        end

      {:error, _} = err ->
        err
    end
  end

  # ── Scheme Extraction ─────────────────────────────────────────────────
  #
  # We look for "://" first (standard URLs like http://...), then fall back
  # to ":" for scheme-only URLs like "mailto:user@host". If neither is
  # found, the URL has no scheme.

  defp extract_scheme(input) do
    case :binary.match(input, "://") do
      {pos, 3} ->
        scheme = binary_part(input, 0, pos)
        leftover = binary_part(input, pos + 3, byte_size(input) - pos - 3)
        {:ok, scheme, leftover}

      :nomatch ->
        # Try scheme:path form (like mailto:)
        case :binary.match(input, ":") do
          {pos, 1} ->
            scheme = binary_part(input, 0, pos)

            if valid_scheme?(String.downcase(scheme)) do
              leftover = binary_part(input, pos + 1, byte_size(input) - pos - 1)
              {:ok, scheme, leftover}
            else
              {:error, :missing_scheme}
            end

          :nomatch ->
            {:error, :missing_scheme}
        end
    end
  end

  # ── Scheme Validation ─────────────────────────────────────────────────
  #
  # A valid scheme starts with a letter and contains only [a-z0-9+.-].
  # Empty strings are invalid.

  defp valid_scheme?(""), do: false

  defp valid_scheme?(scheme) do
    <<first, remaining::binary>> = scheme

    first in @valid_scheme_start_chars and
      all_scheme_chars?(remaining)
  end

  defp all_scheme_chars?(<<>>), do: true

  defp all_scheme_chars?(<<ch, remaining::binary>>) do
    ch in @valid_scheme_chars and all_scheme_chars?(remaining)
  end

  # ── Post-Scheme Parsing ───────────────────────────────────────────────
  #
  # After extracting the scheme, we parse the remainder left-to-right:
  # 1. Strip fragment (everything after first unencoded #)
  # 2. Strip query (everything after first unencoded ?)
  # 3. Split path from authority (first /)
  # 4. Parse authority into userinfo, host, port

  defp parse_after_scheme(scheme, remainder, raw) do
    # Step 1: Extract fragment
    {before_fragment, frag} = split_on_first(remainder, "#")

    # Step 2: Extract query
    {before_query, query_str} = split_on_first(before_fragment, "?")

    # Step 3: Split authority from path
    {authority_str, path_str} = split_authority_path(before_query)

    # Step 4: Parse authority into userinfo, host, port
    {userinfo_val, host_val, port_val} = parse_authority(authority_str)

    # Normalize: empty host becomes nil, path defaults to "/"
    normalized_host =
      case host_val do
        nil -> nil
        "" -> nil
        h -> String.downcase(h)
      end

    normalized_path = if path_str == "", do: "/", else: path_str

    %__MODULE__{
      scheme: scheme,
      userinfo: userinfo_val,
      host: normalized_host,
      port: port_val,
      path: normalized_path,
      query: query_str,
      fragment: frag,
      raw: raw
    }
  end

  # ── String Splitting Helpers ──────────────────────────────────────────
  #
  # `split_on_first/2` splits a string at the first occurrence of a delimiter.
  # Returns {before, nil} if the delimiter is absent, {before, after} otherwise.
  # This is analogous to Python's str.partition() or Rust's splitn(2).

  defp split_on_first(string, delimiter) do
    case :binary.match(string, delimiter) do
      {pos, len} ->
        before = binary_part(string, 0, pos)
        rest = binary_part(string, pos + len, byte_size(string) - pos - len)
        {before, rest}

      :nomatch ->
        {string, nil}
    end
  end

  # ── Authority / Path Splitting ────────────────────────────────────────
  #
  # The authority ends at the first "/" (which starts the path). If there's
  # no "/", the entire remainder is the authority and the path is empty.
  #
  #   "example.com:8080/path"  →  {"example.com:8080", "/path"}
  #   "example.com"            →  {"example.com", ""}

  defp split_authority_path(string) do
    case :binary.match(string, "/") do
      {pos, _} ->
        auth = binary_part(string, 0, pos)
        path_str = binary_part(string, pos, byte_size(string) - pos)
        {auth, path_str}

      :nomatch ->
        {string, ""}
    end
  end

  # ── Authority Parsing ─────────────────────────────────────────────────
  #
  # Authority = [userinfo@]host[:port]
  #
  # We split on "@" first to isolate userinfo, then parse host:port.
  # For IPv6 addresses like [::1]:8080, we need special bracket handling.

  defp parse_authority(""), do: {nil, nil, nil}

  defp parse_authority(auth_string) do
    # Split userinfo from host+port on the last "@"
    {userinfo_val, host_port} =
      case :binary.match(auth_string, "@") do
        {pos, _} ->
          ui = binary_part(auth_string, 0, pos)
          hp = binary_part(auth_string, pos + 1, byte_size(auth_string) - pos - 1)
          {ui, hp}

        :nomatch ->
          {nil, auth_string}
      end

    {host_val, port_val} = parse_host_port(host_port)
    {userinfo_val, host_val, port_val}
  end

  # ── Host:Port Parsing ─────────────────────────────────────────────────
  #
  # Three cases:
  # 1. IPv6 literal: starts with "[", host is inside brackets, port after "]:"
  # 2. IPv4/hostname with port: last ":" separates host from port (if digits)
  # 3. IPv4/hostname without port: entire string is the host
  #
  # Truth table for ":" handling:
  # ```
  #   Input              Host         Port
  #   ─────              ────         ────
  #   "[::1]:80"         "::1"         80        (IPv6 + port: brackets delimit)
  #   "[::1]"            "::1"         nil       (IPv6, no port)
  #   "host:80"          "host"        80        (IPv4 + port)
  #   "host"             "host"        nil       (no port)
  #   "host:abc"         "host:abc"    nil       (non-numeric = not a port)
  # ```

  defp parse_host_port(string) do
    if String.starts_with?(string, "[") do
      # IPv6 address in brackets
      case :binary.match(string, "]") do
        {pos, _} ->
          # Host is between [ and ]
          ipv6_host = binary_part(string, 1, pos - 1)
          suffix = binary_part(string, pos + 1, byte_size(string) - pos - 1)

          case suffix do
            ":" <> port_str ->
              case parse_port(port_str) do
                {:ok, port_num} -> {ipv6_host, port_num}
                {:error, _} -> {ipv6_host, nil}
              end

            _ ->
              {ipv6_host, nil}
          end

        :nomatch ->
          # Malformed IPv6, treat whole thing as host
          {string, nil}
      end
    else
      # IPv4 or hostname: find the last ":"
      case last_index_of(string, ":") do
        nil ->
          {string, nil}

        pos ->
          host_part = binary_part(string, 0, pos)
          port_str = binary_part(string, pos + 1, byte_size(string) - pos - 1)

          case parse_port(port_str) do
            {:ok, port_num} -> {host_part, port_num}
            {:error, _} -> {string, nil}
          end
      end
    end
  end

  # ── Port Parsing ──────────────────────────────────────────────────────
  #
  # A valid port is a string of digits 0-9 that parses to 0..65535.
  # Anything else is `:invalid_port`.

  defp parse_port(""), do: {:error, :invalid_port}

  defp parse_port(port_str) do
    if Regex.match?(~r/^\d+$/, port_str) do
      port_num = String.to_integer(port_str)

      if port_num >= 0 and port_num <= 65535 do
        {:ok, port_num}
      else
        {:error, :invalid_port}
      end
    else
      {:error, :invalid_port}
    end
  end

  # Find the last occurrence of a character in a string.
  # Returns the byte position or nil.
  defp last_index_of(string, char) do
    case :binary.matches(string, char) do
      [] -> nil
      matches -> matches |> List.last() |> elem(0)
    end
  end

  # ── Relative Resolution ───────────────────────────────────────────────
  #
  # RFC 1808 / RFC 3986 Section 5 algorithm. We check conditions in order
  # from most specific to least specific.

  defp do_resolve(base, "") do
    # Empty reference: return base without fragment
    {:ok, %{base | fragment: nil, raw: to_url_string(%{base | fragment: nil})}}
  end

  defp do_resolve(base, "#" <> frag) do
    # Fragment-only reference
    updated = %{base | fragment: frag}
    {:ok, %{updated | raw: to_url_string(updated)}}
  end

  defp do_resolve(base, relative) do
    # Check if relative has a scheme (meaning it's absolute)
    case :binary.match(relative, "://") do
      {_, _} ->
        # Has scheme — parse as absolute URL
        parse(relative)

      :nomatch ->
        resolve_no_scheme(base, relative)
    end
  end

  defp resolve_no_scheme(base, "//" <> scheme_relative) do
    # Scheme-relative: borrow the scheme from base
    parse(base.scheme <> "://" <> scheme_relative)
  end

  defp resolve_no_scheme(base, "/" <> _ = absolute_path) do
    # Absolute path: keep base scheme + authority, replace path
    {path_str, query_str, frag} = split_path_query_fragment(absolute_path)
    resolved_path = remove_dot_segments(path_str)

    result = %__MODULE__{
      scheme: base.scheme,
      userinfo: base.userinfo,
      host: base.host,
      port: base.port,
      path: resolved_path,
      query: query_str,
      fragment: frag,
      raw: ""
    }

    {:ok, %{result | raw: to_url_string(result)}}
  end

  defp resolve_no_scheme(base, relative) do
    # Relative path: merge with base path, then remove dot segments
    {rel_path, query_str, frag} = split_path_query_fragment(relative)
    merged = merge_paths(base.path, rel_path)
    resolved_path = remove_dot_segments(merged)

    result = %__MODULE__{
      scheme: base.scheme,
      userinfo: base.userinfo,
      host: base.host,
      port: base.port,
      path: resolved_path,
      query: query_str,
      fragment: frag,
      raw: ""
    }

    {:ok, %{result | raw: to_url_string(result)}}
  end

  # ── Path/Query/Fragment Splitting ─────────────────────────────────────
  #
  # For relative resolution, we need to split a relative reference into
  # its path, query, and fragment components.

  defp split_path_query_fragment(input) do
    {before_frag, frag} = split_on_first(input, "#")
    {path_str, query_str} = split_on_first(before_frag, "?")
    {path_str, query_str, frag}
  end

  # ── Path Merging ──────────────────────────────────────────────────────
  #
  # RFC 3986 Section 5.2.3: merge a relative path with a base path.
  # If the base has authority and empty path, prepend "/".
  # Otherwise, replace everything after the last "/" in the base path.
  #
  # Example:
  #   base="/a/b/c" + rel="d"  →  "/a/b/d"
  #   base="/a/b/"  + rel="d"  →  "/a/b/d"

  defp merge_paths(base_path, relative_path) do
    case last_index_of(base_path, "/") do
      nil ->
        "/" <> relative_path

      pos ->
        binary_part(base_path, 0, pos + 1) <> relative_path
    end
  end

  # ── Dot Segment Removal ───────────────────────────────────────────────
  #
  # RFC 3986 Section 5.2.4: remove "." and ".." segments from a path.
  #
  # The algorithm uses a stack-based approach:
  # - Split path into segments on "/"
  # - "." means "current directory" — skip it
  # - ".." means "parent directory" — pop one segment off the stack
  # - Everything else — push onto the stack
  #
  # Examples:
  #   /a/./b    → /a/b       (. is removed)
  #   /a/b/../c → /a/c       (b is removed by ..)
  #   /a/../../c → /c        (can't go above root)

  @doc false
  def remove_dot_segments(path_str) do
    segments = String.split(path_str, "/")

    # Track whether original path had a trailing slash
    trailing_slash = String.ends_with?(path_str, "/") and path_str != "/"

    cleaned =
      Enum.reduce(segments, [], fn segment, stack ->
        case segment do
          "." -> stack
          ".." -> if stack == [], do: [], else: Enum.drop(stack, -1)
          "" -> stack
          seg -> stack ++ [seg]
        end
      end)

    result = "/" <> Enum.join(cleaned, "/")

    if trailing_slash and not String.ends_with?(result, "/") do
      result <> "/"
    else
      result
    end
  end

  # ── Percent-Encoding Internals ────────────────────────────────────────
  #
  # RFC 3986 defines "unreserved" characters that never need encoding:
  #   ALPHA / DIGIT / "-" / "." / "_" / "~"
  # We also preserve "/" since it's a path separator.
  #
  # Everything else is encoded as %XX (uppercase hex).

  # Characters that pass through unchanged during percent-encoding.
  # This set covers unreserved chars plus "/" for path compatibility.
  defp unreserved?(byte) do
    (byte >= ?A and byte <= ?Z) or
      (byte >= ?a and byte <= ?z) or
      (byte >= ?0 and byte <= ?9) or
      byte in [?-, ?., ?_, ?~, ?/]
  end

  defp encode_byte(byte) do
    if unreserved?(byte) do
      <<byte>>
    else
      "%" <> String.upcase(Base.encode16(<<byte>>))
    end
  end

  # ── Percent-Decoding ──────────────────────────────────────────────────
  #
  # Walk the string byte by byte. When we see "%", consume two hex digits
  # and convert to the byte value. Lone "%" at end or with non-hex chars
  # is an error.

  defp do_percent_decode(<<>>, acc), do: {:ok, acc}

  defp do_percent_decode(<<"%" , hi, lo, remaining::binary>>, acc) do
    if hex_char?(hi) and hex_char?(lo) do
      byte = hex_to_int(hi) * 16 + hex_to_int(lo)
      do_percent_decode(remaining, acc <> <<byte>>)
    else
      {:error, :invalid_percent_encoding}
    end
  end

  defp do_percent_decode(<<"%" , _::binary>>, _acc) do
    # "%" at end or with only one char following
    {:error, :invalid_percent_encoding}
  end

  defp do_percent_decode(<<byte, remaining::binary>>, acc) do
    do_percent_decode(remaining, acc <> <<byte>>)
  end

  defp hex_char?(ch) do
    (ch >= ?0 and ch <= ?9) or
      (ch >= ?A and ch <= ?F) or
      (ch >= ?a and ch <= ?f)
  end

  defp hex_to_int(ch) when ch >= ?0 and ch <= ?9, do: ch - ?0
  defp hex_to_int(ch) when ch >= ?A and ch <= ?F, do: ch - ?A + 10
  defp hex_to_int(ch) when ch >= ?a and ch <= ?f, do: ch - ?a + 10
end
