package CodingAdventures::Commonmark;

# ============================================================================
# CodingAdventures::Commonmark — Self-contained CommonMark Markdown renderer
# ============================================================================
#
# CommonMark (https://commonmark.org) is a well-specified Markdown variant.
# This module implements a subset of the CommonMark specification:
#
#   - ATX headings:     # H1  ## H2  ### H3  (up to ###### H6)
#   - Setext headings:  underlined with === or ---
#   - Paragraphs:       consecutive non-blank lines
#   - Hard line breaks: two trailing spaces + newline
#   - Blank lines:      separate block elements
#   - Unordered lists:  lines starting with - + *
#   - Ordered lists:    lines starting with  1. 2. 3.
#   - Blockquotes:      lines starting with >
#   - Fenced code:      ``` or ~~~ delimited blocks
#   - Indented code:    4-space / 1-tab indent
#   - Thematic breaks:  --- *** ___ (3+ dashes/asterisks/underscores)
#   - Inline code:      `code`
#   - Bold:             **text** or __text__
#   - Italic:           *text* or _text_
#   - Links:            [text](url)
#   - Images:           ![alt](url)
#   - HTML entities:    &amp; &lt; &gt; &quot;
#
# === ARCHITECTURE ===
#
# The pipeline has three stages:
#
#   Markdown string
#       ↓
#   parse($markdown) → AST  (tree of hashrefs)
#       ↓
#   to_html($ast) → HTML string
#       ↓
#   sanitize($html) → safe HTML (strips dangerous tags)
#
# The high-level render() function runs all three stages.
# render_safe() is the same but always sanitizes.
#
# NOTE: This module is self-contained — it does NOT depend on other
# CodingAdventures packages (commonmark_parser, document_ast_to_html, etc.)
# because in the monorepo build the Perl packages cannot import each other.
#
# === USAGE ===
#
#   use CodingAdventures::Commonmark qw(render parse to_html render_safe);
#
#   my $html = render("# Hello\n\nWorld\n");
#   # → "<h1>Hello</h1>\n<p>World</p>\n"
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(render parse to_html render_safe);

# ============================================================================
# HIGH-LEVEL API
# ============================================================================

# render($markdown) → HTML string (with raw HTML blocks allowed)
sub render {
    my ($markdown) = @_;
    my $ast  = parse($markdown);
    return to_html($ast);
}

# render_safe($markdown) → sanitized HTML (strips script, iframe, etc.)
sub render_safe {
    my ($markdown) = @_;
    my $html = render($markdown);
    return _sanitize_html($html);
}

# ============================================================================
# parse($markdown) → AST
# ============================================================================
#
# Parses a Markdown string into an AST.  The AST is a hashref:
#
#   {
#     type     => "document",
#     children => [ $block, $block, ... ],
#   }
#
# Each block node is one of:
#
#   { type => "heading",    level => 1..6, children => [@inlines] }
#   { type => "paragraph",  children => [@inlines] }
#   { type => "code_block", info => $lang, content => $text }
#   { type => "blockquote", children => [@blocks] }
#   { type => "list",       ordered => 0|1, items => [[@inlines], ...] }
#   { type => "thematic_break" }
#   { type => "html_block", content => $html }
#
# Inline nodes:
#
#   { type => "text",     value => "some text" }
#   { type => "code",     value => "inline code" }
#   { type => "strong",   children => [@inlines] }
#   { type => "emph",     children => [@inlines] }
#   { type => "link",     href => $url, children => [@inlines] }
#   { type => "image",    src  => $url, alt => $text }
#   { type => "softbreak" }
#   { type => "hardbreak" }

sub parse {
    my ($markdown) = @_;
    # Normalize line endings
    $markdown =~ s/\r\n/\n/g;
    $markdown =~ s/\r/\n/g;
    # Ensure trailing newline
    $markdown .= "\n" unless $markdown =~ /\n\z/;

    my @lines  = split /\n/, $markdown, -1;
    my $blocks = _parse_blocks(\@lines, 0);
    return { type => 'document', children => $blocks };
}

# ============================================================================
# BLOCK-LEVEL PARSER
# ============================================================================

sub _parse_blocks {
    my ($lines, $indent) = @_;
    my @blocks;
    my $i = 0;
    my $n = scalar @$lines;

    while ($i < $n) {
        my $line = $lines->[$i];

        # Strip the outer indent (for blockquotes etc.)
        if ($indent > 0) {
            $line = substr($line, $indent) if length($line) >= $indent;
        }

        # --- Blank line ---
        if ($line =~ /\A\s*\z/) { $i++; next; }

        # --- ATX heading: # Heading ---
        if ($line =~ /\A(#{1,6})\s+(.*?)(?:\s+#+\s*)?\z/) {
            my $level = length($1);
            my $text  = $2;
            push @blocks, {
                type     => 'heading',
                level    => $level,
                children => _parse_inlines($text),
            };
            $i++;
            next;
        }

        # --- Thematic break: --- or *** or ___ (3+ of same char, optional spaces) ---
        if ($line =~ /\A\s{0,3}(?:(?:-\s*){3,}|(?:\*\s*){3,}|(?:_\s*){3,})\s*\z/) {
            push @blocks, { type => 'thematic_break' };
            $i++;
            next;
        }

        # --- Fenced code block: ``` or ~~~ ---
        if ($line =~ /\A\s{0,3}(`{3,}|~{3,})(.*)\z/) {
            my $fence = $1;
            my $info  = $2;
            $info =~ s/^\s+|\s+$//g;  # trim
            my $fence_char = substr($fence, 0, 1);
            my $fence_len  = length($fence);
            $i++;
            my @code_lines;
            while ($i < $n) {
                my $cl = $lines->[$i];
                # Closing fence: same or more chars of same type, optional trailing spaces
                if ($cl =~ /\A\s{0,3}(\Q$fence_char\E{$fence_len,})\s*\z/) {
                    $i++;
                    last;
                }
                push @code_lines, $cl;
                $i++;
            }
            push @blocks, {
                type    => 'code_block',
                info    => $info,
                content => join("\n", @code_lines) . "\n",
            };
            next;
        }

        # --- Indented code block (4 spaces or 1 tab) ---
        if ($line =~ /\A(?:    |\t)(.*)/) {
            my @code_lines;
            while ($i < $n && ($lines->[$i] =~ /\A(?:    |\t)(.*)/ || $lines->[$i] =~ /\A\s*\z/)) {
                my $cl = $lines->[$i];
                if ($cl =~ /\A(?:    |\t)(.*)/) {
                    push @code_lines, $1;
                } else {
                    push @code_lines, '';
                }
                $i++;
            }
            # Remove trailing blank lines
            while (@code_lines && $code_lines[-1] =~ /\A\s*\z/) { pop @code_lines; }
            push @blocks, {
                type    => 'code_block',
                info    => '',
                content => join("\n", @code_lines) . "\n",
            };
            next;
        }

        # --- Blockquote: lines starting with > ---
        if ($line =~ /\A\s{0,3}>\s?(.*)/) {
            my @bq_lines;
            while ($i < $n) {
                my $bl = $lines->[$i];
                if ($bl =~ /\A\s{0,3}>\s?(.*)/) {
                    push @bq_lines, $1;
                    $i++;
                } elsif ($bl !~ /\A\s*\z/) {
                    # Lazy continuation
                    push @bq_lines, $bl;
                    $i++;
                } else {
                    last;
                }
            }
            push @blocks, {
                type     => 'blockquote',
                children => _parse_blocks(\@bq_lines, 0),
            };
            next;
        }

        # --- Unordered list: lines starting with - + * ---
        if ($line =~ /\A\s{0,3}[-+*]\s+(.*)/) {
            my @items;
            while ($i < $n) {
                my $ll = $lines->[$i];
                if ($ll =~ /\A\s{0,3}[-+*]\s+(.*)/) {
                    push @items, [ _parse_inlines($1) ];
                    $i++;
                    # Continuation lines (indented 2+ spaces)
                    while ($i < $n && $lines->[$i] =~ /\A  +(.*)/) {
                        # Append to the last item (not well-specified but common)
                        my $cont = $1;
                        push @{ $items[-1] }, _parse_inlines($cont);
                        $i++;
                    }
                } else {
                    last;
                }
            }
            push @blocks, { type => 'list', ordered => 0, items => \@items };
            next;
        }

        # --- Ordered list: lines starting with 1. 2. etc. ---
        if ($line =~ /\A\s{0,3}\d+\.\s+(.*)/) {
            my @items;
            while ($i < $n) {
                my $ll = $lines->[$i];
                if ($ll =~ /\A\s{0,3}\d+\.\s+(.*)/) {
                    push @items, [ _parse_inlines($1) ];
                    $i++;
                    while ($i < $n && $lines->[$i] =~ /\A  +(.*)/) {
                        my $cont = $1;
                        push @{ $items[-1] }, _parse_inlines($cont);
                        $i++;
                    }
                } else {
                    last;
                }
            }
            push @blocks, { type => 'list', ordered => 1, items => \@items };
            next;
        }

        # --- Paragraph (and setext headings) ---
        my @para_lines;
        while ($i < $n && $lines->[$i] !~ /\A\s*\z/) {
            push @para_lines, $lines->[$i];
            $i++;
            # Check for setext underline on next line
            if ($i < $n && $lines->[$i] =~ /\A(?:=+|-+)\s*\z/) {
                my $ul = $lines->[$i];
                $i++;
                my $level = ($ul =~ /\A=/) ? 1 : 2;
                my $text  = join(' ', @para_lines);
                push @blocks, {
                    type     => 'heading',
                    level    => $level,
                    children => _parse_inlines($text),
                };
                @para_lines = ();
                last;
            }
        }
        if (@para_lines) {
            my $text = join("\n", @para_lines);
            push @blocks, {
                type     => 'paragraph',
                children => _parse_inlines($text),
            };
        }
    }

    return \@blocks;
}

# ============================================================================
# INLINE-LEVEL PARSER
# ============================================================================
#
# Parses inline markup within a run of text:
#   `code`  **bold**  *italic*  [text](url)  ![alt](url)
#
# We use a simple left-to-right scan, looking for delimiter runs.

sub _parse_inlines {
    my ($text) = @_;
    my @nodes;
    my $pos = 0;
    my $len = length($text);

    while ($pos < $len) {
        my $remaining = substr($text, $pos);

        # Image: ![alt](url)
        if ($remaining =~ /\A!\[([^\]]*)\]\(([^)]*)\)/) {
            push @nodes, { type => 'image', alt => $1, src => $2 };
            $pos += length($&);
            next;
        }

        # Link: [text](url)
        if ($remaining =~ /\A\[([^\]]*)\]\(([^)]*)\)/) {
            push @nodes, {
                type     => 'link',
                href     => $2,
                children => _parse_inlines($1),
            };
            $pos += length($&);
            next;
        }

        # Inline code: `code` (may use multiple backticks)
        if ($remaining =~ /\A(`+)(.*?)\1/s) {
            my $code = $2;
            $code =~ s/^\s+|\s+$//g if $code =~ /\A\s.*\s\z/;  # strip single space wrapping
            push @nodes, { type => 'code', value => $code };
            $pos += length($&);
            next;
        }

        # Bold: **text** or __text__
        # Use possessive quantifier (++)  to prevent catastrophic backtracking (ReDoS)
        # when there is no closing delimiter: the engine will not re-try already-consumed
        # characters, so worst-case is O(n) regardless of input length.
        if ($remaining =~ /\A\*\*((?:[^*]|\*(?!\*))++)\*\*/s) {
            push @nodes, {
                type     => 'strong',
                children => _parse_inlines($1),
            };
            $pos += length($&);
            next;
        }
        if ($remaining =~ /\A__((?:[^_]|_(?!_))++)__/s) {
            push @nodes, {
                type     => 'strong',
                children => _parse_inlines($1),
            };
            $pos += length($&);
            next;
        }

        # Italic: *text* or _text_ (not part of word for _)
        if ($remaining =~ /\A\*((?:[^*])++)\*/s) {
            push @nodes, {
                type     => 'emph',
                children => _parse_inlines($1),
            };
            $pos += length($&);
            next;
        }
        if ($remaining =~ /\A_((?:[^_])++)_/s) {
            push @nodes, {
                type     => 'emph',
                children => _parse_inlines($1),
            };
            $pos += length($&);
            next;
        }

        # Hard line break: two or more spaces before newline
        if ($remaining =~ /\A  +\n/) {
            push @nodes, { type => 'hardbreak' };
            $pos += length($&);
            next;
        }

        # Soft line break: single newline
        if ($remaining =~ /\A\n/) {
            push @nodes, { type => 'softbreak' };
            $pos++;
            next;
        }

        # Plain text: consume up to the next special character
        if ($remaining =~ /\A([^`*_\[!\n\\]+)/) {
            push @nodes, { type => 'text', value => $1 };
            $pos += length($1);
            next;
        }

        # Escaped character: \* \_ etc.
        if ($remaining =~ /\A\\(.)/) {
            push @nodes, { type => 'text', value => $1 };
            $pos += 2;
            next;
        }

        # Any other single character (covers lone * _ [ ! \ etc.)
        push @nodes, { type => 'text', value => substr($text, $pos, 1) };
        $pos++;
    }

    return \@nodes;
}

# ============================================================================
# to_html($ast) → HTML string
# ============================================================================

sub to_html {
    my ($ast) = @_;
    return _render_blocks($ast->{children});
}

sub _render_blocks {
    my ($blocks) = @_;
    my $out = '';
    for my $block (@$blocks) {
        $out .= _render_block($block);
    }
    return $out;
}

sub _render_block {
    my ($block) = @_;
    my $type = $block->{type};

    if ($type eq 'heading') {
        my $h = $block->{level};
        return "<h$h>" . _render_inlines($block->{children}) . "</h$h>\n";
    }

    if ($type eq 'paragraph') {
        return "<p>" . _render_inlines($block->{children}) . "</p>\n";
    }

    if ($type eq 'code_block') {
        my $lang = $block->{info} ? qq( class="language-$block->{info}") : '';
        my $code = _escape_html($block->{content});
        return "<pre><code$lang>$code</code></pre>\n";
    }

    if ($type eq 'blockquote') {
        return "<blockquote>\n" . _render_blocks($block->{children}) . "</blockquote>\n";
    }

    if ($type eq 'list') {
        my $tag = $block->{ordered} ? 'ol' : 'ul';
        my $out = "<$tag>\n";
        for my $item (@{ $block->{items} }) {
            # Each item is an arrayref of inline arrays
            $out .= '<li>';
            if (ref $item->[0] eq 'ARRAY') {
                $out .= join(' ', map { _render_inlines($_) } @$item);
            } else {
                $out .= _render_inlines($item);
            }
            $out .= "</li>\n";
        }
        $out .= "</$tag>\n";
        return $out;
    }

    if ($type eq 'thematic_break') {
        return "<hr />\n";
    }

    if ($type eq 'html_block') {
        return $block->{content};
    }

    return '';
}

sub _render_inlines {
    my ($nodes) = @_;
    my $out = '';
    for my $node (@$nodes) {
        $out .= _render_inline($node);
    }
    return $out;
}

sub _render_inline {
    my ($node) = @_;
    my $type = $node->{type};

    return _escape_html($node->{value}) if $type eq 'text';
    return '<code>' . _escape_html($node->{value}) . '</code>' if $type eq 'code';
    return '<strong>' . _render_inlines($node->{children}) . '</strong>' if $type eq 'strong';
    return '<em>' . _render_inlines($node->{children}) . '</em>' if $type eq 'emph';
    if ($type eq 'link') {
        my $href = _escape_attr($node->{href});
        return qq(<a href="$href">) . _render_inlines($node->{children}) . '</a>';
    }
    if ($type eq 'image') {
        my $src = _escape_attr($node->{src});
        my $alt = _escape_html($node->{alt});
        return qq(<img src="$src" alt="$alt" />);
    }
    return '<br />' if $type eq 'hardbreak';
    return "\n"     if $type eq 'softbreak';
    return '';
}

# ============================================================================
# HTML escaping helpers
# ============================================================================

sub _escape_html {
    my ($s) = @_;
    $s //= '';
    $s =~ s/&/&amp;/g;
    $s =~ s/</&lt;/g;
    $s =~ s/>/&gt;/g;
    $s =~ s/"/&quot;/g;
    return $s;
}

sub _escape_attr {
    my ($s) = @_;
    $s //= '';
    $s =~ s/&/&amp;/g;
    $s =~ s/"/&quot;/g;
    return $s;
}

# ============================================================================
# HTML sanitization
# ============================================================================
#
# Strips potentially dangerous HTML constructs: <script>, <iframe>,
# javascript: URLs, and on* event attributes.
#
# This is a simple allow-list approach suitable for educational use.
# Production systems should use a dedicated sanitizer.

my @ALLOWED_TAGS = qw(
    h1 h2 h3 h4 h5 h6
    p br hr blockquote pre code
    strong em a img
    ul ol li
    table thead tbody tr th td
);

sub _sanitize_html {
    my ($html) = @_;

    # Remove script tags and their contents (handles nested/malformed via loop)
    1 while $html =~ s/<script\b[^>]*>.*?<\/script>//gis;

    # Remove iframe and object tags
    $html =~ s/<iframe\b[^>]*>.*?<\/iframe>//gis;
    $html =~ s/<object\b[^>]*>.*?<\/object>//gis;

    # Remove on* event attributes (leading-whitespace and start-of-attribute-string forms)
    # The (?<!\w) lookbehind prevents matching mid-word; covers space, tab, newline, and
    # positions directly after the tag name (no leading whitespace).
    $html =~ s/(?<!\w)on[a-z]\w*\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]*)//gi;

    # Remove dangerous URL schemes from href/src/action attributes
    # Covers: javascript:, vbscript:, data:, blob: — all can execute code in browsers
    my $DANGEROUS_SCHEME = qr/(?:javascript|vbscript|data|blob)\s*:/i;
    $html =~ s/(?:href|src|action)\s*=\s*"$DANGEROUS_SCHEME[^"]*"//gi;
    $html =~ s/(?:href|src|action)\s*=\s*'$DANGEROUS_SCHEME[^']*'//gi;
    $html =~ s/(?:href|src|action)\s*=\s*(?!["'])$DANGEROUS_SCHEME\S*//gi;

    return $html;
}

1;

__END__

=head1 NAME

CodingAdventures::Commonmark - Self-contained CommonMark Markdown renderer

=head1 SYNOPSIS

    use CodingAdventures::Commonmark qw(render parse to_html render_safe);

    my $html = render("# Hello\n\nWorld\n");
    # → "<h1>Hello</h1>\n<p>World</p>\n"

    # Two-step
    my $ast  = parse("# Hello\n");
    my $html = to_html($ast);

    # Safe rendering (strips dangerous HTML)
    my $safe = render_safe($user_input);

=head1 DESCRIPTION

A self-contained CommonMark subset Markdown parser and HTML renderer.
Handles headings, paragraphs, lists, blockquotes, code blocks, inline
formatting (bold, italic, code, links, images), and HTML sanitization.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
