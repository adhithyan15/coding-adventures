package CodingAdventures::LossFunctions;

# ============================================================================
# CodingAdventures::LossFunctions — Machine learning loss functions
# ============================================================================
#
# This module provides the four core supervised-learning loss functions
# and their analytical derivatives.  All functions accept array references
# and return (value, undef) on success or (undef, "error message") on
# failure.
#
# ## What Is a Loss Function?
#
# A loss function measures how far a model's predictions are from the
# ground-truth targets.  Training a neural network means finding the
# model weights that minimise this function.
#
# For a batch of n examples, given:
#   - y_true: the ground-truth target values  [y₁, y₂, …, yₙ]
#   - y_pred: the model's predicted values    [ŷ₁, ŷ₂, …, ŷₙ]
#
# each loss function collapses the n differences into a single scalar.
#
# ## Why Derivatives?
#
# The derivative ∂L/∂ŷᵢ tells the optimiser which direction to nudge
# the prediction to reduce the loss.  Backpropagation chains these
# gradients back through all layers using the chain rule.
#
# ## Numerical Stability: Epsilon Clamping
#
# BCE and CCE use log(ŷ).  Since log(0) = −∞, we clamp ŷ to the range
# [ε, 1−ε] where ε = 1e-7.  This is standard practice in every
# deep-learning framework (TensorFlow, PyTorch, JAX).
#
# ## Usage
#
#   use CodingAdventures::LossFunctions qw(mse mae bce cce
#       mse_derivative mae_derivative bce_derivative cce_derivative);
#
#   my ($loss, $err) = mse([1,2,3], [1.1, 1.9, 3.2]);
#   die $err if $err;
#   print "MSE = $loss\n";   # 0.02
#
#   my ($grad, $err2) = mse_derivative([1,2,3], [1.1, 1.9, 3.2]);
#   die $err2 if $err2;
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(
    mse     mae     bce     cce
    mse_derivative  mae_derivative  bce_derivative  cce_derivative
);

# ============================================================================
# Constants
# ============================================================================

# EPSILON is the minimum value a clamped prediction can reach.
#
# We chose 1e-7 to match common deep-learning frameworks.  It is small
# enough not to meaningfully distort loss values, yet large enough to
# prevent log() from receiving zero or negative inputs.
my $EPSILON = 1e-7;

# ============================================================================
# Internal helpers
# ============================================================================

# _clamp restricts a value $x to the closed interval [$lo, $hi].
#
# This is the fundamental tool for numerical stability in BCE and CCE.
# Without it, a prediction of exactly 0.0 would cause log(0) = -Inf and
# propagate NaN through all downstream computations.
sub _clamp {
    my ( $x, $lo, $hi ) = @_;
    return $lo if $x < $lo;
    return $hi if $x > $hi;
    return $x;
}

# _validate checks that y_true and y_pred are compatible array references.
#
# Rules:
#   1. Both must be array references.
#   2. Both must have the same length.
#   3. The length must be > 0 (a loss over no elements is undefined).
#
# Returns undef on success, or an error string describing the problem.
sub _validate {
    my ( $y_true, $y_pred ) = @_;

    return 'y_true must be an array reference'
        unless ref($y_true) eq 'ARRAY';
    return 'y_pred must be an array reference'
        unless ref($y_pred) eq 'ARRAY';

    my $n = scalar @$y_true;
    return 'y_true and y_pred must not be empty'
        if $n == 0;
    return sprintf(
        'y_true and y_pred must have the same length (got %d and %d)',
        $n, scalar @$y_pred
    ) if scalar @$y_pred != $n;

    return undef;   # no error
}

# ============================================================================
# MSE — Mean Squared Error
# ============================================================================

=head2 mse($y_true, $y_pred)

Computes the Mean Squared Error.

    MSE = (1/n) * sum_i (y_true[i] - y_pred[i])^2

MSE squares the residuals, which means large errors are penalised more
than small ones.  It is differentiable everywhere, which makes it the
default loss for regression.

Returns C<(scalar, undef)> on success or C<(undef, error_string)> on failure.

=cut

sub mse {
    my ( $y_true, $y_pred ) = @_;

    my $err = _validate( $y_true, $y_pred );
    return ( undef, $err ) if $err;

    my $n   = scalar @$y_true;
    my $sum = 0.0;

    for my $i ( 0 .. $n - 1 ) {
        # Compute the squared residual for element i.
        # We subtract true from pred; squaring means sign doesn't matter.
        my $diff = $y_true->[$i] - $y_pred->[$i];
        $sum += $diff * $diff;
    }

    return ( $sum / $n, undef );
}

# ============================================================================
# MAE — Mean Absolute Error
# ============================================================================

=head2 mae($y_true, $y_pred)

Computes the Mean Absolute Error.

    MAE = (1/n) * sum_i |y_true[i] - y_pred[i]|

MAE penalises all errors equally (linear penalty), making it more robust
to outliers than MSE.  It is not differentiable at zero, which is why its
derivative returns 0 at an exact match.

Returns C<(scalar, undef)> on success or C<(undef, error_string)> on failure.

=cut

sub mae {
    my ( $y_true, $y_pred ) = @_;

    my $err = _validate( $y_true, $y_pred );
    return ( undef, $err ) if $err;

    my $n   = scalar @$y_true;
    my $sum = 0.0;

    for my $i ( 0 .. $n - 1 ) {
        # abs() is a Perl built-in; no POSIX import needed.
        $sum += abs( $y_true->[$i] - $y_pred->[$i] );
    }

    return ( $sum / $n, undef );
}

# ============================================================================
# BCE — Binary Cross-Entropy
# ============================================================================

=head2 bce($y_true, $y_pred)

Computes the Binary Cross-Entropy loss.

    BCE = -(1/n) * sum_i [ y[i]*log(p[i]) + (1-y[i])*log(1-p[i]) ]

where p[i] = clamp(y_pred[i], epsilon, 1-epsilon).

BCE is used for binary classification where each label is 0 or 1 and
each prediction is a probability in [0, 1].

Returns C<(scalar, undef)> on success or C<(undef, error_string)> on failure.

=cut

sub bce {
    my ( $y_true, $y_pred ) = @_;

    my $err = _validate( $y_true, $y_pred );
    return ( undef, $err ) if $err;

    my $n   = scalar @$y_true;
    my $sum = 0.0;

    for my $i ( 0 .. $n - 1 ) {
        # Clamp to avoid log(0) = -Inf.
        my $p = _clamp( $y_pred->[$i], $EPSILON, 1.0 - $EPSILON );

        # Cross-entropy for this element:
        #   When y[i]=1 → first term -log(p) dominates.
        #   When y[i]=0 → second term -log(1-p) dominates.
        $sum += $y_true->[$i] * log($p)
              + ( 1.0 - $y_true->[$i] ) * log( 1.0 - $p );
    }

    # The negative sign converts cross-entropy into a positive loss to minimise.
    return ( -$sum / $n, undef );
}

# ============================================================================
# CCE — Categorical Cross-Entropy
# ============================================================================

=head2 cce($y_true, $y_pred)

Computes the Categorical Cross-Entropy loss.

    CCE = -(1/n) * sum_i [ y[i]*log(p[i]) ]

where p[i] = clamp(y_pred[i], epsilon, 1-epsilon).

CCE is used for multi-class classification with one-hot encoded y_true
vectors and softmax y_pred distributions.

Returns C<(scalar, undef)> on success or C<(undef, error_string)> on failure.

=cut

sub cce {
    my ( $y_true, $y_pred ) = @_;

    my $err = _validate( $y_true, $y_pred );
    return ( undef, $err ) if $err;

    my $n   = scalar @$y_true;
    my $sum = 0.0;

    for my $i ( 0 .. $n - 1 ) {
        # Clamp to avoid log(0) when the predicted probability is zero.
        my $p = _clamp( $y_pred->[$i], $EPSILON, 1.0 - $EPSILON );

        # Only non-zero y[i] terms contribute.  In a standard one-hot
        # vector exactly one element is 1 (the true class).
        $sum += $y_true->[$i] * log($p);
    }

    return ( -$sum / $n, undef );
}

# ============================================================================
# MSE Derivative
# ============================================================================

=head2 mse_derivative($y_true, $y_pred)

Returns the gradient of MSE with respect to each predicted value.

    dMSE/dy_pred[i] = (2/n) * (y_pred[i] - y_true[i])

The factor of 2 comes from differentiating the squared residual.
Note the sign: (y_pred - y_true), not (y_true - y_pred).

Returns C<(array_ref, undef)> on success or C<(undef, error_string)> on failure.

=cut

sub mse_derivative {
    my ( $y_true, $y_pred ) = @_;

    my $err = _validate( $y_true, $y_pred );
    return ( undef, $err ) if $err;

    my $n    = scalar @$y_true;
    my @grad;

    for my $i ( 0 .. $n - 1 ) {
        # (2/n) * (ŷᵢ - yᵢ)
        # When over-predicted (ŷ > y), gradient is positive → push prediction down.
        # When under-predicted (ŷ < y), gradient is negative → push prediction up.
        $grad[$i] = ( 2.0 / $n ) * ( $y_pred->[$i] - $y_true->[$i] );
    }

    return ( \@grad, undef );
}

# ============================================================================
# MAE Derivative
# ============================================================================

=head2 mae_derivative($y_true, $y_pred)

Returns the subgradient of MAE with respect to each predicted value.

    dMAE/dy_pred[i] = +1/n   if y_pred[i] > y_true[i]
                    = -1/n   if y_pred[i] < y_true[i]
                    =  0     if y_pred[i] == y_true[i]

The absolute value is not differentiable at zero; we use the standard
subgradient convention of 0 at the kink.

Returns C<(array_ref, undef)> on success or C<(undef, error_string)> on failure.

=cut

sub mae_derivative {
    my ( $y_true, $y_pred ) = @_;

    my $err = _validate( $y_true, $y_pred );
    return ( undef, $err ) if $err;

    my $n    = scalar @$y_true;
    my @grad;

    for my $i ( 0 .. $n - 1 ) {
        if    ( $y_pred->[$i] > $y_true->[$i] ) { $grad[$i] =  1.0 / $n; }
        elsif ( $y_pred->[$i] < $y_true->[$i] ) { $grad[$i] = -1.0 / $n; }
        else                                     { $grad[$i] =  0.0;      }
    }

    return ( \@grad, undef );
}

# ============================================================================
# BCE Derivative
# ============================================================================

=head2 bce_derivative($y_true, $y_pred)

Returns the gradient of BCE with respect to each predicted value.

    dBCE/dy_pred[i] = (1/n) * (p - y[i]) / (p * (1-p))

where p = clamp(y_pred[i], epsilon, 1-epsilon).

The denominator p*(1-p) is the variance of a Bernoulli distribution with
success probability p.  It normalises the gradient so that confident
correct predictions receive small gradients and confident wrong predictions
receive large gradients.

Returns C<(array_ref, undef)> on success or C<(undef, error_string)> on failure.

=cut

sub bce_derivative {
    my ( $y_true, $y_pred ) = @_;

    my $err = _validate( $y_true, $y_pred );
    return ( undef, $err ) if $err;

    my $n    = scalar @$y_true;
    my @grad;

    for my $i ( 0 .. $n - 1 ) {
        # Clamp exactly as in the forward pass for consistency.
        my $p = _clamp( $y_pred->[$i], $EPSILON, 1.0 - $EPSILON );

        # ∂BCE/∂ŷᵢ = (1/n) * (p - y) / (p * (1-p))
        $grad[$i] = ( 1.0 / $n ) * ( $p - $y_true->[$i] )
                    / ( $p * ( 1.0 - $p ) );
    }

    return ( \@grad, undef );
}

# ============================================================================
# CCE Derivative
# ============================================================================

=head2 cce_derivative($y_true, $y_pred)

Returns the gradient of CCE with respect to each predicted value.

    dCCE/dy_pred[i] = -(1/n) * (y[i] / p)

where p = clamp(y_pred[i], epsilon, 1-epsilon).

For one-hot y_true, only the gradient of the true class is non-zero.
In practice CCE is usually combined with softmax, whose Jacobian simplifies
the combined gradient to just (ŷ - y).  This function computes the CCE
gradient alone (without the softmax Jacobian).

Returns C<(array_ref, undef)> on success or C<(undef, error_string)> on failure.

=cut

sub cce_derivative {
    my ( $y_true, $y_pred ) = @_;

    my $err = _validate( $y_true, $y_pred );
    return ( undef, $err ) if $err;

    my $n    = scalar @$y_true;
    my @grad;

    for my $i ( 0 .. $n - 1 ) {
        # Clamp to avoid division by zero.
        my $p = _clamp( $y_pred->[$i], $EPSILON, 1.0 - $EPSILON );

        # ∂CCE/∂ŷᵢ = -(1/n) * (yᵢ / p)
        # When yᵢ = 0, gradient is 0.
        # When yᵢ = 1 (true class), gradient is -1/(n*p) < 0 (increasing p reduces loss).
        $grad[$i] = ( -1.0 / $n ) * ( $y_true->[$i] / $p );
    }

    return ( \@grad, undef );
}

1;

__END__

=head1 NAME

CodingAdventures::LossFunctions — Machine learning loss functions and their derivatives

=head1 VERSION

0.01

=head1 SYNOPSIS

  use CodingAdventures::LossFunctions qw(
      mse mae bce cce
      mse_derivative mae_derivative bce_derivative cce_derivative
  );

  my ($loss, $err) = mse([1,2,3], [1.1, 1.9, 3.2]);
  die $err if $err;

  my ($grad, $err2) = mse_derivative([1,2,3], [1.1, 1.9, 3.2]);

=head1 DESCRIPTION

Eight pure-Perl functions covering MSE, MAE, BCE, and CCE loss functions
and their analytical gradients.  See the individual function documentation
for formulas and examples.

=head1 FUNCTIONS

=over 4

=item mse($y_true, $y_pred)

=item mae($y_true, $y_pred)

=item bce($y_true, $y_pred)

=item cce($y_true, $y_pred)

=item mse_derivative($y_true, $y_pred)

=item mae_derivative($y_true, $y_pred)

=item bce_derivative($y_true, $y_pred)

=item cce_derivative($y_true, $y_pred)

=back

All functions return C<(value, undef)> on success or C<(undef, "error")>
on failure.  For scalar loss functions, value is a number.  For derivative
functions, value is an array reference.

=head1 LICENSE

MIT

=cut
