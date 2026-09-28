use strict;
use warnings;
use Test2::V0;
use Digest::SHA qw(sha256_hex);
use JSON::PP qw(decode_json encode_json);

use lib '../paint-instructions/lib';
use lib '../barcode-layout-1d/lib';

require CodingAdventures::Itf;
my $pkg = 'CodingAdventures::Itf';

open my $fixture_handle, '<:raw', '../../../specs/fixtures/barcode-symbologies-v1/cases.json'
  or die "cannot open barcode fixture: $!";
local $/;
my $corpus = decode_json(<$fixture_handle>);
close $fixture_handle;
my @cases = grep { $_->{symbology} eq 'itf' } @{ $corpus->{cases} };
is(scalar @cases, 10, 'found all ITF cases');

sub input_for {
  my ($test_case) = @_;
  return $test_case->{input}{text} if exists $test_case->{input}{text};
  return $test_case->{input}{repeat}{text} x $test_case->{input}{repeat}{count};
}

sub modules_for {
  my ($data) = @_;
  return '1010' . join(q{}, map { $_->{binary_pattern} } @{ $pkg->encode_itf($data) }) . '11101';
}

sub runs_for {
  my ($bits) = @_;
  my @runs;
  my $previous;
  for my $bit (split //, $bits) {
    if (defined $previous && $bit eq $previous) {
      $runs[-1]++;
    } else {
      push @runs, 1;
      $previous = $bit;
    }
  }
  return \@runs;
}

for my $test_case (@cases) {
  subtest $test_case->{id} => sub {
    my $data = input_for($test_case);
    my $expected = $test_case->{expected};
    if (exists $expected->{error}) {
      my $caught = dies { $pkg->normalize_itf($data) };
      is($pkg->error_id($caught), $expected->{error}, 'stable error ID');
      return;
    }

    my $normalized = $pkg->normalize_itf($data);
    my $modules = modules_for($data);
    my $runs = runs_for($modules);
    if (exists $expected->{normalized}) {
      is($normalized, $expected->{normalized}, 'normalized');
      is($modules, $expected->{modules}, 'modules');
      is($runs, $expected->{run_lengths}, 'runs');
    } else {
      is(sha256_hex($normalized), $expected->{normalized_sha256}, 'normalized digest');
      is(length($modules), $expected->{module_count}, 'module count');
      is(sha256_hex($modules), $expected->{module_sha256}, 'module digest');
      is(scalar @$runs, $expected->{run_count}, 'run count');
      is(sha256_hex(encode_json($runs)), $expected->{run_lengths_sha256}, 'run digest');
    }
  };
}

done_testing;
