use strict;
use warnings;
use Test2::V0;
use lib 'lib';
use CodingAdventures::RankStatistics qw(median median_interval exact_median_rank);
use POSIX qw(nextafter);

subtest 'median sorts a copy and rounds the midpoint safely' => sub {
    my $values = [4, 1, 3, 2];
    is(median($values), 2.5, 'even median');
    is($values, [4, 1, 3, 2], 'caller input unchanged');
    my $d = nextafter(0.0, 1.0);
    is(median([$d, 2 * $d]), 2 * $d, 'positive subnormal tie rounds to even');
    is(median([-2 * $d, -$d]), -2 * $d, 'negative subnormal tie rounds to even');
    my $normal = 2.2250738585072014e-308;
    is(median([nextafter($normal, 0), $normal]), $normal, 'normal boundary');
    is(median([-1.7976931348623157e308, 1.7976931348623157e308]), 0, 'opposite maximum finite values');
    is(median([1.7976931348623157e308, 1.7976931348623157e308]), 1.7976931348623157e308, 'same-sign maximum values');
    like dies { median([]) }, qr/empty/;
    like dies { median([1, 'NaN']) }, qr/finite/;
    like dies { median([1, 'Inf']) }, qr/finite/;
};

subtest 'exact cutoff matches independent sign enumeration' => sub {
    for my $n (1 .. 12) {
        for my $confidence (1 .. 99) {
            my $want = 0;
            for my $k (1 .. int($n / 2)) {
                my $tail = 0;
                for my $mask (0 .. (1 << $n) - 1) {
                    my $bits = 0;
                    my $work = $mask;
                    while ($work) { $bits += $work & 1; $work >>= 1; }
                    $tail++ if $bits < $k;
                }
                $want = $k if 200 * $tail <= (100 - $confidence) * (1 << $n);
                last if 200 * $tail > (100 - $confidence) * (1 << $n);
            }
            is(exact_median_rank($n, $confidence), $want, "n=$n confidence=$confidence");
        }
    }
};

subtest 'wide cutoffs match frozen Python reference vectors' => sub {
    my @cases = (
        [64, 95, 23, 40], [65, 99, 21, 43], [127, 95, 51, 75],
        [128, 95, 52, 75], [256, 99, 106, 149], [1000, 95, 468, 531],
        [10000, 50, 4965, 5034], [10000, 95, 4901, 5098], [10000, 99, 4870, 5129],
    );
    for my $case (@cases) {
        my ($n, $confidence, $low_rank, $high_rank) = @$case;
        is(exact_median_rank($n, $confidence), $low_rank + 1, "lower endpoint rank n=$n confidence=$confidence");
        my @values = 0 .. $n - 1;
        is(median_interval(\@values, $confidence), [$low_rank, $high_rank], "selected endpoints n=$n confidence=$confidence");
    }
};

subtest 'generated finite arrays retain order-statistic bounds' => sub {
    my $seed = 0x5eed;
    for my $trial (1 .. 300) {
        my $n = 1 + (($seed = (1103515245 * $seed + 12345) % 2147483648) % 64);
        my @values;
        for (1 .. $n) { $seed = (1103515245 * $seed + 12345) % 2147483648; push @values, ($seed % 2001) - 1000; }
        my @original = @values;
        my $m = median(\@values);
        my $ci = median_interval(\@values, 90);
        ok($m >= -1000 && $m <= 1000, "trial $trial median in range");
        ok(!defined($ci) || ($ci->[0] <= $m && $m <= $ci->[1]), "trial $trial interval contains median");
        is(\@values, \@original, "trial $trial unchanged");
    }
};

done_testing;
