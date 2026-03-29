use strict;
use warnings;
use Test2::V0;

# ---------------------------------------------------------------------------
# Smoke test: does the module compile and load?
# ---------------------------------------------------------------------------

use_ok('CodingAdventures::GradientDescent');

my @methods = qw(new step compute_loss numerical_gradient train);
for my $m (@methods) {
    ok(
        CodingAdventures::GradientDescent->can($m),
        "CodingAdventures::GradientDescent can $m"
    );
}

ok(defined $CodingAdventures::GradientDescent::VERSION, 'has VERSION');

done_testing();
