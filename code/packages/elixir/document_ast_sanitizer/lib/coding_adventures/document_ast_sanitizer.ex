defmodule CodingAdventures.DocumentAstSanitizer do
  @moduledoc """
  Document AST Sanitizer — policy-driven AST transformation.

  This module provides the public API for sanitizing a Document AST before
  rendering. It slots between the CommonMark parser and the HTML renderer
  in the document pipeline:

  ```
  parse(markdown)        ← CommonmarkParser
        ↓
  sanitize(doc, policy)  ← DocumentAstSanitizer  (this package)
        ↓
  render(doc)            ← DocumentAstToHtml
  ```

  ## Quick start

      alias CodingAdventures.DocumentAstSanitizer
      alias CodingAdventures.DocumentAstSanitizer.Policy

      # User-generated content — strict mode
      safe_doc = DocumentAstSanitizer.sanitize(parsed_doc, Policy.strict())

      # Internal wiki — relaxed mode
      wiki_doc = DocumentAstSanitizer.sanitize(parsed_doc, Policy.relaxed())

      # Trusted documentation — no sanitization
      docs_doc = DocumentAstSanitizer.sanitize(parsed_doc, Policy.passthrough())

      # Custom policy: reserve h1 for page title, allow only https
      custom = %Policy{Policy.strict() | min_heading_level: 2,
                                         allowed_url_schemes: ["https"]}
      result = DocumentAstSanitizer.sanitize(parsed_doc, custom)

  ## Design

  - `sanitize/2` is **pure** — it never mutates the input document.
  - Every node type is handled explicitly (no silent passthrough of unknowns).
  - Empty containers are pruned after sanitization.
  - `DocumentNode` is never dropped (an empty document is valid).

  See `CodingAdventures.DocumentAstSanitizer.Policy` for all policy options
  and `CodingAdventures.DocumentAstSanitizer.Sanitizer` for the transformation
  truth table.
  """

  alias CodingAdventures.DocumentAstSanitizer.Policy
  alias CodingAdventures.DocumentAstSanitizer.Sanitizer

  @doc """
  Sanitize a Document AST node by applying a `SanitizationPolicy`.

  Returns a new `DocumentNode` with all policy violations removed or
  neutralised. The input is never mutated. Callers can safely pass the
  same document through multiple policies.

  ## Parameters

  - `document` — a `%{type: :document, children: [...]}` map (Document AST root)
  - `policy`   — a `%Policy{}` struct; use `Policy.strict()`, `Policy.relaxed()`,
                 or `Policy.passthrough()` as starting points

  ## Examples

      iex> alias CodingAdventures.DocumentAst, as: AST
      iex> alias CodingAdventures.DocumentAstSanitizer
      iex> alias CodingAdventures.DocumentAstSanitizer.Policy
      iex> doc = AST.document([AST.paragraph([AST.raw_inline("html", "<b>raw</b>")])])
      iex> result = DocumentAstSanitizer.sanitize(doc, Policy.strict())
      iex> result.children
      []

      iex> alias CodingAdventures.DocumentAst, as: AST
      iex> alias CodingAdventures.DocumentAstSanitizer
      iex> alias CodingAdventures.DocumentAstSanitizer.Policy
      iex> doc = AST.document([AST.paragraph([AST.text("hello")])])
      iex> DocumentAstSanitizer.sanitize(doc, Policy.passthrough()) == doc
      true
  """
  @spec sanitize(map(), Policy.t()) :: map()
  defdelegate sanitize(document, policy), to: Sanitizer
end
