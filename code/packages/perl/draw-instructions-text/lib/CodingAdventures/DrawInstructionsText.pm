package CodingAdventures::DrawInstructionsText;

# ============================================================================
# CodingAdventures::DrawInstructionsText
#
# ASCII/Unicode text renderer for the draw-instructions scene model.
#
# This renderer proves the draw-instructions abstraction is truly backend-
# neutral: the same DrawScene that produces SVG can also render as box-drawing
# characters in a terminal.
#
# HOW IT WORKS
# ------------
# The renderer maps pixel-coordinate scenes to a fixed-width character grid.
# Each cell in the grid is one character.  The mapping uses a configurable
# scale factor (default: 8px per char width, 16px per char height).
#
#   Scene coordinates (pixels)     Character grid
#   +---------------------+        +----------+
#   | rect at (0,0,200,32)|   ->   |##########|
#   |                     |        |##########|
#   +---------------------+        +----------+
#
# CHARACTER PALETTE
# -----------------
# Box-drawing characters create clean table grids:
#
#   Corners:  ┌ ┐ └ ┘
#   Edges:    ─ │
#   Tees:     ┬ ┴ ├ ┤
#   Cross:    ┼
#   Fill:     █
#
# INTERSECTION LOGIC
# ------------------
# When two drawing operations overlap at the same cell, the renderer merges
# them into the correct junction character.  A horizontal line crossing a
# vertical line becomes ┼.  A line meeting a box corner becomes the
# appropriate tee (┬ ┴ ├ ┤).
#
# This is tracked via a "tag" buffer parallel to the character buffer.
# Each cell records which directions have lines passing through it
# (up, down, left, right), and the tag is resolved to the correct
# box-drawing character on each write.
#
# DIRECTION BITMASK
# -----------------
#
#        UP (1)
#         |
# LEFT(8)-+-RIGHT(2)
#         |
#       DOWN(4)
#
# The FILL flag (16) marks cells filled with block characters.
# The TEXT flag (32) marks cells containing text characters.
#
# USAGE
# -----
#
#   use CodingAdventures::DrawInstructions;
#   use CodingAdventures::DrawInstructionsText;
#
#   my $scene = CodingAdventures::DrawInstructions::create_scene(160, 48, [
#       CodingAdventures::DrawInstructions::draw_rect(
#           0, 0, 160, 48, "transparent",
#           stroke => "#000", stroke_width => 1),
#       CodingAdventures::DrawInstructions::draw_line(0, 16, 160, 16, "#000"),
#       CodingAdventures::DrawInstructions::draw_text(8, 8, "Hello"),
#   ], "#fff");
#
#   my $text = CodingAdventures::DrawInstructionsText::render_text($scene);
#   print "$text\n";
#
# ============================================================================

use strict;
use warnings;
use utf8;
use POSIX qw(ceil);

use CodingAdventures::DrawInstructions;

our $VERSION = '0.01';

# ============================================================================
# Direction bitmask constants
# ============================================================================
#
# Each cell in the tag buffer stores a bitmask of these flags.  When
# multiple drawing operations overlap, we OR the flags together and
# resolve the combined tag to the correct box-drawing character.

use constant UP    => 1;
use constant RIGHT => 2;
use constant DOWN  => 4;
use constant LEFT  => 8;
use constant FILL  => 16;
use constant TEXT  => 32;

# ============================================================================
# Box-drawing character lookup table
# ============================================================================
#
# Given a bitmask of directions (UP | DOWN | LEFT | RIGHT), return the
# correct Unicode box-drawing character.  This hash covers all 16
# combinations of the 4 direction bits.
#
#   Bitmask        | Char | Meaning
#   ---------------+------+------------------
#   LEFT|RIGHT     |  ─   | horizontal line
#   UP|DOWN        |  │   | vertical line
#   DOWN|RIGHT     |  ┌   | top-left corner
#   DOWN|LEFT      |  ┐   | top-right corner
#   UP|RIGHT       |  └   | bottom-left corner
#   UP|LEFT        |  ┘   | bottom-right corner
#   L|R|DOWN       |  ┬   | top tee
#   L|R|UP         |  ┴   | bottom tee
#   U|D|RIGHT      |  ├   | left tee
#   U|D|LEFT       |  ┤   | right tee
#   U|D|L|R        |  ┼   | cross

my %BOX_CHARS = (
    (LEFT | RIGHT)                => "\x{2500}",   # ─
    (UP | DOWN)                   => "\x{2502}",   # │
    (DOWN | RIGHT)                => "\x{250C}",   # ┌
    (DOWN | LEFT)                 => "\x{2510}",   # ┐
    (UP | RIGHT)                  => "\x{2514}",   # └
    (UP | LEFT)                   => "\x{2518}",   # ┘
    (LEFT | RIGHT | DOWN)         => "\x{252C}",   # ┬
    (LEFT | RIGHT | UP)           => "\x{2534}",   # ┴
    (UP | DOWN | RIGHT)           => "\x{251C}",   # ├
    (UP | DOWN | LEFT)            => "\x{2524}",   # ┤
    (UP | DOWN | LEFT | RIGHT)    => "\x{253C}",   # ┼
    (RIGHT)                       => "\x{2500}",   # ─  (half-line defaults to full)
    (LEFT)                        => "\x{2500}",   # ─
    (UP)                          => "\x{2502}",   # │
    (DOWN)                        => "\x{2502}",   # │
);

# ============================================================================
# resolve_box_char($tag)
# ============================================================================
#
# Resolves a direction bitmask to a box-drawing character.
# Falls back to "+" if the combination is not in our table (should not happen).

sub resolve_box_char {
    my ($tag) = @_;
    return "\x{2588}" if $tag & FILL;    # █ for filled cells
    return ""          if $tag & TEXT;    # text chars stored directly
    my $dir = $tag & (UP | DOWN | LEFT | RIGHT);
    return $BOX_CHARS{$dir} // "+";
}

# ============================================================================
# Buffer operations
# ============================================================================
#
# The buffer is a hashref with keys:
#   rows  — number of rows
#   cols  — number of columns
#   chars — 2D arrayref of characters (chars->[row][col])
#   tags  — 2D arrayref of bitmask integers (tags->[row][col])
#
# new_buffer($rows, $cols)
# Creates a fresh buffer filled with spaces and zero tags.

sub new_buffer {
    my ($rows, $cols) = @_;
    my @chars;
    my @tags;
    for my $r (0 .. $rows - 1) {
        $chars[$r] = [ (' ') x $cols ];
        $tags[$r]  = [ (0) x $cols ];
    }
    return {
        rows  => $rows,
        cols  => $cols,
        chars => \@chars,
        tags  => \@tags,
    };
}

# write_tag($buf, $row, $col, $dir_flags, $clip)
#
# Writes a box-drawing element at ($row, $col) by adding direction flags.
# The actual character is resolved from the combined tag.  Respects clip
# bounds.  Does not overwrite text cells.

sub write_tag {
    my ($buf, $row, $col, $dir_flags, $clip) = @_;
    return if $row < $clip->{min_row} || $row >= $clip->{max_row};
    return if $col < $clip->{min_col} || $col >= $clip->{max_col};
    return if $row < 0 || $row >= $buf->{rows};
    return if $col < 0 || $col >= $buf->{cols};

    my $existing = $buf->{tags}[$row][$col];

    # Don't overwrite text with box-drawing
    return if $existing & TEXT;

    my $merged = $existing | $dir_flags;
    $buf->{tags}[$row][$col] = $merged;
    $buf->{chars}[$row][$col] = ($dir_flags & FILL)
        ? "\x{2588}"
        : resolve_box_char($merged);
}

# write_char($buf, $row, $col, $ch, $clip)
#
# Writes a text character directly at ($row, $col).
# Text overwrites any existing content.

sub write_char {
    my ($buf, $row, $col, $ch, $clip) = @_;
    return if $row < $clip->{min_row} || $row >= $clip->{max_row};
    return if $col < $clip->{min_col} || $col >= $clip->{max_col};
    return if $row < 0 || $row >= $buf->{rows};
    return if $col < 0 || $col >= $buf->{cols};

    $buf->{chars}[$row][$col] = $ch;
    $buf->{tags}[$row][$col]  = TEXT;
}

# buffer_to_string($buf)
#
# Joins all rows, trims trailing whitespace per line, joins with newlines,
# and trims trailing blank lines.

sub buffer_to_string {
    my ($buf) = @_;
    my @lines;
    for my $r (0 .. $buf->{rows} - 1) {
        my $line = join('', @{$buf->{chars}[$r]});
        $line =~ s/\s+$//;      # trim trailing whitespace
        push @lines, $line;
    }
    my $result = join("\n", @lines);
    $result =~ s/[\s\n]+$//;    # trim trailing blank lines
    return $result;
}

# ============================================================================
# Coordinate mapping
# ============================================================================
#
# Scene coordinates are in pixels; the buffer is in character cells.
# We round to the nearest cell with sprintf to handle non-integer boundaries.

sub to_col {
    my ($x, $sx) = @_;
    return sprintf("%.0f", $x / $sx) + 0;
}

sub to_row {
    my ($y, $sy) = @_;
    return sprintf("%.0f", $y / $sy) + 0;
}

# ============================================================================
# render_rect($inst, $buf, $sx, $sy, $clip)
# ============================================================================
#
# Stroked rects produce box-drawing outlines: corners at the four vertices,
# horizontal edges along the top and bottom, vertical edges along left and
# right.
#
# Filled rects (non-transparent, non-"none" fill with no stroke) produce
# solid block characters covering the entire rect area.
#
# Transparent rects with no stroke produce nothing (they are invisible).

sub render_rect {
    my ($inst, $buf, $sx, $sy, $clip) = @_;

    my $c1 = to_col($inst->{x}, $sx);
    my $r1 = to_row($inst->{y}, $sy);
    my $c2 = to_col($inst->{x} + $inst->{width}, $sx);
    my $r2 = to_row($inst->{y} + $inst->{height}, $sy);

    my $has_stroke = defined($inst->{stroke}) && $inst->{stroke} ne '';
    my $has_fill   = $inst->{fill} ne '' &&
                     $inst->{fill} ne 'transparent' &&
                     $inst->{fill} ne 'none';

    if ($has_stroke) {
        # Corners
        write_tag($buf, $r1, $c1, DOWN | RIGHT, $clip);
        write_tag($buf, $r1, $c2, DOWN | LEFT, $clip);
        write_tag($buf, $r2, $c1, UP | RIGHT, $clip);
        write_tag($buf, $r2, $c2, UP | LEFT, $clip);

        # Top and bottom edges
        for my $c ($c1 + 1 .. $c2 - 1) {
            write_tag($buf, $r1, $c, LEFT | RIGHT, $clip);
            write_tag($buf, $r2, $c, LEFT | RIGHT, $clip);
        }

        # Left and right edges
        for my $r ($r1 + 1 .. $r2 - 1) {
            write_tag($buf, $r, $c1, UP | DOWN, $clip);
            write_tag($buf, $r, $c2, UP | DOWN, $clip);
        }
    } elsif ($has_fill) {
        # Fill the interior with block characters
        for my $r ($r1 .. $r2) {
            for my $c ($c1 .. $c2) {
                write_tag($buf, $r, $c, FILL, $clip);
            }
        }
    }
    # else: transparent rect with no stroke — invisible
}

# ============================================================================
# render_line($inst, $buf, $sx, $sy, $clip)
# ============================================================================
#
# Lines can be horizontal, vertical, or diagonal.
#
# Endpoint-aware direction flags: at endpoints, only the inward direction
# is set.  This way a line endpoint meeting a perpendicular box edge
# resolves to the correct tee character instead of a cross.
#
# For example, a horizontal line's left endpoint gets only the RIGHT flag,
# so when it merges with a vertical box edge (UP|DOWN), the result is
# UP|DOWN|RIGHT which resolves to the left-tee character ├.

sub render_line {
    my ($inst, $buf, $sx, $sy, $clip) = @_;

    my $c1 = to_col($inst->{x1}, $sx);
    my $r1 = to_row($inst->{y1}, $sy);
    my $c2 = to_col($inst->{x2}, $sx);
    my $r2 = to_row($inst->{y2}, $sy);

    if ($r1 == $r2) {
        # Horizontal line
        my $min_c = $c1 < $c2 ? $c1 : $c2;
        my $max_c = $c1 > $c2 ? $c1 : $c2;

        for my $c ($min_c .. $max_c) {
            my $flags = 0;
            if ($min_c == $max_c) {
                # Single-cell line
                $flags = LEFT | RIGHT;
            } elsif ($c == $min_c) {
                # Left endpoint: only points right (inward)
                $flags = RIGHT;
            } elsif ($c == $max_c) {
                # Right endpoint: only points left (inward)
                $flags = LEFT;
            } else {
                # Interior: both directions
                $flags = LEFT | RIGHT;
            }
            write_tag($buf, $r1, $c, $flags, $clip);
        }
    } elsif ($c1 == $c2) {
        # Vertical line
        my $min_r = $r1 < $r2 ? $r1 : $r2;
        my $max_r = $r1 > $r2 ? $r1 : $r2;

        for my $r ($min_r .. $max_r) {
            my $flags = 0;
            if ($min_r == $max_r) {
                # Single-cell line
                $flags = UP | DOWN;
            } elsif ($r == $min_r) {
                # Top endpoint: only points down (inward)
                $flags = DOWN;
            } elsif ($r == $max_r) {
                # Bottom endpoint: only points up (inward)
                $flags = UP;
            } else {
                # Interior: both directions
                $flags = UP | DOWN;
            }
            write_tag($buf, $r, $c1, $flags, $clip);
        }
    } else {
        # Diagonal: approximate with Bresenham's algorithm
        my $dr = abs($r2 - $r1);
        my $dc = abs($c2 - $c1);
        my $sr = $r1 < $r2 ? 1 : -1;
        my $sc = $c1 < $c2 ? 1 : -1;
        my $err = $dc - $dr;
        my $r = $r1;
        my $c = $c1;
        my $dom_flags = $dc > $dr ? (LEFT | RIGHT) : (UP | DOWN);

        while (1) {
            write_tag($buf, $r, $c, $dom_flags, $clip);
            last if $r == $r2 && $c == $c2;
            my $e2 = 2 * $err;
            if ($e2 > -$dr) { $err -= $dr; $c += $sc; }
            if ($e2 < $dc)  { $err += $dc; $r += $sr; }
        }
    }
}

# ============================================================================
# render_text_inst($inst, $buf, $sx, $sy, $clip)
# ============================================================================
#
# Text is placed directly into the character buffer, overwriting any
# existing content.  The align field controls where the text anchor is:
#
#   "start"  — text starts at the x coordinate
#   "middle" — text is centered on the x coordinate
#   "end"    — text ends at the x coordinate

sub render_text_inst {
    my ($inst, $buf, $sx, $sy, $clip) = @_;

    my $row  = to_row($inst->{y}, $sy);
    my $text = $inst->{value};
    my $len  = length($text);
    my $align = $inst->{align} // 'middle';

    my $start_col;
    if ($align eq 'middle') {
        $start_col = to_col($inst->{x}, $sx) - int($len / 2);
    } elsif ($align eq 'end') {
        $start_col = to_col($inst->{x}, $sx) - $len;
    } else {
        # "start" or default
        $start_col = to_col($inst->{x}, $sx);
    }

    for my $i (0 .. $len - 1) {
        write_char($buf, $row, $start_col + $i, substr($text, $i, 1), $clip);
    }
}

# ============================================================================
# render_group($inst, $buf, $sx, $sy, $clip)
# ============================================================================
#
# Groups simply recurse into all children.

sub render_group {
    my ($inst, $buf, $sx, $sy, $clip) = @_;
    for my $child (@{$inst->{children}}) {
        render_instruction($child, $buf, $sx, $sy, $clip);
    }
}

# ============================================================================
# render_clip($inst, $buf, $sx, $sy, $parent_clip)
# ============================================================================
#
# A clip instruction constrains its children to a rectangular region.
# We compute the intersection of the new clip bounds with the parent
# clip bounds, then render children with the tighter clip.

sub render_clip {
    my ($inst, $buf, $sx, $sy, $parent_clip) = @_;

    my $new_min_col = to_col($inst->{x}, $sx);
    my $new_min_row = to_row($inst->{y}, $sy);
    my $new_max_col = to_col($inst->{x} + $inst->{width}, $sx);
    my $new_max_row = to_row($inst->{y} + $inst->{height}, $sy);

    # Intersect with parent clip
    my $clip = {
        min_col => ($parent_clip->{min_col} > $new_min_col) ? $parent_clip->{min_col} : $new_min_col,
        min_row => ($parent_clip->{min_row} > $new_min_row) ? $parent_clip->{min_row} : $new_min_row,
        max_col => ($parent_clip->{max_col} < $new_max_col) ? $parent_clip->{max_col} : $new_max_col,
        max_row => ($parent_clip->{max_row} < $new_max_row) ? $parent_clip->{max_row} : $new_max_row,
    };

    for my $child (@{$inst->{children}}) {
        render_instruction($child, $buf, $sx, $sy, $clip);
    }
}

# ============================================================================
# render_instruction($inst, $buf, $sx, $sy, $clip)
# ============================================================================
#
# Dispatches a single instruction hashref to the correct renderer based
# on the 'kind' field.
#
# Dispatch table:
#
#   Kind     | Function
#   ---------+---------------------
#   rect     | render_rect()
#   line     | render_line()
#   text     | render_text_inst()
#   group    | render_group()
#   clip     | render_clip()

sub render_instruction {
    my ($inst, $buf, $sx, $sy, $clip) = @_;
    my $kind = $inst->{kind};

    if ($kind eq 'rect') {
        render_rect($inst, $buf, $sx, $sy, $clip);
    } elsif ($kind eq 'line') {
        render_line($inst, $buf, $sx, $sy, $clip);
    } elsif ($kind eq 'text') {
        render_text_inst($inst, $buf, $sx, $sy, $clip);
    } elsif ($kind eq 'group') {
        render_group($inst, $buf, $sx, $sy, $clip);
    } elsif ($kind eq 'clip') {
        render_clip($inst, $buf, $sx, $sy, $clip);
    } else {
        die "Unknown draw instruction kind: $kind";
    }
}

# ============================================================================
# render_text($scene [, %opts])
# ============================================================================
#
# The main entry point.  Takes a scene hashref and returns a Unicode
# box-drawing text string.
#
# Options:
#   scale_x — pixels per character column (default: 8)
#   scale_y — pixels per character row (default: 16)
#
# Parameters:
#   $scene — a scene hashref as returned by create_scene()
#   %opts  — optional named parameters
#
# Returns:
#   A string of Unicode box-drawing characters.

sub render_text {
    my ($scene, %opts) = @_;

    my $sx = $opts{scale_x} // 8;
    my $sy = $opts{scale_y} // 16;

    my $cols = ceil($scene->{width} / $sx);
    my $rows = ceil($scene->{height} / $sy);

    # Early exit for zero-sized scenes
    return '' if $cols <= 0 || $rows <= 0;

    my $buf = new_buffer($rows, $cols);

    my $full_clip = {
        min_col => 0,
        min_row => 0,
        max_col => $cols,
        max_row => $rows,
    };

    for my $inst (@{$scene->{instructions}}) {
        render_instruction($inst, $buf, $sx, $sy, $full_clip);
    }

    return buffer_to_string($buf);
}

1;

__END__

=head1 NAME

CodingAdventures::DrawInstructionsText - ASCII/Unicode text renderer for draw instructions

=head1 SYNOPSIS

    use CodingAdventures::DrawInstructions;
    use CodingAdventures::DrawInstructionsText;

    my $scene = CodingAdventures::DrawInstructions::create_scene(160, 48, [
        CodingAdventures::DrawInstructions::draw_rect(
            0, 0, 160, 48, "transparent",
            stroke => "#000", stroke_width => 1),
        CodingAdventures::DrawInstructions::draw_line(0, 16, 160, 16, "#000"),
        CodingAdventures::DrawInstructions::draw_text(8, 8, "Hello"),
    ], "#fff");

    my $text = CodingAdventures::DrawInstructionsText::render_text($scene);
    print "$text\n";

    # With custom scale:
    my $text2 = CodingAdventures::DrawInstructionsText::render_text(
        $scene, scale_x => 4, scale_y => 8);

=head1 DESCRIPTION

Converts a draw instruction scene into a Unicode box-drawing character string.
Handles all instruction kinds: rect, text, line, clip, and group.

Stroked rectangles become box-drawing outlines.  Filled rectangles become
solid block characters.  Lines use endpoint-aware direction flags for correct
junction merging.  Text supports start/middle/end alignment.  Clips constrain
children to rectangular regions.

=head1 FUNCTIONS

=over 4

=item render_text($scene [, %opts])

Takes a scene hashref and returns a Unicode text string.
Options: scale_x (default 8), scale_y (default 16).

=back

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
