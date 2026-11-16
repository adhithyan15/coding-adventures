use strict;
use warnings;
use Test2::V0;
use Config;
use FindBin qw($Bin);
use File::Spec;
use JSON::PP ();
use Digest::SHA qw(sha256_hex);

use CodingAdventures::Trig qw(
    sin_approx cos_approx tan_approx
    sqrt_approx atan_approx atan2_approx
    sin_deg cos_deg tan_deg
    degrees_to_radians radians_to_degrees
);

# ============================================================================
# Bit-exact conformance of CodingAdventures::Trig against TRIG-REF-1
# ============================================================================
#
# code/specs/fixtures/trig-reference-v1 holds every vector as one row
#
#     op <TAB> x <TAB> y <TAB> cr <TAB> flags
#
# where x and y are IEEE binary64 BIT PATTERNS (16 lowercase hex digits), never
# decimals (PHY00a 6.2).  This test
#
#   (a) decodes x by a bit cast,
#   (b) calls the PUBLIC function,
#   (c) encodes the result back to bits and compares the BITS, as hex strings
#       or integers -- never a printed number.
#
# Why never a printed number: Perl prints an NV with 15 significant digits, so
# 0.1 + 0.2 prints as 0.3 and two different doubles can print identically.
# Test2's `is` on numbers goes through the same stringification and must not
# be used for float bits.
#
# The one relaxation is NaN, which matches ANY NaN (VER04p "same": payload and
# sign of a NaN are not portable).
#
# Bit casts in Perl (pack/unpack, a pure reinterpretation of eight bytes):
#
#     unpack("d>", pack("H16", $hex))    16 hex digits -> NV
#     unpack("H16", pack("d>", $nv))     NV -> 16 hex digits

die "these tests need a 64-bit IV (ivsize=$Config{ivsize})\n" unless $Config{ivsize} == 8;
die "these tests need a 64-bit NV (nvsize=$Config{nvsize})\n" unless $Config{nvsize} == 8;

sub hex_to_nv { return unpack( "d>", pack( "H16", $_[0] ) ); }
sub nv_to_hex { return unpack( "H16", pack( "d>", $_[0] ) ); }
sub is_nan    { return $_[0] != $_[0]; }

my $FIXTURE = File::Spec->catdir( $Bin, qw(.. .. .. .. specs fixtures trig-reference-v1) );
my @GROUPS  = qw(edge sweep-binade sweep-cw sweep-prng);
my $QNAN    = '7ff8000000000000';

# Accuracy limits in representable steps (ulps) against the `cr` column.
# PHY00a 3.8 makes NO accuracy claim for tan, so it is named here as a
# no-claim op rather than silently skipped.
my %LIMITS   = ( sin => 3, cos => 3, radians => 1, degrees => 1 );
my %NO_CLAIM = ( tan => 1 );

# op name in the corpus -> the package's public function.
my %FN = (
    sin     => \&sin_approx,
    cos     => \&cos_approx,
    tan     => \&tan_approx,
    radians => \&degrees_to_radians,
    degrees => \&radians_to_degrees,
    sqrt    => \&sqrt_approx,
    atan    => \&atan_approx,
    atan2   => \&atan2_approx,
);

sub slurp {
    my ($path) = @_;
    open my $fh, '<:raw', $path or die "cannot open $path: $!\n";
    local $/;
    my $data = <$fh>;
    close $fh;
    return $data;
}

sub load_rows {
    my @rows;
    for my $group (@GROUPS) {
        my $text = slurp( File::Spec->catfile( $FIXTURE, "$group.tsv" ) );
        die "$group.tsv does not end in a newline\n" unless $text =~ /\n\z/;
        for my $line ( split /\n/, $text ) {
            my ( $op, $x, $y, $cr, $flags ) = split /\t/, $line, -1;
            push @rows, { group => $group, op => $op, x => $x, y => $y, cr => $cr, flags => $flags };
        }
    }
    return \@rows;
}

my $ROWS = load_rows();

# Call the public function for a row.  Returns the NV result, or dies with
# whatever the function dies with.
sub call_op {
    my ( $op, $x ) = @_;
    my $fn = $FN{$op} or die "unknown op $op\n";
    if ( $op eq 'atan2' ) {
        my ( $ys, $xs ) = split /,/, $x;
        return $fn->( hex_to_nv($ys), hex_to_nv($xs) );
    }
    return $fn->( hex_to_nv($x) );
}

# ---------------------------------------------------------------------------
# The fixture on disk is the one the manifest describes
# ---------------------------------------------------------------------------

my $MANIFEST = JSON::PP->new->decode( slurp( File::Spec->catfile( $FIXTURE, 'manifest.json' ) ) );

subtest 'manifest hashes and row counts' => sub {
    is( $MANIFEST->{algorithm}, 'TRIG-REF-1', 'algorithm id' );
    is( [ map { $_->{name} } @{ $MANIFEST->{groups} } ], [@GROUPS], 'groups in order' );
    for my $g ( @{ $MANIFEST->{groups} } ) {
        my $data = slurp( File::Spec->catfile( $FIXTURE, $g->{file} ) );
        is( sha256_hex($data), $g->{sha256}, "$g->{file} sha256" );
        my $lines = () = $data =~ /\n/g;
        is( $lines, $g->{rows}, "$g->{file} row count" );
    }
};

subtest 'row syntax' => sub {
    my $bad = 0;
    my @examples;
    for my $row (@$ROWS) {
        my @problems;
        push @problems, "unknown op $row->{op}" unless exists $FN{ $row->{op} };
        my @fields = $row->{op} eq 'atan2' ? split( /,/, $row->{x} ) : ( $row->{x} );
        push @problems, 'wrong input arity' unless @fields == ( $row->{op} eq 'atan2' ? 2 : 1 );
        push @fields, $row->{y}  unless $row->{y} eq 'error';
        push @fields, $row->{cr} unless $row->{cr} eq '-';
        for my $f (@fields) { push @problems, "bad hex field '$f'" unless $f =~ /\A[0-9a-f]{16}\z/; }
        if (@problems) { $bad++; push @examples, "@problems" if @examples < 5; }
    }
    is( $bad, 0, 'every row is well formed' ) or diag("@examples");
};

# ---------------------------------------------------------------------------
# Constants: every embedded constant, by bits (PHY00a 6.5 (e))
# ---------------------------------------------------------------------------

subtest 'constants match the manifest bit for bit' => sub {
    my $c = $MANIFEST->{constants};
    my %mine = (
        PI          => $CodingAdventures::Trig::PI,
        TWO_PI      => $CodingAdventures::Trig::TWO_PI,
        HALF_PI     => $CodingAdventures::Trig::HALF_PI,
        INV_PIO2    => $CodingAdventures::Trig::INV_PIO2,
        MAGIC       => $CodingAdventures::Trig::MAGIC,
        LIMIT       => $CodingAdventures::Trig::LIMIT,
        TINY        => $CodingAdventures::Trig::TINY,
        RAD_PER_DEG => $CodingAdventures::Trig::RAD_PER_DEG,
        DEG_PER_RAD => $CodingAdventures::Trig::DEG_PER_RAD,
        TAN_POLE    => $CodingAdventures::Trig::TAN_POLE,
        TAN_BIG     => $CodingAdventures::Trig::TAN_BIG,
        ATAN_TERM   => $CodingAdventures::Trig::ATAN_TERM,
        SQRT_REL    => $CodingAdventures::Trig::SQRT_REL,
        SQRT_TINY   => $CodingAdventures::Trig::SQRT_TINY,
    );
    $mine{"PIO2_$_"} = $CodingAdventures::Trig::PIO2[ $_ - 1 ] for 1 .. 5;
    $mine{"S$_"}     = $CodingAdventures::Trig::S[$_]          for 1 .. 9;
    $mine{"C$_"}     = $CodingAdventures::Trig::C[$_]          for 2 .. 9;
    for my $name ( sort keys %mine ) {
        ok( defined $mine{$name}, "$name is defined" ) or next;
        is( nv_to_hex( $mine{$name} ), $c->{$name}, "$name bits" );
    }
    # Nothing in the manifest's constant table is unaccounted for.
    is( [ sort keys %mine ], [ sort keys %$c ], 'the module embeds exactly the manifest constants' );
};

subtest 'degree constants equal the divisions' => sub {
    is( nv_to_hex( $CodingAdventures::Trig::PI / 180.0 ), nv_to_hex($CodingAdventures::Trig::RAD_PER_DEG), 'PI/180' );
    is( nv_to_hex( 180.0 / $CodingAdventures::Trig::PI ), nv_to_hex($CodingAdventures::Trig::DEG_PER_RAD), '180/PI' );
};

# ---------------------------------------------------------------------------
# Every vector, bit for bit
# ---------------------------------------------------------------------------

my %FAILED_BY_OP;    # op -> failing rows over all groups (printed in the summary)
my %CHECKED_BY_OP;

for my $group (@GROUPS) {
    subtest "every vector matches bit for bit: $group" => sub {
        my ( %failures, @examples );
        my $checked = 0;
        for my $row (@$ROWS) {
            next unless $row->{group} eq $group;
            $checked++;
            my $op = $row->{op};
            $CHECKED_BY_OP{$op}++;
            my $problem;
            if ( $row->{y} eq 'error' ) {
                my $ok = eval { call_op( $op, $row->{x} ); 1 };
                $problem = 'no error raised' if $ok;
            }
            else {
                my $got = eval { call_op( $op, $row->{x} ) };
                if ( !defined $got ) {
                    $problem = "died: " . ( $@ // 'undef result' );
                    $problem =~ s/\s+\z//;
                }
                elsif ( $row->{y} eq $QNAN ) {
                    $problem = 'expected NaN, got ' . nv_to_hex($got) unless is_nan($got);
                }
                elsif ( nv_to_hex($got) ne $row->{y} ) {
                    $problem = "expected $row->{y}, got " . nv_to_hex($got);
                }
            }
            next unless $problem;
            $failures{$op}++;
            $FAILED_BY_OP{$op}++;
            push @examples, "$op($row->{x}): $problem" if @examples < 5;
        }
        cmp_ok( $checked, '>', 0, "$group has rows" );
        is( \%failures, {}, "no failing rows" ) or diag( join "\n", @examples );
    };
}

# ---------------------------------------------------------------------------
# Accuracy against the correctly rounded column
# ---------------------------------------------------------------------------

# Map a bit pattern onto a signed integer line on which adjacent doubles
# differ by exactly one, so |ordered(a) - ordered(b)| is the distance in steps.
sub ordered {
    my ($hex) = @_;
    my $bits  = unpack( 'Q>', pack( 'H16', $hex ) );
    no warnings 'portable';    # the mask is a 64-bit literal; ivsize == 8 was checked above
    my $mag   = $bits & 0x7FFFFFFFFFFFFFFF;
    return ( $bits >> 63 ) ? -$mag : $mag;
}

subtest 'accuracy against the correctly rounded column' => sub {
    my %worst = map { $_ => 0 } keys %LIMITS;
    my @unlimited;
    for my $row (@$ROWS) {
        my $op = $row->{op};
        next if $row->{cr} eq '-' || $NO_CLAIM{$op};

        # Fail closed: an op with a cr value but no limit is an error, so a
        # future op cannot be skipped silently.
        if ( !exists $LIMITS{$op} ) { push @unlimited, $op; next; }

        my $got = eval { call_op( $op, $row->{x} ) };
        next unless defined $got;    # reported by the bit-exact test
        my $steps = abs( ordered( nv_to_hex($got) ) - ordered( $row->{cr} ) );
        $worst{$op} = $steps if $steps > $worst{$op};
    }
    my %seen = map { $_ => 1 } @unlimited;
    is( [ sort keys %seen ], [], 'every op with a cr value has an accuracy limit' );
    for my $op ( sort keys %LIMITS ) {
        cmp_ok( $worst{$op}, '<=', $LIMITS{$op}, "$op worst=$worst{$op} steps, limit $LIMITS{$op}" );
    }
};

subtest 'tan is a named no-claim op, and unknown ops are rejected' => sub {
    ok( $NO_CLAIM{tan}, 'tan is explicitly no-claim' );
    ok( !exists $LIMITS{tan}, 'tan has no accuracy limit' );
    my %unknown = map { $_->{op} => 1 } grep { !exists $FN{ $_->{op} } } @$ROWS;
    is( [ sort keys %unknown ], [], 'the corpus holds no unknown op' );
    like( dies { call_op( 'no_such_op', '0000000000000000' ) }, qr/unknown op/, 'an unknown op fails' );
};

# ---------------------------------------------------------------------------
# The behaviours PHY00a 3.3 names, asserted directly
# ---------------------------------------------------------------------------

subtest 'sin of negative zero is negative zero' => sub {
    my $nz = hex_to_nv('8000000000000000');
    is( nv_to_hex( sin_approx($nz) ),             '8000000000000000', 'sin(-0 from bits)' );
    is( nv_to_hex( sin_approx(-0.0) ),            '8000000000000000', 'sin(-0.0 literal)' );
    is( nv_to_hex( sin_approx(0.0) ),             '0000000000000000', 'sin(+0.0)' );
    is( nv_to_hex( CodingAdventures::Trig->sin_approx($nz) ), '8000000000000000', 'as a class method' );

    # An NV that has ALSO been used as an integer (Perl caches an IV next to the
    # NV once a numeric test has looked at it) must keep its sign.
    my $used = hex_to_nv('8000000000000000');
    my $is_zero = ( $used == 0 ) ? 1 : 0;
    $is_zero += int($used);
    is( nv_to_hex( sin_approx($used) ), '8000000000000000', 'sin(-0) after the value was used as an integer' );

    # Perl has no negative-zero INTEGER: an IV 0 (which is what the literals 0
    # and -0 are) is simply +0, and so is its sine.
    is( nv_to_hex( sin_approx(0) ),  '0000000000000000', 'sin(IV 0) is +0' );
    is( nv_to_hex( sin_approx(-0) ), '0000000000000000', 'sin(IV -0) is +0 (there is no integer -0)' );

    is( nv_to_hex( tan_approx($nz) ), '8000000000000000', 'tan(-0) = -0' );
    is( nv_to_hex( degrees_to_radians($nz) ), '8000000000000000', 'radians(-0) = -0' );
    is( nv_to_hex( radians_to_degrees($nz) ), '8000000000000000', 'degrees(-0) = -0' );
    is( nv_to_hex( atan_approx($nz) ),        '8000000000000000', 'atan(-0) = -0' );
};

subtest 'cos of zero is one' => sub {
    is( nv_to_hex( cos_approx( hex_to_nv('0000000000000000') ) ), '3ff0000000000000', 'cos(+0)' );
    is( nv_to_hex( cos_approx( hex_to_nv('8000000000000000') ) ), '3ff0000000000000', 'cos(-0)' );
    is( nv_to_hex( cos_approx(0) ), '3ff0000000000000', 'cos(IV 0)' );
};

subtest 'tiny arguments are returned unchanged' => sub {
    my $tiny = hex_to_nv('3e3fffffffffffff');
    is( nv_to_hex( sin_approx($tiny) ), '3e3fffffffffffff', 'sin(just below 2^-27) = x' );
    is( nv_to_hex( cos_approx($tiny) ), '3ff0000000000000', 'cos(just below 2^-27) = 1' );
    my $sub = hex_to_nv('0000000000000001');
    is( nv_to_hex( sin_approx($sub) ), '0000000000000001', 'sin(min subnormal) = x' );
};

subtest 'outside the domain is NaN' => sub {
    my @xs = (
        [ 'NaN',            hex_to_nv('7ff8000000000000') ],
        [ '+inf',           hex_to_nv('7ff0000000000000') ],
        [ '-inf',           hex_to_nv('fff0000000000000') ],
        [ '1e20',           1e20 ],
        [ '2^29 + 1 ulp',   hex_to_nv('41c0000000000001') ],
        [ 'max finite',     hex_to_nv('7fefffffffffffff') ],
        [ 'IV 2^62',        4611686018427387904 ],
    );
    for my $pair (@xs) {
        my ( $name, $x ) = @$pair;
        ok( is_nan( sin_approx($x) ), "sin($name)" );
        ok( is_nan( cos_approx($x) ), "cos($name)" );
        ok( is_nan( tan_approx($x) ), "tan($name)" );
    }
};

subtest 'the last in-domain value is finite' => sub {
    is( nv_to_hex( sin_approx( 2.0**29 ) ), '3fd4e67c0e2622df', 'sin(2^29)' );
    is( nv_to_hex( cos_approx( 2.0**29 ) ), 'bfee3edd2dfeef90', 'cos(2^29)' );
};

subtest 'integer arguments take the float path' => sub {
    is( nv_to_hex( sin_approx(1) ),   nv_to_hex( sin_approx(1.0) ),   'sin(1)' );
    is( nv_to_hex( cos_approx(3) ),   nv_to_hex( cos_approx(3.0) ),   'cos(3)' );
    is( nv_to_hex( sin_approx(-1) ),  nv_to_hex( sin_approx(-1.0) ),  'sin(-1)' );
    is( nv_to_hex( sin_approx(100000) ), nv_to_hex( sin_approx(100000.0) ), 'sin(100000)' );
    is( nv_to_hex( sin_approx(536870912) ), '3fd4e67c0e2622df', 'sin(IV 2^29)' );
    is( nv_to_hex( degrees_to_radians(180) ), nv_to_hex($CodingAdventures::Trig::PI), 'radians(180) = PI' );
    is( nv_to_hex( radians_to_degrees($CodingAdventures::Trig::PI) ), nv_to_hex(180.0), 'degrees(PI) = 180' );
    is( nv_to_hex( degrees_to_radians(0) ), '0000000000000000', 'radians(IV 0)' );
    is( nv_to_hex( radians_to_degrees(0) ), '0000000000000000', 'degrees(IV 0)' );
};

subtest 'radians and degrees are single multiplies' => sub {
    is( nv_to_hex( radians_to_degrees(1.7e308) ), '7ff0000000000000', 'degrees(1.7e308) overflows to +inf' );
    ok( is_nan( degrees_to_radians( hex_to_nv('7ff8000000000000') ) ), 'radians(NaN)' );
    is( nv_to_hex( degrees_to_radians( hex_to_nv('fff0000000000000') ) ), 'fff0000000000000', 'radians(-inf)' );
};

subtest 'tan pole sentinel' => sub {
    is( nv_to_hex( tan_approx( $CodingAdventures::Trig::HALF_PI ) ),  '7fe1ccf385ebc8a0', 'tan(HALF_PI) = 1e308' );
    is( nv_to_hex( tan_approx( -$CodingAdventures::Trig::HALF_PI ) ), 'ffe1ccf385ebc8a0', 'tan(-HALF_PI) = -1e308' );
};

subtest 'atan2 of NaN is NaN, and the pinned conventions' => sub {
    my $nan = hex_to_nv('7ff8000000000000');
    ok( is_nan( atan2_approx( $nan, 1.0 ) ),  'atan2(NaN, 1)' );
    ok( is_nan( atan2_approx( 1.0, $nan ) ),  'atan2(1, NaN)' );
    ok( is_nan( atan2_approx( -1.0, $nan ) ), 'atan2(-1, NaN)' );
    ok( is_nan( atan2_approx( $nan, $nan ) ), 'atan2(NaN, NaN)' );
    my $inf = hex_to_nv('7ff0000000000000');
    is( nv_to_hex( atan2_approx( hex_to_nv('8000000000000000'), -1.0 ) ), nv_to_hex($CodingAdventures::Trig::PI), 'atan2(-0, -1) = +PI' );
    is( nv_to_hex( atan2_approx( 0.0, 0.0 ) ), '0000000000000000', 'atan2(0, 0) = +0' );
    ok( is_nan( atan2_approx( $inf, $inf ) ), 'atan2(inf, inf) is NaN' );
};

subtest 'quadrants: negative k uses the floor modulus' => sub {
    my $h = $CodingAdventures::Trig::HALF_PI;
    is( nv_to_hex( sin_approx( -$h ) ), nv_to_hex( -sin_approx($h) ), 'sin(-HALF_PI) = -sin(HALF_PI)' );
    cmp_ok( abs( cos_approx( -5 * $h ) ), '<', 1e-15, 'cos(-5 HALF_PI) is about 0' );
};

subtest 'degree wrappers go through the new conversion' => sub {
    is( nv_to_hex( sin_deg(30) ), nv_to_hex( sin_approx( degrees_to_radians(30) ) ), 'sin_deg' );
    is( nv_to_hex( cos_deg(60) ), nv_to_hex( cos_approx( degrees_to_radians(60) ) ), 'cos_deg' );
    is( nv_to_hex( tan_deg(45) ), nv_to_hex( tan_approx( degrees_to_radians(45) ) ), 'tan_deg' );
    is( nv_to_hex( sin_deg( hex_to_nv('8000000000000000') ) ), '8000000000000000', 'sin_deg(-0) = -0' );
};

# The per-op summary: how many rows each op got wrong (all groups).  This is
# the number the TC-15 pull request quotes.
diag( join( '', map { sprintf( "  %-8s %6d failing of %6d\n", $_, $FAILED_BY_OP{$_} // 0, $CHECKED_BY_OP{$_} // 0 ) } sort keys %CHECKED_BY_OP ) );

done_testing;
