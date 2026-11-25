package CodingAdventures::TestBootstrap::Budget;

# The budget of a shrink, as a TREE of scopes (VER05p section 5, ported from the
# Java engine's VER05e 4.1 via the Haskell and Lua lanes).
#
# Why a tree?  A property may itself call `Runner::check`, a custom shrinker may
# call `Gen::shrink`, a filter predicate may do either.  If each of those got a
# fresh budget the work would multiply with the nesting depth.  Here every public
# entry point that runs user code opens a CHILD of the scope it is given (or of the
# ambient scope) with its own local cap, and a charge walks from the node up to the
# outermost root: every node on the path must have room, then every counter is
# incremented, all or nothing.  Whatever the nesting, the total work of one
# top-level check is capped by the outermost root.
#
# Two things are charged: ATTEMPTS (one per property evaluation) and WORK
# (elements copied, at least 1 per charge).
#
# THE CARRIER IS PERL'S DYNAMIC SCOPING (spec 5.1).  `$CURRENT` is the innermost
# open scope; a public entry point does
#
#     my $own = Budget::open_child($parent, $max_steps);   # a guard (a lexical)
#     local $Budget::CURRENT = $own->scope;
#
# and `local` undoes the assignment on EVERY exit, `die` included, so there is no
# `finally` to forget and no stale ambient scope.  A property that calls
# `Runner::check` (or a custom shrinker that calls `Gen::shrink`) without any
# `parent` therefore charges the enclosing tree instead of opening a fresh root.
# An explicit `parent` overrides the ambient scope.
#
# A closed node, or a node with a closed ANCESTOR, is SPENT: it refuses every
# charge.  A stale handle is never a way to obtain a fresh root.
#
# The budget is COOPERATIVE: in-process code can write `$scope->{closed} = 0` or
# `local $Budget::CURRENT` itself.  It bounds honest nesting, not a hostile holder.

use strict;
use warnings;

use Scalar::Util ();

use CodingAdventures::TestBootstrap::Int    ();
use CodingAdventures::TestBootstrap::Limits ();
use CodingAdventures::TestBootstrap::Hooks  ();

use constant MAX_DEPTH         => 64;    # most nested owning scopes; deeper scopes join
use constant MAX_CHECK_NESTING => 64;    # most checks nested inside each other

our $CURRENT;    # the innermost open scope (dynamic scope), or undef

my $blessed = \&Scalar::Util::blessed;

sub current { return $CURRENT }

sub is_scope {
    my $c = $blessed->( $_[0] );
    return ( defined $c && $c eq __PACKAGE__ ) ? 1 : 0;
}

# The node a handle stands for: a joined handle aliases its owner.
sub _owner { return exists $_[0]{target} ? $_[0]{target} : $_[0] }

sub _new_node {
    my ( $parent, $max_attempts, $max_work, $is_check ) = @_;
    my $node = bless {
        parent       => $parent,
        depth_       => $parent ? $parent->{depth_} + 1 : 0,
        checks_      => ( $parent ? $parent->{checks_} : 0 ) + ( $is_check ? 1 : 0 ),
        max_attempts => $max_attempts,
        max_work     => $max_work,
        attempts_    => 0,
        work_        => 0,
        closed       => 0,
        refused_     => 0,    # meaningful on the root only
    }, __PACKAGE__;
    $node->{root} = $parent ? $parent->{root} : $node;
    # A root's `root` is itself: weak always, or every top-level check leaks its node.
    Scalar::Util::weaken( $node->{root} );
    return $node;
}

# A joined handle: charges and queries go to the node it aliases; closing it is a
# no-op.  An internal combinator uses it so it never owns a scope.
sub _join { return bless { target => $_[0] }, __PACKAGE__ }

sub _default_work {
    my $w = $CodingAdventures::TestBootstrap::Hooks::H{default_max_work};
    return defined $w ? $w : CodingAdventures::TestBootstrap::Limits::MAX_SHRINK_WORK;
}

# The guard owns a node: when the lexical holding it goes away (a return, a `die`,
# an unwinding) the node is closed.  A joined handle's close does nothing.
package CodingAdventures::TestBootstrap::Budget::Guard;

sub scope { return $_[0]{scope} }

sub DESTROY {
    my ($self) = @_;
    local ( $@, $!, $? );    # a close during unwinding must not clobber an exception
    eval { $self->{scope}->close; 1 };
}

package CodingAdventures::TestBootstrap::Budget;

sub _guard { return bless { scope => $_[0] }, 'CodingAdventures::TestBootstrap::Budget::Guard' }

# A fresh root scope (an outermost budget) with explicit caps.
sub root {
    my ( $max_attempts, $max_work ) = @_;
    return _guard( _new_node( undef, $max_attempts, defined $max_work ? $max_work : _default_work(), 0 ) );
}

# A child of `$parent` with explicit caps.
sub child {
    my ( $parent, $max_attempts, $max_work ) = @_;
    return _guard( _new_node( _owner($parent), $max_attempts, defined $max_work ? $max_work : CodingAdventures::TestBootstrap::Limits::MAX_SHRINK_WORK, 0 ) );
}

# The scope of a public entry point that runs user code: a CHILD of `$parent` (the
# ambient scope when `$parent` is undef) with a fresh local cap, or a fresh root
# when neither exists.  Past MAX_DEPTH it joins instead of nesting.  Returns a
# guard; keep it in a lexical.
sub open_child {
    my ( $parent, $max_steps, $max_work ) = @_;
    $parent = $CURRENT if !defined $parent;
    my $cap = CodingAdventures::TestBootstrap::Limits::attempt_budget($max_steps);
    return _guard( _new_node( undef, $cap, defined $max_work ? $max_work : _default_work(), 0 ) ) if !defined $parent;
    my $p = _owner($parent);
    return _guard( _join($p) ) if $p->{depth_} + 1 >= MAX_DEPTH;
    return _guard( _new_node( $p, $cap, defined $max_work ? $max_work : CodingAdventures::TestBootstrap::Limits::MAX_SHRINK_WORK, 0 ) );
}

# The scope of a whole `Runner::check` (case loop included): a root without a
# parent, a child otherwise.  Its attempt cap is num_tests plus the shrink attempt
# cap, so a nested check's cases and its shrink are all charged up the chain.
sub open_check {
    my ( $parent, $num_tests, $max_steps ) = @_;
    my $cap = $num_tests + CodingAdventures::TestBootstrap::Limits::attempt_budget($max_steps);
    return _guard( _new_node( undef, $cap, _default_work(), 1 ) ) if !defined $parent;
    my $p = _owner($parent);
    return _guard( _join($p) ) if $p->{depth_} + 1 >= MAX_DEPTH;
    return _guard( _new_node( $p, $cap, CodingAdventures::TestBootstrap::Limits::MAX_SHRINK_WORK, 1 ) );
}

# The same node, as a handle that does not own it (closing it does nothing).
sub join_scope { return _join( _owner( $_[0] ) ) }

# ---------------------------------------------------------------- queries

sub depth    { return _owner( $_[0] )->{depth_} }
sub checks   { return _owner( $_[0] )->{checks_} }    # enclosing checks, this one included
sub attempts { return _owner( $_[0] )->{attempts_} }
sub work     { return _owner( $_[0] )->{work_} }
sub nested   { return defined _owner( $_[0] )->{parent} ? 1 : 0 }
sub refused  { return _owner( $_[0] )->{root}{refused_} ? 1 : 0 }

# Records that something in this tree could not be afforded (a refused nested
# check, say), so a would-be pass becomes `property-exhausted`.
sub mark_refused { _owner( $_[0] )->{root}{refused_} = 1; return }

# True once this node or any ancestor is closed (the handle is SPENT).
sub is_spent {
    my $node = _owner( $_[0] );
    while ($node) {
        return 1 if $node->{closed};
        $node = $node->{parent};
    }
    return 0;
}

# True while at least one more attempt of one element could be afforded everywhere
# on the path.
sub room {
    my $node = _owner( $_[0] );
    while ($node) {
        return 0 if $node->{closed} || $node->{attempts_} >= $node->{max_attempts} || $node->{work_} >= $node->{max_work};
        $node = $node->{parent};
    }
    return 1;
}

# --------------------------------------------------------------- charging

# Reserve `$attempt_cost` attempts and `$elements` work on every node of the path,
# all or nothing.  A cost that is not a non-negative native integer is refused
# (fail closed); zero counts as one.  `cost > max - used` cannot wrap: used <= max.
sub _take {
    my ( $self, $elements, $attempt_cost ) = @_;
    my $start = _owner($self);
    my $root  = $start->{root};
    if ( !CodingAdventures::TestBootstrap::Int::is($elements) || CodingAdventures::TestBootstrap::Int::is_big($elements) || $elements < 0 ) {
        $root->{refused_} = 1;
        return 0;
    }
    my $cost = $elements < 1 ? 1 : $elements;
    my $node = $start;
    while ($node) {
        if ( $node->{closed} || $node->{attempts_} >= $node->{max_attempts} || $cost > $node->{max_work} - $node->{work_} ) {
            $root->{refused_} = 1;
            return 0;
        }
        $node = $node->{parent};
    }
    $node = $start;
    while ($node) {
        $node->{attempts_} += $attempt_cost;
        $node->{work_}     += $cost;
        $node = $node->{parent};
    }
    return 1;
}

# Charges one attempt over a candidate of `$elements` elements (at least 1) to this
# node and every ancestor.  Returns false, charging nothing, when any of them cannot
# afford it: callers must check this BEFORE building the candidate.
sub charge { return _take( $_[0], $_[1], 1 ) }

# Charges a copy that is not itself a property evaluation: work only.
sub charge_copy { return _take( $_[0], $_[1], 0 ) }

# Wraps a falsifies function so each evaluation is charged one attempt first
# (false once refused).
sub charged {
    my ( $self, $falsifies ) = @_;
    return sub { return _take( $self, 1, 1 ) && $falsifies->( $_[0] ) };
}

# Ends an owning scope; a joined handle does nothing.
sub close {
    return if exists $_[0]{target};
    $_[0]{closed} = 1;
    return;
}

1;
