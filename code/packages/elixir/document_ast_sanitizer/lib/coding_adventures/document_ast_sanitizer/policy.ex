defmodule CodingAdventures.DocumentAstSanitizer.Policy do
  @moduledoc """
  Sanitization policy for Document AST nodes.

  A `SanitizationPolicy` is a plain Elixir struct that drives the AST
  sanitizer. It is **pure data** — no functions, no callbacks. This makes
  policies serialisable, composable (via `Map.merge/2` or struct update
  syntax), and easy to reason about.

  ## Presets

  Three named presets cover common scenarios:

  | Preset         | Audience                          | Strictness           |
  |----------------|-----------------------------------|----------------------|
  | `@strict`      | Anonymous users, comment threads  | Most restrictive     |
  | `@relaxed`     | Authenticated users, internal wikis | Moderate            |
  | `@passthrough` | Fully trusted content, static sites | No sanitization     |

  ## Customising a preset

      # Reserve h1 for the page title; force user content to start at h2
      import CodingAdventures.DocumentAstSanitizer.Policy
      custom = %Policy{strict() | min_heading_level: 2}

  ## Policy fields (truth table)

  See the per-field documentation below and the full transformation truth
  table in the `Sanitizer` module.
  """

  @typedoc """
  Controls which `raw_block` formats are allowed through.

  - `:drop_all`       — drop every raw_block node (safest)
  - `:passthrough`    — keep every raw_block node
  - `[string, ...]`   — allowlist of format strings; all others dropped
  """
  @type raw_format_policy :: :drop_all | :passthrough | [String.t()]

  @typedoc "Heading level 1–6 or `:drop` to remove all headings."
  @type heading_level :: 1 | 2 | 3 | 4 | 5 | 6 | :drop

  defstruct [
    # Raw node handling
    allow_raw_block_formats: :passthrough,
    allow_raw_inline_formats: :passthrough,
    # URL scheme policy
    allowed_url_schemes: ["http", "https", "mailto", "ftp"],
    # Node type policy
    drop_links: false,
    drop_images: false,
    transform_image_to_text: false,
    max_heading_level: 6,
    min_heading_level: 1,
    drop_blockquotes: false,
    drop_code_blocks: false,
    transform_code_span_to_text: false
  ]

  @type t :: %__MODULE__{
          allow_raw_block_formats: raw_format_policy(),
          allow_raw_inline_formats: raw_format_policy(),
          allowed_url_schemes: [String.t()] | nil,
          drop_links: boolean(),
          drop_images: boolean(),
          transform_image_to_text: boolean(),
          max_heading_level: heading_level(),
          min_heading_level: 1 | 2 | 3 | 4 | 5 | 6,
          drop_blockquotes: boolean(),
          drop_code_blocks: boolean(),
          transform_code_span_to_text: boolean()
        }

  @doc """
  STRICT preset — for user-generated content (comments, forum posts, chat).

  Drops all raw HTML passthrough. Allows only http, https, mailto URLs.
  Images are converted to alt text. Links are kept but URL-sanitized.
  Headings are clamped to h2–h6 (h1 is reserved for the page title).

  This is the safest starting point. Customise by updating specific fields:

      alias CodingAdventures.DocumentAstSanitizer.Policy
      custom = %Policy{Policy.strict() | max_heading_level: 3}
  """
  @spec strict() :: t()
  def strict do
    %__MODULE__{
      allow_raw_block_formats: :drop_all,
      allow_raw_inline_formats: :drop_all,
      allowed_url_schemes: ["http", "https", "mailto"],
      drop_images: false,
      transform_image_to_text: true,
      min_heading_level: 2,
      max_heading_level: 6,
      drop_links: false,
      drop_blockquotes: false,
      drop_code_blocks: false,
      transform_code_span_to_text: false
    }
  end

  @doc """
  RELAXED preset — for semi-trusted content (authenticated users, internal wikis).

  Allows HTML raw blocks (but not other formats). Allows http, https, mailto,
  ftp URLs. Images pass through unchanged. Headings unrestricted.
  """
  @spec relaxed() :: t()
  def relaxed do
    %__MODULE__{
      allow_raw_block_formats: ["html"],
      allow_raw_inline_formats: ["html"],
      allowed_url_schemes: ["http", "https", "mailto", "ftp"],
      drop_images: false,
      transform_image_to_text: false,
      min_heading_level: 1,
      max_heading_level: 6,
      drop_links: false,
      drop_blockquotes: false,
      drop_code_blocks: false,
      transform_code_span_to_text: false
    }
  end

  @doc """
  PASSTHROUGH preset — for fully trusted content (documentation, static sites).

  No sanitization. Everything passes through unchanged.
  Equivalent to not calling `sanitize/2` at all.
  """
  @spec passthrough() :: t()
  def passthrough do
    %__MODULE__{
      allow_raw_block_formats: :passthrough,
      allow_raw_inline_formats: :passthrough,
      allowed_url_schemes: nil,
      drop_images: false,
      transform_image_to_text: false,
      min_heading_level: 1,
      max_heading_level: 6,
      drop_links: false,
      drop_blockquotes: false,
      drop_code_blocks: false,
      transform_code_span_to_text: false
    }
  end
end
