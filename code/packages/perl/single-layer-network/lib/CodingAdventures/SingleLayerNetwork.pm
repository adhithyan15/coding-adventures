package CodingAdventures::SingleLayerNetwork;

use strict;
use warnings;

our $VERSION = '0.1.0';

sub _validate_matrix {
    my ($name, $matrix) = @_;
    die "$name must contain at least one row" unless @$matrix;
    my $width = scalar @{ $matrix->[0] };
    die "$name must contain at least one column" unless $width;
    for my $row (@$matrix) {
        die "$name must be rectangular" unless @$row == $width;
    }
    return (scalar @$matrix, $width);
}

sub _activate {
    my ($value, $activation) = @_;
    $activation ||= 'linear';
    return $value if $activation eq 'linear';
    if ($activation eq 'sigmoid') {
        if ($value >= 0) {
            my $z = exp(-$value);
            return 1.0 / (1.0 + $z);
        }
        my $z = exp($value);
        return $z / (1.0 + $z);
    }
    die "unsupported activation: $activation";
}

sub _derivative_from_output {
    my ($output, $activation) = @_;
    $activation ||= 'linear';
    return 1.0 if $activation eq 'linear';
    return $output * (1.0 - $output) if $activation eq 'sigmoid';
    die "unsupported activation: $activation";
}

sub predict_with_parameters {
    my ($inputs, $weights, $biases, $activation) = @_;
    my ($sample_count, $input_count) = _validate_matrix('inputs', $inputs);
    my ($weight_rows, $output_count) = _validate_matrix('weights', $weights);
    die 'input column count must match weight row count' unless $input_count == $weight_rows;
    die 'bias count must match output count' unless @$biases == $output_count;

    my @predictions;
    for my $row (0 .. $sample_count - 1) {
        my @prediction_row;
        for my $output (0 .. $output_count - 1) {
            my $total = $biases->[$output];
            for my $input (0 .. $input_count - 1) {
                $total += $inputs->[$row][$input] * $weights->[$input][$output];
            }
            push @prediction_row, _activate($total, $activation);
        }
        push @predictions, \@prediction_row;
    }
    return \@predictions;
}

sub train_one_epoch_with_matrices {
    my ($inputs, $targets, $weights, $biases, $learning_rate, $activation) = @_;
    my ($sample_count, $input_count) = _validate_matrix('inputs', $inputs);
    my ($target_rows, $output_count) = _validate_matrix('targets', $targets);
    my ($weight_rows, $weight_cols) = _validate_matrix('weights', $weights);
    die 'inputs and targets must have the same row count' unless $target_rows == $sample_count;
    die 'weights must be shaped input_count x output_count' unless $weight_rows == $input_count && $weight_cols == $output_count;
    die 'bias count must match output count' unless @$biases == $output_count;

    my $predictions = predict_with_parameters($inputs, $weights, $biases, $activation);
    my $scale = 2.0 / ($sample_count * $output_count);
    my (@errors, @deltas);
    my $loss_total = 0.0;
    for my $row (0 .. $sample_count - 1) {
        my (@error_row, @delta_row);
        for my $output (0 .. $output_count - 1) {
            my $error = $predictions->[$row][$output] - $targets->[$row][$output];
            push @error_row, $error;
            push @delta_row, $scale * $error * _derivative_from_output($predictions->[$row][$output], $activation);
            $loss_total += $error * $error;
        }
        push @errors, \@error_row;
        push @deltas, \@delta_row;
    }

    my (@weight_gradients, @next_weights);
    for my $input (0 .. $input_count - 1) {
        my (@gradient_row, @next_row);
        for my $output (0 .. $output_count - 1) {
            my $gradient = 0.0;
            for my $row (0 .. $sample_count - 1) {
                $gradient += $inputs->[$row][$input] * $deltas[$row][$output];
            }
            push @gradient_row, $gradient;
            push @next_row, $weights->[$input][$output] - $learning_rate * $gradient;
        }
        push @weight_gradients, \@gradient_row;
        push @next_weights, \@next_row;
    }

    my (@bias_gradients, @next_biases);
    for my $output (0 .. $output_count - 1) {
        my $gradient = 0.0;
        for my $row (0 .. $sample_count - 1) {
            $gradient += $deltas[$row][$output];
        }
        push @bias_gradients, $gradient;
        push @next_biases, $biases->[$output] - $learning_rate * $gradient;
    }

    return {
        predictions      => $predictions,
        errors           => \@errors,
        weight_gradients => \@weight_gradients,
        bias_gradients   => \@bias_gradients,
        next_weights     => \@next_weights,
        next_biases      => \@next_biases,
        loss             => $loss_total / ($sample_count * $output_count),
    };
}

sub new {
    my ($class, %args) = @_;
    my $input_count = $args{input_count};
    my $output_count = $args{output_count};
    my @weights = map { [ (0.0) x $output_count ] } 1 .. $input_count;
    my @biases = (0.0) x $output_count;
    return bless {
        weights => \@weights,
        biases => \@biases,
        activation => $args{activation} || 'linear',
    }, $class;
}

sub predict {
    my ($self, $inputs) = @_;
    return predict_with_parameters($inputs, $self->{weights}, $self->{biases}, $self->{activation});
}

sub fit {
    my ($self, $inputs, $targets, %args) = @_;
    my $learning_rate = $args{learning_rate} // 0.05;
    my $epochs = $args{epochs} // 100;
    my @history;
    for (1 .. $epochs) {
        my $step = train_one_epoch_with_matrices($inputs, $targets, $self->{weights}, $self->{biases}, $learning_rate, $self->{activation});
        $self->{weights} = $step->{next_weights};
        $self->{biases} = $step->{next_biases};
        push @history, $step;
    }
    return \@history;
}

1;
