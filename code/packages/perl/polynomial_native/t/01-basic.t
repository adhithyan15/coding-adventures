use strict;
use warnings;
use Test2::V0;
use FindBin;

# t/01-basic.t -- exercises the Rust-backed polynomial XSUBs through the real
# DynaLoader path: the cdylib staged at
#     lib/auto/CodingAdventures/PolynomialNative/PolynomialNative.<dlext>
# is loaded by `bootstrap`.  The BUILD recipe on every OS (Unix `BUILD`,
# Windows `BUILD_windows`) stages that library before running prove, so a
# failure to load it is a real failure here, never a skip.
#
# A polynomial is an array reference of coefficients, lowest power first:
# [1, 2, 3] is 1 + 2x + 3x^2.

use lib "$FindBin::Bin/../lib";

require CodingAdventures::PolynomialNative;
my $P = 'CodingAdventures::PolynomialNative';

sub poly {
    my ($name) = @_;
    my $code = $P->can($name) or die "no $name";
    return $code;
}

subtest 'constants' => sub {
    is(poly('zero')->(), [0], 'zero');
    is(poly('one')->(),  [1], 'one');
};

subtest 'normalize and degree' => sub {
    is(poly('normalize')->([1, 2, 0, 0]), [1, 2], 'trailing zeros removed');
    is(poly('degree')->([1, 2, 3]), 2, 'degree of a quadratic');
    is(poly('degree')->([5]),       0, 'degree of a constant');
};

subtest 'add, subtract, multiply' => sub {
    is(poly('add')->([1, 2], [3, 4]),           [4, 6],       '(1+2x) + (3+4x)');
    is(poly('subtract')->([5, 7, 1], [2, 7, 1]), [3],         'high terms cancel and normalize');
    is(poly('multiply')->([1, 1], [1, 1]),       [1, 2, 1],   '(1+x)^2');
    is(poly('multiply')->([1, 2, 3], [4, 5]),    [4, 13, 22, 15], '(1+2x+3x^2)(4+5x)');
};

subtest 'evaluate (Horner)' => sub {
    is(poly('evaluate')->([1, 2, 3], 2), 17, '1 + 2*2 + 3*4');
    is(poly('evaluate')->([7], 100),     7,  'constant');
};

subtest 'division family' => sub {
    # x^2 - 1 = (x - 1)(x + 1)
    is(poly('divide')->([-1, 0, 1], [1, 1]), [-1, 1], 'quotient of x^2-1 by x+1');
    is(poly('modulo')->([-1, 0, 1], [1, 1]), [],      'exact division leaves the (normalized, empty) zero remainder');
    is(poly('modulo')->([2, 0, 1], [1, 1]),  [3],     'remainder of x^2+2 by x+1');
};

# The Rust side turns bad input into a Perl `die`; this runs last so that, if the
# error path ever misbehaved on some platform, every assertion above has already
# reported.
subtest 'bad input dies instead of crashing the interpreter' => sub {
    like(dies { poly('add')->(1, [1]) },              qr/array reference/, 'non-reference argument');
    like(dies { poly('divide')->([1, 1], [0]) },      qr/zero polynomial/, 'zero divisor');
};

done_testing;
