use strict;
use warnings;
use feature 'state';
use Test2::V0;
use JSON::PP ();
use File::Spec;
use lib 'lib', '../rank-statistics/lib', '../grammar-tools/lib', '../json-lexer/lib', '../json-parser/lib';
use CodingAdventures::Microbenchmark qw(defaults analyze collect report_from_json report_to_json indexed_batch iterated_batch array_cli monotonic_clock);

my $fixture_path = File::Spec->catfile('testdata', 'microbenchmark-pairs-v1.json');
open my $fixture_fh, '<:raw', $fixture_path or die "open fixture: $!";
local $/; my $fixture_bytes = <$fixture_fh>; close $fixture_fh;
my $fixture = JSON::PP->new->utf8->decode($fixture_bytes);

subtest 'fixture remains canonical and all analysis cases conform' => sub {
    if (-f '../../../specs/fixtures/microbenchmark-pairs-v1.json') {
        open my $canonical_fh, '<:raw', '../../../specs/fixtures/microbenchmark-pairs-v1.json' or die "open canonical fixture: $!";
        local $/; my $canonical = <$canonical_fh>; close $canonical_fh;
        is($fixture_bytes, $canonical, 'local fixture byte-equals canonical');
    } else {
        note 'canonical fixture comparison is skipped outside the monorepo; local cases remain covered';
    }
    for my $case (@{$fixture->{cases}}) {
        my @pairs = map { { baseline_ns => $case->{baseline_ns}, candidate_ns => $_, order => 'baseline-first' } } @{$case->{candidate_ns}};
        my $result = analyze(\@pairs, $fixture->{confidence_percent}, $fixture->{minimum_factor});
        is($result->{ratio}, $case->{ratio}, "$case->{name} ratio");
        is($result->{verdict}, $case->{verdict}, "$case->{name} verdict");
        is($result->{interval}, $case->{interval}, "$case->{name} interval");
    }
};

subtest 'collector calibrates, balances and reproduces its schedule' => sub {
    my $ticks = 0;
    my $clock = sub { my $old = $ticks; $ticks += 100; return $old; };
    my $cfg = { pairs => 5, confidence_percent => 95, minimum_factor => 1.05, seed => -5, target_ns => 100, max_batch => 8 };
    my $base = sub { 7 * $_[0] };
    my $candidate = sub { 7 * $_[0] };
    my $first = collect($base, $candidate, 7, $cfg, $clock);
    $ticks = 0;
    my $second = collect($base, $candidate, 7, $cfg, $clock);
    is($first->{batch_size}, 1, 'calibrated common size');
    is([map { $_->{order} } @{$first->{pairs}}], [map { $_->{order} } @{$second->{pairs}}], 'local seed reproduces order');
    is((scalar grep { $_->{order} eq 'baseline-first' } @{$first->{pairs}}), 3, 'odd schedule gives baseline first the extra pair');
    is(scalar @{$first->{pairs}}, 5, 'all pairs retained');
    like dies { collect($base, $candidate, 7, { %$cfg, target_ns => 1000, max_batch => 1 }, sub { 0 }) }, qr/calibration limit/;
    like dies { collect(sub { 8 * $_[0] }, $candidate, 7, $cfg, $clock) }, qr/checksum mismatch/;
    like dies { collect(sub { die 'callback fail' }, $candidate, 7, $cfg, $clock) }, qr/callback fail/;
    my $back = 10;
    like dies { collect($base, $candidate, 7, $cfg, sub { $back-- }) }, qr/regressed/;
    like dies { collect($base, $candidate, 7, $cfg, sub { -1 }) }, qr/(?:non-negative|minimum)/;
    my @zero_measured = (0, 1, 2, 3, 4, 4);
    like dies { collect($base, $candidate, 7, { %$cfg, pairs => 1, target_ns => 1, max_batch => 1 }, sub { shift @zero_measured }) }, qr/positive/;
    my $max = 9_223_372_036_854_775_807;
    my $max_callback = sub { $_[0] == 1 ? $max : 0 };
    my $over_clock = 0;
    like dies { collect($max_callback, $max_callback, $max, { %$cfg, pairs => 1, target_ns => 200, max_batch => 2 }, sub { my $old = $over_clock; $over_clock += 100; return $old }) }, qr/overflow/;
};

subtest 'strict report replay preserves signed 64-bit integer tokens' => sub {
    my $max = 9_223_372_036_854_775_807;
    my $min = -9_223_372_036_854_775_808;
    my $pair = { baseline_ns => $max, candidate_ns => $max, order => 'baseline-first' };
    my $report = { config => { pairs => 1, confidence_percent => 95, minimum_factor => 1.05, seed => $min, target_ns => 1, max_batch => $max },
        batch_size => $max, pairs => [$pair], analysis => analyze([$pair], 95, 1.05),
        workload => 'indexed-array-sum-4096', invocation => ['bench'], revision => 'rev', runtime => 'perl', os => 'win32', arch => 'x64' };
    my $encoded = report_to_json($report);
    like($encoded, qr/-9223372036854775808/, 'min integer emitted exactly');
    like($encoded, qr/9223372036854775807/, 'max integer emitted exactly');
    my $decoded = report_from_json($encoded);
    is($decoded->{config}{seed}, $min, 'min seed round trips');
    is($decoded->{pairs}[0]{baseline_ns}, $max, 'max duration round trips');
    is($decoded->{batch_size}, $max, 'max batch size round trips');
    my $third_pair = { baseline_ns => 3, candidate_ns => 1, order => 'baseline-first' };
    my $third = { config => { pairs => 1, confidence_percent => 95, minimum_factor => 1.05, seed => 0, target_ns => 1, max_batch => 1 },
        batch_size => 1, pairs => [$third_pair], analysis => analyze([$third_pair], 95, 1.05) };
    my $third_json = report_to_json($third);
    like($third_json, qr/0\.33333333333333331/, 'binary64 ratio emitted with 17 significant digits');
    is(report_from_json($third_json)->{analysis}{ratio}, 1 / 3, 'binary64 ratio replays exactly');
    like dies { report_from_json($encoded =~ s/9223372036854775807/9223372036854775808/r) }, qr/signed 64-bit/;
    like dies { report_from_json($encoded =~ s/"seed"\s*:\s*-9223372036854775808/"seed": 1.0/r) }, qr/integer/;
    like dies { report_from_json($encoded =~ s/"indexed-array-sum-4096"/"other"/r) }, qr/schema|workload/;
    like dies { report_from_json($encoded =~ s/("analysis"\s*:)/"unknown": true, $1/r) }, qr/unknown/;
    like dies { report_from_json($encoded . '{}') }, qr/trailing|invalid|JSON/;
    like dies { report_from_json('{"x":1,"x":2}') }, qr/duplicate/;
    like dies { report_from_json('{"x":1,"\\u0078":2}') }, qr/duplicate/;
    like dies { report_from_json($encoded =~ s/("config"\s*:\s*\{)/$1"seed": 1,/r) }, qr/duplicate/;
    like dies { report_from_json($encoded =~ s/"revision"\s*:\s*"rev"/"revision": null/r) }, qr/(?:metadata|revision)/;
    like dies { report_from_json($encoded =~ s/"baseline_ns"\s*:\s*9223372036854775807/"baseline_ns": 1.5/r) }, qr/integer/;
    my $without_metadata = { %$report };
    delete @{$without_metadata}{qw(workload invocation revision runtime os arch)};
    ok(report_from_json(report_to_json($without_metadata)), 'complete metadata absence is valid');
    my @large_pairs = map { { baseline_ns => 1, candidate_ns => 1, order => $_ % 2 ? 'baseline-first' : 'candidate-first' } } 1 .. 10_000;
    my $large = { config => { pairs => 10_000, confidence_percent => 95, minimum_factor => 1.05, seed => 0, target_ns => 1, max_batch => 1 },
        batch_size => 1, pairs => \@large_pairs, analysis => analyze(\@large_pairs, 95, 1.05) };
    is(scalar @{report_from_json(report_to_json($large))->{pairs}}, 10_000, 'valid maximum-pair report fits parser caps');
    my $bad_balance = { %$report, pairs => [ { %$pair, order => 'candidate-first' } ] };
    like dies { report_from_json(report_to_json($bad_balance)) }, qr/unbalanced/;
};

subtest 'array helpers use the same actual input and checked counts' => sub {
    my $values = [2, 5, 8];
    is(indexed_batch($values, 1), 15, 'indexed checksum');
    is(iterated_batch($values, 1), 15, 'iterator checksum');
    is(indexed_batch($values, 0), 0, 'zero operations allowed');
    is(iterated_batch($values, 0), 0, 'zero iterator operations allowed');
    like dies { indexed_batch($values, -1) }, qr/non-negative/;
    like dies { iterated_batch($values, -1) }, qr/non-negative/;
    like dies { indexed_batch([9_223_372_036_854_775_807], 2) }, qr/overflow/;
    my $mutable = [1, 2, 3];
    my $expected = $mutable->[0] + $mutable->[1] + $mutable->[2];
    $mutable->[0] = 50;
    is(indexed_batch($mutable, 1), 55, 'uses mutated caller-owned array, not a regenerated formula');
    is(iterated_batch($mutable, 1), 55, 'iteration uses same array snapshot');
    isnt($expected, 55, 'mutation changed the expected sum');
};

subtest 'CLI metadata, blank revision and JSON output failures' => sub {
    local $ENV{GIT_REVISION} = '  ';
    local $ENV{GITHUB_SHA} = 'fallback-sha';
    my $tick = 0;
    my $clock = sub { my $old = $tick; $tick += 1_000_000; $old };
    my $output = '';
    open my $out, '>:raw', \$output or die $!;
    my $err_text = '';
    open my $err, '>', \$err_text or die $!;
    is(array_cli(['--pairs', '40', '--target-ns', '1', '--max-batch', '1'], $out, $err, $clock), 0, 'CLI succeeds with fallback revision');
    my $decoded = report_from_json($output);
    is($decoded->{revision}, 'fallback-sha', 'blank GIT_REVISION falls through');
    is(scalar @{$decoded->{pairs}}, 40, 'native CLI records 40 positive pairs');
    like dies { array_cli(['--revision', ''], $out, $err, $clock) }, qr/blank/;
    my $help = '';
    open my $help_fh, '>', \$help or die $!;
    is(array_cli(['--help'], $out, $help_fh, $clock), 0, 'help works');
    like($help, qr/Usage:/, 'help text');
};

done_testing;
