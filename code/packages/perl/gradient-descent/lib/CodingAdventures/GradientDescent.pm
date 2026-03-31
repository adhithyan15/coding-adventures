package CodingAdventures::GradientDescent;

# ============================================================================
# CodingAdventures::GradientDescent — Gradient descent weight optimiser
# ============================================================================
#
# Gradient descent is the iterative optimisation algorithm that powers the
# training of virtually every machine-learning model.  Starting from an
# initial weight vector, it repeatedly nudges the weights in the direction
# opposite to the gradient of the loss function — downhill, toward the
# minimum.
#
# ## The Core Update Rule
#
#   w_new[i] = w[i] - learning_rate * gradient[i]
#
# The negative sign is crucial:
#   - gradient > 0 → we're climbing rightward → step left (decrease w)
#   - gradient < 0 → we're climbing leftward  → step right (increase w)
#   - gradient = 0 → we're at a flat point    → no change
#
# ## Numerical vs. Analytical Gradients
#
# Analytical gradients require a closed-form derivative of the loss function.
# Numerical gradients approximate the derivative using central differences:
#
#   ∂L/∂w_i ≈ (L(w + ε·eᵢ) - L(w - ε·eᵢ)) / (2ε)
#
# where ε is a small perturbation (default 1e-5).  Numerical gradients are
# slower (2n loss evaluations for n weights) but require no calculus.  They
# are primarily used for gradient checking and for problems where the
# analytical form is not known.
#
# ## Usage
#
#   use CodingAdventures::GradientDescent;
#
#   my $gd = CodingAdventures::GradientDescent->new(
#       learning_rate  => 0.1,
#       max_iterations => 1000,
#       tolerance      => 1e-6,
#   );
#
#   # Simple MSE loss for linear model y = w*x
#   my $loss_fn = sub {
#       my ($weights, $inputs, $targets) = @_;
#       my $sum = 0.0;
#       for my $i (0..$#$inputs) {
#           my $pred = $weights->[0] * $inputs->[$i][0];
#           $sum += ($pred - $targets->[$i])**2;
#       }
#       return $sum / scalar @$inputs;
#   };
#
#   my ($trained, $err) = $gd->train([0.0], $inputs, $targets, $loss_fn);
#   die $err if $err;
#   # $trained->[0] ≈ 2.0 for y = 2x data
#
# ============================================================================

use strict;
use warnings;
use POSIX ();

our $VERSION = '0.01';

# ============================================================================
# Constructor
# ============================================================================

=head2 new(%args)

Create a new GradientDescent optimiser.

  my $gd = CodingAdventures::GradientDescent->new(
      learning_rate  => 0.01,    # step size per iteration
      max_iterations => 1000,    # maximum gradient steps
      tolerance      => 1e-6,    # stop when loss change < this
  );

All arguments are optional; the defaults shown above are used if omitted.

=cut

sub new {
    my ($class, %args) = @_;
    return bless {
        learning_rate  => $args{learning_rate}  // 0.01,
        max_iterations => $args{max_iterations} // 1000,
        tolerance      => $args{tolerance}      // 1e-6,
    }, $class;
}

# ============================================================================
# step — Apply one gradient update
# ============================================================================

=head2 step($weights, $gradient)

Apply one gradient descent update: w_new[i] = w[i] - lr * grad[i].

Returns C<($new_weights_arrayref, undef)> on success, or C<(undef, $error)>
on failure (mismatched lengths or empty arrays).

  my ($new_w, $err) = $gd->step([1.0, 2.0], [0.5, -0.5]);
  die $err if $err;
  # $new_w = [0.95, 2.05]  (for lr = 0.1)

=cut

sub step {
    my ($self, $weights, $gradient) = @_;

    if (scalar @$weights != scalar @$gradient) {
        return (undef, sprintf(
            "step: weights length (%d) must equal gradient length (%d)",
            scalar @$weights, scalar @$gradient
        ));
    }
    if (scalar @$weights == 0) {
        return (undef, "step: weights must be non-empty");
    }

    my @new_weights;
    for my $i (0..$#$weights) {
        push @new_weights, $weights->[$i] - $self->{learning_rate} * $gradient->[$i];
    }
    return (\@new_weights, undef);
}

# ============================================================================
# compute_loss — Evaluate the loss function
# ============================================================================

=head2 compute_loss($weights, $inputs, $targets, $loss_fn)

Evaluate C<$loss_fn->($weights, $inputs, $targets)> and return the scalar
result.  A thin wrapper used by train() and tests.

=cut

sub compute_loss {
    my ($self, $weights, $inputs, $targets, $loss_fn) = @_;
    return $loss_fn->($weights, $inputs, $targets);
}

# ============================================================================
# numerical_gradient — Central finite-difference gradient approximation
# ============================================================================

=head2 numerical_gradient($weights, $inputs, $targets, $loss_fn, $epsilon)

Approximate the gradient of C<$loss_fn> at C<$weights> using central
differences:

  ∂L/∂w_i ≈ (L(w + ε·eᵢ) - L(w - ε·eᵢ)) / (2ε)

C<$epsilon> defaults to 1e-5.  Requires 2n loss evaluations for n weights.
Use for gradient checking or when no analytical gradient is available.

Returns an array reference of the same length as C<$weights>.

=cut

sub numerical_gradient {
    my ($self, $weights, $inputs, $targets, $loss_fn, $epsilon) = @_;
    $epsilon //= 1e-5;

    my @grad;
    for my $i (0..$#$weights) {
        # Perturb weight i upward.
        my @w_plus = @$weights;
        $w_plus[$i] += $epsilon;

        # Perturb weight i downward.
        my @w_minus = @$weights;
        $w_minus[$i] -= $epsilon;

        my $loss_plus  = $loss_fn->(\@w_plus,  $inputs, $targets);
        my $loss_minus = $loss_fn->(\@w_minus, $inputs, $targets);
        push @grad, ($loss_plus - $loss_minus) / (2.0 * $epsilon);
    }
    return \@grad;
}

# ============================================================================
# train — Full gradient-descent training loop
# ============================================================================

=head2 train($weights, $inputs, $targets, $loss_fn, $loss_derivative_fn)

Run the gradient descent loop until convergence or C<max_iterations>.

  my ($trained_w, $err) = $gd->train(
      [0.0],          # initial weights
      $inputs,        # array-ref of input vectors
      $targets,       # array-ref of target values
      $loss_fn,       # sub($w, $inp, $tgt) → scalar
      $grad_fn,       # sub($w, $inp, $tgt) → arrayref  (optional)
  );

If C<$loss_derivative_fn> is C<undef>, numerical gradients are used.

Returns C<($trained_weights_arrayref, undef)> on success, or
C<(undef, $error)> on failure.

=cut

sub train {
    my ($self, $weights, $inputs, $targets, $loss_fn, $loss_derivative_fn) = @_;

    # Work on a copy to avoid mutating the caller's array.
    my @w = @$weights;

    my $prev_loss = $self->compute_loss(\@w, $inputs, $targets, $loss_fn);

    for my $_iter (1..$self->{max_iterations}) {
        # Compute gradient — analytical if provided, numerical otherwise.
        my $grad;
        if ($loss_derivative_fn) {
            $grad = $loss_derivative_fn->(\@w, $inputs, $targets);
        }
        else {
            $grad = $self->numerical_gradient(\@w, $inputs, $targets, $loss_fn);
        }

        # Apply the weight update.
        my ($new_w, $err) = $self->step(\@w, $grad);
        return (undef, $err) if $err;
        @w = @$new_w;

        # Check convergence.
        my $curr_loss = $self->compute_loss(\@w, $inputs, $targets, $loss_fn);
        if (abs($curr_loss - $prev_loss) < $self->{tolerance}) {
            last;
        }
        $prev_loss = $curr_loss;
    }

    return (\@w, undef);
}

1;

__END__

=head1 NAME

CodingAdventures::GradientDescent - Gradient descent weight optimiser

=head1 VERSION

0.01

=head1 SYNOPSIS

  use CodingAdventures::GradientDescent;

  my $gd = CodingAdventures::GradientDescent->new(
      learning_rate  => 0.1,
      max_iterations => 1000,
  );

  my ($trained, $err) = $gd->train(\@weights, \@inputs, \@targets, \&loss_fn);

=head1 DESCRIPTION

Implements stochastic gradient descent: iteratively adjusts model weights
to minimise a caller-supplied loss function.

=head1 METHODS

See inline documentation above.

=head1 LICENSE

MIT
