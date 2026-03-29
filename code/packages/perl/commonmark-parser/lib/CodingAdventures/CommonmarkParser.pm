package CodingAdventures::CommonmarkParser;

# ============================================================================
# CodingAdventures::CommonmarkParser
# ============================================================================
#
# A subset-CommonMark Markdown parser that converts Markdown text into a
# Document AST. The AST uses plain Perl hashrefs throughout — no objects,
# no blessed references — so it integrates naturally with any renderer.
#
# === Supported Syntax ===
#
#   ATX headings:       # H1 through ###### H6
#   Setext headings:    underline with === or --- on the next line
#   Fenced code blocks: ``` or ~~~ with optional language tag
#   Thematic breaks:    ---, ***, ___ (3+ chars, optional spaces)
#   Blockquotes:        > lines
#   Bullet lists:       - item or * item (with nesting by indentation)
#   Ordered lists:      1. item or 1) item
#   Paragraphs:         contiguous lines of text
#   Blank lines:        separate block-level elements
#
#   Bold:               **text** or __text__
#   Italic:             *text* or _text_
#   Inline code:        `code`
#   Links:              [text](url) or [text](url "title")
#   Images:             ![alt](src) or ![alt](src "title")
#   Hard break:         two or more spaces at end of line
#   Soft break:         single newline within a paragraph
#
# === Two-Phase Architecture ===
#
# CommonMark parsing requires two distinct phases:
#
#   Phase 1 — Block structure: reads lines and builds a tree of block
#   containers (headings, paragraphs, code blocks, lists, etc.), storing
#   the raw inline text content as strings.
#
#   Phase 2 — Inline content: for each block's raw text, parses inline
#   syntax (emphasis, links, images, code spans) into inline AST nodes.
#
# The phases cannot be merged because a * that begins a list item is
# structural (block phase), while a * inside paragraph text is inline
# syntax (italic or bold).
#
# === AST Node Format ===
#
# All nodes are hashrefs. The "type" key identifies the kind:
#
#   {type => "document",   children => [...]}
#   {type => "heading",    level => N, children => [...]}
#   {type => "paragraph",  children => [...]}
#   {type => "code_block", language => "perl", value => "..."}
#   {type => "blockquote", children => [...]}
#   {type => "list",       ordered => bool, tight => bool,
#                          start => N, children => [...]}
#   {type => "list_item",  children => [...]}
#   {type => "thematic_break"}
#   {type => "text",        value => "..."}
#   {type => "strong",      children => [...]}
#   {type => "emphasis",    children => [...]}
#   {type => "code_span",   value => "..."}
#   {type => "link",        destination => "...", title => undef, children => [...]}
#   {type => "image",       destination => "...", title => undef, alt => "..."}
#   {type => "hard_break"}
#   {type => "soft_break"}
#
# @module CodingAdventures::CommonmarkParser

use strict;
use warnings;

our $VERSION = '0.01';

# ─── Block Parser ─────────────────────────────────────────────────────────────

# _is_blank($line) → bool
#
# A blank line is one that contains only whitespace (spaces, tabs) or is
# entirely empty. Blank lines are the primary separator between block elements.
sub _is_blank {
    return $_[0] =~ /^\s*$/;
}

# _parse_atx_heading($line) → {level, content} or undef
#
# ATX headings start with 1-6 # characters followed by a space or end of line.
# Trailing # characters (with optional surrounding spaces) are stripped.
#
#   "# Hello"      → {level=>1, content=>"Hello"}
#   "## Bye ##"    → {level=>2, content=>"Bye"}
#   "##no space"   → undef (no space after hashes)
#   "####### too"  → undef (7 hashes is not a heading)
sub _parse_atx_heading {
    my ($line) = @_;
    # Allow up to 3 leading spaces
    $line =~ s/^ {0,3}//;
    return undef unless $line =~ /^(#{1,6})([ \t]|$)(.*)/;
    my ($hashes, $rest) = ($1, $3);
    # Strip trailing hashes: optional spaces + hashes + optional spaces
    $rest =~ s/\s+#+\s*$//;
    $rest =~ s/^\s+|\s+$//g;
    return {level => length($hashes), content => $rest};
}

# _is_thematic_break($line) → bool
#
# A thematic break is 3+ of the same character (*, -, _) with optional spaces.
# Up to 3 leading spaces are allowed.
#
#   "---"        → true
#   "* * *"      → true
#   "___"        → true
#   "- - -x"     → false (contains non-matching character)
sub _is_thematic_break {
    my ($line) = @_;
    $line =~ s/^ {0,3}//;
    $line =~ s/\s+$//;
    for my $ch ('*', '-', '_') {
        my $stripped = $line;
        $stripped =~ s/[ \t]//g;
        next if length($stripped) < 3;
        next if $stripped =~ /[^\Q$ch\E]/;
        return 1 if $line =~ /^[\Q$ch\E \t]+$/;
    }
    return 0;
}

# _parse_fenced_code_open($line) → {fence_char, fence_len, language} or undef
#
# A fenced code block opens with 3+ backticks or tildes.
# The info string (language) is taken from the rest of the line.
sub _parse_fenced_code_open {
    my ($line) = @_;
    $line =~ s/^ {0,3}//;
    if ($line =~ /^(`{3,}|~{3,})(.*)$/) {
        my ($fence, $info) = ($1, $2);
        my $fence_char = substr($fence, 0, 1);
        my $fence_len  = length($fence);
        # Language is the first word of the info string
        $info =~ s/^\s+|\s+$//g;
        my ($lang) = split(/\s+/, $info, 2);
        $lang //= '';
        return {fence_char => $fence_char, fence_len => $fence_len, language => $lang};
    }
    return undef;
}

# _parse_list_marker($line) → {type, bullet, number, delimiter, indent, rest} or undef
#
# Detects a list item marker at the start of a line.
#
# Bullet list:   "- item", "* item", "+ item"
# Ordered list:  "1. item", "2) item"
#
# Returns the marker metadata and the rest of the line content.
sub _parse_list_marker {
    my ($line) = @_;
    # Allow up to 3 leading spaces
    my $indent = 0;
    if ($line =~ /^( {0,3})(.*)$/) {
        $indent = length($1);
        $line   = $2;
    }
    # Bullet list
    if ($line =~ /^([-*+])([ \t])(.*)$/) {
        my ($bullet, $space, $rest) = ($1, $2, $3);
        return {
            type      => 'bullet',
            bullet    => $bullet,
            indent    => $indent + 1 + length($space),
            rest      => $rest,
        };
    }
    # Bullet with no content after marker
    if ($line =~ /^([-*+])$/) {
        return {
            type   => 'bullet',
            bullet => $1,
            indent => $indent + 1,
            rest   => '',
        };
    }
    # Ordered list
    if ($line =~ /^(\d{1,9})([.)]) (.*)$/) {
        my ($num, $delim, $rest) = ($1, $2, $3);
        return {
            type      => 'ordered',
            number    => int($num),
            delimiter => $delim,
            indent    => $indent + length($num) + 1 + 1,
            rest      => $rest,
        };
    }
    if ($line =~ /^(\d{1,9})([.)])$/) {
        my ($num, $delim) = ($1, $2);
        return {
            type      => 'ordered',
            number    => int($num),
            delimiter => $delim,
            indent    => $indent + length($num) + 1,
            rest      => '',
        };
    }
    return undef;
}

# _parse_blocks(\@lines) → \@block_nodes
#
# Phase 1: Parses a list of text lines into block-level AST nodes.
# Returns an arrayref of block nodes (without inline parsing yet —
# paragraph and heading nodes carry raw text strings in their content).
sub _parse_blocks {
    my ($lines) = @_;
    my @blocks;
    my $i = 0;
    my $n = scalar @$lines;

    while ($i < $n) {
        my $line = $lines->[$i];

        # ── Blank line: separator, skip ────────────────────────────────────
        if (_is_blank($line)) {
            $i++;
            next;
        }

        # ── ATX Heading ─────────────────────────────────────────────────────
        my $atx = _parse_atx_heading($line);
        if ($atx) {
            push @blocks, {
                type    => 'heading',
                level   => $atx->{level},
                _raw    => $atx->{content},
            };
            $i++;
            next;
        }

        # ── Thematic Break ──────────────────────────────────────────────────
        if (_is_thematic_break($line)) {
            push @blocks, {type => 'thematic_break'};
            $i++;
            next;
        }

        # ── Fenced Code Block ───────────────────────────────────────────────
        my $fence_info = _parse_fenced_code_open($line);
        if ($fence_info) {
            $i++;
            my @code_lines;
            while ($i < $n) {
                my $cline = $lines->[$i];
                # Closing fence: same or more of the fence character
                if ($cline =~ /^ {0,3}\Q$fence_info->{fence_char}\E{$fence_info->{fence_len},}\s*$/) {
                    $i++;
                    last;
                }
                # Strip up to 3 leading spaces (indentation of the fence opening)
                $cline =~ s/^ {0,3}//;
                push @code_lines, $cline;
                $i++;
            }
            push @blocks, {
                type     => 'code_block',
                language => $fence_info->{language},
                value    => join("\n", @code_lines) . (scalar @code_lines ? "\n" : ""),
            };
            next;
        }

        # ── Blockquote ───────────────────────────────────────────────────────
        if ($line =~ /^ {0,3}> ?(.*)$/) {
            my @bq_lines;
            while ($i < $n) {
                my $bqline = $lines->[$i];
                if ($bqline =~ /^ {0,3}> ?(.*)$/) {
                    push @bq_lines, $1;
                    $i++;
                } elsif (!_is_blank($bqline)) {
                    # Lazy continuation
                    push @bq_lines, $bqline;
                    $i++;
                } else {
                    last;
                }
            }
            my $inner = _parse_blocks(\@bq_lines);
            push @blocks, {type => 'blockquote', children => $inner};
            next;
        }

        # ── List ─────────────────────────────────────────────────────────────
        my $marker = _parse_list_marker($line);
        if ($marker) {
            my $ordered    = $marker->{type} eq 'ordered';
            my $list_start = $ordered ? $marker->{number} : 1;
            my $bullet_ch  = $ordered ? undef : $marker->{bullet};
            my $delimiter  = $ordered ? $marker->{delimiter} : undef;
            my @items;
            my $tight = 1;  # assumed tight until we see blank lines between items

            while ($i < $n) {
                my $mline   = $lines->[$i];
                my $current = _parse_list_marker($mline);
                # Verify we continue the same list type
                if ($current) {
                    if ($ordered && $current->{type} ne 'ordered') { last }
                    if ($ordered && defined $delimiter && $current->{delimiter} ne $delimiter) { last }
                    if (!$ordered && $current->{type} ne 'bullet') { last }
                    if (!$ordered && defined $bullet_ch && $current->{bullet} ne $bullet_ch) { last }
                } elsif (_is_blank($mline)) {
                    last unless $i + 1 < $n && !_is_blank($lines->[$i + 1]);
                    $tight = 0;
                    $i++;
                    next;
                } else {
                    last;
                }

                # Collect this item's lines
                $i++;
                my @item_lines = ($current->{rest});
                my $item_indent = $current->{indent};

                while ($i < $n) {
                    my $iline = $lines->[$i];
                    if (_is_blank($iline)) {
                        # Blank line may be inside item (loose) or end of item
                        if ($i + 1 < $n) {
                            my $next_line = $lines->[$i + 1];
                            # Check if next non-blank is indented enough to continue
                            if (!_is_blank($next_line) && $next_line =~ /^( {$item_indent,})/) {
                                push @item_lines, '';
                                $tight = 0;
                                $i++;
                                next;
                            } elsif (!_is_blank($next_line) && _parse_list_marker($next_line)) {
                                # Next line is another list item at same level
                                $tight = 0 if !_is_blank($lines->[$i]);
                                last;
                            } else {
                                last;
                            }
                        } else {
                            last;
                        }
                    } elsif ($iline =~ /^( {$item_indent,})(.*)$/) {
                        push @item_lines, $2;
                        $i++;
                    } elsif (_parse_list_marker($iline)) {
                        last;
                    } else {
                        last;
                    }
                }

                my $item_children = _parse_blocks(\@item_lines);
                push @items, {type => 'list_item', children => $item_children};
            }

            push @blocks, {
                type     => 'list',
                ordered  => $ordered ? 1 : 0,
                start    => $list_start,
                tight    => $tight,
                children => \@items,
            };
            next;
        }

        # ── Setext Heading ───────────────────────────────────────────────────
        # A setext heading is a non-blank line followed by a line of = or -.
        # Per CommonMark spec: when a paragraph-like line is followed by
        # "---" (even though that would normally be a thematic break), it
        # forms a setext h2. The setext interpretation takes priority.
        if ($i + 1 < $n && !_is_blank($line) && !_parse_atx_heading($line)) {
            my $next = $lines->[$i + 1];
            if ($next =~ /^=+\s*$/) {
                push @blocks, {type => 'heading', level => 1, _raw => $line};
                $i += 2;
                next;
            } elsif ($next =~ /^-+\s*$/) {
                # Setext h2: dashes after a non-blank line form an underline
                push @blocks, {type => 'heading', level => 2, _raw => $line};
                $i += 2;
                next;
            }
        }

        # ── Paragraph ────────────────────────────────────────────────────────
        my @para_lines = ($line);
        $i++;
        while ($i < $n) {
            my $pline = $lines->[$i];
            last if _is_blank($pline);
            last if _parse_atx_heading($pline);
            last if _is_thematic_break($pline);
            last if _parse_fenced_code_open($pline);
            last if $pline =~ /^ {0,3}> ?/;
            push @para_lines, $pline;
            $i++;
        }
        push @blocks, {type => 'paragraph', _raw => join("\n", @para_lines)};
    }

    return \@blocks;
}

# ─── Inline Parser ────────────────────────────────────────────────────────────
#
# The inline parser converts a raw text string (the content of a paragraph,
# heading, etc.) into a list of inline AST nodes.
#
# === How it works ===
#
# We scan through the text character by character. Most characters are
# accumulated into a text buffer. When we encounter a special character
# (*, _, `, [, !), we check if it opens an inline construct:
#
#   ** or __  → strong
#   * or _    → emphasis
#   `         → code span
#   [text](url) → link
#   ![alt](url) → image
#
# The approach is a simplified delimiter-stack parser. For emphasis/strong,
# we find matching opening and closing delimiters.

# _parse_inline($text) → \@inline_nodes
#
# Parses an inline text string into a list of inline nodes.
sub _parse_inline {
    my ($text) = @_;
    return [] if !defined $text || $text eq '';

    my @nodes;
    my $pos = 0;
    my $len = length($text);
    my $buf = '';

    while ($pos < $len) {
        my $ch = substr($text, $pos, 1);

        # ── Escaped character: backslash + ASCII punctuation ──────────────
        if ($ch eq '\\' && $pos + 1 < $len) {
            my $next = substr($text, $pos + 1, 1);
            if ($next =~ /[!"#\$%&'()*+,\-.\/:<=>?@\[\\\]^_`{|}~]/) {
                $buf .= $next;
                $pos += 2;
                next;
            }
        }

        # ── Hard break: two or more spaces at end of line segment ─────────
        if ($ch eq "\n" && $pos > 0) {
            # Check if the preceding characters were spaces
            if ($buf =~ s/  +$//) {
                # Two+ spaces before newline = hard break
                push @nodes, {type => 'text', value => $buf} if $buf ne '';
                push @nodes, {type => 'hard_break'};
                $buf = '';
                $pos++;
                # Skip leading spaces on next line
                while ($pos < $len && substr($text, $pos, 1) eq ' ') { $pos++ }
                next;
            } else {
                # Soft break
                $buf =~ s/\s+$//;  # trailing spaces on the line
                push @nodes, {type => 'text', value => $buf} if $buf ne '';
                push @nodes, {type => 'soft_break'};
                $buf = '';
                $pos++;
                # Skip leading spaces on next line
                while ($pos < $len && substr($text, $pos, 1) eq ' ') { $pos++ }
                next;
            }
        }

        # ── Inline code span: `code` ──────────────────────────────────────
        if ($ch eq '`') {
            # Count opening backticks
            my $tick_start = $pos;
            my $ticks = 0;
            while ($pos < $len && substr($text, $pos, 1) eq '`') {
                $ticks++;
                $pos++;
            }
            # Find matching closing sequence
            my $close_pos = _find_code_span_close($text, $pos, $ticks);
            if (defined $close_pos) {
                push @nodes, {type => 'text', value => $buf} if $buf ne '';
                $buf = '';
                my $code_content = substr($text, $pos, $close_pos - $pos);
                # Strip one leading/trailing space if both present
                if ($code_content =~ /^ (.+) $/ && $code_content !~ /^ +$/) {
                    $code_content = $1;
                }
                push @nodes, {type => 'code_span', value => $code_content};
                $pos = $close_pos + $ticks;
                next;
            } else {
                # No match: emit the backticks as literal text
                $buf .= substr($text, $tick_start, $ticks);
                next;
            }
        }

        # ── Image: ![alt](url) ────────────────────────────────────────────
        if ($ch eq '!' && $pos + 1 < $len && substr($text, $pos + 1, 1) eq '[') {
            my $result = _try_parse_link_or_image($text, $pos + 1, 1);
            if ($result) {
                push @nodes, {type => 'text', value => $buf} if $buf ne '';
                $buf = '';
                push @nodes, $result->{node};
                $pos = $result->{end};
                next;
            }
        }

        # ── Link: [text](url) ─────────────────────────────────────────────
        if ($ch eq '[') {
            my $result = _try_parse_link_or_image($text, $pos, 0);
            if ($result) {
                push @nodes, {type => 'text', value => $buf} if $buf ne '';
                $buf = '';
                push @nodes, $result->{node};
                $pos = $result->{end};
                next;
            }
        }

        # ── Emphasis and Strong: **, __, *, _ ─────────────────────────────
        if ($ch eq '*' || $ch eq '_') {
            # Try to parse bold (**) or italic (*)
            my $delim = $ch;
            my $count = 0;
            my $save_pos = $pos;
            while ($pos < $len && substr($text, $pos, 1) eq $delim) {
                $count++;
                $pos++;
            }

            # Try strong (2+ delimiters) first, then emphasis (1)
            my $consumed = 0;
            if ($count >= 2) {
                my $close = _find_delimiter_close($text, $pos, $delim x 2);
                if (defined $close) {
                    push @nodes, {type => 'text', value => $buf} if $buf ne '';
                    $buf = '';
                    my $inner_text = substr($text, $pos, $close - $pos);
                    my $inner = _parse_inline($inner_text);
                    push @nodes, {type => 'strong', children => $inner};
                    $pos = $close + 2;
                    # If there were extra delimiters (e.g. ***), re-process remainder
                    if ($count > 2) {
                        my $extra = $delim x ($count - 2);
                        $buf .= $extra;
                    }
                    $consumed = 1;
                }
            }
            unless ($consumed) {
                # Try single emphasis
                my $close = _find_delimiter_close($text, $pos - ($count - 1), $delim);
                if (defined $close) {
                    push @nodes, {type => 'text', value => $buf} if $buf ne '';
                    $buf = '';
                    # Include extra leading delimiters as text if count > 1
                    my $leading = $count > 1 ? ($delim x ($count - 1)) : '';
                    my $inner_text = substr($text, $pos - ($count - 1), $close - ($pos - ($count - 1)));
                    my $inner = _parse_inline($inner_text);
                    push @nodes, {type => 'text', value => $leading} if $leading;
                    push @nodes, {type => 'emphasis', children => $inner};
                    $pos = $close + 1;
                    $consumed = 1;
                }
            }
            unless ($consumed) {
                # No match: emit as literal text
                $buf .= $delim x $count;
            }
            next;
        }

        # ── Default: accumulate into text buffer ──────────────────────────
        $buf .= $ch;
        $pos++;
    }

    push @nodes, {type => 'text', value => $buf} if $buf ne '';
    return \@nodes;
}

# _find_code_span_close($text, $start, $tick_count) → position or undef
#
# Finds the closing backtick sequence of exactly $tick_count backticks,
# starting the search at $start. Returns the position of the first backtick
# in the closing sequence, or undef if not found.
sub _find_code_span_close {
    my ($text, $start, $tick_count) = @_;
    my $len = length($text);
    my $pos = $start;
    while ($pos < $len) {
        my $idx = index($text, '`', $pos);
        last if $idx < 0;
        # Count consecutive backticks at this position
        my $cnt = 0;
        my $p   = $idx;
        while ($p < $len && substr($text, $p, 1) eq '`') {
            $cnt++;
            $p++;
        }
        if ($cnt == $tick_count) {
            return $idx;
        }
        $pos = $p;
    }
    return undef;
}

# _find_delimiter_close($text, $start, $delim) → position or undef
#
# Finds the closing delimiter string, starting search at $start.
# Does not handle nested delimiters (simplified parser).
sub _find_delimiter_close {
    my ($text, $start, $delim) = @_;
    my $len      = length($text);
    my $delim_ch = substr($delim, 0, 1);
    my $pos      = $start;

    while ($pos < $len) {
        my $idx = index($text, $delim, $pos);
        return undef if $idx < 0;
        # Verify the preceding character is not the same delimiter (avoid ***text** matching)
        # For single-char delimiters that's fine; just return
        return $idx;
    }
    return undef;
}

# _try_parse_link_or_image($text, $bracket_pos, $is_image) → {node, end} or undef
#
# Tries to parse a [text](url "title") or ![alt](url "title") construct
# starting at $bracket_pos (which points to the '[').
#
# Returns a hashref {node => ..., end => ...} on success, undef on failure.
sub _try_parse_link_or_image {
    my ($text, $bracket_pos, $is_image) = @_;
    my $len = length($text);

    return undef if $bracket_pos >= $len;
    return undef if substr($text, $bracket_pos, 1) ne '[';

    # Find matching ]
    my $close_bracket = _find_matching_bracket($text, $bracket_pos + 1);
    return undef unless defined $close_bracket;

    # Must be followed by (
    return undef unless $close_bracket + 1 < $len;
    return undef unless substr($text, $close_bracket + 1, 1) eq '(';

    # Find matching )
    my $close_paren = _find_matching_paren($text, $close_bracket + 2);
    return undef unless defined $close_paren;

    my $link_text = substr($text, $bracket_pos + 1, $close_bracket - $bracket_pos - 1);
    my $link_dest = substr($text, $close_bracket + 2, $close_paren - $close_bracket - 2);

    # Parse destination and optional title from link_dest
    my ($dest, $title) = _parse_link_destination($link_dest);

    my $end_pos = $close_paren + 1;
    if ($is_image) {
        $end_pos++;  # skip the leading !
    }

    my $node;
    if ($is_image) {
        $node = {
            type        => 'image',
            destination => $dest,
            title       => $title,
            alt         => $link_text,
        };
    } else {
        my $children = _parse_inline($link_text);
        $node = {
            type        => 'link',
            destination => $dest,
            title       => $title,
            children    => $children,
        };
    }

    return {node => $node, end => $end_pos};
}

# _find_matching_bracket($text, $start) → position or undef
#
# Finds the position of the matching ] for a [ that occurred just before
# $start. Handles one level of nesting (e.g., [a [b] c]).
sub _find_matching_bracket {
    my ($text, $start) = @_;
    my $len   = length($text);
    my $depth = 1;
    my $pos   = $start;
    while ($pos < $len) {
        my $ch = substr($text, $pos, 1);
        if ($ch eq '[')  { $depth++ }
        elsif ($ch eq ']') {
            $depth--;
            return $pos if $depth == 0;
        }
        $pos++;
    }
    return undef;
}

# _find_matching_paren($text, $start) → position or undef
#
# Finds the position of the matching ) starting search at $start.
sub _find_matching_paren {
    my ($text, $start) = @_;
    my $len   = length($text);
    my $depth = 1;
    my $pos   = $start;
    my $in_angle = 0;
    while ($pos < $len) {
        my $ch = substr($text, $pos, 1);
        if ($ch eq '<') {
            $in_angle = 1;
        } elsif ($ch eq '>') {
            $in_angle = 0;
        } elsif (!$in_angle) {
            if ($ch eq '(')  { $depth++ }
            elsif ($ch eq ')') {
                $depth--;
                return $pos if $depth == 0;
            }
        }
        $pos++;
    }
    return undef;
}

# _parse_link_destination($raw) → ($dest, $title)
#
# Parses the content inside (...) of a link or image:
#   "https://example.com"        → ("https://example.com", undef)
#   "url 'My Title'"             → ("url", "My Title")
#   "url \"My Title\""           → ("url", "My Title")
#   "<url with spaces> 'title'"  → ("url with spaces", "title")
sub _parse_link_destination {
    my ($raw) = @_;
    $raw =~ s/^\s+|\s+$//g;

    my $dest  = '';
    my $title = undef;

    # Angle-bracket destination
    if ($raw =~ /^<([^>]*)>(.*)$/) {
        $dest = $1;
        my $rest = $2;
        $rest =~ s/^\s+//;
        $title = _parse_title($rest);
        return ($dest, $title);
    }

    # Regular destination: read until space or end
    if ($raw =~ /^(\S+)(.*)$/) {
        $dest = $1;
        my $rest = $2;
        $rest =~ s/^\s+//;
        $title = _parse_title($rest) if $rest ne '';
        return ($dest, $title);
    }

    return ($raw, undef);
}

# _parse_title($raw) → string or undef
#
# Parses a link title from a string like:
#   '"My Title"'   → "My Title"
#   "'My Title'"   → "My Title"
#   "(My Title)"   → "My Title"
sub _parse_title {
    my ($raw) = @_;
    return undef if !defined $raw || $raw eq '';
    if ($raw =~ /^"(.*)"$/)  { return $1 }
    if ($raw =~ /^'(.*)'$/)  { return $1 }
    if ($raw =~ /^\((.*)\)$/) { return $1 }
    return undef;
}

# _finalize_blocks(\@blocks) → \@blocks
#
# Phase 2: Walk the block tree and replace _raw string content with parsed
# inline nodes. Called after _parse_blocks completes.
sub _finalize_blocks {
    my ($blocks) = @_;
    my @result;
    for my $block (@$blocks) {
        my $t = $block->{type};
        if ($t eq 'heading' || $t eq 'paragraph') {
            my $raw      = $block->{_raw} // '';
            my $children = _parse_inline($raw);
            push @result, {type => $t, ($t eq 'heading' ? (level => $block->{level}) : ()), children => $children};
        } elsif ($t eq 'blockquote') {
            my $inner = _finalize_blocks($block->{children});
            push @result, {type => 'blockquote', children => $inner};
        } elsif ($t eq 'list') {
            my @items;
            for my $item (@{$block->{children}}) {
                my $inner = _finalize_blocks($item->{children});
                push @items, {type => 'list_item', children => $inner};
            }
            push @result, {
                type     => 'list',
                ordered  => $block->{ordered},
                start    => $block->{start},
                tight    => $block->{tight},
                children => \@items,
            };
        } else {
            push @result, $block;
        }
    }
    return \@result;
}

# ─── Public API ───────────────────────────────────────────────────────────────

# parse($markdown) → document AST hashref
#
# Parses a Markdown string and returns a document AST node.
#
# The result is a hashref: {type => "document", children => [...]}
#
# Example:
#
#   use CodingAdventures::CommonmarkParser;
#
#   my $doc = CodingAdventures::CommonmarkParser::parse("# Hello\n\nWorld\n");
#   # $doc->{type}              eq "document"
#   # $doc->{children}[0]{type} eq "heading"
#   # $doc->{children}[0]{level} == 1
#   # $doc->{children}[1]{type} eq "paragraph"
sub parse {
    my ($markdown) = @_;
    $markdown //= '';

    # Normalize line endings: \r\n and \r → \n
    $markdown =~ s/\r\n/\n/g;
    $markdown =~ s/\r/\n/g;

    # Split into lines (preserving empty lines)
    my @lines = split(/\n/, $markdown, -1);
    # Remove trailing empty lines that come from the final newline
    pop @lines if @lines && $lines[-1] eq '';

    my $blocks = _parse_blocks(\@lines);
    my $final  = _finalize_blocks($blocks);

    return {type => 'document', children => $final};
}

1;

__END__

=head1 NAME

CodingAdventures::CommonmarkParser - Pure-Perl CommonMark subset parser

=head1 SYNOPSIS

    use CodingAdventures::CommonmarkParser;

    my $doc = CodingAdventures::CommonmarkParser::parse("# Hello\n\nWorld.\n");
    # $doc->{type} eq "document"
    # $doc->{children}[0]{type} eq "heading"

=head1 DESCRIPTION

Parses a subset of CommonMark Markdown into a Document AST (plain Perl
hashrefs). Supports ATX and setext headings, fenced code blocks, paragraphs,
thematic breaks, blockquotes, bullet and ordered lists, bold, italic,
inline code, links, and images.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
