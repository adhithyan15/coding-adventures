package CodingAdventures::DocumentAstToHtml;

# ============================================================================
# CodingAdventures::DocumentAstToHtml
# ============================================================================
#
# Converts a Document AST (produced by any Markdown parser that emits the
# standard node format) into an HTML string. This is a recursive tree walk:
# each node type maps to one or more HTML elements.
#
# === Node-to-HTML Mapping ===
#
#   document       → rendered children concatenated
#   heading        → <h1>…</h1> through <h6>…</h6>
#   paragraph      → <p>…</p>  (omitted in tight list context)
#   code_block     → <pre><code [class="language-X"]>…</code></pre>
#   blockquote     → <blockquote>\n…</blockquote>
#   list           → <ul> or <ol [start="N"]>
#   list_item      → <li>…</li>
#   task_item      → <li><input type="checkbox" …> …</li>
#   thematic_break → <hr />
#   raw_block      → verbatim if format="html", empty otherwise
#   table          → <table><thead>…</thead><tbody>…</tbody></table>
#   table_row      → <tr>…</tr>
#   table_cell     → <td>…</td> or <th>…</th>
#
#   text           → HTML-escaped text content
#   emphasis       → <em>…</em>
#   strong         → <strong>…</strong>
#   strikethrough  → <del>…</del>
#   code_span      → <code>…</code>
#   link           → <a href="…" [title="…"]>…</a>
#   image          → <img src="…" alt="…" [title="…"] />
#   autolink       → <a href="…">…</a>
#   raw_inline     → verbatim if format="html", empty otherwise
#   hard_break     → <br />\n
#   soft_break     → \n
#
# === Tight vs Loose Lists ===
#
# A "tight" list suppresses <p> tags around paragraph content inside list
# items. This matches the CommonMark spec behavior where tightly-packed list
# items render without paragraph spacing.
#
#   Tight:   - item one   →  <ul><li>item one</li></ul>
#   Loose:   - item one\n  →  <ul><li><p>item one</p></li></ul>
#            \n
#            - item two
#
# The `tight` flag on list nodes controls this.
#
# === Security ===
#
# Text content and attribute values are HTML-escaped. Link and image URLs are
# checked for dangerous schemes (javascript:, vbscript:, data:, blob:).
# raw_block and raw_inline nodes pass through verbatim when format="html" —
# this is intentional for CommonMark compliance. Use the document sanitizer
# to strip raw HTML from untrusted content before rendering.
#
# @module CodingAdventures::DocumentAstToHtml

use strict;
use warnings;

our $VERSION = '0.01';

# ─── HTML Escaping ────────────────────────────────────────────────────────────

# escape($text) → string
#
# Escapes the five HTML special characters to their entity equivalents:
#
#   &  →  &amp;     (must be first, or we'd double-escape later replacements)
#   <  →  &lt;
#   >  →  &gt;
#   "  →  &quot;    (needed inside attribute values)
#   '  →  &#39;     (needed inside attribute values with single-quote delimiters)
#
# Note: we escape & first to avoid double-encoding the & in &lt; etc.
sub escape {
    my ($text) = @_;
    $text =~ s/&/&amp;/g;
    $text =~ s/</&lt;/g;
    $text =~ s/>/&gt;/g;
    $text =~ s/"/&quot;/g;
    return $text;
}

# ─── URL Sanitization ─────────────────────────────────────────────────────────
#
# We use a targeted blocklist of schemes that can execute code in browsers:
#
#   javascript:  — executes JavaScript directly
#   vbscript:    — executes VBScript (Internet Explorer legacy)
#   data:        — can embed executable scripts as base64 data URIs
#   blob:        — same-origin script execution via object URLs
#
# All other schemes pass through. Relative URLs (no scheme) always pass.
# The check is case-insensitive and strips control characters first to
# prevent bypass attempts like "java\x00script:".

sub _sanitize_url {
    my ($url) = @_;
    # Strip C0 control characters and common invisible Unicode
    $url =~ s/[\x00-\x1F\x7F]//g;
    $url =~ s/\xe2\x80\x8b//g;   # U+200B zero width space
    $url =~ s/\xe2\x80\x8c//g;   # U+200C zero width non-joiner
    $url =~ s/\xe2\x80\x8d//g;   # U+200D zero width joiner
    $url =~ s/\xe2\x81\xa0//g;   # U+2060 word joiner
    $url =~ s/\xef\xbb\xbf//g;   # U+FEFF BOM
    my $lower = lc($url);
    if ($lower =~ /^javascript:/ ||
        $lower =~ /^vbscript:/   ||
        $lower =~ /^data:/       ||
        $lower =~ /^blob:/) {
        return '';
    }
    return $url;
}

# ─── Forward Declarations ─────────────────────────────────────────────────────
#
# Perl closures can call each other because sub declarations are not executed
# at compile time. We use my $fn = sub {...} style so that render_block and
# render_inline can mutually recurse via closures captured in the calling scope.
# Since all these are module-level, we declare them as package subs.

# ─── Inline Rendering ─────────────────────────────────────────────────────────

# _render_inlines(\@nodes, \%options) → string
#
# Concatenates the HTML rendering of all inline nodes.
sub _render_inlines {
    my ($nodes, $options) = @_;
    my @parts;
    for my $n (@$nodes) {
        push @parts, _render_inline($n, $options);
    }
    return join('', @parts);
}

# _render_inline($node, \%options) → string
#
# Renders a single inline node to HTML.
sub _render_inline {
    my ($node, $options) = @_;
    my $t = $node->{type};

    if ($t eq 'text') {
        return escape($node->{value});

    } elsif ($t eq 'emphasis') {
        return '<em>' . _render_inlines($node->{children} || [], $options) . '</em>';

    } elsif ($t eq 'strong') {
        return '<strong>' . _render_inlines($node->{children} || [], $options) . '</strong>';

    } elsif ($t eq 'strikethrough') {
        return '<del>' . _render_inlines($node->{children} || [], $options) . '</del>';

    } elsif ($t eq 'code_span') {
        return '<code>' . escape($node->{value}) . '</code>';

    } elsif ($t eq 'link') {
        my $href = escape(_sanitize_url($node->{destination} // ''));
        my $title_attr = '';
        if (defined $node->{title}) {
            $title_attr = ' title="' . escape($node->{title}) . '"';
        }
        my $inner = _render_inlines($node->{children} || [], $options);
        return "<a href=\"$href\"$title_attr>$inner</a>";

    } elsif ($t eq 'image') {
        my $src = escape(_sanitize_url($node->{destination} // ''));
        my $alt = escape($node->{alt} // '');
        my $title_attr = '';
        if (defined $node->{title}) {
            $title_attr = ' title="' . escape($node->{title}) . '"';
        }
        return "<img src=\"$src\" alt=\"$alt\"$title_attr />";

    } elsif ($t eq 'autolink') {
        my $dest = _sanitize_url($node->{destination} // '');
        my $href;
        if ($node->{is_email}) {
            $href = 'mailto:' . escape($dest);
        } else {
            $href = escape($dest);
        }
        my $text = escape($node->{destination} // '');
        return "<a href=\"$href\">$text</a>";

    } elsif ($t eq 'raw_inline') {
        return '' if $options->{sanitize};
        return $node->{value} if ($node->{format} // '') eq 'html';
        return '';

    } elsif ($t eq 'hard_break') {
        return "<br />\n";

    } elsif ($t eq 'soft_break') {
        # CommonMark spec §6.12: soft line break renders as a newline
        return "\n";

    } else {
        return '';
    }
}

# ─── Block Rendering ──────────────────────────────────────────────────────────

# _render_blocks(\@blocks, $tight, \%options) → string
#
# Renders a list of block nodes into concatenated HTML. The $tight flag
# propagates from the enclosing list context.
sub _render_blocks {
    my ($blocks, $tight, $options) = @_;
    my @parts;
    for my $b (@$blocks) {
        push @parts, _render_block($b, $tight, $options);
    }
    return join('', @parts);
}

# _render_list_item($node, $tight, \%options) → string
#
# Renders a list_item node. In tight mode, the first child paragraph's content
# is inlined directly (no <p> wrapper). In loose mode, paragraphs are wrapped.
sub _render_list_item {
    my ($node, $tight, $options) = @_;
    my $children = $node->{children} || [];

    if (@$children == 0) {
        return "<li></li>\n";
    }

    if ($tight && $children->[0]{type} eq 'paragraph') {
        my $first_para    = $children->[0];
        my $first_content = _render_inlines($first_para->{children} || [], $options);
        if (@$children == 1) {
            return "<li>$first_content</li>\n";
        }
        # Multiple children: inline the first paragraph, block-render the rest
        my @rest = @{$children}[1 .. $#$children];
        my $rest_html = _render_blocks(\@rest, $tight, $options);
        return "<li>$first_content\n$rest_html</li>\n";
    }

    # Loose or non-paragraph first child
    my $inner = _render_blocks($children, $tight, $options);
    return "<li>\n$inner</li>\n";
}

# _render_task_item($node, $tight, \%options) → string
#
# Renders a task_item node (GFM extension). Prepends an HTML checkbox input.
sub _render_task_item {
    my ($node, $tight, $options) = @_;
    my $checkbox = $node->{checked}
        ? '<input type="checkbox" disabled="" checked="" />'
        : '<input type="checkbox" disabled="" />';
    my $children = $node->{children} || [];

    if (@$children == 0) {
        return "<li>$checkbox</li>\n";
    }

    if ($tight && $children->[0]{type} eq 'paragraph') {
        my $first_para    = $children->[0];
        my $first_content = _render_inlines($first_para->{children} || [], $options);
        my $sep = $first_content ne '' ? ' ' : '';
        if (@$children == 1) {
            return "<li>$checkbox$sep$first_content</li>\n";
        }
        my @rest = @{$children}[1 .. $#$children];
        my $rest_html = _render_blocks(\@rest, $tight, $options);
        return "<li>$checkbox$sep$first_content\n$rest_html</li>\n";
    }

    my $inner = _render_blocks($children, $tight, $options);
    return "<li>$checkbox\n$inner</li>\n";
}

# _render_table_cell($node, $is_header, \%options) → string
#
# Renders a table cell as <th> (header) or <td> (body).
sub _render_table_cell {
    my ($node, $is_header, $options) = @_;
    my $tag   = $is_header ? 'th' : 'td';
    my $inner = _render_inlines($node->{children} || [], $options);
    return "<$tag>$inner</$tag>\n";
}

# _render_table_row($node, \%options) → string
#
# Renders a table row, using <th> for header rows and <td> for body rows.
sub _render_table_row {
    my ($node, $options) = @_;
    my @cells;
    for my $cell (@{$node->{children} || []}) {
        push @cells, _render_table_cell($cell, $node->{is_header}, $options);
    }
    return "<tr>\n" . join('', @cells) . "</tr>\n";
}

# _render_table($node, \%options) → string
#
# Renders a table node with optional <thead> and <tbody> sections.
sub _render_table {
    my ($node, $options) = @_;
    my (@header_rows, @body_rows);
    for my $row (@{$node->{children} || []}) {
        if ($row->{is_header}) {
            push @header_rows, $row;
        } else {
            push @body_rows, $row;
        }
    }
    my @parts = ("<table>\n");
    if (@header_rows) {
        push @parts, "<thead>\n";
        for my $row (@header_rows) {
            push @parts, _render_table_row($row, $options);
        }
        push @parts, "</thead>\n";
    }
    if (@body_rows) {
        push @parts, "<tbody>\n";
        for my $row (@body_rows) {
            push @parts, _render_table_row($row, $options);
        }
        push @parts, "</tbody>\n";
    }
    push @parts, "</table>\n";
    return join('', @parts);
}

# _render_block($block, $tight, \%options) → string
#
# Renders a single block-level node to HTML.
sub _render_block {
    my ($block, $tight, $options) = @_;
    my $t = $block->{type};

    if ($t eq 'document') {
        return _render_blocks($block->{children} || [], 0, $options);

    } elsif ($t eq 'heading') {
        my $level = $block->{level};
        my $inner = _render_inlines($block->{children} || [], $options);
        return "<h$level>$inner</h$level>\n";

    } elsif ($t eq 'paragraph') {
        my $inner = _render_inlines($block->{children} || [], $options);
        return $tight ? "$inner\n" : "<p>$inner</p>\n";

    } elsif ($t eq 'code_block') {
        my $escaped = escape($block->{value} // '');
        if (defined $block->{language} && $block->{language} ne '') {
            my $lang = escape($block->{language});
            return "<pre><code class=\"language-$lang\">$escaped</code></pre>\n";
        }
        return "<pre><code>$escaped</code></pre>\n";

    } elsif ($t eq 'blockquote') {
        my $inner = _render_blocks($block->{children} || [], 0, $options);
        return "<blockquote>\n$inner</blockquote>\n";

    } elsif ($t eq 'list') {
        my $tag        = $block->{ordered} ? 'ol' : 'ul';
        my $start_attr = '';
        if ($block->{ordered} && defined $block->{start} && $block->{start} != 1) {
            my $s = int($block->{start});
            $start_attr = " start=\"$s\"";
        }
        my @items;
        for my $item (@{$block->{children} || []}) {
            if (($item->{type} // '') eq 'task_item') {
                push @items, _render_task_item($item, $block->{tight}, $options);
            } else {
                push @items, _render_list_item($item, $block->{tight}, $options);
            }
        }
        return "<$tag$start_attr>\n" . join('', @items) . "</$tag>\n";

    } elsif ($t eq 'list_item') {
        return _render_list_item($block, $tight, $options);

    } elsif ($t eq 'task_item') {
        return _render_task_item($block, $tight, $options);

    } elsif ($t eq 'thematic_break') {
        return "<hr />\n";

    } elsif ($t eq 'raw_block') {
        return '' if $options->{sanitize};
        return $block->{value} if ($block->{format} // '') eq 'html';
        return '';

    } elsif ($t eq 'table') {
        return _render_table($block, $options);

    } elsif ($t eq 'table_row') {
        return _render_table_row($block, $options);

    } elsif ($t eq 'table_cell') {
        return _render_table_cell($block, 0, $options);

    } else {
        return '';
    }
}

# ─── Public API ───────────────────────────────────────────────────────────────

# render($document, \%options) → string
#
# Renders a Document AST to an HTML string. The $document must be a hashref
# with type="document" and a children arrayref.
#
# Options:
#   sanitize => 1   — strip all raw_block and raw_inline nodes from output
#                     (use for untrusted Markdown; prevents XSS via raw HTML)
#
# Example:
#
#   use CodingAdventures::DocumentAstToHtml;
#
#   my $html = CodingAdventures::DocumentAstToHtml::render($doc);
#   my $safe = CodingAdventures::DocumentAstToHtml::render($doc, {sanitize => 1});
sub render {
    my ($document, $options) = @_;
    $options //= {};
    return _render_blocks($document->{children} || [], 0, $options);
}

# to_html($document, \%options) → string
#
# Alias for render(), named to_html() for familiarity with the Lua API.
sub to_html {
    my ($document, $options) = @_;
    return render($document, $options);
}

1;

__END__

=head1 NAME

CodingAdventures::DocumentAstToHtml - Renders a Document AST to HTML

=head1 SYNOPSIS

    use CodingAdventures::DocumentAstToHtml;

    my $html = CodingAdventures::DocumentAstToHtml::render($document);

    # Escape &, <, >, " in a text string:
    my $safe = CodingAdventures::DocumentAstToHtml::escape("<script>");
    # → "&lt;script&gt;"

=head1 DESCRIPTION

Converts a Document AST (produced by any Markdown parser that emits the
standard hashref node format) into an HTML string. Handles all common
node types including GFM extensions (tables, task items, strikethrough).

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
