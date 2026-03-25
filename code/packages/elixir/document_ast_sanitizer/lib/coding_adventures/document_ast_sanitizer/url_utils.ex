defmodule CodingAdventures.DocumentAstSanitizer.UrlUtils do
  @moduledoc """
  URL scheme extraction and sanitization utilities.

  ## Why URL sanitization matters

  Browsers silently discard certain characters when parsing URL schemes. An
  attacker can exploit this by inserting invisible characters into a URL like
  `java\x00script:alert(1)` — the browser ignores the null byte and executes
  the JavaScript. Our sanitizer must strip these characters **before** extracting
  the scheme, so the scheme check sees `javascript:alert(1)` and correctly
  rejects it.

  ## Characters stripped

  - C0 control characters U+0000–U+001F (includes null, tab, CR, LF)
  - Zero-width characters: U+200B (zero-width space), U+200C (zero-width
    non-joiner), U+200D (zero-width joiner), U+2060 (word joiner),
    U+FEFF (byte-order mark / zero-width no-break space)

  ## Relative URL handling

  A URL is considered relative (and always allowed) when:
  - It contains no `:` at all
  - The first `:` appears after a `/` or `?` — meaning the colon is inside
    a path or query component, not a scheme separator

  Examples of relative URLs:
  - `/path/to/page`
  - `../relative/link`
  - `page.html?key=value`
  - `//example.com/protocol-relative` — treated as relative (no scheme)
  """

  # C0 control characters: U+0000 through U+001F
  # Zero-width characters often used to bypass scheme detection
  # Note: Elixir's PCRE2 does not support \uXXXX escape in regex — we use
  # Elixir string escapes in the source, which produce literal Unicode chars
  # that PCRE2 happily matches in the character class.
  # U+200B = zero-width space, U+200C = zero-width non-joiner,
  # U+200D = zero-width joiner, U+2060 = word joiner, U+FEFF = BOM
  @strip_pattern Regex.compile!("[\x00-\x1F#{<<0x200B::utf8, 0x200C::utf8, 0x200D::utf8, 0x2060::utf8, 0xFEFF::utf8>>}]", "u")

  @doc """
  Strip C0 control characters and zero-width characters from a URL string.

  These characters are silently ignored by browsers when parsing URL schemes,
  making them useful for bypassing naive scheme checks. We must remove them
  before extracting the scheme.

      iex> alias CodingAdventures.DocumentAstSanitizer.UrlUtils
      iex> UrlUtils.strip_control_chars("java\\x00script:alert(1)")
      "javascript:alert(1)"

      iex> UrlUtils.strip_control_chars("https://example.com")
      "https://example.com"
  """
  @spec strip_control_chars(String.t()) :: String.t()
  def strip_control_chars(url) when is_binary(url) do
    Regex.replace(@strip_pattern, url, "")
  end

  @doc """
  Extract the URL scheme from a (already-stripped) URL string.

  Returns `nil` for relative URLs (no scheme present).

  The scheme is everything before the first `:`, lowercased. If the colon
  appears after a `/` or `?`, the URL is relative and `nil` is returned.

      iex> alias CodingAdventures.DocumentAstSanitizer.UrlUtils
      iex> UrlUtils.extract_scheme("https://example.com")
      "https"

      iex> UrlUtils.extract_scheme("javascript:alert(1)")
      "javascript"

      iex> UrlUtils.extract_scheme("/relative/path")
      nil

      iex> UrlUtils.extract_scheme("page?key:value")
      nil
  """
  @spec extract_scheme(String.t()) :: String.t() | nil
  def extract_scheme(url) when is_binary(url) do
    case :binary.split(url, ":") do
      # No colon found → relative URL
      [_no_colon] ->
        nil

      [before_colon | _rest] ->
        # If the potential scheme contains / or ?, the colon is inside a
        # path/query component, not a scheme separator → relative URL
        if String.contains?(before_colon, ["/", "?"]) do
          nil
        else
          String.downcase(before_colon)
        end
    end
  end

  @doc """
  Check whether a URL's scheme is in the allowed list.

  Returns `true` (allowed) when:
  - `allowed_schemes` is `nil` (allow everything)
  - The URL is relative (no scheme) — relative URLs always pass
  - The lowercased scheme is in `allowed_schemes`

  Returns `false` (blocked) when:
  - The URL has a scheme and that scheme is not in the allowlist

  ## Control-character bypass prevention

  This function strips control characters from the URL before scheme
  extraction. This prevents bypasses like `java\\x00script:alert(1)`.

      iex> alias CodingAdventures.DocumentAstSanitizer.UrlUtils
      iex> UrlUtils.scheme_allowed?("https://example.com", ["http", "https"])
      true

      iex> UrlUtils.scheme_allowed?("javascript:alert(1)", ["http", "https"])
      false

      iex> UrlUtils.scheme_allowed?("/relative/path", ["http", "https"])
      true

      iex> UrlUtils.scheme_allowed?("https://example.com", nil)
      true
  """
  @spec scheme_allowed?(String.t(), [String.t()] | nil) :: boolean()
  def scheme_allowed?(_url, nil), do: true

  def scheme_allowed?(url, allowed_schemes) when is_binary(url) and is_list(allowed_schemes) do
    cleaned = strip_control_chars(url)
    scheme = extract_scheme(cleaned)

    case scheme do
      # Relative URLs always pass
      nil -> true
      # Check scheme against allowlist (already lowercased by extract_scheme)
      s -> s in allowed_schemes
    end
  end
end
