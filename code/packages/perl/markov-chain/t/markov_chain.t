use strict;
use warnings;
use Test::More;

use CodingAdventures::MarkovChain;

# ============================================================================
# DT28 Markov Chain — test suite
# ============================================================================
#
# These tests cover all 10 spec-required cases from the DT28 specification,
# plus a load test and a transition-matrix inspection test.
#
# Test naming follows: "Test N: <what is being verified>"
#
# Quick glossary for readers new to Markov Chains:
#   - state:    a value the chain can be in (e.g., 'A', 'B', 'C').
#   - context:  the last k states (for order-k chains); a k-gram key.
#   - train:    feed observed sequences to estimate transition probabilities.
#   - smoothing: add a small count to every possible transition before
#                normalising, so no transition has exactly zero probability.

# ============================================================================
# Test 0: module loads and version is defined
# ============================================================================

ok( eval { require CodingAdventures::MarkovChain; 1 },
    'CodingAdventures::MarkovChain loads' );

ok( CodingAdventures::MarkovChain->VERSION, 'has a VERSION' );

# ============================================================================
# Test 1: Construction — new() creates an empty chain with 0 states
# ============================================================================

{
    my $chain = CodingAdventures::MarkovChain->new;
    my $states = $chain->states;
    is( scalar @$states, 0,  'Test 1: new chain has 0 states' );
    ok( defined $chain,      'Test 1: new returns a blessed reference' );
}

# ============================================================================
# Test 2: Train single pair — probability(A, B) == 1.0 after [A, B]
# ============================================================================

{
    my $chain = CodingAdventures::MarkovChain->new;
    $chain->train(['A', 'B']);

    # When we only ever see A followed by B, the probability must be 1.0.
    my $p = $chain->probability('A', 'B');
    ok( abs($p - 1.0) < 1e-9, 'Test 2: P(A->B) == 1.0 after training [A,B]' );
}

# ============================================================================
# Test 3: Train sequence — probability estimates from [A,B,A,C,A,B,B,A]
#
# Count transitions:
#   A->B: 2,  A->C: 1   (3 departures from A)
#   B->A: 2,  B->B: 1   (3 departures from B)
#   C->A: 1              (1 departure from C)
#
# Normalised:
#   P(A->B) = 2/3 ≈ 0.667
#   P(A->C) = 1/3 ≈ 0.333
#   P(B->A) = 2/3 ≈ 0.667
#   P(B->B) = 1/3 ≈ 0.333
# ============================================================================

{
    my $chain = CodingAdventures::MarkovChain->new;
    $chain->train(['A', 'B', 'A', 'C', 'A', 'B', 'B', 'A']);

    # Probabilities should match the frequencies above (within floating-point).
    ok( abs($chain->probability('A', 'B') - 2/3) < 1e-6,
        'Test 3: P(A->B) ≈ 0.667' );
    ok( abs($chain->probability('A', 'C') - 1/3) < 1e-6,
        'Test 3: P(A->C) ≈ 0.333' );
    ok( abs($chain->probability('B', 'A') - 2/3) < 1e-6,
        'Test 3: P(B->A) ≈ 0.667' );
    ok( abs($chain->probability('B', 'B') - 1/3) < 1e-6,
        'Test 3: P(B->B) ≈ 0.333' );
}

# ============================================================================
# Test 4: Laplace smoothing
#
# Setup:  order=1, smoothing=1.0, pre-registered states = [A, B, C]
#         train on [A, B]
#
# Raw counts for context "A": {B: 1}
# Smoothed denominator: 1 + 1.0 * 3 = 4   (α * |states|)
# P(A->C) = (0 + 1.0) / 4 = 0.25
# ============================================================================

{
    my $chain = CodingAdventures::MarkovChain->new(
        order     => 1,
        smoothing => 1.0,
        states    => ['A', 'B', 'C'],
    );
    $chain->train(['A', 'B']);

    my $p = $chain->probability('A', 'C');
    ok( abs($p - 0.25) < 1e-9,
        'Test 4: Laplace smoothing — P(A->C) == 0.25' );

    # The row must still sum to 1 even with smoothing.
    my $sum = $chain->probability('A', 'A')
            + $chain->probability('A', 'B')
            + $chain->probability('A', 'C');
    ok( abs($sum - 1.0) < 1e-9, 'Test 4: row sums to 1.0 with smoothing' );
}

# ============================================================================
# Test 5: Generate length — generate(A, 10) returns exactly 10 states
# ============================================================================

{
    my $chain = CodingAdventures::MarkovChain->new;
    $chain->train(['A', 'B', 'A', 'C', 'A', 'B', 'B', 'A']);

    my $seq = $chain->generate('A', 10);
    is( scalar @$seq, 10, 'Test 5: generate returns exactly 10 elements' );
    is( $seq->[0], 'A', 'Test 5: first element is the start state A' );
}

# ============================================================================
# Test 6: Generate string length
#
# train_string on English text, then generate_string("th", 50) must return
# a string of exactly 50 characters starting with "th".
# ============================================================================

{
    my $chain = CodingAdventures::MarkovChain->new(order => 1, smoothing => 1.0);

    # Train on a short corpus — enough to ensure all letter pairs are seen.
    $chain->train_string("the quick brown fox jumps over the lazy dog ");

    my $s = $chain->generate_string("t", 50);
    is( length($s), 50,  'Test 6: generate_string returns exactly 50 chars' );
    is( substr($s, 0, 1), 't', 'Test 6: generated string starts with seed "t"' );
}

# ============================================================================
# Test 7: Stationary distribution sums to 1
# ============================================================================

{
    my $chain = CodingAdventures::MarkovChain->new(smoothing => 1.0);
    $chain->train(['A', 'B', 'A', 'C', 'A', 'B', 'B', 'A']);

    my $pi = $chain->stationary_distribution;
    my $total = 0;
    $total += $_ for values %$pi;
    ok( abs($total - 1.0) < 1e-6,
        'Test 7: stationary distribution sums to 1.0' );
}

# ============================================================================
# Test 8: Order-2 chain on "abcabcabc"
#
# With order=2, the only observed context is:
#   "ab" -> "c"   (probability 1.0)
#   "bc" -> "a"   (probability 1.0)
#   "ca" -> "b"   (probability 1.0)
#
# So generate_string("ab", 9) must reproduce "abcabcabc" deterministically.
# ============================================================================

{
    my $chain = CodingAdventures::MarkovChain->new(order => 2);
    $chain->train([split //, "abcabcabc"]);

    # Verify that "ab" -> "c" with probability 1.0
    # Context key for order-2 is "a\0b"
    my $p = $chain->probability("a\0b", 'c');
    ok( abs($p - 1.0) < 1e-9,
        'Test 8: order-2 P(ab->c) == 1.0' );

    my $out = $chain->generate_string("ab", 9);
    is( $out, "abcabcabc", 'Test 8: order-2 generate_string reproduces "abcabcabc"' );
}

# ============================================================================
# Test 9: Unknown state dies
#
# Calling next_state on a context that was never seen during training must
# raise an error.
# ============================================================================

{
    my $chain = CodingAdventures::MarkovChain->new;
    $chain->train(['A', 'B']);

    eval { $chain->next_state("UNKNOWN") };
    ok( $@, 'Test 9: next_state dies on unknown state' );
    like( $@, qr/unknown state/i,
          'Test 9: error message mentions "unknown state"' );
}

# ============================================================================
# Test 10: Multi-train accumulation
#
# Calling train() twice should accumulate counts before renormalising.
# Train 1: [A, B]          → from A: {B:1}
# Train 2: [A, C, A, C]    → from A: {C:2}
# Combined from A: {B:1, C:2} → P(A->B) = 1/3, P(A->C) = 2/3
# ============================================================================

{
    my $chain = CodingAdventures::MarkovChain->new;
    $chain->train(['A', 'B']);
    $chain->train(['A', 'C', 'A', 'C']);

    ok( abs($chain->probability('A', 'B') - 1/3) < 1e-6,
        'Test 10: multi-train — P(A->B) == 1/3 after two trains' );
    ok( abs($chain->probability('A', 'C') - 2/3) < 1e-6,
        'Test 10: multi-train — P(A->C) == 2/3 after two trains' );
}

# ============================================================================
# Test 11: states() returns all known atomic states
# ============================================================================

{
    my $chain = CodingAdventures::MarkovChain->new(states => ['X', 'Y', 'Z']);
    my $states = $chain->states;
    is_deeply( $states, ['X', 'Y', 'Z'],
               'Test 11: states() returns pre-registered states sorted' );
}

# ============================================================================
# Test 12: transition_matrix() returns a deep copy
# ============================================================================

{
    my $chain = CodingAdventures::MarkovChain->new;
    $chain->train(['A', 'B', 'A', 'C']);

    my $tm = $chain->transition_matrix;
    ok( ref($tm) eq 'HASH', 'Test 12: transition_matrix returns a hashref' );
    ok( exists $tm->{A},    'Test 12: transition_matrix has row for A' );

    # Mutating the copy must not affect internal state.
    $tm->{A}{B} = 999;
    ok( abs($chain->probability('A', 'B') - 0.5) < 1e-9,
        'Test 12: mutating the returned copy does not affect internal state' );
}

# ============================================================================
# Test 13: probability() returns 0.0 for unseen transitions
# ============================================================================

{
    my $chain = CodingAdventures::MarkovChain->new;
    $chain->train(['A', 'B']);

    is( $chain->probability('A', 'Z'), 0.0,
        'Test 13: probability returns 0.0 for unseen transition' );
    is( $chain->probability('Z', 'A'), 0.0,
        'Test 13: probability returns 0.0 for unseen source state' );
}

done_testing;
