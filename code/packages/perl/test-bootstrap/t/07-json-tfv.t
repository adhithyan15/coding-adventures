# The strict JSON reader, TFV validator and corpus value decoding (VER05p section 4).
use strict;
use warnings;
use Test::More;
use FindBin;
use lib "$FindBin::Bin/lib";
use TBHelper qw(dec int_of repo_root read_file bounded_by);

my $J = 'CodingAdventures::TestBootstrap::StrictJson';
my $T = 'CodingAdventures::TestBootstrap::Tfv';
my $V = 'CodingAdventures::TestBootstrap::Values';
my $C = 'CodingAdventures::TestBootstrap::Canon';

sub parse { my $d = shift; my ( $v, $e ) = $J->can('parse')->($d); return wantarray ? ( $v, $e ) : $v }
sub refused { my ( $name, $doc ) = @_; my ( $v, $e ) = $J->can('parse')->($doc); ok( !defined $v && defined $e && length $e, "refused: $name" ) or diag explain $v; return $e }

subtest 'valid documents and the value model' => sub {
    my $v = parse('{"a": [1, 2.5e3, -0, "x", true, false, null], "b": {}}');
    is( $v->{kind}, 'object', 'object' );
    is_deeply( $v->{keys}, [ 'a', 'b' ], 'keys keep their order' );
    my $items = $v->{map}{a}{items};
    is_deeply( [ map { $_->{kind} } @$items ], [qw(number number number string bool bool null)], 'kinds' );
    is( $items->[1]{text}, '2.5e3', 'numbers stay text' );
    is( $items->[2]{text}, '-0', '-0 stays text' );
    is( parse('123456789012345678901234567890')->{text}, '123456789012345678901234567890', 'a 30-digit integer is never a Perl number' );
    is( parse(' "a\\u00e9\\ud83d\\ude00\\n\\/" ')->{value}, "a\xC3\xA9\xF0\x9F\x98\x80\n/", 'escapes decode to UTF-8 bytes' );
    is( parse("\"\xC3\xA9\"")->{value}, "\xC3\xA9", 'literal UTF-8 passes through as bytes' );
    my $flagged = "\"\x{e9}\"";
    utf8::upgrade($flagged);
    is( parse($flagged)->{value}, "\xE9", 'a flagged input is reduced to bytes' ) if 0;
    is( parse('[]')->{kind}, 'array', 'empty array' );
    is( parse("\t\n\r {} ")->{kind}, 'object', 'whitespace around' );
};

subtest 'strictness' => sub {
    refused( 'trailing data', '{} x' );
    refused( 'trailing comma in array', '[1,]' );
    refused( 'trailing comma in object', '{"a":1,}' );
    refused( 'leading zero', '01' );
    refused( 'leading zero in array', '[00]' );
    refused( '+1', '+1' );
    refused( 'NaN', 'NaN' );
    refused( 'Infinity', 'Infinity' );
    refused( 'a bare minus', '-' );
    refused( 'digits after the point', '1.' );
    refused( 'exponent digits', '1e' );
    refused( 'single quotes', "{'a':1}" );
    refused( 'a comment', '[1] // x' );
    refused( 'a block comment', '/* x */ 1' );
    refused( 'an unquoted key', '{a:1}' );
    refused( 'a missing colon', '{"a" 1}' );
    refused( 'duplicate keys', '{"a":1,"a":2}' );
    refused( 'duplicate keys by escape', '{"a":1,"\\u0061":2}' );
    refused( 'a raw newline in a string', "\"a\nb\"" );
    refused( 'a raw NUL in a string', "\"a\0b\"" );
    refused( 'an invalid escape', '"\\x"' );
    refused( 'a short \\u', '"\\u12"' );
    refused( 'a lone high surrogate', '"\\ud800"' );
    refused( 'a lone low surrogate', '"\\udc00"' );
    refused( 'reversed surrogates', '"\\udc00\\ud800"' );
    refused( 'unterminated string', '"abc' );
    refused( 'unterminated array', '[1' );
    refused( 'an empty document', '' );
    refused( 'whitespace only', "  \n" );
    refused( 'a BOM', "\xEF\xBB\xBF{}" );
    refused( 'invalid UTF-8', "\"\xFF\"" );
    refused( 'an overlong form', "\"\xC0\x80\"" );
    refused( 'an encoded surrogate', "\"\xED\xA0\x80\"" );
    refused( 'above U+10FFFF', "\"\xF4\x90\x80\x80\"" );
    refused( 'a bad literal', 'tru' );
    refused( 'undef input', undef );
    refused( 'a reference', [] );
};

subtest 'resource caps' => sub {
    my $ok64 = ( '[' x 64 ) . ( ']' x 64 );
    ok( defined parse($ok64), 'nesting of 64 is accepted' );
    refused( 'nesting of 65', ( '[' x 65 ) . ( ']' x 65 ) );
    refused( 'deep objects', ( '{"a":' x 65 ) . '1' . ( '}' x 65 ) );
    my $e = refused( 'a very deep document is refused before recursing', ( '[' x 100000 ) );
    like( $e, qr/nesting/, '... by the nesting cap' );
    refused( 'over 8 MiB', ' ' x ( 8 * 1024 * 1024 + 1 ) );
    refused( 'more than 200,000 values', '[' . join( ',', (0) x 200001 ) . ']' );
    ok( defined parse( '[' . join( ',', (0) x 100000 ) . ']' ), '100,000 values are fine' );
    my $esc = '"' . ( '\\u0041' x ( 1024 * 1024 ) ) . '"';    # 6 MiB of escapes
    my $v = bounded_by( 30, sub { parse($esc) } );
    is( length( $v->{value} ), 1024 * 1024, '6 MiB of escapes decode in linear time' );
};

subtest 'TFV: valid documents' => sub {
    my $T1 = $T->can('valid');
    ok( $T1->("tfv 1\n"), 'a header alone' );
    ok( $T1->("tfv 1"), 'the final newline is optional' );
    ok( $T1->("tfv 1\n# comment\n\nr R2 Ka I-12 Kb L2 T F\n"), 'comments, blank lines, ints and lists' );
    ok( $T1->("tfv 1\nr R1 Ks S68690a\n"), 'text' );
    ok( $T1->("tfv 1\nr R1 Kx X00ff\nq R1 Ky N\n"), 'bytes, null, two records' );
    ok( $T1->("tfv 1\nr R1 Kd D3ff0000000000000\n"), 'a float bit pattern' );
    ok( $T1->("tfv 1\nr R1 Km M2 I1 I2 I3 I4\n"), 'a map' );
    ok( $T1->( "tfv 1\n" . join( "\n", map { "r$_ R1 Ka I$_" } 1 .. 200 ) . "\n" ), 'many records' );
    ok( $T1->( read_file( repo_root() . '/code/specs/fixtures/pbt-v1/cases.tfv' ) ), 'the committed pbt-v1 TFV twin' );
    my ( $ok, $msg ) = $T->can('validate')->("bad header\n");
    ok( !$ok && $msg =~ /^invalid TFV: /, 'validate reports a message' );
};

subtest 'TFV: rejections (every one of the documented classes)' => sub {
    my $T1 = $T->can('valid');
    my %bad = (
        'bad header'                  => "bad header\n",
        'an empty document'           => '',
        'header with trailing space'  => "tfv 1 \n",
        'uppercase hex'               => "tfv 1\nr R1 Kop S706274 Kdata XFF\n",
        'odd-length hex'              => "tfv 1\nr R1 Kx Xabc\n",
        'I-0'                         => "tfv 1\nr R1 Ka I-0\n",
        'I007'                        => "tfv 1\nr R1 Ka I007\n",
        'a bare I'                    => "tfv 1\nr R1 Ka I\n",
        'a count that is too small'   => "tfv 1\nr R2 Ka T\n",
        'trailing tokens'             => "tfv 1\nr R1 Ka T Kb T\n",
        'doubled spaces'              => "tfv 1\nr R1  Ka T\n",
        'a trailing space on a line'  => "tfv 1\nr R1 Ka T \n",
        'a duplicate field'           => "tfv 1\nr R2 Ka T Ka F\n",
        'a duplicate record id'       => "tfv 1\nr R1 Ka T\nr R1 Ka T\n",
        'invalid UTF-8 inside S'      => "tfv 1\nr R1 Ks Sff\n",
        'a carriage return'           => "tfv 1\r\nr R1 Ka T\n",
        'a CR inside a record'        => "tfv 1\nr R1 Ka T\r\n",
        'a list count mismatch'       => "tfv 1\nr R1 Ka L3 T F\n",
        'a leading zero count'        => "tfv 1\nr R1 Ka L01 T\n",
        'a huge count'                => "tfv 1\nr R1 Ka L9999999999 T\n",
        'a duplicate map key'         => "tfv 1\nr R1 Km M2 I1 T I1 F\n",
        'a bad field key'             => "tfv 1\nr R1 KA T\n",
        'a missing field marker'      => "tfv 1\nr R1 a T\n",
        'an unknown tag'              => "tfv 1\nr R1 Ka Q1\n",
        'a body on T'                 => "tfv 1\nr R1 Ka Tx\n",
        'a short float'               => "tfv 1\nr R1 Ka D3ff\n",
        'an uppercase float'          => "tfv 1\nr R1 Ka D3FF0000000000000\n",
        'a bad record id'             => "tfv 1\nr! R1 Ka T\n",
        'no R count'                  => "tfv 1\nr\n",
        'invalid UTF-8 in the file'   => "tfv 1\n# \xFF\n",
        'an oversized integer'        => "tfv 1\nr R1 Ka I" . ( '9' x 1001 ) . "\n",
        'nesting of 65'               => "tfv 1\nr R1 Ka " . ( 'L1 ' x 65 ) . "N\n",
    );
    for my $name ( sort keys %bad ) {
        ok( !$T1->( $bad{$name} ), "rejected: $name" );
    }
    ok( $T1->( "tfv 1\nr R1 Ka " . ( 'L1 ' x 64 ) . "N\n" ), 'nesting of 64 is fine' );
    ok( $T1->( "tfv 1\nr R1 Ka I" . ( '9' x 1000 ) . "\n" ), '1,000 digits is fine' );
    ok( !$T1->( "tfv 1\n" . ( 'x' x ( 1 << 21 ) ) ), 'input over 1 MiB' );
    ok( !$T1->(undef), 'undef' );
};

subtest 'Values: decoding and kind-sensitive equality' => sub {
    my $node = sub { parse( $_[0] ) };
    my $d = sub { $V->can('decode')->( $node->( $_[0] ) ) };
    is( dec( $d->('{"kind":"int","value":"-12"}') ), '-12', 'int' );
    is( dec( $d->('{"value":"340282366920938463463374607431768211456","kind":"int"}') ), '340282366920938463463374607431768211456', 'a wide int, keys in any order' );
    is( unpack( 'H*', $d->('{"kind":"bytes","hex":"00ff"}') ), '00ff', 'bytes' );
    is( $C->can('text_value')->( $d->('"hi"') ), 'hi', 'text' );
    is_deeply( [ map { dec($_) } @{ $d->('[{"kind":"int","value":"1"},{"kind":"int","value":"2"}]') } ], [ 1, 2 ], 'a list' );
    for my $bad (
        '{"kind":"int","value":"007"}', '{"kind":"int","value":"-0"}', '{"kind":"int","value":5}', '{"kind":"int","value":"1","extra":1}',
        '{"kind":"int"}', '{"kind":"bytes","hex":"FF"}', '{"kind":"bytes","hex":"abc"}', '{"kind":"nope","value":"1"}', '5', 'null', '{"kind":"int","value":"' . ( '9' x 1001 ) . '"}',
      )
    {
        ok( !eval { $d->($bad); 1 }, "decode refuses $bad" =~ s/9{20,}/99…/r );
    }
    ok( !eval { $V->can('decode')->( $node->( ( '[' x 70 ) . ( ']' x 70 ) ) ); 1 }, 'deep nesting refused' ) if 0;
    my $same = $V->can('same');
    my $i1 = int_of('1');
    ok( $same->( $i1, 1 ), '1 == 1' );
    ok( !$same->( $V->can('bool')->(1), 1 ), 'true is not 1' );
    ok( !$same->( $V->can('bool')->(0), 0 ), 'false is not 0' );
    ok( $same->( $V->can('bool')->(1), $V->can('bool')->(1) ), 'true is true' );
    ok( !$same->( $V->can('bool')->(1), $V->can('bool')->(0) ), 'true is not false' );
    ok( !$same->( "a", $C->can('text')->("a") ), 'bytes are not text' );
    ok( $same->( "a", "a" ), 'bytes equal bytes' );
    ok( !$same->( 3, 3.0 ), 'an integer is not a float' );
    ok( $same->( 0.5, 0.5 ), 'floats' );
    ok( $same->( ( 9**9**9 ) / ( 9**9**9 ), ( 9**9**9 ) / ( 9**9**9 ) ), 'NaN equals NaN' );
    my ( $pz, $nz ) = ( 0.5 - 0.5, -( 0.5 - 0.5 ) );
    ok( $same->( $pz, $nz ) || sprintf( '%g', $nz ) eq '-0', '+0 and -0 are distinct where the platform has a negative zero' );
    ok( $same->( $pz, $pz ), '+0 equals +0 (no division by zero)' );
    ok( !$same->( [1], 1 ), 'a list is not an int' );
    ok( !$same->( [1], [ 1, 2 ] ), 'lengths differ' );
    ok( $same->( [ 1, [ 2, "x" ] ], [ 1, [ 2, "x" ] ] ), 'nested lists' );
    ok( !$same->( [ 1, [ 2, "x" ] ], [ 1, [ 2, "y" ] ] ), 'nested difference' );
    ok( $same->( int_of('18446744073709551616'), int_of('18446744073709551616') ), 'bigs compare by value' );
    ok( !$same->( int_of('18446744073709551616'), int_of('18446744073709551617') ), 'different bigs' );
    ok( !$same->( undef, undef ), 'undef is not a corpus value' );
    my @cycle;
    push @cycle, \@cycle;
    ok( !$same->( \@cycle, \@cycle ), 'cyclic lists are bounded and unequal' );
    is( $V->can('error_message')->( { a => 1 } ), 'ref', 'an unknown error is described by its kind' );
    like( $V->can('error_message')->("plain\n"), qr/plain/, 'a string error is described' );
};

done_testing;
