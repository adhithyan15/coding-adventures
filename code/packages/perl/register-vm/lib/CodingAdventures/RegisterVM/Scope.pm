package CodingAdventures::RegisterVM::Scope;

# ============================================================================
# CodingAdventures::RegisterVM::Scope — Lexical scope chain (context)
# ============================================================================
#
# # What is a Scope Chain?
#
# In languages like JavaScript, Python, and Ruby, variables declared inside a
# function are "local" to that function — they cannot be seen from outside.
# But an inner function CAN see the variables of the outer function that
# created it. This is called a *closure*.
#
# The runtime implements closures using a *scope chain* (also called a
# "context chain" in V8 terminology):
#
#   Global scope: { x: 10 }
#       ↑ parent
#   Outer function scope: { y: 20 }
#       ↑ parent
#   Inner function scope: { z: 30 }   ← current
#
# When the inner function reads `y`, the VM:
#   1. Looks in the inner scope — not found.
#   2. Follows the parent link to the outer scope — found! Return 20.
#
# In our VM, context lookup is compiled to LDA_CONTEXT_SLOT with a precomputed
# depth (how many parent links to follow) and index (which slot in that scope's
# array). This avoids hash lookups at runtime.
#
# ## Scope Representation
#
# Each scope is a hashref:
#   {
#     parent => $parent_scope_or_undef,
#     slots  => [ $val0, $val1, ... ],
#   }
#
# 'slots' is an arrayref. The compiler assigns each variable in a scope a
# numeric index rather than storing names at runtime (names are only in the
# debug info / names array of the CodeObject).
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.1.0';

# ----------------------------------------------------------------------------
# new($parent) → $scope
#
# Create a new lexical scope. Pass undef for the global/outermost scope.
#
# @param $parent   parent scope hashref, or undef
# @return hashref { parent => ..., slots => [] }
# ----------------------------------------------------------------------------
sub new {
    my ($class, $parent) = @_;
    return bless {
        parent => $parent,    # undef at the global level
        slots  => [],         # variable values indexed by slot number
    }, $class;
}

# ----------------------------------------------------------------------------
# get($scope, $depth, $idx) → $value
#
# Read a variable from the scope chain.
#
# Walk `depth` parent links from the given scope, then return slots[$idx].
#
# @param $scope   starting scope (current frame's context)
# @param $depth   number of parent links to follow (0 = current scope)
# @param $idx     slot index within the target scope
# @return the stored value (may be undef)
# ----------------------------------------------------------------------------
sub get {
    my ($scope, $depth, $idx) = @_;

    # Walk up the parent chain $depth times.
    my $target = $scope;
    for my $i (1 .. $depth) {
        die "Scope::get: ran out of parent scopes at depth $i (requested $depth)"
            unless defined $target->{parent};
        $target = $target->{parent};
    }

    return $target->{slots}[$idx];
}

# ----------------------------------------------------------------------------
# set($scope, $depth, $idx, $value)
#
# Write a variable into the scope chain.
#
# @param $scope   starting scope
# @param $depth   parent links to follow
# @param $idx     slot index
# @param $value   value to store
# ----------------------------------------------------------------------------
sub set {
    my ($scope, $depth, $idx, $value) = @_;

    my $target = $scope;
    for my $i (1 .. $depth) {
        die "Scope::set: ran out of parent scopes at depth $i (requested $depth)"
            unless defined $target->{parent};
        $target = $target->{parent};
    }

    $target->{slots}[$idx] = $value;
    return;
}

1;

__END__

=head1 NAME

CodingAdventures::RegisterVM::Scope - Lexical scope chain for the register VM

=head1 SYNOPSIS

    use CodingAdventures::RegisterVM::Scope;

    my $global = CodingAdventures::RegisterVM::Scope->new(undef);
    my $local  = CodingAdventures::RegisterVM::Scope->new($global);

    CodingAdventures::RegisterVM::Scope::set($local, 0, 0, 42);
    my $v = CodingAdventures::RegisterVM::Scope::get($local, 0, 0);  # 42

    # Access a variable in the parent scope:
    CodingAdventures::RegisterVM::Scope::set($global, 0, 0, 99);
    $v = CodingAdventures::RegisterVM::Scope::get($local, 1, 0);     # 99

=head1 DESCRIPTION

Implements a simple linked-list scope chain used by context-slot opcodes.
Each scope holds a slot array and a parent reference.

=cut
