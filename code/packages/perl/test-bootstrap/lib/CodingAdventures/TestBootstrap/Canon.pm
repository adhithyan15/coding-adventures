package CodingAdventures::TestBootstrap::Canon;

# A stable, TOTAL text form of a witness (used in messages and the CI digest),
# VER05p section 4.
#
# It never dies, and it never runs user code.  A Perl value can be an object whose
# class overloads `""`, `0+`, `bool`, `eq`, `==`, `<=>`, `cmp`, `@{}`, `%{}`, or a
# variable tied to a class whose FETCH, FETCHSIZE and FIRSTKEY die.  So this module
# reads values ONLY through `B` (the internal slots), never dereferences a user
# reference, never stringifies a user scalar, and treats anything with magic as
# opaque (see Safe.pm).
#
#   undef                    undef
#   integer                  decimal (above 100,000 bits: "<bigint N bits>")
#   float                    shortest round-trip text that still looks like a
#                            float (3.0, never 3); nan, inf, -inf
#   string, valid UTF-8      "quoted", \" \\ \n \r \t short forms, \uXXXX otherwise
#   string, other            0x<lowercase hex>
#   array                    [a,b,c]
#   hash                     %{k=>v,...}   sorted by the BYTES of "k=>v"
#   blessed array/hash       Class=[...]  Class=%{...}
#   scalar reference         \x
#   code / glob / regexp     #Code  #Glob  #Regexp          magic  #Magic
#
# Caps, enforced while the value is traversed: nesting deeper than 16 renders
# "...", more than 10,000 nodes or 1 MiB of output truncates (cut on a character
# boundary, then "..."), and a failure while rendering turns the WHOLE witness
# into "<unrenderable>".  A structure that contains itself is just a deep one.
#
# STRINGS ARE BYTES.  The utf8 flag is never consulted for meaning: `to_bytes`
# reduces a string to bytes (a flagged string whose characters all fit in a byte
# is those bytes, otherwise its UTF-8 octets) so equal characters render alike
# whichever way Perl happens to store them.
#
# The escape set is copied exactly from the Elixir lane's Canon (VER05m): every C0
# control, DEL, the C1 controls, U+2028/9, the bidirectional controls and the
# invisible formatting characters are escaped, so a witness can never forge a log
# line or reorder the text around it.

use strict;
use warnings;

use B ();
use Encode ();
use Scalar::Util ();

use CodingAdventures::TestBootstrap::Int  ();
use CodingAdventures::TestBootstrap::Safe ();

use constant {
    MAX_DEPTH       => 16,
    MAX_NODES       => 10_000,
    MAX_OUTPUT      => 1 << 20,
    MAX_BIGINT_BITS => 100_000,
    UNRENDERABLE    => '<unrenderable>',
    TEXT_CLASS      => 'CodingAdventures::TestBootstrap::Canon::Text',
};

my $blessed = \&Scalar::Util::blessed;
my $TRUNC   = \my $trunc_marker;    # thrown when a cap is crossed
my $MAGIC   = CodingAdventures::TestBootstrap::Safe::MAGIC;

# ------------------------------------------------------------------- text

# TEXT (as opposed to a byte string): a wrapped record, so corpus text can be
# told from bytes (both are Perl strings).
sub text {
    my ($s) = @_;
    my $copy = $s;
    return bless \$copy, TEXT_CLASS;
}

sub is_text {
    my $c = $blessed->( $_[0] );
    return ( defined $c && $c eq TEXT_CLASS ) ? 1 : 0;
}

sub text_value { return ${ $_[0] } }

# A string value reduced to bytes, independent of the utf8 flag.
sub to_bytes {
    my ($s) = @_;
    if ( utf8::is_utf8($s) ) {
        my $c = $s;
        return $c if utf8::downgrade( $c, 1 );
        utf8::encode($c);
        return $c;
    }
    return $s;
}

# Strict UTF-8: no surrogates, no overlong forms, nothing above U+10FFFF (the
# noncharacters U+FFFE, U+FFFF ... are valid, as in the Lua lane).  Encode's lax
# 'utf8' decoder rejects overlong forms and truncation; the two code point ranges it
# lets through are refused here.  Returns the decoded character string, or undef.
sub _decode {
    my ($bytes) = @_;
    my $copy = $bytes;
    local $@;
    my $chars = eval { Encode::decode( 'utf8', $copy, Encode::FB_CROAK() | Encode::LEAVE_SRC() ) };
    return undef if !defined $chars;
    return undef if $chars =~ /[\x{D800}-\x{DFFF}]|[^\x{0}-\x{10FFFF}]/;
    return $chars;
}

sub utf8_valid { return defined _decode( $_[0] ) ? 1 : 0 }

# ------------------------------------------------------------- escaping

my $ESCAPE = qr/[\x00-\x1F\x7F-\x9F\x{2028}\x{2029}\x{061C}\x{200E}\x{200F}\x{202A}-\x{202E}\x{2066}-\x{2069}\x{00AD}\x{034F}\x{180E}\x{FEFF}\x{200B}-\x{200D}\x{2060}-\x{2064}\x{FFF9}-\x{FFFB}\x{E0000}-\x{E007F}]/;
my $ESCAPE_Q = qr/[\\"\x00-\x1F\x7F-\x9F\x{2028}\x{2029}\x{061C}\x{200E}\x{200F}\x{202A}-\x{202E}\x{2066}-\x{2069}\x{00AD}\x{034F}\x{180E}\x{FEFF}\x{200B}-\x{200D}\x{2060}-\x{2064}\x{FFF9}-\x{FFFB}\x{E0000}-\x{E007F}]/;

sub _escape_cp {
    my ($c) = @_;
    return '\\n'  if $c == 10;
    return '\\r'  if $c == 13;
    return '\\t'  if $c == 9;
    return '\\\\' if $c == 92;
    return '\\"'  if $c == 34;
    return $c > 0xFFFF ? sprintf( '\\u{%x}', $c ) : sprintf( '\\u%04x', $c );
}

sub hex_of { return unpack( 'H*', $_[0] ) }

# Drops up to 3 trailing bytes of a character cut in half.
sub trim_partial {
    my ($s) = @_;
    for ( 0 .. 3 ) {
        return $s if length($s) == 0 || utf8_valid($s);
        chop $s;
    }
    return $s;
}

# Escapes control and invisible characters (and, with `$quote`, backslash and the
# double quote) so a string can never forge a log line.  A string that is not
# valid UTF-8 becomes `0x<hex>`.  Input and output are BYTE strings.
sub escape {
    my ( $bytes, $quote, $limit ) = @_;
    my $chars = _decode($bytes);
    if ( !defined $chars ) {
        return '0x' . hex_of( defined $limit ? substr( $bytes, 0, $limit ) : $bytes );
    }
    if ($quote) { $chars =~ s/($ESCAPE_Q)/_escape_cp(ord $1)/ge }
    else        { $chars =~ s/($ESCAPE)/_escape_cp(ord $1)/ge }
    utf8::encode($chars);
    return $chars;
}

# A bounded fragment for reports: fixture-derived text must never make a problem
# line (or a log) unboundedly large.  Cut on a character boundary.
sub clip {
    my ( $text, $limit ) = @_;
    return trim_partial( substr( $text, 0, $limit ) ) . '...' if length($text) > $limit;
    return $text;
}

# ---------------------------------------------------------------- rendering

sub float_text {
    my ($v) = @_;
    return 'nan'  if $v != $v;
    return 'inf'  if $v == 9**9**9;
    return '-inf' if $v == -9**9**9;
    my $s;
    for my $digits ( 15 .. 17 ) {
        $s = sprintf( "%.${digits}g", $v );
        last if $s + 0 == $v;
    }
    $s .= '.0' if $s !~ /[.eEn]/;
    return $s;
}

sub _leaf {
    my ( $ctx, $out, $text ) = @_;
    push @$out, $text;
    $ctx->{size} += length $text;
    die $TRUNC if $ctx->{size} > MAX_OUTPUT;
    return;
}

sub _count {
    my ($ctx) = @_;
    die $TRUNC if ++$ctx->{nodes} > MAX_NODES;
    return;
}

sub _string_text {
    my ( $bytes, $ctx ) = @_;
    # The cap is applied BEFORE any allocation: clip to what the output may still
    # hold (plus a little, so the overflow is noticed) before validity checking,
    # escaping or hex encoding.
    my $remaining = MAX_OUTPUT - $ctx->{size};
    $remaining = 0 if $remaining < 0;
    $remaining += 4;
    my $cut = 0;
    if ( length($bytes) > $remaining ) {
        $bytes = trim_partial( substr( $bytes, 0, $remaining ) );
        $cut   = 1;
    }
    my $text;
    if ( utf8_valid($bytes) ) { $text = '"' . escape( $bytes, 1, $remaining ) . '"' }
    else                       { $text = '0x' . hex_of( substr( $bytes, 0, int( $remaining / 2 ) + 2 ) ) }
    $text .= '..' if $cut && length($text) <= $remaining;    # guarantee the cap is crossed
    return $text;
}

# The string value of a scalar B object (a copy: no magic was checked away).
sub _sv_bytes {
    my ($b) = @_;
    return to_bytes( ${ $b->object_2svref } );
}

sub _emit_scalar {
    my ( $b, $depth, $ctx, $out ) = @_;
    my $f = $b->FLAGS;
    if ( $f & B::SVf_IOK ) {
        my $int = ( $f & B::SVf_IVisUV ) ? sprintf( '%u', $b->UVX ) : sprintf( '%d', $b->IVX );
        if ( !( $f & B::SVf_POK ) || $b->PV eq $int ) {
            return _leaf( $ctx, $out, $int );
        }
    }
    elsif ( $f & B::SVf_NOK ) {
        return _leaf( $ctx, $out, float_text( $b->NV ) );
    }
    if ( $f & B::SVf_POK ) {
        return _leaf( $ctx, $out, _string_text( _sv_bytes($b), $ctx ) );
    }
    return _leaf( $ctx, $out, 'undef' );
}

sub _render_sub {
    my ( $b, $depth, $ctx ) = @_;
    my @sub;
    _emit( $b, $depth, $ctx, \@sub );
    return join( '', @sub );
}

sub _emit_ref {
    my ( $r, $depth, $ctx, $out ) = @_;
    return _leaf( $ctx, $out, '#Magic' ) if $r->FLAGS & $MAGIC;
    my $cls  = ref $r;
    my $name = CodingAdventures::TestBootstrap::Safe::stash_name($r);
    if ( defined $name ) {
        if ( $name eq 'Math::BigInt' && $cls eq 'B::HV' ) {
            # NaN and infinity are not integers: render them, never measure them.
            my $obj = $r->object_2svref;
            return _leaf( $ctx, $out, 'NaN' ) if Math::BigInt::is_nan($obj);
            return _leaf( $ctx, $out, Math::BigInt::is_neg($obj) ? '-inf' : 'inf' ) if Math::BigInt::is_inf($obj);
            my $bits = CodingAdventures::TestBootstrap::Int::bit_length( $r->object_2svref );
            return _leaf( $ctx, $out, sprintf( '<bigint %d bits>', $bits ) ) if $bits > MAX_BIGINT_BITS;
            die $TRUNC if $ctx->{size} + int( $bits / 3 ) > MAX_OUTPUT;
            return _leaf( $ctx, $out, CodingAdventures::TestBootstrap::Int::tostring( $r->object_2svref ) );
        }
        if ( $name eq TEXT_CLASS ) {
            return _leaf( $ctx, $out, _string_text( _sv_bytes($r), $ctx ) );
        }
        return _leaf( $ctx, $out, '#Regexp' ) if $name eq 'Regexp';
    }
    return _leaf( $ctx, $out, '#Regexp' ) if $cls eq 'B::REGEXP';
    return _leaf( $ctx, $out, '#Code' )   if $cls eq 'B::CV';
    return _leaf( $ctx, $out, '#Glob' )   if $cls eq 'B::GV';
    return _leaf( $ctx, $out, '#Io' )     if $cls eq 'B::IO' || $cls eq 'B::FM';
    return _leaf( $ctx, $out, '...' ) if $depth >= MAX_DEPTH;
    if ( defined $name && ( $cls eq 'B::AV' || $cls eq 'B::HV' ) ) {
        my $label = to_bytes($name);
        $label = substr( $label, 0, 100 ) if length($label) > 100;
        _leaf( $ctx, $out, escape( $label, 0 ) . '=' );
    }
    if ( $cls eq 'B::AV' ) {
        my $n = $r->FILL + 1;
        $n = MAX_NODES + 1 if $n > MAX_NODES + 1;
        _leaf( $ctx, $out, '[' );
        for my $i ( 0 .. $n - 1 ) {
            _leaf( $ctx, $out, ',' ) if $i > 0;
            _emit( $r->ARRAYelt($i), $depth + 1, $ctx, $out );
        }
        return _leaf( $ctx, $out, ']' );
    }
    if ( $cls eq 'B::HV' ) {
        _leaf( $ctx, $out, '%{' );
        die $TRUNC if $r->KEYS > MAX_NODES;
        my @pairs = $r->ARRAY;
        my @entries;
        while (@pairs) {
            my ( $k, $v ) = splice @pairs, 0, 2;
            _count($ctx);
            my $ks = _string_text( to_bytes($k), $ctx );
            push @entries, $ks . '=>' . _render_sub( $v, $depth + 1, $ctx );
        }
        @entries = sort { $a cmp $b } @entries;
        _leaf( $ctx, $out, join( ',', @entries ) );
        return _leaf( $ctx, $out, '}' );
    }
    # a scalar reference
    _leaf( $ctx, $out, '\\' );
    return _emit( $r, $depth + 1, $ctx, $out );
}

sub _emit {
    my ( $b, $depth, $ctx, $out ) = @_;
    _count($ctx);
    if ( ref($b) eq 'B::SPECIAL' ) {
        my $i = $$b;
        return _leaf( $ctx, $out, $i == 2 ? '1' : $i == 3 ? '""' : 'undef' );
    }
    return _leaf( $ctx, $out, '#Magic' ) if $b->FLAGS & $MAGIC;
    return _emit_ref( $b->RV, $depth, $ctx, $out ) if $b->FLAGS & B::SVf_ROK;
    return _emit_scalar( $b, $depth, $ctx, $out );
}

sub _truncate {
    my ($parts) = @_;
    my $s = join( '', @$parts );
    $s = substr( $s, 0, MAX_OUTPUT ) if length($s) > MAX_OUTPUT;
    return trim_partial($s) . '...';
}

# Renders any value; never dies, never runs user code.  Pass the value itself
# (it is inspected in place, not copied).
sub render {
    my $ctx = { nodes => 0, size => 0 };
    my @out;
    my $err;
    my $ok = do {
        local $@;
        my $r = eval { _emit( B::svref_2object( \$_[0] ), 0, $ctx, \@out ); 1 };
        $err = $@ if !$r;
        $r;
    };
    return join( '', @out ) if $ok;
    return _truncate( \@out ) if ref $err && Scalar::Util::refaddr($err) == Scalar::Util::refaddr($TRUNC);
    return UNRENDERABLE;
}

1;
