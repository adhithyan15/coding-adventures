package CodingAdventures::TestBootstrap::Values;

# Strict decoding of corpus values and kind-sensitive equality (VER05p 4).
#
# Encoding (JSON):
#   {"kind":"int","value":"-12"}     an Int (decimal text, any width up to 1,000 digits)
#   {"kind":"bytes","hex":"00ff"}    a byte string
#   true / false                     the two Bool singletons
#   [ ... ]                          a list (an array reference) of the above
#   "text"                           TEXT (a wrapped record: types, property names)
#
# Perl scalars have no kind, so text is wrapped (Canon::text) and booleans are two
# engine singletons.  Equality never conflates true with 1, bytes with text, an
# integer with a float, or a list with anything else.  It reads values through `B`
# only (see Safe.pm): it runs no overload and no tie.

use strict;
use warnings;

use B ();
use Scalar::Util ();

use CodingAdventures::TestBootstrap::Int   ();
use CodingAdventures::TestBootstrap::Canon ();
use CodingAdventures::TestBootstrap::Safe  ();

use constant {
    MAX_SIZE         => 10_000,
    MAX_INT_DIGITS   => 1000,
    MAX_DEPTH        => 64,
    MAX_FRAGMENT_LEN => 200,
    MAX_PROBLEM_LEN  => 400,
};

my $Int    = 'CodingAdventures::TestBootstrap::Int';
my $Canon  = 'CodingAdventures::TestBootstrap::Canon';
my $blessed = \&Scalar::Util::blessed;

# ------------------------------------------------------------------- booleans

package CodingAdventures::TestBootstrap::Values::Bool;
package CodingAdventures::TestBootstrap::Values;

our $TRUE  = bless \( my $t = 1 ), 'CodingAdventures::TestBootstrap::Values::Bool';
our $FALSE = bless \( my $f = 0 ), 'CodingAdventures::TestBootstrap::Values::Bool';

sub bool { return $_[0] ? $TRUE : $FALSE }

# ------------------------------------------------------------- case errors

# A malformed case (not an engine mismatch): an object of our own class, so the
# corpus runner can print its message without touching any other error.
package CodingAdventures::TestBootstrap::Values::CaseError;
package CodingAdventures::TestBootstrap::Values;

sub fail { die bless { message => $_[0] }, 'CodingAdventures::TestBootstrap::Values::CaseError' }

# A printable, bounded description of ANY error value (string messages are clipped;
# our own case errors print their message; everything else its kind).
sub error_message {
    my ($err) = @_;
    my $c = $blessed->($err);
    if ( defined $c && $c eq 'CodingAdventures::TestBootstrap::Values::CaseError' ) {
        return CodingAdventures::TestBootstrap::Canon::clip( CodingAdventures::TestBootstrap::Canon::escape( CodingAdventures::TestBootstrap::Canon::to_bytes( $err->{message} ) ), MAX_FRAGMENT_LEN );
    }
    if ( defined $err && !ref $err ) {
        return CodingAdventures::TestBootstrap::Canon::clip( CodingAdventures::TestBootstrap::Canon::escape( CodingAdventures::TestBootstrap::Canon::to_bytes($err) ), MAX_FRAGMENT_LEN );
    }
    return CodingAdventures::TestBootstrap::Safe::kind($err);
}

# Decimal digits only, optional leading "-", no leading zeros, no "-0".
sub canonical_int { return ( defined $_[0] && !ref $_[0] && $_[0] =~ /\A(?:0|-?[1-9][0-9]*)\z/ ) ? 1 : 0 }

sub lower_hex { return ( defined $_[0] && !ref $_[0] && length( $_[0] ) % 2 == 0 && $_[0] =~ /\A[0-9a-f]*\z/ ) ? 1 : 0 }

# A short, bounded rendering for problem lines.
sub show { return CodingAdventures::TestBootstrap::Canon::clip( CodingAdventures::TestBootstrap::Canon::render( $_[0] ), MAX_FRAGMENT_LEN ) }

sub _keys_are {
    my ( $node, $a, $b ) = @_;
    my $k = $node->{keys};
    return @$k == 2 && ( ( $k->[0] eq $a && $k->[1] eq $b ) || ( $k->[0] eq $b && $k->[1] eq $a ) );
}

sub _decode_object {
    my ($node) = @_;
    my $kind = $node->{map}{kind};
    my $k = ( ref $kind eq 'HASH' && $kind->{kind} eq 'string' ) ? $kind->{value} : '';
    if ( $k eq 'int' && _keys_are( $node, 'kind', 'value' ) ) {
        my $v = $node->{map}{value};
        fail( 'non-canonical int ' . show( $v->{value} ) ) if ref $v ne 'HASH' || $v->{kind} ne 'string' || !canonical_int( $v->{value} );
        my $text   = $v->{value};
        my $digits = length($text) - ( substr( $text, 0, 1 ) eq '-' ? 1 : 0 );
        fail( 'int exceeds ' . MAX_INT_DIGITS . ' digits' ) if $digits > MAX_INT_DIGITS;
        return CodingAdventures::TestBootstrap::Int::parse($text);
    }
    if ( $k eq 'bytes' && _keys_are( $node, 'kind', 'hex' ) ) {
        my $h = $node->{map}{hex};
        fail( 'bytes hex must be even-length lowercase: ' . show( $h->{value} ) ) if ref $h ne 'HASH' || $h->{kind} ne 'string' || !lower_hex( $h->{value} );
        fail( 'bytes exceed ' . MAX_SIZE . ' bytes' ) if length( $h->{value} ) > 2 * MAX_SIZE;
        return pack 'H*', $h->{value};
    }
    fail('unrecognised value node');
}

# Decodes one JSON node (from StrictJson) into an engine value.
sub decode {
    my ( $node, $depth ) = @_;
    $depth ||= 0;
    fail( 'value nesting deeper than ' . MAX_DEPTH ) if $depth > MAX_DEPTH;
    fail('unsupported JSON value') if ref $node ne 'HASH';
    my $kind = $node->{kind};
    return bool( $node->{value} ) if $kind eq 'bool';
    return CodingAdventures::TestBootstrap::Canon::text( $node->{value} ) if $kind eq 'string';
    if ( $kind eq 'array' ) {
        fail( 'list exceeds ' . MAX_SIZE . ' elements' ) if @{ $node->{items} } > MAX_SIZE;
        return [ map { decode( $_, $depth + 1 ) } @{ $node->{items} } ];
    }
    return _decode_object($node) if $kind eq 'object';
    fail('unsupported JSON value');
}

# The "kind" of an engine value, for kind-sensitive equality; reads internal slots
# only.  Pass the value itself.
sub _kind_of {
    my $b = B::svref_2object( \$_[0] );
    return 'other' if ref($b) eq 'B::SPECIAL';
    my $f = $b->FLAGS;
    return 'other' if $f & CodingAdventures::TestBootstrap::Safe::MAGIC;
    if ( $f & B::SVf_ROK ) {
        my $c = $blessed->( $_[0] );
        if ( defined $c ) {
            if ( $c eq 'Math::BigInt' ) {
                # NaN and infinity are never equal to a number (or to each other)
                return 'other' if Math::BigInt::is_nan( $_[0] ) || Math::BigInt::is_inf( $_[0] );
                return 'int';
            }
            return 'text' if $c eq 'CodingAdventures::TestBootstrap::Canon::Text';
            return ${ $_[0] } ? 'yes' : 'no' if $c eq 'CodingAdventures::TestBootstrap::Values::Bool';
            return 'other';
        }
        return ( ref $_[0] eq 'ARRAY' && !( $b->RV->FLAGS & CodingAdventures::TestBootstrap::Safe::MAGIC ) ) ? 'list' : 'other';
    }
    if ( $f & B::SVf_IOK ) {
        return 'int' if !( $f & B::SVf_POK ) || $b->PV eq ( ( $f & B::SVf_IVisUV ) ? sprintf( '%u', $b->UVX ) : sprintf( '%d', $b->IVX ) );
    }
    return 'float' if ( $f & B::SVf_NOK ) && !( $f & B::SVf_IOK );
    return 'bytes' if $f & B::SVf_POK;
    return 'other';
}

sub _same {
    my ( $a, $b, $depth, $state ) = @_;
    return 0 if $depth > MAX_DEPTH;
    return 0 if ++$state->{nodes} > MAX_SIZE;
    my $ka = _kind_of($a);
    return 0 if $ka ne _kind_of($b);
    return CodingAdventures::TestBootstrap::Int::eq( $a, $b ) ? 1 : 0 if $ka eq 'int';
    if ( $ka eq 'float' ) {
        return 1 if $a != $a && $b != $b;
        return 0 if $a != $b;
        # +0 and -0 stay distinct (the sign shows in the printed form)
        return $a != 0 || sprintf( '%g', $a ) eq sprintf( '%g', $b ) ? 1 : 0;
    }
    return CodingAdventures::TestBootstrap::Canon::text_value($a) eq CodingAdventures::TestBootstrap::Canon::text_value($b) ? 1 : 0 if $ka eq 'text';
    if ( $ka eq 'list' ) {
        return 0 if @$a != @$b;
        for my $i ( 0 .. $#$a ) {
            return 0 if !_same( $a->[$i], $b->[$i], $depth + 1, $state );
        }
        return 1;
    }
    return CodingAdventures::TestBootstrap::Canon::to_bytes($a) eq CodingAdventures::TestBootstrap::Canon::to_bytes($b) ? 1 : 0 if $ka eq 'bytes';
    return ( $ka eq 'yes' || $ka eq 'no' ) ? 1 : 0;
}

# Equality that does not conflate true with 1, bytes with text, integers with floats,
# or lists with anything else.
sub same { return _same( $_[0], $_[1], 0, { nodes => 0 } ) }

1;
