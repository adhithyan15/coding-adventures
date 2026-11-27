# The fuzz mutator algebra (VER06q section 3): the Go lane's 820 oracle vectors (parsed from
# the committed Go source, no copy), the Ruby-derived pins, the interest lists, the caps and
# the argument checks.  Every test is bounded.
use strict;
use warnings;
use Test::More;
use Math::BigInt;
use FindBin;
use lib "$FindBin::Bin/lib";
use TBHelper qw(repo_root read_file bounded_by);
use FuzzPins;
use CodingAdventures::TestBootstrap;

my $M   = 'CodingAdventures::TestBootstrap::Fuzz::Mutate';
my $Int = 'CodingAdventures::TestBootstrap::Int';
sub I   { return CodingAdventures::TestBootstrap::Int::parse( $_[0] ) }
sub unhex { return pack 'H*', $_[0] }
sub hex_of { return unpack 'H*', $_[0] }

sub run_op {
    my ( $op, $a ) = @_;
    no strict 'refs';
    if    ( $op eq 'bit_flip' )        { return CodingAdventures::TestBootstrap::Fuzz::Mutate::bit_flip( unhex( $a->[0] ), I( $a->[1] ), I( $a->[2] ) ) }
    elsif ( $op eq 'byte_flip' )       { return CodingAdventures::TestBootstrap::Fuzz::Mutate::byte_flip( unhex( $a->[0] ), I( $a->[1] ), I( $a->[2] ) ) }
    elsif ( $op eq 'arithmetic' )      { return CodingAdventures::TestBootstrap::Fuzz::Mutate::byte_arithmetic( unhex( $a->[0] ), I( $a->[1] ), I( $a->[2] ), I( $a->[3] ), $a->[4] ) }
    elsif ( $op eq 'boundary' )        { return CodingAdventures::TestBootstrap::Fuzz::Mutate::boundary_insert( unhex( $a->[0] ), I( $a->[1] ), I( $a->[2] ), I( $a->[3] ), $a->[4] ) }
    elsif ( $op eq 'block_delete' )    { return CodingAdventures::TestBootstrap::Fuzz::Mutate::block_delete( unhex( $a->[0] ), I( $a->[1] ), I( $a->[2] ) ) }
    elsif ( $op eq 'block_insert' )    { return CodingAdventures::TestBootstrap::Fuzz::Mutate::block_insert( unhex( $a->[0] ), I( $a->[1] ), unhex( $a->[2] ) ) }
    elsif ( $op eq 'block_duplicate' ) { return CodingAdventures::TestBootstrap::Fuzz::Mutate::block_duplicate( unhex( $a->[0] ), I( $a->[1] ), I( $a->[2] ), I( $a->[3] ) ) }
    elsif ( $op eq 'splice' )          { return CodingAdventures::TestBootstrap::Fuzz::Mutate::splice_bytes( unhex( $a->[0] ), unhex( $a->[1] ), I( $a->[2] ), I( $a->[3] ) ) }
    elsif ( $op eq 'sequence' )        { return CodingAdventures::TestBootstrap::Fuzz::Mutate::mutate_sequence( unhex( $a->[0] ), I( $a->[1] ), I( $a->[2] ), I( $a->[3] ) ) }
    die "unknown op $op\n";
}

subtest 'the Go lane reference vectors (parsed from the committed Go source)' => sub {
    my $path = repo_root() . '/code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go';
    my ( @rows, $started );
    for my $line ( split /\n/, read_file($path) ) {
        if ($started) {
            last if $line eq '}';
            $line =~ /^\t\{"([a-z_]+)", \[\]string\{(.*)\}, "([0-9a-f]*)"\},$/ or BAIL_OUT("unparseable vector row: $line");
            my ( $op, $args, $out ) = ( $1, $2, $3 );
            push @rows, [ $op, [ $args =~ /"([^"]*)"/g ], $out ];
        }
        elsif ( $line eq 'var referenceVectors = []refVector{' ) { $started = 1 }
    }
    ok( $started, 'the vector table is where the test expects it' );
    is( scalar @rows, 820, '820 vectors' );
    my ( %count, @bad );
    for my $i ( 0 .. $#rows ) {
        my ( $op, $a, $want ) = @{ $rows[$i] };
        $count{$op}++;
        my $got = eval { bounded_by( 20, sub { run_op( $op, $a ) } ) };
        if ( !defined $got || hex_of($got) ne $want ) {
            push @bad, sprintf( 'vector %d %s %s: want %s got %s', $i + 1, $op, join( ',', @$a ), $want, defined $got ? hex_of($got) : 'error' );
            last if @bad >= 5;
        }
    }
    is_deeply( \@bad, [], 'all 820 vectors match' );
    for my $op (qw(bit_flip byte_flip arithmetic boundary block_delete block_insert block_duplicate splice sequence)) {
        cmp_ok( $count{$op} || 0, '>=', 50, "at least 50 vectors for $op" );
    }
    diag("vectors: " . scalar(@rows) . "/" . scalar(@rows)) if !@bad;
};

subtest 'the Ruby-derived mutator pins' => sub {
    my ( @bad, %count );
    for my $i ( 0 .. $#FuzzPins::MUTATE ) {
        my ( $op, $a, $want ) = @{ $FuzzPins::MUTATE[$i] };
        $count{$op}++;
        my $got = eval { run_op( $op, $a ) };
        push @bad, sprintf( 'row %d %s %s: want %s got %s', $i + 1, $op, join( ',', @$a ), $want, defined $got ? hex_of($got) : 'error' )
          if !defined $got || hex_of($got) ne $want;
    }
    is_deeply( \@bad, [], 'all ' . scalar(@FuzzPins::MUTATE) . ' rows agree' );
    ok( $count{$_}, "a pin for $_" ) for qw(splice boundary arithmetic bit_flip byte_flip block_delete block_insert block_duplicate sequence);
};

subtest 'the interest lists and deltas are exactly the specification\'s' => sub {
    is_deeply( \@CodingAdventures::TestBootstrap::Fuzz::Mutate::INTEREST_8,  [ 0, 1, 127, 128, 255 ] );
    is_deeply( \@CodingAdventures::TestBootstrap::Fuzz::Mutate::INTEREST_16, [ 0, 1, 255, 256, 32767, 32768, 65535 ] );
    is_deeply( \@CodingAdventures::TestBootstrap::Fuzz::Mutate::INTEREST_32, [ 0, 1, 65535, 65536, 2147483647, 2147483648, 4294967295 ] );
    is_deeply( [ map { CodingAdventures::TestBootstrap::Int::tostring($_) } @CodingAdventures::TestBootstrap::Fuzz::Mutate::INTEREST_64 ],
        [qw(0 1 4294967295 4294967296 9223372036854775807 9223372036854775808 18446744073709551615)] );
    is_deeply( \@CodingAdventures::TestBootstrap::Fuzz::Mutate::DELTAS, [ -35, -8, -1, 1, 8, 35 ] );
};

subtest 'cheap flips: a 1 MiB buffer with a 1,000-digit count builds no bignum from bytes' => sub {
    my $big = I( '9' x 999 . '7' );
    my $buf = "\0" x ( 1 << 20 );
    my $t0  = time;
    my $out = bounded_by( 20, sub { CodingAdventures::TestBootstrap::Fuzz::Mutate::bit_flip( $buf, 5, $big ) } );
    is( length $out, 1 << 20, 'length preserved' );
    my $odd = bounded_by( 20, sub { CodingAdventures::TestBootstrap::Fuzz::Mutate::byte_flip( $buf, 5, I( '1' . '0' x 999 ) ) } );
    is( length $odd, 1 << 20, 'a huge even count: parity only' );
    is( $odd, $buf, 'an even number of full passes plus a zero remainder is a no-op' );
    my $inv = CodingAdventures::TestBootstrap::Fuzz::Mutate::byte_flip( $buf, 0, 1 << 20 );
    is( $inv, "\xff" x ( 1 << 20 ), 'a count of one pass inverts everything' );
    cmp_ok( time - $t0, '<', 15, 'cheap' );
    # work is bounded by the buffer, not by the count
    is( hex_of( CodingAdventures::TestBootstrap::Fuzz::Mutate::bit_flip( "\0\0", 0, 17 ) ), 'feff', 'bit 0 twice (even), bits 1..15 once' );
    is( hex_of( CodingAdventures::TestBootstrap::Fuzz::Mutate::bit_flip( "\0\0", 0, 0 ) ), '0000', 'a zero count is a no-op' );
    is( hex_of( CodingAdventures::TestBootstrap::Fuzz::Mutate::bit_flip( "\0\0", 0, -3 ) ), '0000', 'a negative count is a no-op' );
    is( hex_of( CodingAdventures::TestBootstrap::Fuzz::Mutate::bit_flip( "\0\0", 15, 3 ) ), '0380', 'a window that wraps' );
    is( CodingAdventures::TestBootstrap::Fuzz::Mutate::bit_flip( '', 3, 1 ), '', 'an empty buffer stays empty' );
};

subtest 'argument checks' => sub {
    my $bf = \&CodingAdventures::TestBootstrap::Fuzz::Mutate::bit_flip;
    ok( !eval { $bf->( undef, 0, 1 ); 1 }, 'undef data' );
    ok( !eval { $bf->( [], 0, 1 ); 1 }, 'a reference as data' );
    ok( !eval { $bf->( 'ab', 1.5, 1 ); 1 }, 'a float offset' );
    ok( !eval { $bf->( 'ab', '1', 1 ); 1 }, 'a numeric string offset' );
    is( hex_of( $bf->( "\0", 0, undef ) ), '01', 'undef count is 1' );
    ok( !eval { CodingAdventures::TestBootstrap::Fuzz::Mutate::byte_arithmetic( 'ab', 0, 1, 1, 'middle' ); 1 }, 'a bad endian' );
    is( CodingAdventures::TestBootstrap::Fuzz::Mutate::byte_arithmetic( 'abc', 0, 1, 3, 'little' ), 'abc', 'width 3 is unsupported: the input' );
    is( CodingAdventures::TestBootstrap::Fuzz::Mutate::byte_arithmetic( 'a', 0, 1, 2, 'little' ), 'a', 'a buffer shorter than the width: the input' );
    is( CodingAdventures::TestBootstrap::Fuzz::Mutate::boundary_insert( 'abcd', 0, I('18446744073709551616'), 0, 'little' ), 'abcd', 'a big width: the input' );
    is( CodingAdventures::TestBootstrap::Fuzz::Mutate::block_delete( '', 0, 3 ), '', 'delete from nothing' );
    is( CodingAdventures::TestBootstrap::Fuzz::Mutate::block_insert( '', 5, 'zz' ), 'zz', 'insert into nothing' );
    is( CodingAdventures::TestBootstrap::Fuzz::Mutate::block_duplicate( 'ab', 0, 0, 0 ), 'ab', 'a zero-length duplicate' );
    is( CodingAdventures::TestBootstrap::Fuzz::Mutate::splice_bytes( 'abcd', 'wxyz', I('99999999999999999999999'), I('-99999999999999999999999') ), 'abcdwxyz', 'huge splits clamp' );
    # utf8-flagged strings are reduced to bytes
    my $wide = "\x{263a}";
    is( hex_of( CodingAdventures::TestBootstrap::Fuzz::Mutate::block_insert( $wide, 0, '' ) ), 'e298ba', 'a wide character is its UTF-8 octets' );
    is( hex_of( CodingAdventures::TestBootstrap::Fuzz::Mutate::block_insert( "\xe9", 0, '' ) ), 'e9', 'a latin-1 character is one byte' );
    my $num = "12";
    my $x = $num + 1;    # the scalar now has a numeric flag too
    is( hex_of( CodingAdventures::TestBootstrap::Fuzz::Mutate::byte_flip( $num, 0, 2 ) ), 'cecd', 'a numeric-looking string is still XORed as bytes' );
};

subtest 'mutate_sequence caps (CWE-400) are checked before any allocation' => sub {
    my $S = \&CodingAdventures::TestBootstrap::Fuzz::Mutate::mutate_sequence;
    ok( !eval { $S->( "\0", 1, 1_000_001, 10 ); 1 }, 'steps above 1,000,000 are refused' );
    ok( !eval { $S->( "\0" x ( ( 1 << 20 ) + 1 ), 1, 1, 10 ); 1 }, 'an input above 1 MiB is refused' );
    my $C = \&CodingAdventures::TestBootstrap::Fuzz::Mutate::mutate_sequence_counted;
    my ( $out, $work ) = $C->( "\0\0\0\0", 3, 5, 100, 1 << 28 );
    ok( defined $out, 'a small run completes' );
    cmp_ok( $work, '>=', 4 * 6, 'work charges the initial copy and a buffer per step' );
    ( $out, $work ) = $C->( "\0" x 100, 3, 1000, 100, 1000 );
    ok( !defined $out, 'a failed call returns undef ...' );
    cmp_ok( $work, '<=', 1000, '... charged what it did' );
    cmp_ok( $work, '>', 0, '... which is not nothing' );
    ( $out, $work ) = $C->( "\0" x 100, 3, 1, 100, 50 );
    ok( !defined $out && $work == 0, 'an input over the cap is refused at once, charged 0' );
    is( hex_of( $S->( "\0" x 8, 42, 4 ) ), '03ff00000000bf080000', 'seed 42, 4 steps (Ruby lane)' );
    is( hex_of( $S->( "\0" x 8, 42, 4, 8 ) ) , hex_of( $S->( "\0" x 8, 42, 4, 8 ) ), 'deterministic' );
    is( $S->( "\0", 5, 0, 10 ), "\0", 'zero steps return the input' );
    like( hex_of( $S->( '', 5, 1, 10 ) ), qr/^[0-9a-f]+$/, 'an empty buffer is refilled' );
};

done_testing;
