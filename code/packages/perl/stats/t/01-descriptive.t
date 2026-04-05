use Test2::V0;
use CodingAdventures::Stats qw(
    mean median mode variance standard_deviation
    stats_min stats_max stats_range
);

# ============================================================================
# Helper: compare floats within a tolerance (epsilon)
# ============================================================================
# Floating-point arithmetic is not exact. We use a small epsilon for
# comparisons.

sub approx {
    my ($got, $expected, $eps) = @_;
    $eps //= 1e-6;
    return abs($got - $expected) < $eps;
}

# ============================================================================
# Mean Tests
# ============================================================================

subtest "mean" => sub {
    # Parity test vector: mean([1,2,3,4,5]) = 3.0
    ok(approx(mean(1, 2, 3, 4, 5), 3.0), "mean of 1..5 is 3.0");

    # Worked example
    ok(approx(mean(2, 4, 4, 4, 5, 5, 7, 9), 5.0), "mean of worked example is 5.0");

    # Single value
    ok(approx(mean(42), 42.0), "mean of single value");

    # Negative values
    ok(approx(mean(-1, -2, -3), -2.0), "mean of negative values");

    # Empty input errors
    like(dies { mean() }, qr/requires at least one value/, "errors on empty input");
};

# ============================================================================
# Median Tests
# ============================================================================

subtest "median" => sub {
    ok(approx(median(1, 2, 3, 4, 5), 3.0), "median of odd-length list");
    ok(approx(median(2, 4, 4, 4, 5, 5, 7, 9), 4.5), "median of even-length list");
    ok(approx(median(7), 7.0), "median of single value");
    ok(approx(median(5, 1, 3), 3.0), "median sorts unsorted input");
    like(dies { median() }, qr/requires at least one value/, "errors on empty input");
};

# ============================================================================
# Mode Tests
# ============================================================================

subtest "mode" => sub {
    ok(approx(mode(2, 4, 4, 4, 5, 5, 7, 9), 4.0), "mode finds most frequent");
    ok(approx(mode(1, 2, 1, 2, 3), 1.0), "mode returns first occurrence on tie");
    ok(approx(mode(99), 99.0), "mode of single value");
    like(dies { mode() }, qr/requires at least one value/, "errors on empty input");
};

# ============================================================================
# Variance Tests
# ============================================================================

subtest "variance" => sub {
    # Parity test vector: sample variance = 4.571428571428571
    ok(
        approx(variance([2, 4, 4, 4, 5, 5, 7, 9]), 4.571428571428571),
        "sample variance of worked example"
    );

    # Parity test vector: population variance = 4.0
    ok(
        approx(variance([2, 4, 4, 4, 5, 5, 7, 9], population => 1), 4.0),
        "population variance of worked example"
    );

    # Two values
    ok(approx(variance([1, 3]), 2.0), "sample variance of two values");

    # Error on single value for sample
    like(
        dies { variance([42]) },
        qr/sample variance requires at least two values/,
        "errors on single value for sample"
    );

    # Population variance of single value
    ok(
        approx(variance([42], population => 1), 0.0),
        "population variance of single value"
    );
};

# ============================================================================
# Standard Deviation Tests
# ============================================================================

subtest "standard_deviation" => sub {
    ok(
        approx(standard_deviation([2, 4, 4, 4, 5, 5, 7, 9]), 2.13809, 1e-4),
        "sample standard deviation"
    );
    ok(
        approx(standard_deviation([2, 4, 4, 4, 5, 5, 7, 9], population => 1), 2.0),
        "population standard deviation"
    );
};

# ============================================================================
# Min / Max / Range Tests
# ============================================================================

subtest "stats_min" => sub {
    ok(approx(stats_min(2, 4, 4, 4, 5, 5, 7, 9), 2.0), "min of worked example");
    ok(approx(stats_min(3, -1, 7), -1.0), "min with negative values");
    like(dies { stats_min() }, qr/requires at least one value/, "errors on empty");
};

subtest "stats_max" => sub {
    ok(approx(stats_max(2, 4, 4, 4, 5, 5, 7, 9), 9.0), "max of worked example");
    like(dies { stats_max() }, qr/requires at least one value/, "errors on empty");
};

subtest "stats_range" => sub {
    ok(approx(stats_range(2, 4, 4, 4, 5, 5, 7, 9), 7.0), "range of worked example");
    ok(approx(stats_range(5, 5, 5), 0.0), "range of identical values");
};

done_testing;
