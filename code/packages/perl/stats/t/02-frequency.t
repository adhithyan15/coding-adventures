use Test2::V0;
use CodingAdventures::Stats qw(
    frequency_count frequency_distribution
    chi_squared chi_squared_text
    index_of_coincidence entropy
    ENGLISH_FREQUENCIES
);

# ============================================================================
# Helper: compare floats within a tolerance
# ============================================================================

sub approx {
    my ($got, $expected, $eps) = @_;
    $eps //= 1e-6;
    return abs($got - $expected) < $eps;
}

# ============================================================================
# Frequency Count Tests
# ============================================================================

subtest "frequency_count" => sub {
    my $counts = frequency_count("Hello");
    is($counts->{H}, 1, "H appears once");
    is($counts->{E}, 1, "E appears once");
    is($counts->{L}, 2, "L appears twice");
    is($counts->{O}, 1, "O appears once");

    # Non-alphabetic ignored
    my $counts2 = frequency_count("A1B2C3!!!");
    is($counts2->{A}, 1, "A counted");
    is($counts2->{B}, 1, "B counted");
    is($counts2->{C}, 1, "C counted");
    is($counts2->{D}, 0, "D is zero");

    # Empty string
    my $counts3 = frequency_count("");
    is($counts3->{A}, 0, "empty string has zero counts");
};

# ============================================================================
# Frequency Distribution Tests
# ============================================================================

subtest "frequency_distribution" => sub {
    my $dist = frequency_distribution("AABB");
    ok(approx($dist->{A}, 0.5), "A proportion is 0.5");
    ok(approx($dist->{B}, 0.5), "B proportion is 0.5");
    ok(approx($dist->{C}, 0.0), "C proportion is 0.0");

    my $dist2 = frequency_distribution("");
    ok(approx($dist2->{A}, 0.0), "empty string proportions are 0");
};

# ============================================================================
# Chi-Squared Tests
# ============================================================================

subtest "chi_squared" => sub {
    # Parity test vector: 10.0
    ok(
        approx(chi_squared([10, 20, 30], [20, 20, 20]), 10.0),
        "chi-squared of [10,20,30] vs [20,20,20]"
    );

    # Identical distributions
    ok(
        approx(chi_squared([10, 10, 10], [10, 10, 10]), 0.0),
        "chi-squared of identical distributions"
    );

    # Mismatched lengths
    like(
        dies { chi_squared([1, 2], [1, 2, 3]) },
        qr/same length/,
        "errors on mismatched lengths"
    );
};

# ============================================================================
# Chi-Squared Text Tests
# ============================================================================

subtest "chi_squared_text" => sub {
    my $result = chi_squared_text("AABB", ENGLISH_FREQUENCIES);
    ok($result > 0, "chi-squared of text against English is positive");

    ok(approx(chi_squared_text("", ENGLISH_FREQUENCIES), 0.0), "empty text returns 0");
    ok(approx(chi_squared_text("12345!!!", ENGLISH_FREQUENCIES), 0.0), "non-alpha text returns 0");
};

# ============================================================================
# Index of Coincidence Tests
# ============================================================================

subtest "index_of_coincidence" => sub {
    # Parity test vector: IC("AABB") = 4/12 = 0.333...
    ok(
        approx(index_of_coincidence("AABB"), 1.0 / 3.0),
        "IC of AABB is 0.333..."
    );

    ok(approx(index_of_coincidence("A"), 0.0), "IC of single letter is 0");
    ok(approx(index_of_coincidence(""), 0.0), "IC of empty string is 0");

    # All same letter -> IC = 1.0
    ok(
        approx(index_of_coincidence("AAAA"), 1.0),
        "IC of same letter is 1.0"
    );

    # Pangram has near-uniform distribution so IC is low; just check positive
    my $ic = index_of_coincidence("THEQUICKBROWNFOXJUMPSOVERTHELAZYDOG");
    ok($ic > 0, "IC of pangram is positive");
};

# ============================================================================
# Entropy Tests
# ============================================================================

subtest "entropy" => sub {
    # Parity test vector: log2(26) ~ 4.700
    my $uniform = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    ok(
        approx(entropy($uniform), log(26) / log(2), 0.01),
        "entropy of uniform 26 letters"
    );

    ok(approx(entropy("AAAA"), 0.0), "entropy of single letter");

    # H("AABB") = 1.0
    ok(approx(entropy("AABB"), 1.0, 0.01), "entropy of two-letter uniform");

    ok(approx(entropy(""), 0.0), "entropy of empty string");
};

# ============================================================================
# ENGLISH_FREQUENCIES Tests
# ============================================================================

subtest "ENGLISH_FREQUENCIES" => sub {
    my $freqs = ENGLISH_FREQUENCIES;
    is(scalar keys %$freqs, 26, "has 26 entries");

    my $total = 0;
    $total += $_ for values %$freqs;
    ok(approx($total, 1.0, 0.01), "sums to approximately 1.0");

    # E is the most common letter
    my $max_letter = 'A';
    my $max_freq = 0;
    for my $letter (keys %$freqs) {
        if ($freqs->{$letter} > $max_freq) {
            $max_freq = $freqs->{$letter};
            $max_letter = $letter;
        }
    }
    is($max_letter, 'E', "E is the most common letter");
};

done_testing;
