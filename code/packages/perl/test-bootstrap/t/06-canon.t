# Hazards 4 and 5: hostile values (cycles, depth, size) and strings as bytes, in
# Canon and the CI digest (VER05p section 4).
use strict;
use warnings;
use Test::More;
use Math::BigInt;
use FindBin;
use lib "$FindBin::Bin/lib";
use TBHelper qw(dec int_of bounded_by);

my $C = 'CodingAdventures::TestBootstrap::Canon';
my $R = 'CodingAdventures::TestBootstrap::Runner';
my $G = 'CodingAdventures::TestBootstrap::Gens';
my $Opts = 'CodingAdventures::TestBootstrap::Runner::Options';

sub render { my $v = $_[0]; return bounded_by( 30, sub { $C->can('render')->($v) } ) }
sub utf8b { my $s = shift; utf8::encode($s); return $s }

subtest 'scalars' => sub {
    is( render(5), '5', 'integer' );
    is( render(-5), '-5', 'negative integer' );
    is( render(18446744073709551615), '18446744073709551615', 'a UV above IV_MAX' );
    is( render(undef), 'undef', 'undef' );
    is( render(3.0), '3.0', 'an integral float keeps its .0' );
    is( render(0.5), '0.5', 'a float' );
    is( render(-0.25), '-0.25', 'a negative float' );
    is( render(0.1), '0.1', 'shortest round trip' );
    is( render(1e100), '1e+100', 'an exponent' );
    is( render( 9**9**9 ), 'inf', 'inf' );
    is( render( -9**9**9 ), '-inf', '-inf' );
    is( render( ( 9**9**9 ) / ( 9**9**9 ) ), 'nan', 'nan' );
    isnt( render(3), render(3.0), '3 and 3.0 render differently' );
    my $f = 3.0;
    is( render($f), '3.0', 'a variable holding a float' );
    my $n = 7;
    my $s = "$n";
    is( render($n), '7', 'an integer that has been stringified is still an integer' );
    is( render("007"), '"007"', 'a numeric-looking string stays a string' );
    is( render("12"), '"12"', 'a string of digits is a string' );
    is( render(''), '""', 'the empty string' );
    is( render( Math::BigInt->new('123456789012345678901234567890') ), '123456789012345678901234567890', 'a big' );
    like( render( $C->can('text')->('hi') ), qr/^"hi"$/, 'text' );
    like( render( Math::BigInt->new(2)->blsft(200000) ), qr/^<bigint 200002 bits>$/, 'a huge big is summarized' );
    is( render( \"s" ), '\\"s"', 'a scalar reference' );
    is( render( \\5 ), '\\\\5', 'a reference to a reference' );
    is( render( sub { 1 } ), '#Code', 'code' );
    is( render( \*STDOUT ), '#Glob', 'a glob' );
    is( render( qr/x/ ), '#Regexp', 'a regexp' );
    is( render( bless( [ 1, 2 ], 'My::Pkg' ) ), 'My::Pkg=[1,2]', 'a blessed array' );
    is( render( bless( {}, 'My::Pkg' ) ), 'My::Pkg=%{}', 'a blessed hash' );
    is( render( [] ), '[]', 'an empty array' );
    is( render( [ 1, [ 2, 3 ], 'x' ] ), '[1,[2,3],"x"]', 'nested arrays' );
};

subtest 'hash order never reaches the output' => sub {
    my %h = map { ( "k$_" => $_ ) } 1 .. 40;
    my $a = render( \%h );
    my %g = map { ( "k$_" => $_ ) } reverse 1 .. 40;
    is( render( \%g ), $a, 'insertion order is irrelevant' );
    is( render( { b => 1, a => 2 } ), '%{"a"=>2,"b"=>1}', 'entries are sorted by bytes' );
    is( render( { "\xE9" => 1, "z" => 2, "A" => 3 } ), '%{"A"=>3,"z"=>2,0xe9=>1}' =~ s/0xe9/0xe9/r, 'bytes sort unsigned (0xe9 after z)' );
};

subtest 'the escape set is the Elixir lane\'s, exactly' => sub {
    my @escaped = ( 0 .. 0x1F, 0x7F .. 0x9F, 0x2028, 0x2029, 0x061C, 0x200E, 0x200F, 0x202A .. 0x202E, 0x2066 .. 0x2069, 0xAD, 0x34F, 0x180E, 0xFEFF, 0x200B .. 0x200D, 0x2060 .. 0x2064, 0xFFF9 .. 0xFFFB, 0xE0000 .. 0xE007F );
    my %esc = map { $_ => 1 } @escaped;
    my $bad = 0;
    for my $cp (@escaped) {
        my $r = render( utf8b( chr $cp ) );
        my $want = $cp == 10 ? '"\\n"' : $cp == 13 ? '"\\r"' : $cp == 9 ? '"\\t"' : $cp > 0xFFFF ? sprintf( '"\\u{%x}"', $cp ) : sprintf( '"\\u%04x"', $cp );
        if ( $r ne $want ) { $bad++; diag sprintf( 'U+%04X rendered %s wanted %s', $cp, $r, $want ) }
    }
    is( $bad, 0, scalar(@escaped) . ' code points are escaped' );
    my @plain = ( 0x20, 0x21, 0x41, 0x7E, 0xA0, 0xA1, 0xAC, 0xAE, 0x34E, 0x350, 0x180D, 0x180F, 0x2027, 0x202F, 0x2065, 0x206A, 0xFEFE, 0xFF00, 0xFFF8, 0xFFFC, 0xE00FF, 0x1F600, 0x10FFFF );
    $bad = 0;
    for my $cp (@plain) {
        next if $esc{$cp};
        my $s = utf8b( chr $cp );
        my $r = render($s);
        my $want = '"' . ( $cp == 0x22 ? '\\"' : $s ) . '"';
        if ( $r ne $want ) { $bad++; diag sprintf( 'U+%04X rendered %s', $cp, $r ) }
    }
    is( $bad, 0, 'neighbours of the escaped ranges pass through' );
    is( render( utf8b("a\"b\\c") ), '"a\\"b\\\\c"', 'quote and backslash are escaped inside quotes' );
    is( render( utf8b("caf\x{e9}") ), '"café"' =~ s/é/\xC3\xA9/r, 'ordinary non-ASCII text is kept as UTF-8 bytes' );
    is( render( [ "a\nb" ] ), '["a\\nb"]', 'a newline cannot forge a log line' );
    is( $C->can('escape')->( utf8b("a\"\x{2028}"), 0 ), "a\"\\u2028", 'unquoted escape leaves quotes alone' );
};

subtest 'strings are bytes: invalid UTF-8 is hex, the utf8 flag is irrelevant' => sub {
    is( render("\xFF\xFE"), '0xfffe', 'invalid UTF-8 is hex' );
    is( render("\xC0\x80"), '0xc080', 'an overlong NUL is invalid' );
    is( render("\xED\xA0\x80"), '0xeda080', 'an encoded surrogate is invalid' );
    is( render("\xF4\x90\x80\x80"), '0xf4908080', 'above U+10FFFF is invalid' );
    is( render("\xC3"), '0xc3', 'a truncated sequence is invalid' );
    is( render("\0"), '"\\u0000"', 'NUL is a control' );
    my $flagged = "\x{e9}";
    utf8::upgrade($flagged);
    ok( utf8::is_utf8($flagged), 'the scalar is utf8-flagged' );
    is( render($flagged), render("\xE9"), 'the same characters render the same, flagged or not' );
    my $wide = "caf\x{e9}\x{20ac}";
    is( render($wide), render( utf8b($wide) ), 'a wide-character string renders as its UTF-8 bytes' );
    is( render($wide), '"café€"' =~ s/(.*)/utf8b($1)/er =~ s/^"caf.*$/"caf\xC3\xA9\xE2\x82\xAC"/sr, 'as text' );
    is( render("\x{10FFFF}"), "\"\xF4\x8F\xBF\xBF\"", 'the top code point' );
    my $lone = chr(0xD800);
    like( render($lone), qr/^0x/, 'a lone surrogate character is not text' );
    is( $C->can('to_bytes')->($flagged), "\xE9", 'to_bytes downgrades' );
    is( $C->can('to_bytes')->($wide), utf8b($wide), 'to_bytes encodes what cannot be downgraded' );
    ok( $C->can('utf8_valid')->("\xF0\x9F\x98\x80"), 'valid 4-byte sequence' );
    ok( !$C->can('utf8_valid')->("\xF0\x9F\x98"), 'truncated 4-byte sequence' );
};

subtest 'caps are enforced while traversing' => sub {
    my @cycle;
    push @cycle, \@cycle;
    my $r = render( \@cycle );
    like( $r, qr/^(\[){16}\.\.\.(\]){16}$/, 'a self-containing array terminates with ...' );
    my %self;
    $self{me} = \%self;
    like( render( \%self ), qr/\.\.\./, 'a self-containing hash' );
    my $chain = [];
    my $tip   = $chain;
    for ( 1 .. 1000 ) { my $n = []; push @$tip, $n; $tip = $n }
    like( render($chain), qr/^(\[){16}\.\.\.(\]){16}$/, 'a depth-1000 chain is cut at depth 16' );
    my $deep_hash = {};
    my $h = $deep_hash;
    for ( 1 .. 1000 ) { $h->{a} = {}; $h = $h->{a} }
    like( render($deep_hash), qr/\.\.\./, 'a deep hash chain' );
    my @million = (1) x 1_000_000;
    my $m = render( \@million );
    like( $m, qr/\.\.\.$/, 'a million-entry array is truncated' );
    ok( length($m) < 30_000, 'at the node cap (' . length($m) . ' bytes)' );
    my %big = map { ( $_ => 1 ) } 1 .. 300_000;
    like( render( \%big ), qr/\.\.\.$/, 'a 300,000-entry hash is truncated' );
    my $s10 = 'x' x ( 10 * 1024 * 1024 );
    my $rs = render($s10);
    ok( length($rs) <= ( 1 << 20 ) + 8, 'a 10 MiB string renders within 1 MiB (' . length($rs) . ')' );
    like( $rs, qr/\.\.$|\.\.\.$/, '... marked as cut' );
    my @pieces = ( 'y' x 600_000 ) x 3;
    my $rp = render( \@pieces );
    ok( length($rp) <= ( 1 << 20 ) + 8, 'many strings together stay within 1 MiB' );
    my $hex = render( "\xFF" x 3_000_000 );
    ok( length($hex) <= ( 1 << 20 ) + 8, 'a 3 MB invalid string is hex-clipped before encoding (' . length($hex) . ')' );
    my $wide = render( [ map { [$_] } 1 .. 20000 ] );
    like( $wide, qr/\.\.\.$/, '20,000 nested arrays hit the node cap' );
};

subtest 'witnesses with hostile shape still produce a verdict and a digest' => sub {
    my @cycle;
    push @cycle, \@cycle;
    my $gen = CodingAdventures::TestBootstrap::Gen::new( sub { return \@cycle } );
    for my $mode ( 'on', 'off' ) {
        my $v = bounded_by( 30, sub { $R->can('check')->( 1, $gen, sub { 0 }, $Opts->new( { ci => $mode } ) ) } );
        is( $v->{failure_id}, 'property-counterexample', "$mode: a cyclic witness is a counterexample" );
        ok( length( $v->{message} ) < 200_000, "$mode: the message is bounded" );
    }
    my $big = CodingAdventures::TestBootstrap::Gen::new( sub { return 'z' x ( 3 * 1024 * 1024 ) } );
    my $v = bounded_by( 30, sub { $R->can('check')->( 1, $big, sub { 0 }, $Opts->new( { ci => 'on' } ) ) } );
    like( $v->{digest}, qr/^[0-9a-f]{16}$/, 'a 3 MiB witness still has a digest' );
};

subtest 'clip and unrenderable' => sub {
    is( $C->can('clip')->( 'abcdef', 3 ), 'abc...', 'clip' );
    is( $C->can('clip')->( 'abc', 3 ), 'abc', 'clip below the limit' );
    is( $C->can('clip')->( "a\xC3\xA9\xC3\xA9", 3 ), "a\xC3\xA9...", 'clip never cuts a character in half' );
    is( $C->can('UNRENDERABLE')->(), '<unrenderable>', 'the placeholder' );
};

done_testing;
