package CodingAdventures::TestBootstrap::Safe;

# The boundary between the engine and user code (VER05p section 4).
#
# Perl lets nearly any operation run user code: stringification, numification,
# `eq`, `==`, `bool`, dereferencing an object that overloads `@{}`, reading a tied
# variable, DESTROY.  So the engine follows two rules, both implemented here:
#
#   1. User code runs ONLY through `call`: the property, a generator callback, a
#      shrinker callback.  `call` classifies the outcome by the result of `eval`,
#      never by `$@`, never by stringifying a thrown value.
#   2. User VALUES are inspected only through the `B` module (the internal slots
#      of the scalar: flags, integer, double, string, referent, array and hash
#      contents), which calls no method, no overload and no tie.  Anything with
#      get, set or tie magic is opaque.

use strict;
use warnings;

use B ();
use Math::BigInt ();
use Scalar::Util ();

my $blessed = \&Scalar::Util::blessed;
my $reftype = \&Scalar::Util::reftype;

use constant MAGIC => B::SVs_GMG() | B::SVs_SMG() | B::SVs_RMG();

# ------------------------------------------------------------------- calling

# Runs `$code->(@args)` in scalar context.  Returns (status, value):
#
#   ('ok',      $result)  it returned
#   ('died',    $error)   it died; $error is ANY value (never inspected here)
#   ('escaped', undef)    it left through `last`, `next` or `redo` (which cross
#                         sub and eval frames), or any other non-local exit
#
# The caller's $@ and $_ and the die/warn handlers are restored on every exit,
# and a user-installed handler is not run while the code is called.
sub call {
    my $code = shift;
    my ( $status, $value ) = ( 'escaped', undef );
    {
        local $@;
        local $_;
        local $SIG{__DIE__}  = 'DEFAULT';
        local $SIG{__WARN__} = 'DEFAULT';
        ONCE: for my $once (1) {
            my $r;
            if ( eval { $r = $code->(@_); 1 } ) {
                ( $status, $value ) = ( 'ok', $r );
            }
            else {
                ( $status, $value ) = ( 'died', $@ );
            }
        }
    }
    return ( $status, $value );
}

# A property holds ONLY on the plain scalar 1: a reference (even an object with a
# true `bool`), undef, "", 0, 2, the empty list and any other value falsify.
sub holds {
    return 0 if !defined $_[0] || ref $_[0];
    return $_[0] eq '1' ? 1 : 0;
}

# The kind of an error value, by `ref` and `blessed` only (never its content).
sub kind {
    return 'undef'  if !defined $_[0];
    return 'string' if !ref $_[0];
    return 'object' if defined $blessed->( $_[0] );
    return 'ref';
}

# ------------------------------------------------------------- B inspection

# The B object of a scalar variable: pass the scalar itself (it is aliased, not
# copied, so a tied scalar is not FETCHed).
sub sv { return B::svref_2object( \$_[0] ) }

sub is_magic {
    my ($b) = @_;
    return ( $b->FLAGS & MAGIC ) ? 1 : 0;
}

# The class of the referent of a B object that is a reference, or undef.
sub stash_name {
    my ($b) = @_;
    return undef if !( $b->FLAGS & B::SVs_OBJECT );
    my $st = $b->SvSTASH;
    return $st->isa('B::HV') ? $st->NAME : undef;
}


# Author-supplied containers (an Options hash, a one_of array) are copied without
# running any overload or tie: the reference must be a plain, magic-free ARRAY or
# HASH, and `no overloading` disables `@{}` / `%{}` for the copy.
sub array_copy {
    my ( $ref, $what ) = @_;
    die "$what must be an array reference\n" if !ref $ref || ( $reftype->($ref) // '' ) ne 'ARRAY';
    die "$what must not be tied\n" if B::svref_2object($ref)->FLAGS & MAGIC;
    no overloading;
    return [ @{$ref} ];
}

sub hash_copy {
    my ( $ref, $what ) = @_;
    die "$what must be a hash reference\n" if !ref $ref || ( $reftype->($ref) // '' ) ne 'HASH';
    die "$what must not be tied\n" if B::svref_2object($ref)->FLAGS & MAGIC;
    no overloading;
    return { %{$ref} };
}

# ----------------------------------------------- size estimate and deep copy

use constant TEXT_CLASS => 'CodingAdventures::TestBootstrap::Canon::Text';
use constant WALK_CAP   => 10_000;

# The B object of the referent when `$b` is a reference, else undef.
sub _referent { return ( $_[0]->FLAGS & B::SVf_ROK ) ? $_[0]->RV : undef }

# Elements of a string, array or hash; ONE visit allowance is shared by the whole
# walk.  Reads internal slots only (no method, overload or tie runs).
sub _shallow {
    my ( $b, $allow ) = @_;
    return 0 if ref($b) eq 'B::SPECIAL';
    my $f = $b->FLAGS;
    return 0 if $f & MAGIC;
    if ( $f & B::SVf_ROK ) {
        my $r = $b->RV;
        return 0 if $r->FLAGS & MAGIC;
        my $cls = ref $r;
        my $name = stash_name($r);
        return 0 if defined $name && ( $name eq 'Math::BigInt' || $name eq TEXT_CLASS );
        my $n = 0;
        if ( $cls eq 'B::AV' ) { $n = $r->FILL + 1 }
        elsif ( $cls eq 'B::HV' ) { $n = $r->KEYS }
        else { return 0 }
        $n = $$allow if $n > $$allow;
        $$allow -= $n;
        return $n;
    }
    return $b->CUR if ( $f & B::SVf_POK ) && !( $f & B::SVf_IOK );
    return 0;
}

# An approximate size of a value in elements (work): a string counts its bytes, an
# array or hash its entries plus, one level deeper, the size of each string or
# container child.  Approximate by design; a hostile value is bounded by the
# allowance.  Pass the value itself (it is not copied).
sub size_of {
    my $allow = WALK_CAP;
    my $b     = B::svref_2object( \$_[0] );
    my $n     = eval { _shallow( $b, \$allow ) };
    return 0 if !$n;
    my $total = $n;
    my $r     = eval { _referent($b) };
    if ( defined $r && !( $r->FLAGS & MAGIC ) ) {
        my $cls = ref $r;
        if ( $cls eq 'B::AV' ) {
            for my $i ( 0 .. $r->FILL ) {
                last if $allow <= 0;
                $allow--;
                $total += eval { _shallow( $r->ARRAYelt($i), \$allow ) } || 0;
            }
        }
        elsif ( $cls eq 'B::HV' && $r->KEYS <= $allow ) {
            my @pairs = $r->ARRAY;
            while ( @pairs && $allow > 0 ) {
                shift @pairs;
                my $v = shift @pairs;
                $allow--;
                $total += eval { _shallow( $v, \$allow ) } || 0;
            }
        }
    }
    return $total;
}

sub _clone {
    my ( $b, $memo, $st, $depth ) = @_;
    if ( ++$st->{nodes} > 100_000 || $depth > 64 ) { $st->{failed} = 1; return undef }
    if ( ref($b) eq 'B::SPECIAL' ) {
        return $$b == 2 ? !!1 : $$b == 3 ? !!0 : undef;
    }
    my $f = $b->FLAGS;
    return undef if $f & MAGIC;    # opaque: the copy holds undef
    my $copy = ${ $b->object_2svref };    # a plain copy (no magic, no FETCH)
    return $copy if !( $f & B::SVf_ROK );
    my $r = $b->RV;
    return $copy if $r->FLAGS & MAGIC;
    my $addr = $$r;
    return $memo->{$addr} if exists $memo->{$addr};
    my $name = stash_name($r);
    if ( defined $name ) {
        # A big gets a fresh object; every other object is SHARED, not copied
        # (copying could run its code), as is text (immutable).
        return Math::BigInt::copy($copy) if $name eq 'Math::BigInt';
        return $copy;
    }
    my $cls = ref $r;
    if ( $cls eq 'B::AV' ) {
        my @new;
        $memo->{$addr} = \@new;
        for my $i ( 0 .. $r->FILL ) {
            push @new, _clone( $r->ARRAYelt($i), $memo, $st, $depth + 1 );
        }
        return \@new;
    }
    if ( $cls eq 'B::HV' ) {
        my %new;
        $memo->{$addr} = \%new;
        my @pairs = $r->ARRAY;
        while (@pairs) {
            my ( $k, $v ) = splice @pairs, 0, 2;
            $new{$k} = _clone( $v, $memo, $st, $depth + 1 );
        }
        return \%new;
    }
    if ( $cls =~ /\AB::(?:IV|NV|PV|PVIV|PVNV|PVMG|NULL)\z/ ) {
        my $inner;
        $memo->{$addr} = \$inner;
        $inner = _clone( $r, $memo, $st, $depth + 1 );
        return \$inner;
    }
    return $copy;    # code, glob, regexp ...: shared
}

# A structural copy of a value (so a defective shrinker cannot damage the original
# witness), bounded; returns ($copy, $failed).  When a bound is crossed nothing is
# shrunk (fail closed).
sub clone {
    my $st = { nodes => 0, failed => 0 };
    my $copy;
    my $ok = eval { $copy = _clone( B::svref_2object( \$_[0] ), {}, $st, 0 ); 1 };
    return ( undef, 1 ) if !$ok;
    return ( $copy, $st->{failed} );
}

# Replaces every engine sub by a read-only glob, so a property cannot `local`ise
# or reassign one (`*Pkg::sub = sub {...}`).  It cannot stop a source-level
# redefinition by `eval`: the engine is cooperative, not sandboxed.
sub seal {
    no strict 'refs';
    for my $pkg (@_) {
        for my $name ( keys %{"${pkg}::"} ) {
            my $gv = \*{"${pkg}::$name"};
            next if !defined *{$gv}{CODE};
            Internals::SvREADONLY( ${$gv}, 1 );
        }
    }
    return 1;
}

1;
