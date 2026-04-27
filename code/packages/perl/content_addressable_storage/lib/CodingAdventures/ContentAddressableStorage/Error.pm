package CodingAdventures::ContentAddressableStorage::Error;

# ============================================================================
# CodingAdventures::ContentAddressableStorage::Error — typed exception hierarchy for CAS operations
# ============================================================================
#
# In Perl there is no built-in exception class system, but the idiomatic way
# to create typed exceptions is to bless a hashref into a package and then
# `die` with that object. Callers use `eval { ... }` to catch exceptions and
# `ref $@` to distinguish types.
#
# The exception hierarchy mirrors the Rust CasError enum:
#
#   CodingAdventures::ContentAddressableStorage::Error          — base class (never thrown directly)
#     ├── CasNotFoundError                — key was not in the store
#     ├── CasCorruptedError               — stored bytes don't hash to the key
#     ├── CasAmbiguousPrefixError         — hex prefix matches 2+ keys
#     ├── CasPrefixNotFoundError          — hex prefix matches 0 keys
#     └── CasInvalidPrefixError           — hex string is empty or not valid hex
#
# Usage pattern (throwing):
#
#   die CodingAdventures::ContentAddressableStorage::Error::CasNotFoundError->new($hex_key);
#
# Usage pattern (catching):
#
#   eval { $cas->get($key) };
#   if (ref $@ && $@->isa('CodingAdventures::ContentAddressableStorage::Error::CasNotFoundError')) {
#       say "not found: ", $@->key;
#   } elsif ($@) {
#       die $@;   # re-throw anything we don't handle
#   }

use strict;
use warnings;
use utf8;

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# Base class: CodingAdventures::ContentAddressableStorage::Error
#
# All CAS exceptions inherit from this package so callers can catch the whole
# family with a single `isa` check.
# ---------------------------------------------------------------------------

sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

# stringify() — called when the object is used in string context.
# Each subclass overrides this to produce a human-readable message.
use overload '""' => \&stringify, fallback => 1;

sub stringify {
    my ($self) = @_;
    return ref($self) . ': (unknown error)';
}

sub isa_cas_error { 1 }

# ---------------------------------------------------------------------------
# CasNotFoundError — the requested key was not in the store
#
# Fields:
#   key  — the 40-char hex string of the missing key
# ---------------------------------------------------------------------------

package CodingAdventures::ContentAddressableStorage::Error::CasNotFoundError;

use strict;
use warnings;
use utf8;

our @ISA = ('CodingAdventures::ContentAddressableStorage::Error');

# new($hex_key) — create a not-found exception for the given hex key string.
sub new {
    my ($class, $key) = @_;
    return bless { key => $key }, $class;
}

sub key { $_[0]->{key} }

sub stringify {
    my ($self) = @_;
    return "CasNotFoundError: object not found: " . $self->{key};
}

# ---------------------------------------------------------------------------
# CasCorruptedError — stored bytes do not hash to the requested key
#
# This is a data integrity violation: what the store returned does not match
# what it promised. The CAS layer detects this by re-hashing after every get.
#
# Fields:
#   key  — the 40-char hex string of the key that was requested
# ---------------------------------------------------------------------------

package CodingAdventures::ContentAddressableStorage::Error::CasCorruptedError;

use strict;
use warnings;
use utf8;

our @ISA = ('CodingAdventures::ContentAddressableStorage::Error');

sub new {
    my ($class, $key) = @_;
    return bless { key => $key }, $class;
}

sub key { $_[0]->{key} }

sub stringify {
    my ($self) = @_;
    return "CasCorruptedError: object corrupted: " . $self->{key};
}

# ---------------------------------------------------------------------------
# CasAmbiguousPrefixError — hex prefix matches two or more stored keys
#
# When a user types `git show a3f4`, git expects the abbreviation to be
# unique. If two objects share that prefix, git reports "ambiguous argument".
# We replicate that behaviour here.
#
# Fields:
#   prefix — the hex prefix string the caller supplied
# ---------------------------------------------------------------------------

package CodingAdventures::ContentAddressableStorage::Error::CasAmbiguousPrefixError;

use strict;
use warnings;
use utf8;

our @ISA = ('CodingAdventures::ContentAddressableStorage::Error');

sub new {
    my ($class, $prefix) = @_;
    return bless { prefix => $prefix }, $class;
}

sub prefix { $_[0]->{prefix} }

sub stringify {
    my ($self) = @_;
    return "CasAmbiguousPrefixError: ambiguous prefix: " . $self->{prefix};
}

# ---------------------------------------------------------------------------
# CasPrefixNotFoundError — hex prefix matches no stored keys
#
# Fields:
#   prefix — the hex prefix string the caller supplied
# ---------------------------------------------------------------------------

package CodingAdventures::ContentAddressableStorage::Error::CasPrefixNotFoundError;

use strict;
use warnings;
use utf8;

our @ISA = ('CodingAdventures::ContentAddressableStorage::Error');

sub new {
    my ($class, $prefix) = @_;
    return bless { prefix => $prefix }, $class;
}

sub prefix { $_[0]->{prefix} }

sub stringify {
    my ($self) = @_;
    return "CasPrefixNotFoundError: object not found for prefix: " . $self->{prefix};
}

# ---------------------------------------------------------------------------
# CasInvalidPrefixError — empty string or non-hex characters in prefix
#
# An empty string would match every object (not useful). Non-hex characters
# cannot correspond to any stored key. Both are programmer errors that deserve
# their own exception type.
#
# Fields:
#   prefix — the invalid string the caller supplied
# ---------------------------------------------------------------------------

package CodingAdventures::ContentAddressableStorage::Error::CasInvalidPrefixError;

use strict;
use warnings;
use utf8;

our @ISA = ('CodingAdventures::ContentAddressableStorage::Error');

sub new {
    my ($class, $prefix) = @_;
    return bless { prefix => $prefix }, $class;
}

sub prefix { $_[0]->{prefix} }

sub stringify {
    my ($self) = @_;
    return "CasInvalidPrefixError: invalid hex prefix: " . $self->{prefix};
}

1;
