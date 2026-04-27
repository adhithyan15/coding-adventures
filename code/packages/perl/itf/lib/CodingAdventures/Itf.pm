package CodingAdventures::Itf;

use strict;
use warnings;
use Carp qw(croak);

use CodingAdventures::BarcodeLayout1D ();

our $VERSION = '0.01';

my $START_PATTERN = '1010';
my $STOP_PATTERN = '11101';

my @DIGIT_PATTERNS = qw(
  00110 10001 01001 11000 00101 10100 01100 00011 10010 01010
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

sub normalize_itf {
  my ($class, $data) = @_;
  croak 'ITF input must contain digits only' unless $data =~ /\A\d+\z/;
  croak 'ITF input must contain an even number of digits'
    if $data eq q{} || length($data) % 2 != 0;
  return $data;
}

sub encode_itf {
  my ($class, $data) = @_;
  my $normalized = $class->normalize_itf($data);
  my @digits = split //, $normalized;
  my @encoded;

  for (my $index = 0; $index < @digits; $index += 2) {
    my $left = $digits[$index];
    my $right = $digits[$index + 1];
    my $bar_pattern = $DIGIT_PATTERNS[$left];
    my $space_pattern = $DIGIT_PATTERNS[$right];
    my $binary_pattern = q{};

    for my $offset (0 .. 4) {
      my $bar_marker = substr($bar_pattern, $offset, 1);
      my $space_marker = substr($space_pattern, $offset, 1);
      $binary_pattern .= ($bar_marker eq '1' ? '111' : '1') . ($space_marker eq '1' ? '000' : '0');
    }

    push @encoded, {
      pair           => $left . $right,
      bar_pattern    => $bar_pattern,
      space_pattern  => $space_pattern,
      binary_pattern => $binary_pattern,
      source_index   => $index / 2,
    };
  }

  return \@encoded;
}

sub expand_itf_runs {
  my ($class, $data) = @_;
  my $encoded = $class->encode_itf($data);
  my @runs;

  push @runs, @{ _retag_runs(
    CodingAdventures::BarcodeLayout1D->runs_from_binary_pattern($START_PATTERN, source_char => 'start', source_index => -1),
    'start',
  ) };

  for my $entry (@{$encoded}) {
    push @runs, @{ _retag_runs(
      CodingAdventures::BarcodeLayout1D->runs_from_binary_pattern(
        $entry->{binary_pattern},
        source_char  => $entry->{pair},
        source_index => $entry->{source_index},
      ),
      'data',
    ) };
  }

  push @runs, @{ _retag_runs(
    CodingAdventures::BarcodeLayout1D->runs_from_binary_pattern($STOP_PATTERN, source_char => 'stop', source_index => -2),
    'stop',
  ) };

  return \@runs;
}

sub layout_itf {
  my ($class, $data, $config) = @_;
  $config //= {};
  my %cfg = (%DEFAULT_CONFIG, %{$config});
  my $normalized = $class->normalize_itf($data);

  return CodingAdventures::BarcodeLayout1D->draw_one_dimensional_barcode(
    $class->expand_itf_runs($normalized),
    \%cfg,
    {
      metadata => {
        symbology => 'itf',
        pair_count => length($normalized) / 2,
      },
    },
  );
}

sub draw_itf {
  my ($class, @args) = @_;
  return $class->layout_itf(@args);
}

1;
