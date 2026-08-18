package CodingAdventures::PaintVmAscii;

use strict;
use warnings;
use utf8;

use CodingAdventures::PaintInstructions;

our $VERSION = '0.2.0';

# Cell flags for box-drawing character merging (bitwise, mirrors the
# csharp/fsharp CellFlags enum so intersecting strokes merge into the
# right box-drawing glyph regardless of draw order).
use constant {
    FLAG_NONE  => 0,
    FLAG_UP    => 1,
    FLAG_RIGHT => 2,
    FLAG_DOWN  => 4,
    FLAG_LEFT  => 8,
    FLAG_FILL  => 16,
    FLAG_TEXT  => 32,
};

my %BOX_CHARACTERS = (
    (FLAG_LEFT | FLAG_RIGHT)                      => "\x{2500}",
    (FLAG_UP | FLAG_DOWN)                         => "\x{2502}",
    (FLAG_DOWN | FLAG_RIGHT)                      => "\x{250C}",
    (FLAG_DOWN | FLAG_LEFT)                       => "\x{2510}",
    (FLAG_UP | FLAG_RIGHT)                        => "\x{2514}",
    (FLAG_UP | FLAG_LEFT)                         => "\x{2518}",
    (FLAG_LEFT | FLAG_RIGHT | FLAG_DOWN)           => "\x{252C}",
    (FLAG_LEFT | FLAG_RIGHT | FLAG_UP)             => "\x{2534}",
    (FLAG_UP | FLAG_DOWN | FLAG_RIGHT)             => "\x{251C}",
    (FLAG_UP | FLAG_DOWN | FLAG_LEFT)              => "\x{2524}",
    (FLAG_UP | FLAG_DOWN | FLAG_LEFT | FLAG_RIGHT) => "\x{253C}",
    FLAG_RIGHT()                                   => "\x{2500}",
    FLAG_LEFT()                                    => "\x{2500}",
    FLAG_UP()                                      => "\x{2502}",
    FLAG_DOWN()                                    => "\x{2502}",
);

my %UNSAFE_SINGLE_CODE_POINTS = map { $_ => 1 } (0x200e, 0x200f, 0x061c);

sub _scale_x {
    my ($options) = @_;
    return 8 if !defined $options || !defined $options->{scale_x} || !$options->{scale_x};
    return $options->{scale_x};
}

sub _scale_y {
    my ($options) = @_;
    return 16 if !defined $options || !defined $options->{scale_y} || !$options->{scale_y};
    return $options->{scale_y};
}

# Rounds half away from zero, matching every other language port's rounding
# convention for scene-to-cell coordinate conversion (e.g. C#'s
# Math.Round(..., MidpointRounding.AwayFromZero)). The prior sprintf('%.0f', ...)
# approach was libc-dependent (banker's rounding on some platforms).
sub _round_half_away_from_zero {
    my ($value) = @_;
    return $value >= 0 ? int($value + 0.5) : -int(-$value + 0.5);
}

sub _to_col {
    my ($x, $scale_x) = @_;
    return _round_half_away_from_zero($x / $scale_x);
}

sub _to_row {
    my ($y, $scale_y) = @_;
    return _round_half_away_from_zero($y / $scale_y);
}

sub _new_buffer {
    my ($rows, $cols) = @_;
    my (@chars, @tags);
    for my $row (0 .. $rows - 1) {
        $chars[$row] = [ (' ') x $cols ];
        $tags[$row]  = [ (FLAG_NONE) x $cols ];
    }
    return {
        rows  => $rows,
        cols  => $cols,
        chars => \@chars,
        tags  => \@tags,
    };
}

sub _inside_buffer {
    my ($buffer, $row, $col) = @_;
    return $row >= 0 && $row < $buffer->{rows} && $col >= 0 && $col < $buffer->{cols};
}

sub _inside_clip {
    my ($row, $col, $clip) = @_;
    return $row >= $clip->{min_row}
        && $row < $clip->{max_row}
        && $col >= $clip->{min_col}
        && $col < $clip->{max_col};
}

sub _resolve_cell {
    my ($flags) = @_;
    my $directions = $flags & (FLAG_UP | FLAG_RIGHT | FLAG_DOWN | FLAG_LEFT);
    if ($directions != FLAG_NONE && exists $BOX_CHARACTERS{$directions}) {
        return $BOX_CHARACTERS{$directions};
    }
    return "\x{2588}" if $flags & FLAG_FILL;
    return '+';
}

# Merges a directional/fill tag into a cell (used by rect/line so
# intersecting strokes combine into the correct box-drawing glyph). A cell
# that already holds literal text (from a glyph_run) is never overwritten —
# text takes priority over box-drawing and fill, matching P2D02's rendering
# rules.
sub _write_tag {
    my ($buffer, $row, $col, $flags, $clip) = @_;
    return unless _inside_clip($row, $col, $clip) && _inside_buffer($buffer, $row, $col);

    my $existing = $buffer->{tags}[$row][$col];
    return if $existing & FLAG_TEXT;

    my $merged = $existing | $flags;
    $buffer->{tags}[$row][$col]  = $merged;
    $buffer->{chars}[$row][$col] = _resolve_cell($merged);
}

# Writes a literal glyph into a cell, unconditionally overwriting whatever
# box-drawing/fill was there before (text priority, per P2D02).
sub _write_char {
    my ($buffer, $row, $col, $value, $clip) = @_;
    return unless _inside_clip($row, $col, $clip) && _inside_buffer($buffer, $row, $col);

    $buffer->{chars}[$row][$col] = $value;
    $buffer->{tags}[$row][$col]  = FLAG_TEXT;
}

sub _buffer_to_string {
    my ($buffer) = @_;
    my @lines;
    for my $row (0 .. $buffer->{rows} - 1) {
        my $line = join('', @{ $buffer->{chars}[$row] });
        $line =~ s/\s+$//;
        push @lines, $line;
    }

    my $last_content = -1;
    for my $i (0 .. $#lines) {
        $last_content = $i if length($lines[$i]) > 0;
    }
    return '' if $last_content < 0;

    return join("\n", @lines[0 .. $last_content]);
}

sub _handle_rect {
    my ($instruction, $buffer, $scale_x, $scale_y, $clip) = @_;
    my $c1 = _to_col($instruction->{x}, $scale_x);
    my $r1 = _to_row($instruction->{y}, $scale_y);
    my $c2 = _to_col($instruction->{x} + $instruction->{width}, $scale_x);
    my $r2 = _to_row($instruction->{y} + $instruction->{height}, $scale_y);

    my $fill = $instruction->{fill};
    my $has_fill = defined $fill && $fill ne '' && $fill ne 'transparent' && $fill ne 'none';
    my $stroke = $instruction->{stroke};
    my $has_stroke = defined $stroke && $stroke ne '';

    if ($has_fill) {
        for my $row ($r1 .. $r2) {
            for my $col ($c1 .. $c2) {
                _write_tag($buffer, $row, $col, FLAG_FILL, $clip);
            }
        }
    }

    return unless $has_stroke;

    _write_tag($buffer, $r1, $c1, FLAG_DOWN | FLAG_RIGHT, $clip);
    _write_tag($buffer, $r1, $c2, FLAG_DOWN | FLAG_LEFT, $clip);
    _write_tag($buffer, $r2, $c1, FLAG_UP | FLAG_RIGHT, $clip);
    _write_tag($buffer, $r2, $c2, FLAG_UP | FLAG_LEFT, $clip);

    for my $col (($c1 + 1) .. ($c2 - 1)) {
        _write_tag($buffer, $r1, $col, FLAG_LEFT | FLAG_RIGHT, $clip);
        _write_tag($buffer, $r2, $col, FLAG_LEFT | FLAG_RIGHT, $clip);
    }

    for my $row (($r1 + 1) .. ($r2 - 1)) {
        _write_tag($buffer, $row, $c1, FLAG_UP | FLAG_DOWN, $clip);
        _write_tag($buffer, $row, $c2, FLAG_UP | FLAG_DOWN, $clip);
    }
}

sub _handle_line {
    my ($instruction, $buffer, $scale_x, $scale_y, $clip) = @_;
    my $c1 = _to_col($instruction->{x1}, $scale_x);
    my $r1 = _to_row($instruction->{y1}, $scale_y);
    my $c2 = _to_col($instruction->{x2}, $scale_x);
    my $r2 = _to_row($instruction->{y2}, $scale_y);

    if ($r1 == $r2) {
        my $min_col = $c1 < $c2 ? $c1 : $c2;
        my $max_col = $c1 > $c2 ? $c1 : $c2;
        for my $col ($min_col .. $max_col) {
            my $flags = FLAG_NONE;
            $flags |= FLAG_LEFT  if $col > $min_col;
            $flags |= FLAG_RIGHT if $col < $max_col;
            $flags = FLAG_LEFT | FLAG_RIGHT if $col == $min_col && $col == $max_col;
            _write_tag($buffer, $r1, $col, $flags, $clip);
        }
        return;
    }

    if ($c1 == $c2) {
        my $min_row = $r1 < $r2 ? $r1 : $r2;
        my $max_row = $r1 > $r2 ? $r1 : $r2;
        for my $row ($min_row .. $max_row) {
            my $flags = FLAG_NONE;
            $flags |= FLAG_UP   if $row > $min_row;
            $flags |= FLAG_DOWN if $row < $max_row;
            $flags = FLAG_UP | FLAG_DOWN if $row == $min_row && $row == $max_row;
            _write_tag($buffer, $row, $c1, $flags, $clip);
        }
        return;
    }

    # Bresenham's line algorithm for the diagonal case.
    my $delta_row = abs($r2 - $r1);
    my $delta_col = abs($c2 - $c1);
    my $step_row  = $r1 < $r2 ? 1 : -1;
    my $step_col  = $c1 < $c2 ? 1 : -1;
    my $error     = $delta_col - $delta_row;
    my $row_cursor = $r1;
    my $col_cursor = $c1;

    while (1) {
        my $flags = $delta_col > $delta_row ? (FLAG_LEFT | FLAG_RIGHT) : (FLAG_UP | FLAG_DOWN);
        _write_tag($buffer, $row_cursor, $col_cursor, $flags, $clip);

        last if $row_cursor == $r2 && $col_cursor == $c2;

        my $doubled = 2 * $error;
        if ($doubled > -$delta_row) {
            $error -= $delta_row;
            $col_cursor += $step_col;
        }
        if ($doubled < $delta_col) {
            $error += $delta_col;
            $row_cursor += $step_row;
        }
    }
}

# ASCII-backend-specific relaxation of the general PaintGlyphRun contract:
# glyph_id is treated as a literal Unicode scalar value here (no font
# resolution happens in a terminal), per P2D02-paint-vm-ascii.md.
sub _is_safe_terminal_code_point {
    my ($code_point) = @_;
    return 0 if $code_point < 0x20;
    return 0 if $code_point >= 0x7f && $code_point <= 0x9f;
    return 0 if $UNSAFE_SINGLE_CODE_POINTS{$code_point};
    return 0 if $code_point >= 0x202a && $code_point <= 0x202e;
    return 0 if $code_point >= 0x2066 && $code_point <= 0x2069;
    return 1;
}

sub _to_safe_terminal_glyph {
    my ($code_point) = @_;
    return '?' unless _is_safe_terminal_code_point($code_point);
    my $ch = eval { chr($code_point) };
    return defined $ch ? $ch : '?';
}

sub _handle_glyph_run {
    my ($instruction, $buffer, $scale_x, $scale_y, $clip) = @_;
    for my $glyph (@{ $instruction->{glyphs} // [] }) {
        _write_char(
            $buffer,
            _to_row($glyph->{y}, $scale_y),
            _to_col($glyph->{x}, $scale_x),
            _to_safe_terminal_glyph($glyph->{glyph_id}),
            $clip,
        );
    }
}

sub _is_identity_transform {
    my ($transform) = @_;
    return 1 unless defined $transform;
    return ($transform->{a} // 1) == 1
        && ($transform->{b} // 0) == 0
        && ($transform->{c} // 0) == 0
        && ($transform->{d} // 1) == 1
        && ($transform->{e} // 0) == 0
        && ($transform->{f} // 0) == 0;
}

sub _assert_plain_group {
    my ($group) = @_;
    die "paint-vm-ascii: does not support transformed groups\n"
        unless _is_identity_transform($group->{transform});
    die "paint-vm-ascii: does not support group opacity\n"
        if defined $group->{opacity} && $group->{opacity} != 1.0;
}

sub _assert_plain_layer {
    my ($layer) = @_;
    die "paint-vm-ascii: does not support transformed layers\n"
        unless _is_identity_transform($layer->{transform});
    die "paint-vm-ascii: does not support layer opacity\n"
        if defined $layer->{opacity} && $layer->{opacity} != 1.0;
    die "paint-vm-ascii: does not support layer filters\n"
        if defined $layer->{filters} && @{ $layer->{filters} } > 0;
    die "paint-vm-ascii: does not support layer blend modes\n"
        if defined $layer->{blend_mode} && $layer->{blend_mode} ne 'normal';
}

sub _max { return $_[0] > $_[1] ? $_[0] : $_[1]; }
sub _min { return $_[0] < $_[1] ? $_[0] : $_[1]; }

sub _dispatch {
    my ($instruction, $buffer, $scale_x, $scale_y, $clip_stack) = @_;
    my $kind = $instruction->{kind} // '';
    my $clip = $clip_stack->[-1];

    if ($kind eq 'rect') {
        _handle_rect($instruction, $buffer, $scale_x, $scale_y, $clip);
    }
    elsif ($kind eq 'line') {
        _handle_line($instruction, $buffer, $scale_x, $scale_y, $clip);
    }
    elsif ($kind eq 'glyph_run') {
        _handle_glyph_run($instruction, $buffer, $scale_x, $scale_y, $clip);
    }
    elsif ($kind eq 'group') {
        _assert_plain_group($instruction);
        _dispatch($_, $buffer, $scale_x, $scale_y, $clip_stack) for @{ $instruction->{children} // [] };
    }
    elsif ($kind eq 'layer') {
        _assert_plain_layer($instruction);
        _dispatch($_, $buffer, $scale_x, $scale_y, $clip_stack) for @{ $instruction->{children} // [] };
    }
    elsif ($kind eq 'clip') {
        my $parent = $clip_stack->[-1];
        my $next = {
            min_col => _max($parent->{min_col}, _to_col($instruction->{x}, $scale_x)),
            min_row => _max($parent->{min_row}, _to_row($instruction->{y}, $scale_y)),
            max_col => _min($parent->{max_col}, _to_col($instruction->{x} + $instruction->{width}, $scale_x)),
            max_row => _min($parent->{max_row}, _to_row($instruction->{y} + $instruction->{height}, $scale_y)),
        };
        push @$clip_stack, $next;
        _dispatch($_, $buffer, $scale_x, $scale_y, $clip_stack) for @{ $instruction->{children} // [] };
        pop @$clip_stack;
    }
    else {
        die "paint-vm-ascii: unsupported paint instruction kind: $kind\n";
    }
}

sub render {
    my ($class, $scene, $options) = @_;
    my $scale_x = _scale_x($options);
    my $scale_y = _scale_y($options);

    my $cols = int((($scene->{width} // 0) + $scale_x - 1) / $scale_x);
    my $rows = int((($scene->{height} // 0) + $scale_y - 1) / $scale_y);
    my $buffer = _new_buffer($rows, $cols);
    my $clip_stack = [ { min_col => 0, min_row => 0, max_col => $cols, max_row => $rows } ];

    for my $instruction (@{ $scene->{instructions} // [] }) {
        _dispatch($instruction, $buffer, $scale_x, $scale_y, $clip_stack);
    }

    return _buffer_to_string($buffer);
}

1;
