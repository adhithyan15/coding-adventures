use strict;
use warnings;
use Test2::V0;
use Scalar::Util qw(dualvar blessed);

use CodingAdventures::LogicGates qw(
    AND OR NOT XOR NAND NOR XNOR
    NAND_NOT NAND_AND NAND_OR NAND_XOR
    ANDn ORn XORn
    SRLatch DLatch DFlipFlop Register ShiftRegister Counter
    new_flip_flop_state new_counter_state
);

my $ERR = 'CodingAdventures::LogicGates::Error';

sub err_of (&) { my $c = shift; my $r = eval { $c->(); 1 }; return $r ? undef : $@ }
sub is_err { my ( $e, $code, $name ) = @_;
    ok( blessed($e) && ref($e) eq $ERR && $e->code eq $code, $name ) or diag( "got: " . ( $e // 'undef' ) ) }

# Spy classes: every call into caller code is counted.
my $calls = 0;
{
    package SpyScalar; sub TIESCALAR { bless {}, shift } sub FETCH { $calls++; 1 } sub STORE { $calls++ }
    package SpyArray;  sub TIEARRAY { bless {}, shift } sub FETCH { $calls++; 1 } sub FETCHSIZE { $calls++; 2 }
                       sub STORE { $calls++ } sub STORESIZE { $calls++ } sub EXISTS { $calls++; 1 }
    package SpyHash;   sub TIEHASH { bless {}, shift } sub FETCH { $calls++; 1 } sub EXISTS { $calls++; 1 }
                       sub FIRSTKEY { $calls++; undef } sub NEXTKEY { undef } sub STORE { $calls++ } sub SCALAR { $calls++; 0 }
    package SpyObj;    use overload '""' => sub { $calls++; '1' }, '0+' => sub { $calls++; 1 },
                       'bool' => sub { $calls++; 1 }, '==' => sub { $calls++; 1 }, 'eq' => sub { $calls++; 1 }, fallback => 1;
}

my %gate2 = ( AND => \&AND, OR => \&OR, XOR => \&XOR, NAND => \&NAND, NOR => \&NOR, XNOR => \&XNOR,
              NAND_AND => \&NAND_AND, NAND_OR => \&NAND_OR, NAND_XOR => \&NAND_XOR );

# --- short-circuit rows: a decisive first argument must not skip validation of the second.
for my $g ( sort keys %gate2 ) {
    for my $first ( 0, 1 ) {
        for my $bad ( 'x', undef, '1abc', [], 2, "\x{0661}" ) {
            my $e = err_of { $gate2{$g}->( $first, $bad ) };
            is_err( $e, 'INVALID_BIT', "$g($first, bad) rejected" );
        }
    }
}

# --- exotic numerics and spellings are not bits.
{
    no warnings;
    for my $v ( 2**53 + 1, 2**64, 'inf', 'nan', '-0', "\x{0661}", "\x{ff11}", 1e0 * 2, !!0 eq '' ? '' : 'z' ) {
        is_err( err_of { AND( $v, 1 ) }, 'INVALID_BIT', 'AND rejects ' . ( $v // 'undef' ) );
    }
    # -0.0 stringifies to "0" or "-0" depending on the perl; either way it is never mixed up.
    my $nz = -0.0;
    my $e = err_of { AND( $nz, 1 ) };
    ok( !defined $e || $e->code eq 'INVALID_BIT', '-0.0 is a bit or INVALID_BIT, nothing else' );
    # !!0 is the dualvar ("" , 0): the empty string is not a bit.
    is_err( err_of { AND( !!0, 1 ) }, 'INVALID_BIT', '!!0 rejected' );
    is( AND( !!1, 1 ), 1, '!!1 is the string "1"' );
}

# --- dualvars: canonical outputs everywhere.
{
    my $dv = dualvar( 5, '1' );
    my @r = ( AND( $dv, $dv ), OR( $dv, 0 ), NOT( dualvar( 7, '0' ) ), ANDn( $dv, $dv ), XORn($dv) );
    is( \@r, [ 1, 1, 1, 1, 1 ], 'dualvar inputs behave as their string' );
    ok( $_ == 0 || $_ == 1, 'canonical numeric' ) for @r;

    my $st = [ map { { master_q => 0, master_q_bar => 1, slave_q => $dv, slave_q_bar => dualvar( 9, '0' ) } } 1 .. 2 ];
    for my $dir (qw(left right)) {
        my ( $out, $so, $ns ) = ShiftRegister( 0, 0, $st, $dir );
        is( $so + 0, 1, "$dir serial_out numerically 1, not 5" );
        ok( $so == 1 && !ref($so), 'serial_out canonical' );
        for my $q (@$out) { ok( $q == 0 || $q == 1, 'out canonical' ) }
        for my $h (@$ns) { for my $k ( keys %$h ) { ok( $h->{$k} == 0 || $h->{$k} == 1, "state $k canonical" ) } }
    }
    my ( $q, $qb ) = SRLatch( 0, 0, $dv, dualvar( 3, '0' ) );
    is( [ $q + 0, $qb + 0 ], [ 1, 0 ], 'SRLatch canonical' );
}

# --- @_ aliasing: the caller's variables are never written.
{
    my ( $a, $b ) = ( 1, 0 );
    AND( $a, $b ); ANDn( $a, $b ); SRLatch( $a, $b, 1, 0 );
    is( [ $a, $b ], [ 1, 0 ], 'arguments untouched' );
    my @in = ( 1, 0, 1 );
    ANDn( \@in ); ORn(@in); XORn( \@in );
    is( \@in, [ 1, 0, 1 ], 'input array untouched' );
}

# --- ShiftRegister state validation, no autovivification.
{
    my $good = sub { { master_q => 0, master_q_bar => 1, slave_q => 0, slave_q_bar => 1 } };
    my @bad = ( 'str', 5, [], \1, sub { 1 }, bless( {}, 'Foo' ),
                { master_q => 0 },
                { %{ $good->() }, extra => 1 },
                { %{ $good->() }, slave_q => undef },
                { %{ $good->() }, slave_q => 'x' } );
    for my $b (@bad) {
        my $state = [ $good->(), $b ];
        my $e = err_of { ShiftRegister( 1, 1, $state, 'left' ) };
        ok( blessed($e) && ref($e) eq $ERR, 'ShiftRegister bad element -> package Error' )
            or diag( "got: " . ( $e // 'undef' ) );
    }
    # undef element: rejected AND not autovivified.
    my $state = [ $good->(), undef ];
    my $e = err_of { ShiftRegister( 1, 1, $state, 'right' ) };
    is_err( $e, 'INVALID_SHAPE', 'undef element rejected' );
    is( scalar(@$state), 2, 'array length unchanged' );
    ok( !defined $state->[1], 'undef element NOT autovivified' );

    # missing key: not created.
    my $h = { master_q => 0, master_q_bar => 1, slave_q => 0 };
    err_of { ShiftRegister( 1, 1, [$h], 'left' ) };
    err_of { DFlipFlop( 1, 1, $h ) };
    is( [ sort keys %$h ], [qw(master_q master_q_bar slave_q)], 'missing key not created' );

    # a good state is not modified.
    my $s = [ $good->(), $good->() ];
    ShiftRegister( 1, 1, $s, 'left' );
    is( $s, [ $good->(), $good->() ], 'good state unchanged' );
}

# --- Register: undef state elements rejected; caller structures unchanged.
{
    is_err( err_of { Register( [1], 1, [undef] ) }, 'INVALID_SHAPE', 'Register undef state element' );
    my $st = [undef];
    err_of { Register( [1], 1, $st ) };
    ok( !defined $st->[0], 'Register: no autovivification' );
    is_err( err_of { Register( [1], 1, ['x'] ) }, 'INVALID_SHAPE', 'Register scalar state element' );
    my @r = Register( [1], 1 );
    is( scalar(@r), 2, 'Register without state still works' );
}

# --- caps.
{
    my $max = CodingAdventures::LogicGates::MAX_WIDTH();
    is( $max, 65536, 'MAX_WIDTH' );
    my @big = (1) x ( $max + 1 );
    is_err( err_of { ANDn(@big) }, 'INVALID_ARITY', 'ANDn list over cap' );
    is_err( err_of { ORn( \@big ) }, 'INVALID_ARITY', 'ORn array over cap' );
    is_err( err_of { XORn(@big) }, 'INVALID_ARITY', 'XORn over cap' );
    is_err( err_of { Register( \@big, 1 ) }, 'INVALID_WIDTH', 'Register over cap' );
    is_err( err_of { ShiftRegister( 1, 1, [ (undef) x ( $max + 1 ) ], 'left' ) }, 'INVALID_WIDTH', 'ShiftRegister over cap (before elements are read)' );
    my @ok = (1) x $max;
    is( ANDn( \@ok ), 1, 'exactly at cap works' );
}

# --- tied and overloaded inputs: no caller code runs.
{
    $calls = 0;
    tie my @ta, 'SpyArray';
    tie my %th, 'SpyHash';
    my $to = bless {}, 'SpyObj';
    $calls = 0;    # tie() itself may call nothing; reset anyway.
    my @es = (
        err_of { ANDn( \@ta ) },
        err_of { ORn( \@ta ) },
        err_of { XORn( \@ta ) },
        err_of { Register( \@ta, 1 ) },
        err_of { Register( [1], 1, \@ta ) },
        err_of { ShiftRegister( 1, 1, \@ta, 'left' ) },
        err_of { ShiftRegister( 1, 1, [ \%th ], 'left' ) },
        err_of { DFlipFlop( 1, 1, \%th ) },
        err_of { Counter( 1, 0, \%th ) },
        err_of { Counter( 1, 0, { bits => \@ta, width => 2 } ) },
        err_of { AND( $to, 1 ) },
        err_of { AND( 1, $to ) },
        err_of { ANDn( $to, 1 ) },
        err_of { NOT($to) },
        err_of { Register( [$to], 1 ) },
        err_of { DFlipFlop( 1, 1, { master_q => $to, master_q_bar => 0, slave_q => 0, slave_q_bar => 1 } ) },
    );
    is( $calls, 0, 'no tie method or overload ran' );
    for my $e (@es) { ok( blessed($e) && ref($e) eq $ERR, 'rejected with package Error' ) or diag("got $e") }
}

# --- the package Error even for odd invocants; blessed class is fixed.
{
    for my $inv ( 'Evil', bless( {}, 'Other' ), $ERR ) {
        my $e = $ERR->can('new')->( $inv, code => 'INVALID_BIT', message => 'x' );
        is( ref($e), $ERR, 'new blesses into the fixed package' );
    }
    my $e = $ERR->new( code => 'INVALID_CONTEXT', message => 'm' );
    is( $e->code, 'INVALID_CONTEXT', 'INVALID_CONTEXT is a known code' );
}

# --- scalar context is rejected for list-returning blocks.
{
    my $fs = new_flip_flop_state();
    my $cs = new_counter_state(2);
    my @calls = (
        sub { scalar SRLatch( 1, 0, 0, 1 ) },
        sub { scalar DLatch( 1, 1, 0, 1 ) },
        sub { scalar DFlipFlop( 1, 1, $fs ) },
        sub { scalar Register( [1], 1 ) },
        sub { scalar ShiftRegister( 1, 1, [$fs], 'left' ) },
        sub { scalar Counter( 1, 0, $cs ) },
    );
    for my $c (@calls) { is_err( err_of { $c->() }, 'INVALID_CONTEXT', 'scalar context rejected' ) }
    my ($q) = SRLatch( 1, 0, 0, 1 );
    is( $q, 1, 'list context fine' );
    SRLatch( 1, 0, 0, 1 );    # void context fine
    ok( 1, 'void context fine' );
}

# --- Counter state.
{
    is_err( err_of { Counter( 1, 0, { bits => [ 0, 0 ], width => 2, extra => 1 } ) }, 'INVALID_SHAPE', 'Counter extra key' );
    is_err( err_of { Counter( 1, 0, { bits => [ 0, 0 ] } ) }, 'INVALID_SHAPE', 'Counter missing width' );
    is_err( err_of { Counter( 1, 1, { bits => [ 0, 'x' ], width => 2 } ) }, 'INVALID_BIT', 'Counter reset still validates bits' );
    my $st = { bits => [ 1, 1 ], width => 2 };
    my ( $bits, $ns ) = Counter( 1, 0, $st );
    is( $bits, [ 0, 0 ], 'wraps' );
    is( $st, { bits => [ 1, 1 ], width => 2 }, 'input state unchanged' );
    ok( Scalar::Util::refaddr($bits) != Scalar::Util::refaddr( $ns->{bits} ), 'output and state do not share storage' );
}

# --- tied SCALARS: never read (FETCH) at any entry point.
{
    my $ok_state = sub { { master_q => 0, master_q_bar => 1, slave_q => 0, slave_q_bar => 1 } };
    tie my $ts, 'SpyScalar';
    my @a2 = ( [ 'a first', sub { AND( $ts, 1 ) } ], [ 'a second', sub { AND( 1, $ts ) } ],
               [ 'extra arity', sub { AND( 1, 1, $ts ) } ], [ 'NOT', sub { NOT($ts) } ],
               [ 'ANDn arg', sub { ANDn( 1, $ts ) } ], [ 'ANDn single', sub { ANDn($ts) } ],
               [ 'XORn single', sub { XORn($ts) } ],
               [ 'SRLatch', sub { my @x = SRLatch( 1, 0, 0, $ts ) } ],
               [ 'new_counter_state', sub { new_counter_state($ts) } ],
               [ 'new_ff_state', sub { new_flip_flop_state( 0, 1, 0, $ts ) } ],
               [ 'ANDn array elem', sub { my @x = ( 1, 1 ); tie $x[1], 'SpyScalar'; ANDn( \@x ) } ],
               [ 'Register data elem', sub { my @x = (1); tie $x[0], 'SpyScalar'; my @r = Register( \@x, 1 ) } ],
               [ 'Register state elem', sub { my @x = ( $ok_state->() ); tie $x[0], 'SpyScalar'; my @r = Register( [1], 1, \@x ) } ],
               [ 'ShiftRegister elem', sub { my @x = ( $ok_state->() ); tie $x[0], 'SpyScalar'; my @r = ShiftRegister( 1, 1, \@x, 'left' ) } ],
               [ 'DFlipFlop hash value', sub { my $h = $ok_state->(); tie $h->{slave_q}, 'SpyScalar'; my @r = DFlipFlop( 1, 1, $h ) } ],
               [ 'ShiftRegister hash value', sub { my $h = $ok_state->(); tie $h->{master_q}, 'SpyScalar'; my @r = ShiftRegister( 1, 1, [$h], 'right' ) } ],
               [ 'Counter width', sub { my $h = { bits => [ 0, 0 ], width => 2 }; tie $h->{width}, 'SpyScalar'; my @r = Counter( 1, 0, $h ) } ],
               [ 'Counter bits', sub { my $h = { bits => [ 0, 0 ], width => 2 }; tie $h->{bits}, 'SpyScalar'; my @r = Counter( 1, 0, $h ) } ],
               [ 'Counter bit elem', sub { my $b = [ 0, 0 ]; tie $b->[1], 'SpyScalar'; my @r = Counter( 1, 0, { bits => $b, width => 2 } ) } ],
               [ 'Counter clock', sub { my @r = Counter( $ts, 0, { bits => [ 0, 0 ], width => 2 } ) } ],
    );
    for my $c (@a2) {
        $calls = 0;
        my $e = err_of { $c->[1]->() };
        is( $calls, 0, "$c->[0]: tied scalar never read" );
        ok( blessed($e) && ref($e) eq $ERR, "$c->[0]: package Error" ) or diag("got: " . ( $e // 'undef' ));
    }
}

# --- TOCTOU: state values are read once / tied values rejected before reading.
{
    my $reads = 0;
    {
        package FlipBits; sub TIESCALAR { bless {}, shift } sub FETCH { $reads++; $reads == 1 ? [ 0, 0 ] : bless( {}, 'Boom' ) }
        package Boom; use overload '@{}' => sub { $reads += 100; [ 1, 1 ] }, fallback => 1;
    }
    my $h = { width => 2 };
    tie $h->{bits}, 'FlipBits';
    my $e = err_of { my @r = Counter( 1, 0, $h ) };
    ok( blessed($e) && ref($e) eq $ERR, 'tied bits rejected' );
    is( $reads, 0, 'no read happened' );
    my $o = { width => 2, bits => bless( {}, 'Boom' ) };
    $reads = 0;
    is_err( err_of { my @r = Counter( 1, 0, $o ) }, 'INVALID_SHAPE', 'overloaded @{} bits rejected' );
    is( $reads, 0, 'overload @{} never invoked' );
}

# --- Error strictness.
{
    my $n = 0;
    { package StrObj; use overload '""' => sub { $n++; 'INVALID_BIT' }, fallback => 1; }
    my $o = bless {}, 'StrObj';
    for my $args ( [ code => $o ], [ code => 'INVALID_BIT', message => $o ], [ code => 'INVALID_BIT', param => $o ],
                   [ code => 'INVALID_BIT', value_description => [] ], [ code => 'NOPE' ], [ code => undef ] ) {
        my $e = err_of { $ERR->new(@$args) };
        ok( blessed($e) && ref($e) eq $ERR, 'bad Error fields die with package Error' );
    }
    is( $n, 0, 'overloaded stringification never ran' );
    ok( !CodingAdventures::LogicGates::is_invalid_bit( bless [], $ERR ), 'array-based forged Error is not invalid_bit (no die)' );
    ok( !CodingAdventures::LogicGates::is_invalid_bit( bless \my $s, $ERR ), 'scalar-based forged Error' );
}

# --- Counter outputs never alias the returned state.
{
    my $cs = new_counter_state(3);
    for my $args ( [ 1, 0 ], [ 1, 1 ], [ 0, 0 ] ) {
        my ( $bits, $ns ) = Counter( @$args, $cs );
        ok( Scalar::Util::refaddr($bits) != Scalar::Util::refaddr( $ns->{bits} ), "Counter(@$args): fresh arrays" );
        push @$bits, 9;
        is( scalar @{ $ns->{bits} }, 3, 'mutating output does not touch state' );
    }
}

# --- is_invalid_bit never reads tied or object values.
{
    my $disp = 0;
    { package EqObj; use overload 'eq' => sub { $disp++; 1 }, '""' => sub { $disp++; 'INVALID_BIT' }, fallback => 1; }
    tie my $ts, 'SpyScalar';
    $calls = 0;
    ok( !CodingAdventures::LogicGates::is_invalid_bit($ts), 'tied scalar argument -> 0' );
    is( $calls, 0, 'tied scalar argument never read' );
    my $h = bless { code => 'INVALID_BIT' }, $ERR;
    tie $h->{code}, 'SpyScalar';
    $calls = 0;
    ok( !CodingAdventures::LogicGates::is_invalid_bit($h), 'tied code field -> 0' );
    is( $calls, 0, 'tied code field never read' );
    my $o = bless { code => bless( {}, 'EqObj' ) }, $ERR;
    ok( !CodingAdventures::LogicGates::is_invalid_bit($o), 'object code field -> 0' );
    is( $disp, 0, 'overloaded eq/"" never dispatched' );
    my $nc = bless {}, $ERR;
    ok( !CodingAdventures::LogicGates::is_invalid_bit($nc), 'missing code -> 0' );
    ok( !exists $nc->{code}, 'no autovivification of code' );
    ok( CodingAdventures::LogicGates::is_invalid_bit( $ERR->new( code => 'INVALID_BIT', message => 'm' ) ), 'genuine error still true' );
}

done_testing;
