package CodingAdventures::ActivationFunctions;

# ============================================================================
# CodingAdventures::ActivationFunctions — Neural network activation functions
# ============================================================================
#
# Activation functions introduce non-linearity into neural networks. Without
# them, a multi-layer network collapses to a single linear transformation and
# cannot learn complex patterns.
#
# ## The Biological Metaphor
#
# A biological neuron receives weighted electrical signals from its dendrites.
# When the sum exceeds a threshold, it "fires" an action potential down its
# axon. Activation functions mimic this: they take a weighted sum z and
# decide how strongly the neuron should activate in response.
#
# ## Functions Provided
#
#   | Function              | Range        | Typical Use                      |
#   |-----------------------|-------------|----------------------------------|
#   | linear                | (-∞, ∞)      | Regression output baseline       |
#   | linear_derivative     | {1}          | Backprop through linear          |
#   | sigmoid               | (0, 1)       | Binary classification output     |
#   | sigmoid_derivative    | (0, 0.25]    | Backprop through sigmoid         |
#   | relu                  | [0, ∞)       | Default hidden layer activation  |
#   | relu_derivative       | {0, 1}       | Backprop through relu            |
#   | tanh_activation       | (-1, 1)      | Zero-centred hidden layers       |
#   | tanh_derivative       | (0, 1]       | Backprop through tanh            |
#   | leaky_relu            | (-∞, ∞)      | Fixes "dying ReLU" problem       |
#   | leaky_relu_derivative | {α, 1}       | Backprop through leaky_relu      |
#   | softplus              | (0, ∞)       | Smooth alternative to ReLU       |
#   | softplus_derivative   | (0, 1)       | Backprop through softplus        |
#   | elu                   | (-α, ∞)      | Smooth, saturating alternative   |
#   | elu_derivative        | (0, 1]       | Backprop through elu             |
#   | softmax               | (0,1)^n      | Multi-class output probabilities |
#   | softmax_derivative    | (0, 0.25]^n  | Diagonal Jacobian of softmax     |
#
# ## Numerical Stability
#
# Two functions require special care:
#
#   sigmoid: For x < -709, exp(-x) would overflow to Inf in IEEE 754 double,
#   making 1/(1+Inf) = 0.0. We short-circuit at the clamping threshold.
#   Similarly for x > 709 we return 1.0 directly.
#
#   softmax: We subtract max(x) from all logits before exponentiating.
#   This keeps all exponents ≤ 0, preventing overflow while being
#   mathematically equivalent (the max cancels in numerator/denominator).
#
# ## Naming Note
#
# We cannot export a function named 'tanh' because POSIX already exports
# that name via the C math library. Instead we export 'tanh_activation'.
#
# ## Usage
#
#   use CodingAdventures::ActivationFunctions qw(
#       sigmoid sigmoid_derivative
#       relu relu_derivative
#       tanh_activation tanh_derivative
#       leaky_relu leaky_relu_derivative
#       softplus softplus_derivative
#       elu elu_derivative
#       softmax softmax_derivative
#   );
#
#   print sigmoid(0);             # 0.5
#   print relu(-3);               # 0
#   my @probs = softmax(1, 2, 3); # (0.09003, 0.24473, 0.66524)
#
# ============================================================================

use strict;
use warnings;
use POSIX qw();   # for POSIX::tanh

our $VERSION = '0.01';

use parent 'Exporter';
our @EXPORT_OK = qw(
    linear              linear_derivative
    sigmoid             sigmoid_derivative
    relu                relu_derivative
    tanh_activation     tanh_derivative
    leaky_relu          leaky_relu_derivative
    softplus            softplus_derivative
    elu                 elu_derivative
    softmax             softmax_derivative
);

# ============================================================================
# LINEAR
# ============================================================================

# linear($x) — identity activation for regression outputs.
sub linear {
    my ($x) = @_;
    return 0.0 + $x;
}

# linear_derivative($x) — constant slope of the identity function.
sub linear_derivative {
    my ($x) = @_;
    return 1.0;
}

# ============================================================================
# SIGMOID
# ============================================================================

# sigmoid($x) — logistic activation function
#
# ## Definition
#
#     σ(x) = 1 / (1 + e^(−x))
#
# ## Shape
#
# S-shaped curve mapping all of ℝ into the open interval (0, 1):
#   - x → −∞ : σ → 0
#   - x = 0  : σ = 0.5
#   - x → +∞ : σ → 1
#
# ## Clamping for Numerical Stability
#
# In Perl (and C double precision), exp(710) overflows to infinity.
# We handle the tails explicitly to avoid Inf/NaN propagation:
#   x < −709 → return 0.0 (1 / (1 + Inf) = 0)
#   x >  709 → return 1.0 (1 / (1 + ~0) ≈ 1)
#
# @param $x  any real number
# @return    σ(x) in (0, 1)
sub sigmoid {
    my ($x) = @_;
    return 0.0 if $x < -709;
    return 1.0 if $x >  709;
    return 1.0 / (1.0 + exp(-$x));
}

# sigmoid_derivative($x) — d/dx σ(x) = σ(x) · (1 − σ(x))
#
# ## Derivation
#
#     d/dx σ(x) = σ(x) · (1 − σ(x))
#
# This elegant form follows from differentiating 1/(1+e^(-x)) and
# using the identity e^(-x)/(1+e^(-x))^2 = σ(x)·(1−σ(x)).
#
# The maximum is 0.25 at x = 0 (σ(0) = 0.5 → 0.5 × 0.5 = 0.25).
# As |x| grows, the derivative shrinks toward 0 (vanishing gradient).
#
# @param $x  any real number
# @return    derivative of sigmoid at x, in (0, 0.25]
sub sigmoid_derivative {
    my ($x) = @_;
    my $s = sigmoid($x);
    return $s * (1.0 - $s);
}

# ============================================================================
# RELU
# ============================================================================

# relu($x) — Rectified Linear Unit: max(0, x)
#
# ## Why ReLU Became the Default
#
# Sigmoid and tanh both suffer from vanishing gradients for large |x|.
# ReLU's derivative is exactly 1 for x > 0, so gradients flow unchanged
# through active neurons. This enabled training of very deep networks
# (e.g., AlexNet in 2012 used ReLU and had 8 layers).
#
# @param $x  any real number
# @return    max(0, x)
sub relu {
    my ($x) = @_;
    return $x > 0 ? $x : 0.0;
}

# relu_derivative($x) — sub-gradient of ReLU
#
# ReLU is not differentiable at x = 0 (there is a kink). By convention
# we assign the sub-gradient 0 at that point. This matches every major
# deep learning framework.
#
# @param $x  any real number
# @return    1 if x > 0, else 0
sub relu_derivative {
    my ($x) = @_;
    return $x > 0 ? 1 : 0;
}

# ============================================================================
# TANH
# ============================================================================

# tanh_activation($x) — hyperbolic tangent: maps ℝ to (−1, 1)
#
# ## Definition
#
#     tanh(x) = (e^x − e^(−x)) / (e^x + e^(−x))
#
# ## Relation to Sigmoid
#
#     tanh(x) = 2·sigmoid(2x) − 1
#
# Tanh is zero-centred (output averages to 0), which avoids the
# "positive gradient bias" issue of sigmoid, often leading to faster
# convergence during gradient descent.
#
# We use POSIX::tanh for accuracy. The POSIX module delegates to the
# C standard library's tanh(), which handles edge cases correctly.
#
# NOTE: We cannot name this sub 'tanh' because that conflicts with the
# POSIX-exported tanh symbol. We use 'tanh_activation' instead.
#
# @param $x  any real number
# @return    tanh(x) in (−1, 1)
sub tanh_activation {
    my ($x) = @_;
    return POSIX::tanh($x);
}

# tanh_derivative($x) — d/dx tanh(x) = 1 − tanh(x)²
#
# ## Derivation
#
#     d/dx tanh(x) = sech²(x) = 1 − tanh²(x)
#
# Maximum is 1.0 at x = 0. Vanishes for large |x|.
#
# @param $x  any real number
# @return    derivative in (0, 1]
sub tanh_derivative {
    my ($x) = @_;
    my $t = POSIX::tanh($x);
    return 1.0 - $t * $t;
}

# ============================================================================
# LEAKY RELU
# ============================================================================

# leaky_relu($x, $alpha) — x if x>0 else alpha*x (default alpha=0.01)
#
# ## Motivation
#
# Standard ReLU kills neurons that always receive negative inputs (the
# "dying ReLU" problem). Leaky ReLU fixes this with a small negative slope
# α, keeping the gradient alive even when x ≤ 0.
#
# ## Definition
#
#     LeakyReLU(x; α) = x      if x > 0
#                     = α · x  if x ≤ 0
#
# @param $x      any real number
# @param $alpha  negative slope (default 0.01)
# @return        activated value
sub leaky_relu {
    my ($x, $alpha) = @_;
    $alpha //= 0.01;
    return $x > 0 ? $x : $alpha * $x;
}

# leaky_relu_derivative($x, $alpha) — 1 if x>0 else alpha
#
# @param $x      any real number
# @param $alpha  negative slope (default 0.01)
# @return        1 for positive input, alpha otherwise
sub leaky_relu_derivative {
    my ($x, $alpha) = @_;
    $alpha //= 0.01;
    return $x > 0 ? 1 : $alpha;
}

# ============================================================================
# SOFTPLUS
# ============================================================================

# softplus($x) — smooth ReLU approximation: log(1 + e^x)
#
# Uses the stable equivalent log(1 + e^(-abs(x))) + max(x, 0).
sub softplus {
    my ($x) = @_;
    return log(1.0 + exp(-abs($x))) + ($x > 0 ? $x : 0.0);
}

# softplus_derivative($x) — derivative of softplus is sigmoid.
sub softplus_derivative {
    my ($x) = @_;
    return sigmoid($x);
}

# ============================================================================
# ELU
# ============================================================================

# elu($x, $alpha) — x if x>=0 else alpha*(e^x - 1)  (default alpha=1.0)
#
# ## Motivation
#
# ELU improves on Leaky ReLU:
#   1. Negative saturation: as x → −∞, ELU → −α (bounded, unlike Leaky)
#   2. Smooth at zero: derivative is continuous (= 1 from both sides when α=1)
#
# ## Definition
#
#     ELU(x; α) = x               if x ≥ 0
#              = α · (e^x − 1)   if x < 0
#
# @param $x      any real number
# @param $alpha  scale for the negative part (default 1.0)
# @return        activated value
sub elu {
    my ($x, $alpha) = @_;
    $alpha //= 1.0;
    return $x >= 0 ? $x : $alpha * (exp($x) - 1.0);
}

# elu_derivative($x, $alpha) — 1 if x>=0 else alpha*e^x
#
# ## Derivation
#
#   For x ≥ 0: d/dx [x] = 1
#   For x < 0: d/dx [α(e^x − 1)] = α · e^x
#
# When α = 1.0, the derivative is continuous at x = 0 (both sides = 1).
#
# @param $x      any real number
# @param $alpha  scale for the negative part (default 1.0)
# @return        derivative of elu at x
sub elu_derivative {
    my ($x, $alpha) = @_;
    $alpha //= 1.0;
    return $x >= 0 ? 1 : $alpha * exp($x);
}

# ============================================================================
# SOFTMAX
# ============================================================================

# softmax(@logits) — probability distribution over classes
#
# ## Definition
#
#     softmax(x)_i = e^(x_i) / Σ_j e^(x_j)
#
# ## Why Softmax for Multi-Class Classification?
#
# The output layer of a K-class classifier produces K real-valued "logits".
# Softmax converts them into a valid probability distribution:
#   1. All outputs > 0 (since e^x > 0 always)
#   2. All outputs sum to exactly 1
#   3. Relative ordering is preserved (largest logit → largest probability)
#
# ## Numerical Stability: Max-Subtraction
#
# For large logits (e.g., x_i = 1000), e^1000 overflows. The fix:
# subtract max(x) from every element first. This is mathematically
# equivalent because:
#
#     e^(x_i − c) / Σ_j e^(x_j − c)
#   = e^(x_i) · e^(−c) / [Σ_j e^(x_j) · e^(−c)]
#   = e^(x_i) / Σ_j e^(x_j)
#
# After the shift, all exponents are ≤ 0, so exp() ∈ (0, 1].
#
# @param @logits  list of real-valued logits
# @return         list of probabilities summing to 1
sub softmax {
    my @x = @_;
    die "softmax: requires at least one value\n" unless @x;

    # Find the maximum for numerical stability
    my $max = $x[0];
    for my $v (@x) {
        $max = $v if $v > $max;
    }

    # Compute shifted exponentials and their sum
    my @exps;
    my $sum = 0.0;
    for my $v (@x) {
        my $e = exp($v - $max);
        push @exps, $e;
        $sum += $e;
    }

    # Normalize
    return map { $_ / $sum } @exps;
}

# softmax_derivative(@logits) — diagonal Jacobian entries s_i * (1 - s_i)
#
# ## The Full Jacobian
#
# The complete softmax Jacobian is a K×K matrix:
#
#     J_ij = ∂softmax_i / ∂x_j = s_i · (δ_ij − s_j)
#
# The diagonal entries (i = j) simplify to:
#
#     J_ii = s_i · (1 − s_i)
#
# This function returns those diagonal values — the same formula as the
# sigmoid derivative, which is not coincidental: sigmoid is softmax for
# two classes.
#
# In practice, when pairing softmax with cross-entropy loss, the combined
# gradient simplifies to (ŷ − y), bypassing the Jacobian entirely.
#
# @param @logits  list of real-valued logits
# @return         list of diagonal Jacobian values s_i * (1 - s_i)
sub softmax_derivative {
    my @x = @_;
    my @s = softmax(@x);
    return map { $_ * (1.0 - $_) } @s;
}

1;

__END__

=head1 NAME

CodingAdventures::ActivationFunctions - Neural network activation functions

=head1 SYNOPSIS

    use CodingAdventures::ActivationFunctions qw(
        linear linear_derivative
        sigmoid sigmoid_derivative
        relu relu_derivative
        tanh_activation tanh_derivative
        leaky_relu leaky_relu_derivative
        softplus softplus_derivative
        elu elu_derivative
        softmax softmax_derivative
    );

    print sigmoid(0);              # 0.5
    print relu(-3);                # 0
    my @probs = softmax(1, 2, 3);  # approx (0.090, 0.245, 0.665)

=head1 DESCRIPTION

Pure Perl implementation of the standard neural network activation functions
and their derivatives. No external dependencies. Uses only Perl's built-in
C<exp> and C<POSIX::tanh>.

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
