use strict;
use warnings;
use Test2::V0;

use CodingAdventures::LossFunctions qw(
    mse     mae     bce     cce
    mse_derivative  mae_derivative  bce_derivative  cce_derivative
);

# ---------------------------------------------------------------------------
# Helper: floating-point comparison with tolerance.
# ---------------------------------------------------------------------------
# We use a default tolerance of 1e-6.  This is more than sufficient for
# hand-computed golden values and accommodates the tiny rounding errors
# inherent to IEEE 754 double precision.
sub near {
    my ( $got, $expected, $tol ) = @_;
    $tol //= 1e-6;
    return abs( $got - $expected ) < $tol;
}

# ---------------------------------------------------------------------------
# Helper: numerical gradient via central differences.
# ---------------------------------------------------------------------------
# The central-difference formula approximates ∂L/∂ŷᵢ as:
#
#     (L(ŷᵢ + h) - L(ŷᵢ - h)) / (2h)
#
# We use h = 1e-5, which balances truncation error (wants h small) and
# round-off error (wants h not too small).
sub numerical_gradient {
    my ( $loss_fn, $y_true, $y_pred ) = @_;
    my $h   = 1e-5;
    my $n   = scalar @$y_pred;
    my @num;

    for my $i ( 0 .. $n - 1 ) {
        my @y_plus  = @$y_pred;
        my @y_minus = @$y_pred;
        $y_plus[$i]  += $h;
        $y_minus[$i] -= $h;
        my ($lp) = $loss_fn->( $y_true, \@y_plus  );
        my ($lm) = $loss_fn->( $y_true, \@y_minus );
        $num[$i] = ( $lp - $lm ) / ( 2.0 * $h );
    }
    return \@num;
}

# ---------------------------------------------------------------------------
# Check whether an array ref of gradients matches another within tolerance.
# ---------------------------------------------------------------------------
sub near_array {
    my ( $got, $expected, $tol ) = @_;
    $tol //= 1e-5;
    return 0 unless scalar @$got == scalar @$expected;
    for my $i ( 0 .. $#$got ) {
        return 0 unless near( $got->[$i], $expected->[$i], $tol );
    }
    return 1;
}

# ===========================================================================
# 1. VERSION
# ===========================================================================

ok( defined $CodingAdventures::LossFunctions::VERSION, 'VERSION is defined' );

# ===========================================================================
# 2. mse — forward
# ===========================================================================
# y_true = [1, 2, 3],  y_pred = [1.1, 1.9, 3.2]
# residuals: [-0.1, 0.1, -0.2]   squares: [0.01, 0.01, 0.04]
# MSE = 0.06 / 3 = 0.02

{
    my ( $loss, $err ) = mse( [1.0, 2.0, 3.0], [1.1, 1.9, 3.2] );
    ok( !defined $err,              'mse: no error for valid inputs' );
    ok( near( $loss, 0.02 ),        'mse: golden value 0.02' );
}

{
    my ( $loss, $err ) = mse( [1.0, 2.0], [1.0, 2.0] );
    ok( near( $loss, 0.0 ),         'mse: zero when predictions exact' );
}

{
    my ( $loss, $err ) = mse( [3.0], [5.0] );
    ok( near( $loss, 4.0 ),         'mse: single element (3-5)^2 = 4' );
}

{
    my ( undef, $err ) = mse( [1.0, 2.0], [1.0] );
    ok( defined $err,               'mse: error for mismatched lengths' );
    ok( $err,                       'mse: error string is non-empty' );
}

{
    my ( undef, $err ) = mse( [], [] );
    ok( defined $err,               'mse: error for empty arrays' );
}

{
    my ( undef, $err ) = mse( 42, [1.0] );
    ok( defined $err,               'mse: error when y_true is not an arrayref' );
}

# ===========================================================================
# 3. mae — forward
# ===========================================================================
# |residuals| = [0.1, 0.1, 0.2]   MAE = 0.4/3

{
    my ( $loss, $err ) = mae( [1.0, 2.0, 3.0], [1.1, 1.9, 3.2] );
    ok( !defined $err,              'mae: no error' );
    ok( near( $loss, 0.4 / 3.0 ),   'mae: golden value 0.4/3' );
}

{
    my ( $loss, $err ) = mae( [1.0, 2.0], [1.0, 2.0] );
    ok( near( $loss, 0.0 ),         'mae: zero when predictions exact' );
}

{
    my ( $loss, $err ) = mae( [3.0], [5.0] );
    ok( near( $loss, 2.0 ),         'mae: single element |3-5| = 2' );
}

{
    my ( undef, $err ) = mae( [1.0], [1.0, 2.0] );
    ok( defined $err,               'mae: error for mismatched lengths' );
}

{
    my ( undef, $err ) = mae( [], [] );
    ok( defined $err,               'mae: error for empty arrays' );
}

# ===========================================================================
# 4. bce — forward
# ===========================================================================
# y_true=[1,0,1], y_pred=[0.9,0.1,0.8]
# BCE = -(log(0.9) + log(0.9) + log(0.8)) / 3

{
    my @yt = ( 1.0, 0.0, 1.0 );
    my @yp = ( 0.9, 0.1, 0.8 );
    my $expected = -( log(0.9) + log(0.9) + log(0.8) ) / 3.0;
    my ( $loss, $err ) = bce( \@yt, \@yp );
    ok( !defined $err,                  'bce: no error' );
    ok( near( $loss, $expected ),        'bce: golden value' );
    ok( $loss >= 0,                      'bce: non-negative' );
}

{
    # Nearly perfect predictions → low loss.
    my ( $loss ) = bce( [1.0, 0.0], [0.9999, 0.0001] );
    ok( $loss < 0.01,               'bce: low loss for confident correct prediction' );
}

{
    # Worst-case predictions: model confident and wrong.
    my ( $loss ) = bce( [1.0, 0.0], [0.0001, 0.9999] );
    ok( $loss > 5.0,                'bce: high loss for confident wrong prediction' );
}

{
    # Predictions at 0 and 1 trigger clamping: result should be finite.
    my ( $loss, $err ) = bce( [1.0, 0.0], [0.0, 1.0] );
    ok( !defined $err,              'bce: no error for boundary predictions' );
    ok( defined $loss && $loss == $loss,  'bce: finite (not NaN) with clamping' );
}

{
    my ( undef, $err ) = bce( [1.0, 0.0], [0.9] );
    ok( defined $err,               'bce: error for mismatched lengths' );
}

# ===========================================================================
# 5. cce — forward
# ===========================================================================
# y_true=[0,1,0], y_pred=[0.2,0.7,0.1]
# CCE = -log(0.7) / 3

{
    my @yt = ( 0.0, 1.0, 0.0 );
    my @yp = ( 0.2, 0.7, 0.1 );
    my $expected = -log(0.7) / 3.0;
    my ( $loss, $err ) = cce( \@yt, \@yp );
    ok( !defined $err,                  'cce: no error' );
    ok( near( $loss, $expected ),        'cce: golden value' );
    ok( $loss >= 0,                      'cce: non-negative' );
}

{
    # Near-perfect: true class predicted at 0.999 → very low loss.
    my ( $loss ) = cce( [0.0, 1.0, 0.0], [0.001, 0.998, 0.001] );
    ok( $loss < 0.01,               'cce: low loss for near-perfect prediction' );
}

{
    # Zero prediction for true class → clamping prevents −Inf.
    my ( $loss, $err ) = cce( [0.0, 1.0, 0.0], [0.0, 0.0, 1.0] );
    ok( !defined $err,              'cce: no error for zero-prediction (clamped)' );
    ok( defined $loss && $loss == $loss, 'cce: finite with clamping' );
}

{
    my ( undef, $err ) = cce( [0.0, 1.0], [0.3, 0.3, 0.4] );
    ok( defined $err,               'cce: error for mismatched lengths' );
}

# ===========================================================================
# 6. mse_derivative
# ===========================================================================
# n=2, y_true=[1,2], y_pred=[3,4]
# grad[0] = (2/2)*(3-1) = 2.0
# grad[1] = (2/2)*(4-2) = 2.0

{
    my ( $grad, $err ) = mse_derivative( [1.0, 2.0], [3.0, 4.0] );
    ok( !defined $err,                      'mse_derivative: no error' );
    ok( near( $grad->[0], 2.0 ),             'mse_derivative: grad[0] = 2' );
    ok( near( $grad->[1], 2.0 ),             'mse_derivative: grad[1] = 2' );
}

{
    # Gradient = 0 at exact match.
    my ( $grad ) = mse_derivative( [1.0, 2.0, 3.0], [1.0, 2.0, 3.0] );
    ok( near( $grad->[0], 0.0 ),             'mse_derivative: zero at exact match [0]' );
    ok( near( $grad->[2], 0.0 ),             'mse_derivative: zero at exact match [2]' );
}

{
    # Over-prediction → positive gradient.
    my ( $grad ) = mse_derivative( [0.0], [1.0] );
    ok( $grad->[0] > 0,                      'mse_derivative: positive when over-predicted' );
}

{
    # Under-prediction → negative gradient.
    my ( $grad ) = mse_derivative( [1.0], [0.0] );
    ok( $grad->[0] < 0,                      'mse_derivative: negative when under-predicted' );
}

{
    # Numerical gradient check.
    my @yt = ( 1.0, 2.0, 3.0 );
    my @yp = ( 1.5, 2.5, 2.0 );
    my ( $analytic ) = mse_derivative( \@yt, \@yp );
    my $numeric      = numerical_gradient( \&mse, \@yt, \@yp );
    ok( near_array( $analytic, $numeric ),   'mse_derivative: matches numerical gradient' );
}

{
    my ( undef, $err ) = mse_derivative( [1.0], [1.0, 2.0] );
    ok( defined $err,                        'mse_derivative: error for mismatched lengths' );
}

# ===========================================================================
# 7. mae_derivative
# ===========================================================================

{
    # Both over-predicted → grad = +1/n = +0.5
    my ( $grad, $err ) = mae_derivative( [0.0, 0.0], [1.0, 2.0] );
    ok( !defined $err,                      'mae_derivative: no error' );
    ok( near( $grad->[0],  0.5 ),            'mae_derivative: +1/n when over-predicted [0]' );
    ok( near( $grad->[1],  0.5 ),            'mae_derivative: +1/n when over-predicted [1]' );
}

{
    # Both under-predicted → grad = -1/n = -0.5
    my ( $grad ) = mae_derivative( [1.0, 2.0], [0.0, 0.0] );
    ok( near( $grad->[0], -0.5 ),            'mae_derivative: -1/n when under-predicted [0]' );
    ok( near( $grad->[1], -0.5 ),            'mae_derivative: -1/n when under-predicted [1]' );
}

{
    # Exact match → 0
    my ( $grad ) = mae_derivative( [1.0, 2.0], [1.0, 2.0] );
    ok( near( $grad->[0], 0.0 ),             'mae_derivative: 0 at exact match [0]' );
    ok( near( $grad->[1], 0.0 ),             'mae_derivative: 0 at exact match [1]' );
}

{
    # Mixed: [over, under, exact, over]
    my ( $grad ) = mae_derivative( [0.0, 1.0, 1.0, 0.0], [1.0, 0.0, 1.0, 2.0] );
    ok( near( $grad->[0],  0.25 ),           'mae_derivative: mixed [0] over' );
    ok( near( $grad->[1], -0.25 ),           'mae_derivative: mixed [1] under' );
    ok( near( $grad->[2],  0.0  ),           'mae_derivative: mixed [2] exact' );
    ok( near( $grad->[3],  0.25 ),           'mae_derivative: mixed [3] over' );
}

# ===========================================================================
# 8. bce_derivative
# ===========================================================================

{
    my ( $grad, $err ) = bce_derivative( [1.0, 0.0, 1.0], [0.8, 0.2, 0.7] );
    ok( !defined $err,                          'bce_derivative: no error' );
    ok( scalar @$grad == 3,                     'bce_derivative: correct length' );
}

{
    # y=1, p=0.9 → grad = (1/1)*(0.9-1)/(0.9*0.1) = -1.111  → negative.
    my ( $grad ) = bce_derivative( [1.0], [0.9] );
    ok( $grad->[0] < 0,                         'bce_derivative: negative for y=1, p=0.9' );
}

{
    # y=0, p=0.9 → grad = (1/1)*(0.9-0)/(0.9*0.1) = +10  → positive.
    my ( $grad ) = bce_derivative( [0.0], [0.9] );
    ok( $grad->[0] > 0,                         'bce_derivative: positive for y=0, p=0.9' );
}

{
    # Boundary predictions → clamped → finite, no NaN.
    my ( $grad, $err ) = bce_derivative( [1.0, 0.0], [0.0, 1.0] );
    ok( !defined $err,                          'bce_derivative: no error at boundaries' );
    ok( $grad->[0] == $grad->[0],               'bce_derivative: no NaN [0]' );
    ok( $grad->[1] == $grad->[1],               'bce_derivative: no NaN [1]' );
}

{
    # Numerical gradient check.
    my @yt = ( 1.0, 0.0, 1.0 );
    my @yp = ( 0.7, 0.3, 0.6 );
    my ( $analytic ) = bce_derivative( \@yt, \@yp );
    my $numeric      = numerical_gradient( \&bce, \@yt, \@yp );
    ok( near_array( $analytic, $numeric, 1e-4 ), 'bce_derivative: matches numerical gradient' );
}

{
    my ( undef, $err ) = bce_derivative( [1.0], [0.5, 0.5] );
    ok( defined $err,                           'bce_derivative: error for mismatched lengths' );
}

# ===========================================================================
# 9. cce_derivative
# ===========================================================================

{
    my ( $grad, $err ) = cce_derivative( [0.0, 1.0, 0.0], [0.2, 0.7, 0.1] );
    ok( !defined $err,                          'cce_derivative: no error' );
    ok( scalar @$grad == 3,                     'cce_derivative: correct length' );
}

{
    # Non-true class (y=0) → gradient = 0.
    my ( $grad ) = cce_derivative( [0.0, 1.0, 0.0], [0.2, 0.7, 0.1] );
    ok( near( $grad->[0], 0.0 ),                'cce_derivative: grad=0 for non-true class [0]' );
    ok( near( $grad->[2], 0.0 ),                'cce_derivative: grad=0 for non-true class [2]' );
}

{
    # True class (y=1) → gradient < 0 (increasing prediction reduces loss).
    my ( $grad ) = cce_derivative( [0.0, 1.0, 0.0], [0.2, 0.7, 0.1] );
    ok( $grad->[1] < 0,                         'cce_derivative: negative for true class' );
}

{
    # Check analytic formula for true class: -(1/n)*(y/p) = -(1/3)*(1/0.7)
    my $n        = 3;
    my $expected = -(1.0 / $n) * (1.0 / 0.7);
    my ( $grad ) = cce_derivative( [0.0, 1.0, 0.0], [0.2, 0.7, 0.1] );
    ok( near( $grad->[1], $expected ),           'cce_derivative: analytic formula for true class' );
}

{
    # Numerical gradient check.
    my @yt = ( 0.0, 1.0, 0.0 );
    my @yp = ( 0.2, 0.7, 0.1 );
    my ( $analytic ) = cce_derivative( \@yt, \@yp );
    my $numeric      = numerical_gradient( \&cce, \@yt, \@yp );
    ok( near_array( $analytic, $numeric, 1e-4 ), 'cce_derivative: matches numerical gradient' );
}

{
    # Clamping for zero predictions: no NaN, no Inf.
    my ( $grad, $err ) = cce_derivative( [0.0, 1.0, 0.0], [0.0, 0.0, 1.0] );
    ok( !defined $err,                           'cce_derivative: no error for zero predictions' );
    ok( $grad->[1] == $grad->[1],                'cce_derivative: no NaN with clamping' );
}

# ===========================================================================
# 10. Gradient-descent consistency
# ===========================================================================
# Taking one gradient-descent step should not increase the loss.

sub check_descent {
    my ( $loss_fn, $deriv_fn, $y_true, $y_pred, $step ) = @_;
    $step //= 0.001;
    my ($before) = $loss_fn->( $y_true, $y_pred );
    my ($grad)   = $deriv_fn->( $y_true, $y_pred );
    my @y_new = map { $y_pred->[$_] - $step * $grad->[$_] } 0 .. $#$y_pred;
    my ($after)  = $loss_fn->( $y_true, \@y_new );
    return $after <= $before + 1e-9;
}

ok( check_descent( \&mse, \&mse_derivative, [1.0,2.0,3.0], [0.5,2.5,2.5] ),
    'mse: gradient descent reduces loss' );

ok( check_descent( \&mae, \&mae_derivative, [1.0,2.0], [0.0,3.0] ),
    'mae: gradient descent reduces loss' );

ok( check_descent( \&bce, \&bce_derivative, [1.0,0.0], [0.6,0.4] ),
    'bce: gradient descent reduces loss' );

ok( check_descent( \&cce, \&cce_derivative, [0.0,1.0,0.0], [0.3,0.5,0.2] ),
    'cce: gradient descent reduces loss' );

done_testing;
