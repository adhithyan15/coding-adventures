defmodule CodingAdventures.DocumentAstSanitizer.Sanitizer do
  @moduledoc """
  Core AST sanitization logic — a pure, recursive tree transformation.

  ## How it works

  The sanitizer performs a single recursive descent of the Document AST,
  producing a **brand-new tree**. The input is never mutated. Callers can
  safely pass the same document through multiple sanitizers with different
  policies (e.g. preview vs. publish).

  ## Transformation model

  Each node falls into one of four categories:

  1. **Leaf — keep as-is**: `ThematicBreakNode`, `CodeBlockNode` (unless
     dropped), `TextNode`, `CodeSpanNode` (unless converted), `HardBreakNode`,
     `SoftBreakNode`, `RawBlockNode` (if format allowed).

  2. **Container — recurse**: `DocumentNode`, `HeadingNode`, `ParagraphNode`,
     `BlockquoteNode`, `ListNode`, `ListItemNode`, `EmphasisNode`, `StrongNode`,
     `LinkNode`. Children are sanitized first; if all children are dropped, the
     parent is dropped too (except `DocumentNode`).

  3. **Convert**: `ImageNode` → `TextNode` (alt text), `CodeSpanNode` → `TextNode`.

  4. **Drop**: nodes removed entirely from output.

  ## Truth table

  ```
  Node type          Condition                            Action
  ────────────────────────────────────────────────────────────────────────
  document           always                               recurse children
  heading            max_heading_level == :drop           drop
  heading            level < min_heading_level            clamp level up
  heading            level > max_heading_level            clamp level down
  heading            otherwise                            recurse children
  paragraph          always                               recurse children
  code_block         drop_code_blocks == true             drop
  code_block         otherwise                            keep as-is
  blockquote         drop_blockquotes == true             drop
  blockquote         otherwise                            recurse children
  list               always                               recurse children
  list_item          always                               recurse children
  thematic_break     always                               keep as-is
  raw_block          allow_raw_block_formats == :drop_all drop
  raw_block          allow_raw_block_formats == :pass     keep as-is
  raw_block          allow_raw_block_formats == [...]     keep if format in list
  text               always                               keep as-is
  emphasis           always                               recurse children
  strong             always                               recurse children
  code_span          transform_code_span_to_text == true  convert to text node
  code_span          otherwise                            keep as-is
  link               drop_links == true                   promote children to parent
  link               URL scheme not allowed               keep, set destination=""
  link               otherwise                            sanitize URL, recurse
  image              drop_images == true                  drop
  image              transform_image_to_text == true      TextNode{value: alt}
  image              URL scheme not allowed               keep, set destination=""
  image              otherwise                            sanitize URL, keep as-is
  autolink           URL scheme not allowed               drop
  autolink           otherwise                            keep as-is
  raw_inline         allow_raw_inline_formats == :drop_all drop
  raw_inline         allow_raw_inline_formats == :pass     keep as-is
  raw_inline         allow_raw_inline_formats == [...]     keep if format in list
  hard_break         always                               keep as-is
  soft_break         always                               keep as-is
  ```

  ## Empty children pruning

  After recursing into a container node, if **all children** were dropped, the
  container itself is dropped — except `DocumentNode`, which always survives
  (an empty document is valid). This prevents empty `<p></p>` tags in the
  rendered output.

  ## Link child promotion

  When `drop_links: true`, a link's children are not dropped — they are
  **promoted** to the parent container. The caller sees a flattened list of
  inline nodes where the link used to be. This preserves text content while
  removing the hyperlink.

  Example: `[link("https://x.com", [text("click here")])]` becomes
  `[text("click here")]` in the parent's children list.
  """

  alias CodingAdventures.DocumentAstSanitizer.Policy
  alias CodingAdventures.DocumentAstSanitizer.UrlUtils

  @doc """
  Sanitize a Document AST node according to the given policy.

  Returns a new node with all policy violations removed or neutralised.
  The input is never mutated.

      iex> alias CodingAdventures.DocumentAst, as: AST
      iex> alias CodingAdventures.DocumentAstSanitizer.Sanitizer
      iex> alias CodingAdventures.DocumentAstSanitizer.Policy
      iex> doc = AST.document([AST.paragraph([AST.text("hello")])])
      iex> Sanitizer.sanitize(doc, Policy.passthrough())
      %{type: :document, children: [%{type: :paragraph, children: [%{type: :text, value: "hello"}]}]}
  """
  @spec sanitize(map(), Policy.t()) :: map()
  def sanitize(%{type: :document} = node, policy) do
    sanitized_children = sanitize_block_children(node.children, policy)
    %{node | children: sanitized_children}
  end

  # ── Block node handlers ──────────────────────────────────────────────────────

  # ThematicBreak is a leaf — always keep as-is
  defp sanitize_block(%{type: :thematic_break} = node, _policy), do: {:keep, node}

  # CodeBlock: drop if policy says so, otherwise keep as-is (it's a leaf)
  defp sanitize_block(%{type: :code_block}, %Policy{drop_code_blocks: true}),
    do: {:drop, nil}

  defp sanitize_block(%{type: :code_block} = node, _policy), do: {:keep, node}

  # Blockquote: drop entire subtree if policy says drop
  defp sanitize_block(%{type: :blockquote}, %Policy{drop_blockquotes: true}),
    do: {:drop, nil}

  defp sanitize_block(%{type: :blockquote} = node, policy) do
    kids = sanitize_block_children(node.children, policy)
    if kids == [], do: {:drop, nil}, else: {:keep, %{node | children: kids}}
  end

  # RawBlock: check format policy
  defp sanitize_block(%{type: :raw_block}, %Policy{allow_raw_block_formats: :drop_all}),
    do: {:drop, nil}

  defp sanitize_block(%{type: :raw_block} = node, %Policy{allow_raw_block_formats: :passthrough}),
    do: {:keep, node}

  defp sanitize_block(%{type: :raw_block, format: fmt} = node, %Policy{
         allow_raw_block_formats: formats
       })
       when is_list(formats) do
    if fmt in formats, do: {:keep, node}, else: {:drop, nil}
  end

  # Heading: check level clamping / drop policy
  defp sanitize_block(%{type: :heading}, %Policy{max_heading_level: :drop}),
    do: {:drop, nil}

  defp sanitize_block(%{type: :heading, level: lvl} = node, policy) do
    clamped =
      lvl
      |> max(policy.min_heading_level)
      |> min(policy.max_heading_level)

    kids = sanitize_inline_children(node.children, policy)
    if kids == [], do: {:drop, nil}, else: {:keep, %{node | level: clamped, children: kids}}
  end

  # Paragraph: recurse into children; drop if empty after sanitization
  defp sanitize_block(%{type: :paragraph} = node, policy) do
    kids = sanitize_inline_children(node.children, policy)
    if kids == [], do: {:drop, nil}, else: {:keep, %{node | children: kids}}
  end

  # List: recurse into list_item children
  defp sanitize_block(%{type: :list} = node, policy) do
    kids = sanitize_block_children(node.children, policy)
    if kids == [], do: {:drop, nil}, else: {:keep, %{node | children: kids}}
  end

  # ListItem: recurse into block children
  defp sanitize_block(%{type: :list_item} = node, policy) do
    kids = sanitize_block_children(node.children, policy)
    if kids == [], do: {:drop, nil}, else: {:keep, %{node | children: kids}}
  end

  # Unknown block node type: drop it for safety (the spec requires explicit handling)
  defp sanitize_block(_unknown, _policy), do: {:drop, nil}

  # ── Inline node handlers ─────────────────────────────────────────────────────

  # Text: always keep as-is
  defp sanitize_inline(%{type: :text} = node, _policy), do: {:keep_one, node}

  # HardBreak / SoftBreak: always keep as-is
  defp sanitize_inline(%{type: :hard_break} = node, _policy), do: {:keep_one, node}
  defp sanitize_inline(%{type: :soft_break} = node, _policy), do: {:keep_one, node}

  # CodeSpan: optionally convert to text
  defp sanitize_inline(%{type: :code_span, value: val}, %Policy{
         transform_code_span_to_text: true
       }) do
    {:keep_one, %{type: :text, value: val}}
  end

  defp sanitize_inline(%{type: :code_span} = node, _policy), do: {:keep_one, node}

  # Emphasis and Strong: recurse into children
  defp sanitize_inline(%{type: :emphasis} = node, policy) do
    kids = sanitize_inline_children(node.children, policy)
    if kids == [], do: {:drop, nil}, else: {:keep_one, %{node | children: kids}}
  end

  defp sanitize_inline(%{type: :strong} = node, policy) do
    kids = sanitize_inline_children(node.children, policy)
    if kids == [], do: {:drop, nil}, else: {:keep_one, %{node | children: kids}}
  end

  # Link: drop_links → promote children to parent
  defp sanitize_inline(%{type: :link} = node, %Policy{drop_links: true} = policy) do
    # Sanitize children, then promote them (return :keep_many so the caller
    # splices them into the parent's children list)
    kids = sanitize_inline_children(node.children, policy)
    {:keep_many, kids}
  end

  defp sanitize_inline(%{type: :link} = node, policy) do
    dest =
      if UrlUtils.scheme_allowed?(node.destination, policy.allowed_url_schemes) do
        node.destination
      else
        ""
      end

    kids = sanitize_inline_children(node.children, policy)

    if kids == [] do
      {:drop, nil}
    else
      {:keep_one, %{node | destination: dest, children: kids}}
    end
  end

  # Image: apply dropImages / transformImageToText / URL scheme checks
  defp sanitize_inline(%{type: :image}, %Policy{drop_images: true}),
    do: {:drop, nil}

  defp sanitize_inline(%{type: :image, alt: alt}, %Policy{transform_image_to_text: true}),
    do: {:keep_one, %{type: :text, value: alt}}

  defp sanitize_inline(%{type: :image} = node, policy) do
    dest =
      if UrlUtils.scheme_allowed?(node.destination, policy.allowed_url_schemes) do
        node.destination
      else
        ""
      end

    {:keep_one, %{node | destination: dest}}
  end

  # Autolink: drop if scheme not allowed
  defp sanitize_inline(%{type: :autolink} = node, policy) do
    if UrlUtils.scheme_allowed?(node.destination, policy.allowed_url_schemes) do
      {:keep_one, node}
    else
      {:drop, nil}
    end
  end

  # RawInline: check format policy
  defp sanitize_inline(%{type: :raw_inline}, %Policy{allow_raw_inline_formats: :drop_all}),
    do: {:drop, nil}

  defp sanitize_inline(%{type: :raw_inline} = node, %Policy{
         allow_raw_inline_formats: :passthrough
       }),
       do: {:keep_one, node}

  defp sanitize_inline(%{type: :raw_inline, format: fmt} = node, %Policy{
         allow_raw_inline_formats: formats
       })
       when is_list(formats) do
    if fmt in formats, do: {:keep_one, node}, else: {:drop, nil}
  end

  # Unknown inline node: drop for safety
  defp sanitize_inline(_unknown, _policy), do: {:drop, nil}

  # ── Helpers ──────────────────────────────────────────────────────────────────

  # Sanitize a list of block children, dropping those that return {:drop, nil}.
  defp sanitize_block_children(children, policy) do
    Enum.flat_map(children, fn child ->
      case sanitize_block(child, policy) do
        {:keep, node} -> [node]
        {:drop, nil} -> []
      end
    end)
  end

  # Sanitize a list of inline children.
  # Handles :keep_one, :keep_many (link promotion), and :drop.
  defp sanitize_inline_children(children, policy) do
    Enum.flat_map(children, fn child ->
      case sanitize_inline(child, policy) do
        {:keep_one, node} -> [node]
        {:keep_many, nodes} -> nodes
        {:drop, nil} -> []
      end
    end)
  end
end
