package CodingAdventures::Matrix;

# ============================================================================
# CodingAdventures::Matrix — 2D matrix type with arithmetic operations
# ============================================================================
#
# This module implements a two-dimensional matrix in pure Perl using
# object-oriented (bless-based) style.  It is designed for clarity and
# educational value: every algorithm is explained at the mathematical level
# so that a reader new to linear algebra can follow along.
#
# ## Representation
#
# A Matrix object is a blessed hash reference with three fields:
#
#   $self->{rows}  — number of rows   (positive integer)
#   $self->{cols}  — number of columns (positive integer)
#   $self->{data}  — array-ref of row-array-refs
#                    $self->{data}[$i][$j] = element at row i, col j
#                    (zero-based indexing internally)
#
# All public methods use zero-based indices internally but the
# get/set interface also uses zero-based indices for consistency with
# Perl's array convention.
#
# ## Error Handling
#
# Methods that can fail (dimension mismatches, bad arguments) return
# a list ($result, $error_string) where $error_string is undef on success
# and a human-readable string on failure.  Methods that cannot fail
# return their result directly.
#
# ## Usage
#
#   use CodingAdventures::Matrix;
#
#   my $A = CodingAdventures::Matrix->zeros(2, 3);
#   my $B = CodingAdventures::Matrix->from_2d([[1,2],[3,4]]);
#   my ($C, $err) = $A->dot($B);
#   die $err if $err;
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# Constructor: new
# ============================================================================

=head2 new(%args)

Low-level constructor.  Prefer the factory methods C<zeros>, C<from_2d>,
C<from_1d>, and C<from_scalar> over calling this directly.

  my $m = CodingAdventures::Matrix->new(rows => 2, cols => 3, data => [...]);

C<data> must be an array-ref of row-array-refs with the same dimensions as
C<rows> and C<cols>.

=cut

sub new {
    my ( $class, %args ) = @_;
    return bless {
        rows => $args{rows},
        cols => $args{cols},
        data => $args{data},
    }, $class;
}

# ============================================================================
# Factory: zeros
# ============================================================================

=head2 zeros($rows, $cols)

Class method.  Creates an ($rows × $cols) matrix filled with 0.0.

The zero matrix is the additive identity: A + 0 = A.  It is also a
convenient starting point when accumulating a result element by element.

=cut

sub zeros {
    my ( $class, $rows, $cols ) = @_;
    my @data;
    for my $i ( 0 .. $rows - 1 ) {
        # Allocate each row as a flat anonymous array of zeros.
        # We use 0.0 (float) explicitly rather than 0 (integer) to keep
        # all elements consistently floating-point.
        $data[$i] = [ (0.0) x $cols ];
    }
    return $class->new( rows => $rows, cols => $cols, data => \@data );
}

# ============================================================================
# Factory: from_2d
# ============================================================================

=head2 from_2d($data)

Class method.  Creates a matrix from a reference to an array of row-arrays.

    my $A = CodingAdventures::Matrix->from_2d([[1,2,3],[4,5,6]]);  # 2×3

The number of rows is inferred from the outer array; the number of columns
from the first inner array.  The data is deep-copied so the caller's
original array cannot accidentally corrupt the matrix.

=cut

sub from_2d {
    my ( $class, $src ) = @_;
    my $rows = scalar @$src;
    my $cols = $rows > 0 ? scalar @{ $src->[0] } : 0;

    # Deep copy every element to decouple the matrix from the source table.
    my @data;
    for my $i ( 0 .. $rows - 1 ) {
        $data[$i] = [ @{ $src->[$i] } ];   # shallow copy of row is fine for scalars
    }
    return $class->new( rows => $rows, cols => $cols, data => \@data );
}

# ============================================================================
# Factory: from_1d
# ============================================================================

=head2 from_1d($data)

Class method.  Creates a 1×n row vector from a flat array reference.

    my $v = CodingAdventures::Matrix->from_1d([1, 2, 3]);  # 1×3

A row vector is a matrix with a single row.  Many operations in machine
learning (bias addition, output scaling) are naturally expressed as
row vectors.

=cut

sub from_1d {
    my ( $class, $src ) = @_;
    my $n = scalar @$src;
    # Wrap the flat array in an outer array-ref to match the 2D layout.
    return $class->new( rows => 1, cols => $n, data => [ [ @$src ] ] );
}

# ============================================================================
# Factory: from_scalar
# ============================================================================

=head2 from_scalar($val)

Class method.  Creates a 1×1 matrix containing a single value.

Useful when an algorithm expects a Matrix object but the operand is
logically a scalar.

=cut

sub from_scalar {
    my ( $class, $val ) = @_;
    return $class->new( rows => 1, cols => 1, data => [ [$val] ] );
}

# ============================================================================
# Accessors
# ============================================================================

=head2 rows()

Returns the number of rows.

=cut

sub rows { return $_[0]->{rows} }

=head2 cols()

Returns the number of columns.

=cut

sub cols { return $_[0]->{cols} }

=head2 data()

Returns the internal data structure (array-ref of row-array-refs).
Modifying the returned reference will mutate the matrix.

=cut

sub data { return $_[0]->{data} }

=head2 get($i, $j)

Returns the element at row $i, column $j (zero-based).
Raises an error if the indices are out of bounds.

=cut

sub get {
    my ( $self, $i, $j ) = @_;
    die sprintf( "index out of bounds: (%d, %d) for (%dx%d) matrix\n",
        $i, $j, $self->{rows}, $self->{cols} )
        if $i < 0 || $i >= $self->{rows} || $j < 0 || $j >= $self->{cols};
    return $self->{data}[$i][$j];
}

=head2 set($i, $j, $val)

Returns a B<new> matrix with the element at ($i, $j) replaced by $val.
The original matrix is not mutated.

=cut

sub set {
    my ( $self, $i, $j, $val ) = @_;
    die sprintf( "index out of bounds: (%d, %d) for (%dx%d) matrix\n",
        $i, $j, $self->{rows}, $self->{cols} )
        if $i < 0 || $i >= $self->{rows} || $j < 0 || $j >= $self->{cols};
    # Deep copy, then replace the target element.
    my @data;
    for my $r ( 0 .. $self->{rows} - 1 ) {
        $data[$r] = [ @{ $self->{data}[$r] } ];
    }
    $data[$i][$j] = $val;
    return ref($self)->new( rows => $self->{rows}, cols => $self->{cols}, data => \@data );
}

# ============================================================================
# Internal helper
# ============================================================================

# _check_same_dims validates that two matrices have the same shape.
# Returns undef on success, or an error string on failure.
sub _check_same_dims {
    my ( $A, $B ) = @_;
    return undef if $A->{rows} == $B->{rows} && $A->{cols} == $B->{cols};
    return sprintf(
        'dimension mismatch: (%dx%d) vs (%dx%d)',
        $A->{rows}, $A->{cols}, $B->{rows}, $B->{cols}
    );
}

# ============================================================================
# add
# ============================================================================

=head2 add($B)

Instance method.  Returns a new matrix = self + B, element-wise.

    (A + B)[i][j] = A[i][j] + B[i][j]

Both matrices must have the same dimensions.

Returns C<($matrix, undef)> on success or C<(undef, $error)> on failure.

=cut

sub add {
    my ( $self, $B ) = @_;
    my $err = _check_same_dims( $self, $B );
    return ( undef, $err ) if $err;

    my $result = ref($self)->zeros( $self->{rows}, $self->{cols} );
    for my $i ( 0 .. $self->{rows} - 1 ) {
        for my $j ( 0 .. $self->{cols} - 1 ) {
            $result->{data}[$i][$j] =
                $self->{data}[$i][$j] + $B->{data}[$i][$j];
        }
    }
    return ( $result, undef );
}

# ============================================================================
# add_scalar
# ============================================================================

=head2 add_scalar($s)

Instance method.  Returns a new matrix with the scalar $s added to every
element.

    (A + s)[i][j] = A[i][j] + s

=cut

sub add_scalar {
    my ( $self, $s ) = @_;
    my $result = ref($self)->zeros( $self->{rows}, $self->{cols} );
    for my $i ( 0 .. $self->{rows} - 1 ) {
        for my $j ( 0 .. $self->{cols} - 1 ) {
            $result->{data}[$i][$j] = $self->{data}[$i][$j] + $s;
        }
    }
    return $result;
}

# ============================================================================
# subtract
# ============================================================================

=head2 subtract($B)

Instance method.  Returns a new matrix = self - B, element-wise.

Both matrices must have the same dimensions.

Returns C<($matrix, undef)> on success or C<(undef, $error)> on failure.

=cut

sub subtract {
    my ( $self, $B ) = @_;
    my $err = _check_same_dims( $self, $B );
    return ( undef, $err ) if $err;

    my $result = ref($self)->zeros( $self->{rows}, $self->{cols} );
    for my $i ( 0 .. $self->{rows} - 1 ) {
        for my $j ( 0 .. $self->{cols} - 1 ) {
            $result->{data}[$i][$j] =
                $self->{data}[$i][$j] - $B->{data}[$i][$j];
        }
    }
    return ( $result, undef );
}

# ============================================================================
# scale
# ============================================================================

=head2 scale($s)

Instance method.  Returns a new matrix with every element multiplied by $s.

    (s * A)[i][j] = s * A[i][j]

=cut

sub scale {
    my ( $self, $s ) = @_;
    my $result = ref($self)->zeros( $self->{rows}, $self->{cols} );
    for my $i ( 0 .. $self->{rows} - 1 ) {
        for my $j ( 0 .. $self->{cols} - 1 ) {
            $result->{data}[$i][$j] = $self->{data}[$i][$j] * $s;
        }
    }
    return $result;
}

# ============================================================================
# transpose
# ============================================================================

=head2 transpose()

Instance method.  Returns the transpose of self.

The transpose of an m×n matrix is an n×m matrix where rows and columns
are swapped:

    A_transpose[i][j] = A[j][i]

Properties:
  - (A^T)^T = A
  - (A + B)^T = A^T + B^T
  - (AB)^T = B^T * A^T   (note the reversal)

=cut

sub transpose {
    my ($self) = @_;
    # Result dimensions are swapped: m×n becomes n×m.
    my $result = ref($self)->zeros( $self->{cols}, $self->{rows} );
    for my $i ( 0 .. $self->{rows} - 1 ) {
        for my $j ( 0 .. $self->{cols} - 1 ) {
            # Row i, column j in A becomes row j, column i in A^T.
            $result->{data}[$j][$i] = $self->{data}[$i][$j];
        }
    }
    return $result;
}

# ============================================================================
# dot
# ============================================================================

=head2 dot($B)

Instance method.  Returns the matrix product self · B.

The matrix product C = A · B is defined when A is m×k and B is k×n
(A's column count must equal B's row count).  The result is m×n:

    C[i][j] = sum_{l=0}^{k-1}  A[i][l] * B[l][j]

Think of each C[i][j] as the dot product of row i of A with column j of B.

Properties:
  - NOT commutative in general: A·B ≠ B·A
  - Associative: (A·B)·C = A·(B·C)
  - Distributive: A·(B+C) = A·B + A·C

Returns C<($matrix, undef)> on success or C<(undef, $error)> on failure.

=cut

sub dot {
    my ( $self, $B ) = @_;

    # The inner dimensions must agree: A.cols must equal B.rows.
    unless ( $self->{cols} == $B->{rows} ) {
        return ( undef, sprintf(
            'dot: inner dimensions must match; got (%dx%d) * (%dx%d)',
            $self->{rows}, $self->{cols}, $B->{rows}, $B->{cols}
        ) );
    }

    my $m      = $self->{rows};
    my $k      = $self->{cols};   # = B.rows (shared inner dimension)
    my $n      = $B->{cols};
    my $result = ref($self)->zeros( $m, $n );

    for my $i ( 0 .. $m - 1 ) {
        for my $j ( 0 .. $n - 1 ) {
            # Compute the inner product of row i of self with column j of B.
            my $sum = 0.0;
            for my $l ( 0 .. $k - 1 ) {
                $sum += $self->{data}[$i][$l] * $B->{data}[$l][$j];
            }
            $result->{data}[$i][$j] = $sum;
        }
    }

    return ( $result, undef );
}

# ============================================================================
# Reductions
# ============================================================================

=head2 sum()

Returns the sum of all elements.

=cut

sub sum {
    my ($self) = @_;
    my $total = 0.0;
    for my $i ( 0 .. $self->{rows} - 1 ) {
        for my $j ( 0 .. $self->{cols} - 1 ) {
            $total += $self->{data}[$i][$j];
        }
    }
    return $total;
}

=head2 sum_rows()

Returns an m x 1 column vector where each element is the sum of that row.

=cut

sub sum_rows {
    my ($self) = @_;
    my @data;
    for my $i ( 0 .. $self->{rows} - 1 ) {
        my $s = 0.0;
        for my $j ( 0 .. $self->{cols} - 1 ) {
            $s += $self->{data}[$i][$j];
        }
        $data[$i] = [$s];
    }
    return ref($self)->new( rows => $self->{rows}, cols => 1, data => \@data );
}

=head2 sum_cols()

Returns a 1 x n row vector where each element is the sum of that column.

=cut

sub sum_cols {
    my ($self) = @_;
    my @row;
    for my $j ( 0 .. $self->{cols} - 1 ) {
        my $s = 0.0;
        for my $i ( 0 .. $self->{rows} - 1 ) {
            $s += $self->{data}[$i][$j];
        }
        $row[$j] = $s;
    }
    return ref($self)->new( rows => 1, cols => $self->{cols}, data => [ \@row ] );
}

=head2 mean()

Returns the arithmetic mean of all elements.

=cut

sub mean {
    my ($self) = @_;
    return $self->sum() / ( $self->{rows} * $self->{cols} );
}

=head2 mat_min()

Returns the minimum element value.  Named C<mat_min> to avoid shadowing
Perl's built-in C<CORE::min>.

=cut

sub mat_min {
    my ($self) = @_;
    my $best = $self->{data}[0][0];
    for my $i ( 0 .. $self->{rows} - 1 ) {
        for my $j ( 0 .. $self->{cols} - 1 ) {
            $best = $self->{data}[$i][$j] if $self->{data}[$i][$j] < $best;
        }
    }
    return $best;
}

=head2 mat_max()

Returns the maximum element value.  Named C<mat_max> to avoid shadowing
Perl's built-in C<CORE::max>.

=cut

sub mat_max {
    my ($self) = @_;
    my $best = $self->{data}[0][0];
    for my $i ( 0 .. $self->{rows} - 1 ) {
        for my $j ( 0 .. $self->{cols} - 1 ) {
            $best = $self->{data}[$i][$j] if $self->{data}[$i][$j] > $best;
        }
    }
    return $best;
}

=head2 argmin()

Returns the (row, col) of the minimum element (zero-based, first occurrence).

=cut

sub argmin {
    my ($self) = @_;
    my $best = $self->{data}[0][0];
    my ( $bi, $bj ) = ( 0, 0 );
    for my $i ( 0 .. $self->{rows} - 1 ) {
        for my $j ( 0 .. $self->{cols} - 1 ) {
            if ( $self->{data}[$i][$j] < $best ) {
                $best = $self->{data}[$i][$j];
                ( $bi, $bj ) = ( $i, $j );
            }
        }
    }
    return ( $bi, $bj );
}

=head2 argmax()

Returns the (row, col) of the maximum element (zero-based, first occurrence).

=cut

sub argmax {
    my ($self) = @_;
    my $best = $self->{data}[0][0];
    my ( $bi, $bj ) = ( 0, 0 );
    for my $i ( 0 .. $self->{rows} - 1 ) {
        for my $j ( 0 .. $self->{cols} - 1 ) {
            if ( $self->{data}[$i][$j] > $best ) {
                $best = $self->{data}[$i][$j];
                ( $bi, $bj ) = ( $i, $j );
            }
        }
    }
    return ( $bi, $bj );
}

# ============================================================================
# Element-wise math
# ============================================================================

=head2 mat_map($fn)

Applies a code reference to every element, returning a new matrix.

=cut

sub mat_map {
    my ( $self, $fn ) = @_;
    my @data;
    for my $i ( 0 .. $self->{rows} - 1 ) {
        $data[$i] = [];
        for my $j ( 0 .. $self->{cols} - 1 ) {
            $data[$i][$j] = $fn->( $self->{data}[$i][$j] );
        }
    }
    return ref($self)->new( rows => $self->{rows}, cols => $self->{cols}, data => \@data );
}

=head2 mat_sqrt()

Element-wise square root.  Named C<mat_sqrt> to avoid shadowing
Perl's built-in C<CORE::sqrt>.

=cut

sub mat_sqrt {
    my ($self) = @_;
    return $self->mat_map( sub { CORE::sqrt( $_[0] ) } );
}

=head2 mat_abs()

Element-wise absolute value.  Named C<mat_abs> to avoid shadowing
Perl's built-in C<CORE::abs>.

=cut

sub mat_abs {
    my ($self) = @_;
    return $self->mat_map( sub { CORE::abs( $_[0] ) } );
}

=head2 mat_pow($exp)

Element-wise exponentiation.

=cut

sub mat_pow {
    my ( $self, $exp ) = @_;
    return $self->mat_map( sub { $_[0] ** $exp } );
}

# ============================================================================
# Shape operations
# ============================================================================

=head2 flatten()

Returns a 1 x n row vector with elements in row-major order.

=cut

sub flatten {
    my ($self) = @_;
    my @flat;
    for my $i ( 0 .. $self->{rows} - 1 ) {
        for my $j ( 0 .. $self->{cols} - 1 ) {
            push @flat, $self->{data}[$i][$j];
        }
    }
    return ref($self)->new( rows => 1, cols => scalar @flat, data => [ \@flat ] );
}

=head2 reshape($new_rows, $new_cols)

Reshapes the matrix.  Total elements must be preserved.

=cut

sub reshape {
    my ( $self, $nr, $nc ) = @_;
    my $total = $self->{rows} * $self->{cols};
    die sprintf(
        "reshape: cannot reshape (%dx%d) = %d elements into (%dx%d) = %d elements\n",
        $self->{rows}, $self->{cols}, $total, $nr, $nc, $nr * $nc
    ) unless $nr * $nc == $total;

    # Flatten, then refill.
    my @flat;
    for my $i ( 0 .. $self->{rows} - 1 ) {
        push @flat, @{ $self->{data}[$i] };
    }
    my @data;
    my $idx = 0;
    for my $i ( 0 .. $nr - 1 ) {
        $data[$i] = [];
        for my $j ( 0 .. $nc - 1 ) {
            $data[$i][$j] = $flat[$idx++];
        }
    }
    return ref($self)->new( rows => $nr, cols => $nc, data => \@data );
}

=head2 mat_row($i)

Returns row $i as a 1 x cols matrix (zero-based).

=cut

sub mat_row {
    my ( $self, $i ) = @_;
    die sprintf( "row: index %d out of bounds for %d rows\n", $i, $self->{rows} )
        if $i < 0 || $i >= $self->{rows};
    return ref($self)->new(
        rows => 1,
        cols => $self->{cols},
        data => [ [ @{ $self->{data}[$i] } ] ],
    );
}

=head2 mat_col($j)

Returns column $j as a rows x 1 matrix (zero-based).

=cut

sub mat_col {
    my ( $self, $j ) = @_;
    die sprintf( "col: index %d out of bounds for %d cols\n", $j, $self->{cols} )
        if $j < 0 || $j >= $self->{cols};
    my @data;
    for my $i ( 0 .. $self->{rows} - 1 ) {
        $data[$i] = [ $self->{data}[$i][$j] ];
    }
    return ref($self)->new( rows => $self->{rows}, cols => 1, data => \@data );
}

=head2 slice($r0, $r1, $c0, $c1)

Extracts a sub-matrix for rows [$r0..$r1) and columns [$c0..$c1)
(zero-based, half-open ranges).

=cut

sub slice {
    my ( $self, $r0, $r1, $c0, $c1 ) = @_;
    die sprintf(
        "slice: bounds (%d:%d, %d:%d) out of range for (%dx%d) matrix\n",
        $r0, $r1, $c0, $c1, $self->{rows}, $self->{cols}
    ) if $r0 < 0 || $r1 > $self->{rows} || $c0 < 0 || $c1 > $self->{cols};

    my $nr = $r1 - $r0;
    my $nc = $c1 - $c0;
    my @data;
    for my $i ( 0 .. $nr - 1 ) {
        $data[$i] = [];
        for my $j ( 0 .. $nc - 1 ) {
            $data[$i][$j] = $self->{data}[ $r0 + $i ][ $c0 + $j ];
        }
    }
    return ref($self)->new( rows => $nr, cols => $nc, data => \@data );
}

# ============================================================================
# Equality and comparison
# ============================================================================

=head2 equals($B)

Returns true (1) if self and B have the same shape and identical elements.

=cut

sub equals {
    my ( $self, $B ) = @_;
    return 0 unless $self->{rows} == $B->{rows} && $self->{cols} == $B->{cols};
    for my $i ( 0 .. $self->{rows} - 1 ) {
        for my $j ( 0 .. $self->{cols} - 1 ) {
            return 0 if $self->{data}[$i][$j] != $B->{data}[$i][$j];
        }
    }
    return 1;
}

=head2 close($B, $tol)

Returns true (1) if self and B have the same shape and all elements are
within $tol of each other.  Default $tol = 1e-9.

=cut

sub close {
    my ( $self, $B, $tol ) = @_;
    $tol //= 1e-9;
    return 0 unless $self->{rows} == $B->{rows} && $self->{cols} == $B->{cols};
    for my $i ( 0 .. $self->{rows} - 1 ) {
        for my $j ( 0 .. $self->{cols} - 1 ) {
            return 0 if CORE::abs( $self->{data}[$i][$j] - $B->{data}[$i][$j] ) > $tol;
        }
    }
    return 1;
}

# ============================================================================
# Factory methods
# ============================================================================

=head2 identity($n)

Class method.  Returns an n x n identity matrix.

=cut

sub identity {
    my ( $class, $n ) = @_;
    my @data;
    for my $i ( 0 .. $n - 1 ) {
        $data[$i] = [ (0.0) x $n ];
        $data[$i][$i] = 1.0;
    }
    return $class->new( rows => $n, cols => $n, data => \@data );
}

=head2 from_diagonal($values)

Class method.  Returns a square diagonal matrix from an array-ref of values.

=cut

sub from_diagonal {
    my ( $class, $values ) = @_;
    my $n = scalar @$values;
    my @data;
    for my $i ( 0 .. $n - 1 ) {
        $data[$i] = [ (0.0) x $n ];
        $data[$i][$i] = $values->[$i];
    }
    return $class->new( rows => $n, cols => $n, data => \@data );
}

1;

__END__

=head1 NAME

CodingAdventures::Matrix — 2D matrix type with arithmetic and linear-algebra operations

=head1 VERSION

0.01

=head1 SYNOPSIS

  use CodingAdventures::Matrix;

  my $A = CodingAdventures::Matrix->zeros(2, 3);
  my $B = CodingAdventures::Matrix->from_2d([[1,2],[3,4]]);
  my ($C, $err) = $B->dot($B);
  die $err if $err;
  print $C->get(0, 0), "\n";  # 7

=head1 DESCRIPTION

A 2D matrix object backed by an array-of-arrays.  Supports zeros, from_2d,
from_1d, from_scalar constructors; get/set element access; element-wise
add, add_scalar, subtract, scale; transpose; and dot (matrix multiplication).

=head1 LICENSE

MIT

=cut
