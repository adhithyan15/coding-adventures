use strict;
use warnings;
use Test2::V0;
use CodingAdventures::Transistors qw(
    nmos pmos npn pnp
    cmos_not cmos_nand cmos_nor cmos_and cmos_or cmos_xor cmos_xnor
    ttl_not ttl_nand is_invalid_bit
);

my $E = 'CodingAdventures::Transistors::Error';

sub err_of(&) { my $c = shift; local $@; eval { $c->(); 1 } ? undef : $@ }

# Assert class, code, param and predicate for one rejection.
sub chk {
    my ( $err, $code, $param, $label ) = @_;
    is( ref($err), $E, "$label: class" );
    is( $err->code,  $code,  "$label: code $code" );
    is( $err->param, $param, "$label: param" );
    is( is_invalid_bit($err) ? 1 : 0, $code eq 'INVALID_BIT' ? 1 : 0, "$label: is_invalid_bit" );
}

my @bad = ( '1abc', 'a', '', undef, '1.0', "1\n", '01', [1], {}, 2 );

# --- bit matrix over every digital function and argument position ---------
my %two = ( cmos_nand => \&cmos_nand, cmos_nor => \&cmos_nor, cmos_and => \&cmos_and,
            cmos_or => \&cmos_or, cmos_xor => \&cmos_xor, cmos_xnor => \&cmos_xnor,
            ttl_nand => \&ttl_nand );
my %one = ( cmos_not => \&cmos_not, ttl_not => \&ttl_not );
for my $n ( sort keys %two ) {
    for my $i ( 0 .. $#bad ) {
        my $v = $bad[$i];
        chk( err_of { $two{$n}->( $v, 1 ) }, 'INVALID_BIT', 'a', "$n a #$i" );
        chk( err_of { $two{$n}->( 1, $v ) }, 'INVALID_BIT', 'b', "$n b #$i" );
    }
    chk( err_of { $two{$n}->(1) },         'INVALID_ARITY', undef, "$n 1 arg" );
    chk( err_of { $two{$n}->( 1, 1, 1 ) }, 'INVALID_ARITY', undef, "$n 3 args" );
}
for my $n ( sort keys %one ) {
    for my $i ( 0 .. $#bad ) {
        chk( err_of { $one{$n}->( $bad[$i] ) }, 'INVALID_BIT', 'a', "$n #$i" );
    }
    chk( err_of { $one{$n}->() },       'INVALID_ARITY', undef, "$n 0 args" );
    chk( err_of { $one{$n}->( 1, 1 ) }, 'INVALID_ARITY', undef, "$n 2 args" );
}

# --- analog matrix over nmos pmos npn pnp ---------------------------------
my @badnum = ( 'x', '', undef, [1], '1abc', ' 1', '0x10', 'nan', 'inf', '1e999', '-1e999' );
my %analog = ( nmos => [ \&nmos, 'vgs', 'vth' ], pmos => [ \&pmos, 'vgs', 'vth' ],
               npn  => [ \&npn, 'ib', 'beta' ],  pnp  => [ \&pnp, 'ib', 'beta' ] );
for my $n ( sort keys %analog ) {
    my ( $f, $p1, $p2 ) = @{ $analog{$n} };
    for my $i ( 0 .. $#badnum ) {
        my $v = $badnum[$i];
        chk( err_of { $f->($v) }, 'INVALID_NUMBER', $p1, "$n $p1 #$i" );
        next unless defined $v;    # explicit undef optional arg means "default"
        chk( err_of { $f->( 1, $v ) }, 'INVALID_NUMBER', $p2, "$n $p2 #$i" );
    }
    chk( err_of { $f->() },          'INVALID_ARITY', undef, "$n 0 args" );
    chk( err_of { $f->( 1, 1, 1 ) }, 'INVALID_ARITY', undef, "$n 3 args" );
}

# --- messages and bounded description -------------------------------------
{
    my $e = err_of { cmos_and( 'x', 1 ) };
    is( "$e", "Transistors: a must be 0 or 1\n", 'stringification is the old message' );
    is( $e->message, "Transistors: a must be 0 or 1\n", 'message accessor' );
    is( $e->value_description, "'x'", 'value_description' );
    my $big = err_of { cmos_not( 'z' x 1_000_000 ) };
    ok( length( $big->value_description ) < 64, 'value_description bounded' );
    is( err_of { cmos_not(undef) }->value_description, 'undef', 'undef described' );
    is( err_of { cmos_not( [] ) }->value_description, 'a ARRAY reference', 'ref described' );
    is( err_of { cmos_not("\x01") }->value_description, "'\\x01'", 'control chars escaped' );
}

# --- overload behaviour ----------------------------------------------------
{
    my $e = err_of { cmos_not('x') };
    ok( $e ? 1 : 0, 'bool true' );
    ok( $e eq "Transistors: a must be 0 or 1\n", 'eq' );
    ok( $e ne 'other', 'ne' );
    is( "failed: $e", "failed: Transistors: a must be 0 or 1\n", 'interpolation' );
    is( $e . 'y', "Transistors: a must be 0 or 1\ny", 'concat right' );
    is( 'y' . $e, "yTransistors: a must be 0 or 1\n", 'concat left' );
    is( ( $e x 2 ), "Transistors: a must be 0 or 1\n" x 2, 'repetition' );
    is( ( $e cmp "$e" ), 0, 'cmp' );
    is( ( 'A' cmp $e ), -1, 'cmp swapped' );
    like( $e, qr/must be 0 or 1/, 'like on object' );

    # `!`, `.=`, ordering comparisons, swapped x.
    ok( !( !$e ), '! is false for an error' );
    my $ok = !$e;
    ok( !$ok, 'my $ok = !$e' );
    my $took = 0;
    if ( !$e ) { $took = 1 }
    is( $took, 0, 'if (!$err) not taken' );
    my $acc = $e;
    $acc .= 'z';
    is( ref( \$acc ), 'SCALAR', '.= yields a plain string' );
    is( $acc, "Transistors: a must be 0 or 1\nz", '.= text' );
    ok( $e lt 'zzz', 'lt' );
    ok( $e le "$e",  'le' );
    ok( $e gt 'A',   'gt' );
    ok( $e ge "$e",  'ge' );
    ok( 'zzz' gt $e, 'swapped gt' );
    ok( 'A' lt $e,   'swapped lt' );
    ok( !( $e gt 'zzz' ), 'gt false' );
    ok( dies { my $x = 'a' x $e }, 'swapped x dies' );
    ok( dies { my $x = $e <=> 1 }, '<=> fails loudly' );
    ok( dies { my $x = $e * 2 },   '* fails loudly' );

    my @w;
    { local $SIG{__WARN__} = sub { push @w, @_ }; warn $e; }
    like( $w[0], qr/\ATransistors: a must be 0 or 1/, 'warn prints message' );

    # die wrapping: the original error survives and keeps its code.
    my $wrapped = err_of { eval { cmos_not('x'); 1 } or die $@ };
    is( $wrapped->code, 'INVALID_BIT', 'rethrow keeps object' );
    my $str = err_of { eval { cmos_not('x'); 1 } or die "ctx: " . $@ };
    like( $str, qr/\Actx: Transistors: a must be/, 'die with concatenation works' );

    ok( dies { my $x = $e == 1 }, '== fails loudly' );
    ok( dies { my $x = $e + 1 },  '+ fails loudly' );
}

# --- predicate exactness ---------------------------------------------------
{
    package Sub::Err;
    our @ISA = ('CodingAdventures::Transistors::Error');
}
{
    ok( !is_invalid_bit(undef), 'undef' );
    ok( !is_invalid_bit('INVALID_BIT'), 'plain string' );
    ok( !is_invalid_bit( { code => 'INVALID_BIT' } ), 'forged hashref' );
    ok( !is_invalid_bit( bless { code => 'INVALID_BIT' }, 'Other' ), 'foreign class' );
    my $sub = Sub::Err->new( code => 'INVALID_BIT', message => 'x' );
    ok( !is_invalid_bit($sub), 'subclass rejected' );
    ok( is_invalid_bit( $E->new( code => 'INVALID_BIT', message => 'x' ) ), 'genuine' );
    ok( !is_invalid_bit( $E->new( code => 'INVALID_ARITY', message => 'x' ) ), 'other code' );
    like( dies { $E->new( code => 'NOPE', message => 'x' ) }, qr/unknown code 'NOPE'/, 'unknown code' );
    like( dies { $E->new( message => 'x' ) }, qr/unknown code 'undef'/, 'missing code' );
    is( $E->new( code => 'INVALID_BIT', message => 'no newline' )->message, "no newline\n", 'newline added' );
}

done_testing;
