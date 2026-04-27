package CodingAdventures::Codabar;

use strict;
use warnings;
use Carp qw(croak);

use CodingAdventures::BarcodeLayout1D ();

our $VERSION = '0.01';

my %PATTERNS = (
  '0' => '101010011',  '1' => '101011001',  '2' => '101001011',  '3' => '110010101',
  '4' => '101101001',  '5' => '110101001',  '6' => '100101011',  '7' => '100101101',
  '8' => '100110101',  '9' => '110100101',  '-' => '101001101',  '$' => '101100101',
  ':' => '1101011011', '/' => '1101101011', '.' => '1101101101', '+' => '1011011011',
  'A' => '1011001001', 'B' => '1001001011', 'C' => '1010010011', 'D' => '1010011001',
);

my %DEFAULT_CONFIG = (
  module_unit        => 4,
  bar_height         => 120,
  quiet_zone_modules => 10,
);

my %GUARDS = map { $_ => 1 } qw(A B C D);

sub patterns       { return \%PATTERNS }
sub default_config { return { %DEFAULT_CONFIG } }

sub _guard {
  my ($char) = @_;
  return exists $GUARDS{$char};
}

sub _assert_body_chars {
  my ($body) = @_;
  for my $char (split //, $body) {
    croak qq(invalid Codabar body character "$char")
      unless exists $PATTERNS{$char} && !_guard($char);
  }
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

sub normalize_codabar {
  my ($class, $data, %options) = @_;
  my $start = uc($options{start} // 'A');
  my $stop = uc($options{stop} // 'A');
  my $normalized = uc($data);

  if (length($normalized) >= 2 && _guard(substr($normalized, 0, 1)) && _guard(substr($normalized, -1))) {
    _assert_body_chars(substr($normalized, 1, -1));
    return $normalized;
  }

  croak 'Codabar guards must be one of A, B, C, or D'
    unless _guard($start) && _guard($stop);

  _assert_body_chars($normalized);
  return $start . $normalized . $stop;
}

sub encode_codabar {
  my ($class, $data, %options) = @_;
  my $normalized = $class->normalize_codabar($data, %options);
  my @chars = split //, $normalized;
  my @encoded;

  for my $index (0 .. $#chars) {
    my $char = $chars[$index];
    my $role = $index == 0 ? 'start' : $index == $#chars ? 'stop' : 'data';
    push @encoded, {
      char         => $char,
      pattern      => $PATTERNS{$char},
      source_index => $index,
      role         => $role,
    };
  }

  return \@encoded;
}

sub expand_codabar_runs {
  my ($class, $data, %options) = @_;
  my $encoded = $class->encode_codabar($data, %options);
  my @runs;

  for my $index (0 .. $#$encoded) {
    my $symbol = $encoded->[$index];
    my $symbol_runs = _retag_runs(
      CodingAdventures::BarcodeLayout1D->runs_from_binary_pattern(
        $symbol->{pattern},
        source_char  => $symbol->{char},
        source_index => $symbol->{source_index},
      ),
      $symbol->{role},
    );
    push @runs, @{$symbol_runs};

    if ($index < $#$encoded) {
      push @runs, {
        color        => 'space',
        modules      => 1,
        source_char  => $symbol->{char},
        source_index => $symbol->{source_index},
        role         => 'inter-character-gap',
        metadata     => {},
      };
    }
  }

  return \@runs;
}

sub layout_codabar {
  my ($class, $data, $config, $options) = @_;
  $config //= {};
  $options //= {};
  my %cfg = (%DEFAULT_CONFIG, %{$config});

  my $start = $options->{start} // 'A';
  my $stop = $options->{stop} // 'A';
  my $normalized = $class->normalize_codabar($data, start => $start, stop => $stop);

  return CodingAdventures::BarcodeLayout1D->draw_one_dimensional_barcode(
    $class->expand_codabar_runs($normalized),
    \%cfg,
    {
      metadata => {
        symbology => 'codabar',
        start     => substr($normalized, 0, 1),
        stop      => substr($normalized, -1),
      },
    },
  );
}

sub draw_codabar {
  my ($class, @args) = @_;
  return $class->layout_codabar(@args);
}

1;
