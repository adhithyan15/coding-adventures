defmodule CodingAdventures.DocumentHtmlSanitizer.Policy do
  @moduledoc """
  Sanitization policy for the HTML string sanitizer.

  `HtmlSanitizationPolicy` is a plain Elixir struct that controls what the
  regex-based HTML sanitizer keeps, transforms, or drops. All fields have
  sensible defaults. Three named presets cover common scenarios:

  | Preset           | Audience                          |
  |------------------|-----------------------------------|
  | `html_strict/0`  | Untrusted HTML from external sources |
  | `html_relaxed/0` | Authenticated users / internal tools |
  | `html_passthrough/0` | No sanitization — trusted content |

  ## Using presets

      alias CodingAdventures.DocumentHtmlSanitizer.Policy

      # Strict — good default for user-supplied HTML
      p = Policy.html_strict()

      # Custom: keep scripts but strip all event handlers
      p = %Policy{Policy.html_strict() | drop_elements: []}
  """

  @typedoc "List of lowercase element names to remove (including content)."
  @type element_list :: [String.t()]

  @typedoc "List of lowercase attribute names to strip from every element."
  @type attribute_list :: [String.t()]

  defstruct [
    drop_elements: [
      "script", "style", "iframe", "object", "embed", "applet",
      "form", "input", "button", "select", "textarea",
      "noscript", "meta", "link", "base"
    ],
    drop_attributes: [],
    allowed_url_schemes: ["http", "https", "mailto", "ftp"],
    drop_comments: true,
    sanitize_style_attributes: true
  ]

  @type t :: %__MODULE__{
          drop_elements: element_list(),
          drop_attributes: attribute_list(),
          allowed_url_schemes: [String.t()] | nil,
          drop_comments: boolean(),
          sanitize_style_attributes: boolean()
        }

  @doc """
  HTML_STRICT preset — for untrusted HTML from external sources.

  Drops the most dangerous element types. Strips all `on*` event handler
  attributes, `srcdoc`, and `formaction`. Sanitizes `href`/`src` URL schemes.
  Drops HTML comments. Strips CSS expressions.

  This is the recommended starting point for HTML from anonymous users,
  CMS content, or third-party APIs.
  """
  @spec html_strict() :: t()
  def html_strict do
    %__MODULE__{
      drop_elements: [
        "script", "style", "iframe", "object", "embed", "applet",
        "form", "input", "button", "select", "textarea",
        "noscript", "meta", "link", "base"
      ],
      drop_attributes: [],
      allowed_url_schemes: ["http", "https", "mailto"],
      drop_comments: true,
      sanitize_style_attributes: true
    }
  end

  @doc """
  HTML_RELAXED preset — for authenticated users / internal tools.

  Drops script, iframe, and plugin elements. Allows `<style>`, `<form>`, and
  `<meta>`. Allows `http`, `https`, `mailto`, and `ftp` URL schemes.
  Preserves comments. Still strips CSS expressions.
  """
  @spec html_relaxed() :: t()
  def html_relaxed do
    %__MODULE__{
      drop_elements: ["script", "iframe", "object", "embed", "applet"],
      drop_attributes: [],
      allowed_url_schemes: ["http", "https", "mailto", "ftp"],
      drop_comments: false,
      sanitize_style_attributes: true
    }
  end

  @doc """
  HTML_PASSTHROUGH preset — no sanitization.

  Passes all HTML through unchanged. Use only for fully trusted content
  (e.g. your own static site generator output). Equivalent to not calling
  `sanitize_html/2` at all.
  """
  @spec html_passthrough() :: t()
  def html_passthrough do
    %__MODULE__{
      drop_elements: [],
      drop_attributes: [],
      allowed_url_schemes: nil,
      drop_comments: false,
      sanitize_style_attributes: false
    }
  end
end
