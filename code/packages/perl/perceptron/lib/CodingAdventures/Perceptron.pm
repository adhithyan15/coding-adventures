package CodingAdventures::Perceptron;

# ============================================================================
# CodingAdventures::Perceptron — Single-layer perceptron neural network
# ============================================================================
#
# The perceptron (Rosenblatt, 1957) is the simplest model that can learn to
# classify linearly separable data.  It is the atomic building block of all
# modern neural networks.
#
# ## What a Perceptron Does
#
# Given n input features [x₁, x₂, …, xₙ]:
#
#   1. Compute the weighted sum:  z = w₁x₁ + w₂x₂ + … + wₙxₙ + b
#   2. Apply activation function: output = f(z)
#
# For binary classification with the step function:
#   - z > 0 → output = 1 (class 1)
#   - z ≤ 0 → output = 0 (class 0)
#
# ## The Perceptron Learning Rule
#
# When the prediction is wrong:
#
#   error        = target - prediction    (either +1 or -1)
#   w_new[i]     = w[i] + lr * error * x[i]
#   b_new        = b    + lr * error
#
# When correct: no update (error = 0).
#
# ## Convergence
#
# Novikoff's theorem (1962) guarantees convergence for linearly separable data.
# For non-separable data (like XOR), the rule oscillates — use logistic
# regression or a multi-layer network.
#
# ## Usage
#
#   use CodingAdventures::Perceptron;
#
#   my $p = CodingAdventures::Perceptron->new(n_inputs => 2, learning_rate => 0.1);
#
#   # Train AND gate
#   my @inputs  = ([0,0], [0,1], [1,0], [1,1]);
#   my @targets = (0, 0, 0, 1);
#   $p->train(\@inputs, \@targets, 200);
#
#   print $p->predict([1, 1]);   # 1
#   print $p->predict([0, 1]);   # 0
#
# ============================================================================

use strict;
use warnings;
use POSIX ();
use Carp  ();

our $VERSION = '0.01';

# ============================================================================
# Activation Functions
# ============================================================================

=head2 Activation functions

These are provided as class methods so callers can pass them as C<activation_fn>:

  activation_fn => \&CodingAdventures::Perceptron::step
  activation_fn => \&CodingAdventures::Perceptron::sigmoid

=cut

# step(z) — classic binary step function.
# Returns 1 if z > 0, else 0.
sub step {
    my ($z) = @_;
    return $z > 0 ? 1 : 0;
}

# sigmoid(z) = 1 / (1 + e^(-z)).  Range: (0, 1).
# Clamped for numerical stability: very negative z → 0, very positive z → 1.
sub sigmoid {
    my ($z) = @_;
    return 0.0 if $z < -709;
    return 1.0 if $z >  709;
    return 1.0 / (1.0 + exp(-$z));
}

# sigmoid_derivative(z) = sigmoid(z) * (1 - sigmoid(z)).
sub sigmoid_derivative {
    my ($z) = @_;
    my $s = sigmoid($z);
    return $s * (1.0 - $s);
}

# ============================================================================
# Constructor
# ============================================================================

=head2 new(%args)

Create a new Perceptron.

  my $p = CodingAdventures::Perceptron->new(
      n_inputs      => 2,            # required: number of input features
      learning_rate => 0.1,          # optional, default 0.1
      activation_fn => \&step,       # optional, default step
      weights       => [0.0, 0.0],   # optional, default all-zeros
      bias          => 0.0,          # optional, default 0.0
  );

=cut

sub new {
    my ($class, %args) = @_;

    Carp::croak("Perceptron->new: n_inputs is required and must be > 0")
        unless defined $args{n_inputs} && $args{n_inputs} > 0;

    my $n = $args{n_inputs};

    my $weights = $args{weights} // [(0.0) x $n];
    Carp::croak(sprintf(
        "Perceptron->new: weights length (%d) must equal n_inputs (%d)",
        scalar @$weights, $n
    )) unless scalar @$weights == $n;

    return bless {
        n_inputs      => $n,
        learning_rate => $args{learning_rate} // 0.1,
        activation_fn => $args{activation_fn} // \&step,
        weights       => [@$weights],    # defensive copy
        bias          => $args{bias}    // 0.0,
    }, $class;
}

# ============================================================================
# predict — Forward pass
# ============================================================================

=head2 predict($input)

Compute the perceptron's output for a single input vector.

  my ($output, $z) = $p->predict([1, 0]);

Returns C<($output, $z)> where C<$output> is the activation result and
C<$z> is the pre-activation weighted sum (useful for debugging).

=cut

sub predict {
    my ($self, $input) = @_;

    Carp::croak(sprintf(
        "predict: input length (%d) must equal n_inputs (%d)",
        scalar @$input, $self->{n_inputs}
    )) unless scalar @$input == $self->{n_inputs};

    # Compute weighted sum: z = Σ wᵢ·xᵢ + b
    my $z = $self->{bias};
    for my $i (0..$#$input) {
        $z += $self->{weights}[$i] * $input->[$i];
    }

    my $output = $self->{activation_fn}->($z);
    return ($output, $z);
}

# ============================================================================
# train_step — Single learning update
# ============================================================================

=head2 train_step($input, $target)

Perform one step of the Rosenblatt perceptron learning rule.

  my ($output, $error) = $p->train_step([1, 1], 1);

Updates C<< $p->{weights} >> and C<< $p->{bias} >> in place.
Returns C<($output, $error)> where C<$error = $target - $output>.

=cut

sub train_step {
    my ($self, $input, $target) = @_;

    my ($output, $_z) = $self->predict($input);
    my $err = $target - $output;

    if ($err != 0) {
        for my $i (0..$#$input) {
            $self->{weights}[$i] += $self->{learning_rate} * $err * $input->[$i];
        }
        $self->{bias} += $self->{learning_rate} * $err;
    }

    return ($output, $err);
}

# ============================================================================
# train — Full training loop
# ============================================================================

=head2 train($inputs, $targets, $epochs)

Run the perceptron learning rule for C<$epochs> complete passes through
the training set (default: 100 epochs).

  $p->train(\@inputs, \@targets, 200);

Mutates the perceptron in place and returns C<$self> for chaining.

=cut

sub train {
    my ($self, $inputs, $targets, $epochs) = @_;
    $epochs //= 100;

    Carp::croak(sprintf(
        "train: inputs length (%d) must equal targets length (%d)",
        scalar @$inputs, scalar @$targets
    )) unless scalar @$inputs == scalar @$targets;

    for my $_epoch (1..$epochs) {
        for my $i (0..$#$inputs) {
            $self->train_step($inputs->[$i], $targets->[$i]);
        }
    }

    return $self;
}

1;

__END__

=head1 NAME

CodingAdventures::Perceptron - Single-layer perceptron neural network

=head1 VERSION

0.01

=head1 SYNOPSIS

  use CodingAdventures::Perceptron;

  my $p = CodingAdventures::Perceptron->new(n_inputs => 2);
  $p->train([[0,0],[0,1],[1,0],[1,1]], [0,0,0,1], 200);
  my ($out) = $p->predict([1,1]);  # 1

=head1 DESCRIPTION

Single-layer perceptron with the Rosenblatt learning rule.  Guaranteed to
converge on linearly separable problems (AND, OR, NOT).

=head1 LICENSE

MIT
