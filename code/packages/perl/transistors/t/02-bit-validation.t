use strict;
use warnings;
use Test2::V0;
use Scalar::Util qw(dualvar);
use CodingAdventures::Transistors qw(cmos_not cmos_nand cmos_nor cmos_and cmos_or cmos_xor cmos_xnor);

# Regression: numeric != used to numify these into valid-looking bits.
my @bad = ( '1abc', 'a', '', undef, '1.0', ' 1', "1\n", '01', '+1', '0 but true', [1], {}, bless( {}, 'X' ) );
for my $f ( [ cmos_nand => \&cmos_nand ], [ cmos_nor => \&cmos_nor ], [ cmos_and => \&cmos_and ],
            [ cmos_or => \&cmos_or ], [ cmos_xor => \&cmos_xor ], [ cmos_xnor => \&cmos_xnor ] ) {
    for my $v (@bad) {
        my $d = defined $v ? ( ref $v ? ref $v : "'$v'" ) : 'undef';
        like( dies { $f->[1]->( $v, 1 ) }, qr/Transistors: a must be 0 or 1/, "$f->[0] a rejects $d" );
        like( dies { $f->[1]->( 1, $v ) }, qr/Transistors: b must be 0 or 1/, "$f->[0] b rejects $d" );
    }
}
for my $v (@bad) {
    like( dies { cmos_not($v) }, qr/Transistors: a must be 0 or 1/, 'cmos_not rejects bad value' );
}
is( cmos_nand( '1', '1' ), 0, 'string bits still accepted' );
is( cmos_not(0), 1, 'cmos_not(0)' );

# Canonicalisation: dualvars are read by their STRING form and computed on a
# fresh integer, so a numeric part that disagrees cannot change the answer.
for my $f ( [ cmos_nand => \&cmos_nand ], [ cmos_nor => \&cmos_nor ], [ cmos_and => \&cmos_and ],
            [ cmos_or => \&cmos_or ], [ cmos_xor => \&cmos_xor ], [ cmos_xnor => \&cmos_xnor ] ) {
    for my $d ( [ dualvar( 5, '1' ), 1 ], [ dualvar( 0, '1' ), 1 ], [ dualvar( 1, '0' ), 0 ], [ dualvar( 9, '0' ), 0 ] ) {
        is( $f->[1]->( $d->[0], 1 ), $f->[1]->( $d->[1], 1 ), "$f->[0] dualvar a" );
        is( $f->[1]->( 1, $d->[0] ), $f->[1]->( 1, $d->[1] ), "$f->[0] dualvar b" );
    }
    like( dies { $f->[1]->( dualvar( 1, 'x' ), 1 ) }, qr/must be 0 or 1/, "$f->[0] rejects dualvar(1,'x')" );
    like( dies { $f->[1]->( 1, dualvar( 1, 'x' ) ) }, qr/must be 0 or 1/, "$f->[0] rejects dualvar(1,'x') b" );
}
is( cmos_not( dualvar( 5, '1' ) ), 0, 'cmos_not(dualvar(5,"1")) = 0' );
is( cmos_not( dualvar( 1, '0' ) ), 1, 'cmos_not(dualvar(1,"0")) = 1' );
like( dies { cmos_not( dualvar( 1, 'x' ) ) }, qr/must be 0 or 1/, 'cmos_not rejects dualvar(1,"x")' );
# Arity: wrong counts die (cmos_nand(0,1,1) used to silently drop the 0).
{
    my $C = 'CodingAdventures::Transistors';
    my %two = ( cmos_nand => \&cmos_nand, cmos_nor => \&cmos_nor, cmos_and => \&cmos_and,
                cmos_or => \&cmos_or, cmos_xor => \&cmos_xor, cmos_xnor => \&cmos_xnor,
                ttl_nand => \&CodingAdventures::Transistors::ttl_nand );
    my %one = ( cmos_not => \&cmos_not, ttl_not => \&CodingAdventures::Transistors::ttl_not );
    for my $n ( sort keys %two ) {
        like( dies { $two{$n}->( 0, 1, 1 ) }, qr/Transistors: $n takes 2 argument\(s\), got 3/, "$n 3 args dies" );
        like( dies { $two{$n}->(1) },         qr/$n takes 2 argument\(s\), got 1/,              "$n 1 arg dies" );
        like( dies { $two{$n}->() },          qr/$n takes 2 argument\(s\), got 0/,              "$n 0 args dies" );
        like( dies { $two{$n}->( $C, 1, 0 ) }, qr/takes 2 argument\(s\), got 3/, "$n method style removed" );
        like( dies { $two{$n}->( $C, 1 ) }, qr/must be 0 or 1/, "$n package name as a bit dies" );
        like( dies { $two{$n}->( 1, $C ) }, qr/must be 0 or 1/, "$n package name as b dies" );
    }
    for my $n ( sort keys %one ) {
        like( dies { $one{$n}->( 0, 1 ) }, qr/$n takes 1 argument\(s\), got 2/, "$n 2 args dies" );
        like( dies { $one{$n}->() },       qr/$n takes 1 argument\(s\), got 0/, "$n 0 args dies" );
        like( dies { $one{$n}->( $C, 1 ) }, qr/takes 1 argument\(s\), got 2/, "$n method style removed" );
        like( dies { $one{$n}->($C) }, qr/must be 0 or 1/, "$n package name as data dies" );
    }
}

# ===========================================================================
# Analog functions: strict arity, strict numeric shape, no invocant guessing.
# ===========================================================================
{
    my %f = (
        nmos => \&CodingAdventures::Transistors::nmos, pmos => \&CodingAdventures::Transistors::pmos,
        npn  => \&CodingAdventures::Transistors::npn,  pnp  => \&CodingAdventures::Transistors::pnp,
    );
    # Literal results pinned from the pre-change formulas.
    is( [ $f{nmos}->( 1.8, 0.4 ), $f{nmos}->( 0.3, 0.4 ), $f{nmos}->( 0.4, 0.4 ), $f{nmos}->(3.3) ], [ 1, 0, 1, 1 ], 'nmos results' );
    is( [ $f{pmos}->( -3.3, 0.4 ), $f{pmos}->( 0, 0.4 ), $f{pmos}->(-0.4) ], [ 1, 0, 1 ], 'pmos results' );
    is( [ $f{npn}->(1e-6), $f{npn}->(0), $f{npn}->( -1, 50 ) ], [ 1, 0, 0 ], 'npn results' );
    is( [ $f{pnp}->(2), $f{pnp}->( 0, 100 ) ], [ 1, 0 ], 'pnp results' );
    is( [ $f{nmos}->( '1.8', '4e-1' ), $f{nmos}->( '+.5', '0.4' ), $f{nmos}->( '-1.', '-2' ) ], [ 1, 1, 1 ], 'numeric strings accepted' );
    is( $f{nmos}->( 1.8, undef ), 1, 'undef optional arg uses the default' );
    is( $f{nmos}->( dualvar( -5, '1.8' ), 0.4 ), 1, 'dualvar computed on its string form' );

    my @badnum = ( '1abc', '', undef, 'nan', 'NaN', 'inf', '-inf', 'Inf', ' 1', '1 ', "1\n", '0x10', '1_0', '--1', '1e', '.', '1e999',
                   [1], {}, sub {1}, bless( {}, 'X' ), 'CodingAdventures::Transistors', 'Foo::Bar', dualvar( 1, 'x' ) );
    for my $n ( sort keys %f ) {
        like( dies { $f{$n}->() },           qr/$n takes 1 to 2 argument\(s\), got 0/, "$n() arity" );
        like( dies { $f{$n}->( 1, 1, 1 ) },  qr/$n takes 1 to 2 argument\(s\), got 3/, "$n 3 args arity" );
        like( dies { $f{$n}->( 'Foo::Bar', 1.8, 0.4 ) }, qr/$n takes 1 to 2 argument\(s\), got 3/, "$n(Foo::Bar,x,y) dies" );
        like( dies { $f{$n}->( 'CodingAdventures::Transistors', 1.8, 0.4 ) }, qr/takes 1 to 2/, "$n method style dies" );
        for my $v (@badnum) {
            like( dies { $f{$n}->( $v, 0.4 ) }, qr/Transistors: \w+ must be a finite number/, "$n arg1 rejects bad value" );
            # An explicit undef for the OPTIONAL second argument means "use the default".
            next if !defined $v;
            like( dies { $f{$n}->( 1, $v ) },   qr/Transistors: \w+ must be a finite number/, "$n arg2 rejects bad value" );
        }
        my @w;
        local $SIG{__WARN__} = sub { push @w, @_ };
        eval { $f{$n}->(undef) };
        is( \@w, [], "$n(undef) emits no warning" );
    }
}

done_testing;
