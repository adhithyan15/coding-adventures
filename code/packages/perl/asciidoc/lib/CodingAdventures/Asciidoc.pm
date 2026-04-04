package CodingAdventures::Asciidoc;

# ============================================================================
# CodingAdventures::Asciidoc — Self-contained AsciiDoc parser and HTML renderer
# ============================================================================
#
# AsciiDoc (https://asciidoc.org) is a lightweight markup language designed
# for technical writing.  This module implements a practical subset of the
# AsciiDoc specification, covering the constructs most commonly found in
# software documentation:
#
#   BLOCK ELEMENTS
#   ==============
#   Headings:           = H1   == H2   === H3  … ====== H6
#   Thematic breaks:    ''' (three or more single-quote characters)
#   Paragraphs:         consecutive non-blank lines
#   Code blocks:        ---- delimited
#   Literal blocks:     .... delimited   (same output as code block)
#   Passthrough blocks: ++++ delimited   (raw HTML; passed through verbatim)
#   Quote/sidebar:      ____ delimited   (renders as <blockquote>)
#   Unordered lists:    * item  ** nested item  (any depth)
#   Ordered lists:      . item  .. nested item  (any depth)
#   Source language:    [source,lang] before a code block
#   Comments:           // comment lines (silently skipped)
#
#   INLINE ELEMENTS
#   ===============
#   Strong (bold):      *text*   or  **text**  (unconstrained)
#   Emphasis (italic):  _text_   or  __text__  (unconstrained)
#   Inline code:        `code`   (verbatim — no nested parsing)
#   Link macro:         link:url[text]
#   Image macro:        image:url[alt]
#   Cross-reference:    <<anchor,text>>  or  <<anchor>>
#   Bare URL:           https://...  or  http://...
#
# KEY DIFFERENCE FROM MARKDOWN
# ============================
# In AsciiDoc, single `*asterisks*` produce STRONG (bold), not emphasis!
# Emphasis uses `_underscores_`.  This is the opposite of CommonMark.
#
# === ARCHITECTURE ===
#
# The pipeline has two stages:
#
#   AsciiDoc string
#       ↓
#   parse($text) → arrayref of block hashrefs  (AST)
#       ↓
#   to_html($text) → HTML string
#
# Block hashref schema:
#   { type => 'heading',    level => 1..6, children => [@inlines] }
#   { type => 'paragraph',  children => [@inlines] }
#   { type => 'code_block', language => $lang, value => $text }
#   { type => 'blockquote', children => [@blocks] }
#   { type => 'list',       ordered => 0|1, items => [[@inlines], ...] }
#   { type => 'thematic_break' }
#   { type => 'raw_block',  value => $html }
#
# Inline hashref schema:
#   { type => 'text',       value => $escaped_html }
#   { type => 'code_span',  value => $escaped_html }
#   { type => 'strong',     children => [@inlines] }
#   { type => 'emph',       children => [@inlines] }
#   { type => 'link',       href => $url, children => [@inlines] }
#   { type => 'image',      src => $url,  alt => $escaped_html }
#   { type => 'hard_break' }
#   { type => 'soft_break' }
#
# NOTE: This module is self-contained — it does NOT depend on other
# CodingAdventures packages.  The Perl packages in the monorepo are each
# standalone units that cannot import each other.
#
# === USAGE ===
#
#   use CodingAdventures::Asciidoc qw(to_html parse);
#
#   my $html = to_html("= Hello\n\nWorld\n");
#   # → "<h1>Hello</h1>\n<p>World</p>\n"
#
#   my $blocks = parse("= Title\n\nParagraph\n");
#   # → [{ type=>'heading', level=>1, children=>[...] }, ...]
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.1.0';

use Exporter 'import';
our @EXPORT_OK = qw(to_html parse);

# ============================================================================
# PUBLIC API
# ============================================================================

# to_html($text) → HTML string
#
# Parse an AsciiDoc string and render it to HTML in one step.
sub to_html {
    my ($text) = @_;
    my $blocks = parse($text);
    return _render_blocks($blocks);
}

# parse($text) → arrayref of block hashrefs
#
# Parse an AsciiDoc string into an array of block-level AST nodes.
# Each node is a hashref following the schema described above.
sub parse {
    my ($text) = @_;
    $text //= '';
    # Normalize line endings.
    $text =~ s/\r\n/\n/g;
    $text =~ s/\r/\n/g;
    # Ensure trailing newline.
    $text .= "\n" unless $text =~ /\n\z/;

    my @lines = split /\n/, $text, -1;
    return _parse_blocks(\@lines);
}

# ============================================================================
# BLOCK-LEVEL PARSER
# ============================================================================
#
# A line-oriented state machine.  We walk through @lines maintaining a
# $state variable that records which kind of block we are accumulating.
#
# States:
#   normal            — top-level dispatch
#   paragraph         — accumulating paragraph lines
#   code_block        — inside ---- fenced block
#   literal_block     — inside .... fenced block
#   passthrough_block — inside ++++ block (raw content)
#   quote_block       — inside ____ block (blockquote)
#   unordered_list    — accumulating * list items
#   ordered_list      — accumulating . list items

sub _parse_blocks {
    my ($lines) = @_;
    my @blocks;

    my $state            = 'normal';
    my @accumulated      = ();   # lines for the current block
    my $code_language    = '';
    my $pending_language = undef;
    my $list_ordered     = 0;
    my @list_items       = ();   # raw text of list items

    # ── Flush helpers ──────────────────────────────────────────────────────

    my $flush_paragraph = sub {
        if (@accumulated) {
            my $text = join("\n", @accumulated);
            push @blocks, { type => 'paragraph', children => _parse_inlines($text) };
            @accumulated = ();
        }
    };

    my $flush_list = sub {
        if (@list_items) {
            my @items = map { _parse_inlines($_) } @list_items;
            push @blocks, { type => 'list', ordered => $list_ordered, items => \@items };
            @list_items = ();
        }
    };

    # ── Line loop ──────────────────────────────────────────────────────────

    my $i = 0;
    my $n = scalar @$lines;

    while ($i < $n) {
        my $line = $lines->[$i];
        $i++;

        # ── code_block: collect until closing ---- ────────────────────────
        if ($state eq 'code_block') {
            if ($line =~ /\A-{4,}\s*\z/) {
                my $value = join("\n", @accumulated);
                $value .= "\n" if @accumulated;
                push @blocks, { type => 'code_block', language => $code_language, value => $value };
                @accumulated  = ();
                $code_language = '';
                $state         = 'normal';
            } else {
                push @accumulated, $line;
            }
            next;
        }

        # ── literal_block: collect until closing .... ─────────────────────
        if ($state eq 'literal_block') {
            if ($line =~ /\A\.{4,}\s*\z/) {
                my $value = join("\n", @accumulated);
                $value .= "\n" if @accumulated;
                push @blocks, { type => 'code_block', language => '', value => $value };
                @accumulated = ();
                $state       = 'normal';
            } else {
                push @accumulated, $line;
            }
            next;
        }

        # ── passthrough_block: collect until closing ++++ ─────────────────
        if ($state eq 'passthrough_block') {
            if ($line =~ /\A\+{4,}\s*\z/) {
                my $value = join("\n", @accumulated);
                $value .= "\n" if @accumulated;
                push @blocks, { type => 'raw_block', value => $value };
                @accumulated = ();
                $state       = 'normal';
            } else {
                push @accumulated, $line;
            }
            next;
        }

        # ── quote_block: collect until closing ____ ───────────────────────
        if ($state eq 'quote_block') {
            if ($line =~ /\A_{4,}\s*\z/) {
                my $inner_text = join("\n", @accumulated);
                my $inner_blocks = _parse_blocks([split /\n/, $inner_text, -1]);
                push @blocks, { type => 'blockquote', children => $inner_blocks };
                @accumulated = ();
                $state       = 'normal';
            } else {
                push @accumulated, $line;
            }
            next;
        }

        # ── paragraph: accumulate until blank or structural line ──────────
        if ($state eq 'paragraph') {
            if ($line =~ /\A\s*\z/) {
                $flush_paragraph->();
                $state = 'normal';
            } elsif (_is_structural($line)) {
                $flush_paragraph->();
                $state = 'normal';
                $i--;   # re-process in normal state
            } else {
                push @accumulated, $line;
            }
            next;
        }

        # ── unordered_list ────────────────────────────────────────────────
        if ($state eq 'unordered_list') {
            if ($line =~ /\A\s*\z/) {
                $flush_list->();
                $state = 'normal';
            } elsif ($line =~ /\A\*+\s+(.+)\z/) {
                push @list_items, $1;
            } else {
                $flush_list->();
                $state = 'normal';
                $i--;
            }
            next;
        }

        # ── ordered_list ──────────────────────────────────────────────────
        if ($state eq 'ordered_list') {
            if ($line =~ /\A\s*\z/) {
                $flush_list->();
                $state = 'normal';
            } elsif ($line =~ /\A\.+\s+(.+)\z/) {
                push @list_items, $1;
            } else {
                $flush_list->();
                $state = 'normal';
                $i--;
            }
            next;
        }

        # ── normal state: dispatch on line type ───────────────────────────

        # Blank line — nothing to do.
        if ($line =~ /\A\s*\z/) {
            next;
        }

        # Single-line comment.
        if ($line =~ /\A\/\//) {
            next;
        }

        # Attribute block: [source,lang] sets pending_language.
        if ($line =~ /\A\[source,([^\]]+)\]\s*\z/) {
            $pending_language = $1;
            next;
        }

        # Other attribute blocks — consume and ignore.
        if ($line =~ /\A\[[^\]]*\]\s*\z/) {
            next;
        }

        # Heading: = H1 through ====== H6.
        if ($line =~ /\A(={1,6})\s+(.+)\z/) {
            my $level    = length($1);
            my $children = _parse_inlines($2);
            push @blocks, { type => 'heading', level => $level, children => $children };
            next;
        }

        # Thematic break: ''' (three or more single quotes).
        if ($line =~ /\A'{3,}\s*\z/) {
            push @blocks, { type => 'thematic_break' };
            next;
        }

        # Code block fence: ---- (four or more dashes).
        if ($line =~ /\A-{4,}\s*\z/) {
            $code_language    = defined $pending_language ? $pending_language : '';
            $pending_language = undef;
            @accumulated      = ();
            $state            = 'code_block';
            next;
        }

        # Literal block fence: .... (four or more dots).
        if ($line =~ /\A\.{4,}\s*\z/) {
            $pending_language = undef;
            @accumulated      = ();
            $state            = 'literal_block';
            next;
        }

        # Passthrough block fence: ++++ (four or more plus signs).
        if ($line =~ /\A\+{4,}\s*\z/) {
            @accumulated = ();
            $state       = 'passthrough_block';
            next;
        }

        # Quote block fence: ____ (four or more underscores).
        if ($line =~ /\A_{4,}\s*\z/) {
            @accumulated = ();
            $state       = 'quote_block';
            next;
        }

        # Unordered list item: * item (one or more asterisks).
        if ($line =~ /\A\*+\s+(.+)\z/) {
            $list_ordered = 0;
            @list_items   = ($1);
            $state        = 'unordered_list';
            next;
        }

        # Ordered list item: . item (one or more dots).
        if ($line =~ /\A\.+\s+(.+)\z/) {
            $list_ordered = 1;
            @list_items   = ($1);
            $state        = 'ordered_list';
            next;
        }

        # Anything else starts a paragraph.
        $pending_language = undef;
        @accumulated      = ($line);
        $state            = 'paragraph';
    }

    # ── End-of-input: flush open state ────────────────────────────────────

    if ($state eq 'paragraph') {
        $flush_paragraph->();
    } elsif ($state eq 'code_block' || $state eq 'literal_block') {
        # Unclosed fence — emit what we have (tolerant parsing).
        my $value = join("\n", @accumulated);
        $value .= "\n" if @accumulated;
        push @blocks, { type => 'code_block', language => $code_language, value => $value };
    } elsif ($state eq 'passthrough_block') {
        my $value = join("\n", @accumulated);
        $value .= "\n" if @accumulated;
        push @blocks, { type => 'raw_block', value => $value };
    } elsif ($state eq 'quote_block') {
        my $inner_text   = join("\n", @accumulated);
        my $inner_blocks = _parse_blocks([split /\n/, $inner_text, -1]);
        push @blocks, { type => 'blockquote', children => $inner_blocks };
    } elsif ($state eq 'unordered_list' || $state eq 'ordered_list') {
        $flush_list->();
    }

    return \@blocks;
}

# True if the line would start a new structural element (interrupting a paragraph).
sub _is_structural {
    my ($line) = @_;
    return 1 if $line =~ /\A={1,6}\s+/;      # heading
    return 1 if $line =~ /\A-{4,}\s*\z/;     # code fence
    return 1 if $line =~ /\A\.{4,}\s*\z/;    # literal fence
    return 1 if $line =~ /\A\+{4,}\s*\z/;    # passthrough fence
    return 1 if $line =~ /\A_{4,}\s*\z/;     # quote fence
    return 1 if $line =~ /\A'{3,}\s*\z/;     # thematic break
    return 1 if $line =~ /\A\*+\s+/;         # unordered list
    return 1 if $line =~ /\A\.+\s+/;         # ordered list
    return 0;
}

# ============================================================================
# INLINE-LEVEL PARSER
# ============================================================================
#
# Scans inline AsciiDoc markup left-to-right, emitting inline node hashrefs.
#
# Priority order (checked top-to-bottom; first match wins):
#   1. hard break:           "  \n" or "+\n"
#   2. soft break:           "\n"
#   3. inline code:          `code`
#   4. unconstrained bold:   **text**   (check ** before *)
#   5. unconstrained italic: __text__   (check __ before _)
#   6. constrained bold:     *text*
#   7. constrained italic:   _text_
#   8. link macro:           link:url[text]
#   9. image macro:          image:url[alt]
#  10. cross-reference:      <<anchor,text>> or <<anchor>>
#  11. bare URL:             https://... or http://...
#  12. plain text / fallback

sub _parse_inlines {
    my ($text) = @_;
    my @nodes;
    my $pos = 0;
    my $len = length($text);

    while ($pos < $len) {
        my $rest = substr($text, $pos);

        # 1. Hard break: two+ spaces before newline, or AsciiDoc + before newline.
        if ($rest =~ /\A  +\n/) {
            push @nodes, { type => 'hard_break' };
            $pos += length($&);
            next;
        }
        if ($rest =~ /\A\+\n/) {
            push @nodes, { type => 'hard_break' };
            $pos += length($&);
            next;
        }

        # 2. Soft break: single newline.
        if ($rest =~ /\A\n/) {
            push @nodes, { type => 'soft_break' };
            $pos++;
            next;
        }

        # 3. Inline code: `code` (verbatim — no nested parsing).
        if ($rest =~ /\A`([^`]*)`/) {
            push @nodes, { type => 'code_span', value => _escape_html($1) };
            $pos += length($&);
            next;
        }

        # 4. Unconstrained bold: **text** (must check before constrained *text*).
        if ($rest =~ /\A\*\*(.*?)\*\*/s) {
            push @nodes, { type => 'strong', children => _parse_inlines($1) };
            $pos += length($&);
            next;
        }

        # 5. Unconstrained italic: __text__ (must check before constrained _text_).
        if ($rest =~ /\A__(.*?)__/s) {
            push @nodes, { type => 'emph', children => _parse_inlines($1) };
            $pos += length($&);
            next;
        }

        # 6. Constrained bold: *text*
        #    NOTE: In AsciiDoc, single asterisks make STRONG (bold), not emphasis!
        #    This is the critical difference from CommonMark Markdown.
        if ($rest =~ /\A\*([^*]+)\*/s) {
            push @nodes, { type => 'strong', children => _parse_inlines($1) };
            $pos += length($&);
            next;
        }

        # 7. Constrained italic: _text_
        if ($rest =~ /\A_([^_]+)_/s) {
            push @nodes, { type => 'emph', children => _parse_inlines($1) };
            $pos += length($&);
            next;
        }

        # 8. Link macro: link:url[text]
        if ($rest =~ /\Alink:([^\[]+)\[([^\]]*)\]/) {
            my ($url, $label) = ($1, $2);
            my $children = ($label ne '')
                ? _parse_inlines($label)
                : [{ type => 'text', value => _escape_html($url) }];
            push @nodes, { type => 'link', href => $url, children => $children };
            $pos += length($&);
            next;
        }

        # 9. Image macro: image:url[alt]
        if ($rest =~ /\Aimage:([^\[]+)\[([^\]]*)\]/) {
            my ($url, $alt) = ($1, $2);
            push @nodes, { type => 'image', src => $url, alt => _escape_html($alt) };
            $pos += length($&);
            next;
        }

        # 10. Cross-reference: <<anchor,text>> or <<anchor>>
        if ($rest =~ /\A<<([^,>]+),([^>]*)>>/) {
            my ($anchor, $label) = ($1, $2);
            my $children = ($label ne '')
                ? _parse_inlines($label)
                : [{ type => 'text', value => _escape_html($anchor) }];
            push @nodes, { type => 'link', href => "#$anchor", children => $children };
            $pos += length($&);
            next;
        }
        if ($rest =~ /\A<<([^>]+)>>/) {
            my $anchor = $1;
            push @nodes, {
                type     => 'link',
                href     => "#$anchor",
                children => [{ type => 'text', value => _escape_html($anchor) }],
            };
            $pos += length($&);
            next;
        }

        # 11. Bare https:// or http:// URL.
        if ($rest =~ /\A(https?:\/\/[^\s<>"'\]\)]+)/) {
            my $url = $1;
            push @nodes, {
                type     => 'link',
                href     => $url,
                children => [{ type => 'text', value => _escape_html($url) }],
            };
            $pos += length($url);
            next;
        }

        # 12. Plain text: consume up to the next special character.
        #     Special starters: ` * _ l i h < \n +
        if ($rest =~ /\A([^`*_lihH<\n+]+)/) {
            push @nodes, { type => 'text', value => _escape_html($1) };
            $pos += length($1);
            next;
        }

        # Fallback: consume one character as literal text.
        my $ch = substr($text, $pos, 1);
        push @nodes, { type => 'text', value => _escape_html($ch) };
        $pos++;
    }

    # Merge consecutive text nodes.
    #
    # The plain-text scanner stops at characters like `l`, `i`, `h`, `H`
    # because they could start inline macros (`link:`, `image:`, `https://`).
    # This causes words like "hello" to be fragmented into single-character
    # text nodes.  Merging consecutive text nodes restores full words so that
    # callers (and tests) see a single text node with the complete value.
    my @merged;
    for my $node (@nodes) {
        if ($node->{type} eq 'text' && @merged && $merged[-1]{type} eq 'text') {
            $merged[-1]{value} .= $node->{value};
        } else {
            push @merged, $node;
        }
    }

    return \@merged;
}

# ============================================================================
# HTML RENDERER
# ============================================================================

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

    # Heading: <h1>…</h1> through <h6>…</h6>
    if ($type eq 'heading') {
        my $h   = $block->{level};
        my $content = _render_inlines($block->{children});
        return "<h$h>$content</h$h>\n";
    }

    # Paragraph: <p>…</p>
    if ($type eq 'paragraph') {
        return "<p>" . _render_inlines($block->{children}) . "</p>\n";
    }

    # Code block: <pre><code class="language-X">…</code></pre>
    if ($type eq 'code_block') {
        my $lang  = $block->{language} // '';
        my $class = $lang ? qq( class="language-$lang") : '';
        my $code  = _escape_html($block->{value} // '');
        return "<pre><code$class>$code</code></pre>\n";
    }

    # Blockquote: <blockquote>…</blockquote>
    if ($type eq 'blockquote') {
        return "<blockquote>\n" . _render_blocks($block->{children}) . "</blockquote>\n";
    }

    # List: <ul> or <ol> with <li> items.
    if ($type eq 'list') {
        my $tag = $block->{ordered} ? 'ol' : 'ul';
        my $out = "<$tag>\n";
        for my $item (@{ $block->{items} }) {
            $out .= '<li>' . _render_inlines($item) . "</li>\n";
        }
        $out .= "</$tag>\n";
        return $out;
    }

    # Thematic break: <hr />
    if ($type eq 'thematic_break') {
        return "<hr />\n";
    }

    # Raw/passthrough block: emitted verbatim.
    if ($type eq 'raw_block') {
        return $block->{value} // '';
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

    # Text node: value is already HTML-escaped by the parser.
    return $node->{value}                                          if $type eq 'text';
    return '<code>' . $node->{value} . '</code>'                  if $type eq 'code_span';
    return '<strong>' . _render_inlines($node->{children}) . '</strong>' if $type eq 'strong';
    return '<em>' . _render_inlines($node->{children}) . '</em>'         if $type eq 'emph';
    return '<br />'                                                if $type eq 'hard_break';
    return "\n"                                                    if $type eq 'soft_break';

    if ($type eq 'link') {
        my $href = _escape_attr($node->{href} // '');
        return qq(<a href="$href">) . _render_inlines($node->{children}) . '</a>';
    }

    if ($type eq 'image') {
        my $src = _escape_attr($node->{src} // '');
        my $alt = $node->{alt} // '';
        return qq(<img src="$src" alt="$alt" />);
    }

    return '';
}

# ============================================================================
# HTML ESCAPING HELPERS
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

1;

__END__

=head1 NAME

CodingAdventures::Asciidoc - Self-contained AsciiDoc parser and HTML renderer

=head1 SYNOPSIS

    use CodingAdventures::Asciidoc qw(to_html parse);

    my $html = to_html("= Hello\n\nWorld\n");
    # → "<h1>Hello</h1>\n<p>World</p>\n"

    # Two-step
    my $blocks = parse("= Hello\n");
    # → [{ type=>'heading', level=>1, children=>[...] }]

=head1 DESCRIPTION

A self-contained AsciiDoc parser and HTML renderer.

AsciiDoc differs from CommonMark Markdown in several important ways:

=over 4

=item * C<*bold*> → B<strong> (in Markdown, C<*text*> makes I<emphasis>!)

=item * C<_italic_> → I<emphasis>

=item * Headings use C<=> sigils: C<= H1>, C<== H2>, …, C<====== H6>

=item * Code blocks delimited by C<----> (four dashes)

=item * Literal blocks delimited by C<....> (four dots)

=item * Quote/sidebar blocks delimited by C<____> (four underscores)

=item * Passthrough blocks delimited by C<++++> (four plus signs)

=item * Links: C<link:url[text]>

=item * Images: C<image:url[alt]>

=item * Cross-references: C<<<anchor,text>>>

=back

=head1 FUNCTIONS

=head2 to_html($text)

Parse an AsciiDoc string and return an HTML string.

=head2 parse($text)

Parse an AsciiDoc string and return an arrayref of block hashrefs (AST).

=head1 VERSION

0.1.0

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
