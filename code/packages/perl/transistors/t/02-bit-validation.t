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
done_testing;
