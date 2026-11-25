package CodingAdventures::TestBootstrap::Tfv;

# Strict validator for TFV v1 text, the line format that carries test corpora
# between languages (TF01 section 2).  This is a VALIDATOR, not a reader: its one
# job here is to let the corpus runner check the pbt-v1 `corpus_reject` cases with
# a real parser instead of skipping them.
#
#   tfv 1
#   # comment
#   <id> R<n> K<field> <value> K<field> <value> ...
#
# A value is one of
#
#   N  T  F                 null, true, false
#   I<int>                  canonical decimal integer, at most 1000 digits
#   D<16 hex>               IEEE-754 bit pattern, lowercase
#   X<hex>  S<hex>          bytes / UTF-8 text as even-length lowercase hex
#   L<n> v1 ... vn          list
#   M<n> k1 v1 ... kn vn    map; no two keys deep-equal
#
# Tokens are separated by exactly one ASCII space.  Rejected: uppercase or
# odd-length hex, `I-0`, `I007`, a count that disagrees with the tokens, trailing
# tokens, doubled spaces, a duplicate field or record id, invalid UTF-8 inside S, a
# carriage return in a record line, nesting deeper than 64 containers and integers
# of more than 1000 digits.
#
# Worked example: "tfv 1\nr R1 Kop S706274 Kdata XFF\n" fails (XFF is uppercase and
# odd length); "bad header\n" fails (the first line must be exactly "tfv 1").

use strict;
use warnings;

use Scalar::Util ();

use CodingAdventures::TestBootstrap::Canon ();

use constant {
    HEADER         => 'tfv 1',
    MAX_DEPTH      => 64,
    MAX_INT_DIGITS => 1000,
    MAX_INPUT      => 1 << 20,
};

package CodingAdventures::TestBootstrap::Tfv::Failure;

package CodingAdventures::TestBootstrap::Tfv;

sub _q {
    my ($text) = @_;
    my $shown = length($text) > 64 ? substr( $text, 0, 64 ) . '...' : $text;
    return '"' . CodingAdventures::TestBootstrap::Canon::escape( $shown, 1 ) . '"';
}

sub _fail { die bless { message => $_[0] }, 'CodingAdventures::TestBootstrap::Tfv::Failure' }

sub _lower_hex_bytes {
    my ($text) = @_;
    return undef if length($text) % 2 != 0 || $text !~ /\A[0-9a-f]*\z/;
    return pack 'H*', $text;
}

sub _canonical_nat {
    my ($text) = @_;
    _fail('empty count') if $text eq '';
    _fail( 'non-digit in count ' . _q($text) ) if $text !~ /\A[0-9]+\z/;
    _fail( 'leading zero in count ' . _q($text) ) if length($text) > 1 && substr( $text, 0, 1 ) eq '0';
    _fail( 'count ' . _q($text) . ' too large' ) if length($text) > 9;
    return 0 + $text;
}

sub _valid_int {
    my ($body) = @_;
    my $neg    = substr( $body, 0, 1 ) eq '-';
    my $digits = $neg ? substr( $body, 1 ) : $body;
    _fail('empty integer') if $digits eq '';
    _fail( 'non-digit in integer ' . _q($body) ) if $digits !~ /\A[0-9]+\z/;
    _fail( 'non-canonical integer ' . _q($body) ) if substr( $digits, 0, 1 ) eq '0' && ( length($digits) > 1 || $neg );
    _fail( 'integer of ' . length($digits) . ' digits exceeds ' . MAX_INT_DIGITS ) if length($digits) > MAX_INT_DIGITS;
}

# Walks the tokens of one record.  `_value` returns a deep-equality key: equal for
# two values exactly when TF01 3.1 calls them deep-equal.
sub _next_token {
    my ($r) = @_;
    _fail('record ends early') if $r->{pos} >= @{ $r->{tokens} };
    return $r->{tokens}[ $r->{pos}++ ];
}

sub _container {
    my ( $r, $tag, $body, $depth ) = @_;
    _fail( 'nesting deeper than ' . MAX_DEPTH ) if $depth + 1 > MAX_DEPTH;
    my $count = _canonical_nat($body);
    if ( $tag eq 'L' ) {
        my @items;
        push @items, _value( $r, $depth + 1 ) for 1 .. $count;    # never reserve memory from a count
        return 'L[' . join( ',', @items ) . ',]';
    }
    my ( %keys, @entries );
    for ( 1 .. $count ) {
        my $k = _value( $r, $depth + 1 );
        my $v = _value( $r, $depth + 1 );
        _fail('duplicate map key') if $keys{$k}++;
        push @entries, "($k=>$v)";
    }
    return 'M{' . join( '', sort { $a cmp $b } @entries ) . '}';
}

sub _value {
    my ( $r, $depth ) = @_;
    my $tok  = _next_token($r);
    my $tag  = substr( $tok, 0, 1 );
    my $body = substr( $tok, 1 );
    if ( $tag eq 'N' || $tag eq 'T' || $tag eq 'F' ) {
        _fail( 'unexpected body in ' . _q($tok) ) if $body ne '';
        return $tag;
    }
    if ( $tag eq 'I' ) { _valid_int($body); return $tok }
    if ( $tag eq 'D' ) {
        my $raw = length($body) == 16 ? _lower_hex_bytes($body) : undef;
        _fail( 'float needs exactly 16 lowercase hex digits: ' . _q($tok) ) if !defined $raw;
        my ( $b1, $b2 ) = unpack 'C2', $raw;
        my $exp = ( ( $b1 & 0x7F ) << 4 ) | ( $b2 >> 4 );
        my $mantissa_zero = ( $b2 & 0x0F ) == 0 && substr( $raw, 2, 6 ) eq "\0" x 6;
        # Every NaN equals every NaN; -0 and +0 stay distinct.
        return 'Dnan' if $exp == 0x7FF && !$mantissa_zero;
        return $tok;
    }
    if ( $tag eq 'X' ) {
        _fail( 'bytes need even-length lowercase hex: ' . _q($tok) ) if !defined _lower_hex_bytes($body);
        return $tok;
    }
    if ( $tag eq 'S' ) {
        my $raw = _lower_hex_bytes($body);
        _fail( 'text needs even-length lowercase hex: ' . _q($tok) ) if !defined $raw;
        _fail( 'text is not strict UTF-8: ' . _q($tok) ) if !CodingAdventures::TestBootstrap::Canon::utf8_valid($raw);
        return $tok;
    }
    return _container( $r, $tag, $body, $depth ) if $tag eq 'L' || $tag eq 'M';
    _fail( 'unknown tag in ' . _q($tok) );
}

sub _validate_record {
    my ($line) = @_;
    _fail('carriage return in a record line') if index( $line, "\r" ) >= 0;
    my @tokens = split / /, $line, -1;
    for my $t (@tokens) {
        _fail('leading, trailing or doubled space') if $t eq '';
    }
    _fail('a record needs an id and R<n>') if @tokens < 2;
    my $id = $tokens[0];
    _fail( 'bad record id ' . _q($id) ) if $id !~ /\A[A-Za-z0-9._\/-]+\z/;
    _fail( 'expected R<n>, got ' . _q( $tokens[1] ) ) if substr( $tokens[1], 0, 1 ) ne 'R';
    my $count  = _canonical_nat( substr( $tokens[1], 1 ) );
    my $reader = { tokens => \@tokens, pos => 2 };
    my %seen;
    for ( 1 .. $count ) {
        my $key_tok = _next_token($reader);
        _fail( 'bad field key ' . _q($key_tok) ) if substr( $key_tok, 0, 1 ) ne 'K' || substr( $key_tok, 1 ) !~ /\A[a-z][a-z0-9_]*\z/;
        _fail( 'duplicate field ' . _q( substr( $key_tok, 1 ) ) ) if $seen{$key_tok}++;
        _value( $reader, 0 );
    }
    _fail('trailing tokens after the last field') if $reader->{pos} < @tokens;
    return $id;
}

# Returns 1 for a well-formed document, `(0, $message)` otherwise.  Reserves no
# memory from a count in the input.
sub validate {
    my ($text) = @_;
    my $err;
    my $ok = do {
        local $@;
        local $SIG{__DIE__} = 'DEFAULT';
        my $r = eval {
            _fail('input must be a string') if !defined $text || ref $text;
            _fail( 'input of ' . length($text) . ' bytes exceeds ' . MAX_INPUT ) if length($text) > MAX_INPUT;
            _fail('document is not valid UTF-8') if !CodingAdventures::TestBootstrap::Canon::utf8_valid($text);
            my @lines = split /\n/, $text, -1;
            pop @lines if @lines && $lines[-1] eq '';    # the final newline is optional
            _fail( 'the first line must be exactly ' . _q(HEADER) ) if !@lines || $lines[0] ne HEADER;
            my %seen_ids;
            for my $n ( 2 .. @lines ) {
                my $line = $lines[ $n - 1 ];
                next if $line eq '' || substr( $line, 0, 1 ) eq '#';
                my $id = eval { _validate_record($line) };
                if ( !defined $id ) {
                    my $e = $@;
                    my $c = Scalar::Util::blessed($e);
                    _fail( "line $n: " . $e->{message} ) if defined $c && $c eq 'CodingAdventures::TestBootstrap::Tfv::Failure';
                    die $e;
                }
                _fail( "line $n: duplicate record id " . _q($id) ) if $seen_ids{$id}++;
            }
            1;
        };
        $err = $@ if !$r;
        $r;
    };
    return 1 if $ok;
    my $c = Scalar::Util::blessed($err);
    return ( 0, 'invalid TFV: ' . $err->{message} ) if defined $c && $c eq 'CodingAdventures::TestBootstrap::Tfv::Failure';
    die $err;
}

# True when `$text` is a valid TFV v1 document.
sub valid { return ( validate( $_[0] ) )[0] ? 1 : 0 }

1;
