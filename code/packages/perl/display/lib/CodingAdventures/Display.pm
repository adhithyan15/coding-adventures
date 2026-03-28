package CodingAdventures::Display;

# ============================================================================
# CodingAdventures::Display — Text-mode display driver with framebuffer
# ============================================================================
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.
#
# WHAT IS A TEXT-MODE DISPLAY?
# ----------------------------
# Before graphical user interfaces, computers displayed output as a grid of
# characters — the classic "80x25" terminal.  The VGA text-mode hardware used
# by early PCs stored two bytes for every cell on screen:
#
#   byte 0 — the ASCII code of the character (e.g., 65 for 'A')
#   byte 1 — the attribute byte (foreground/background colour, e.g., 0x07)
#
# So a 80-column × 25-row display requires:
#   80 × 25 × 2 = 4000 bytes
#
# This module simulates that hardware:
#
#   +---+---+---+---+---+ ... +---+---+
#   | H | . | e | . | l | ... | . | . |   row 0 (80 char+attr pairs)
#   +---+---+---+---+---+ ... +---+---+
#   | . | . | . | . | . | ... | . | . |   row 1
#   ...
#
# CURSOR MODEL
# ------------
# The display maintains a cursor (row, col).  Characters are written at the
# cursor position, then the cursor advances.  Special control characters alter
# cursor movement without writing a visible glyph.
#
# Usage:
#
#   use CodingAdventures::Display;
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# DisplayConfig — immutable configuration for a virtual display
# ============================================================================
#
# Stores the dimensions of the display.  Kept separate from the driver so the
# same config can be passed to multiple drivers.

package CodingAdventures::Display::DisplayConfig;

sub new {
    my ($class, $args) = @_;
    $args ||= {};
    return bless {
        columns => $args->{columns} // 80,
        rows    => $args->{rows}    // 25,
    }, $class;
}

# Accessor: number of columns (characters per row)
sub columns { return $_[0]->{columns} }

# Accessor: number of rows
sub rows    { return $_[0]->{rows}    }

# ============================================================================
# DisplayDriver — stateful character-display controller
# ============================================================================
#
# The driver owns:
#   - a reference to the flat memory array (2 elements per cell)
#   - a cursor position (row, col)
#   - the config (dimensions)
#
# Memory layout for cell (row, col):
#   index = (row * columns + col) * 2
#   memory[index]     = character byte (ASCII)
#   memory[index + 1] = attribute byte (colour)
#
# The default attribute is 0x07 (light-grey on black, classic VGA).

package CodingAdventures::Display::DisplayDriver;

# Default attribute byte: 0x07 = white-on-black in VGA text mode
use constant DEFAULT_ATTR => 0x07;
use constant SPACE_CHAR   => 32;   # ASCII space

# new($config, \@memory)
# ---------------------
# $config is a DisplayConfig.
# \@memory is an arrayref pre-allocated to columns*rows*2 elements.
# The driver does NOT resize the array; the caller must allocate it.
sub new {
    my ($class, $config, $memory_ref) = @_;
    return bless {
        config      => $config,
        memory      => $memory_ref,
        cursor_row  => 0,
        cursor_col  => 0,
    }, $class;
}

# _cell_index(row, col) — compute byte offset in memory array
sub _cell_index {
    my ($self, $row, $col) = @_;
    return ($row * $self->{config}->columns + $col) * 2;
}

# _write_cell(row, col, char_byte, attr_byte) — write a single cell
sub _write_cell {
    my ($self, $row, $col, $char_byte, $attr_byte) = @_;
    my $idx = $self->_cell_index($row, $col);
    $self->{memory}[$idx]     = $char_byte;
    $self->{memory}[$idx + 1] = $attr_byte;
}

# _read_char(row, col) — return the character byte at a cell
sub _read_char {
    my ($self, $row, $col) = @_;
    my $idx = $self->_cell_index($row, $col);
    return $self->{memory}[$idx] // SPACE_CHAR;
}

# _scroll() — shift all rows up by one, clear the bottom row
#
# When the cursor would move below the last row, the display scrolls:
#
#   Before:          After:
#   row 0: "Hello"   row 0: "World"
#   row 1: "World"   row 1: "Foo"
#   row 2: "Foo"     row 2: ""       <- cleared
#
# This is the same behaviour as a real VT100/VGA text terminal.
sub _scroll {
    my ($self) = @_;
    my $cols = $self->{config}->columns;
    my $rows = $self->{config}->rows;
    # Copy each row upward (row 1 -> row 0, row 2 -> row 1, …)
    for my $row (0 .. $rows - 2) {
        for my $col (0 .. $cols - 1) {
            my $from = $self->_cell_index($row + 1, $col);
            my $to   = $self->_cell_index($row,     $col);
            $self->{memory}[$to]     = $self->{memory}[$from] // SPACE_CHAR;
            $self->{memory}[$to + 1] = $self->{memory}[$from + 1] // DEFAULT_ATTR;
        }
    }
    # Clear the last row
    for my $col (0 .. $cols - 1) {
        $self->_write_cell($rows - 1, $col, SPACE_CHAR, DEFAULT_ATTR);
    }
}

# _advance_cursor() — move cursor right, wrapping and scrolling as needed
sub _advance_cursor {
    my ($self) = @_;
    my $cols = $self->{config}->columns;
    my $rows = $self->{config}->rows;
    $self->{cursor_col}++;
    if ($self->{cursor_col} >= $cols) {
        $self->{cursor_col} = 0;
        $self->{cursor_row}++;
    }
    if ($self->{cursor_row} >= $rows) {
        $self->_scroll;
        $self->{cursor_row} = $rows - 1;
    }
}

# _newline() — move cursor to start of next row, scrolling if needed
sub _newline {
    my ($self) = @_;
    my $rows = $self->{config}->rows;
    $self->{cursor_col} = 0;
    $self->{cursor_row}++;
    if ($self->{cursor_row} >= $rows) {
        $self->_scroll;
        $self->{cursor_row} = $rows - 1;
    }
}

# put_char($byte) — write one byte to the display at the current cursor
# -------------------------------------------------------------------
# Control characters are handled specially:
#
#   0x08  Backspace — move cursor left one column (clamp at 0)
#   0x09  Horizontal Tab — advance to next multiple of 8
#   0x0A  Line Feed (newline) — move cursor to col=0, row+1
#   0x0D  Carriage Return — move cursor to col=0
#
# All other byte values are written as visible characters.
sub put_char {
    my ($self, $byte) = @_;
    if ($byte == 0x0A) {
        # Line feed: col=0, next row
        $self->_newline;
    }
    elsif ($byte == 0x0D) {
        # Carriage return: col=0, same row
        $self->{cursor_col} = 0;
    }
    elsif ($byte == 0x09) {
        # Horizontal tab: advance to next tab stop (every 8 columns)
        my $next_tab = (int($self->{cursor_col} / 8) + 1) * 8;
        my $cols     = $self->{config}->columns;
        if ($next_tab >= $cols) {
            $self->_newline;
        } else {
            $self->{cursor_col} = $next_tab;
        }
    }
    elsif ($byte == 0x08) {
        # Backspace: move cursor left (but not before column 0)
        $self->{cursor_col}-- if $self->{cursor_col} > 0;
    }
    else {
        # Printable character
        $self->_write_cell(
            $self->{cursor_row},
            $self->{cursor_col},
            $byte,
            DEFAULT_ATTR
        );
        $self->_advance_cursor;
    }
}

# puts_str($string) — write a string character by character
# ---------------------------------------------------------
# A convenience wrapper around put_char that accepts a Perl string.
sub puts_str {
    my ($self, $str) = @_;
    for my $ch (split //, $str) {
        $self->put_char(ord($ch));
    }
}

# clear() — fill the entire framebuffer with spaces and reset cursor
# ------------------------------------------------------------------
# Equivalent to the ANSI "clear screen" operation.  Each cell is set to
# (SPACE_CHAR=32, DEFAULT_ATTR=0x07).
sub clear {
    my ($self) = @_;
    my $cols = $self->{config}->columns;
    my $rows = $self->{config}->rows;
    for my $row (0 .. $rows - 1) {
        for my $col (0 .. $cols - 1) {
            $self->_write_cell($row, $col, SPACE_CHAR, DEFAULT_ATTR);
        }
    }
    $self->{cursor_row} = 0;
    $self->{cursor_col} = 0;
}

# snapshot() — capture current framebuffer as human-readable lines
# ----------------------------------------------------------------
# Returns a hashref:
#   {
#     lines => [ "row0 content", "row1 content", ... ]
#   }
#
# Trailing spaces are stripped from each line so tests are easy to write.
# Empty rows appear as the empty string "".
sub snapshot {
    my ($self) = @_;
    my $cols = $self->{config}->columns;
    my $rows = $self->{config}->rows;
    my @lines;
    for my $row (0 .. $rows - 1) {
        my $line = '';
        for my $col (0 .. $cols - 1) {
            my $byte = $self->_read_char($row, $col);
            # Treat NUL (0x00) as a space — uncleared cells may be zero
            $line .= ($byte == 0) ? ' ' : chr($byte);
        }
        # Strip trailing whitespace (spaces and NUL-turned-spaces)
        $line =~ s/\s+$//;
        push @lines, $line;
    }
    return { lines => \@lines };
}

# cursor_row() / cursor_col() — read-only accessors for the cursor position
sub cursor_row { return $_[0]->{cursor_row} }
sub cursor_col { return $_[0]->{cursor_col} }

# ============================================================================
# Back to the top-level package so the file ends with a true value
# ============================================================================

package CodingAdventures::Display;

1;

__END__

=head1 NAME

CodingAdventures::Display - Text-mode display driver with framebuffer, cursor, and scrolling

=head1 SYNOPSIS

    use CodingAdventures::Display;

    my $config = CodingAdventures::Display::DisplayConfig->new({
        columns => 80,
        rows    => 25,
    });
    my @memory = (0) x ($config->columns * $config->rows * 2);
    my $driver = CodingAdventures::Display::DisplayDriver->new($config, \@memory);

    $driver->puts_str("Hello, world!");
    $driver->put_char(0x0A);   # newline

    my $snap = $driver->snapshot;
    print $snap->{lines}[0];   # "Hello, world!"

=head1 DESCRIPTION

Simulates a VGA-style character-mode display.  Memory is a flat array of
(char_byte, attr_byte) pairs, one pair per cell.  The driver keeps a cursor
and handles wrap-around, scrolling, and common control characters.

=head2 Classes

=over 4

=item CodingAdventures::Display::DisplayConfig

Holds C<columns> and C<rows>.  Immutable after construction.

=item CodingAdventures::Display::DisplayDriver

The stateful driver.  Call C<put_char>, C<puts_str>, C<clear>, and
C<snapshot>.

=back

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
