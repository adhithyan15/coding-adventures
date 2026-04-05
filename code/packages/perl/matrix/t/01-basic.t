use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Matrix;

# Short alias for convenience.
my $M = 'CodingAdventures::Matrix';

# ---------------------------------------------------------------------------
# Helper: floating-point comparison with tolerance.
# ---------------------------------------------------------------------------
sub near {
    my ( $got, $expected, $tol ) = @_;
    $tol //= 1e-10;
    return abs( $got - $expected ) < $tol;
}

# ---------------------------------------------------------------------------
# Helper: check whether two Matrix objects have the same shape and all
# elements within tolerance.
# ---------------------------------------------------------------------------
sub mat_equal {
    my ( $A, $B, $tol ) = @_;
    $tol //= 1e-10;
    return 0 unless $A->rows == $B->rows && $A->cols == $B->cols;
    for my $i ( 0 .. $A->rows - 1 ) {
        for my $j ( 0 .. $A->cols - 1 ) {
            return 0 unless near( $A->get($i,$j), $B->get($i,$j), $tol );
        }
    }
    return 1;
}

# ===========================================================================
# 1. VERSION
# ===========================================================================

ok( defined $CodingAdventures::Matrix::VERSION, 'VERSION is defined' );

# ===========================================================================
# 2. zeros
# ===========================================================================

{
    my $A = $M->zeros(3, 4);
    ok( $A->rows == 3,  'zeros: correct rows' );
    ok( $A->cols == 4,  'zeros: correct cols' );
    my $all_zero = 1;
    for my $i (0..2) { for my $j (0..3) { $all_zero = 0 unless near($A->get($i,$j), 0.0) } }
    ok( $all_zero,      'zeros: all elements are 0' );
}

{
    my $A = $M->zeros(1, 1);
    ok( $A->rows == 1 && $A->cols == 1, 'zeros: 1x1 works' );
    ok( near($A->get(0,0), 0.0),        'zeros: 1x1 element is 0' );
}

# ===========================================================================
# 3. from_2d
# ===========================================================================

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    ok( $A->rows == 2, 'from_2d: 2x2 rows' );
    ok( $A->cols == 2, 'from_2d: 2x2 cols' );
    ok( near($A->get(0,0), 1.0), 'from_2d: [0][0]' );
    ok( near($A->get(0,1), 2.0), 'from_2d: [0][1]' );
    ok( near($A->get(1,0), 3.0), 'from_2d: [1][0]' );
    ok( near($A->get(1,1), 4.0), 'from_2d: [1][1]' );
}

{
    my $A = $M->from_2d([[1,2,3],[4,5,6]]);
    ok( $A->rows == 2, 'from_2d: 2x3 rows' );
    ok( $A->cols == 3, 'from_2d: 2x3 cols' );
    ok( near($A->get(0,2), 3.0), 'from_2d: [0][2]' );
    ok( near($A->get(1,0), 4.0), 'from_2d: [1][0]' );
}

{
    # Deep copy: mutating source should not affect matrix.
    my $src = [[1,2],[3,4]];
    my $A = $M->from_2d($src);
    $src->[0][0] = 99;
    ok( near($A->get(0,0), 1.0), 'from_2d: deep-copied (mutation of source safe)' );
}

# ===========================================================================
# 4. from_1d
# ===========================================================================

{
    my $v = $M->from_1d([5,6,7,8]);
    ok( $v->rows == 1, 'from_1d: 1 row' );
    ok( $v->cols == 4, 'from_1d: 4 cols' );
    ok( near($v->get(0,0), 5.0), 'from_1d: [0]' );
    ok( near($v->get(0,3), 8.0), 'from_1d: [3]' );
}

{
    my $v = $M->from_1d([42]);
    ok( $v->rows == 1 && $v->cols == 1, 'from_1d: single element' );
    ok( near($v->get(0,0), 42.0),       'from_1d: value' );
}

# ===========================================================================
# 5. from_scalar
# ===========================================================================

{
    my $s = $M->from_scalar(3.14);
    ok( $s->rows == 1 && $s->cols == 1, 'from_scalar: 1x1' );
    ok( near($s->get(0,0), 3.14),        'from_scalar: value' );
}

{
    my $s = $M->from_scalar(0.0);
    ok( near($s->get(0,0), 0.0), 'from_scalar: zero' );
}

{
    my $s = $M->from_scalar(-7.5);
    ok( near($s->get(0,0), -7.5), 'from_scalar: negative' );
}

# ===========================================================================
# 6. get and set
# ===========================================================================

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    ok( near($A->get(0,0), 1.0), 'get: (0,0)' );
    ok( near($A->get(1,1), 4.0), 'get: (1,1)' );
}

{
    my $A = $M->zeros(2,2);
    my $B = $A->set(0, 1, 99.0);
    ok( near($B->get(0,1), 99.0), 'set: value updated in new matrix' );
    ok( near($B->get(0,0),  0.0), 'set: other element unchanged' );
    ok( near($A->get(0,1),  0.0), 'set: original not mutated' );
}

# ===========================================================================
# 7. add
# ===========================================================================

{
    # [[1,2],[3,4]] + [[5,6],[7,8]] = [[6,8],[10,12]]
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $B = $M->from_2d([[5,6],[7,8]]);
    my ($C, $err) = $A->add($B);
    ok( !defined $err,               'add: no error' );
    ok( near($C->get(0,0),  6.0),    'add: [0][0]' );
    ok( near($C->get(0,1),  8.0),    'add: [0][1]' );
    ok( near($C->get(1,0), 10.0),    'add: [1][0]' );
    ok( near($C->get(1,1), 12.0),    'add: [1][1]' );
}

{
    # A + zeros == A
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $Z = $M->zeros(2,2);
    my ($C, $err) = $A->add($Z);
    ok( !defined $err,               'add: A + zeros succeeds' );
    ok( mat_equal($C, $A),           'add: A + zeros == A' );
}

{
    # Commutative: A + B == B + A
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $B = $M->from_2d([[9,8],[7,6]]);
    my ($AB) = $A->add($B);
    my ($BA) = $B->add($A);
    ok( mat_equal($AB, $BA),         'add: commutative' );
}

{
    # Dimension mismatch → error.
    my $A = $M->zeros(2,3);
    my $B = $M->zeros(3,2);
    my (undef, $err) = $A->add($B);
    ok( defined $err,                'add: error for incompatible dimensions' );
}

# ===========================================================================
# 8. add_scalar
# ===========================================================================

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $B = $A->add_scalar(10);
    ok( near($B->get(0,0), 11.0), 'add_scalar: [0][0]' );
    ok( near($B->get(0,1), 12.0), 'add_scalar: [0][1]' );
    ok( near($B->get(1,0), 13.0), 'add_scalar: [1][0]' );
    ok( near($B->get(1,1), 14.0), 'add_scalar: [1][1]' );
}

{
    my $A = $M->from_2d([[5,6],[7,8]]);
    my $B = $A->add_scalar(0.0);
    ok( mat_equal($A, $B), 'add_scalar: adding 0 is identity' );
}

{
    # Original should not be mutated.
    my $A = $M->from_2d([[1,2]]);
    $A->add_scalar(100);
    ok( near($A->get(0,0), 1.0), 'add_scalar: does not mutate original' );
}

# ===========================================================================
# 9. subtract
# ===========================================================================

{
    my $A = $M->from_2d([[5,6],[7,8]]);
    my $B = $M->from_2d([[1,2],[3,4]]);
    my ($C, $err) = $A->subtract($B);
    ok( !defined $err,               'subtract: no error' );
    ok( near($C->get(0,0), 4.0),     'subtract: [0][0]' );
    ok( near($C->get(1,1), 4.0),     'subtract: [1][1]' );
}

{
    # A - A == zeros
    my $A = $M->from_2d([[9,8],[7,6]]);
    my ($C, $err) = $A->subtract($A);
    ok( !defined $err, 'subtract: A-A succeeds' );
    ok( mat_equal($C, $M->zeros(2,2)), 'subtract: A-A == zeros' );
}

{
    # A - zeros == A
    my $A = $M->from_2d([[3,1],[4,2]]);
    my ($C, $err) = $A->subtract($M->zeros(2,2));
    ok( mat_equal($C, $A), 'subtract: A - zeros == A' );
}

{
    my $A = $M->zeros(2,2);
    my $B = $M->zeros(2,3);
    my (undef, $err) = $A->subtract($B);
    ok( defined $err, 'subtract: error for incompatible dimensions' );
}

# ===========================================================================
# 10. scale
# ===========================================================================

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $B = $A->scale(3.0);
    ok( near($B->get(0,0),  3.0), 'scale: [0][0]' );
    ok( near($B->get(0,1),  6.0), 'scale: [0][1]' );
    ok( near($B->get(1,0),  9.0), 'scale: [1][0]' );
    ok( near($B->get(1,1), 12.0), 'scale: [1][1]' );
}

{
    my $A = $M->from_2d([[7,8],[9,10]]);
    ok( mat_equal($A->scale(1.0), $A), 'scale: by 1 is identity' );
}

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    ok( mat_equal($A->scale(0.0), $M->zeros(2,2)), 'scale: by 0 gives zeros' );
}

{
    my $A = $M->from_2d([[1,-2],[3,0]]);
    my $B = $A->scale(-1.0);
    ok( near($B->get(0,0), -1.0), 'scale: by -1 negates [0][0]' );
    ok( near($B->get(0,1),  2.0), 'scale: by -1 negates [0][1]' );
}

{
    # Does not mutate original.
    my $A = $M->from_2d([[5,5]]);
    $A->scale(100);
    ok( near($A->get(0,0), 5.0), 'scale: does not mutate original' );
}

# ===========================================================================
# 11. transpose
# ===========================================================================

{
    # [[1,2,3],[4,5,6]]^T = [[1,4],[2,5],[3,6]]
    my $A  = $M->from_2d([[1,2,3],[4,5,6]]);
    my $AT = $A->transpose;
    ok( $AT->rows == 3, 'transpose: rows' );
    ok( $AT->cols == 2, 'transpose: cols' );
    ok( near($AT->get(0,0), 1.0), 'transpose: [0][0]' );
    ok( near($AT->get(0,1), 4.0), 'transpose: [0][1]' );
    ok( near($AT->get(1,0), 2.0), 'transpose: [1][0]' );
    ok( near($AT->get(1,1), 5.0), 'transpose: [1][1]' );
    ok( near($AT->get(2,0), 3.0), 'transpose: [2][0]' );
    ok( near($AT->get(2,1), 6.0), 'transpose: [2][1]' );
}

{
    # (A^T)^T == A
    my $A = $M->from_2d([[1,2,3],[4,5,6]]);
    ok( mat_equal($A->transpose->transpose, $A), 'transpose: double transpose is identity' );
}

{
    # Square: off-diagonal elements swap, diagonal stays.
    my $A  = $M->from_2d([[1,2],[3,4]]);
    my $AT = $A->transpose;
    ok( near($AT->get(0,1), $A->get(1,0)), 'transpose: off-diagonal swapped [0][1]' );
    ok( near($AT->get(1,0), $A->get(0,1)), 'transpose: off-diagonal swapped [1][0]' );
    ok( near($AT->get(0,0), $A->get(0,0)), 'transpose: diagonal unchanged [0][0]' );
}

{
    # Row vector transpose → column vector.
    my $v  = $M->from_1d([1,2,3]);   # 1×3
    my $vT = $v->transpose;           # 3×1
    ok( $vT->rows == 3, 'transpose: row→col rows' );
    ok( $vT->cols == 1, 'transpose: row→col cols' );
    ok( near($vT->get(0,0), 1.0), 'transpose: row→col [0]' );
    ok( near($vT->get(1,0), 2.0), 'transpose: row→col [1]' );
    ok( near($vT->get(2,0), 3.0), 'transpose: row→col [2]' );
}

# ===========================================================================
# 12. dot (matrix multiplication)
# ===========================================================================

{
    # A = [[1,2],[3,4]], B = [[5,6],[7,8]]
    # C[0][0] = 1*5 + 2*7 = 19
    # C[0][1] = 1*6 + 2*8 = 22
    # C[1][0] = 3*5 + 4*7 = 43
    # C[1][1] = 3*6 + 4*8 = 50
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $B = $M->from_2d([[5,6],[7,8]]);
    my ($C, $err) = $A->dot($B);
    ok( !defined $err,               'dot: 2x2 no error' );
    ok( $C->rows == 2,               'dot: result rows' );
    ok( $C->cols == 2,               'dot: result cols' );
    ok( near($C->get(0,0), 19.0),    'dot: C[0][0] = 19' );
    ok( near($C->get(0,1), 22.0),    'dot: C[0][1] = 22' );
    ok( near($C->get(1,0), 43.0),    'dot: C[1][0] = 43' );
    ok( near($C->get(1,1), 50.0),    'dot: C[1][1] = 50' );
}

{
    # 2×3 · 3×2 → 2×2
    # A=[[1,2,3],[4,5,6]], B=[[7,8],[9,10],[11,12]]
    # C[0][0] = 1*7+2*9+3*11 = 7+18+33 = 58
    # C[0][1] = 1*8+2*10+3*12 = 8+20+36 = 64
    # C[1][0] = 4*7+5*9+6*11 = 28+45+66 = 139
    # C[1][1] = 4*8+5*10+6*12 = 32+50+72 = 154
    my $A = $M->from_2d([[1,2,3],[4,5,6]]);
    my $B = $M->from_2d([[7,8],[9,10],[11,12]]);
    my ($C, $err) = $A->dot($B);
    ok( !defined $err,                'dot: 2x3 * 3x2 no error' );
    ok( $C->rows == 2 && $C->cols == 2, 'dot: 2x3 * 3x2 shape' );
    ok( near($C->get(0,0),  58.0),    'dot: 2x3*3x2 C[0][0]' );
    ok( near($C->get(0,1),  64.0),    'dot: 2x3*3x2 C[0][1]' );
    ok( near($C->get(1,0), 139.0),    'dot: 2x3*3x2 C[1][0]' );
    ok( near($C->get(1,1), 154.0),    'dot: 2x3*3x2 C[1][1]' );
}

{
    # I · A = A  (identity matrix)
    my $I = $M->from_2d([[1,0],[0,1]]);
    my $A = $M->from_2d([[3,7],[2,5]]);
    my ($IA, $err) = $I->dot($A);
    ok( !defined $err, 'dot: I*A no error' );
    ok( mat_equal($IA, $A), 'dot: I*A == A' );
}

{
    # A · I = A
    my $I  = $M->from_2d([[1,0],[0,1]]);
    my $A  = $M->from_2d([[3,7],[2,5]]);
    my ($AI, $err) = $A->dot($I);
    ok( !defined $err, 'dot: A*I no error' );
    ok( mat_equal($AI, $A), 'dot: A*I == A' );
}

{
    # Associativity: (A·B)·C == A·(B·C)
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $B = $M->from_2d([[5,0],[0,1]]);
    my $C = $M->from_2d([[1,1],[1,1]]);
    my ($AB) = $A->dot($B);  my ($ABC1) = $AB->dot($C);
    my ($BC) = $B->dot($C);  my ($ABC2) = $A->dot($BC);
    ok( mat_equal($ABC1, $ABC2), 'dot: associative' );
}

{
    # 1×n · n×1 = 1×1 (dot product as matrix multiply)
    # [1,2,3] · [4,5,6]^T = 1*4 + 2*5 + 3*6 = 32
    my $row = $M->from_1d([1,2,3]);             # 1×3
    my $col = $M->from_1d([4,5,6])->transpose;  # 3×1
    my ($C, $err) = $row->dot($col);
    ok( !defined $err,            'dot: 1xn * nx1 no error' );
    ok( $C->rows==1 && $C->cols==1, 'dot: 1xn * nx1 shape' );
    ok( near($C->get(0,0), 32.0), 'dot: inner product = 32' );
}

{
    # Dimension mismatch → error.
    my $A = $M->zeros(2,3);
    my $B = $M->zeros(2,2);
    my (undef, $err) = $A->dot($B);
    ok( defined $err, 'dot: error for incompatible dimensions' );
}

# ===========================================================================
# 13. Combined / property tests
# ===========================================================================

{
    # (A+B)^T == A^T + B^T
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $B = $M->from_2d([[5,6],[7,8]]);
    my ($AB)  = $A->add($B);   my $lhs  = $AB->transpose;
    my ($ATA) = $A->transpose->add($B->transpose);
    ok( mat_equal($lhs, $ATA), 'transpose distributes over addition' );
}

{
    # scale(A+B, s) == scale(A,s) + scale(B,s)
    my $A   = $M->from_2d([[1,2],[3,4]]);
    my $B   = $M->from_2d([[5,6],[7,8]]);
    my $s   = 3.0;
    my ($AB) = $A->add($B);
    my $lhs  = $AB->scale($s);
    my ($rhs) = $A->scale($s)->add($B->scale($s));
    ok( mat_equal($lhs, $rhs), 'scale distributes over addition' );
}

{
    # A - B == A + scale(B, -1)
    my $A = $M->from_2d([[10,20],[30,40]]);
    my $B = $M->from_2d([[ 1, 2],[ 3, 4]]);
    my ($sub) = $A->subtract($B);
    my ($add) = $A->add($B->scale(-1.0));
    ok( mat_equal($sub, $add), 'subtract == add of negated' );
}

# ===========================================================================
# 14. ML03 Extension Tests — Reductions
# ===========================================================================

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    ok( near($A->sum, 10.0), 'sum: [[1,2],[3,4]] = 10' );
}

{
    ok( near($M->zeros(3,3)->sum, 0.0), 'sum: zeros = 0' );
}

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $sr = $A->sum_rows;
    ok( $sr->rows == 2 && $sr->cols == 1, 'sum_rows: shape' );
    ok( near($sr->get(0,0), 3.0), 'sum_rows: row 0' );
    ok( near($sr->get(1,0), 7.0), 'sum_rows: row 1' );
}

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $sc = $A->sum_cols;
    ok( $sc->rows == 1 && $sc->cols == 2, 'sum_cols: shape' );
    ok( near($sc->get(0,0), 4.0), 'sum_cols: col 0' );
    ok( near($sc->get(0,1), 6.0), 'sum_cols: col 1' );
}

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    ok( near($A->mean, 2.5), 'mean: [[1,2],[3,4]] = 2.5' );
}

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    ok( near($A->mat_min, 1.0), 'mat_min: 1' );
    ok( near($A->mat_max, 4.0), 'mat_max: 4' );
}

{
    my $A = $M->from_2d([[-5,2],[3,-1]]);
    ok( near($A->mat_min, -5.0), 'mat_min: -5 with negatives' );
    ok( near($A->mat_max,  3.0), 'mat_max: 3 with negatives' );
}

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    my ($ri, $rj) = $A->argmin;
    ok( $ri == 0 && $rj == 0, 'argmin: (0,0)' );
    my ($xi, $xj) = $A->argmax;
    ok( $xi == 1 && $xj == 1, 'argmax: (1,1)' );
}

{
    my $A = $M->from_2d([[3,1],[3,2]]);
    my ($r, $c) = $A->argmax;
    ok( $r == 0 && $c == 0, 'argmax: first occurrence on tie' );
}

# ===========================================================================
# 15. ML03 Extension Tests — Element-wise math
# ===========================================================================

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $B = $A->mat_map(sub { $_[0] * 2 });
    ok( near($B->get(0,0), 2.0), 'mat_map: doubles [0][0]' );
    ok( near($B->get(1,1), 8.0), 'mat_map: doubles [1][1]' );
}

{
    my $A = $M->from_2d([[4,9],[16,25]]);
    my $B = $A->mat_sqrt;
    ok( near($B->get(0,0), 2.0), 'mat_sqrt: sqrt(4)' );
    ok( near($B->get(0,1), 3.0), 'mat_sqrt: sqrt(9)' );
    ok( near($B->get(1,0), 4.0), 'mat_sqrt: sqrt(16)' );
    ok( near($B->get(1,1), 5.0), 'mat_sqrt: sqrt(25)' );
}

{
    my $A = $M->from_2d([[-1,2],[-3,4]]);
    my $B = $A->mat_abs;
    ok( near($B->get(0,0), 1.0), 'mat_abs: |-1|' );
    ok( near($B->get(1,0), 3.0), 'mat_abs: |-3|' );
}

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $B = $A->mat_pow(2.0);
    ok( near($B->get(0,0), 1.0), 'mat_pow: 1^2' );
    ok( near($B->get(0,1), 4.0), 'mat_pow: 2^2' );
    ok( near($B->get(1,0), 9.0), 'mat_pow: 3^2' );
    ok( near($B->get(1,1), 16.0), 'mat_pow: 4^2' );
}

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    ok( $A->close($A->mat_sqrt->mat_pow(2.0), 1e-9), 'sqrt then pow roundtrip is close' );
}

# ===========================================================================
# 16. ML03 Extension Tests — Shape operations
# ===========================================================================

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $F = $A->flatten;
    ok( $F->rows == 1 && $F->cols == 4, 'flatten: shape' );
    ok( near($F->get(0,0), 1.0), 'flatten: [0]' );
    ok( near($F->get(0,1), 2.0), 'flatten: [1]' );
    ok( near($F->get(0,2), 3.0), 'flatten: [2]' );
    ok( near($F->get(0,3), 4.0), 'flatten: [3]' );
}

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $R = $A->flatten->reshape(2,2);
    ok( $A->equals($R), 'flatten then reshape roundtrip' );
}

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    eval { $A->reshape(3,3) };
    ok( $@, 'reshape: error on size mismatch' );
}

{
    my $A = $M->from_2d([[1,2,3],[4,5,6]]);
    my $r = $A->mat_row(0);
    ok( $r->rows == 1 && $r->cols == 3, 'mat_row: shape' );
    ok( near($r->get(0,0), 1.0), 'mat_row: [0]' );
    ok( near($r->get(0,2), 3.0), 'mat_row: [2]' );
}

{
    my $A = $M->from_2d([[1,2,3],[4,5,6]]);
    my $c = $A->mat_col(1);
    ok( $c->rows == 2 && $c->cols == 1, 'mat_col: shape' );
    ok( near($c->get(0,0), 2.0), 'mat_col: [0]' );
    ok( near($c->get(1,0), 5.0), 'mat_col: [1]' );
}

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $S = $A->slice(0, 2, 0, 1);
    ok( $S->rows == 2 && $S->cols == 1, 'slice: shape' );
    ok( near($S->get(0,0), 1.0), 'slice: [0][0]' );
    ok( near($S->get(1,0), 3.0), 'slice: [1][0]' );
}

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $S = $A->slice(0, 2, 0, 2);
    ok( $A->equals($S), 'slice: full matrix equals original' );
}

# ===========================================================================
# 17. ML03 Extension Tests — Equality and comparison
# ===========================================================================

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $B = $M->from_2d([[1,2],[3,4]]);
    ok( $A->equals($B), 'equals: identical matrices' );
}

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $B = $M->from_2d([[1,2],[3,5]]);
    ok( !$A->equals($B), 'equals: different matrices' );
}

{
    my $A = $M->from_2d([[1,2]]);
    my $B = $M->from_2d([[1],[2]]);
    ok( !$A->equals($B), 'equals: different shapes' );
}

{
    my $A = $M->from_2d([[1.0, 2.0]]);
    my $B = $M->from_2d([[1.0 + 1e-10, 2.0 - 1e-10]]);
    ok( $A->close($B, 1e-9), 'close: within tolerance' );
}

{
    my $A = $M->from_2d([[1.0, 2.0]]);
    my $B = $M->from_2d([[1.1, 2.0]]);
    ok( !$A->close($B, 1e-9), 'close: outside tolerance' );
}

# ===========================================================================
# 18. ML03 Extension Tests — Factory methods
# ===========================================================================

{
    my $I = $M->identity(3);
    ok( $I->rows == 3 && $I->cols == 3, 'identity: shape' );
    for my $i (0..2) {
        for my $j (0..2) {
            my $expected = ($i == $j) ? 1.0 : 0.0;
            ok( near($I->get($i,$j), $expected), "identity: [$i][$j]" );
        }
    }
}

{
    my $I = $M->identity(3);
    my $A = $M->from_2d([[1,2],[3,4],[5,6]]);
    my ($IA) = $I->dot($A);
    ok( $IA->equals($A), 'identity(3) . M == M' );
}

{
    my $D = $M->from_diagonal([2, 3]);
    ok( $D->rows == 2 && $D->cols == 2, 'from_diagonal: shape' );
    ok( near($D->get(0,0), 2.0), 'from_diagonal: [0][0]' );
    ok( near($D->get(0,1), 0.0), 'from_diagonal: [0][1]' );
    ok( near($D->get(1,0), 0.0), 'from_diagonal: [1][0]' );
    ok( near($D->get(1,1), 3.0), 'from_diagonal: [1][1]' );
}

{
    ok( $M->from_diagonal([1,1,1])->equals($M->identity(3)), 'from_diagonal([1,1,1]) == identity(3)' );
}

# ===========================================================================
# 19. Parity test vectors
# ===========================================================================

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    ok( near($A->sum, 10.0), 'parity: sum = 10' );
    ok( near($A->mean, 2.5), 'parity: mean = 2.5' );
}

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $sr = $A->sum_rows;
    my $sc = $A->sum_cols;
    ok( near($sr->get(0,0), 3.0) && near($sr->get(1,0), 7.0), 'parity: sum_rows' );
    ok( near($sc->get(0,0), 4.0) && near($sc->get(0,1), 6.0), 'parity: sum_cols' );
}

{
    my $I = $M->identity(3);
    my $A = $M->from_2d([[1,2,3],[4,5,6],[7,8,9]]);
    my ($IA) = $I->dot($A);
    ok( $IA->equals($A), 'parity: identity dot' );
}

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    ok( $A->flatten->reshape($A->rows, $A->cols)->equals($A), 'parity: flatten/reshape roundtrip' );
}

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    ok( $A->close($A->mat_sqrt->mat_pow(2.0), 1e-9), 'parity: close after sqrt/pow' );
}

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    ok( near($A->get(0,0), 1.0), 'parity: get(0,0) = 1.0' );
}

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    my ($r, $c) = $A->argmax;
    ok( $r == 1 && $c == 1, 'parity: argmax = (1,1)' );
}

{
    my $A = $M->from_2d([[1,2],[3,4]]);
    my $S = $A->slice(0, 2, 0, 1);
    ok( $S->rows == 2 && $S->cols == 1, 'parity: slice shape' );
    ok( near($S->get(0,0), 1.0) && near($S->get(1,0), 3.0), 'parity: slice values' );
}

done_testing;
