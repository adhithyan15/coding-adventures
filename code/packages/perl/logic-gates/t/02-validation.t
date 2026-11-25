use strict;
use warnings;
use Test2::V0;
use Scalar::Util qw(dualvar);

use CodingAdventures::LogicGates qw(
    AND OR NOT XOR NAND NOR XNOR
    NAND_NOT NAND_AND NAND_OR NAND_XOR
    ANDn ORn XORn
    SRLatch DLatch DFlipFlop Register ShiftRegister Counter
    new_flip_flop_state new_counter_state is_invalid_bit
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
        # Class-method call style was removed: the package name is just data.
        like( dies { $fn{$name}->( 'CodingAdventures::LogicGates', @in ) }, qr/logic_gates: /,
              "$name rejects a leading package name" );
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
            # Typed identity (LG02 s7): class, stable code, first bad param.
            is( ref($err), 'CodingAdventures::LogicGates::Error', "$name arg $pos / $label: error class" );
            is( $err->code,  'INVALID_BIT', "$name arg $pos / $label: INVALID_BIT" );
            is( $err->param, $pos == 0 ? 'a' : 'b', "$name arg $pos / $label: param name" );
            ok( is_invalid_bit($err), "$name arg $pos / $label: is_invalid_bit" );
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
like( dies { AND( 'CodingAdventures::LogicGates', 1, 1 ) }, qr/AND takes 2 argument\(s\), got 3/,
      'package-name invocant is NOT stripped either' );

# ===========================================================================
# Variadic gates.
# ===========================================================================

is( ANDn( 1, 1, 1 ), 1, 'ANDn 1,1,1' );
is( ANDn( 1, 1, 0 ), 0, 'ANDn 1,1,0' );
is( ANDn( [ 1, 1 ] ), 1, 'ANDn arrayref' );
is( ORn( 0, 0, 1 ), 1, 'ORn 0,0,1' );
is( ORn( 0, 0, 0 ), 0, 'ORn 0,0,0' );
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
like( dies { Counter( 1, 0, { width => 0, bits => [] } ) }, qr/width must be 1 to 64/, 'Counter width 0' );
like( dies { Counter( 1, 0, { width => 2, bits => [0] } ) }, qr/array of width bits/, 'Counter bits length' );
like( dies { Counter( 1, 0, { width => 2, bits => [ 0, 'x' ] } ) }, qr/must be 0 or 1/, 'Counter bits content' );
like( dies { Counter( 1, 0 ) },    qr/Counter takes 3/, 'Counter arity' );
like( dies { new_flip_flop_state( 0, 1 ) }, qr/takes 0 or 4/, 'new_flip_flop_state odd arity' );
like( dies { new_flip_flop_state( 1, 2, 3, 4, 5 ) }, qr/takes 0 to 4/, 'new_flip_flop_state too many' );

for my $w ( 'abc', '', undef, 0, -1, 1.5, '01', ' 2', "2\n", [2], 10_000_000 ) {
    like( dies { new_counter_state($w) }, qr/Counter width must be 1 to 64/,
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
        like( dies { new_counter_state($w) }, qr/Counter width must be 1 to 64/, 'huge width rejected' );
        like( dies { Counter( 1, 0, { width => $w, bits => [] } ) }, qr/Counter width must be 1 to 64/,
              'Counter huge state width rejected' );
    }
    is( scalar @{ new_counter_state(64)->{bits} }, 64, 'maximum width 64 accepted' );
    like( dies { new_counter_state(65) }, qr/1 to 64/, 'width 65 rejected' );
    alarm 0;
    ok( time - $t0 < 10, 'rejection of absurd widths returns promptly' );
}

# ===========================================================================
# Canonical widths: the STRING form decides (dualvar(-5,"3") is width 3).
# ===========================================================================

{
    for my $dv ( dualvar( -5, '3' ), dualvar( 70000, '3' ), dualvar( 0, '3' ) ) {
        my $st = new_counter_state($dv);
        is( $st->{width}, 3, 'dualvar width canonicalised to its string form' );
        is( scalar @{ $st->{bits} }, 3, 'dualvar width: bits sized from the string form' );
        my ( $bits, $st2 ) = Counter( 1, 0, { width => $dv, bits => [ 0, 0, 0 ] } );
        is( $bits, [ 1, 0, 0 ], 'Counter with dualvar width counts on width 3' );
        is( $st2->{width}, 3, 'Counter stores the canonical width' );
    }
    like( dies { new_counter_state( dualvar( 3, '-5' ) ) }, qr/Counter width must be/, 'dualvar(3,"-5") rejected' );
}

# ===========================================================================
# Error text is bounded: a 10 MB input must not be copied into the message.
# ===========================================================================

{
    my $huge = 'x' x ( 10 * 1024 * 1024 );
    my $err = dies { AND( $huge, 1 ) };
    like( $err, qr/must be 0 or 1/, 'huge string rejected' );
    ok( length($err) < 200, 'error message for a 10 MB input is short' );
    my $err2 = dies { new_counter_state($huge) };
    ok( length($err2) < 200, 'width error for a 10 MB input is short' );
}

# ===========================================================================
# No method-style invocation anywhere: a leading package name is plain data and
# every public function rejects it (nothing is silently dropped).
# ===========================================================================

{
    my $C = 'CodingAdventures::LogicGates';
    like( dies { ANDn( $C, 1, 1 ) },  qr/input must be 0 or 1, got '\Q$C\E'/, 'ANDn(pkg,1,1) dies' );
    like( dies { ANDn( $C, 1, 0 ) },  qr/input must be 0 or 1/, 'ANDn(pkg,1,0) dies' );
    like( dies { ORn( $C, 0, 0 ) },   qr/input must be 0 or 1/, 'ORn(pkg,0,0) dies' );
    like( dies { ORn( $C, 0, 1 ) },   qr/input must be 0 or 1/, 'ORn(pkg,0,1) dies' );
    like( dies { XORn( $C, 1, 0 ) },  qr/input must be 0 or 1/, 'XORn(pkg,1,0) dies' );
    like( dies { XORn( $C, 1 ) },     qr/input must be 0 or 1/, 'XORn(pkg,1) dies' );
    like( dies { XORn($C) },          qr/input must be 0 or 1/, 'XORn(pkg) dies' );
    like( dies { ANDn( [ $C, 1 ] ) }, qr/input must be 0 or 1/, 'ANDn([pkg,1]) dies' );
    like( dies { SRLatch( $C, 1, 0, 0, 1 ) }, qr/SRLatch takes 4/, 'SRLatch(pkg,...) dies' );
    like( dies { SRLatch( $C, 0, 0, 1 ) },    qr/data|set must be 0 or 1/, 'SRLatch pkg as set dies' );
    like( dies { DLatch( $C, 1, 0, 1 ) },     qr/data must be 0 or 1/, 'DLatch pkg dies' );
    like( dies { DFlipFlop( $C, 1, 0 ) },     qr/data must be 0 or 1/, 'DFlipFlop(pkg,1,0) dies' );
    like( dies { DFlipFlop( $C, 1 ) },        qr/data must be 0 or 1/, 'DFlipFlop(pkg,1) dies' );
    like( dies { Register( $C, [ 1, 0 ], 1 ) }, qr/takes 2 to 3|clock must be 0 or 1/, 'Register(pkg,...) dies' );
    like( dies { Register( [ 1, 0 ], 1, $C ) }, qr/state must be an array reference/, 'Register pkg as state dies' );
    like( dies { ShiftRegister( $C, 1, [ new_flip_flop_state() ], 'left' ) }, qr/serial_in must be 0 or 1/,
          'ShiftRegister(pkg,...) dies' );
    like( dies { ShiftRegister( $C, 1, 0, [ new_flip_flop_state() ], 'left' ) }, qr/ShiftRegister takes 4/,
          'ShiftRegister 5 args dies' );
    like( dies { Counter( $C, 1, 0, new_counter_state(2) ) }, qr/Counter takes 3/, 'Counter(pkg,...) dies' );
    like( dies { Counter( $C, 0, new_counter_state(2) ) },    qr/clock must be 0 or 1/, 'Counter pkg as clock dies' );
    like( dies { new_flip_flop_state($C) },      qr/takes 0 or 4/, 'new_flip_flop_state(pkg) dies' );
    like( dies { new_flip_flop_state( $C, 1, 0, 1, 0 ) }, qr/takes 0 to 4/, 'new_flip_flop_state(pkg,4) dies' );
    like( dies { new_flip_flop_state( $C, 1, 0, 1 ) },    qr/must be 0 or 1/, 'new_flip_flop_state pkg as bit dies' );
    like( dies { new_counter_state($C) },        qr/Counter width must be/, 'new_counter_state(pkg) dies' );
    like( dies { new_counter_state( $C, 2 ) },   qr/new_counter_state takes 1/, 'new_counter_state(pkg,2) dies' );
    like( dies { NOT( $C, 1 ) },                 qr/NOT takes 1/, 'NOT(pkg,1) dies' );
    like( dies { NAND_NOT( $C, 1 ) },            qr/NAND_NOT takes 1/, 'NAND_NOT(pkg,1) dies' );
    # Real method-call syntax is not supported either.
    like( dies { CodingAdventures::LogicGates->AND( 1, 1 ) }, qr/AND takes 2 argument\(s\), got 3/,
          'Class->AND dies' );
}


# ===========================================================================
# Typed error identity: codes, accessors, is_invalid_bit, stringification.
# ===========================================================================

sub err_of(&) { my $c = shift; local $@; eval { $c->(); 1 } ? undef : $@ }

{
    my $e = err_of { AND( 0, 1, 1 ) };
    is( ref($e), 'CodingAdventures::LogicGates::Error', 'arity: class' );
    is( $e->code, 'INVALID_ARITY', 'AND arity code' );
    ok( !is_invalid_bit($e), 'arity error is not invalid_bit' );
    is( err_of { ANDn(1) }->code,              'INVALID_ARITY', 'ANDn arity' );
    is( err_of { new_flip_flop_state( 0, 1 ) }->code, 'INVALID_ARITY', 'new_flip_flop_state arity' );
    is( err_of { DFlipFlop(1) }->code,         'INVALID_ARITY', 'DFlipFlop arity' );

    my $w = err_of { new_counter_state('abc') };
    is( $w->code, 'INVALID_WIDTH', 'width code' );
    is( $w->param, 'width', 'width param' );
    is( $w->value_description, "'abc'", 'width value_description' );
    is( err_of { Counter( 1, 0, { width => 0, bits => [] } ) }->code, 'INVALID_WIDTH', 'Counter width' );

    is( err_of { Counter( 1, 0, [] ) }->code, 'INVALID_SHAPE', 'shape: non-hash state' );
    is( err_of { Counter( 1, 0, { width => 2, bits => [0] } ) }->code, 'INVALID_SHAPE', 'shape: bits length' );
    is( err_of { Register( [], 1 ) }->code, 'INVALID_SHAPE', 'shape: empty register' );
    my $d = err_of { ShiftRegister( 1, 1, [0], 'up' ) };
    is( $d->code, 'INVALID_SHAPE', 'shape: direction' );
    is( $d->param, 'direction', 'direction param' );
    is( err_of { Counter( 1, 0, { width => 2, bits => [ 0, 'x' ] } ) }->code, 'INVALID_BIT', 'bits content is INVALID_BIT' );

    # value_description is the bounded rendering, never the raw value.
    my $big = err_of { AND( 'x' x 100_000, 1 ) };
    is( $big->value_description, "'" . ( 'x' x 32 ) . "...'", 'value_description bounded' );
    ok( length( $big->message ) < 200, 'message bounded' );
    my $ref = err_of { AND( [1], 1 ) };
    is( $ref->value_description, 'a ARRAY reference', 'ref described, not dumped' );

    # Stringification is the old message, ending in newline; bool/eq/ne work.
    my $s = err_of { AND( 'x', 1 ) };
    is( "$s", "logic_gates: a must be 0 or 1, got 'x'\n", 'stringifies to message' );
    ok( $s, 'bool is true' );
    ok( $s eq "logic_gates: a must be 0 or 1, got 'x'\n", 'eq falls back to string' );
    ok( $s ne 'other', 'ne works' );
    is( $s->message, "logic_gates: a must be 0 or 1, got 'x'\n", 'message accessor' );
}

# is_invalid_bit: exact class + code only.
{
    my $real = err_of { NOT('z') };
    ok( is_invalid_bit($real), 'real INVALID_BIT' );
    ok( !is_invalid_bit( { code => 'INVALID_BIT' } ), 'forged hashref rejected' );
    ok( !is_invalid_bit( bless { code => 'INVALID_BIT' }, 'Other::Class' ), 'foreign blessed rejected' );
    ok( !is_invalid_bit(undef), 'undef' );
    ok( !is_invalid_bit('INVALID_BIT'), 'plain string' );
    ok( !is_invalid_bit("logic_gates: a must be 0 or 1\n"), 'old-style message string' );
    {
        package Sub::Err;
        our @ISA = ('CodingAdventures::LogicGates::Error');
    }
    my $sub = bless { %{$real} }, 'Sub::Err';
    ok( !is_invalid_bit($sub), 'subclass instance rejected' );
    ok( dies { CodingAdventures::LogicGates::Error->new( code => 'NOPE', message => 'x' ) }, 'unknown code refused' );
}

# String operators preserve the message; numeric ones fail loudly.
{
    my $e = err_of { AND( 'x', 1 ) };
    my $m = "logic_gates: a must be 0 or 1, got 'x'\n";
    is( "x: $e", "x: $m", 'interpolation' );
    is( $e . "y", $m . "y", 'concat right' );
    is( "x" . $e, "x" . $m, 'concat left' );
    my $w;
    { local $SIG{__WARN__} = sub { $w = shift }; warn "failed: $e"; }
    is( $w, "failed: $m", 'warn wrapping' );
    my $wrapped = do { local $@; eval { die "wrapped: $e" }; $@ };
    is( $wrapped, "wrapped: $m", 'die wrapping' );
    is( ( $e x 2 ), $m x 2, 'repetition' );
    is( [ sort { $a cmp $b } ( $e, 'a', 'z' ) ]->[0], 'a', 'cmp sorts by message' );
    is( $e cmp $m, 0, 'cmp equal' );
    ok( dies { my $n = $e == 1 }, '== still fails loudly' );
    ok( dies { my $n = $e + 1 },  '+ still fails loudly' );
}

# Negation, .=, string relational operators, swapped x.
{
    my $e = err_of { AND( 'x', 1 ) };
    my $m = "logic_gates: a must be 0 or 1, got 'x'\n";
    ok( !( !$e ), 'double negation true' );
    my $nb = !$e;
    ok( !$nb, '!$err is false, no die' );
    my $c = $e;
    $c .= "z";
    is( $c, $m . "z", '.= gives plain string' );
    is( ref( \$c ), 'SCALAR', '.= result is not the object' );
    ok( $e lt "m", 'lt' );  ok( !( "m" lt $e ), 'lt swapped' );
    ok( $e le $m, 'le' );   ok( "m" ge $e, 'ge swapped' );
    ok( $e gt "a", 'gt' );  ok( "z" gt $e, 'gt swapped' );
    ok( $e ge $m, 'ge' );   ok( "a" le $e, 'le swapped' );
    ok( dies { my $n = 3 x $e }, 'swapped x dies loudly' );
    for my $op ( '-', '*', '/', '<=>', '0+' ) {
        my $code = $op eq '0+' ? '0 + $e' : "\$e $op 1";
        ok( dies { eval "no warnings; my \$r = $code; 1" or die $@ }, "$op fails loudly" );
    }
}

done_testing;
