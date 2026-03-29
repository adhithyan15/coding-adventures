package CodingAdventures::StateMachine;

# ============================================================================
# CodingAdventures::StateMachine — Finite State Machine in Pure Perl
# ============================================================================
#
# This module implements a Finite State Machine (FSM), also called a
# Deterministic Finite Automaton (DFA). It is the simplest model of
# computation and the foundation of all higher-level computing theory.
#
# # What is a State Machine?
#
# Every state machine — whether a traffic light controller, a coin-operated
# turnstile, or a network protocol handler — is built from the same concepts:
#
#   State      — where the machine is right now (e.g., "locked", "idle")
#   Event      — what input the machine just received (e.g., "coin", "timeout")
#   Transition — the rule "in state X, on event Y, go to state Z"
#   Action     — optional callback that fires when a transition occurs
#   Guard      — optional condition that must be true for a transition to fire
#
# # Example: A Coin-Operated Turnstile
#
#   States:      locked, unlocked
#   Events:      coin, push
#   Transitions:
#     (locked,   coin) → unlocked    # insert a coin to unlock
#     (locked,   push) → locked      # push without coin — stays locked
#     (unlocked, push) → locked      # go through — relocks
#     (unlocked, coin) → unlocked    # extra coin — stays unlocked
#   Initial:     locked
#   Accepting:   unlocked
#
# # Formal Definition (5-tuple)
#
# A DFA is formally defined as (Q, Σ, δ, q0, F):
#
#   Q   = finite set of states
#   Σ   = finite set of input symbols (the "alphabet")
#   δ   = transition function: Q × Σ → Q
#   q0  = the initial state (q0 ∈ Q)
#   F   = set of accepting/final states (F ⊆ Q)
#
# # API Design
#
# This implementation uses a builder-style API that feels natural for Perl:
#
#   my $sm = CodingAdventures::StateMachine->new();
#   $sm->add_state('locked', { initial => 1 });
#   $sm->add_state('unlocked', { accepting => 1 });
#   $sm->add_transition('locked', 'coin', 'unlocked');
#   $sm->add_transition('unlocked', 'push', 'locked');
#   $sm->process('coin');     # → 'unlocked'
#   $sm->is_accepting();      # → 1
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ----------------------------------------------------------------------------
# new() → StateMachine instance
#
# Creates a new, empty state machine. You must add states and transitions
# before calling process().
#
# Internal representation:
#   states      — hashref: name → { accepting => 0|1, entry => coderef|undef,
#                                   exit => coderef|undef }
#   transitions — hashref: "from\0event" → { to => name, action => coderef|undef,
#                                             guard  => coderef|undef }
#   initial     — string (state name) or undef
#   current     — string (current state name) or undef
#
# @return blessed hashref
# ----------------------------------------------------------------------------
sub new {
    my ($class) = @_;
    return bless {
        states      => {},     # state_name → { accepting, entry, exit }
        transitions => {},     # "from\0event" → { to, action, guard }
        initial     => undef,
        current     => undef,
    }, $class;
}

# ----------------------------------------------------------------------------
# add_state($name, \%opts) → void
#
# Register a state with the machine.
#
# Options hashref (all optional):
#   initial   => 1         # set this as the initial (and current) state
#   accepting => 1         # mark as an accepting/final state
#   entry     => sub { }   # callback fired when the machine ENTERS this state
#   exit      => sub { }   # callback fired when the machine EXITS this state
#
# Entry and exit actions implement the "Mealy/Moore model" distinction.
# In a Moore machine, outputs depend only on state (entry/exit actions).
# In a Mealy machine, outputs depend on both state and event (transition actions).
# This implementation supports both patterns.
#
# @param $name  String name for the state
# @param $opts  Optional hashref of state options
# ----------------------------------------------------------------------------
sub add_state {
    my ($self, $name, $opts) = @_;
    $opts //= {};

    die "StateMachine: state '$name' already defined"
        if exists $self->{states}{$name};

    $self->{states}{$name} = {
        accepting => $opts->{accepting} ? 1 : 0,
        entry     => $opts->{entry},
        exit      => $opts->{exit},
    };

    # If this state is marked as initial, record it and set current state
    if ($opts->{initial}) {
        $self->{initial} = $name;
        $self->{current} = $name;
    }
}

# ----------------------------------------------------------------------------
# set_initial($state) → void
#
# Set the initial state of the machine. Also sets the current state.
# Useful when you want to define states first without marking one as initial,
# then set the initial state separately.
#
# @param $state  Name of the state to use as initial
# ----------------------------------------------------------------------------
sub set_initial {
    my ($self, $state) = @_;
    die "StateMachine: state '$state' not defined"
        unless exists $self->{states}{$state};
    $self->{initial} = $state;
    $self->{current} = $state;
}

# ----------------------------------------------------------------------------
# add_transition($from, $event, $to, \%opts) → void
#
# Register a transition rule: "in state $from, on event $event, go to $to".
#
# Options hashref (all optional):
#   action => sub { my ($from, $event, $to) = @_; ... }
#             Called after the transition fires. Receives the source state,
#             event name, and target state.
#   guard  => sub { my ($from, $event) = @_; return 1 or 0; }
#             Called BEFORE the transition fires. If it returns false, the
#             transition is skipped (the machine stays in the current state).
#
# Guards enable conditional transitions — the same event can have different
# outcomes depending on runtime conditions, which is useful for modelling
# complex real-world systems.
#
# @param $from   Source state name
# @param $event  Event name (input symbol)
# @param $to     Target state name
# @param $opts   Optional hashref
# ----------------------------------------------------------------------------
sub add_transition {
    my ($self, $from, $event, $to, $opts) = @_;
    $opts //= {};

    die "StateMachine: source state '$from' not defined"
        unless exists $self->{states}{$from};
    die "StateMachine: target state '$to' not defined"
        unless exists $self->{states}{$to};

    my $key = "$from\0$event";
    $self->{transitions}{$key} = {
        to     => $to,
        action => $opts->{action},
        guard  => $opts->{guard},
    };
}

# ----------------------------------------------------------------------------
# process($event) → $new_state or undef
#
# Feed one event into the state machine.
#
# Algorithm:
#   1. Look up the transition for (current_state, event).
#   2. If no transition exists, return undef (machine "rejects" the event).
#   3. Evaluate the guard condition (if any). If false, return current state.
#   4. Fire the exit action of the current state (if any).
#   5. Fire the transition action (if any).
#   6. Move to the new state.
#   7. Fire the entry action of the new state (if any).
#   8. Return the new current state.
#
# Returning undef for missing transitions implements the "implicit error state"
# pattern. Some designs prefer to raise an exception; we return undef so that
# callers can choose how to handle unrecognized events.
#
# @param $event  The input event (string)
# @return The new current state name, or undef if no transition applies
# ----------------------------------------------------------------------------
sub process {
    my ($self, $event) = @_;

    die "StateMachine: no initial state set — call set_initial() first"
        unless defined $self->{current};

    my $from = $self->{current};
    my $key  = "$from\0$event";
    my $trans = $self->{transitions}{$key};

    # No transition defined for (state, event) → stay put, return undef
    return undef unless defined $trans;

    # Evaluate guard condition (if present)
    if (defined $trans->{guard}) {
        my $allowed = $trans->{guard}->($from, $event);
        return $self->{current} unless $allowed;   # guard blocked transition
    }

    my $to = $trans->{to};

    # Fire exit action for the current (departing) state
    my $exit_action = $self->{states}{$from}{exit};
    $exit_action->($from, $event, $to) if defined $exit_action;

    # Fire transition action
    my $action = $trans->{action};
    $action->($from, $event, $to) if defined $action;

    # Perform the state change
    $self->{current} = $to;

    # Fire entry action for the new state
    my $entry_action = $self->{states}{$to}{entry};
    $entry_action->($from, $event, $to) if defined $entry_action;

    return $to;
}

# ----------------------------------------------------------------------------
# current_state() → $state_name or undef
#
# Return the name of the state the machine is currently in.
#
# @return String state name, or undef if no initial state has been set
# ----------------------------------------------------------------------------
sub current_state {
    my ($self) = @_;
    return $self->{current};
}

# ----------------------------------------------------------------------------
# is_accepting() → 1 or 0
#
# Return 1 if the machine is currently in an accepting/final state.
#
# In formal automata theory, a DFA *accepts* an input string if it ends in
# an accepting state. Here, we can query this at any time.
#
# @return 1 if current state is accepting, 0 otherwise
# ----------------------------------------------------------------------------
sub is_accepting {
    my ($self) = @_;
    return 0 unless defined $self->{current};
    return $self->{states}{ $self->{current} }{accepting} ? 1 : 0;
}

# ----------------------------------------------------------------------------
# reset() → void
#
# Reset the machine to its initial state.
#
# After reset, the machine behaves as if it was just created — it's in the
# initial state and ready to process a new input sequence.
# ----------------------------------------------------------------------------
sub reset {
    my ($self) = @_;
    die "StateMachine: no initial state defined — cannot reset"
        unless defined $self->{initial};
    $self->{current} = $self->{initial};
}

# ----------------------------------------------------------------------------
# states() → @names
#
# Return a sorted list of all defined state names.
#
# @return list of strings
# ----------------------------------------------------------------------------
sub states {
    my ($self) = @_;
    return sort keys %{ $self->{states} };
}

1;

__END__

=head1 NAME

CodingAdventures::StateMachine - Finite State Machine in Pure Perl

=head1 SYNOPSIS

    use CodingAdventures::StateMachine;

    my $sm = CodingAdventures::StateMachine->new();
    $sm->add_state('locked',   { initial => 1 });
    $sm->add_state('unlocked', { accepting => 1 });
    $sm->add_transition('locked',   'coin', 'unlocked');
    $sm->add_transition('locked',   'push', 'locked');
    $sm->add_transition('unlocked', 'push', 'locked');
    $sm->add_transition('unlocked', 'coin', 'unlocked');

    print $sm->current_state();  # locked
    $sm->process('coin');
    print $sm->current_state();  # unlocked
    print $sm->is_accepting();   # 1
    $sm->reset();
    print $sm->current_state();  # locked

=head1 DESCRIPTION

A Finite State Machine (FSM/DFA) implementation supporting:

=over 4

=item * States with entry/exit callbacks

=item * Transitions with action callbacks and guard conditions

=item * Accepting states for formal language recognition

=item * reset() to return to initial state

=back

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
