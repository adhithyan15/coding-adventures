use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Polynomial qw(
    normalize degree zero one
    add subtract multiply divmod_poly divide modulo
    evaluate gcd_poly
);

# ============================================================================
# normalize
# ============================================================================
# normalize() strips trailing near-zero coefficients.  The invariant is:
# the returned array always has at least one element (so the zero polynomial
# is [0], never []).

subtest 'normalize — basic cases' => sub {
    # Already normalized: no change
    is( normalize([1, 2, 3]), [1, 2, 3], 'normalize: no trailing zeros' );

    # Single zero should stay as [0]
    is( normalize([0]), [0], 'normalize: [0] stays [0]' );

    # Trailing zeros stripped
    is( normalize([1, 0, 0]), [1], 'normalize: strips trailing zeros' );
    is( normalize([0, 0, 0]), [0], 'normalize: all-zero collapses to [0]' );

    # Middle zeros preserved
    is( normalize([0, 1, 0, 1]), [0, 1, 0, 1], 'normalize: interior zeros preserved' );

    # Near-zero threshold: 1e-11 is below 1e-10 and should be stripped
    is( normalize([1, 1e-11]), [1], 'normalize: near-zero stripped (1e-11)' );

    # 1e-9 is above threshold and should be kept
    is( normalize([1, 1e-9]), [1, 1e-9], 'normalize: 1e-9 kept (above threshold)' );
};

# ============================================================================
# degree
# ============================================================================

subtest 'degree' => sub {
    is( degree([0]),       0, 'degree: zero polynomial is 0' );
    is( degree([1]),       0, 'degree: constant non-zero is 0' );
    is( degree([0, 1]),    1, 'degree: x has degree 1' );
    is( degree([1, 2, 3]), 2, 'degree: 3x^2+2x+1 has degree 2' );
    is( degree([0, 0, 0, 0, 5]), 4, 'degree: 5x^4 has degree 4' );
    is( degree([1, 0, 0]),  0, 'degree: [1,0,0] normalizes to constant' );
};

# ============================================================================
# zero / one
# ============================================================================

subtest 'zero and one' => sub {
    is( zero(), [0], 'zero() returns [0]' );
    is( one(),  [1], 'one() returns [1]' );

    # zero is additive identity
    is( add(zero(), [3, 2, 1]), [3, 2, 1], 'zero additive identity (left)' );
    is( add([3, 2, 1], zero()), [3, 2, 1], 'zero additive identity (right)' );

    # one is multiplicative identity
    is( multiply(one(), [3, 2, 1]), [3, 2, 1], 'one multiplicative identity (left)' );
    is( multiply([3, 2, 1], one()), [3, 2, 1], 'one multiplicative identity (right)' );
};

# ============================================================================
# add
# ============================================================================
# Polynomial addition is the simplest operation: add corresponding coefficients.
# If one polynomial is shorter it is padded with zeros at the high end.

subtest 'add' => sub {
    # [1,2,3] + [5,4] = [6,6,3]  i.e. (3x^2+2x+1) + (4x+5) = 3x^2+6x+6
    is( add([1,2,3], [5,4]), [6,6,3], 'add: different lengths' );

    # Adding zero polynomial
    is( add([1,2], [0]), [1,2], 'add: + zero poly' );

    # Commutativity
    is( add([1,2,3], [4,5]), add([4,5], [1,2,3]), 'add: commutative' );

    # Cancellation: p + (-p) = 0
    is( add([1,2,3], [-1,-2,-3]), [0], 'add: cancellation gives zero' );

    # Same-length addition
    is( add([1,1], [1,1]), [2,2], 'add: same-length' );

    # Result is normalized
    is( add([1,2,3], [-1,-2,-3]), [0], 'add: result is normalized' );
};

# ============================================================================
# subtract
# ============================================================================

subtest 'subtract' => sub {
    # (3x^2+2x+1) - (4x+5) = 3x^2 - 2x - 4  i.e. [1,2,3] - [5,4] = [-4,-2,3]
    is( subtract([1,2,3], [5,4]), [-4,-2,3], 'subtract: different lengths' );

    # p - p = 0
    is( subtract([1,2,3], [1,2,3]), [0], 'subtract: self gives zero' );

    # p - 0 = p
    is( subtract([1,2], [0]), [1,2], 'subtract: minus zero poly' );

    # Result normalized when leading terms cancel
    is( subtract([1,2,3], [0,0,3]), [1,2], 'subtract: normalizes after leading cancel' );
};

# ============================================================================
# multiply
# ============================================================================
# Polynomial multiplication uses the convolution of coefficient arrays.
# (x+2)(x+3) = x^2 + 5x + 6  → [2,1]*[3,1] = [6,5,1]

subtest 'multiply' => sub {
    is( multiply([2,1], [3,1]), [6,5,1], 'multiply: (x+2)(x+3)=x^2+5x+6' );

    # Multiply by zero → zero
    is( multiply([1,2,3], [0]), [0], 'multiply: by zero gives zero' );

    # Multiply by one → unchanged
    is( multiply([1,2,3], [1]), [1,2,3], 'multiply: by one unchanged' );

    # Squaring: (x+1)^2 = x^2+2x+1 → [1,1]*[1,1] = [1,2,1]
    is( multiply([1,1], [1,1]), [1,2,1], 'multiply: (x+1)^2 = x^2+2x+1' );

    # Degree property: deg(a*b) = deg(a) + deg(b)
    my $a = [1,0,1];  # x^2 + 1, degree 2
    my $b = [1,1];    # x+1, degree 1
    my $prod = multiply($a, $b);
    is( degree($prod), 3, 'multiply: degree adds' );

    # Commutativity
    is( multiply([1,2], [3,4]), multiply([3,4], [1,2]), 'multiply: commutative' );
};

# ============================================================================
# divmod_poly
# ============================================================================
# Long division: dividend = quotient * divisor + remainder
# with degree(remainder) < degree(divisor).

subtest 'divmod_poly' => sub {
    # x^2 - 1 divided by x - 1:
    # [−1, 0, 1] ÷ [−1, 1]
    # quotient = [1, 1] (x+1), remainder = [0] (zero)
    my ($q, $r) = divmod_poly([-1,0,1], [-1,1]);
    is( $q, [1,1], 'divmod: (x^2-1)/(x-1) quotient=x+1' );
    is( $r, [0],   'divmod: (x^2-1)/(x-1) remainder=0' );

    # x^2 + x + 1 divided by x + 1:
    # [1,1,1] ÷ [1,1]
    # (x^2+x+1) = (x)(x+1) + 1 → quotient=[0,1], remainder=[1]
    ($q, $r) = divmod_poly([1,1,1], [1,1]);
    is( $q, [0,1], 'divmod: (x^2+x+1)/(x+1) quotient=x' );
    is( $r, [1],   'divmod: (x^2+x+1)/(x+1) remainder=1' );

    # When dividend degree < divisor degree: quotient=0, remainder=dividend
    ($q, $r) = divmod_poly([1,2], [1,0,1]);  # (2x+1) / (x^2+1)
    is( $q, [0],   'divmod: low-degree dividend quotient=0' );
    is( $r, [1,2], 'divmod: low-degree dividend remainder=dividend' );

    # Division by zero polynomial dies
    ok( dies { divmod_poly([1,2], [0]) }, 'divmod: zero divisor dies' );

    # Verify dividend = quotient * divisor + remainder invariant
    my $dividend = [6, 11, 6, 1];  # x^3 + 6x^2 + 11x + 6
    my $divisor  = [2, 1];         # x + 2
    ($q, $r) = divmod_poly($dividend, $divisor);
    my $reconstructed = add( multiply($q, $divisor), $r );
    # Compare all coefficients
    is( $reconstructed, $dividend, 'divmod: dividend = quot*div + rem' );
};

# ============================================================================
# divide and modulo
# ============================================================================

subtest 'divide' => sub {
    # (x^2-1) / (x-1) = x+1
    my $q = divide([-1,0,1], [-1,1]);
    is( $q, [1,1], 'divide: (x^2-1)/(x-1) = x+1' );
};

subtest 'modulo' => sub {
    # (x^2+x+1) mod (x+1) = 1
    my $r = modulo([1,1,1], [1,1]);
    is( $r, [1], 'modulo: (x^2+x+1) mod (x+1) = 1' );

    # x^2-1 mod x-1 = 0
    $r = modulo([-1,0,1], [-1,1]);
    is( $r, [0], 'modulo: (x^2-1) mod (x-1) = 0' );
};

# ============================================================================
# evaluate
# ============================================================================
# Horner's method gives exact results for exact integer coefficients.

subtest 'evaluate' => sub {
    # Constant polynomial: always returns that constant
    is( evaluate([7], 100), 7, 'evaluate: constant polynomial' );

    # Linear: 2x + 3 at x=4 → 11
    is( evaluate([3, 2], 4), 11, 'evaluate: linear 2x+3 at x=4' );

    # Quadratic: 3x^2 + 2x + 1 at x=2 → 17
    is( evaluate([1, 2, 3], 2), 17, 'evaluate: 3x^2+2x+1 at x=2' );

    # Evaluate at x=0 gives constant term
    is( evaluate([5, 3, 2], 0), 5, 'evaluate: at x=0 gives constant term' );

    # Evaluate at x=1: sum of all coefficients
    is( evaluate([1, 2, 3], 1), 6, 'evaluate: at x=1 is sum of coefficients' );

    # Evaluate at negative x
    is( evaluate([1, 0, 1], -1), 2, 'evaluate: x^2+1 at x=-1 is 2' );

    # Root check: x^2 - 1 at x=1 should be 0
    is( evaluate([-1, 0, 1], 1), 0, 'evaluate: x=1 is root of x^2-1' );

    # Root check: x^2 - 1 at x=-1 should be 0
    is( evaluate([-1, 0, 1], -1), 0, 'evaluate: x=-1 is root of x^2-1' );
};

# ============================================================================
# gcd_poly
# ============================================================================
# The GCD of two polynomials is the highest-degree polynomial that divides both.
# Result is always monic (leading coefficient 1).

subtest 'gcd_poly' => sub {
    # gcd(x^2-1, x-1) = x-1
    # x^2-1 = (x-1)(x+1), and x-1 divides x-1 trivially.
    my $g = gcd_poly([-1,0,1], [-1,1]);
    is( $g, [-1,1], 'gcd: gcd(x^2-1, x-1) = x-1' );

    # gcd(x^2-1, x+1) = x+1
    $g = gcd_poly([-1,0,1], [1,1]);
    is( $g, [1,1], 'gcd: gcd(x^2-1, x+1) = x+1' );

    # gcd(x^2-1, x^2-1) = x^2-1  (monic)
    $g = gcd_poly([-1,0,1], [-1,0,1]);
    is( $g, [-1,0,1], 'gcd: gcd(p, p) = monic p' );

    # gcd of coprime polynomials = 1
    # x+1 and x+2 are coprime (no common root)
    $g = gcd_poly([1,1], [2,1]);
    is( $g, [1], 'gcd: coprime polynomials give 1' );

    # gcd(p, 0) = monic(p)   [gcd with zero]
    $g = gcd_poly([2,2], [0]);
    is( $g, [1,1], 'gcd: gcd(2x+2, 0) = x+1 (monic)' );

    # gcd(x^3-x, x^2-1) = x^2-1  because x^3-x = x(x^2-1)
    $g = gcd_poly([0,-1,0,1], [-1,0,1]);
    is( $g, [-1,0,1], 'gcd: gcd(x^3-x, x^2-1) = x^2-1' );
};

done_testing;
