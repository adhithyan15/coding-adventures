use strict;
use warnings;
use Test2::V0;
use POSIX qw();

use CodingAdventures::ActivationFunctions qw(
    linear              linear_derivative
    sigmoid             sigmoid_derivative
    relu                relu_derivative
    tanh_activation     tanh_derivative
    leaky_relu          leaky_relu_derivative
    softplus            softplus_derivative
    elu                 elu_derivative
    softmax             softmax_derivative
);

# ---------------------------------------------------------------------------
# Helper: floating-point comparison with tolerance.
# ---------------------------------------------------------------------------
# IEEE 754 double arithmetic introduces tiny rounding errors. We use a
# tolerance of 1e-9 for most comparisons (sufficient for ~15-digit precision)
# and 1e-5 for finite-difference gradient checks (which have their own error).
sub near {
    my ($got, $expected, $tol) = @_;
    $tol //= 1e-9;
    return abs($got - $expected) < $tol;
}

# ---------------------------------------------------------------------------
# Helper: finite-difference derivative approximation.
# ---------------------------------------------------------------------------
# Central difference:  f'(x) ≈ (f(x+h) - f(x-h)) / (2h)
# Using h = 1e-5 balances truncation error (wants small h) vs floating-point
# cancellation error (wants h not too small).
sub fd_deriv {
    my ($f, $x, $h) = @_;
    $h //= 1e-5;
    return ($f->($x + $h) - $f->($x - $h)) / (2.0 * $h);
}

# ---------------------------------------------------------------------------
# Helper: sum of an array (for softmax probability check).
# ---------------------------------------------------------------------------
sub array_sum {
    my $s = 0;
    $s += $_ for @_;
    return $s;
}

# ===========================================================================
# LINEAR
# ===========================================================================

subtest 'linear' => sub {
    ok near(linear(-3), -3.0), 'linear(-3) = -3';
    ok near(linear(0), 0.0), 'linear(0) = 0';
    ok near(linear(5), 5.0), 'linear(5) = 5';
};

subtest 'linear_derivative' => sub {
    for my $x (-3, 0, 5) {
        ok near(linear_derivative($x), 1.0), "linear_derivative($x) = 1";
    }
};

# ===========================================================================
# SIGMOID
# ===========================================================================

subtest 'sigmoid' => sub {
    ok near(sigmoid(0), 0.5),              'sigmoid(0) = 0.5';
    ok near(sigmoid(1), 0.7310585786, 1e-8), 'sigmoid(1) golden value';
    ok near(sigmoid(-1), 0.2689414214, 1e-8), 'sigmoid(-1) golden value';
    ok near(sigmoid(100), 1.0, 1e-6),      'sigmoid(100) ≈ 1';
    ok near(sigmoid(-100), 0.0, 1e-6),     'sigmoid(-100) ≈ 0';
    ok sigmoid(-710) == 0.0,               'sigmoid clamps at x < -709';
    ok sigmoid(710)  == 1.0,               'sigmoid clamps at x > 709';

    # Monotonically increasing
    my @xs = (-10, -3, -1, 0, 1, 3, 10);
    for my $i (0 .. $#xs - 1) {
        ok sigmoid($xs[$i]) < sigmoid($xs[$i+1]),
            "sigmoid is monotone at x=$xs[$i]";
    }
};

subtest 'sigmoid_derivative' => sub {
    ok near(sigmoid_derivative(0), 0.25), 'maximum at x=0 is 0.25';

    # All positive
    for my $x (-10, -3, 0, 3, 10) {
        ok sigmoid_derivative($x) > 0, "sigmoid_derivative($x) > 0";
    }

    # Matches finite difference
    my $fd1 = fd_deriv(\&sigmoid, 1);
    ok near(sigmoid_derivative(1), $fd1, 1e-5),
        'matches finite difference at x=1';

    my $fd_neg = fd_deriv(\&sigmoid, -2);
    ok near(sigmoid_derivative(-2), $fd_neg, 1e-5),
        'matches finite difference at x=-2';

    # Vanishing gradient
    ok sigmoid_derivative(20)  < 1e-7, 'sigmoid_derivative(20) vanishes';
    ok sigmoid_derivative(-20) < 1e-7, 'sigmoid_derivative(-20) vanishes';
};

# ===========================================================================
# RELU
# ===========================================================================

subtest 'relu' => sub {
    ok relu(0)     == 0.0, 'relu(0) = 0';
    ok relu(-5)    == 0.0, 'relu(-5) = 0';
    ok relu(-0.001) == 0.0, 'relu(-0.001) = 0';
    ok near(relu(3.5), 3.5), 'relu(3.5) = 3.5';
    ok near(relu(100), 100), 'relu(100) = 100';
};

subtest 'relu_derivative' => sub {
    ok relu_derivative(-5)    == 0, 'relu_derivative(-5) = 0';
    ok relu_derivative(0)     == 0, 'relu_derivative(0) = 0 (sub-gradient)';
    ok relu_derivative(5)     == 1, 'relu_derivative(5) = 1';
    ok relu_derivative(0.001) == 1, 'relu_derivative(0.001) = 1';

    # Finite difference away from the kink
    my $fd = fd_deriv(\&relu, 1);
    ok near(relu_derivative(1), $fd, 1e-4), 'matches finite difference at x=1';
};

# ===========================================================================
# TANH
# ===========================================================================

subtest 'tanh_activation' => sub {
    ok near(tanh_activation(0), 0.0),              'tanh(0) = 0';
    ok near(tanh_activation(1), 0.7615941559557649, 1e-9), 'tanh(1) golden value';
    ok near(tanh_activation(100), 1.0, 1e-6),      'tanh(100) ≈ 1';
    ok near(tanh_activation(-100), -1.0, 1e-6),    'tanh(-100) ≈ -1';

    # Anti-symmetry: tanh(-x) = -tanh(x)
    for my $x (0.5, 1, 2, 3) {
        ok near(tanh_activation(-$x), -tanh_activation($x)),
            "tanh is anti-symmetric at x=$x";
    }
};

subtest 'tanh_derivative' => sub {
    ok near(tanh_derivative(0), 1.0), 'maximum at x=0 is 1.0';

    # Non-negative everywhere
    for my $x (-10, -3, 0, 3, 10) {
        ok tanh_derivative($x) >= 0, "tanh_derivative($x) >= 0";
    }

    # Matches finite difference
    my $fd = fd_deriv(\&tanh_activation, 1);
    ok near(tanh_derivative(1), $fd, 1e-5), 'matches finite difference at x=1';

    # Vanishes for large |x|
    ok tanh_derivative(10)  < 1e-8, 'tanh_derivative(10) vanishes';
    ok tanh_derivative(-10) < 1e-8, 'tanh_derivative(-10) vanishes';
};

# ===========================================================================
# LEAKY RELU
# ===========================================================================

subtest 'leaky_relu' => sub {
    ok near(leaky_relu(5), 5),          'leaky_relu(5) = 5 (identity)';
    ok near(leaky_relu(-10), -0.1),     'leaky_relu(-10) = -0.1 (default alpha)';
    ok near(leaky_relu(0), 0.0),        'leaky_relu(0) = 0';
    ok near(leaky_relu(-5, 0.2), -1.0), 'leaky_relu(-5, 0.2) = -1.0';
    ok leaky_relu(-100) < 0,            'leaky_relu(-100) < 0';
};

subtest 'leaky_relu_derivative' => sub {
    ok leaky_relu_derivative(5)  == 1,    'leaky_relu_derivative(5) = 1';
    ok near(leaky_relu_derivative(-5), 0.01), 'leaky_relu_derivative(-5) = 0.01';
    ok near(leaky_relu_derivative(0), 0.01),  'leaky_relu_derivative(0) = alpha';
    ok near(leaky_relu_derivative(-1, 0.1), 0.1), 'custom alpha in derivative';

    my $fd = fd_deriv(sub { leaky_relu($_[0]) }, 2);
    ok near(leaky_relu_derivative(2), $fd, 1e-4),
        'matches finite difference at x=2';
};

# ===========================================================================
# SOFTPLUS
# ===========================================================================

subtest 'softplus' => sub {
    ok near(softplus(0), log(2.0)), 'softplus(0) = log(2)';
    ok near(softplus(1), 1.3132616875182228), 'softplus(1) golden value';
    ok near(softplus(-1), 0.31326168751822286), 'softplus(-1) golden value';
    ok softplus(1000) > 999.0, 'softplus remains stable for large positives';
};

subtest 'softplus_derivative' => sub {
    ok near(softplus_derivative(0), 0.5), 'softplus_derivative(0) = 0.5';
    ok near(softplus_derivative(1), sigmoid(1)), 'softplus derivative equals sigmoid at x=1';
    ok near(softplus_derivative(-1), sigmoid(-1)), 'softplus derivative equals sigmoid at x=-1';

    my $fd = fd_deriv(\&softplus, 1);
    ok near(softplus_derivative(1), $fd, 1e-5),
        'matches finite difference at x=1';
};

# ===========================================================================
# ELU
# ===========================================================================

subtest 'elu' => sub {
    ok near(elu(0), 0.0), 'elu(0) = 0';
    ok near(elu(3), 3.0), 'elu(3) = 3 (identity for x >= 0)';

    # elu(-1) = 1*(e^-1 - 1) ≈ -0.6321205588
    my $expected = exp(-1) - 1;
    ok near(elu(-1), $expected), 'elu(-1) = e^(-1) - 1';

    # Saturates at -1 for large negative x (alpha=1)
    ok near(elu(-100), -1.0, 1e-6), 'elu saturates at -alpha for large negative x';

    # Custom alpha
    ok near(elu(-1, 2.0), 2.0 * (exp(-1) - 1)), 'elu(-1, alpha=2) uses custom alpha';

    # Continuous at zero
    ok near(elu(-1e-10), 0.0, 1e-9), 'elu is continuous at x=0';
};

subtest 'elu_derivative' => sub {
    ok elu_derivative(0) == 1, 'elu_derivative(0) = 1';
    ok elu_derivative(5) == 1, 'elu_derivative(5) = 1';
    ok near(elu_derivative(-1), exp(-1)), 'elu_derivative(-1) = e^(-1)';
    ok near(elu_derivative(-1, 2.0), 2.0 * exp(-1)), 'custom alpha in derivative';
    ok elu_derivative(-100) < 1e-10, 'approaches 0 for very negative x';

    # Finite difference at x=-1
    my $fd = fd_deriv(sub { elu($_[0]) }, -1);
    ok near(elu_derivative(-1), $fd, 1e-5),
        'matches finite difference at x=-1';
};

# ===========================================================================
# SOFTMAX
# ===========================================================================

subtest 'softmax' => sub {
    # Sum to 1
    my @probs = softmax(1, 2, 3);
    ok near(array_sum(@probs), 1.0, 1e-12), 'softmax sums to 1';

    # All positive
    for my $p (@probs) {
        ok $p > 0, "softmax output $p > 0";
    }

    # Correct golden values for (1, 2, 3)
    # e^1 ≈ 2.71828, e^2 ≈ 7.38906, e^3 ≈ 20.08554  sum ≈ 30.19288
    ok near($probs[0], 0.09003057, 1e-6), 'softmax({1,2,3})[0] ≈ 0.09003';
    ok near($probs[1], 0.24472847, 1e-6), 'softmax({1,2,3})[1] ≈ 0.24473';
    ok near($probs[2], 0.66524096, 1e-6), 'softmax({1,2,3})[2] ≈ 0.66524';

    # Invariant to constant shift (numerical stability property)
    my @r1 = softmax(1, 2, 3);
    my @r2 = softmax(1001, 1002, 1003);
    for my $i (0 .. 2) {
        ok near($r1[$i], $r2[$i], 1e-10),
            "softmax is shift-invariant at index $i";
    }

    # Handles large values without overflow
    my @large = softmax(1000, 1001, 1002);
    ok near(array_sum(@large), 1.0, 1e-12), 'softmax handles large values';

    # Single element → probability 1
    my @single = softmax(42);
    ok near($single[0], 1.0), 'softmax of single element is 1.0';

    # Uniform for equal inputs
    my @uniform = softmax(5, 5, 5);
    for my $p (@uniform) {
        ok near($p, 1.0/3.0, 1e-10), "softmax uniform: $p ≈ 1/3";
    }

    # Dies on empty input
    ok dies { softmax() }, 'softmax dies on empty input';
};

subtest 'softmax_derivative' => sub {
    my @d = softmax_derivative(1, 2, 3);
    my @s = softmax(1, 2, 3);

    # Each entry equals s_i * (1 - s_i)
    for my $i (0 .. 2) {
        ok near($d[$i], $s[$i] * (1 - $s[$i])),
            "softmax_derivative[$i] = s[$i]*(1-s[$i])";
    }

    # All positive
    for my $v (@d) {
        ok $v > 0, "softmax_derivative entry $v > 0";
    }

    # All <= 0.25
    for my $v (@d) {
        ok $v <= 0.25 + 1e-12, "softmax_derivative entry $v <= 0.25";
    }

    # Same length as input
    ok scalar(@d) == 3, 'softmax_derivative has same length as input';
};

done_testing();
