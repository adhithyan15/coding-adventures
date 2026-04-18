package CodingAdventures::PaintVmAscii;

use strict;
use warnings;
use utf8;

use CodingAdventures::PaintInstructions;

our $VERSION = '0.1.0';

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

sub _to_col {
    my ($x, $scale_x) = @_;
    return sprintf('%.0f', $x / $scale_x) + 0;
}

sub _to_row {
    my ($y, $scale_y) = @_;
    return sprintf('%.0f', $y / $scale_y) + 0;
}

sub _new_buffer {
    my ($rows, $cols) = @_;
    my @chars;
    for my $row (0 .. $rows - 1) {
        $chars[$row] = [ (' ') x $cols ];
    }
    return {
        rows  => $rows,
        cols  => $cols,
        chars => \@chars,
    };
}

sub _write_char {
    my ($buffer, $row, $col, $ch) = @_;
    return if $row < 0 || $row >= $buffer->{rows};
    return if $col < 0 || $col >= $buffer->{cols};
    $buffer->{chars}[$row][$col] = $ch;
}

sub _buffer_to_string {
    my ($buffer) = @_;
    my @lines;
    for my $row (0 .. $buffer->{rows} - 1) {
        my $line = join('', @{ $buffer->{chars}[$row] });
        $line =~ s/\s+$//;
        push @lines, $line;
    }
    my $result = join("\n", @lines);
    $result =~ s/[\s\n]+$//;
    return $result;
}

sub _render_rect {
    my ($instruction, $buffer, $scale_x, $scale_y) = @_;
    my $fill = $instruction->{fill} // '#000000';
    return if $fill eq '' || $fill eq 'transparent' || $fill eq 'none';

    my $c1 = _to_col($instruction->{x}, $scale_x);
    my $r1 = _to_row($instruction->{y}, $scale_y);
    my $c2 = _to_col($instruction->{x} + $instruction->{width}, $scale_x);
    my $r2 = _to_row($instruction->{y} + $instruction->{height}, $scale_y);

    for my $row ($r1 .. $r2) {
        for my $col ($c1 .. $c2) {
            _write_char($buffer, $row, $col, "\x{2588}");
        }
    }
}

sub render {
    my ($class, $scene, $options) = @_;
    my $scale_x = _scale_x($options);
    my $scale_y = _scale_y($options);

    my $cols = int((($scene->{width} // 0) + $scale_x - 1) / $scale_x);
    my $rows = int((($scene->{height} // 0) + $scale_y - 1) / $scale_y);
    my $buffer = _new_buffer($rows, $cols);

    for my $instruction (@{ $scene->{instructions} // [] }) {
        my $kind = $instruction->{kind} // '';
        if ($kind eq 'rect') {
            _render_rect($instruction, $buffer, $scale_x, $scale_y);
            next;
        }
        die "paint-vm-ascii: unsupported paint instruction kind: $kind";
    }

    return _buffer_to_string($buffer);
}

1;
