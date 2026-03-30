use strict;
use warnings;
use Test2::V0;

use CodingAdventures::GradientDescent;

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

# near: floating-point comparison with tolerance.
sub near {
    my ($got, $expected, $tol) = @_;
    $tol //= 1e-6;
    return abs($got - $expected) < $tol;
}

# MSE loss: mean((w[0]*x[0] - target)^2) for single-weight linear model.
sub mse {
    my ($weights, $inputs, $targets) = @_;
    my $sum = 0.0;
    for my $i (0..$#$inputs) {
        my $pred = 0.0;
        for my $j (0..$#$weights) {
            $pred += $weights->[$j] * $inputs->[$i][$j];
        }
        $sum += ($pred - $targets->[$i])**2;
    }
    return $sum / scalar @$inputs;
}

# Analytical gradient of MSE: 2/n * sum((w*x - t) * x[j])
sub mse_gradient {
    my ($weights, $inputs, $targets) = @_;
    my $n = scalar @$inputs;
    my @grad = (0.0) x scalar @$weights;
    for my $i (0..$#$inputs) {
        my $pred = 0.0;
        for my $j (0..$#$weights) {
            $pred += $weights->[$j] * $inputs->[$i][$j];
        }
        my $residual = $pred - $targets->[$i];
        for my $j (0..$#$weights) {
            $grad[$j] += (2.0 / $n) * $residual * $inputs->[$i][$j];
        }
    }
    return \@grad;
}

# Training data: y = 2x
my @INPUTS  = ([1.0], [2.0], [3.0], [4.0], [5.0]);
my @TARGETS = (2.0,   4.0,   6.0,   8.0,  10.0);

# ---------------------------------------------------------------------------
# new()
# ---------------------------------------------------------------------------

subtest 'new() — defaults' => sub {
    my $gd = CodingAdventures::GradientDescent->new();
    is($gd->{learning_rate},  0.01, 'default learning_rate');
    is($gd->{max_iterations}, 1000, 'default max_iterations');
    is($gd->{tolerance},      1e-6, 'default tolerance');
};

subtest 'new() — custom' => sub {
    my $gd = CodingAdventures::GradientDescent->new(
        learning_rate  => 0.1,
        max_iterations => 500,
        tolerance      => 1e-8,
    );
    is($gd->{learning_rate},  0.1,  'custom learning_rate');
    is($gd->{max_iterations}, 500,  'custom max_iterations');
    is($gd->{tolerance},      1e-8, 'custom tolerance');
};

# ---------------------------------------------------------------------------
# step()
# ---------------------------------------------------------------------------

subtest 'step() — basic update' => sub {
    my $gd = CodingAdventures::GradientDescent->new(learning_rate => 0.1);
    my ($new_w, $err) = $gd->step([1.0, 2.0], [0.5, -0.5]);
    ok(!defined $err, 'no error');
    ok(near($new_w->[0], 0.95), "w[0] = 1.0 - 0.1*0.5 = 0.95");
    ok(near($new_w->[1], 2.05), "w[1] = 2.0 - 0.1*(-0.5) = 2.05");
};

subtest 'step() — zero gradient leaves weights unchanged' => sub {
    my $gd = CodingAdventures::GradientDescent->new(learning_rate => 0.5);
    my ($new_w, $err) = $gd->step([3.0, -1.0], [0.0, 0.0]);
    ok(!defined $err, 'no error');
    ok(near($new_w->[0],  3.0), 'w[0] unchanged');
    ok(near($new_w->[1], -1.0), 'w[1] unchanged');
};

subtest 'step() — does not mutate input' => sub {
    my $gd = CodingAdventures::GradientDescent->new(learning_rate => 0.5);
    my $orig = [1.0];
    $gd->step($orig, [0.3]);
    ok(near($orig->[0], 1.0), 'original unchanged');
};

subtest 'step() — length mismatch returns error' => sub {
    my $gd = CodingAdventures::GradientDescent->new();
    my (undef, $err) = $gd->step([1.0, 2.0], [0.1]);
    ok(defined $err, 'error returned');
    like($err, qr/length/, 'error mentions length');
};

subtest 'step() — empty arrays return error' => sub {
    my $gd = CodingAdventures::GradientDescent->new();
    my (undef, $err) = $gd->step([], []);
    ok(defined $err, 'error returned');
};

# ---------------------------------------------------------------------------
# compute_loss()
# ---------------------------------------------------------------------------

subtest 'compute_loss() — perfect weights give zero loss' => sub {
    my $gd = CodingAdventures::GradientDescent->new();
    my $loss = $gd->compute_loss([2.0], \@INPUTS, \@TARGETS, \&mse);
    ok(near($loss, 0.0, 1e-10), "MSE = 0 for w=2, y=2x");
};

subtest 'compute_loss() — wrong weights give positive loss' => sub {
    my $gd = CodingAdventures::GradientDescent->new();
    my $loss = $gd->compute_loss([0.0], \@INPUTS, \@TARGETS, \&mse);
    ok($loss > 0, 'positive loss for wrong weights');
};

# ---------------------------------------------------------------------------
# numerical_gradient()
# ---------------------------------------------------------------------------

subtest 'numerical_gradient() — matches analytical gradient' => sub {
    my $gd = CodingAdventures::GradientDescent->new();
    my $num_grad = $gd->numerical_gradient([1.5], \@INPUTS, \@TARGETS, \&mse);
    my $ana_grad = mse_gradient([1.5], \@INPUTS, \@TARGETS);
    ok(near($num_grad->[0], $ana_grad->[0], 1e-4), 'numerical ≈ analytical');
};

subtest 'numerical_gradient() — returns correct length' => sub {
    my $gd = CodingAdventures::GradientDescent->new();
    my $grad = $gd->numerical_gradient([1.0, 0.5],
        [[1.0, 0.0], [0.0, 1.0]],
        [1.0, 0.5],
        \&mse);
    is(scalar @$grad, 2, 'gradient length = 2');
};

# ---------------------------------------------------------------------------
# train()
# ---------------------------------------------------------------------------

subtest 'train() — converges to w ≈ 2.0 for y=2x (analytical)' => sub {
    my $gd = CodingAdventures::GradientDescent->new(
        learning_rate  => 0.05,
        max_iterations => 2000,
        tolerance      => 1e-8,
    );
    my ($trained, $err) = $gd->train([0.0], \@INPUTS, \@TARGETS, \&mse, \&mse_gradient);
    ok(!defined $err, 'no error');
    ok(near($trained->[0], 2.0, 0.01), "w ≈ 2.0 (got $trained->[0])");
};

subtest 'train() — converges using numerical gradient' => sub {
    my $gd = CodingAdventures::GradientDescent->new(
        learning_rate  => 0.05,
        max_iterations => 3000,
        tolerance      => 1e-8,
    );
    my ($trained, $err) = $gd->train([0.0], \@INPUTS, \@TARGETS, \&mse);
    ok(!defined $err, 'no error');
    ok(near($trained->[0], 2.0, 0.05), "w ≈ 2.0 with numerical grad");
};

subtest 'train() — reduces loss vs. initial weights' => sub {
    my $gd = CodingAdventures::GradientDescent->new(
        learning_rate => 0.05, max_iterations => 100,
    );
    my $init_loss = $gd->compute_loss([0.0], \@INPUTS, \@TARGETS, \&mse);
    my ($trained, ) = $gd->train([0.0], \@INPUTS, \@TARGETS, \&mse, \&mse_gradient);
    my $final_loss = $gd->compute_loss($trained, \@INPUTS, \@TARGETS, \&mse);
    ok($final_loss < $init_loss, 'final loss < initial loss');
};

subtest 'train() — higher lr converges faster' => sub {
    my $gd_fast = CodingAdventures::GradientDescent->new(learning_rate => 0.07, max_iterations => 500);
    my $gd_slow = CodingAdventures::GradientDescent->new(learning_rate => 0.01, max_iterations => 500);

    my ($w_fast, ) = $gd_fast->train([0.0], \@INPUTS, \@TARGETS, \&mse, \&mse_gradient);
    my ($w_slow, ) = $gd_slow->train([0.0], \@INPUTS, \@TARGETS, \&mse, \&mse_gradient);

    my $loss_fast = $gd_fast->compute_loss($w_fast, \@INPUTS, \@TARGETS, \&mse);
    my $loss_slow = $gd_slow->compute_loss($w_slow, \@INPUTS, \@TARGETS, \&mse);

    ok($loss_fast < $loss_slow, 'fast lr reaches lower loss in same budget');
};

subtest 'train() — does not mutate original weight array' => sub {
    my $gd = CodingAdventures::GradientDescent->new(learning_rate => 0.05, max_iterations => 10);
    my $initial = [0.0];
    $gd->train($initial, \@INPUTS, \@TARGETS, \&mse, \&mse_gradient);
    ok(near($initial->[0], 0.0), 'original weight unchanged');
};

done_testing();
