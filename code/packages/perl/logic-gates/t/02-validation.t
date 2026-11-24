use strict;
use warnings;
use Test2::V0;
use Scalar::Util qw(dualvar);

use CodingAdventures::LogicGates qw(
    AND OR NOT XOR NAND NOR XNOR
    NAND_NOT NAND_AND NAND_OR NAND_XOR
    ANDn ORn XORn
    SRLatch DLatch DFlipFlop Register ShiftRegister Counter
    new_flip_flop_state new_counter_state
);

# ===========================================================================
# Literal truth tables for all 11 gates: [ inputs..., expected ].
# ===========================================================================

my %tables = (
    AND      => [ [0,0,0], [0,1,0], [1,0,0], [1,1,1] ],
    OR       => [ [0,0,0], [0,1,1], [1,0,1], [1,1,1] ],
    XOR      => [ [0,0,0], [0,1,1], [1,0,1], [1,1,0] ],
    NAND     => [ [0,0,1], [0,1,1], [1,0,1], [1,1,0] ],
    NOR      => [ [0,0,1], [0,1,0], [1,0,0], [1,1,0] ],
    XNOR     => [ [0,0,1], [0,1,0], [1,0,0], [1,1,1] ],
    NAND_AND => [ [0,0,0], [0,1,0], [1,0,0], [1,1,1] ],
    NAND_OR  => [ [0,0,0], [0,1,1], [1,0,1], [1,1,1] ],
    NAND_XOR => [ [0,0,0], [0,1,1], [1,0,1], [1,1,0] ],
    NOT      => [ [0,1], [1,0] ],
    NAND_NOT => [ [0,1], [1,0] ],
);
my %fn = (
    AND => \&AND, OR => \&OR, XOR => \&XOR, NAND => \&NAND, NOR => \&NOR,
    XNOR => \&XNOR, NAND_AND => \&NAND_AND, NAND_OR => \&NAND_OR,
    NAND_XOR => \&NAND_XOR, NOT => \&NOT, NAND_NOT => \&NAND_NOT,
);

for my $name ( sort keys %tables ) {
    for my $row ( @{ $tables{$name} } ) {
        my @in  = @{$row}[ 0 .. $#$row - 1 ];
        my $exp = $row->[-1];
        is( $fn{$name}->(@in), $exp, "$name(" . join( ',', @in ) . ") = $exp" );
        # The string spellings "0"/"1" are accepted and give the same answer.
        is( $fn{$name}->( map {"$_"} @in ), $exp, "$name string form" );
        # Class-method call style still works.
        is( CodingAdventures::LogicGates->can($name)->( 'CodingAdventures::LogicGates', @in ),
            $exp, "$name method style" );
    }
}

# ===========================================================================
# Type-confusion matrix: every gate, every argument position, every bad value.
# ===========================================================================

package Overloaded {
    use overload '0+' => sub {1}, '""' => sub {'1'}, '==' => sub {1}, 'eq' => sub {1}, fallback => 1;
}

my @bad = (
    [ '"1abc"'       => '1abc' ],
    [ '"a"'          => 'a' ],
    [ 'empty string' => '' ],
    [ 'undef'        => undef ],
    [ '"1.0"'        => '1.0' ],
    [ '" 1"'         => ' 1' ],
    [ '"1 "'         => '1 ' ],
    [ '"1\\n"'       => "1\n" ],
    [ '"0\\n"'       => "0\n" ],
    [ '"01"'         => '01' ],
    [ '"+1"'         => '+1' ],
    [ '"-0"'         => '-0' ],
    [ '"0 but true"' => '0 but true' ],
    [ '"true"'       => 'true' ],
    [ '"2"'          => '2' ],
    [ '-1'           => -1 ],
    [ '0.5'          => 0.5 ],
    [ '1e3'          => 1e3 ],
    [ 'NaN'          => 9**9**9 / 9**9**9 ],
    [ 'Inf'          => 9**9**9 ],
    [ 'dualvar(1,"evil")'    => dualvar( 1, 'evil' ) ],
    [ 'dualvar(1,"x")'       => dualvar( 1, 'x' ) ],
    [ 'dualvar(0,"1abc")'    => dualvar( 0, '1abc' ) ],
    [ 'array ref'    => [1] ],
    [ 'hash ref'     => { 1 => 1 } ],
    [ 'code ref'     => sub {1} ],
    [ 'scalar ref'   => \1 ],
    [ 'blessed obj'  => bless( {}, 'Foo' ) ],
    [ 'overloaded obj' => bless( {}, 'Overloaded' ) ],
);

my %arity = map { $_ => scalar @{ $tables{$_}[0] } - 1 } keys %tables;

for my $name ( sort keys %fn ) {
    my $n = $arity{$name};
    for my $pos ( 0 .. $n - 1 ) {
        for my $case (@bad) {
            my ( $label, $value ) = @{$case};
            my @args = (1) x $n;
            $args[$pos] = $value;
            my @warnings;
            my $result;
            my $err = do {
                local $SIG{__WARN__} = sub { push @warnings, @_ };
                local $@;
                eval { $result = $fn{$name}->(@args); 1 } ? undef : $@;
            };
            like( $err, qr/\Alogic_gates: \w+ must be 0 or 1, got /,
                  "$name arg $pos rejects $label" );
            is( \@warnings, [], "$name arg $pos / $label: no warnings emitted" );
        }
    }
}

# Same matrix with warnings made fatal: the validator must not depend on them.
{
    use warnings FATAL => 'all';
    for my $case (@bad) {
        my $err = do { local $@; eval { AND( $case->[1], 1 ); 1 } ? undef : $@ };
        like( $err, qr/\Alogic_gates: a must be 0 or 1/, "FATAL warnings: AND rejects $case->[0]" );
    }
}

# ===========================================================================
# Regression test for the reported bug: numification accepted "1abc".
# ===========================================================================

like( dies { AND( "1abc", 1 ) }, qr/logic_gates: a must be 0 or 1, got '1abc'/,
      'regression: AND("1abc", 1) dies' );
like( dies { AND( 1, "1abc" ) }, qr/logic_gates: b must be 0 or 1/,
      'regression: AND(1, "1abc") dies' );
like( dies { AND( undef, 1 ) }, qr/got undef/, 'AND(undef, 1) dies' );

# Error message for refs and control characters is safe and readable.
like( dies { AND( [1], 1 ) }, qr/got a ARRAY reference/, 'ref described by type' );
like( dies { AND( "x\x00y", 1 ) }, qr/got 'x\\x00y'/, 'control chars escaped' );
like( dies { AND( 'x' x 100, 1 ) }, qr/got 'x{32}\.\.\.'/, 'long values truncated' );

# ===========================================================================
# Arity: wrong argument counts die instead of silently dropping an argument.
# ===========================================================================

like( dies { AND( 0, 1, 1 ) },    qr/AND takes 2 argument\(s\), got 3/, 'AND 3 args dies' );
like( dies { AND(1) },            qr/AND takes 2 argument\(s\), got 1/, 'AND 1 arg dies' );
like( dies { AND() },             qr/AND takes 2 argument\(s\), got 0/, 'AND 0 args dies' );
like( dies { NOT( 1, 1 ) },       qr/NOT takes 1 argument\(s\), got 2/, 'NOT 2 args dies' );
like( dies { NOT() },             qr/NOT takes 1 argument\(s\), got 0/, 'NOT 0 args dies' );
like( dies { XOR( 1, 1, 1, 1 ) }, qr/XOR takes 2 argument/,             'XOR 4 args dies' );
# A 3-argument call whose first argument is not our package name is an error,
# not a method call: AND(x, y, z) must never drop x.
like( dies { AND( 'Some::Other', 1, 1 ) }, qr/AND takes 2 argument\(s\), got 3/,
      'non-package invocant is not stripped' );
is( AND( 'CodingAdventures::LogicGates', 1, 1 ), 1, 'exact package invocant is stripped' );

# ===========================================================================
# Variadic gates.
# ===========================================================================

is( ANDn( 1, 1, 1 ), 1, 'ANDn 1,1,1' );
is( ANDn( 1, 1, 0 ), 0, 'ANDn 1,1,0' );
is( ANDn( [ 1, 1 ] ), 1, 'ANDn arrayref' );
is( ORn( 0, 0, 1 ), 1, 'ORn 0,0,1' );
is( ORn( 0, 0, 0 ), 0, 'ORn 0,0,0' );
is( ORn( 'CodingAdventures::LogicGates', 0, 1 ), 1, 'ORn method style' );
is( XORn( 1, 1, 1 ), 1, 'XORn odd' );
is( XORn( 1, 1 ), 0, 'XORn even' );
is( XORn(1), 1, 'XORn single' );
is( XORn(0), 0, 'XORn single zero' );

for my $f ( [ ANDn => \&ANDn ], [ ORn => \&ORn ], [ XORn => \&XORn ] ) {
    my ( $fname, $code ) = @{$f};
    for my $case (@bad) {
        for my $pos ( 0 .. 2 ) {
            my @args = ( 1, 1, 1 );
            $args[$pos] = $case->[1];
            like( dies { $code->(@args) }, qr/input must be 0 or 1/,
                  "$fname pos $pos rejects $case->[0]" );
            like( dies { $code->( \@args ) }, qr/input must be 0 or 1/,
                  "$fname arrayref pos $pos rejects $case->[0]" );
        }
    }
}
like( dies { ANDn(1) },  qr/ANDn requires at least 2/, 'ANDn needs 2' );
like( dies { ORn(1) },   qr/ORn requires at least 2/,  'ORn needs 2' );
like( dies { XORn() },   qr/XORn requires at least 1/, 'XORn needs 1' );

# ===========================================================================
# Sequential and state helpers.
# ===========================================================================

is( [ SRLatch( 1, 0, 0, 1 ) ], [ 1, 0 ], 'SRLatch set' );
is( [ SRLatch( 0, 1, 1, 0 ) ], [ 0, 1 ], 'SRLatch reset' );
is( [ SRLatch( 0, 0, 1, 0 ) ], [ 1, 0 ], 'SRLatch hold' );
is( [ DLatch( 1, 1, 0, 1 ) ],  [ 1, 0 ], 'DLatch transparent' );
is( [ DLatch( 1, 0, 0, 1 ) ],  [ 0, 1 ], 'DLatch hold' );

for my $case (@bad) {
    my ( $label, $v ) = @{$case};
    for my $pos ( 0 .. 3 ) {
        my @a = ( 0, 0, 0, 1 );
        $a[$pos] = $v;
        like( dies { SRLatch(@a) }, qr/must be 0 or 1/, "SRLatch pos $pos rejects $label" );
        like( dies { DLatch(@a) },  qr/must be 0 or 1/, "DLatch pos $pos rejects $label" );
    }
    like( dies { DFlipFlop( $v, 1 ) }, qr/data must be 0 or 1/,  "DFlipFlop data rejects $label" );
    like( dies { DFlipFlop( 1, $v ) }, qr/clock must be 0 or 1/, "DFlipFlop clock rejects $label" );
    like( dies { Register( [ 1, 0 ], $v ) }, qr/clock must be 0 or 1/, "Register clock rejects $label" );
    like( dies { Register( [ 1, $v ], 1 ) }, qr/data must be 0 or 1/,  "Register data rejects $label" );
    like( dies { ShiftRegister( $v, 1, [ new_flip_flop_state() ], 'left' ) },
          qr/serial_in must be 0 or 1/, "ShiftRegister serial_in rejects $label" );
    like( dies { ShiftRegister( 1, $v, [ new_flip_flop_state() ], 'left' ) },
          qr/clock must be 0 or 1/, "ShiftRegister clock rejects $label" );
    like( dies { Counter( $v, 0, new_counter_state(2) ) }, qr/clock must be 0 or 1/,
          "Counter clock rejects $label" );
    like( dies { Counter( 1, $v, new_counter_state(2) ) }, qr/reset must be 0 or 1/,
          "Counter reset rejects $label" );
    like( dies { DFlipFlop( 1, 1, { master_q => $v, master_q_bar => 1, slave_q => 0, slave_q_bar => 1 } ) },
          qr/must be 0 or 1/, "DFlipFlop state rejects $label" );
    like( dies { new_flip_flop_state( $v, 1, 0, 1 ) }, qr/must be 0 or 1/,
          "new_flip_flop_state rejects $label" );
}

like( dies { SRLatch( 0, 0, 0 ) }, qr/SRLatch takes 4/, 'SRLatch arity' );
like( dies { DLatch( 0, 0, 0 ) },  qr/DLatch takes 4/,  'DLatch arity' );
like( dies { DFlipFlop(1) },       qr/DFlipFlop takes 2 to 3/, 'DFlipFlop arity' );
like( dies { DFlipFlop( 1, 1, [] ) }, qr/state must be a hash/, 'DFlipFlop state type' );
like( dies { Register( 1, 1 ) },   qr/data must be an array reference/, 'Register data type' );
like( dies { Register( [], 1 ) },  qr/at least 1 bit/, 'Register empty' );
like( dies { Register( [1], 1, {} ) }, qr/state must be an array reference/, 'Register state type' );
like( dies { Register( [1], 1, [] ) }, qr/length mismatch/, 'Register state length' );
like( dies { ShiftRegister( 1, 1, [ new_flip_flop_state() ], undef ) }, qr/direction/, 'ShiftRegister undef direction' );
like( dies { ShiftRegister( 1, 1, [ new_flip_flop_state() ], [] ) },    qr/direction/, 'ShiftRegister ref direction' );
like( dies { ShiftRegister( 1, 1, [ new_flip_flop_state() ], 'up' ) },  qr/direction/, 'ShiftRegister bad direction' );
like( dies { ShiftRegister( 1, 1, undef, 'left' ) }, qr/non-empty state/, 'ShiftRegister undef state' );
like( dies { ShiftRegister( 1, 1, [], 'left' ) },    qr/non-empty state/, 'ShiftRegister empty state' );
like( dies { ShiftRegister( 1, 1, {}, 'left' ) },    qr/non-empty state/, 'ShiftRegister hash state' );
like( dies { Counter( 1, 0, undef ) }, qr/non-nil state/, 'Counter undef state' );
like( dies { Counter( 1, 0, [] ) },    qr/non-nil state/, 'Counter non-hash state' );
like( dies { Counter( 1, 0, { width => 0, bits => [] } ) }, qr/width must be 1 to 65536/, 'Counter width 0' );
like( dies { Counter( 1, 0, { width => 2, bits => [0] } ) }, qr/array of width bits/, 'Counter bits length' );
like( dies { Counter( 1, 0, { width => 2, bits => [ 0, 'x' ] } ) }, qr/must be 0 or 1/, 'Counter bits content' );
like( dies { Counter( 1, 0 ) },    qr/Counter takes 3/, 'Counter arity' );
like( dies { new_flip_flop_state( 0, 1 ) }, qr/takes 0 or 4/, 'new_flip_flop_state odd arity' );
like( dies { new_flip_flop_state( 1, 2, 3, 4, 5 ) }, qr/takes 0 to 4/, 'new_flip_flop_state too many' );

for my $w ( 'abc', '', undef, 0, -1, 1.5, '01', ' 2', "2\n", [2], 10_000_000 ) {
    like( dies { new_counter_state($w) }, qr/Counter width must be 1 to 65536/,
          'new_counter_state rejects ' . ( defined $w ? "'$w'" : 'undef' ) );
}
is( new_counter_state(3), { bits => [ 0, 0, 0 ], width => 3 }, 'new_counter_state(3)' );
is( new_counter_state('4')->{width}, '4', 'new_counter_state accepts "4"' );
is( new_flip_flop_state(), { master_q => 0, master_q_bar => 1, slave_q => 0, slave_q_bar => 1 },
    'default flip-flop state' );
is( new_flip_flop_state( 1, 0, 1, 0 ),
    { master_q => 1, master_q_bar => 0, slave_q => 1, slave_q_bar => 0 }, 'explicit flip-flop state' );

# Counter still counts: 0 -> 1 -> 2 -> 3 -> 0 (2-bit wrap), LSB first.
{
    my $st = new_counter_state(2);
    my $bits;
    my @seen;
    for ( 1 .. 4 ) {
        ( $bits, $st ) = Counter( 1, 0, $st );
        push @seen, join '', @{$bits};
    }
    is( \@seen, [ '10', '01', '11', '00' ], 'Counter counts and wraps' );
    ( $bits, $st ) = Counter( 1, 1, $st );
    is( $bits, [ 0, 0 ], 'Counter reset' );
    ( $bits, $st ) = Counter( 0, 0, $st );
    is( $bits, [ 0, 0 ], 'Counter hold' );
}

# ShiftRegister right and left, Register round-trip.
{
    my $st = [ map { new_flip_flop_state() } 1 .. 3 ];
    my ( $out, $so );
    for my $bit ( 1, 0, 1 ) {
        ( $out, $so, $st ) = ShiftRegister( $bit, 1, $st, 'left' );
        ( $out, $so, $st ) = ShiftRegister( $bit, 0, $st, 'left' );
    }
    is( scalar @{$out}, 3, 'ShiftRegister left keeps width' );
    my $rst = [ map { new_flip_flop_state() } 1 .. 3 ];
    ( $out, $so, $rst ) = ShiftRegister( 1, 1, $rst, 'right' );
    ( $out, $so, $rst ) = ShiftRegister( 1, 0, $rst, 'right' );
    is( $out->[2], 1, 'ShiftRegister right shifts into MSB' );
    my ( $q, $qb, $fs ) = DFlipFlop( 1, 1 );
    ( $q, $qb, $fs ) = DFlipFlop( 1, 0, $fs );
    is( [ $q, $qb ], [ 1, 0 ], 'DFlipFlop captures 1' );
}


# ===========================================================================
# Canonicalisation: the STRING form decides; computation uses a fresh integer.
# A dualvar whose numeric part disagrees with its string part is accepted as
# the bit its STRING says (documented rule), never computed on its number.
# ===========================================================================

{
    my @dual = (
        [ 'dualvar(5,"1")' => dualvar( 5, '1' ), 1 ],
        [ 'dualvar(0,"1")' => dualvar( 0, '1' ), 1 ],
        [ 'dualvar(1,"0")' => dualvar( 1, '0' ), 0 ],
        [ 'dualvar(7,"0")' => dualvar( 7, '0' ), 0 ],
    );
    for my $name ( sort grep { $arity{$_} == 2 } keys %fn ) {
        for my $d (@dual) {
            my ( $label, $v, $bit ) = @{$d};
            is( $fn{$name}->( $v, 1 ), $fn{$name}->( $bit, 1 ), "$name($label, 1) computes on the string form" );
            is( $fn{$name}->( 1, $v ), $fn{$name}->( 1, $bit ), "$name(1, $label) computes on the string form" );
        }
    }
    for my $name (qw(NOT NAND_NOT)) {
        for my $d (@dual) {
            is( $fn{$name}->( $d->[1] ), $fn{$name}->( $d->[2] ), "$name($d->[0])" );
        }
    }
    is( AND( dualvar( 5, '1' ), 1 ), 1, 'AND(dualvar(5,"1"),1) = 1 (was 0 before canonicalising)' );
    is( AND( dualvar( 0, '1' ), 1 ), 1, 'AND(dualvar(0,"1"),1) = 1' );
    is( ANDn( dualvar( 5, '1' ), 1, 1 ), 1, 'ANDn canonicalises' );
    is( [ SRLatch( dualvar( 5, '1' ), 0, 0, 1 ) ], [ 1, 0 ], 'SRLatch canonicalises' );
}

# ===========================================================================
# CWE-400: absurd widths must be rejected promptly, never allocated.
# (The old numeric check let a reference through; it numifies to its address,
# ~1e11, and `(0) x $width` then exhausted memory / hung.)
# ===========================================================================

{
    my @huge = ( [2], {}, sub {1}, 1e12, '1e3', '100000', 65537, '999999999999', 2**40, 9**9**9 );
    my $t0 = time;
    local $SIG{ALRM} = sub { die "TIMEOUT\n" };
    alarm 10;
    for my $w (@huge) {
        like( dies { new_counter_state($w) }, qr/Counter width must be 1 to 65536/, 'huge width rejected' );
        like( dies { Counter( 1, 0, { width => $w, bits => [] } ) }, qr/Counter width must be 1 to 65536/,
              'Counter huge state width rejected' );
    }
    is( scalar @{ new_counter_state(65536)->{bits} }, 65536, 'maximum width 65536 accepted' );
    alarm 0;
    ok( time - $t0 < 10, 'rejection of absurd widths returns promptly' );
}

done_testing;
