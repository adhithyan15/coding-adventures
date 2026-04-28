use strict;
use warnings;
use Test2::V0;

use CodingAdventures::SingleLayerNetwork;

sub near {
    my ($actual, $expected) = @_;
    ok(abs($actual - $expected) <= 1e-6, "$actual ~= $expected");
}

my $step = CodingAdventures::SingleLayerNetwork::train_one_epoch_with_matrices(
    [[1.0, 2.0]],
    [[3.0, 5.0]],
    [[0.0, 0.0], [0.0, 0.0]],
    [0.0, 0.0],
    0.1,
);

is($step->{predictions}, [[0.0, 0.0]], 'predictions');
is($step->{errors}, [[-3.0, -5.0]], 'errors');
is($step->{weight_gradients}, [[-3.0, -5.0], [-6.0, -10.0]], 'gradients');
near($step->{next_weights}[0][0], 0.3);
near($step->{next_weights}[1][1], 1.0);

my $model = CodingAdventures::SingleLayerNetwork->new(input_count => 3, output_count => 2);
my $history = $model->fit(
    [[0.0, 0.0, 1.0], [1.0, 2.0, 1.0], [2.0, 1.0, 1.0]],
    [[1.0, -1.0], [3.0, 2.0], [4.0, 1.0]],
    learning_rate => 0.05,
    epochs => 500,
);
ok($history->[-1]{loss} < $history->[0]{loss}, 'loss improves');
is(scalar @{ $model->predict([[1.0, 1.0, 1.0]])->[0] }, 2, 'two outputs');

done_testing;
