package CodingAdventures::RegisterVM::Feedback;

# ============================================================================
# CodingAdventures::RegisterVM::Feedback — Feedback-slot state machine
# ============================================================================
#
# # Why Feedback?
#
# Ahead-of-time compilers (like C's gcc) know all types at compile time.
# Dynamic language runtimes (JavaScript, Ruby, Python) don't — types are
# discovered at runtime. So how can they generate fast machine code?
#
# The answer is *adaptive optimisation*:
#
#   1. Run in an interpreter (slow but type-agnostic).
#   2. Track which types actually appear at each operation site.
#   3. Speculate that future runs will use the same types.
#   4. JIT-compile fast machine code for those specific types.
#   5. If the speculation is wrong (new type appears), fall back to slow path.
#
# V8 Ignition uses "feedback vectors" — one per function — with one slot per
# interesting operation (property access, function call, binary op, etc.).
# Each slot transitions through states as it observes more type combinations:
#
#   uninitialized → monomorphic → polymorphic → megamorphic
#
# ## State Meanings
#
#   uninitialized  No operation has executed through this slot yet.
#
#   monomorphic    Only ONE distinct type combination has been seen.
#                  A JIT can generate a single fast inline cache (IC) check.
#
#   polymorphic    2–4 distinct type combinations have been seen.
#                  A JIT generates a small dispatch table (still fast).
#
#   megamorphic    5+ distinct type combinations have been seen.
#                  Abandon specialisation — fall back to a generic slow path.
#
# ## Type-Pair Strings
#
# For binary operations we record the type of BOTH operands as a string like
# "int:int" or "float:string". This lets the JIT check "are both still ints?"
# with a single comparison.
#
# Type names we use:
#   "int"      — Perl integer (looks_like_number && integer)
#   "float"    — Perl float
#   "string"   — Perl string
#   "bool"     — 1 or ''
#   "undef"    — undef
#   "object"   — hashref with __type eq 'object'
#   "function" — hashref with __type eq 'function'
#   "ref"      — any other reference
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.1.0';

# Maximum distinct type-pairs before a slot goes megamorphic.
# V8 uses 4 as the polymorphic limit.
use constant POLYMORPHIC_LIMIT => 4;

# ----------------------------------------------------------------------------
# make() → $slot
#
# Create a brand-new feedback slot in the "uninitialized" state.
# Call this once per slot in a function's feedback vector.
#
# @return  hashref { kind => 'uninitialized' }
# ----------------------------------------------------------------------------
sub make {
    return { kind => 'uninitialized' };
}

# ----------------------------------------------------------------------------
# record($slot, $type_pair_string) → $slot (mutated in-place)
#
# Advance the feedback slot state machine by recording a new type-pair
# observation. The slot is mutated in-place AND returned for convenience.
#
# State machine transitions:
#
#   uninitialized → monomorphic   (first observation)
#   monomorphic   → polymorphic   (second distinct pair)
#   polymorphic   → megamorphic   (>POLYMORPHIC_LIMIT distinct pairs)
#   megamorphic   → megamorphic   (absorbing state — no way back)
#
# Deduplication: if the same pair is observed again, no transition occurs.
# This is crucial for correctness — a tight loop running "int + int" a
# million times should stay monomorphic, not go megamorphic.
#
# @param $slot           hashref (mutated in place)
# @param $type_pair      string like "int:float"
# @return $slot
# ----------------------------------------------------------------------------
sub record {
    my ($slot, $type_pair) = @_;

    # Megamorphic is an absorbing state — nothing changes it.
    return $slot if $slot->{kind} eq 'megamorphic';

    if ($slot->{kind} eq 'uninitialized') {
        # First ever observation: become monomorphic with this one type-pair.
        $slot->{kind}  = 'monomorphic';
        $slot->{types} = [$type_pair];
        return $slot;
    }

    # We are in monomorphic or polymorphic state.
    # Check whether this type-pair has been seen before (deduplication).
    my %seen = map { $_ => 1 } @{ $slot->{types} };
    return $slot if $seen{$type_pair};    # already recorded — no transition

    # New distinct pair: add it.
    push @{ $slot->{types} }, $type_pair;
    my $count = scalar @{ $slot->{types} };

    if ($count > POLYMORPHIC_LIMIT) {
        # Too many distinct types — give up on specialisation.
        $slot->{kind}  = 'megamorphic';
        delete $slot->{types};    # free the list; not needed in mega state
    } elsif ($count == 2) {
        # Second distinct pair: promote from mono to poly.
        $slot->{kind} = 'polymorphic';
    }
    # else count is 3 or 4: remain polymorphic

    return $slot;
}

# ----------------------------------------------------------------------------
# type_of($value) → $type_string
#
# Classify a Perl scalar into one of our type-name strings.
# Used to build type-pair strings for feedback recording.
#
# This is NOT the same as Perl's ref() — we distinguish numbers from strings
# and check for our VM object tags.
#
# @param $value   any Perl scalar
# @return string
# ----------------------------------------------------------------------------
sub type_of {
    my ($value) = @_;

    return 'undef' unless defined $value;

    # Check for VM object/function types first (they are hashrefs with a tag).
    if (ref($value) eq 'HASH') {
        my $t = $value->{__type} // '';
        return 'object'   if $t eq 'object';
        return 'function' if $t eq 'function';
        return 'ref';     # some other hashref
    }

    return 'ref' if ref($value);    # arrayref, coderef, etc.

    # Distinguish booleans (our encoding: 1 or '') from numbers and strings.
    # We treat the exact strings '1' and '' as booleans only when they came
    # from a boolean operation. Since we can't tag plain scalars, we use a
    # heuristic: if it's '' it's bool/false, if it's 1 and looks like a number
    # from a boolean context we still just say 'bool'.  In practice the VM
    # writes 1 or '' for boolean results, and those collide with numbers.
    # We resolve this pragmatically: check for '' first (undef would have been
    # caught above), then numeric, then string.

    if (!defined($value) || $value eq '') {
        return 'bool';    # false boolean
    }

    # Use Perl's built-in looks_like_number heuristic.
    # We import it lazily to avoid a hard dep on Scalar::Util at load time.
    {
        no warnings 'numeric';
        if ($value =~ /\A[+-]?[0-9]+\z/) {
            return 'int';
        }
        if ($value =~ /\A[+-]?(?:[0-9]*\.[0-9]+|[0-9]+\.[0-9]*)(?:[eE][+-]?[0-9]+)?\z/) {
            return 'float';
        }
    }

    return 'string';
}

# ----------------------------------------------------------------------------
# type_pair($left, $right) → $string
#
# Convenience: build the "type1:type2" string for a binary operation.
#
# @param $left   left operand value
# @param $right  right operand value
# @return string like "int:int"
# ----------------------------------------------------------------------------
sub type_pair {
    my ($left, $right) = @_;
    return type_of($left) . ':' . type_of($right);
}

1;

__END__

=head1 NAME

CodingAdventures::RegisterVM::Feedback - Feedback-slot state machine

=head1 SYNOPSIS

    use CodingAdventures::RegisterVM::Feedback;

    my $slot = CodingAdventures::RegisterVM::Feedback::make();
    CodingAdventures::RegisterVM::Feedback::record($slot, 'int:int');
    # $slot->{kind} eq 'monomorphic'

    CodingAdventures::RegisterVM::Feedback::record($slot, 'float:int');
    # $slot->{kind} eq 'polymorphic'

=head1 DESCRIPTION

Implements the four-state feedback slot used by the register VM to track
type information at operation sites. States: uninitialized, monomorphic,
polymorphic, megamorphic.

=cut
