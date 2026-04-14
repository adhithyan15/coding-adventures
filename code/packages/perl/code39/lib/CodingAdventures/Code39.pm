package CodingAdventures::Code39;

use strict;
use warnings;
use Carp qw(croak);

use CodingAdventures::BarcodeLayout1D ();

our $VERSION = '0.01';

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

my @COLORS = ('bar', 'space', 'bar', 'space', 'bar', 'space', 'bar', 'space', 'bar');
my %DEFAULT_CONFIG = (
  module_unit        => 4,
  bar_height         => 120,
  quiet_zone_modules => 10,
);

sub patterns       { return \%PATTERNS }
sub default_config { return { %DEFAULT_CONFIG } }

sub normalize_code39 {
  my ($class, $data) = @_;
  my $normalized = uc($data);
  for my $ch (split //, $normalized) {
    croak 'input must not contain "*" because it is reserved for start/stop' if $ch eq '*';
    croak sprintf('invalid character: "%s" is not supported by Code 39', $ch)
      unless exists $PATTERNS{$ch};
  }
  return $normalized;
}

sub encode_code39_char {
  my ($class, $char) = @_;
  my $raw = $PATTERNS{$char}
    or croak sprintf('unknown Code 39 character: "%s"', $char);
  (my $pattern = $raw) =~ s/(.)/uc($1) eq $1 ? 'W' : 'N'/ge;
  return {
    char          => $char,
    is_start_stop => ($char eq '*') ? 1 : 0,
    pattern       => $pattern,
  };
}

sub encode_code39 {
  my ($class, $data) = @_;
  my $normalized = $class->normalize_code39($data);
  my $with_markers = '*' . $normalized . '*';
  my @result = map { $class->encode_code39_char($_) } split //, $with_markers;
  return \@result;
}

sub expand_code39_runs {
  my ($class, $data) = @_;
  my $encoded = $class->encode_code39($data);
  my @runs;

  for my $source_index (0 .. $#$encoded) {
    my $enc_char = $encoded->[$source_index];
    my $char_runs = CodingAdventures::BarcodeLayout1D->runs_from_width_pattern(
      $enc_char->{pattern},
      \@COLORS,
      source_char  => $enc_char->{char},
      source_index => $source_index,
    );
    push @runs, @{$char_runs};

    if ($source_index < $#$encoded) {
      push @runs, {
        color        => 'space',
        modules      => 1,
        source_char  => $enc_char->{char},
        source_index => $source_index,
        role         => 'inter-character-gap',
        metadata     => {},
      };
    }
  }

  return \@runs;
}

sub layout_code39 {
  my ($class, $data, $config) = @_;
  $config //= {};
  my %cfg = (%DEFAULT_CONFIG, %{$config});

  my $normalized = $class->normalize_code39($data);
  return CodingAdventures::BarcodeLayout1D->layout_barcode_1d(
    $class->expand_code39_runs($normalized),
    \%cfg,
    {
      fill       => '#000000',
      background => '#ffffff',
      metadata   => {
        symbology => 'code39',
        data      => $normalized,
      },
    },
  );
}

sub draw_code39 {
  my ($class, @args) = @_;
  return $class->layout_code39(@args);
}

1;
