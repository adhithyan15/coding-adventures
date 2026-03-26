"""Document AST sanitizer — policy-driven tree transformation.

The sanitizer performs a **single recursive descent** of the Document AST,
producing a new tree. The input is never mutated. Each node is handled by
an exhaustive match on its ``type`` field.

Design principles (from TE02):
  1. Pure / immutable  — always returns a fresh DocumentNode.
  2. Complete          — every node type handled explicitly (no silent pass-through).
  3. Composable        — run sanitize() multiple times with different policies.
  4. Drop empty parents — a ParagraphNode whose only child was dropped is itself
                          dropped (except DocumentNode, which is never dropped).

The central function is sanitize(). Everything else is a private helper that
handles one category of node.

Spec: TE02 — Document Sanitization, section "Transformation Rules"
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from coding_adventures_document_ast import (
    AutolinkNode,
    BlockquoteNode,
    CodeSpanNode,
    DocumentNode,
    EmphasisNode,
    HeadingNode,
    ImageNode,
    InlineNode,
    LinkNode,
    ListItemNode,
    ListNode,
    ParagraphNode,
    RawBlockNode,
    RawInlineNode,
    StrikethroughNode,
    StrongNode,
    TableCellNode,
    TableNode,
    TableRowNode,
    TaskItemNode,
    TextNode,
)

from coding_adventures_document_ast_sanitizer.policy import SanitizationPolicy
from coding_adventures_document_ast_sanitizer.url_utils import is_scheme_allowed

if TYPE_CHECKING:
    from coding_adventures_document_ast import BlockNode


# ─── Public API ───────────────────────────────────────────────────────────────


def sanitize(document: DocumentNode, policy: SanitizationPolicy) -> DocumentNode:
    """Sanitize *document* by applying *policy* to every node in the tree.

    Returns a new DocumentNode. The input is never mutated.

    The transformation follows the truth table in TE02:
      - Some nodes are kept as-is (leaf nodes that are always safe).
      - Some nodes are recursively processed (container nodes).
      - Some nodes are dropped (policy violations).
      - Some nodes are transformed (e.g. ImageNode → TextNode).
      - Empty containers after sanitization are themselves dropped.

    DocumentNode is the one exception to the "drop empty containers" rule:
    an empty document { type: "document", children: [] } is valid and returned
    as-is. It is never dropped.

    Args:
        document: The DocumentNode to sanitize. Not mutated.
        policy:   The SanitizationPolicy to apply.

    Returns:
        A new DocumentNode with all policy violations removed or neutralised.

    Example:
        # User-generated content — strict policy
        safe = sanitize(parse(user_markdown), STRICT)
        html = to_html(safe)

        # Documentation — pass through everything
        doc = sanitize(parse(trusted_markdown), PASSTHROUGH)
        html = to_html(doc)
    """
    # Sanitize all block children of the document root.
    # _sanitize_block_list returns only the blocks that survive sanitization.
    safe_children: list[BlockNode] = _sanitize_block_list(document["children"], policy)
    return DocumentNode(type="document", children=safe_children)


# ─── Block-level helpers ──────────────────────────────────────────────────────


def _sanitize_block_list(
    blocks: list,  # list[BlockNode]
    policy: SanitizationPolicy,
) -> list:  # list[BlockNode]
    """Sanitize a list of block nodes, returning only those that survive.

    Container nodes that become empty after sanitizing their children are
    dropped (see "Empty Children After Sanitization" in TE02).
    """
    result = []
    for block in blocks:
        sanitized = _sanitize_block(block, policy)
        if sanitized is not None:
            result.append(sanitized)
    return result


def _sanitize_block(
    node: dict,  # BlockNode
    policy: SanitizationPolicy,
) -> dict | None:  # BlockNode | None
    """Sanitize a single block node.

    Returns the sanitized node, or None if the node should be dropped.

    Truth table (from TE02):
      DocumentNode       → recurse into children  (handled separately in sanitize())
      HeadingNode        → drop / clamp / recurse depending on policy
      ParagraphNode      → recurse into children
      CodeBlockNode      → drop or keep as-is (leaf)
      BlockquoteNode     → drop or recurse
      ListNode           → recurse into children
      ListItemNode       → recurse into children
      ThematicBreakNode  → keep as-is (leaf)
      RawBlockNode       → drop or keep depending on format allowlist
    """
    node_type = node["type"]

    if node_type == "heading":
        return _sanitize_heading(node, policy)  # type: ignore[arg-type]

    if node_type == "paragraph":
        return _sanitize_paragraph(node, policy)  # type: ignore[arg-type]

    if node_type == "code_block":
        # Leaf node — no children to recurse into.
        # The policy may drop code blocks entirely.
        if policy.drop_code_blocks:
            return None
        return node  # keep as-is

    if node_type == "blockquote":
        return _sanitize_blockquote(node, policy)  # type: ignore[arg-type]

    if node_type == "list":
        return _sanitize_list(node, policy)  # type: ignore[arg-type]

    if node_type == "list_item":
        return _sanitize_list_item(node, policy)  # type: ignore[arg-type]

    if node_type == "task_item":
        return _sanitize_task_item(node, policy)  # type: ignore[arg-type]

    if node_type == "thematic_break":
        # Leaf node — always kept as-is.
        return node

    if node_type == "raw_block":
        return _sanitize_raw_block(node, policy)  # type: ignore[arg-type]

    if node_type == "table":
        return _sanitize_table(node, policy)  # type: ignore[arg-type]

    if node_type == "table_row":
        return _sanitize_table_row(node, policy)  # type: ignore[arg-type]

    if node_type == "table_cell":
        return _sanitize_table_cell(node, policy)  # type: ignore[arg-type]

    # Safety net: unknown node types are dropped rather than silently passed
    # through. This prevents future node types from bypassing sanitization.
    # When new node types are added to the Document AST, this function must
    # be updated to handle them.
    raise ValueError(  # pragma: no cover
        f"Unknown block node type '{node_type}'. "
        "Update sanitizer.py to handle this new node type."
    )


def _sanitize_heading(
    node: HeadingNode,
    policy: SanitizationPolicy,
) -> HeadingNode | None:
    """Sanitize a HeadingNode according to the heading level policy.

    Truth table:
      max_heading_level == "drop" → drop node entirely
      level < min_heading_level  → clamp level UP to min_heading_level
      level > max_heading_level  → clamp level DOWN to max_heading_level
      otherwise                  → recurse into children

    Note: "clamping up" means if the author wrote h1 and min_heading_level=2,
    the heading becomes h2. This is intentional: it reserves h1 for the
    page title (the calling application controls the h1).

    Note: "clamping down" means if the author wrote h5 and max_heading_level=3,
    the heading becomes h3. All deeply nested headings merge to the maximum.
    """
    # Drop all headings if policy says so.
    if policy.max_heading_level == "drop":
        return None

    level = node["level"]

    # Clamp heading level to the allowed range.
    # We clamp max first, then min (order doesn't matter for valid inputs
    # since min <= max, but this is the clearest order to reason about).
    max_level = policy.max_heading_level
    assert isinstance(max_level, int)  # narrowed: not "drop"
    min_level = policy.min_heading_level

    new_level = level
    if new_level < min_level:
        new_level = min_level
    if new_level > max_level:
        new_level = max_level

    # Validate: Python's Literal[1,2,3,4,5,6] doesn't enforce at runtime,
    # so we clamp to the valid range as defence-in-depth.
    new_level = max(1, min(6, new_level))

    # Sanitize children. If all children are dropped, drop the heading too.
    safe_children = _sanitize_inline_list(node["children"], policy)
    if not safe_children:
        return None

    return HeadingNode(type="heading", level=new_level, children=safe_children)  # type: ignore[typeddict-item]


def _sanitize_paragraph(
    node: ParagraphNode,
    policy: SanitizationPolicy,
) -> ParagraphNode | None:
    """Sanitize a ParagraphNode by recursing into its inline children.

    If all inline children are dropped (e.g. the paragraph contained only
    a RawInlineNode that was dropped), the paragraph itself is dropped.
    This prevents empty <p></p> tags in rendered HTML.
    """
    safe_children = _sanitize_inline_list(node["children"], policy)
    if not safe_children:
        return None
    return ParagraphNode(type="paragraph", children=safe_children)


def _sanitize_blockquote(
    node: BlockquoteNode,
    policy: SanitizationPolicy,
) -> BlockquoteNode | None:
    """Sanitize a BlockquoteNode.

    If drop_blockquotes is True, the entire subtree is dropped (unlike
    drop_links where children are promoted — blockquotes are structural
    containers and promoting their content into the parent would garble
    the document structure).

    Otherwise, recurse into children. Empty blockquote → dropped.
    """
    if policy.drop_blockquotes:
        return None
    safe_children = _sanitize_block_list(node["children"], policy)
    if not safe_children:
        return None
    return BlockquoteNode(type="blockquote", children=safe_children)


def _sanitize_list(
    node: ListNode,
    policy: SanitizationPolicy,
) -> ListNode | None:
    """Sanitize a ListNode by recursing into its ListItemNode children.

    Lists are always kept (no policy to drop lists). An empty list is dropped.
    """
    safe_items = []
    for item in node["children"]:
        if item["type"] == "task_item":
            sanitized_item = _sanitize_task_item(item, policy)
        else:
            sanitized_item = _sanitize_list_item(item, policy)
        if sanitized_item is not None:
            safe_items.append(sanitized_item)
    if not safe_items:
        return None
    return ListNode(
        type="list",
        ordered=node["ordered"],
        start=node["start"],
        tight=node["tight"],
        children=safe_items,
    )


def _sanitize_list_item(
    node: ListItemNode,
    policy: SanitizationPolicy,
) -> ListItemNode | None:
    """Sanitize a ListItemNode by recursing into its block children.

    An empty list item (all children dropped) is dropped.
    """
    safe_children = _sanitize_block_list(node["children"], policy)
    if not safe_children:
        return None
    return ListItemNode(type="list_item", children=safe_children)


def _sanitize_task_item(
    node: TaskItemNode,
    policy: SanitizationPolicy,
) -> TaskItemNode | None:
    safe_children = _sanitize_block_list(node["children"], policy)
    if not safe_children:
        return None
    return TaskItemNode(type="task_item", checked=node["checked"], children=safe_children)


def _sanitize_raw_block(
    node: RawBlockNode,
    policy: SanitizationPolicy,
) -> RawBlockNode | None:
    """Sanitize a RawBlockNode based on the format allowlist.

    Truth table:
      allow_raw_block_formats == "drop-all"    → drop node
      allow_raw_block_formats == "passthrough" → keep node as-is
      allow_raw_block_formats == tuple         → keep if format in tuple, else drop
    """
    fmt_policy = policy.allow_raw_block_formats
    if fmt_policy == "drop-all":
        return None
    if fmt_policy == "passthrough":
        return node
    # Tuple of allowed format strings.
    assert isinstance(fmt_policy, tuple)
    if node["format"] in fmt_policy:
        return node
    return None


def _sanitize_table(
    node: TableNode,
    policy: SanitizationPolicy,
) -> TableNode | None:
    safe_rows = []
    for row in node["children"]:
        sanitized_row = _sanitize_table_row(row, policy)
        if sanitized_row is not None:
            safe_rows.append(sanitized_row)
    if not safe_rows:
        return None
    return TableNode(type="table", align=list(node["align"]), children=safe_rows)


def _sanitize_table_row(
    node: TableRowNode,
    policy: SanitizationPolicy,
) -> TableRowNode | None:
    safe_cells = []
    for cell in node["children"]:
        sanitized_cell = _sanitize_table_cell(cell, policy)
        if sanitized_cell is not None:
            safe_cells.append(sanitized_cell)
    if not safe_cells:
        return None
    return TableRowNode(type="table_row", isHeader=node["isHeader"], children=safe_cells)


def _sanitize_table_cell(
    node: TableCellNode,
    policy: SanitizationPolicy,
) -> TableCellNode | None:
    safe_children = _sanitize_inline_list(node["children"], policy)
    if not safe_children:
        return None
    return TableCellNode(type="table_cell", children=safe_children)


# ─── Inline-level helpers ─────────────────────────────────────────────────────


def _sanitize_inline_list(
    inlines: list,  # list[InlineNode]
    policy: SanitizationPolicy,
) -> list:  # list[InlineNode]
    """Sanitize a list of inline nodes, returning only those that survive.

    Note: LinkNode with drop_links=True returns a *list* of promoted children,
    not a single node. This is why we use extend() for the link case.
    """
    result: list[InlineNode] = []
    for inline in inlines:
        inline_type = inline["type"]

        if inline_type == "link":
            # _sanitize_link may return None (link dropped entirely, which
            # shouldn't happen with drop_links — children are promoted instead)
            # or a list of nodes (when drop_links=True: promoted children)
            # or a single LinkNode (when URL is sanitized but link kept).
            link_result = _sanitize_link(inline, policy)
            if link_result is None:
                pass  # dropped
            elif isinstance(link_result, list):
                result.extend(link_result)  # promoted children
            else:
                result.append(link_result)  # kept as LinkNode
        else:
            sanitized = _sanitize_inline(inline, policy)
            if sanitized is not None:
                result.append(sanitized)
    return result


def _sanitize_inline(
    node: dict,  # InlineNode (excluding LinkNode which is handled separately)
    policy: SanitizationPolicy,
) -> InlineNode | None:
    """Sanitize a single inline node (not LinkNode — that has special handling).

    Truth table (from TE02):
      TextNode          → keep as-is
      EmphasisNode      → recurse into children
      StrongNode        → recurse into children
      CodeSpanNode      → convert to TextNode or keep as-is
      ImageNode         → drop / convert to TextNode / sanitize URL
      AutolinkNode      → drop if URL scheme not allowed, else keep
      RawInlineNode     → keep/drop based on format allowlist
      HardBreakNode     → keep as-is
      SoftBreakNode     → keep as-is
    """
    node_type = node["type"]

    if node_type == "text":
        return node  # TextNode: always safe

    if node_type == "emphasis":
        return _sanitize_emphasis(node, policy)  # type: ignore[arg-type]

    if node_type == "strong":
        return _sanitize_strong(node, policy)  # type: ignore[arg-type]

    if node_type == "strikethrough":
        return _sanitize_strikethrough(node, policy)  # type: ignore[arg-type]

    if node_type == "code_span":
        return _sanitize_code_span(node, policy)  # type: ignore[arg-type]

    if node_type == "image":
        return _sanitize_image(node, policy)  # type: ignore[arg-type]

    if node_type == "autolink":
        return _sanitize_autolink(node, policy)  # type: ignore[arg-type]

    if node_type == "raw_inline":
        return _sanitize_raw_inline(node, policy)  # type: ignore[arg-type]

    if node_type == "hard_break":
        return node  # leaf node, always kept

    if node_type == "soft_break":
        return node  # leaf node, always kept

    # Unknown inline node type — safety net (same as block-level).
    raise ValueError(  # pragma: no cover
        f"Unknown inline node type '{node_type}'. "
        "Update sanitizer.py to handle this new node type."
    )


def _sanitize_emphasis(
    node: EmphasisNode,
    policy: SanitizationPolicy,
) -> EmphasisNode | None:
    """Recurse into EmphasisNode children. Empty emphasis → dropped."""
    safe_children = _sanitize_inline_list(node["children"], policy)
    if not safe_children:
        return None
    return EmphasisNode(type="emphasis", children=safe_children)


def _sanitize_strong(
    node: StrongNode,
    policy: SanitizationPolicy,
) -> StrongNode | None:
    """Recurse into StrongNode children. Empty strong → dropped."""
    safe_children = _sanitize_inline_list(node["children"], policy)
    if not safe_children:
        return None
    return StrongNode(type="strong", children=safe_children)


def _sanitize_strikethrough(
    node: StrikethroughNode,
    policy: SanitizationPolicy,
) -> StrikethroughNode | None:
    safe_children = _sanitize_inline_list(node["children"], policy)
    if not safe_children:
        return None
    return StrikethroughNode(type="strikethrough", children=safe_children)


def _sanitize_code_span(
    node: CodeSpanNode,
    policy: SanitizationPolicy,
) -> CodeSpanNode | TextNode:
    """Sanitize a CodeSpanNode.

    If transform_code_span_to_text is True, return a TextNode with the same
    value. The <code> wrapper is removed, but the text content is preserved.
    """
    if policy.transform_code_span_to_text:
        return TextNode(type="text", value=node["value"])
    return node  # keep as-is


def _sanitize_link(
    node: LinkNode,
    policy: SanitizationPolicy,
) -> LinkNode | list[InlineNode] | None:
    """Sanitize a LinkNode.

    Returns one of three things:
      list[InlineNode]  → when drop_links=True: children promoted to parent
      LinkNode          → keep (destination sanitized if needed)
      None              → (never happens currently, but included for symmetry)

    Promotion: when drop_links=True, we unwrap the link and return the
    sanitized children as a flat list. The calling code (in _sanitize_inline_list)
    extends the parent list with these children.

    URL sanitization: if the link's destination has a disallowed scheme,
    set destination="" (not drop the link — the text content is still useful).

    Example:
        # drop_links=True
        LinkNode { destination: "https://evil.com", children: [TextNode("click")] }
        → [TextNode("click")]   (link gone, text preserved)

        # URL sanitization
        LinkNode { destination: "javascript:alert(1)", children: [...] }
        → LinkNode { destination: "", ... }  (with STRICT policy)
    """
    if policy.drop_links:
        # Promote children to parent — recurse into them first.
        return _sanitize_inline_list(node["children"], policy)

    # Sanitize the destination URL.
    dest = node["destination"]
    safe_dest = dest if is_scheme_allowed(dest, policy.allowed_url_schemes) else ""

    # Recurse into children (link text).
    safe_children = _sanitize_inline_list(node["children"], policy)

    # Build new LinkNode with sanitized destination and children.
    return LinkNode(
        type="link",
        destination=safe_dest,
        title=node["title"],
        children=safe_children,
    )


def _sanitize_image(
    node: ImageNode,
    policy: SanitizationPolicy,
) -> ImageNode | TextNode | None:
    """Sanitize an ImageNode.

    Truth table:
      drop_images=True            → None (drop entirely)
      transform_image_to_text=True → TextNode { value: node.alt }
      URL scheme not allowed      → ImageNode { destination: "" }
      otherwise                   → ImageNode unchanged

    Precedence: drop_images takes priority over transform_image_to_text.
    If both are True, the image is dropped.
    """
    if policy.drop_images:
        return None

    if policy.transform_image_to_text:
        # Replace the image with its alt text.
        # If alt is empty, this produces an empty TextNode — which is
        # technically valid (empty TextNode renders as nothing).
        return TextNode(type="text", value=node["alt"])

    # Sanitize the destination URL.
    dest = node["destination"]
    safe_dest = dest if is_scheme_allowed(dest, policy.allowed_url_schemes) else ""

    return ImageNode(
        type="image",
        destination=safe_dest,
        title=node["title"],
        alt=node["alt"],
    )


def _sanitize_autolink(
    node: AutolinkNode,
    policy: SanitizationPolicy,
) -> AutolinkNode | None:
    """Sanitize an AutolinkNode.

    Unlike LinkNode (where a disallowed URL gets destination=""), an AutolinkNode
    with a disallowed scheme is **dropped entirely**. The reason: an AutolinkNode's
    link text IS its URL, so setting destination="" would produce a link like
    <a href="">javascript:alert(1)</a> — the dangerous-looking text would remain.
    Dropping the node is safer.

    For email autolinks (is_email=True), the destination is the email address
    without the mailto: prefix. The HTML renderer prepends mailto:. Email addresses
    are always considered safe here — the scheme check is on the final URL.
    """
    dest = node["destination"]

    # For email autolinks, the effective URL is "mailto:{dest}".
    # The scheme to check is "mailto".
    effective_url = f"mailto:{dest}" if node["is_email"] else dest

    if not is_scheme_allowed(effective_url, policy.allowed_url_schemes):
        return None  # drop entirely

    return node  # keep as-is (destination unchanged)


def _sanitize_raw_inline(
    node: RawInlineNode,
    policy: SanitizationPolicy,
) -> RawInlineNode | None:
    """Sanitize a RawInlineNode based on the inline format allowlist.

    Truth table:
      allow_raw_inline_formats == "drop-all"    → drop
      allow_raw_inline_formats == "passthrough" → keep as-is
      allow_raw_inline_formats == tuple         → keep if format in tuple, else drop
    """
    fmt_policy = policy.allow_raw_inline_formats
    if fmt_policy == "drop-all":
        return None
    if fmt_policy == "passthrough":
        return node
    assert isinstance(fmt_policy, tuple)
    if node["format"] in fmt_policy:
        return node
    return None
