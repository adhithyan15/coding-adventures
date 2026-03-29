package CodingAdventures::DocumentAst;

# ============================================================================
# CodingAdventures::DocumentAst — Format-Agnostic Document Intermediate Rep.
# ============================================================================
#
# The Document AST is the "LLVM IR of documents" — a stable, typed tree
# that every front-end parser produces and every back-end renderer consumes.
# With a shared IR, N front-ends × M back-ends requires only N + M
# implementations instead of N × M.
#
#   Markdown ─────────────────────────────────► HTML
#   reStructuredText ─────► Document AST ─────► PDF
#   HTML ──────────────────────────────────────► Plain text
#   DOCX ──────────────────────────────────────► DOCX
#
# === DESIGN PRINCIPLES ===
#
#   1. Semantic, not notational — nodes carry meaning, not syntax
#   2. Resolved, not deferred   — all link references resolved before IR
#   3. Format-agnostic          — raw_block/raw_inline carry a `format` tag
#   4. Immutable and typed      — hashrefs with a `type` field for dispatch
#   5. Minimal and stable       — only universal document concepts
#
# === NODE DISCRIMINATION ===
#
# Every node is a Perl hashref with a `type` field. Dispatch on node type
# using a chain of `if`/`elsif` comparisons, or use walk() for traversal:
#
#   my $type = $node->{type};
#   if    ($type eq 'heading')    { ... use $node->{level}, $node->{children} }
#   elsif ($type eq 'text')       { ... use $node->{value} }
#   elsif ($type eq 'code_block') { ... use $node->{language}, $node->{value} }
#
# === NODE TAXONOMY ===
#
# BLOCK nodes (form the document structure):
#   document      — root container
#   heading       — h1-h6, has level + children
#   paragraph     — block of prose, has children
#   code_block    — literal code, has language + value
#   blockquote    — quoted block, has children
#   list          — ordered or unordered, has ordered/start/tight/children
#   list_item     — one item in a list, has children
#   task_item     — checkbox list item, has checked + children
#   thematic_break — horizontal rule, leaf node
#   raw_block     — pass-through for specific back-end, has format + value
#   table         — data table, has align + children (rows)
#   table_row     — one row, has is_header + children (cells)
#   table_cell    — one cell, has children
#
# INLINE nodes (live inside block nodes with prose):
#   text          — plain text, has value
#   emphasis      — italic, has children
#   strong        — bold, has children
#   strikethrough — strikethrough text, has children
#   code_span     — inline code, has value
#   link          — hyperlink, has destination/title/children
#   image         — embedded image, has destination/title/alt
#   autolink      — URL/email link, has destination/is_email
#   raw_inline    — pass-through inline, has format + value
#   hard_break    — forced line break, leaf node
#   soft_break    — soft line break, leaf node
#
# === USAGE ===
#
#   use CodingAdventures::DocumentAst qw(make_document make_heading make_text walk);
#
#   my $doc = make_document([
#       make_heading(1, [make_text('Hello World')]),
#       make_paragraph([make_text('Some prose.')]),
#   ]);
#
#   walk($doc, sub {
#       my ($node) = @_;
#       print $node->{value} if $node->{type} eq 'text';
#   });
#
# ============================================================================

use strict;
use warnings;
use Exporter 'import';

our $VERSION = '0.01';

our @EXPORT_OK = qw(
    make_document
    make_heading
    make_paragraph
    make_code_block
    make_blockquote
    make_list
    make_list_item
    make_task_item
    make_thematic_break
    make_raw_block
    make_table
    make_table_row
    make_table_cell
    make_text
    make_emphasis
    make_strong
    make_strikethrough
    make_code_span
    make_link
    make_image
    make_autolink
    make_raw_inline
    make_hard_break
    make_soft_break
    node_type
    is_block
    is_inline
    walk
    BLOCK_TYPES
    INLINE_TYPES
);

# ============================================================================
# BLOCK NODE CONSTRUCTORS
# ============================================================================
#
# Block nodes form the structural skeleton of a document. They live at the
# top level of the document and can be nested (e.g. blockquotes, list items).

# make_document(children) — create the root document node.
#
# Every IR value is exactly one document node. An empty document has an
# empty children array. The document node cannot appear as a child.
#
#   document
#     ├── heading (level 1)
#     ├── paragraph
#     └── list (ordered, tight)
#
# @param children  arrayref of block nodes (default: [])
# @return hashref  { type => "document", children => [...] }
sub make_document {
    my ($children) = @_;
    return { type => 'document', children => $children || [] };
}

# make_heading(level, children) — create a heading node.
#
# Corresponds to <h1>–<h6> in HTML. Level 1 is the most prominent.
#
#   make_heading(2, [make_text('Introduction')])
#   → <h2>Introduction</h2>
#
# @param level     integer 1–6 (level beyond 6 is allowed but non-standard)
# @param children  arrayref of inline nodes
# @return hashref  { type => "heading", level => N, children => [...] }
sub make_heading {
    my ($level, $children) = @_;
    return { type => 'heading', level => $level, children => $children || [] };
}

# make_paragraph(children) — create a paragraph node.
#
# A block of prose containing inline nodes. Renders as <p> in HTML.
#
# @param children  arrayref of inline nodes
# @return hashref  { type => "paragraph", children => [...] }
sub make_paragraph {
    my ($children) = @_;
    return { type => 'paragraph', children => $children || [] };
}

# make_code_block(language, text) — create a code block node.
#
# A block of literal code or pre-formatted text. The `value` is raw —
# not decoded for HTML entities and not processed for inline markup.
#
#   make_code_block('perl', "print 'hello';\n")
#   → <pre><code class="language-perl">print 'hello';
#     </code></pre>
#
# @param language  string|undef — syntax hint, undef when unknown
# @param text      string       — raw source code (may include newlines)
# @return hashref  { type => "code_block", language => ..., value => ... }
sub make_code_block {
    my ($language, $text) = @_;
    return { type => 'code_block', language => $language, value => $text // '' };
}

# make_blockquote(children) — create a blockquote node.
#
# A block of content set apart as a quotation. Can contain any block nodes.
#
# @param children  arrayref of block nodes
# @return hashref  { type => "blockquote", children => [...] }
sub make_blockquote {
    my ($children) = @_;
    return { type => 'blockquote', children => $children || [] };
}

# make_list(ordered, start, tight, children) — create a list node.
#
# Ordered (numbered) or unordered (bulleted) list.
#
# `tight` is a rendering hint: a tight list was written without blank lines
# between items. In HTML, tight lists suppress <p> wrappers.
#
# `start` is the opening number for ordered lists (default 1). Nil for
# unordered lists.
#
#   make_list(0, undef, 1, [make_list_item([make_text('item1')])])
#   → <ul><li>item1</li></ul>
#
# @param ordered   bool  — 1 for ordered (numbered), 0 for unordered
# @param start     int|undef — opening number; undef for unordered
# @param tight     bool  — 1 if no blank lines between items
# @param children  arrayref of list_item nodes
# @return hashref  { type => "list", ordered => ..., ... }
sub make_list {
    my ($ordered, $start, $tight, $children) = @_;
    return {
        type     => 'list',
        ordered  => $ordered  ? 1 : 0,
        start    => $start,
        tight    => $tight    ? 1 : 0,
        children => $children || [],
    };
}

# make_list_item(children) — create a list item node.
#
# One item in a list. Contains block-level content.
#
# @param children  arrayref of block nodes
# @return hashref  { type => "list_item", children => [...] }
sub make_list_item {
    my ($children) = @_;
    return { type => 'list_item', children => $children || [] };
}

# make_task_item(checked, children) — create a task list item.
#
# A GitHub-Flavored Markdown checkbox item.
#
# @param checked   bool — whether the checkbox is checked
# @param children  arrayref of block nodes
# @return hashref  { type => "task_item", checked => ..., children => [...] }
sub make_task_item {
    my ($checked, $children) = @_;
    return { type => 'task_item', checked => $checked ? 1 : 0, children => $children || [] };
}

# make_thematic_break() — create a thematic break (horizontal rule).
#
# A visual separator between sections. Leaf node — no children.
# Renders as <hr /> in HTML, --- in Markdown, ---- in RST.
#
# @return hashref  { type => "thematic_break" }
sub make_thematic_break {
    return { type => 'thematic_break' };
}

# make_raw_block(format, value) — create a raw block node.
#
# A block of raw content passed verbatim to a specific back-end.
# The `format` field identifies the target renderer ("html", "latex", etc.).
# Back-ends that don't recognise `format` MUST skip this node silently.
#
#   Back-end contract:
#     format matches output → emit value verbatim (no escaping)
#     format doesn't match  → skip silently
#
# @param format  string — target back-end format tag
# @param value   string — raw content
# @return hashref  { type => "raw_block", format => ..., value => ... }
sub make_raw_block {
    my ($format, $value) = @_;
    return { type => 'raw_block', format => $format, value => $value // '' };
}

# make_table(align, children) — create a table node.
#
# @param align     arrayref of alignment strings (e.g. ['left','center','right'])
# @param children  arrayref of table_row nodes
# @return hashref  { type => "table", align => [...], children => [...] }
sub make_table {
    my ($align, $children) = @_;
    return { type => 'table', align => $align || [], children => $children || [] };
}

# make_table_row(is_header, children) — create a table row node.
#
# @param is_header  bool — true if this is a header row (<thead>)
# @param children   arrayref of table_cell nodes
# @return hashref   { type => "table_row", is_header => ..., children => [...] }
sub make_table_row {
    my ($is_header, $children) = @_;
    return { type => 'table_row', is_header => $is_header ? 1 : 0, children => $children || [] };
}

# make_table_cell(children) — create a table cell node.
#
# @param children  arrayref of inline nodes
# @return hashref  { type => "table_cell", children => [...] }
sub make_table_cell {
    my ($children) = @_;
    return { type => 'table_cell', children => $children || [] };
}

# ============================================================================
# INLINE NODE CONSTRUCTORS
# ============================================================================
#
# Inline nodes live inside block nodes that contain prose content: headings,
# paragraphs, and list items. They represent formatted text spans, links,
# images, and structural characters within a paragraph.

# make_text(value) — create a plain text node.
#
# Plain text with no markup. All HTML character references (e.g. &amp;,
# &#65;, &#x41;) should be decoded into their Unicode equivalents before
# being stored. The `value` field contains the final display-ready string.
#
#   "Hello &amp; world" → make_text("Hello & world")
#
# @param value  string — decoded Unicode string, ready for display
# @return hashref  { type => "text", value => ... }
sub make_text {
    my ($value) = @_;
    return { type => 'text', value => $value // '' };
}

# make_emphasis(children) — create an emphasis (italic) node.
#
# Stressed emphasis. Renders as <em> in HTML.
# In Markdown: *text* or _text_.
#
# @param children  arrayref of inline nodes
# @return hashref  { type => "emphasis", children => [...] }
sub make_emphasis {
    my ($children) = @_;
    return { type => 'emphasis', children => $children || [] };
}

# make_strong(children) — create a strong (bold) node.
#
# Strong importance. Renders as <strong> in HTML.
# In Markdown: **text** or __text__.
#
# @param children  arrayref of inline nodes
# @return hashref  { type => "strong", children => [...] }
sub make_strong {
    my ($children) = @_;
    return { type => 'strong', children => $children || [] };
}

# make_strikethrough(children) — create a strikethrough node.
#
# GFM extension. Renders as <del> in HTML.
# In Markdown: ~~text~~.
#
# @param children  arrayref of inline nodes
# @return hashref  { type => "strikethrough", children => [...] }
sub make_strikethrough {
    my ($children) = @_;
    return { type => 'strikethrough', children => $children || [] };
}

# make_code_span(value) — create an inline code node.
#
# Inline code. The value is raw — not decoded for HTML entities.
# Renders as <code> in HTML.
#
#   `const x = 1` → make_code_span("const x = 1")
#
# @param value  string — raw code content, not decoded
# @return hashref  { type => "code_span", value => ... }
sub make_code_span {
    my ($value) = @_;
    return { type => 'code_span', value => $value // '' };
}

# make_link(destination, title, children) — create a hyperlink node.
#
# A hyperlink with resolved destination. The `destination` is always fully
# resolved — all reference indirections resolved by the front-end.
#
#   make_link("https://example.com", "Example", [make_text("click here")])
#   → <a href="https://example.com" title="Example">click here</a>
#
# @param destination  string     — fully resolved URL
# @param title        string|undef — optional tooltip/hover text
# @param children     arrayref of inline nodes
# @return hashref     { type => "link", destination => ..., title => ..., children => [...] }
sub make_link {
    my ($destination, $title, $children) = @_;
    return {
        type        => 'link',
        destination => $destination // '',
        title       => $title,
        children    => $children || [],
    };
}

# make_image(destination, title, alt) — create an image node.
#
# An embedded image. Like link, `destination` is always fully resolved.
# `alt` is the plain-text fallback description (all inline markup stripped).
#
#   make_image("cat.png", undef, "a cute cat")
#   → <img src="cat.png" alt="a cute cat" />
#
# @param destination  string     — fully resolved image URL
# @param title        string|undef — optional tooltip/hover text
# @param alt          string     — plain-text alt description
# @return hashref     { type => "image", destination => ..., title => ..., alt => ... }
sub make_image {
    my ($destination, $title, $alt) = @_;
    return {
        type        => 'image',
        destination => $destination // '',
        title       => $title,
        alt         => $alt // '',
    };
}

# make_autolink(destination, is_email) — create an autolink node.
#
# A URL or email presented as a direct link without custom link text.
# Why preserve is_email?
#   1. HTML back-ends need to prepend mailto: for email autolinks.
#   2. Other back-ends may format emails differently from URLs.
#
#   make_autolink("user@example.com", 1)
#   → <a href="mailto:user@example.com">user@example.com</a>
#
# @param destination  string — URL or email address (without < >)
# @param is_email     bool   — 1 for email autolinks
# @return hashref     { type => "autolink", destination => ..., is_email => ... }
sub make_autolink {
    my ($destination, $is_email) = @_;
    return {
        type        => 'autolink',
        destination => $destination // '',
        is_email    => $is_email ? 1 : 0,
    };
}

# make_raw_inline(format, value) — create a raw inline node.
#
# An inline span of raw content passed verbatim to a specific back-end.
# Same back-end contract as raw_block.
#
# @param format  string — target back-end format tag
# @param value   string — raw content
# @return hashref  { type => "raw_inline", format => ..., value => ... }
sub make_raw_inline {
    my ($format, $value) = @_;
    return { type => 'raw_inline', format => $format, value => $value // '' };
}

# make_hard_break() — create a hard break node.
#
# A forced line break within a paragraph.
# In HTML: <br />. In LaTeX: \newline. In plain text: literal \n.
# In Markdown: two trailing spaces before a newline, or backslash + newline.
#
# @return hashref  { type => "hard_break" }
sub make_hard_break {
    return { type => 'hard_break' };
}

# make_soft_break() — create a soft break node.
#
# A soft line break — a newline within a paragraph that is not a hard break.
# In HTML: browsers collapse soft breaks to a single space.
# The IR preserves soft breaks so back-ends controlling line-wrapping can
# make the right choice.
#
# @return hashref  { type => "soft_break" }
sub make_soft_break {
    return { type => 'soft_break' };
}

# ============================================================================
# NODE TYPE PREDICATES
# ============================================================================
#
# Helper functions to test whether a node is a block or inline node type.
# These mirror the TypeScript union types `BlockNode` and `InlineNode`.

# BLOCK_TYPES — the set of block node type strings (as a hashref for O(1) lookup)
use constant BLOCK_TYPES => {
    document      => 1,
    heading       => 1,
    paragraph     => 1,
    code_block    => 1,
    blockquote    => 1,
    list          => 1,
    list_item     => 1,
    task_item     => 1,
    thematic_break => 1,
    raw_block     => 1,
    table         => 1,
    table_row     => 1,
    table_cell    => 1,
};

# INLINE_TYPES — the set of inline node type strings
use constant INLINE_TYPES => {
    text          => 1,
    emphasis      => 1,
    strong        => 1,
    strikethrough => 1,
    code_span     => 1,
    link          => 1,
    image         => 1,
    autolink      => 1,
    raw_inline    => 1,
    hard_break    => 1,
    soft_break    => 1,
};

# node_type(node) — return the type string of a node.
#
# @param node  hashref — any AST node
# @return string — the type field, or undef if node is undef/invalid
sub node_type {
    my ($node) = @_;
    return undef unless defined $node && ref($node) eq 'HASH';
    return $node->{type};
}

# is_block(node) — return 1 if the node is a block node, 0 otherwise.
#
# @param node  hashref — any AST node
# @return bool
sub is_block {
    my ($node) = @_;
    return 0 unless defined $node && ref($node) eq 'HASH';
    return BLOCK_TYPES->{ $node->{type} } ? 1 : 0;
}

# is_inline(node) — return 1 if the node is an inline node, 0 otherwise.
#
# @param node  hashref — any AST node
# @return bool
sub is_inline {
    my ($node) = @_;
    return 0 unless defined $node && ref($node) eq 'HASH';
    return INLINE_TYPES->{ $node->{type} } ? 1 : 0;
}

# ============================================================================
# TREE TRAVERSAL
# ============================================================================

# walk(node, visitor) — depth-first pre-order traversal of the AST.
#
# Calls visitor->($node) for each node in the tree, visiting parent before
# children (pre-order). This lets the visitor see every node, including
# leaf nodes (text, hard_break, etc.).
#
# Example — collect all text values:
#
#   my @texts;
#   walk($doc, sub {
#       my ($n) = @_;
#       push @texts, $n->{value} if $n->{type} eq 'text';
#   });
#
# @param node     hashref — the root node to traverse
# @param visitor  coderef — called with each node
sub walk {
    my ($node, $visitor) = @_;
    return unless defined $node && ref($node) eq 'HASH';

    # Visit this node first (pre-order)
    $visitor->($node);

    # Recurse into children
    my $children = $node->{children};
    if ( defined $children && ref($children) eq 'ARRAY' ) {
        for my $child ( @$children ) {
            walk($child, $visitor);
        }
    }
}

1;

__END__

=head1 NAME

CodingAdventures::DocumentAst - Format-agnostic document intermediate representation

=head1 SYNOPSIS

    use CodingAdventures::DocumentAst qw(make_document make_heading make_text walk);

    my $doc = make_document([
        make_heading(1, [make_text('Hello World')]),
        make_paragraph([make_text('Some prose.')]),
    ]);

    walk($doc, sub {
        my ($node) = @_;
        print "$node->{value}\n" if $node->{type} eq 'text';
    });

=head1 DESCRIPTION

The Document AST is a format-agnostic intermediate representation for
structured documents. Every parser produces this AST; every renderer
consumes it. This decoupling means N parsers × M renderers requires
only N + M implementations.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
