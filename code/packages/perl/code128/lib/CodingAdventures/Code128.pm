package CodingAdventures::Code128;

use strict;
use warnings;
use Carp qw(croak);

use CodingAdventures::BarcodeLayout1D ();

our $VERSION = '0.01';

my $START_B = 104;
my $STOP = 106;

my @PATTERNS = qw(
  11011001100 11001101100 11001100110 10010011000 10010001100 10001001100 10011001000 10011000100
  10001100100 11001001000 11001000100 11000100100 10110011100 10011011100 10011001110 10111001100
  10011101100 10011100110 11001110010 11001011100 11001001110 11011100100 11001110100 11101101110
  11101001100 11100101100 11100100110 11101100100 11100110100 11100110010 11011011000 11011000110
  11000110110 10100011000 10001011000 10001000110 10110001000 10001101000 10001100010 11010001000
  11000101000 11000100010 10110111000 10110001110 10001101110 10111011000 10111000110 10001110110
  11101110110 11010001110 11000101110 11011101000 11011100010 11011101110 11101011000 11101000110
  11100010110 11101101000 11101100010 11100011010 11101111010 11001000010 11110001010 10100110000
  10100001100 10010110000 10010000110 10000101100 10000100110 10110010000 10110000100 10011010000
  10011000010 10000110100 10000110010 11000010010 11001010000 11110111010 11000010100 10001111010
  10100111100 10010111100 10010011110 10111100100 10011110100 10011110010 11110100100 11110010100
  11110010010 11011011110 11011110110 11110110110 10101111000 10100011110 10001011110 10111101000
  10111100010 11110101000 11110100010 10111011110 10111101110 11101011110 11110101110 11010000100
  11010010000 11010011100 1100011101011
);

my %DEFAULT_CONFIG = (
  module_unit        => 4,
  bar_height         => 120,
  quiet_zone_modules => 10,
);

sub default_config { return { %DEFAULT_CONFIG } }

sub _retag_runs {
  my ($runs, $role) = @_;
  return [
    map {
      +{
        %$_,
        role     => $role,
        metadata => { %{ $_->{metadata} // {} } },
      }
    } @{$runs}
  ];
}

sub normalize_code128_b {
  my ($class, $data) = @_;
  for my $char (split //, $data) {
    my $code = ord($char);
    next if $code >= 32 && $code <= 126;
    croak 'Code 128 Code Set B supports printable ASCII characters only';
  }
  return $data;
}

sub value_for_code128_b_char {
  my ($class, $char) = @_;
  return ord($char) - 32;
}

sub compute_code128_checksum {
  my ($class, $values) = @_;
  my $total = $START_B;
  for my $index (0 .. $#{$values}) {
    $total += $values->[$index] * ($index + 1);
  }
  return $total % 103;
}

sub encode_code128_b {
  my ($class, $data) = @_;
  my $normalized = $class->normalize_code128_b($data);
  my @chars = split //, $normalized;
  my @symbols;

  for my $index (0 .. $#chars) {
    my $char = $chars[$index];
    my $value = $class->value_for_code128_b_char($char);
    push @symbols, {
      label        => $char,
      value        => $value,
      pattern      => $PATTERNS[$value],
      source_index => $index,
      role         => 'data',
    };
  }

  my $checksum = $class->compute_code128_checksum([ map { $_->{value} } @symbols ]);

  return [
    {
      label        => 'Start B',
      value        => $START_B,
      pattern      => $PATTERNS[$START_B],
      source_index => -1,
      role         => 'start',
    },
    @symbols,
    {
      label        => "Checksum $checksum",
      value        => $checksum,
      pattern      => $PATTERNS[$checksum],
      source_index => length($normalized),
      role         => 'check',
    },
    {
      label        => 'Stop',
      value        => $STOP,
      pattern      => $PATTERNS[$STOP],
      source_index => length($normalized) + 1,
      role         => 'stop',
    },
  ];
}

sub expand_code128_runs {
  my ($class, $data) = @_;
  my $encoded = $class->encode_code128_b($data);
  return [
    map {
      @{ _retag_runs(
        CodingAdventures::BarcodeLayout1D->runs_from_binary_pattern(
          $_->{pattern},
          source_char  => $_->{label},
          source_index => $_->{source_index},
        ),
        $_->{role},
      ) }
    } @{$encoded}
  ];
}

sub layout_code128 {
  my ($class, $data, $config) = @_;
  $config //= {};
  my %cfg = (%DEFAULT_CONFIG, %{$config});

  my $normalized = $class->normalize_code128_b($data);
  my $checksum = $class->encode_code128_b($normalized)->[-2]{value};

  return CodingAdventures::BarcodeLayout1D->draw_one_dimensional_barcode(
    $class->expand_code128_runs($normalized),
    \%cfg,
    {
      metadata => {
        symbology => 'code128',
        code_set  => 'B',
        checksum  => $checksum,
      },
    },
  );
}

sub draw_code128 {
  my ($class, @args) = @_;
  return $class->layout_code128(@args);
}

1;
