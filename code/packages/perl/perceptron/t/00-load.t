use strict;
use warnings;
use Test2::V0;

use_ok('CodingAdventures::Perceptron');

my @methods = qw(new predict train_step train step sigmoid sigmoid_derivative);
for my $m (@methods) {
    ok(
        CodingAdventures::Perceptron->can($m),
        "CodingAdventures::Perceptron can $m"
    );
}

ok(defined $CodingAdventures::Perceptron::VERSION, 'has VERSION');

done_testing();
