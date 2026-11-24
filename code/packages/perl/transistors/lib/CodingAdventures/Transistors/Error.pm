package CodingAdventures::Transistors::Error;

# ============================================================================
# CodingAdventures::Transistors::Error -- the typed rejection of transistors
# ============================================================================
#
# WHY A TYPED ERROR?
#
# A rejection used to be a free-text string: "Transistors: a must be 0 or 1".
# Anything that wants to know WHY a call was rejected (a test, a verification
# binding, a caller choosing a fallback) had to match that text, and text is
# not a contract: rewording a message would silently break every consumer.
# So each rejection carries a stable IDENTITY: a class plus a small closed set
# of codes.  Consumers match on the code and never on the message.
#
# CODES (a closed set; every rejection in this package maps to exactly one)
#
#   INVALID_BIT     a digital argument is not the bit 0 or 1
#   INVALID_ARITY   wrong number of arguments
#   INVALID_NUMBER  an analog argument (vgs, vth, ib, beta) is not a finite
#                   decimal number
#
# FIELDS (all read-only)
#
#   code               one of the codes above
#   param              name of the offending parameter (undef for arity)
#   value_description  BOUNDED rendering of the offending value -- never the
#                      raw value, so an error can never retain a huge or
#                      hostile input (and never triggers overloading)
#   message            the human text, ending in "\n"
#
# The object stringifies to the message, so `die $err` prints exactly what the
# old string died with and existing `like($@, qr/.../)` checks keep passing.

use strict;
use warnings;
use Scalar::Util ();

our $VERSION = '0.03';

use constant CODES => qw(INVALID_BIT INVALID_ARITY INVALID_NUMBER);

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
    'lt'     => sub { my ( $s, $o, $swap ) = @_; ( $swap ? "$o" lt "$s" : "$s" lt "$o" ) ? 1 : 0 },
    'le'     => sub { my ( $s, $o, $swap ) = @_; ( $swap ? "$o" le "$s" : "$s" le "$o" ) ? 1 : 0 },
    'gt'     => sub { my ( $s, $o, $swap ) = @_; ( $swap ? "$o" gt "$s" : "$s" gt "$o" ) ? 1 : 0 },
    'ge'     => sub { my ( $s, $o, $swap ) = @_; ( $swap ? "$o" ge "$s" : "$s" ge "$o" ) ? 1 : 0 },
    '!'      => sub { 0 },
    # Copy constructor: Perl needs one before a mutator such as `.=` runs on a
    # shared reference.  The object is immutable, so sharing it is fine.
    '='      => sub { $_[0] },
    'x'      => sub {
        my ( $s, $o, $swap ) = @_;
        die "CodingAdventures::Transistors::Error: an error object is not a repeat count\n"
            if $swap;
        $s->{message} x $o;
    },
    fallback => 0;

# WHY fallback => 0 PLUS EXPLICIT STRING OPERATORS?  Interpolation and
# concatenation ("failed: $@", warn/die wrapping) are how error text is
# normally carried along; with no `.` overload they would throw 'Operation
# ".": no method found', REPLACING $@ and hiding the original code.  So the
# string operators (. .= x eq ne cmp lt le gt ge "") are all defined, plus
# `!` (always false: an error object is a true value, so `if (!$@)` works).
# `.=` yields a plain STRING (message . rhs), not an Error: appending text
# deliberately ends the object's identity.  `"a" x $err` dies (the object is
# not a repeat count).  Numeric operators (==, +, -, *, /, <=>, 0+) are
# deliberately NOT defined and keep failing loudly: an error is not a number.
#
# MUTABILITY: the accessors are read-only, but the fields live in a plain
# hash and are not enforced read-only; code that reaches into the hash, or
# blesses its own hash into this very class, can alter or forge an error.
# Perl has no private state, so that is the trust boundary of the package.

# new(code => ..., param => ..., value_description => ..., message => ...)
sub new {
    my ( $class, %a ) = @_;
    my %known = map { $_ => 1 } CODES;
    die "CodingAdventures::Transistors::Error: unknown code '"
        . ( defined $a{code} ? $a{code} : 'undef' ) . "'\n"
        unless defined $a{code} && $known{ $a{code} };
    my $message = defined $a{message} ? $a{message} : '';
    $message .= "\n" unless $message =~ /\n\z/;
    return bless {
        code              => $a{code},
        param             => $a{param},
        value_description => $a{value_description},
        message           => $message,
    }, $class;
}

sub code              { $_[0]{code} }
sub param             { $_[0]{param} }
sub value_description { $_[0]{value_description} }
sub message           { $_[0]{message} }

# describe($value) -- render a rejected value without triggering overloading,
# undef warnings, or copying megabytes of input.  Cut BEFORE copying: substr
# and length on the original scalar never build a full copy.
sub describe {
    my ($value) = @_;
    return 'undef' if !defined $value;
    return 'a ' . Scalar::Util::reftype($value) . ' reference' if ref $value;
    my $long = length($value) > 32;
    my $text = substr( $value, 0, 32 );
    $text .= '...' if $long;
    $text =~ s/([^\x20-\x7e])/sprintf( '\\x%02x', ord $1 )/ge;
    return "'$text'";
}

# is_invalid_bit($err) -- true ONLY for a genuine Error object whose code is
# INVALID_BIT.  The class must be EXACTLY this class (`ref`, not `isa`), so
# neither a plain hashref {code => 'INVALID_BIT'} nor a subclass nor a foreign
# object that merely claims the code counts.  (Anyone who blesses their own
# hash into this very class can forge it; Perl has no private constructors,
# and that is the same trust boundary as the package.)
sub is_invalid_bit {
    my ($err) = @_;
    return 0 unless Scalar::Util::blessed($err);
    return 0 unless ref($err) eq __PACKAGE__;
    return ( defined $err->{code} && $err->{code} eq 'INVALID_BIT' ) ? 1 : 0;
}

1;
