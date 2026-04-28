package CodingAdventures::TwoLayerNetwork;

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
    $activation ||= 'sigmoid';
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

sub _derivative {
    my ($raw, $activated, $activation) = @_;
    $activation ||= 'sigmoid';
    return 1.0 if $activation eq 'linear';
    return $activated * (1.0 - $activated) if $activation eq 'sigmoid';
    die "unsupported activation: $activation";
}

sub _dot {
    my ($left, $right) = @_;
    my ($rows, $width) = _validate_matrix('left', $left);
    my ($right_rows, $cols) = _validate_matrix('right', $right);
    die 'matrix shapes do not align' unless $width == $right_rows;
    my @result;
    for my $row (0 .. $rows - 1) {
        my @result_row;
        for my $col (0 .. $cols - 1) {
            my $sum = 0.0;
            for my $k (0 .. $width - 1) {
                $sum += $left->[$row][$k] * $right->[$k][$col];
            }
            push @result_row, $sum;
        }
        push @result, \@result_row;
    }
    return \@result;
}

sub _transpose {
    my ($matrix) = @_;
    my ($rows, $cols) = _validate_matrix('matrix', $matrix);
    my @result;
    for my $col (0 .. $cols - 1) {
        my @row;
        for my $source_row (0 .. $rows - 1) {
            push @row, $matrix->[$source_row][$col];
        }
        push @result, \@row;
    }
    return \@result;
}

sub _add_biases {
    my ($matrix, $biases) = @_;
    return [map {
        my $row = $_;
        [map { $row->[$_] + $biases->[$_] } 0 .. @$row - 1]
    } @$matrix];
}

sub _apply_activation {
    my ($matrix, $activation) = @_;
    return [map { [map { _activate($_, $activation) } @$_] } @$matrix];
}

sub _column_sums {
    my ($matrix) = @_;
    my ($rows, $cols) = _validate_matrix('matrix', $matrix);
    my @sums = (0.0) x $cols;
    for my $row (@$matrix) {
        for my $col (0 .. $cols - 1) {
            $sums[$col] += $row->[$col];
        }
    }
    return \@sums;
}

sub _mse {
    my ($errors) = @_;
    my ($sum, $count) = (0.0, 0);
    for my $row (@$errors) {
        for my $value (@$row) {
            $sum += $value * $value;
            $count++;
        }
    }
    return $sum / $count;
}

sub _subtract_scaled {
    my ($matrix, $gradients, $learning_rate) = @_;
    my @result;
    for my $row (0 .. @$matrix - 1) {
        my @next_row;
        for my $col (0 .. @{ $matrix->[$row] } - 1) {
            push @next_row, $matrix->[$row][$col] - $learning_rate * $gradients->[$row][$col];
        }
        push @result, \@next_row;
    }
    return \@result;
}

sub xor_warm_start_parameters {
    return {
        input_to_hidden_weights => [[4.0, -4.0], [4.0, -4.0]],
        hidden_biases => [-2.0, 6.0],
        hidden_to_output_weights => [[4.0], [4.0]],
        output_biases => [-6.0],
    };
}

sub forward {
    my ($inputs, $parameters, $hidden_activation, $output_activation) = @_;
    $hidden_activation ||= 'sigmoid';
    $output_activation ||= 'sigmoid';
    my $hidden_raw = _add_biases(_dot($inputs, $parameters->{input_to_hidden_weights}), $parameters->{hidden_biases});
    my $hidden_activations = _apply_activation($hidden_raw, $hidden_activation);
    my $output_raw = _add_biases(_dot($hidden_activations, $parameters->{hidden_to_output_weights}), $parameters->{output_biases});
    my $predictions = _apply_activation($output_raw, $output_activation);
    return {
        hidden_raw => $hidden_raw,
        hidden_activations => $hidden_activations,
        output_raw => $output_raw,
        predictions => $predictions,
    };
}

sub train_one_epoch {
    my ($inputs, $targets, $parameters, $learning_rate, $hidden_activation, $output_activation) = @_;
    $hidden_activation ||= 'sigmoid';
    $output_activation ||= 'sigmoid';
    my ($sample_count) = _validate_matrix('inputs', $inputs);
    my (undef, $output_count) = _validate_matrix('targets', $targets);
    my $pass = forward($inputs, $parameters, $hidden_activation, $output_activation);
    my $scale = 2.0 / ($sample_count * $output_count);
    my (@errors, @output_deltas);
    for my $row (0 .. $sample_count - 1) {
        my (@error_row, @delta_row);
        for my $output (0 .. $output_count - 1) {
            my $error = $pass->{predictions}[$row][$output] - $targets->[$row][$output];
            push @error_row, $error;
            push @delta_row, $scale * $error * _derivative($pass->{output_raw}[$row][$output], $pass->{predictions}[$row][$output], $output_activation);
        }
        push @errors, \@error_row;
        push @output_deltas, \@delta_row;
    }
    my $h2o_gradients = _dot(_transpose($pass->{hidden_activations}), \@output_deltas);
    my $output_bias_gradients = _column_sums(\@output_deltas);
    my $hidden_errors = _dot(\@output_deltas, _transpose($parameters->{hidden_to_output_weights}));
    my $hidden_width = scalar @{ $parameters->{hidden_biases} };
    my @hidden_deltas;
    for my $row (0 .. $sample_count - 1) {
        my @delta_row;
        for my $hidden (0 .. $hidden_width - 1) {
            push @delta_row, $hidden_errors->[$row][$hidden] * _derivative($pass->{hidden_raw}[$row][$hidden], $pass->{hidden_activations}[$row][$hidden], $hidden_activation);
        }
        push @hidden_deltas, \@delta_row;
    }
    my $i2h_gradients = _dot(_transpose($inputs), \@hidden_deltas);
    my $hidden_bias_gradients = _column_sums(\@hidden_deltas);
    return {
        predictions => $pass->{predictions},
        errors => \@errors,
        output_deltas => \@output_deltas,
        hidden_deltas => \@hidden_deltas,
        hidden_to_output_weight_gradients => $h2o_gradients,
        output_bias_gradients => $output_bias_gradients,
        input_to_hidden_weight_gradients => $i2h_gradients,
        hidden_bias_gradients => $hidden_bias_gradients,
        next_parameters => {
            input_to_hidden_weights => _subtract_scaled($parameters->{input_to_hidden_weights}, $i2h_gradients, $learning_rate),
            hidden_biases => [map { $parameters->{hidden_biases}[$_] - $learning_rate * $hidden_bias_gradients->[$_] } 0 .. $hidden_width - 1],
            hidden_to_output_weights => _subtract_scaled($parameters->{hidden_to_output_weights}, $h2o_gradients, $learning_rate),
            output_biases => [map { $parameters->{output_biases}[$_] - $learning_rate * $output_bias_gradients->[$_] } 0 .. $output_count - 1],
        },
        loss => _mse(\@errors),
    };
}

1;
