package CodingAdventures::Code39;

# ============================================================================
# CodingAdventures::Code39 — Code 39 Barcode Encoder
# ============================================================================
#
# # What is Code 39?
# ==================
#
# Code 39 is the first 1D barcode symbology to support both digits and
# uppercase letters.  It was invented in 1974 and is widely used in:
#
#   - US military logistics (MIL-STD-1189)
#   - Healthcare (HIBC)
#   - Automotive parts labeling
#   - Electronics manufacturing
#
# # How Code 39 Encodes Characters
# ==================================
#
# Every character is encoded as exactly 9 elements: 5 BARS and 4 SPACES,
# alternating strictly: bar-space-bar-space-bar-space-bar-space-bar.
#
# Of those 9 elements, exactly 3 are WIDE and 6 are NARROW.
# The pattern of wide/narrow widths encodes the character.
#
# Example — the character 'A':
#
#   Raw pattern: BwbwbWbwB
#
#   Position:  1   2   3   4   5   6   7   8   9
#   Element:   bar sp  bar sp  bar sp  bar sp  bar
#   Width:      W   n   n   n   n   W   n   n   W
#
#   Visual (not to scale):
#
#     ████  █  █   ██  █  ████
#      W    n  n    W  n   W
#
# Pattern key:
#   b = narrow bar    B = wide bar
#   w = narrow space  W = wide space
#
# # Start/Stop Character
# =======================
#
# Every Code 39 barcode begins and ends with the '*' character.
# '*' is NOT a valid character in user input — the encoder inserts it
# automatically.
#
#   Input:    "HELLO"
#   Encoded:  "*HELLO*"
#
# # Inter-Character Gap
# =======================
#
# Between each pair of encoded characters, a NARROW WHITE SPACE is inserted.
# This gap allows scanners to distinguish the boundary between characters.
#
# # Optional Checksum
# ====================
#
# Code 39 has an optional mod-43 checksum.  Each character has a numeric
# value (0-9 → 0-9, A-Z → 10-35, special chars → 36-42).  The check
# character is the character whose value equals (sum of all values) mod 43.
# This implementation provides compute_checksum() but does not require it.
#
# ============================================================================

use strict;
use warnings;
use Carp qw(croak);

our $VERSION = '0.01';

# ============================================================================
# Complete Code 39 pattern table
# ============================================================================
#
# Maps each of the 44 supported characters to its 9-element bar/space pattern.
# Uppercase letters in the pattern = WIDE; lowercase = NARROW.
# b/B = bar element; w/W = space element.

my %PATTERNS = (
  '0' => 'bwbWBwBwb', '1' => 'BwbWbwbwB', '2' => 'bwBWbwbwB', '3' => 'BwBWbwbwb',
  '4' => 'bwbWBwbwB', '5' => 'BwbWBwbwb', '6' => 'bwBWBwbwb', '7' => 'bwbWbwBwB',
  '8' => 'BwbWbwBwb', '9' => 'bwBWbwBwb', 'A' => 'BwbwbWbwB', 'B' => 'bwBwbWbwB',
  'C' => 'BwBwbWbwb', 'D' => 'bwbwBWbwB', 'E' => 'BwbwBWbwb', 'F' => 'bwBwBWbwb',
  'G' => 'bwbwbWBwB', 'H' => 'BwbwbWBwb', 'I' => 'bwBwbWBwb', 'J' => 'bwbwBWBwb',
  'K' => 'BwbwbwbWB', 'L' => 'bwBwbwbWB', 'M' => 'BwBwbwbWb', 'N' => 'bwbwBwbWB',
  'O' => 'BwbwBwbWb', 'P' => 'bwBwBwbWb', 'Q' => 'bwbwbwBWB', 'R' => 'BwbwbwBWb',
  'S' => 'bwBwbwBWb', 'T' => 'bwbwBwBWb', 'U' => 'BWbwbwbwB', 'V' => 'bWBwbwbwB',
  'W' => 'BWBwbwbwb', 'X' => 'bWbwBwbwB', 'Y' => 'BWbwBwbwb', 'Z' => 'bWBwBwbwb',
  '-' => 'bWbwbwBwB', '.' => 'BWbwbwBwb', ' ' => 'bWBwbwBwb', '$' => 'bWbWbWbwb',
  '/' => 'bWbWbwbWb', '+' => 'bWbwbWbWb', '%' => 'bwbWbWbWb', '*' => 'bWbwBwBwb',
);

# Ordered list of characters for mod-43 checksum lookup.
# Index 0='0', 1='1', ..., 9='9', 10='A', ..., 35='Z',
# 36='-', 37='.', 38=' ', 39='$', 40='/', 41='+', 42='%'
my @CHECKSUM_CHARS = split //, '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%';
my %CHECKSUM_VALUE = map { $CHECKSUM_CHARS[$_] => $_ } 0..$#CHECKSUM_CHARS;

# Default render configuration
my %DEFAULT_CONFIG = (
  narrow_unit              => 4,
  wide_unit                => 12,
  bar_height               => 120,
  quiet_zone_units         => 10,
  include_human_readable_text => 1,
);

# ============================================================================
# Public accessors for tests
# ============================================================================

sub patterns        { return \%PATTERNS }
sub default_config  { return {%DEFAULT_CONFIG} }

# ============================================================================
# normalize_code39($data) → $normalized_string
# ============================================================================
#
# Converts to uppercase, rejects '*', rejects any char not in the alphabet.

sub normalize_code39 {
  my ($class, $data) = @_;
  my $normalized = uc($data);
  for my $ch (split //, $normalized) {
    if ($ch eq '*') {
      croak 'input must not contain "*" because it is reserved for start/stop';
    }
    unless (exists $PATTERNS{$ch}) {
      croak sprintf('invalid character: "%s" is not supported by Code 39', $ch);
    }
  }
  return $normalized;
}

# ============================================================================
# encode_code39_char($char) → \%encoded_character
# ============================================================================
#
# Returns:
#   {
#     char          => 'A',
#     is_start_stop => 0,
#     pattern       => 'WNNNNWNNW',   # N=narrow, W=wide
#   }

sub encode_code39_char {
  my ($class, $char) = @_;
  my $raw = $PATTERNS{$char}
    or croak sprintf('unknown Code 39 character: "%s"', $char);

  # Convert b/B/w/W → N/W:  uc($c) eq $c means WIDE
  (my $pattern = $raw) =~ s/(.)/uc($1) eq $1 ? 'W' : 'N'/ge;

  return {
    char          => $char,
    is_start_stop => ($char eq '*') ? 1 : 0,
    pattern       => $pattern,
  };
}

# ============================================================================
# encode_code39($data) → \@encoded_characters
# ============================================================================
#
# Normalizes input, wraps with start/stop, encodes each character.

sub encode_code39 {
  my ($class, $data) = @_;
  my $normalized = $class->normalize_code39($data);
  my $with_markers = '*' . $normalized . '*';
  my @result = map { $class->encode_code39_char($_) } split //, $with_markers;
  return \@result;
}

# ============================================================================
# expand_code39_runs($data) → \@runs
# ============================================================================
#
# Each run is:
#   {
#     color                  => 'bar' | 'space',
#     width                  => 'narrow' | 'wide',
#     source_char            => 'A',
#     source_index           => 0,        # 0-based
#     is_inter_character_gap => 0 | 1,
#   }

my @COLORS = ('bar','space','bar','space','bar','space','bar','space','bar');

sub expand_code39_runs {
  my ($class, $data) = @_;
  my $encoded = $class->encode_code39($data);
  my @runs;

  for my $source_index (0..$#$encoded) {
    my $enc_char = $encoded->[$source_index];

    # Emit 9 elements for this character
    for my $elem_idx (0..8) {
      my $element = substr($enc_char->{pattern}, $elem_idx, 1);
      push @runs, {
        color                  => $COLORS[$elem_idx],
        width                  => ($element eq 'W') ? 'wide' : 'narrow',
        source_char            => $enc_char->{char},
        source_index           => $source_index,
        is_inter_character_gap => 0,
      };
    }

    # Inter-character gap (not after the last character)
    if ($source_index < $#$encoded) {
      push @runs, {
        color                  => 'space',
        width                  => 'narrow',
        source_char            => $enc_char->{char},
        source_index           => $source_index,
        is_inter_character_gap => 1,
      };
    }
  }

  return \@runs;
}

# ============================================================================
# draw_code39($data, \%config) → \%scene
# ============================================================================
#
# Produces a scene hash with either:
#   - svg: SVG string
#   - width, height, symbology, data
#
# Config keys (all optional):
#   narrow_unit, wide_unit, bar_height, quiet_zone_units,
#   include_human_readable_text

sub draw_code39 {
  my ($class, $data, $config) = @_;
  $config //= {};
  my %cfg = (%DEFAULT_CONFIG, %$config);

  my $normalized  = $class->normalize_code39($data);
  my $quiet_px    = $cfg{quiet_zone_units} * $cfg{narrow_unit};
  my $runs        = $class->expand_code39_runs($normalized);

  my $text_margin    = 8;
  my $text_font_size = 16;

  # Compute bar rectangles and total width
  my $cursor_x = $quiet_px;
  my @rects;

  for my $run (@$runs) {
    my $w = ($run->{width} eq 'wide') ? $cfg{wide_unit} : $cfg{narrow_unit};
    if ($run->{color} eq 'bar') {
      push @rects, {
        x      => $cursor_x,
        y      => 0,
        width  => $w,
        height => $cfg{bar_height},
        fill   => '#000000',
      };
    }
    $cursor_x += $w;
  }

  my $total_width = $cursor_x + $quiet_px;
  my $text_block  = $cfg{include_human_readable_text}
                  ? ($text_margin + $text_font_size + 4)
                  : 0;
  my $total_height = $cfg{bar_height} + $text_block;

  # Build SVG
  my @svg = (
    sprintf('<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" viewBox="0 0 %d %d">',
      $total_width, $total_height, $total_width, $total_height),
    sprintf('<rect x="0" y="0" width="%d" height="%d" fill="#ffffff"/>',
      $total_width, $total_height),
  );

  for my $r (@rects) {
    push @svg, sprintf('<rect x="%d" y="%d" width="%d" height="%d" fill="%s"/>',
      $r->{x}, $r->{y}, $r->{width}, $r->{height}, $r->{fill});
  }

  if ($cfg{include_human_readable_text}) {
    push @svg, sprintf('<text x="%d" y="%d" text-anchor="middle" font-size="%d">%s</text>',
      int($total_width / 2),
      $cfg{bar_height} + $text_margin + $text_font_size - 2,
      $text_font_size,
      $normalized);
  }

  push @svg, '</svg>';

  return {
    svg       => join("\n", @svg),
    width     => $total_width,
    height    => $total_height,
    symbology => 'code39',
    data      => $normalized,
    rects     => \@rects,
  };
}

# ============================================================================
# compute_checksum($data) → $check_char
# ============================================================================
#
# Computes the optional mod-43 check character.
# Raises if the data contains an invalid character.

sub compute_checksum {
  my ($class, $data) = @_;
  my $total = 0;
  for my $ch (split //, uc($data)) {
    croak sprintf('cannot compute checksum for invalid character "%s"', $ch)
      unless exists $CHECKSUM_VALUE{$ch};
    $total += $CHECKSUM_VALUE{$ch};
  }
  return $CHECKSUM_CHARS[$total % 43];
}

1;
__END__

=head1 NAME

CodingAdventures::Code39 - Code 39 barcode encoder

=head1 SYNOPSIS

  use CodingAdventures::Code39;

  my $scene = CodingAdventures::Code39->draw_code39('HELLO123');
  print $scene->{svg};

=head1 DESCRIPTION

Encodes strings in the Code 39 barcode format. Supports uppercase letters,
digits, and the special characters - . SPACE $ / + %.

=cut
