package CodingAdventures::Gfm;

# ============================================================================
# CodingAdventures::Gfm — GitHub Flavored Markdown renderer
# ============================================================================
#
# GitHub Flavored Markdown (GFM) extends CommonMark with several additions:
#
#   1. **Tables** — pipe-separated column layout:
#
#         | Name  | Age |
#         |-------|-----|
#         | Alice | 30  |
#         | Bob   | 25  |
#
#   2. **Task lists** — checkboxes in list items:
#
#         - [x] Done task
#         - [ ] Pending task
#
#   3. **Strikethrough** — ~~struck-through text~~
#
#   4. **Autolinks** — bare URLs become clickable links
#
#   5. **Fenced code blocks** — same as CommonMark but also accepts ```lang
#
# === IMPLEMENTATION STRATEGY ===
#
# We build on the CommonMark logic (copied inline, since Perl packages
# can't import each other in this monorepo build system) and add GFM
# extensions on top.
#
# The GFM parser is a superset of the CommonMark parser:
#
#   - Block parser adds: tables, task-list detection
#   - Inline parser adds: strikethrough, autolinks
#
# === USAGE ===
#
#   use CodingAdventures::Gfm qw(render parse to_html);
#
#   my $html = render("| A | B |\n|---|---|\n| 1 | 2 |\n");
#   # → "<table>...</table>\n"
#
#   my $html = render("- [x] Done\n- [ ] Todo\n");
#   # → "<ul><li><input type=\"checkbox\" checked> Done</li>...</ul>\n"
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

sub render {
    my ($markdown) = @_;
    my $ast = parse($markdown);
    return to_html($ast);
}

sub render_safe {
    my ($markdown) = @_;
    my $html = render($markdown);
    return _sanitize_html($html);
}

# ============================================================================
# parse($markdown) → AST
# ============================================================================

sub parse {
    my ($markdown) = @_;
    $markdown =~ s/\r\n/\n/g;
    $markdown =~ s/\r/\n/g;
    $markdown .= "\n" unless $markdown =~ /\n\z/;

    my @lines  = split /\n/, $markdown, -1;
    my $blocks = _parse_blocks(\@lines);
    return { type => 'document', children => $blocks };
}

# ============================================================================
# BLOCK-LEVEL PARSER (GFM superset of CommonMark)
# ============================================================================

sub _parse_blocks {
    my ($lines) = @_;
    my @blocks;
    my $i = 0;
    my $n = scalar @$lines;

    while ($i < $n) {
        my $line = $lines->[$i];

        # --- Blank line ---
        if ($line =~ /\A\s*\z/) { $i++; next; }

        # --- ATX heading ---
        if ($line =~ /\A(#{1,6})\s+(.*?)(?:\s+#+\s*)?\z/) {
            push @blocks, {
                type     => 'heading',
                level    => length($1),
                children => _parse_inlines($2),
            };
            $i++;
            next;
        }

        # --- Thematic break ---
        if ($line =~ /\A\s{0,3}(?:(?:-\s*){3,}|(?:\*\s*){3,}|(?:_\s*){3,})\s*\z/) {
            push @blocks, { type => 'thematic_break' };
            $i++;
            next;
        }

        # --- GFM: Table ---
        # A table has a header row, a separator row (|---|---|), and data rows.
        if ($i + 1 < $n && _is_table_separator($lines->[$i + 1])) {
            my @rows;
            # Header row
            push @rows, _parse_table_row($line);
            $i++;  # skip header line
            # Parse alignments from separator
            my @aligns = _parse_table_aligns($lines->[$i]);
            $i++;  # skip separator
            # Data rows
            while ($i < $n && $lines->[$i] =~ /\|/) {
                push @rows, _parse_table_row($lines->[$i]);
                $i++;
            }
            push @blocks, {
                type   => 'table',
                aligns => \@aligns,
                rows   => \@rows,
            };
            next;
        }

        # --- Fenced code block ---
        if ($line =~ /\A\s{0,3}(`{3,}|~{3,})(.*)\z/) {
            my $fence      = $1;
            my $info       = $2;
            $info =~ s/^\s+|\s+$//g;
            my $fence_char = substr($fence, 0, 1);
            my $fence_len  = length($fence);
            $i++;
            my @code_lines;
            while ($i < $n) {
                my $cl = $lines->[$i];
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

        # --- Indented code block ---
        if ($line =~ /\A(?:    |\t)(.*)/) {
            my @code_lines;
            while ($i < $n && ($lines->[$i] =~ /\A(?:    |\t)(.*)/ || $lines->[$i] =~ /\A\s*\z/)) {
                if ($lines->[$i] =~ /\A(?:    |\t)(.*)/) {
                    push @code_lines, $1;
                } else {
                    push @code_lines, '';
                }
                $i++;
            }
            while (@code_lines && $code_lines[-1] =~ /\A\s*\z/) { pop @code_lines; }
            push @blocks, {
                type    => 'code_block',
                info    => '',
                content => join("\n", @code_lines) . "\n",
            };
            next;
        }

        # --- Blockquote ---
        if ($line =~ /\A\s{0,3}>\s?(.*)/) {
            my @bq_lines;
            while ($i < $n) {
                my $bl = $lines->[$i];
                if ($bl =~ /\A\s{0,3}>\s?(.*)/) {
                    push @bq_lines, $1;
                    $i++;
                } elsif ($bl !~ /\A\s*\z/) {
                    push @bq_lines, $bl;
                    $i++;
                } else {
                    last;
                }
            }
            push @blocks, {
                type     => 'blockquote',
                children => _parse_blocks(\@bq_lines),
            };
            next;
        }

        # --- Unordered list (with GFM task list detection) ---
        if ($line =~ /\A\s{0,3}[-+*]\s+(.*)/) {
            my @items;
            while ($i < $n) {
                my $ll = $lines->[$i];
                if ($ll =~ /\A\s{0,3}[-+*]\s+(.*)/) {
                    my $content = $1;
                    my $checked = undef;
                    # GFM task list syntax: - [x] or - [ ]
                    if ($content =~ /\A\[( |x|X)\]\s+(.*)/) {
                        # IMPORTANT: save $1 and $2 BEFORE any other regex,
                        # because a subsequent regex match (even $1 =~ ...)
                        # would clobber $2.
                        my $mark         = $1;
                        my $rest         = $2;
                        $checked = ($mark =~ /[xX]/) ? 1 : 0;
                        $content = $rest;
                    }
                    push @items, {
                        checked  => $checked,
                        children => _parse_inlines($content),
                    };
                    $i++;
                } else {
                    last;
                }
            }
            push @blocks, { type => 'list', ordered => 0, items => \@items };
            next;
        }

        # --- Ordered list ---
        if ($line =~ /\A\s{0,3}\d+\.\s+(.*)/) {
            my @items;
            while ($i < $n) {
                my $ll = $lines->[$i];
                if ($ll =~ /\A\s{0,3}\d+\.\s+(.*)/) {
                    push @items, {
                        checked  => undef,
                        children => _parse_inlines($1),
                    };
                    $i++;
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
            if ($i < $n && $lines->[$i] =~ /\A(?:=+|-+)\s*\z/) {
                my $ul    = $lines->[$i];
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
            push @blocks, {
                type     => 'paragraph',
                children => _parse_inlines(join("\n", @para_lines)),
            };
        }
    }

    return \@blocks;
}

# ============================================================================
# GFM table helpers
# ============================================================================

sub _is_table_separator {
    my ($line) = @_;
    return 0 unless defined $line;
    return $line =~ /\A\s*\|?\s*:?-{1,}:?\s*(\|\s*:?-{1,}:?\s*)*\|?\s*\z/;
}

sub _parse_table_row {
    my ($line) = @_;
    $line =~ s/^\s*\|\s*//;
    $line =~ s/\s*\|\s*$//;
    my @cells = split /\s*\|\s*/, $line;
    return [ map { _parse_inlines($_) } @cells ];
}

sub _parse_table_aligns {
    my ($line) = @_;
    $line =~ s/^\s*\|\s*//;
    $line =~ s/\s*\|\s*$//;
    my @cols  = split /\s*\|\s*/, $line;
    my @aligns;
    for my $col (@cols) {
        $col =~ s/^\s+|\s+$//g;
        if ($col =~ /\A:.*:\z/) { push @aligns, 'center'; }
        elsif ($col =~ /:\z/)   { push @aligns, 'right';  }
        elsif ($col =~ /\A:/)   { push @aligns, 'left';   }
        else                    { push @aligns, '';        }
    }
    return @aligns;
}

# ============================================================================
# INLINE-LEVEL PARSER (GFM superset of CommonMark)
# ============================================================================

sub _parse_inlines {
    my ($text) = @_;
    $text //= '';
    my @nodes;
    my $pos = 0;
    my $len = length($text);

    while ($pos < $len) {
        my $remaining = substr($text, $pos);

        # Image
        if ($remaining =~ /\A!\[([^\]]*)\]\(([^)]*)\)/) {
            push @nodes, { type => 'image', alt => $1, src => $2 };
            $pos += length($&);
            next;
        }

        # Link
        if ($remaining =~ /\A\[([^\]]*)\]\(([^)]*)\)/) {
            push @nodes, {
                type     => 'link',
                href     => $2,
                children => _parse_inlines($1),
            };
            $pos += length($&);
            next;
        }

        # GFM Autolink: bare http:// or https:// URLs
        if ($remaining =~ /\A(https?:\/\/[^\s<>\"'`\[\]{}|\\^]+)/) {
            my $url = $1;
            push @nodes, {
                type     => 'link',
                href     => $url,
                children => [ { type => 'text', value => $url } ],
            };
            $pos += length($url);
            next;
        }

        # Inline code
        if ($remaining =~ /\A(`+)(.*?)\1/s) {
            my $code = $2;
            $code =~ s/^\s+|\s+$//g if $code =~ /\A\s.*\s\z/;
            push @nodes, { type => 'code', value => $code };
            $pos += length($&);
            next;
        }

        # GFM Strikethrough: ~~text~~
        # Possessive quantifier (++) prevents ReDoS on unterminated input.
        if ($remaining =~ /\A~~((?:(?!~~).)++)~~/s) {
            push @nodes, {
                type     => 'strikethrough',
                children => _parse_inlines($1),
            };
            $pos += length($&);
            next;
        }

        # Bold: **text** or __text__
        # Possessive quantifiers prevent catastrophic backtracking when there
        # is no closing delimiter (ReDoS mitigation).
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

        # Italic: *text*
        if ($remaining =~ /\A\*((?:[^*])++)\*/s) {
            push @nodes, {
                type     => 'emph',
                children => _parse_inlines($1),
            };
            $pos += length($&);
            next;
        }

        # Italic: _text_
        if ($remaining =~ /\A_((?:[^_])++)_/s) {
            push @nodes, {
                type     => 'emph',
                children => _parse_inlines($1),
            };
            $pos += length($&);
            next;
        }

        # Hard line break
        if ($remaining =~ /\A  +\n/) {
            push @nodes, { type => 'hardbreak' };
            $pos += length($&);
            next;
        }

        # Soft line break
        if ($remaining =~ /\A\n/) {
            push @nodes, { type => 'softbreak' };
            $pos++;
            next;
        }

        # Plain text: consume up to the next special character.
        # We exclude backtick, *, _, [, !, newline, ~, backslash.
        # We also stop before https?:// so that the autolink rule above
        # gets a chance to match when we next loop around.
        if ($remaining =~ /\A((?:(?!https?:\/\/).)[^`*_\[!\n~\\])+/) {
            push @nodes, { type => 'text', value => $& };
            $pos += length($&);
            next;
        }
        # Fallback single char (handles the case of a lone special char)
        if ($remaining =~ /\A([^`*_\[!\n~\\])/) {
            push @nodes, { type => 'text', value => $1 };
            $pos += length($1);
            next;
        }

        # Escaped character
        if ($remaining =~ /\A\\(.)/) {
            push @nodes, { type => 'text', value => $1 };
            $pos += 2;
            next;
        }

        # Any other single character
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
        # HTML-escape the language tag before embedding in a class attribute.
        # Without this, a language string like: foo" onload="alert(1)
        # would inject an attribute into the <code> element (XSS).
        my $lang = $block->{info}
            ? qq( class="language-) . _escape_html($block->{info}) . '"'
            : '';
        return "<pre><code$lang>" . _escape_html($block->{content}) . "</code></pre>\n";
    }

    if ($type eq 'blockquote') {
        return "<blockquote>\n" . _render_blocks($block->{children}) . "</blockquote>\n";
    }

    if ($type eq 'list') {
        my $tag = $block->{ordered} ? 'ol' : 'ul';
        my $out = "<$tag>\n";
        for my $item (@{ $block->{items} }) {
            $out .= '<li>';
            # GFM task list checkbox
            if (defined $item->{checked}) {
                my $checked = $item->{checked} ? ' checked' : '';
                $out .= qq(<input type="checkbox"$checked disabled> );
            }
            $out .= _render_inlines($item->{children});
            $out .= "</li>\n";
        }
        $out .= "</$tag>\n";
        return $out;
    }

    # GFM Table
    if ($type eq 'table') {
        my @rows   = @{ $block->{rows} };
        my @aligns = @{ $block->{aligns} };
        my $out    = "<table>\n";

        # Whitelist of valid alignment values — prevents injection into style attribute.
        # _parse_table_aligns() only produces these values, but we validate defensively.
        my %VALID_ALIGNS = map { $_ => 1 } qw(left right center);

        # Header row
        if (@rows) {
            $out .= "<thead>\n<tr>\n";
            my $header = shift @rows;
            for my $j (0 .. $#$header) {
                my $align  = $VALID_ALIGNS{ $aligns[$j] // '' } ? $aligns[$j] : '';
                my $astyle = $align ? qq( style="text-align: $align") : '';
                $out .= "<th$astyle>" . _render_inlines($header->[$j]) . "</th>\n";
            }
            $out .= "</tr>\n</thead>\n";
        }

        # Body rows
        if (@rows) {
            $out .= "<tbody>\n";
            for my $row (@rows) {
                $out .= "<tr>\n";
                for my $j (0 .. $#$row) {
                    my $align  = $VALID_ALIGNS{ $aligns[$j] // '' } ? $aligns[$j] : '';
                    my $astyle = $align ? qq( style="text-align: $align") : '';
                    $out .= "<td$astyle>" . _render_inlines($row->[$j]) . "</td>\n";
                }
                $out .= "</tr>\n";
            }
            $out .= "</tbody>\n";
        }

        $out .= "</table>\n";
        return $out;
    }

    if ($type eq 'thematic_break') {
        return "<hr />\n";
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
    return '<del>' . _render_inlines($node->{children}) . '</del>' if $type eq 'strikethrough';
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
# HTML escaping
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

sub _sanitize_html {
    my ($html) = @_;

    # Remove script tags and their contents (loop handles nested/malformed cases)
    1 while $html =~ s/<script\b[^>]*>.*?<\/script>//gis;

    # Remove iframe and object tags
    $html =~ s/<iframe\b[^>]*>.*?<\/iframe>//gis;
    $html =~ s/<object\b[^>]*>.*?<\/object>//gis;

    # Remove on* event attributes.  The (?<!\w) lookbehind fires even when the
    # handler is at the very start of the attribute string (no leading whitespace
    # required), closing the bypass vector that \s+on\w+ left open.
    $html =~ s/(?<!\w)on[a-z]\w*\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]*)//gi;

    # Remove dangerous URL schemes from href/src/action attributes.
    # Covers javascript:, vbscript:, data:, and blob: — all can execute code.
    my $DANGEROUS_SCHEME = qr/(?:javascript|vbscript|data|blob)\s*:/i;
    $html =~ s/(?:href|src|action)\s*=\s*"$DANGEROUS_SCHEME[^"]*"//gi;
    $html =~ s/(?:href|src|action)\s*=\s*'$DANGEROUS_SCHEME[^']*'//gi;
    $html =~ s/(?:href|src|action)\s*=\s*(?!["'])$DANGEROUS_SCHEME\S*//gi;

    return $html;
}

1;

__END__

=head1 NAME

CodingAdventures::Gfm - GitHub Flavored Markdown renderer

=head1 SYNOPSIS

    use CodingAdventures::Gfm qw(render parse to_html);

    # Tables
    my $html = render("| A | B |\n|---|---|\n| 1 | 2 |\n");

    # Task lists
    my $html = render("- [x] Done\n- [ ] Todo\n");

    # Strikethrough
    my $html = render("~~old text~~");

=head1 DESCRIPTION

A self-contained GitHub Flavored Markdown parser and HTML renderer.
Extends CommonMark with GFM tables, task lists, strikethrough, and autolinks.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
