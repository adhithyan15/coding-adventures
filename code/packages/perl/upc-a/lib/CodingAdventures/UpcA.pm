package CodingAdventures::UpcA;

use strict;
use warnings;
use Carp qw(croak);

use CodingAdventures::BarcodeLayout1D ();

our $VERSION = '0.01';

my $SIDE_GUARD = '101';
my $CENTER_GUARD = '01010';

my %DIGIT_PATTERNS = (
  L => [qw(0001101 0011001 0010011 0111101 0100011 0110001 0101111 0111011 0110111 0001011)],
  R => [qw(1110010 1100110 1101100 1000010 1011100 1001110 1010000 1000100 1001000 1110100)],
);

my %DEFAULT_CONFIG = (
  module_unit        => 4,
  bar_height         => 120,
  quiet_zone_modules => 10,
);

sub default_config { return { %DEFAULT_CONFIG } }

sub _assert_digits {
  my ($data, @lengths) = @_;
  croak 'UPC-A input must contain digits only' unless $data =~ /\A\d+\z/;
  my %allowed = map { $_ => 1 } @lengths;
  croak 'UPC-A input must contain 11 digits or 12 digits'
    unless exists $allowed{length($data)};
}

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

sub compute_upc_a_check_digit {
  my ($class, $payload11) = @_;
  _assert_digits($payload11, 11);
  my @digits = split //, $payload11;
  my ($odd_sum, $even_sum) = (0, 0);

  for my $index (0 .. $#digits) {
    if ($index % 2 == 0) {
      $odd_sum += $digits[$index];
    } else {
      $even_sum += $digits[$index];
    }
  }

  return (10 - ((($odd_sum * 3) + $even_sum) % 10)) % 10;
}

sub normalize_upc_a {
  my ($class, $data) = @_;
  _assert_digits($data, 11, 12);
  return $data . $class->compute_upc_a_check_digit($data) if length($data) == 11;

  my $expected = $class->compute_upc_a_check_digit(substr($data, 0, 11));
  my $actual = substr($data, 11, 1);
  croak "Invalid UPC-A check digit: expected $expected but received $actual"
    unless $expected eq $actual;
  return $data;
}

sub encode_upc_a {
  my ($class, $data) = @_;
  my $normalized = $class->normalize_upc_a($data);
  my @digits = split //, $normalized;
  my @encoded;

  for my $index (0 .. $#digits) {
    my $digit = $digits[$index];
    my $encoding = $index < 6 ? 'L' : 'R';
    push @encoded, {
      digit        => $digit,
      encoding     => $encoding,
      pattern      => $DIGIT_PATTERNS{$encoding}[$digit],
      source_index => $index,
      role         => $index == 11 ? 'check' : 'data',
    };
  }

  return \@encoded;
}

sub expand_upc_a_runs {
  my ($class, $data) = @_;
  my $encoded = $class->encode_upc_a($data);
  my @runs;

  push @runs, @{ _retag_runs(
    CodingAdventures::BarcodeLayout1D->runs_from_binary_pattern($SIDE_GUARD, source_char => 'start', source_index => -1),
    'guard',
  ) };

  for my $entry (@{$encoded}[0 .. 5]) {
    push @runs, @{ _retag_runs(
      CodingAdventures::BarcodeLayout1D->runs_from_binary_pattern(
        $entry->{pattern},
        source_char  => $entry->{digit},
        source_index => $entry->{source_index},
      ),
      $entry->{role},
    ) };
  }

  push @runs, @{ _retag_runs(
    CodingAdventures::BarcodeLayout1D->runs_from_binary_pattern($CENTER_GUARD, source_char => 'center', source_index => -2),
    'guard',
  ) };

  for my $entry (@{$encoded}[6 .. $#$encoded]) {
    push @runs, @{ _retag_runs(
      CodingAdventures::BarcodeLayout1D->runs_from_binary_pattern(
        $entry->{pattern},
        source_char  => $entry->{digit},
        source_index => $entry->{source_index},
      ),
      $entry->{role},
    ) };
  }

  push @runs, @{ _retag_runs(
    CodingAdventures::BarcodeLayout1D->runs_from_binary_pattern($SIDE_GUARD, source_char => 'end', source_index => -3),
    'guard',
  ) };

  return \@runs;
}

sub layout_upc_a {
  my ($class, $data, $config) = @_;
  $config //= {};
  my %cfg = (%DEFAULT_CONFIG, %{$config});
  my $normalized = $class->normalize_upc_a($data);

  return CodingAdventures::BarcodeLayout1D->draw_one_dimensional_barcode(
    $class->expand_upc_a_runs($normalized),
    \%cfg,
    {
      metadata => {
        symbology      => 'upc-a',
        content_modules => 95,
      },
    },
  );
}

sub draw_upc_a {
  my ($class, @args) = @_;
  return $class->layout_upc_a(@args);
}

1;
