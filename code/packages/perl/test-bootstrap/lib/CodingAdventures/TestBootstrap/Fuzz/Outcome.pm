package CodingAdventures::TestBootstrap::Fuzz::Outcome;

# What one execution of the target did (VER06 section 5.1).
#
# An outcome has a `kind` (`pass`, `crash`, `timeout`, `oom`), for a crash a `crash_kind`
# (the class, reftype or a fixed word; never an error message) and its `frames`, and an
# `escaped` flag: true when the target escaped the execution contract (it tampered with
# the alarm or the engine), which counts toward the leak cap (VER06q 5.4).  Only this
# module builds one; `is` recognises them by IDENTITY (a private registry keyed by
# `refaddr`, cleaned by DESTROY), so a hash a test `exec` fabricates, or a copy, is never
# one.

use strict;
use warnings;

use Scalar::Util ();

my $refaddr = \&Scalar::Util::refaddr;
my $blessed = \&Scalar::Util::blessed;

my %MADE;    # refaddr => 1, for every live outcome built here

use overload '""' => sub {
    my ($o) = @_;
    return $o->{kind} eq 'crash' ? "#<Fuzz.Outcome crash $o->{crash_kind}>" : "#<Fuzz.Outcome $o->{kind}>";
  },
  fallback => 1;

sub DESTROY { delete $MADE{ $refaddr->( $_[0] ) } if ref $_[0] }

sub _make {
    my ( $kind, $crash_kind, $frames, $escaped ) = @_;
    my $o = bless { kind => $kind, crash_kind => $crash_kind, frames => $frames || [], escaped => $escaped ? 1 : 0 }, __PACKAGE__;
    $MADE{ $refaddr->($o) } = 1;
    return $o;
}

sub pass    { return _make( 'pass',    undef, undef, $_[0] ) }
sub timeout { return _make( 'timeout', undef, undef, $_[0] ) }
sub oom     { return _make( 'oom',     undef, undef, $_[0] ) }
sub crash   { return _make( 'crash',   $_[0], $_[1], $_[2] ) }

# True only for an outcome this module built.
sub is {
    my ($x) = @_;
    my $c = $blessed->($x);
    return ( defined $c && $c eq __PACKAGE__ && $MADE{ $refaddr->($x) } ) ? 1 : 0;
}

sub is_crash { return ( is( $_[0] ) && $_[0]{kind} eq 'crash' ) ? 1 : 0 }
sub is_pass  { return ( is( $_[0] ) && $_[0]{kind} eq 'pass' ) ? 1 : 0 }

1;
