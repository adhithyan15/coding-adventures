package CodingAdventures::LogicGates::Error;

# ============================================================================
# CodingAdventures::LogicGates::Error -- the typed rejection of logic-gates
# ============================================================================
#
# WHY A TYPED ERROR?
#
# A rejection used to be a free-text string: "logic_gates: a must be 0 or 1,
# got 'x'".  Anything that wants to know WHY a call was rejected (a test, a
# verification binding, a caller choosing a fallback) had to match that text,
# and text is not a contract: rewording a message would silently break every
# consumer.  The cross-lane contract (LG02 sections 7 and 9) therefore asks
# for a stable IDENTITY: a class plus a small closed set of codes.  Bindings
# match on the code and never on the message.
#
# CODES (a closed set; every rejection in this package maps to exactly one)
#
#   INVALID_BIT    an argument is not the bit 0 or 1
#   INVALID_ARITY  wrong number of arguments or inputs
#   INVALID_WIDTH  a counter width outside 1..65536 or not an integer
#   INVALID_CONTEXT a list-returning block was called in scalar context
#   INVALID_SHAPE  a container argument has the wrong type, length or value
#                  (state hash, bit array, direction, missing state ...)
#
# FIELDS (read-only accessors; the object has no setters.  The object itself is
# an ordinary blessed hash: `$err .= "z"` replaces the VARIABLE with a plain
# string, it never mutates the error.)
#
#   code               one of the codes above
#   param              name of the first offending parameter (or undef)
#   value_description  the BOUNDED rendering of the offending value, as made
#                      by LogicGates::_describe -- never the raw value, so an
#                      error object can never retain a huge or hostile input
#   message            the human text, ending in "\n"
#
# The object stringifies to the message, so `die $err` prints exactly what the
# old string died with and existing `like($@, qr/.../)` checks keep passing.

use strict;
use warnings;
use Scalar::Util ();

our $VERSION = '0.7.0';

use constant CODES => qw(INVALID_BIT INVALID_ARITY INVALID_WIDTH INVALID_SHAPE INVALID_CONTEXT);

use overload
    '""'     => sub { $_[0]{message} },
    'bool'   => sub { 1 },
    'eq'     => sub { "$_[0]" eq "$_[1]" },
    'ne'     => sub { "$_[0]" ne "$_[1]" },
    'cmp'    => sub { my ( $s, $o, $swap ) = @_; my $r = "$s" cmp "$o"; $swap ? -$r : $r },
    '.'      => sub {
        my ( $s, $o, $swap ) = @_;
        $swap ? "$o" . $s->{message} : $s->{message} . "$o";
    },
    '.='     => sub { my ( $s, $o ) = @_; $s->{message} . "$o" },
    'x'      => sub {
        my ( $s, $o, $swap ) = @_;
        # `3 x $err` would need the error as a repeat COUNT: that is numeric.
        die "CodingAdventures::LogicGates::Error: cannot use an error as a repeat count\n" if $swap;
        $s->{message} x $o;
    },
    '!'      => sub { 0 },
    # Copy constructor: `$b = $a; $b .= "z"` needs one.  The object is
    # immutable, so sharing it is a correct copy.
    '='      => sub { $_[0] },
    'lt'     => sub { my ( $s, $o, $w ) = @_; ( $w ? "$o" cmp "$s" : "$s" cmp "$o" ) <  0 },
    'le'     => sub { my ( $s, $o, $w ) = @_; ( $w ? "$o" cmp "$s" : "$s" cmp "$o" ) <= 0 },
    'gt'     => sub { my ( $s, $o, $w ) = @_; ( $w ? "$o" cmp "$s" : "$s" cmp "$o" ) >  0 },
    'ge'     => sub { my ( $s, $o, $w ) = @_; ( $w ? "$o" cmp "$s" : "$s" cmp "$o" ) >= 0 },
    fallback => 0;

# WHY fallback => 0 PLUS EXPLICIT STRING OPERATORS?  Interpolation and
# concatenation ("failed: $@", $@ . "y", warn/die wrapping) are the ways error
# text is normally carried along, and with no `.` overload they would throw
# 'Operation ".": no method found', REPLACING $@ and hiding the original
# error and its code.  So the string operators (. .= x eq ne lt le gt ge cmp
# "") and boolean negation (!) are all defined.  `.=` yields a plain string
# (the variable stops holding the object, as with any string append).  Numeric operators (==, +, ...) are deliberately NOT defined and
# keep failing loudly: an error is not a number, and `$err == 1` would
# otherwise silently compare something meaningless.

# new(code => ..., param => ..., value_description => ..., message => ...)
sub new {
    shift;
    my %a = @_;
    # Fields must be plain (non-reference) scalars.  A reference or overloaded
    # object would be stringified, running caller code; reject with a fixed
    # message built from constants.
    for my $f (qw(code param value_description message)) {
        _fixed_die("field '$f' must be a plain string")
            if defined $a{$f} && ref $a{$f};
    }
    my %known = map { $_ => 1 } CODES;
    _fixed_die("unknown code")
        unless defined $a{code} && $known{ $a{code} };
    my $message = defined $a{message} ? $a{message} : '';
    $message .= "\n" unless $message =~ /\n\z/;
    return bless {
        code              => $a{code},
        param             => $a{param},
        value_description => $a{value_description},
        message           => $message,
    }, __PACKAGE__;
}

# Die with a package Error built from constants only (no caller text).
sub _fixed_die {
    my ($why) = @_;
    die bless {
        code => 'INVALID_SHAPE', param => undef, value_description => undef,
        message => "CodingAdventures::LogicGates::Error: $why\n",
    }, __PACKAGE__;
}

sub code              { $_[0]{code} }
sub param             { $_[0]{param} }
sub value_description { $_[0]{value_description} }
sub message           { $_[0]{message} }

# is_invalid_bit($err) -- true ONLY for a genuine Error object whose code is
# INVALID_BIT.  Decision: the class must be EXACTLY this class (`ref`, not
# `isa`), so neither a plain hashref {code => 'INVALID_BIT'} nor a subclass or
# a foreign object that merely claims the code counts.  A binding that wants
# the stable identity gets exactly one class to trust.  (Anyone who blesses
# their own hash into this very class can of course forge it; Perl has no
# private constructors, and that is the same trust boundary as the package.)
sub is_invalid_bit {
    return 0 if tied $_[0];    # reading a tied scalar would run caller code
    my ($err) = @_;
    return 0 unless Scalar::Util::blessed($err);
    return 0 unless ref($err) eq __PACKAGE__;
    return 0 unless Scalar::Util::reftype($err) eq 'HASH';
    return 0 if tied %{$err};
    return 0 unless exists $err->{code};
    return 0 if tied $err->{code};
    my $code = $err->{code};
    return 0 if !defined $code || ref $code;
    return $code eq 'INVALID_BIT' ? 1 : 0;
}

1;
