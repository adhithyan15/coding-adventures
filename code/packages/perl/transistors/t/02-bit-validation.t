use strict;
use warnings;
use Test2::V0;
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
done_testing;
