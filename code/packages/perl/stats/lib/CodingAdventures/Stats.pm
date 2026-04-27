package CodingAdventures::Stats;

# ============================================================================
# CodingAdventures::Stats
# ============================================================================
#
# Descriptive statistics, frequency analysis, and cryptanalysis helpers.
#
# Overview
# --------
# This module provides three categories of pure functions:
#
# 1. **Descriptive statistics** (mean, median, mode, variance, standard
#    deviation, min, max, range) -- operate on arrays of numbers.
# 2. **Frequency analysis** (frequency_count, frequency_distribution,
#    chi_squared, chi_squared_text) -- operate on text strings or arrays.
# 3. **Cryptanalysis helpers** (index_of_coincidence, entropy,
#    ENGLISH_FREQUENCIES) -- tools for breaking classical ciphers.
#
# Design Principles
# -----------------
# - **Pure functions.** No side effects, no mutation of inputs.
# - **No external dependencies.** Pure math only (uses only POSIX).
# - **Population vs sample.** Variance and standard deviation default to
#   sample (Bessel-corrected, dividing by n-1). Pass population => 1
#   for population statistics (dividing by n).
#
# Worked Example
# --------------
# Given the dataset (2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0):
#
#   mean     = (2+4+4+4+5+5+7+9) / 8 = 40 / 8 = 5.0
#   median   = average of 4th and 5th values (sorted) = (4+5)/2 = 4.5
#   mode     = 4.0 (appears 3 times, more than any other)
#   variance = sample: sum of squared deviations / (n-1)
#            = [(2-5)^2 + (4-5)^2 + ... + (9-5)^2] / 7
#            = 32 / 7 = 4.571428...
#   std_dev  = sqrt(4.571428...) = 2.138...
#   min      = 2.0
#   max      = 9.0
#   range    = 9.0 - 2.0 = 7.0
# ============================================================================

use strict;
use warnings;
use POSIX qw(floor);
use Exporter 'import';

our $VERSION = '0.1.0';
our @EXPORT_OK = qw(
    mean median mode variance standard_deviation
    stats_min stats_max stats_range
    frequency_count frequency_distribution
    chi_squared chi_squared_text
    index_of_coincidence entropy
    ENGLISH_FREQUENCIES
);

# ============================================================================
# English Letter Frequencies
# ============================================================================
#
# Standard frequencies of each letter in English text, derived from large-
# corpus analysis. Used as the expected distribution for chi-squared tests
# in frequency analysis and cryptanalysis.
#
#   A = 0.08167  (about 8.2% of English text)
#   E = 0.12702  (the most common letter at ~12.7%)
#   Z = 0.00074  (the least common letter at ~0.07%)
# ============================================================================

use constant ENGLISH_FREQUENCIES => {
    A => 0.08167, B => 0.01492, C => 0.02782, D => 0.04253,
    E => 0.12702, F => 0.02228, G => 0.02015, H => 0.06094,
    I => 0.06966, J => 0.00153, K => 0.00772, L => 0.04025,
    M => 0.02406, N => 0.06749, O => 0.07507, P => 0.01929,
    Q => 0.00095, R => 0.05987, S => 0.06327, T => 0.09056,
    U => 0.02758, V => 0.00978, W => 0.02360, X => 0.00150,
    Y => 0.01974, Z => 0.00074,
};

# ============================================================================
# Descriptive Statistics
# ============================================================================

# Arithmetic mean: sum of all values divided by the count.
#
# The mean is the most common measure of central tendency. It uses every
# data point, which makes it sensitive to outliers.
#
# Formula: mean = (x_1 + x_2 + ... + x_n) / n
#
# Example:
#   mean([1, 2, 3, 4, 5]) = 15 / 5 = 3.0
sub mean {
    my @values = @_;
    die "mean requires at least one value\n" unless @values;
    my $total = 0;
    $total += $_ for @values;
    return $total / scalar @values;
}

# Median: the middle value when sorted.
#
# The median splits the dataset in half -- 50% of values are below it and
# 50% are above. Unlike the mean, the median is robust to outliers.
#
# For odd-length lists, the median is the middle element.
# For even-length lists, it is the average of the two middle elements.
#
# Examples:
#   median(1, 2, 3, 4, 5)  -> 3.0   (middle of 5 elements)
#   median(1, 2, 3, 4)     -> 2.5   (average of 2 and 3)
sub median {
    my @values = @_;
    die "median requires at least one value\n" unless @values;
    my @sorted = sort { $a <=> $b } @values;
    my $n = scalar @sorted;
    my $mid = floor($n / 2);

    # Odd length: single middle element
    if ($n % 2 == 1) {
        return $sorted[$mid];
    }

    # Even length: average of two middle elements
    return ($sorted[$mid - 1] + $sorted[$mid]) / 2.0;
}

# Mode: the most frequently occurring value.
#
# If multiple values share the highest frequency, the one that appears
# first in the original list wins. This "first occurrence" tie-breaking
# rule ensures deterministic results across all languages in the repo.
#
# How it works:
# 1. Count occurrences of each value.
# 2. Find the maximum count.
# 3. Return the first value in the original list that has that count.
sub mode {
    my @values = @_;
    die "mode requires at least one value\n" unless @values;

    # Step 1: count occurrences
    my %counts;
    $counts{$_}++ for @values;

    # Step 2: find the maximum frequency
    my $max_count = 0;
    for my $c (values %counts) {
        $max_count = $c if $c > $max_count;
    }

    # Step 3: return the first value with that frequency
    for my $v (@values) {
        return $v if $counts{$v} == $max_count;
    }
}

# Variance: average of squared deviations from the mean.
#
# Variance measures how spread out the data is. A variance of 0 means
# all values are identical.
#
# Two flavors:
#   - Sample variance (default, population=0): divides by n-1.
#     Used when your data is a sample from a larger population.
#   - Population variance (population=1): divides by n.
#     Used when your data IS the entire population.
#
# Formula:
#   variance = Sum((x_i - mean)^2) / d
#   where d = n (population) or n-1 (sample)
#
# Usage:
#   variance(@values)            # sample (default)
#   variance(@values, population => 1)  # pass hashref with population key
sub variance {
    my ($values_ref, %opts) = @_;
    my @values = @$values_ref;
    my $population = $opts{population} || 0;

    die "variance requires at least one value\n" unless @values;
    my $n = scalar @values;
    if (!$population && $n == 1) {
        die "sample variance requires at least two values\n";
    }

    my $m = mean(@values);

    # Sum of squared deviations
    my $squared_diffs = 0;
    $squared_diffs += ($_ - $m) ** 2 for @values;

    my $divisor = $population ? $n : ($n - 1);
    return $squared_diffs / $divisor;
}

# Standard deviation: square root of variance.
#
# The standard deviation has the same units as the original data.
#
# For a normal distribution:
#   - ~68% of data falls within 1 standard deviation of the mean
#   - ~95% falls within 2 standard deviations
#   - ~99.7% falls within 3 standard deviations
sub standard_deviation {
    my ($values_ref, %opts) = @_;
    return sqrt(variance($values_ref, %opts));
}

# Minimum value in the dataset.
sub stats_min {
    my @values = @_;
    die "min requires at least one value\n" unless @values;
    my $result = $values[0];
    for my $v (@values) {
        $result = $v if $v < $result;
    }
    return $result;
}

# Maximum value in the dataset.
sub stats_max {
    my @values = @_;
    die "max requires at least one value\n" unless @values;
    my $result = $values[0];
    for my $v (@values) {
        $result = $v if $v > $result;
    }
    return $result;
}

# Range: the difference between the maximum and minimum values.
#
# The range is the simplest measure of spread. It only looks at the two
# extreme values, so it is very sensitive to outliers.
#
# Formula: range = max - min
sub stats_range {
    my @values = @_;
    return stats_max(@values) - stats_min(@values);
}

# ============================================================================
# Frequency Analysis
# ============================================================================
#
# These functions analyze the frequency distribution of letters in text.
# They are the foundation of classical cipher analysis.
# ============================================================================

# Count each letter in text (case-insensitive, A-Z only).
#
# Non-alphabetic characters are ignored. Returns a hashref mapping
# uppercase letters to their integer counts.
#
# Example:
#   frequency_count("Hello!") -> {H => 1, E => 1, L => 2, O => 1, ...}
sub frequency_count {
    my ($text) = @_;
    my %counts;

    # Initialize all 26 letters to 0
    for my $i (0 .. 25) {
        $counts{chr(65 + $i)} = 0;
    }

    my $upper = uc($text);
    for my $ch (split //, $upper) {
        if ($ch ge 'A' && $ch le 'Z') {
            $counts{$ch}++;
        }
    }

    return \%counts;
}

# Frequency distribution: proportion of each letter in the text.
#
# Returns a hashref mapping uppercase letters to float proportions
# (counts / total letters).
sub frequency_distribution {
    my ($text) = @_;
    my $counts = frequency_count($text);

    # Sum total alphabetic characters
    my $total = 0;
    $total += $_ for values %$counts;

    my %dist;
    for my $letter (keys %$counts) {
        $dist{$letter} = $total > 0 ? ($counts->{$letter} / $total) : 0.0;
    }

    return \%dist;
}

# Chi-squared goodness-of-fit test for parallel arrays.
#
# The chi-squared statistic measures how well observed data fits an
# expected distribution:
#
#   chi2 = Sum((O_i - E_i)^2 / E_i)
#
# Example:
#   chi_squared([10, 20, 30], [20, 20, 20])
#   = (10-20)^2/20 + (20-20)^2/20 + (30-20)^2/20
#   = 5.0 + 0.0 + 5.0 = 10.0
sub chi_squared {
    my ($observed_ref, $expected_ref) = @_;
    my @observed = @$observed_ref;
    my @expected = @$expected_ref;

    die "observed and expected must have same length\n"
        unless scalar @observed == scalar @expected;
    die "arrays must not be empty\n" unless @observed;

    my $chi2 = 0;
    for my $i (0 .. $#observed) {
        if ($expected[$i] > 1e-10) {
            my $diff = $observed[$i] - $expected[$i];
            $chi2 += ($diff * $diff) / $expected[$i];
        }
    }

    return $chi2;
}

# Chi-squared test of text against an expected frequency table.
#
# Combines frequency_count with chi_squared. Counts the letters in the
# text, then compares those counts against the expected frequencies
# scaled to the text length.
sub chi_squared_text {
    my ($text, $expected_freq) = @_;
    my $counts = frequency_count($text);

    # Total alphabetic characters
    my $total = 0;
    $total += $_ for values %$counts;

    return 0 if $total == 0;

    my $chi2 = 0;
    for my $i (0 .. 25) {
        my $letter = chr(65 + $i);
        my $observed = $counts->{$letter} || 0;
        my $expected = $total * ($expected_freq->{$letter} || 0);
        if ($expected > 1e-10) {
            my $diff = $observed - $expected;
            $chi2 += ($diff * $diff) / $expected;
        }
    }

    return $chi2;
}

# ============================================================================
# Cryptanalysis Helpers
# ============================================================================

# Index of Coincidence: probability that two random letters match.
#
# Formula:
#   IC = Sum(n_i * (n_i - 1)) / (N * (N - 1))
#
# Reference values:
#   - English text:  IC ~ 0.0667
#   - Random text:   IC ~ 0.0385 (1/26)
#   - "AABB":        IC = (2*1 + 2*1) / (4*3) = 4/12 = 0.333...
sub index_of_coincidence {
    my ($text) = @_;
    my $counts = frequency_count($text);

    # N = total alphabetic characters
    my $n = 0;
    $n += $_ for values %$counts;

    # Need at least 2 letters
    return 0.0 if $n < 2;

    # Sum(n_i * (n_i - 1))
    my $numerator = 0;
    for my $c (values %$counts) {
        $numerator += $c * ($c - 1);
    }

    return $numerator / ($n * ($n - 1));
}

# Shannon entropy of the letter distribution in text.
#
# Formula:
#   H = -Sum(p_i * log2(p_i))
#
# Reference values:
#   - 26 equal letters: H = log2(26) ~ 4.700 bits
#   - English text:     H ~ 4.1 bits
#   - "AAAA":           H = 0.0 bits (no surprise)
sub entropy {
    my ($text) = @_;
    my $dist = frequency_distribution($text);

    my $h = 0;
    for my $p (values %$dist) {
        if ($p > 0) {
            $h -= $p * (log($p) / log(2));
        }
    }

    return $h;
}

1;
