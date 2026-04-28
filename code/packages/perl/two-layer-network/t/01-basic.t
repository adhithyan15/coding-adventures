use strict;
use warnings;
use Test2::V0;

use CodingAdventures::TwoLayerNetwork;

my $inputs = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]];
my $targets = [[0.0], [1.0], [1.0], [0.0]];
my $pass = CodingAdventures::TwoLayerNetwork::forward($inputs, CodingAdventures::TwoLayerNetwork::xor_warm_start_parameters());

is(scalar @{ $pass->{hidden_activations} }, 4, 'hidden activation row count');
is(scalar @{ $pass->{hidden_activations}[0] }, 2, 'hidden activation width');
ok($pass->{predictions}[1][0] > 0.7, 'true XOR row is high');
ok($pass->{predictions}[0][0] < 0.3, 'false XOR row is low');

my $step = CodingAdventures::TwoLayerNetwork::train_one_epoch(
    $inputs,
    $targets,
    CodingAdventures::TwoLayerNetwork::xor_warm_start_parameters(),
    0.5,
);

is(scalar @{ $step->{input_to_hidden_weight_gradients} }, 2, 'input-to-hidden gradient rows');
is(scalar @{ $step->{hidden_to_output_weight_gradients}[0] }, 1, 'hidden-to-output gradient width');

my @cases = (
    ['XNOR', $inputs, [[1.0], [0.0], [0.0], [1.0]], 3],
    ['absolute value', [[-1.0], [-0.5], [0.0], [0.5], [1.0]], [[1.0], [0.5], [0.0], [0.5], [1.0]], 4],
    ['piecewise pricing', [[0.1], [0.3], [0.5], [0.7], [0.9]], [[0.12], [0.25], [0.55], [0.88], [0.88]], 4],
    ['circle classifier', [[0.0, 0.0], [0.5, 0.0], [1.0, 1.0], [-0.5, 0.5], [-1.0, 0.0]], [[1.0], [1.0], [0.0], [1.0], [0.0]], 5],
    ['two moons', [[1.0, 0.0], [0.0, 0.5], [0.5, 0.85], [0.5, -0.35], [-1.0, 0.0], [2.0, 0.5]], [[0.0], [1.0], [0.0], [1.0], [0.0], [1.0]], 5],
    ['interaction features', [[0.2, 0.25, 0.0], [0.6, 0.5, 1.0], [1.0, 0.75, 1.0], [1.0, 1.0, 0.0]], [[0.08], [0.72], [0.96], [0.76]], 5],
);

for my $case (@cases) {
    my ($name, $example_inputs, $example_targets, $hidden_count) = @$case;
    my $example_step = CodingAdventures::TwoLayerNetwork::train_one_epoch(
        $example_inputs,
        $example_targets,
        sample_parameters(scalar @{ $example_inputs->[0] }, $hidden_count),
        0.4,
    );

    ok($example_step->{loss} >= 0.0, "$name loss is finite");
    is(scalar @{ $example_step->{input_to_hidden_weight_gradients} }, scalar @{ $example_inputs->[0] }, "$name input gradient shape");
    is(scalar @{ $example_step->{hidden_to_output_weight_gradients} }, $hidden_count, "$name hidden gradient shape");
}

done_testing;

sub sample_parameters {
    my ($input_count, $hidden_count) = @_;
    my @input_to_hidden;
    for my $feature (0 .. $input_count - 1) {
        my @row;
        for my $hidden (0 .. $hidden_count - 1) {
            push @row, 0.17 * ($feature + 1) - 0.11 * ($hidden + 1);
        }
        push @input_to_hidden, \@row;
    }
    my @hidden_biases = map { 0.05 * ($_ - 1) } 0 .. $hidden_count - 1;
    my @hidden_to_output = map { [0.13 * ($_ + 1) - 0.25] } 0 .. $hidden_count - 1;

    return {
        input_to_hidden_weights => \@input_to_hidden,
        hidden_biases => \@hidden_biases,
        hidden_to_output_weights => \@hidden_to_output,
        output_biases => [0.02],
    };
}
