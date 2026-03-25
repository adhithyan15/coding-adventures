defmodule CodingAdventures.DocumentHtmlSanitizer.UrlUtils do
  @moduledoc """
  URL scheme checking for the HTML sanitizer.

  This is an **independent copy** of the URL utilities — it does not depend on
  `document_ast` or `document_ast_sanitizer`. The HTML sanitizer has zero
  dependencies; duplicating this small module keeps it that way.

  See `CodingAdventures.DocumentAstSanitizer.UrlUtils` for the same
  logic with full literate comments.
  """

  # Build the strip pattern using binary literals for zero-width characters
  # because Elixir's PCRE2 does not support \uXXXX escapes in regex.
  @strip_pattern Regex.compile!(
    "[\\x00-\\x1F#{<<0x200B::utf8, 0x200C::utf8, 0x200D::utf8, 0x2060::utf8, 0xFEFF::utf8>>}]",
    "u"
  )

  @doc """
  Strip C0 control characters and zero-width characters from a URL string.

      iex> alias CodingAdventures.DocumentHtmlSanitizer.UrlUtils
      iex> UrlUtils.strip_control_chars("java\\x00script:alert(1)")
      "javascript:alert(1)"
  """
  @spec strip_control_chars(String.t()) :: String.t()
  def strip_control_chars(url) when is_binary(url) do
    Regex.replace(@strip_pattern, url, "")
  end

  @doc """
  Check whether a URL's scheme is permitted by the allowlist.

  Returns `true` when the URL is relative, when `allowed_schemes` is `nil`,
  or when the scheme (lowercased, after control-char stripping) is in the list.

      iex> alias CodingAdventures.DocumentHtmlSanitizer.UrlUtils
      iex> UrlUtils.scheme_allowed?("https://example.com", ["http", "https"])
      true

      iex> UrlUtils.scheme_allowed?("javascript:alert(1)", ["http", "https"])
      false
  """
  @spec scheme_allowed?(String.t(), [String.t()] | nil) :: boolean()
  def scheme_allowed?(_url, nil), do: true

  def scheme_allowed?(url, allowed) when is_binary(url) and is_list(allowed) do
    cleaned = strip_control_chars(url)

    case :binary.split(cleaned, ":") do
      [_no_colon] ->
        true

      [before_colon | _rest] ->
        if String.contains?(before_colon, ["/", "?"]) do
          true
        else
          String.downcase(before_colon) in allowed
        end
    end
  end
end
