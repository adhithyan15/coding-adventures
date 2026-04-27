package CodingAdventures::Ean13;

use strict;
use warnings;
use Carp qw(croak);

use CodingAdventures::BarcodeLayout1D ();

our $VERSION = '0.01';

my $SIDE_GUARD = '101';
my $CENTER_GUARD = '01010';

my %DIGIT_PATTERNS = (
  L => [qw(0001101 0011001 0010011 0111101 0100011 0110001 0101111 0111011 0110111 0001011)],
  G => [qw(0100111 0110011 0011011 0100001 0011101 0111001 0000101 0010001 0001001 0010111)],
  R => [qw(1110010 1100110 1101100 1000010 1011100 1001110 1010000 1000100 1001000 1110100)],
);

my @LEFT_PARITY_PATTERNS = qw(
  LLLLLL LLGLGG LLGGLG LLGGGL LGLLGG LGGLLG LGGGLL LGLGLG LGLGGL LGGLGL
);

my %DEFAULT_CONFIG = (
  module_unit        => 4,
  bar_height         => 120,
  quiet_zone_modules => 10,
);

sub default_config { return { %DEFAULT_CONFIG } }

sub _assert_digits {
  my ($data, @lengths) = @_;
  croak 'EAN-13 input must contain digits only' unless $data =~ /\A\d+\z/;
  my %allowed = map { $_ => 1 } @lengths;
  croak 'EAN-13 input must contain 12 digits or 13 digits'
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

sub compute_ean_13_check_digit {
  my ($class, $payload12) = @_;
  _assert_digits($payload12, 12);
  my @digits = reverse split //, $payload12;
  my $total = 0;
  for my $index (0 .. $#digits) {
    $total += $digits[$index] * ($index % 2 == 0 ? 3 : 1);
  }
  return (10 - ($total % 10)) % 10;
}

sub normalize_ean_13 {
  my ($class, $data) = @_;
  _assert_digits($data, 12, 13);
  return $data . $class->compute_ean_13_check_digit($data) if length($data) == 12;

  my $expected = $class->compute_ean_13_check_digit(substr($data, 0, 12));
  my $actual = substr($data, 12, 1);
  croak "Invalid EAN-13 check digit: expected $expected but received $actual"
    unless $expected eq $actual;
  return $data;
}

sub left_parity_pattern {
  my ($class, $data) = @_;
  my $normalized = $class->normalize_ean_13($data);
  return $LEFT_PARITY_PATTERNS[substr($normalized, 0, 1)];
}

sub encode_ean_13 {
  my ($class, $data) = @_;
  my $normalized = $class->normalize_ean_13($data);
  my $parity = $class->left_parity_pattern($normalized);
  my @digits = split //, $normalized;
  my @encoded;

  for my $offset (0 .. 5) {
    my $digit = $digits[$offset + 1];
    my $encoding = substr($parity, $offset, 1);
    push @encoded, {
      digit        => $digit,
      encoding     => $encoding,
      pattern      => $DIGIT_PATTERNS{$encoding}[$digit],
      source_index => $offset + 1,
      role         => 'data',
    };
  }

  for my $offset (0 .. 5) {
    my $digit = $digits[$offset + 7];
    push @encoded, {
      digit        => $digit,
      encoding     => 'R',
      pattern      => $DIGIT_PATTERNS{R}[$digit],
      source_index => $offset + 7,
      role         => $offset == 5 ? 'check' : 'data',
    };
  }

  return \@encoded;
}

sub expand_ean_13_runs {
  my ($class, $data) = @_;
  my $encoded = $class->encode_ean_13($data);
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

sub layout_ean_13 {
  my ($class, $data, $config) = @_;
  $config //= {};
  my %cfg = (%DEFAULT_CONFIG, %{$config});
  my $normalized = $class->normalize_ean_13($data);

  return CodingAdventures::BarcodeLayout1D->draw_one_dimensional_barcode(
    $class->expand_ean_13_runs($normalized),
    \%cfg,
    {
      metadata => {
        symbology      => 'ean-13',
        leading_digit  => substr($normalized, 0, 1),
        left_parity    => $class->left_parity_pattern($normalized),
        content_modules => 95,
      },
    },
  );
}

sub draw_ean_13 {
  my ($class, @args) = @_;
  return $class->layout_ean_13(@args);
}

1;
