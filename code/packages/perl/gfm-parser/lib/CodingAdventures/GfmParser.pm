package CodingAdventures::GfmParser;

# ============================================================================
# CodingAdventures::GfmParser
# ============================================================================
#
# GitHub Flavored Markdown (GFM) parser. Extends CommonMark with four
# additional features:
#
#   1. Tables          — pipe-separated rows with an alignment row
#   2. Task list items — "- [ ] unchecked" and "- [x] checked"
#   3. Strikethrough   — ~~text~~
#   4. Autolinks       — bare URLs and email addresses wrapped in <>
#
# This parser builds on the same two-phase architecture as CommonmarkParser:
#
#   Phase 1 — Block structure: reads lines into block nodes (headings,
#   paragraphs, lists, tables, etc.).
#
#   Phase 2 — Inline content: converts raw text strings into inline nodes
#   (emphasis, links, strikethrough, etc.).
#
# === GFM Table Syntax ===
#
#   | Column A | Column B |
#   |----------|----------|
#   | cell 1   | cell 2   |
#   | cell 3   | cell 4   |
#
# The separator row (dashes) may contain optional colons for alignment:
#   |:---------|  — left aligned
#   |---------:|  — right aligned
#   |:--------:|  — centered
#
# Tables produce this AST structure:
#
#   {
#     type => "table",
#     children => [
#       {type => "table_row", is_header => 1, children => [{type => "table_cell", ...}, ...]},
#       {type => "table_row", is_header => 0, children => [...]},
#       ...
#     ]
#   }
#
# === GFM Task List Items ===
#
# A task item is a list item whose content starts with "[ ]" or "[x]":
#
#   - [ ] unchecked task
#   - [x] checked task
#
# These produce:
#   {type => "task_item", checked => 0/1, children => [...]}
#
# === GFM Strikethrough ===
#
#   ~~text~~ → {type => "strikethrough", children => [...text nodes...]}
#
# === GFM Autolinks ===
#
#   <https://example.com>   → {type => "autolink", destination => "https://...", is_email => 0}
#   <user@example.com>      → {type => "autolink", destination => "user@...", is_email => 1}
#
# @module CodingAdventures::GfmParser

use strict;
use warnings;

our $VERSION = '0.01';

# ─── Block Parser Utilities ───────────────────────────────────────────────────

sub _is_blank {
    return $_[0] =~ /^\s*$/;
}

sub _parse_atx_heading {
    my ($line) = @_;
    $line =~ s/^ {0,3}//;
    return undef unless $line =~ /^(#{1,6})([ \t]|$)(.*)/;
    my ($hashes, $rest) = ($1, $3);
    $rest =~ s/\s+#+\s*$//;
    $rest =~ s/^\s+|\s+$//g;
    return {level => length($hashes), content => $rest};
}

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

sub _parse_fenced_code_open {
    my ($line) = @_;
    $line =~ s/^ {0,3}//;
    if ($line =~ /^(`{3,}|~{3,})(.*)$/) {
        my ($fence, $info) = ($1, $2);
        my $fence_char = substr($fence, 0, 1);
        my $fence_len  = length($fence);
        $info =~ s/^\s+|\s+$//g;
        my ($lang) = split(/\s+/, $info, 2);
        $lang //= '';
        return {fence_char => $fence_char, fence_len => $fence_len, language => $lang};
    }
    return undef;
}

sub _parse_list_marker {
    my ($line) = @_;
    my $indent = 0;
    if ($line =~ /^( {0,3})(.*)$/) {
        $indent = length($1);
        $line   = $2;
    }
    if ($line =~ /^([-*+])([ \t])(.*)$/) {
        return {type => 'bullet', bullet => $1, indent => $indent + 1 + length($2), rest => $3};
    }
    if ($line =~ /^([-*+])$/) {
        return {type => 'bullet', bullet => $1, indent => $indent + 1, rest => ''};
    }
    if ($line =~ /^(\d{1,9})([.)]) (.*)$/) {
        return {type => 'ordered', number => int($1), delimiter => $2,
                indent => $indent + length($1) + 1 + 1, rest => $3};
    }
    if ($line =~ /^(\d{1,9})([.)])$/) {
        return {type => 'ordered', number => int($1), delimiter => $2,
                indent => $indent + length($1) + 1, rest => ''};
    }
    return undef;
}

# ─── GFM Table Detection ─────────────────────────────────────────────────────

# _is_table_separator($line) → bool
#
# A table separator row consists of cells with at least one dash, optionally
# surrounded by colons, separated by pipes.
#
# Valid: | --- | :---: | ---: |
# Valid: |---|---|
sub _is_table_separator {
    my ($line) = @_;
    $line =~ s/^\s*\|?\s*//;
    $line =~ s/\s*\|?\s*$//;
    # Must be only dashes, colons, pipes, and spaces
    return 0 unless $line =~ /^[|:\-\s]+$/;
    my @cells = split(/\s*\|\s*/, $line);
    return 0 if @cells < 1;
    for my $cell (@cells) {
        $cell =~ s/^\s+|\s+$//g;
        return 0 unless $cell =~ /^:?-+:?$/;
    }
    return 1;
}

# _split_table_row($line) → \@cells
#
# Splits a pipe-delimited table row into cell content strings.
# Leading and trailing pipes are optional.
sub _split_table_row {
    my ($line) = @_;
    # Remove leading/trailing pipe and whitespace
    $line =~ s/^\s*\|\s*//;
    $line =~ s/\s*\|\s*$//;
    my @cells = split(/\s*\|\s*/, $line, -1);
    return \@cells;
}

# ─── Block Parser ─────────────────────────────────────────────────────────────

sub _parse_blocks {
    my ($lines) = @_;
    my @blocks;
    my $i = 0;
    my $n = scalar @$lines;

    while ($i < $n) {
        my $line = $lines->[$i];

        # ── Blank line ───────────────────────────────────────────────────────
        if (_is_blank($line)) {
            $i++;
            next;
        }

        # ── ATX Heading ──────────────────────────────────────────────────────
        my $atx = _parse_atx_heading($line);
        if ($atx) {
            push @blocks, {type => 'heading', level => $atx->{level}, _raw => $atx->{content}};
            $i++;
            next;
        }

        # ── Thematic Break ───────────────────────────────────────────────────
        if (_is_thematic_break($line)) {
            push @blocks, {type => 'thematic_break'};
            $i++;
            next;
        }

        # ── Fenced Code Block ─────────────────────────────────────────────────
        my $fence_info = _parse_fenced_code_open($line);
        if ($fence_info) {
            $i++;
            my @code_lines;
            while ($i < $n) {
                my $cline = $lines->[$i];
                if ($cline =~ /^ {0,3}\Q$fence_info->{fence_char}\E{$fence_info->{fence_len},}\s*$/) {
                    $i++;
                    last;
                }
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

        # ── GFM Table ────────────────────────────────────────────────────────
        # A table requires at least two lines: header row + separator row
        if ($i + 1 < $n && $line =~ /\|/ && _is_table_separator($lines->[$i + 1])) {
            my @table_rows;
            # Header row
            my $header_cells = _split_table_row($line);
            push @table_rows, {
                type      => 'table_row',
                is_header => 1,
                _raw_cells => $header_cells,
            };
            $i += 2;  # skip header and separator

            # Body rows
            while ($i < $n && !_is_blank($lines->[$i])) {
                my $tline = $lines->[$i];
                last unless $tline =~ /\|/ || $tline =~ /^\|/;
                my $cells = _split_table_row($tline);
                push @table_rows, {
                    type       => 'table_row',
                    is_header  => 0,
                    _raw_cells => $cells,
                };
                $i++;
            }

            push @blocks, {type => 'table', _rows => \@table_rows};
            next;
        }

        # ── List ──────────────────────────────────────────────────────────────
        my $marker = _parse_list_marker($line);
        if ($marker) {
            my $ordered    = $marker->{type} eq 'ordered';
            my $list_start = $ordered ? $marker->{number} : 1;
            my $bullet_ch  = $ordered ? undef : $marker->{bullet};
            my $delimiter  = $ordered ? $marker->{delimiter} : undef;
            my @items;
            my $tight = 1;

            while ($i < $n) {
                my $mline   = $lines->[$i];
                my $current = _parse_list_marker($mline);
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

                $i++;
                my @item_lines = ($current->{rest});
                my $item_indent = $current->{indent};

                while ($i < $n) {
                    my $iline = $lines->[$i];
                    if (_is_blank($iline)) {
                        if ($i + 1 < $n) {
                            my $next_line = $lines->[$i + 1];
                            if (!_is_blank($next_line) && $next_line =~ /^( {$item_indent,})/) {
                                push @item_lines, '';
                                $tight = 0;
                                $i++;
                                next;
                            } elsif (!_is_blank($next_line) && _parse_list_marker($next_line)) {
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

                # Detect task list item: first line starts with "[ ] " or "[x] " or "[X] "
                my $is_task   = 0;
                my $checked   = 0;
                my $item_rest = $item_lines[0] // '';
                if ($item_rest =~ s/^\[([ xX])\] ?//) {
                    $is_task = 1;
                    $checked = ($1 ne ' ') ? 1 : 0;
                    $item_lines[0] = $item_rest;
                }

                my $item_children = _parse_blocks(\@item_lines);
                if ($is_task) {
                    push @items, {type => 'task_item', checked => $checked, children => $item_children};
                } else {
                    push @items, {type => 'list_item', children => $item_children};
                }
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
        if ($i + 1 < $n && !_is_blank($line)) {
            my $next = $lines->[$i + 1];
            if ($next =~ /^=+\s*$/) {
                push @blocks, {type => 'heading', level => 1, _raw => $line};
                $i += 2;
                next;
            } elsif ($next =~ /^-+\s*$/ && !_is_thematic_break($next)) {
                push @blocks, {type => 'heading', level => 2, _raw => $line};
                $i += 2;
                next;
            }
        }

        # ── Paragraph ─────────────────────────────────────────────────────────
        my @para_lines = ($line);
        $i++;
        while ($i < $n) {
            my $pline = $lines->[$i];
            last if _is_blank($pline);
            last if _parse_atx_heading($pline);
            last if _is_thematic_break($pline);
            last if _parse_fenced_code_open($pline);
            last if $pline =~ /^ {0,3}> ?/;
            last if ($pline =~ /\|/ && $i + 1 < $n && _is_table_separator($lines->[$i + 1]));
            push @para_lines, $pline;
            $i++;
        }
        push @blocks, {type => 'paragraph', _raw => join("\n", @para_lines)};
    }

    return \@blocks;
}

# ─── Inline Parser ────────────────────────────────────────────────────────────

sub _find_code_span_close {
    my ($text, $start, $tick_count) = @_;
    my $len = length($text);
    my $pos = $start;
    while ($pos < $len) {
        my $idx = index($text, '`', $pos);
        last if $idx < 0;
        my $cnt = 0;
        my $p   = $idx;
        while ($p < $len && substr($text, $p, 1) eq '`') { $cnt++; $p++ }
        return $idx if $cnt == $tick_count;
        $pos = $p;
    }
    return undef;
}

sub _find_delimiter_close {
    my ($text, $start, $delim) = @_;
    my $len = length($text);
    my $pos = $start;
    while ($pos < $len) {
        my $idx = index($text, $delim, $pos);
        return undef if $idx < 0;
        return $idx;
    }
    return undef;
}

sub _find_matching_bracket {
    my ($text, $start) = @_;
    my $len = length($text);
    my $depth = 1;
    my $pos = $start;
    while ($pos < $len) {
        my $ch = substr($text, $pos, 1);
        if ($ch eq '[')  { $depth++ }
        elsif ($ch eq ']') { $depth--; return $pos if $depth == 0 }
        $pos++;
    }
    return undef;
}

sub _find_matching_paren {
    my ($text, $start) = @_;
    my $len = length($text);
    my $depth = 1;
    my $pos = $start;
    my $in_angle = 0;
    while ($pos < $len) {
        my $ch = substr($text, $pos, 1);
        if ($ch eq '<') { $in_angle = 1 }
        elsif ($ch eq '>') { $in_angle = 0 }
        elsif (!$in_angle) {
            if ($ch eq '(')  { $depth++ }
            elsif ($ch eq ')') { $depth--; return $pos if $depth == 0 }
        }
        $pos++;
    }
    return undef;
}

sub _parse_link_destination {
    my ($raw) = @_;
    $raw =~ s/^\s+|\s+$//g;
    if ($raw =~ /^<([^>]*)>(.*)$/) {
        my ($dest, $rest) = ($1, $2);
        $rest =~ s/^\s+//;
        return ($dest, _parse_title($rest));
    }
    if ($raw =~ /^(\S+)(.*)$/) {
        my ($dest, $rest) = ($1, $2);
        $rest =~ s/^\s+//;
        return ($dest, $rest ne '' ? _parse_title($rest) : undef);
    }
    return ($raw, undef);
}

sub _parse_title {
    my ($raw) = @_;
    return undef if !defined $raw || $raw eq '';
    return $1 if $raw =~ /^"(.*)"$/;
    return $1 if $raw =~ /^'(.*)'$/;
    return $1 if $raw =~ /^\((.*)\)$/;
    return undef;
}

sub _try_parse_link_or_image {
    my ($text, $bracket_pos, $is_image) = @_;
    my $len = length($text);
    return undef if $bracket_pos >= $len;
    return undef if substr($text, $bracket_pos, 1) ne '[';
    my $close_bracket = _find_matching_bracket($text, $bracket_pos + 1);
    return undef unless defined $close_bracket;
    return undef unless $close_bracket + 1 < $len;
    return undef unless substr($text, $close_bracket + 1, 1) eq '(';
    my $close_paren = _find_matching_paren($text, $close_bracket + 2);
    return undef unless defined $close_paren;
    my $link_text = substr($text, $bracket_pos + 1, $close_bracket - $bracket_pos - 1);
    my $link_dest = substr($text, $close_bracket + 2, $close_paren - $close_bracket - 2);
    my ($dest, $title) = _parse_link_destination($link_dest);
    my $end_pos = $close_paren + 1;
    $end_pos++ if $is_image;
    my $node;
    if ($is_image) {
        $node = {type => 'image', destination => $dest, title => $title, alt => $link_text};
    } else {
        my $children = _parse_inline($link_text);
        $node = {type => 'link', destination => $dest, title => $title, children => $children};
    }
    return {node => $node, end => $end_pos};
}

# _parse_inline($text) → \@inline_nodes
#
# Parses inline Markdown text into inline AST nodes. Extends the CommonMark
# inline parser with:
#
#   ~~text~~      → strikethrough
#   <url>         → autolink (URL)
#   <email@foo>   → autolink (email)
sub _parse_inline {
    my ($text) = @_;
    return [] if !defined $text || $text eq '';

    my @nodes;
    my $pos = 0;
    my $len = length($text);
    my $buf = '';

    while ($pos < $len) {
        my $ch = substr($text, $pos, 1);

        # ── Backslash escape ──────────────────────────────────────────────
        if ($ch eq '\\' && $pos + 1 < $len) {
            my $next = substr($text, $pos + 1, 1);
            if ($next =~ /[!"#\$%&'()*+,\-.\/:<=>?@\[\\\]^_`{|}~]/) {
                $buf .= $next;
                $pos += 2;
                next;
            }
        }

        # ── Newline (hard or soft break) ──────────────────────────────────
        if ($ch eq "\n") {
            if ($buf =~ s/  +$//) {
                push @nodes, {type => 'text', value => $buf} if $buf ne '';
                push @nodes, {type => 'hard_break'};
                $buf = '';
            } else {
                $buf =~ s/\s+$//;
                push @nodes, {type => 'text', value => $buf} if $buf ne '';
                push @nodes, {type => 'soft_break'};
                $buf = '';
            }
            $pos++;
            while ($pos < $len && substr($text, $pos, 1) eq ' ') { $pos++ }
            next;
        }

        # ── Inline code span ──────────────────────────────────────────────
        if ($ch eq '`') {
            my $tick_start = $pos;
            my $ticks = 0;
            while ($pos < $len && substr($text, $pos, 1) eq '`') { $ticks++; $pos++ }
            my $close_pos = _find_code_span_close($text, $pos, $ticks);
            if (defined $close_pos) {
                push @nodes, {type => 'text', value => $buf} if $buf ne '';
                $buf = '';
                my $code_content = substr($text, $pos, $close_pos - $pos);
                if ($code_content =~ /^ (.+) $/ && $code_content !~ /^ +$/) {
                    $code_content = $1;
                }
                push @nodes, {type => 'code_span', value => $code_content};
                $pos = $close_pos + $ticks;
            } else {
                $buf .= substr($text, $tick_start, $ticks);
            }
            next;
        }

        # ── GFM Autolink: <url> or <email> ───────────────────────────────
        if ($ch eq '<') {
            my $close = index($text, '>', $pos + 1);
            if ($close > $pos + 1) {
                my $inner = substr($text, $pos + 1, $close - $pos - 1);
                # Email autolink: contains @ but no spaces
                if ($inner =~ /^[^@\s]+@[^@\s]+\.[^@\s]+$/) {
                    push @nodes, {type => 'text', value => $buf} if $buf ne '';
                    $buf = '';
                    push @nodes, {type => 'autolink', destination => $inner, is_email => 1};
                    $pos = $close + 1;
                    next;
                }
                # URL autolink: has a scheme
                if ($inner =~ /^[a-zA-Z][a-zA-Z0-9+\-.]*:[^\s<>]*$/) {
                    push @nodes, {type => 'text', value => $buf} if $buf ne '';
                    $buf = '';
                    push @nodes, {type => 'autolink', destination => $inner, is_email => 0};
                    $pos = $close + 1;
                    next;
                }
            }
        }

        # ── Image ─────────────────────────────────────────────────────────
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

        # ── Link ──────────────────────────────────────────────────────────
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

        # ── GFM Strikethrough: ~~text~~ ───────────────────────────────────
        if ($ch eq '~' && $pos + 1 < $len && substr($text, $pos + 1, 1) eq '~') {
            my $search_start = $pos + 2;
            my $close = index($text, '~~', $search_start);
            if ($close >= $search_start) {
                push @nodes, {type => 'text', value => $buf} if $buf ne '';
                $buf = '';
                my $inner_text = substr($text, $pos + 2, $close - $pos - 2);
                my $inner = _parse_inline($inner_text);
                push @nodes, {type => 'strikethrough', children => $inner};
                $pos = $close + 2;
                next;
            }
        }

        # ── Bold: ** or __ ────────────────────────────────────────────────
        if (($ch eq '*' || $ch eq '_') && $pos + 1 < $len && substr($text, $pos + 1, 1) eq $ch) {
            my $delim = $ch x 2;
            my $search_start = $pos + 2;
            # Don't match if immediately followed by whitespace
            if ($search_start < $len && substr($text, $search_start, 1) !~ /\s/) {
                my $close = _find_delimiter_close($text, $search_start, $delim);
                if (defined $close) {
                    push @nodes, {type => 'text', value => $buf} if $buf ne '';
                    $buf = '';
                    my $inner_text = substr($text, $search_start, $close - $search_start);
                    my $inner = _parse_inline($inner_text);
                    push @nodes, {type => 'strong', children => $inner};
                    $pos = $close + 2;
                    next;
                }
            }
        }

        # ── Italic: * or _ ────────────────────────────────────────────────
        if ($ch eq '*' || $ch eq '_') {
            my $delim = $ch;
            my $search_start = $pos + 1;
            # Don't match if immediately followed by whitespace
            if ($search_start < $len && substr($text, $search_start, 1) !~ /\s/) {
                my $close = _find_delimiter_close($text, $search_start, $delim);
                if (defined $close) {
                    push @nodes, {type => 'text', value => $buf} if $buf ne '';
                    $buf = '';
                    my $inner_text = substr($text, $search_start, $close - $search_start);
                    my $inner = _parse_inline($inner_text);
                    push @nodes, {type => 'emphasis', children => $inner};
                    $pos = $close + 1;
                    next;
                }
            }
        }

        # ── Default: accumulate ───────────────────────────────────────────
        $buf .= $ch;
        $pos++;
    }

    push @nodes, {type => 'text', value => $buf} if $buf ne '';
    return \@nodes;
}

# ─── Block Finalization ───────────────────────────────────────────────────────

# _finalize_blocks(\@blocks) → \@blocks
#
# Phase 2: Walk the block tree and parse inline content in heading and
# paragraph nodes. Also finalizes table rows' raw cell strings.
sub _finalize_blocks {
    my ($blocks) = @_;
    my @result;
    for my $block (@$blocks) {
        my $t = $block->{type};

        if ($t eq 'heading' || $t eq 'paragraph') {
            my $raw      = $block->{_raw} // '';
            my $children = _parse_inline($raw);
            push @result, {
                type     => $t,
                ($t eq 'heading' ? (level => $block->{level}) : ()),
                children => $children,
            };

        } elsif ($t eq 'blockquote') {
            push @result, {type => 'blockquote', children => _finalize_blocks($block->{children})};

        } elsif ($t eq 'list') {
            my @items;
            for my $item (@{$block->{children}}) {
                my $inner = _finalize_blocks($item->{children});
                if ($item->{type} eq 'task_item') {
                    push @items, {type => 'task_item', checked => $item->{checked}, children => $inner};
                } else {
                    push @items, {type => 'list_item', children => $inner};
                }
            }
            push @result, {
                type     => 'list',
                ordered  => $block->{ordered},
                start    => $block->{start},
                tight    => $block->{tight},
                children => \@items,
            };

        } elsif ($t eq 'table') {
            # Finalize table rows
            my @rows;
            for my $row (@{$block->{_rows}}) {
                my @cells;
                for my $cell_raw (@{$row->{_raw_cells}}) {
                    $cell_raw =~ s/^\s+|\s+$//g;
                    my $children = _parse_inline($cell_raw);
                    push @cells, {type => 'table_cell', children => $children};
                }
                push @rows, {
                    type      => 'table_row',
                    is_header => $row->{is_header},
                    children  => \@cells,
                };
            }
            push @result, {type => 'table', children => \@rows};

        } else {
            push @result, $block;
        }
    }
    return \@result;
}

# ─── Public API ───────────────────────────────────────────────────────────────

# parse($gfm_string) → document AST hashref
#
# Parses a GFM Markdown string and returns a document AST node.
#
# The result is: {type => "document", children => [...]}
#
# Example:
#
#   use CodingAdventures::GfmParser;
#
#   my $doc = CodingAdventures::GfmParser::parse("# Hi\n\n- [ ] task\n- [x] done\n");
#   # $doc->{children}[0]{type} eq "heading"
#   # $doc->{children}[1]{type} eq "list"
#   # $doc->{children}[1]{children}[0]{type} eq "task_item"
#   # $doc->{children}[1]{children}[0]{checked} == 0
#   # $doc->{children}[1]{children}[1]{checked} == 1
sub parse {
    my ($gfm) = @_;
    $gfm //= '';
    $gfm =~ s/\r\n/\n/g;
    $gfm =~ s/\r/\n/g;
    my @lines = split(/\n/, $gfm, -1);
    pop @lines if @lines && $lines[-1] eq '';
    my $blocks = _parse_blocks(\@lines);
    my $final  = _finalize_blocks($blocks);
    return {type => 'document', children => $final};
}

1;

__END__

=head1 NAME

CodingAdventures::GfmParser - Pure-Perl GitHub Flavored Markdown parser

=head1 SYNOPSIS

    use CodingAdventures::GfmParser;

    my $doc = CodingAdventures::GfmParser::parse("# Hello\n\n~~bye~~\n");

=head1 DESCRIPTION

Parses GitHub Flavored Markdown into a Document AST. Extends CommonMark
with tables, task list items, strikethrough, and autolinks.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
