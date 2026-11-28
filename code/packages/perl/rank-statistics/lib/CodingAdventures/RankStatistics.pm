package CodingAdventures::RankStatistics;

# Reusable sample medians and exact distribution-free median intervals.
#
# The median consumes finite binary64 observations and never sorts the
# caller's array. Even samples use sum-then-half to preserve subnormal
# rounding, with a split fallback only when the sum overflows.

use strict;
use warnings;
use Exporter qw(import);
use Math::BigInt;
use POSIX qw(isfinite);
use Scalar::Util qw(looks_like_number);

our $VERSION = '0.1.0';
our @EXPORT_OK = qw(median median_interval exact_median_rank);

sub _sorted_finite {
    my ($values) = @_;
    die "sample must be an array reference" unless ref($values) eq 'ARRAY';
    die "sample must not be empty" unless @$values;
    my @copy;
    for my $value (@$values) {
        die "sample values must be finite numbers"
            if !defined($value) || ref($value) || !looks_like_number($value);
        my $number = 0 + $value;
        die "sample values must be finite numbers" unless isfinite($number);
        push @copy, $number;
    }
    return sort { $a <=> $b } @copy;
}

sub _midpoint {
    my ($low, $high) = @_;
    my $sum = $low + $high;
    return isfinite($sum) ? $sum / 2 : $low / 2 + $high / 2;
}

sub median {
    my ($values) = @_;
    my @ordered = _sorted_finite($values);
    my $middle = int(@ordered / 2);
    return $ordered[$middle] if @ordered % 2;
    return _midpoint($ordered[$middle - 1], $ordered[$middle]);
}

sub exact_median_rank {
    my ($n, $confidence) = @_;
    die "sample size must be a positive integer" unless defined($n) && $n =~ /\A[1-9][0-9]*\z/;
    die "confidence must be an integer from 1 through 99"
        unless defined($confidence) && $confidence =~ /\A(?:[1-9]|[1-9][0-9])\z/ && $confidence <= 99;
    my $max_tail = Math::BigInt->new(100 - $confidence)->blsft($n);
    my $coefficient = Math::BigInt->bone;
    my $tail = Math::BigInt->bzero;
    my $rank = 0;
    for my $j (0 .. int($n / 2)) {
        $tail->badd($coefficient);
        last if $tail->copy->bmul(200)->bcmp($max_tail) > 0;
        $rank = $j + 1;
        $coefficient->bmul($n - $j)->bdiv($j + 1);
    }
    return $rank;
}

sub median_interval {
    my ($values, $confidence) = @_;
    die "confidence must be an integer from 1 through 99"
        unless defined($confidence) && $confidence =~ /\A(?:[1-9]|[1-9][0-9])\z/ && $confidence <= 99;
    my @ordered = _sorted_finite($values);
    my $rank = exact_median_rank(scalar @ordered, $confidence);
    return undef if $rank == 0;
    return [$ordered[$rank - 1], $ordered[@ordered - $rank]];
}

1;
