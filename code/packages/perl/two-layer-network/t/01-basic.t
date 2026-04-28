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

done_testing;
