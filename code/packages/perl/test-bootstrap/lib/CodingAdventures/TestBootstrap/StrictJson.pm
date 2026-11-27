package CodingAdventures::TestBootstrap::StrictJson;

# A strict JSON reader for the conformance corpus (VER05p section 4).
#
# A lenient parser would let a corrupted corpus through, and Perl's JSON::PP is
# not strict about everything below (and is not a dependency we want), so this one
# is strict, and works on BYTES:
#
#   * nesting deeper than 64 (the 65th container is refused BEFORE recursing),
#     documents over 8 MiB, more than 200,000 values (object keys count);
#   * invalid UTF-8 (overlong forms, surrogates, > U+10FFFF) and a BOM are refused;
#   * raw control characters in strings, lone or reversed surrogate escapes,
#     leading zeros, `+1`, `NaN`, comments, trailing commas and TRAILING DATA are
#     refused;
#   * duplicate object keys are refused, compared by their decoded bytes;
#   * NUMBERS STAY TEXT: a 30-digit integer is never converted to a Perl number.
#
# Every value is a hash with a `kind`:
#
#   { kind => 'null' }                       { kind => 'bool',   value => 1 or 0 }
#   { kind => 'number', text => '12' }       { kind => 'string', value => $bytes }
#   { kind => 'array',  items => [...] }
#   { kind => 'object', keys => [...in order...], map => { key => value } }
#
# `parse($data)` returns the value, or `(undef, $message)`; it never dies on bad
# input.

use strict;
use warnings;

use Scalar::Util ();

use CodingAdventures::TestBootstrap::Canon ();

use constant {
    MAX_DEPTH => 64,
    MAX_BYTES => 8 * 1024 * 1024,
    MAX_NODES => 200_000,
};

my %ESCAPES = ( '"' => '"', '\\' => '\\', '/' => '/', 'b' => "\b", 'f' => "\f", 'n' => "\n", 'r' => "\r", 't' => "\t" );

package CodingAdventures::TestBootstrap::StrictJson::Failure;

package CodingAdventures::TestBootstrap::StrictJson;

sub _fail {
    my ( $p, $message ) = @_;
    die bless { message => $message . ' at byte ' . $p->{pos} }, 'CodingAdventures::TestBootstrap::StrictJson::Failure';
}

sub _peek { return $_[0]{pos} < length( $_[0]{s} ) ? ord( substr( $_[0]{s}, $_[0]{pos}, 1 ) ) : undef }

sub _skip_ws {
    my ($p) = @_;
    pos( $p->{s} ) = $p->{pos};
    $p->{s} =~ /\G[ \t\n\r]*/gc;
    $p->{pos} = pos( $p->{s} );
}

sub _count_node {
    my ($p) = @_;
    _fail( $p, 'more than ' . MAX_NODES . ' values' ) if ++$p->{nodes} > MAX_NODES;
}

sub _literal {
    my ( $p, $word, $value ) = @_;
    _fail( $p, 'invalid literal' ) if substr( $p->{s}, $p->{pos}, length $word ) ne $word;
    $p->{pos} += length $word;
    return $value;
}

sub _hex4 {
    my ($p) = @_;
    pos( $p->{s} ) = $p->{pos};
    _fail( $p, 'invalid \\u escape' ) if $p->{s} !~ /\G([0-9A-Fa-f]{4})/gc;
    $p->{pos} += 4;
    return hex $1;
}

sub _utf8_of {
    my ($cp) = @_;
    my $c = chr $cp;
    utf8::encode($c);
    return $c;
}

sub _escape {
    my ($p) = @_;
    my $code = _peek($p);
    _fail( $p, 'unterminated escape' ) if !defined $code;
    my $ch = chr $code;
    $p->{pos}++;
    return $ESCAPES{$ch} if exists $ESCAPES{$ch};
    _fail( $p, 'invalid escape' ) if $ch ne 'u';
    my $unit = _hex4($p);
    if ( $unit >= 0xD800 && $unit <= 0xDBFF ) {
        _fail( $p, 'lone high surrogate' ) if substr( $p->{s}, $p->{pos}, 2 ) ne '\\u';
        $p->{pos} += 2;
        my $low = _hex4($p);
        _fail( $p, 'invalid surrogate pair' ) if $low < 0xDC00 || $low > 0xDFFF;
        return _utf8_of( 0x10000 + ( ( $unit - 0xD800 ) << 10 ) + ( $low - 0xDC00 ) );
    }
    _fail( $p, 'lone low surrogate' ) if $unit >= 0xDC00 && $unit <= 0xDFFF;
    return _utf8_of($unit);
}

sub _parse_string {
    my ($p) = @_;
    $p->{pos}++;
    my $out = '';
    while (1) {
        pos( $p->{s} ) = $p->{pos};
        $p->{s} =~ /\G([^\x00-\x1F"\\]*)/gc;
        $out .= $1;
        $p->{pos} = pos( $p->{s} );
        my $byte = _peek($p);
        _fail( $p, 'unterminated string' ) if !defined $byte;
        if ( $byte == 0x22 ) { $p->{pos}++; return $out }
        _fail( $p, 'raw control character in string' ) if $byte < 0x20;
        $p->{pos}++;
        $out .= _escape($p);
    }
}

sub _parse_number {
    my ($p) = @_;
    pos( $p->{s} ) = $p->{pos};
    _fail( $p, 'invalid number' ) if $p->{s} !~ /\G(-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)/gc;
    my $text = $1;
    $p->{pos} = pos( $p->{s} );
    my $next = _peek($p);
    _fail( $p, 'leading zero' ) if defined $next && $next >= 0x30 && $next <= 0x39;
    return { kind => 'number', text => $text };
}

sub _parse_array {
    my ( $p, $depth ) = @_;
    _fail( $p, 'nesting deeper than ' . MAX_DEPTH ) if $depth + 1 > MAX_DEPTH;
    $p->{pos}++;
    my @items;
    _skip_ws($p);
    if ( ( _peek($p) // -1 ) == 0x5D ) { $p->{pos}++; return { kind => 'array', items => \@items } }
    while (1) {
        _skip_ws($p);
        push @items, _parse_value( $p, $depth + 1 );
        _skip_ws($p);
        my $c = _peek($p);
        if    ( defined $c && $c == 0x2C ) { $p->{pos}++ }
        elsif ( defined $c && $c == 0x5D ) { $p->{pos}++; return { kind => 'array', items => \@items } }
        else                               { _fail( $p, 'expected , or ]' ) }
    }
}

sub _parse_object {
    my ( $p, $depth ) = @_;
    _fail( $p, 'nesting deeper than ' . MAX_DEPTH ) if $depth + 1 > MAX_DEPTH;
    $p->{pos}++;
    my $out = { kind => 'object', keys => [], map => {} };
    _skip_ws($p);
    if ( ( _peek($p) // -1 ) == 0x7D ) { $p->{pos}++; return $out }
    while (1) {
        _skip_ws($p);
        _fail( $p, 'expected a string key' ) if ( _peek($p) // -1 ) != 0x22;
        _count_node($p);
        my $key = _parse_string($p);
        _fail( $p, 'duplicate key' ) if exists $out->{map}{$key};
        _skip_ws($p);
        _fail( $p, 'expected :' ) if ( _peek($p) // -1 ) != 0x3A;
        $p->{pos}++;
        _skip_ws($p);
        push @{ $out->{keys} }, $key;
        $out->{map}{$key} = _parse_value( $p, $depth + 1 );
        _skip_ws($p);
        my $c = _peek($p);
        if    ( defined $c && $c == 0x2C ) { $p->{pos}++ }
        elsif ( defined $c && $c == 0x7D ) { $p->{pos}++; return $out }
        else                               { _fail( $p, 'expected , or }' ) }
    }
}

sub _parse_value {
    my ( $p, $depth ) = @_;
    _count_node($p);
    my $c = _peek($p);
    _fail( $p, 'unexpected end of input' ) if !defined $c;
    return _parse_object( $p, $depth ) if $c == 0x7B;
    return _parse_array( $p, $depth )  if $c == 0x5B;
    return { kind => 'string', value => _parse_string($p) } if $c == 0x22;
    return _literal( $p, 'true',  { kind => 'bool', value => 1 } ) if $c == 0x74;
    return _literal( $p, 'false', { kind => 'bool', value => 0 } ) if $c == 0x66;
    return _literal( $p, 'null',  { kind => 'null' } ) if $c == 0x6E;
    return _parse_number($p) if $c == 0x2D || ( $c >= 0x30 && $c <= 0x39 );
    _fail( $p, sprintf( 'unexpected byte 0x%02x', $c ) );
}

# Parses `$data` (a byte string).  Returns the value, or `(undef, $message)`.
sub parse {
    my ($data_in) = @_;
    return ( undef, 'JSON input must be a string' ) if !defined $data_in || ref $data_in;
    my $data = CodingAdventures::TestBootstrap::Canon::to_bytes($data_in);
    return ( undef, 'document of ' . length($data) . ' bytes exceeds ' . MAX_BYTES ) if length($data) > MAX_BYTES;
    return ( undef, 'document is not valid UTF-8' ) if !CodingAdventures::TestBootstrap::Canon::utf8_valid($data);
    return ( undef, 'a byte-order mark is not allowed' ) if substr( $data, 0, 3 ) eq "\xEF\xBB\xBF";
    my $p = { s => $data, pos => 0, nodes => 0 };
    my ( $value, $err );
    {
        local $@;
        local $SIG{__DIE__} = 'DEFAULT';
        my $ok = eval {
            _skip_ws($p);
            $value = _parse_value( $p, 0 );
            _skip_ws($p);
            _fail( $p, 'trailing data after the document' ) if $p->{pos} < length $data;
            1;
        };
        $err = $@ if !$ok;
        return ($value) if $ok;
    }
    my $c = Scalar::Util::blessed($err);
    return ( undef, $err->{message} ) if defined $c && $c eq 'CodingAdventures::TestBootstrap::StrictJson::Failure';
    die $err;
}

1;
