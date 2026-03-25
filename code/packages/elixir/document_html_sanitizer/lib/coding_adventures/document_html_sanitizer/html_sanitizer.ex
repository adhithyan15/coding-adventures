defmodule CodingAdventures.DocumentHtmlSanitizer.HtmlSanitizer do
  @moduledoc """
  Regex-based HTML sanitizer — string in, string out.

  ## Design rationale

  This sanitizer deliberately avoids a full DOM parser. Regex-based sanitization
  is less precise than DOM-based sanitization, but it is:

  - **Portable** — works in any Elixir environment (no native HTML parser needed)
  - **Predictable** — no surprise behaviour from quirks-mode HTML parsing
  - **Fast** — a few regex passes is cheaper than building and walking a DOM

  The trade-off: some pathological edge cases (e.g. deeply nested malformed HTML)
  may not be handled correctly. For environments with a real HTML parser, a
  DOM-adapter approach is superior. Use this sanitizer as a defence-in-depth
  layer, not as the sole security boundary.

  ## Processing pipeline

  The sanitizer applies transformations in this order:

  1. **Drop comments** — if `drop_comments: true`, strip `<!-- … -->`
  2. **Drop elements** — for each element in `drop_elements`, remove the full
     open tag, all content, and the close tag
  3. **Strip dangerous attributes from remaining tags** — event handlers (`on*`),
     `srcdoc`, `formaction`, and any attrs in `drop_attributes`
  4. **Sanitize URL attributes** — check `href` and `src` against `allowed_url_schemes`
  5. **Sanitize style attributes** — if `sanitize_style_attributes: true`, remove
     `style` attrs containing CSS expression attacks

  ## Regex patterns used

  All patterns are compiled once at module load time for performance.

  ### Comment pattern
  Matches `<!-- ... -->`, including multi-line comments. Uses a non-greedy match
  so that `<!-- a --> <!-- b -->` drops both comments independently.

  ### Element drop pattern
  For each element `tag`, matches `<tag ...>...</tag>` including all whitespace
  and attributes. Case-insensitive to handle `<SCRIPT>`, `<Script>`, etc.
  Self-closing forms like `<meta ... />` are also matched.

  ### Attribute strip pattern
  Strips dangerous attributes from within tag bodies. We match attributes with
  optional values (quoted or unquoted). The pattern is applied inside each tag
  rather than globally to avoid mangling text content that looks like attributes.

  ### URL attribute pattern
  Matches `href="..."` and `src="..."` (with single or double quotes, or unquoted).
  The value is extracted and checked against the scheme allowlist.
  """

  alias CodingAdventures.DocumentHtmlSanitizer.Policy
  alias CodingAdventures.DocumentHtmlSanitizer.UrlUtils

  # ── Pre-compiled patterns ─────────────────────────────────────────────────

  # HTML comment: <!-- anything, including newlines, non-greedy -->
  @comment_pattern ~r/<!--.*?-->/s

  # Match a single HTML opening/closing tag with all its attributes.
  # Used to iterate over tags and sanitize their attribute lists.
  # Captures: [$1 = full tag including < and >]
  # Pattern: a < followed by an optional /, a tag name, attributes, optional /, >
  @tag_pattern ~r/<\/?[a-zA-Z][a-zA-Z0-9\-]*(?:\s[^>]*)?\s*\/?>/

  # Event handler attribute: on<name>=<value> where value can be:
  # - double-quoted string
  # - single-quoted string
  # - unquoted (stops at space or >)
  @event_attr_pattern ~r/\s+on[a-zA-Z]+\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]*)/i

  # srcdoc attribute (any value form)
  @srcdoc_pattern ~r/\s+srcdoc\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]*)/i

  # formaction attribute (any value form)
  @formaction_pattern ~r/\s+formaction\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]*)/i

  # href attribute: captures the value for URL sanitization
  # Groups: [1] = quote char or empty, [2] = the URL value
  @href_pattern ~r/\s+href\s*=\s*("([^"]*)"|'([^']*)'|([^\s>]*))/i

  # src attribute: same structure as href
  @src_pattern ~r/\s+src\s*=\s*("([^"]*)"|'([^']*)'|([^\s>]*))/i

  # style attribute: captures the full value
  @style_pattern ~r/\s+style\s*=\s*("([^"]*)"|'([^']*)'|([^\s>]*))/i

  # CSS expression() attack: expression( anything )
  @css_expression_pattern ~r/expression\s*\(/i

  # CSS url() with non-http/https content
  # Matches url( then optionally whitespace then optionally a quote,
  # then anything that doesn't start with http/https
  @css_url_dangerous_pattern ~r/url\s*\(\s*(?:'|")?(?!https?:)/i

  # ── Public API ────────────────────────────────────────────────────────────

  @doc """
  Sanitize an HTML string by applying the given policy.

  Returns a new string with dangerous elements and attributes removed.
  The transformations are applied in a fixed pipeline order (see module doc).

      iex> alias CodingAdventures.DocumentHtmlSanitizer.HtmlSanitizer
      iex> alias CodingAdventures.DocumentHtmlSanitizer.Policy
      iex> HtmlSanitizer.sanitize_html("<p>Safe</p><script>alert(1)</script>", Policy.html_strict())
      "<p>Safe</p>"
  """
  @spec sanitize_html(String.t(), Policy.t()) :: String.t()
  def sanitize_html(html, %Policy{} = policy) when is_binary(html) do
    html
    |> maybe_drop_comments(policy)
    |> drop_elements(policy.drop_elements)
    |> sanitize_tags(policy)
  end

  # ── Pipeline steps ────────────────────────────────────────────────────────

  # Step 1: Remove HTML comments if policy says so.
  defp maybe_drop_comments(html, %Policy{drop_comments: true}) do
    Regex.replace(@comment_pattern, html, "")
  end

  defp maybe_drop_comments(html, _policy), do: html

  # Step 2: Remove entire elements (open tag + content + close tag).
  #
  # We build a per-element pattern at call time because the element name is
  # variable. Each element is removed independently, from the outer-most match
  # inward. This handles nested elements of the same type imperfectly (PCRE
  # cannot count nesting depth), but covers all common attack vectors.
  defp drop_elements(html, []), do: html

  defp drop_elements(html, [tag | rest]) do
    # Match:
    # - opening tag (case-insensitive, with any attrs): <tag ...>
    # - any content including newlines (non-greedy)
    # - closing tag: </tag>
    # Also matches standalone self-closing tags: <meta ... />
    pattern =
      Regex.compile!(
        "<#{tag}(?:\\s[^>]*)?>.*?</#{tag}>|<#{tag}(?:\\s[^>]*)?\\s*/>|<#{tag}(?:\\s[^>]*)?>",
        "is"
      )

    new_html = Regex.replace(pattern, html, "")
    drop_elements(new_html, rest)
  end

  # Step 3: Walk every HTML tag in the remaining HTML, sanitize attributes.
  defp sanitize_tags(html, policy) do
    Regex.replace(@tag_pattern, html, fn tag_str ->
      tag_str
      |> strip_event_handlers()
      |> strip_specific_attrs(@srcdoc_pattern)
      |> strip_specific_attrs(@formaction_pattern)
      |> strip_policy_attrs(policy.drop_attributes)
      |> sanitize_url_attr(@href_pattern, policy.allowed_url_schemes)
      |> sanitize_url_attr(@src_pattern, policy.allowed_url_schemes)
      |> maybe_sanitize_style(policy.sanitize_style_attributes)
    end)
  end

  # Remove all on* event handler attributes from a single tag string.
  defp strip_event_handlers(tag), do: Regex.replace(@event_attr_pattern, tag, "")

  # Remove a specific attribute (by pre-compiled pattern) from a tag string.
  defp strip_specific_attrs(tag, pattern), do: Regex.replace(pattern, tag, "")

  # Remove attributes named in the policy's drop_attributes list.
  defp strip_policy_attrs(tag, []), do: tag

  defp strip_policy_attrs(tag, [attr | rest]) do
    # Build pattern for this specific attribute name
    pattern = Regex.compile!("\\s+#{Regex.escape(attr)}\\s*=\\s*(?:\"[^\"]*\"|'[^']*'|[^\\s>]*)", "i")
    new_tag = Regex.replace(pattern, tag, "")
    strip_policy_attrs(new_tag, rest)
  end

  # Sanitize a URL attribute (href or src) in a tag string.
  # Finds each occurrence of the attribute, extracts the URL value,
  # checks it against the scheme allowlist, and clears blocked URLs.
  defp sanitize_url_attr(tag, pattern, allowed_schemes) do
    Regex.replace(pattern, tag, fn full_match, _outer_val, dq, sq, uq ->
      # Determine the URL value from whichever capture group is non-empty.
      # The regex captures three groups: double-quoted, single-quoted, unquoted.
      # Only one will be non-empty. We use cond rather than ||: empty string
      # is truthy in Elixir, so "" || sq would return "" instead of sq.
      {url, quote_style} =
        cond do
          dq != "" -> {dq, :double}
          sq != "" -> {sq, :single}
          true -> {uq, :none}
        end

      if UrlUtils.scheme_allowed?(url, allowed_schemes) do
        full_match
      else
        case quote_style do
          :double -> Regex.replace(~r/="[^"]*"/, full_match, "=\"\"")
          :single -> Regex.replace(~r/='[^']*'/, full_match, "=''")
          :none -> Regex.replace(~r/=[^\s>]*/, full_match, "=")
        end
      end
    end)
  end

  # Sanitize style attribute: remove entire style attr if it contains
  # CSS expression attacks.
  defp maybe_sanitize_style(tag, false), do: tag

  defp maybe_sanitize_style(tag, true) do
    Regex.replace(@style_pattern, tag, fn full_match, _outer, dq, sq, uq ->
      # Extract the CSS value from whichever capture group matched.
      # Cannot use ||: empty string is truthy in Elixir, so "" || sq returns "".
      value =
        cond do
          dq != "" -> dq
          sq != "" -> sq
          true -> uq
        end

      if dangerous_css?(value) do
        ""
      else
        full_match
      end
    end)
  end

  # Check if a CSS value string contains known injection patterns.
  defp dangerous_css?(value) do
    Regex.match?(@css_expression_pattern, value) or
      Regex.match?(@css_url_dangerous_pattern, value)
  end
end
