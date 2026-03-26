defmodule CodingAdventures.DocumentHtmlSanitizer do
  @moduledoc """
  HTML String Sanitizer — strip dangerous elements and attributes from HTML.

  This module provides the public API for the HTML-layer sanitizer. It operates
  on **opaque HTML strings** — it has no knowledge of how the HTML was produced.
  No dependency on `document_ast` or any other package: string in, string out.

  ## Pipeline position

  ```
  parse(markdown)        ← CommonmarkParser
        ↓
  sanitize(doc, policy)  ← DocumentAstSanitizer  (preferred — AST stage)
        ↓
  render(doc)            ← DocumentAstToHtml
        ↓
  sanitize_html(html, p) ← DocumentHtmlSanitizer  (this package — belt & suspenders)
  ```

  Both sanitizer stages can be used independently. The HTML sanitizer is also
  useful for HTML arriving from external sources (CMS, APIs, paste-from-editor).

  ## Quick start

      alias CodingAdventures.DocumentHtmlSanitizer
      alias CodingAdventures.DocumentHtmlSanitizer.Policy

      # Untrusted HTML from a third-party API
      safe = DocumentHtmlSanitizer.sanitize_html(cms_html, Policy.html_strict())

      # HTML from authenticated users (allow more through)
      safe = DocumentHtmlSanitizer.sanitize_html(user_html, Policy.html_relaxed())

      # No sanitization — trusted static-site output
      safe = DocumentHtmlSanitizer.sanitize_html(trusted_html, Policy.html_passthrough())

  ## What is sanitized

  - **Dropped elements** (including all content): `<script>`, `<style>`,
    `<iframe>`, `<object>`, `<embed>`, `<applet>`, `<form>`, `<input>`,
    `<button>`, `<select>`, `<textarea>`, `<noscript>`, `<meta>`, `<link>`,
    `<base>` — configurable via `Policy.drop_elements`
  - **Stripped attributes**: all `on*` event handlers, `srcdoc`, `formaction`
  - **URL sanitization**: `href` and `src` values with disallowed schemes
    are cleared to `""`
  - **CSS expression attacks**: `style` attrs with `expression()` or
    dangerous `url()` patterns are removed entirely
  - **HTML comments**: stripped when `drop_comments: true`

  See `CodingAdventures.DocumentHtmlSanitizer.Policy` for full policy options.
  """

  alias CodingAdventures.DocumentHtmlSanitizer.HtmlSanitizer
  alias CodingAdventures.DocumentHtmlSanitizer.Policy

  @doc """
  Sanitize an HTML string by applying the given `HtmlSanitizationPolicy`.

  Returns a new string with dangerous elements and attributes removed.
  The input string is never mutated.

  ## Parameters

  - `html`   — the HTML string to sanitize
  - `policy` — a `%Policy{}` struct; start with `Policy.html_strict()`,
               `Policy.html_relaxed()`, or `Policy.html_passthrough()`

  ## Examples

      iex> alias CodingAdventures.DocumentHtmlSanitizer
      iex> alias CodingAdventures.DocumentHtmlSanitizer.Policy
      iex> DocumentHtmlSanitizer.sanitize_html("<p>Safe</p><script>alert(1)</script>", Policy.html_strict())
      "<p>Safe</p>"

      iex> alias CodingAdventures.DocumentHtmlSanitizer
      iex> alias CodingAdventures.DocumentHtmlSanitizer.Policy
      iex> DocumentHtmlSanitizer.sanitize_html("<script>alert(1)</script>", Policy.html_passthrough())
      "<script>alert(1)</script>"
  """
  @spec sanitize_html(String.t(), Policy.t()) :: String.t()
  defdelegate sanitize_html(html, policy), to: HtmlSanitizer
end
