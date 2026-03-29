use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Perceptron;

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

sub near {
    my ($got, $exp, $tol) = @_;
    $tol //= 1e-6;
    return abs($got - $exp) < $tol;
}

# accuracy: fraction of examples classified correctly (step rounding)
sub accuracy {
    my ($p, $inputs, $targets) = @_;
    my $correct = 0;
    for my $i (0..$#$inputs) {
        my ($out) = $p->predict($inputs->[$i]);
        my $pred = int($out + 0.5);
        $correct++ if $pred == $targets->[$i];
    }
    return $correct / scalar @$inputs;
}

# Logic gate training data
my @AND_INPUTS  = ([0,0], [0,1], [1,0], [1,1]);
my @AND_TARGETS = (0, 0, 0, 1);

my @OR_INPUTS   = ([0,0], [0,1], [1,0], [1,1]);
my @OR_TARGETS  = (0, 1, 1, 1);

# ---------------------------------------------------------------------------
# new()
# ---------------------------------------------------------------------------

subtest 'new() — defaults' => sub {
    my $p = CodingAdventures::Perceptron->new(n_inputs => 2);
    is($p->{n_inputs},      2,   'n_inputs = 2');
    is($p->{learning_rate}, 0.1, 'learning_rate = 0.1');
    is($p->{bias},          0.0, 'bias = 0.0');
    is(scalar @{$p->{weights}}, 2, 'weights has 2 elements');
    ok(near($p->{weights}[0], 0.0), 'weight[0] = 0');
    ok(near($p->{weights}[1], 0.0), 'weight[1] = 0');
};

subtest 'new() — custom weights and bias' => sub {
    my $p = CodingAdventures::Perceptron->new(
        n_inputs => 2, weights => [0.5, -0.3], bias => 0.1
    );
    ok(near($p->{weights}[0],  0.5), 'weight[0]');
    ok(near($p->{weights}[1], -0.3), 'weight[1]');
    ok(near($p->{bias},        0.1), 'bias');
};

subtest 'new() — errors without n_inputs' => sub {
    ok(dies { CodingAdventures::Perceptron->new() }, 'dies without n_inputs');
};

subtest 'new() — errors when weights length mismatches' => sub {
    ok(dies { CodingAdventures::Perceptron->new(n_inputs => 2, weights => [0.1]) },
       'dies on weight length mismatch');
};

# ---------------------------------------------------------------------------
# Activation functions
# ---------------------------------------------------------------------------

subtest 'step() activation' => sub {
    is(CodingAdventures::Perceptron::step(0.0),   0, 'step(0) = 0');
    is(CodingAdventures::Perceptron::step(-1.0),  0, 'step(-1) = 0');
    is(CodingAdventures::Perceptron::step(0.001), 1, 'step(0.001) = 1');
    is(CodingAdventures::Perceptron::step(100.0), 1, 'step(100) = 1');
};

subtest 'sigmoid() activation' => sub {
    ok(near(CodingAdventures::Perceptron::sigmoid(0), 0.5), 'sigmoid(0) = 0.5');
    ok(CodingAdventures::Perceptron::sigmoid(-1000) >= 0.0, 'sigmoid(-1000) >= 0');
    ok(CodingAdventures::Perceptron::sigmoid( 1000) <= 1.0, 'sigmoid(1000) <= 1');
    ok(CodingAdventures::Perceptron::sigmoid(0) > 0.0, 'sigmoid(0) > 0');
    ok(CodingAdventures::Perceptron::sigmoid(0) < 1.0, 'sigmoid(0) < 1');
};

subtest 'sigmoid_derivative() at 0 = 0.25' => sub {
    ok(near(CodingAdventures::Perceptron::sigmoid_derivative(0), 0.25, 1e-9),
       'sigmoid_deriv(0) = 0.25');
};

# ---------------------------------------------------------------------------
# predict()
# ---------------------------------------------------------------------------

subtest 'predict() — all-zero inputs with zero weights' => sub {
    my $p = CodingAdventures::Perceptron->new(n_inputs => 2);
    my ($out) = $p->predict([0, 0]);
    is($out, 0, 'output = 0');
};

subtest 'predict() — w={1,1}, b=-1.5 correctly classifies AND' => sub {
    my $p = CodingAdventures::Perceptron->new(
        n_inputs => 2, weights => [1.0, 1.0], bias => -1.5
    );
    is($p->predict([0,0]), 0, '(0,0) → 0');
    is($p->predict([0,1]), 0, '(0,1) → 0');
    is($p->predict([1,0]), 0, '(1,0) → 0');
    is($p->predict([1,1]), 1, '(1,1) → 1');
};

subtest 'predict() — returns pre-activation z' => sub {
    my $p = CodingAdventures::Perceptron->new(
        n_inputs => 1, weights => [2.0], bias => 1.0
    );
    my (undef, $z) = $p->predict([3.0]);
    # z = 2.0 * 3.0 + 1.0 = 7.0
    ok(near($z, 7.0), 'z = 7.0');
};

subtest 'predict() — errors on wrong input size' => sub {
    my $p = CodingAdventures::Perceptron->new(n_inputs => 2);
    ok(dies { $p->predict([1, 2, 3]) }, 'dies on wrong input length');
};

subtest 'predict() — sigmoid activation in (0,1)' => sub {
    my $p = CodingAdventures::Perceptron->new(
        n_inputs      => 2,
        weights       => [1.0, 1.0],
        activation_fn => \&CodingAdventures::Perceptron::sigmoid,
    );
    my ($out) = $p->predict([0.5, 0.5]);
    ok($out > 0.0, 'output > 0');
    ok($out < 1.0, 'output < 1');
};

# ---------------------------------------------------------------------------
# train_step()
# ---------------------------------------------------------------------------

subtest 'train_step() — no update on correct prediction' => sub {
    my $p = CodingAdventures::Perceptron->new(
        n_inputs => 2, weights => [1.0, 1.0], bias => -1.5
    );
    my $w0_before = $p->{weights}[0];
    my (undef, $err) = $p->train_step([0, 0], 0);
    is($err, 0, 'error = 0');
    ok(near($p->{weights}[0], $w0_before), 'weight unchanged on correct prediction');
};

subtest 'train_step() — updates weights on wrong prediction' => sub {
    my $p = CodingAdventures::Perceptron->new(n_inputs => 2, learning_rate => 0.5);
    my $w0_before = $p->{weights}[0];
    my (undef, $err) = $p->train_step([1, 1], 1);
    is($err, 1, 'error = 1 (predicted 0, target 1)');
    ok($p->{weights}[0] > $w0_before, 'weight[0] increased');
};

# ---------------------------------------------------------------------------
# train() — Logic gates
# ---------------------------------------------------------------------------

subtest 'train() — AND gate reaches 100% accuracy' => sub {
    my $p = CodingAdventures::Perceptron->new(n_inputs => 2, learning_rate => 0.1);
    $p->train(\@AND_INPUTS, \@AND_TARGETS, 200);
    ok(near(accuracy($p, \@AND_INPUTS, \@AND_TARGETS), 1.0),
       'AND gate: 100% accuracy');
};

subtest 'train() — OR gate reaches 100% accuracy' => sub {
    my $p = CodingAdventures::Perceptron->new(n_inputs => 2, learning_rate => 0.1);
    $p->train(\@OR_INPUTS, \@OR_TARGETS, 200);
    ok(near(accuracy($p, \@OR_INPUTS, \@OR_TARGETS), 1.0),
       'OR gate: 100% accuracy');
};

subtest 'train() — returns self for chaining' => sub {
    my $p = CodingAdventures::Perceptron->new(n_inputs => 2);
    my $ret = $p->train(\@AND_INPUTS, \@AND_TARGETS, 10);
    ok($ret == $p, 'returns self');
};

subtest 'train() — errors on length mismatch' => sub {
    my $p = CodingAdventures::Perceptron->new(n_inputs => 2);
    ok(dies { $p->train([[1,0]], [0, 1], 10) }, 'dies on length mismatch');
};

subtest 'train() — non-zero initial bias still converges on AND' => sub {
    my $p = CodingAdventures::Perceptron->new(
        n_inputs => 2, learning_rate => 0.1, bias => -0.5
    );
    $p->train(\@AND_INPUTS, \@AND_TARGETS, 500);
    ok(near(accuracy($p, \@AND_INPUTS, \@AND_TARGETS), 1.0),
       'AND gate converges with non-zero bias');
};

done_testing();
