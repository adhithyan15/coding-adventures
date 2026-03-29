use strict;
use warnings;
use Test2::V0;

# ---------------------------------------------------------------------------
# Smoke test: verify the module loads and exports all expected symbols.
# ---------------------------------------------------------------------------
# A "smoke test" is the minimal sanity check: does it even compile and load?
# If this fails, all other tests are moot. Running this separately ensures
# fast feedback on syntax errors or missing dependencies.

ok(eval { require CodingAdventures::ActivationFunctions; 1 }, 'CodingAdventures::ActivationFunctions loads');

# Verify all exported functions exist in the module's namespace
my @expected_exports = qw(
    sigmoid             sigmoid_derivative
    relu                relu_derivative
    tanh_activation     tanh_derivative
    leaky_relu          leaky_relu_derivative
    elu                 elu_derivative
    softmax             softmax_derivative
);

for my $fn (@expected_exports) {
    ok(
        CodingAdventures::ActivationFunctions->can($fn),
        "CodingAdventures::ActivationFunctions can $fn"
    );
}

done_testing();
