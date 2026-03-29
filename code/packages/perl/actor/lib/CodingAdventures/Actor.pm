package CodingAdventures::Actor;

# ============================================================================
# CodingAdventures::Actor — Actor model implementation with message passing
# ============================================================================
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.
#
# WHAT IS THE ACTOR MODEL?
# ------------------------
# The Actor model is a concurrency model invented by Carl Hewitt in 1973.
# The core idea:
#
#   * Everything is an "actor" — an independent unit with private state.
#   * Actors communicate ONLY by sending messages (no shared memory).
#   * When an actor receives a message it can:
#       1. Update its own state.
#       2. Send messages to other actors.
#       3. Create new actors.
#       4. Stop itself.
#
# Languages like Erlang, Elixir, and Akka (Scala/Java) are built on actors.
#
# ERLANG ANALOGY
# --------------
# In Erlang every process has a mailbox.  Messages queue up there until the
# process calls `receive`.  Our ActorSystem is a simplified, synchronous
# version: run() drains the global queue in FIFO order.
#
# COMPONENTS
# ----------
#   ActorResult   — the return value of a behavior function
#   ActorSpec     — a blueprint for spawning a new actor
#   ActorSystem   — the runtime that owns all actors and the message queue
#
# SINGLE-THREADED NOTE
# --------------------
# Perl is single-threaded (ignoring ithreads).  This implementation is
# therefore 100% synchronous.  run() processes messages one at a time in
# FIFO order until the queue is empty.  This is sufficient for simulation
# and testing, and matches the mental model for understanding actors even
# in real concurrent systems.
#
# Usage:
#
#   use CodingAdventures::Actor;
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# ActorResult — what a behavior function returns
# ============================================================================
#
# A behavior function has the signature:
#
#   sub my_behavior {
#       my ($state, $message) = @_;
#       # ... compute ...
#       return CodingAdventures::Actor::ActorResult->new(
#           new_state        => $new_state,
#           messages_to_send => [ [$target_id, $msg], ... ],
#           actors_to_create => [ $actor_spec, ... ],
#           stop             => 0,
#       );
#   }
#
# Fields
# ------
# new_state        — the actor's state after processing this message.
#                    Required.
#
# messages_to_send — arrayref of [$target_id, $message] pairs to enqueue.
#                    Default: [].
#
# actors_to_create — arrayref of ActorSpec objects to spawn.
#                    Default: [].
#
# stop             — if true, the actor is marked stopped after this
#                    message.  Future messages go to dead_letters.
#                    Default: 0.

package CodingAdventures::Actor::ActorResult;

sub new {
    my ($class, %args) = @_;
    return bless {
        new_state        => $args{new_state},
        messages_to_send => $args{messages_to_send} // [],
        actors_to_create => $args{actors_to_create} // [],
        stop             => $args{stop}             // 0,
    }, $class;
}

sub new_state        { return $_[0]->{new_state}        }
sub messages_to_send { return $_[0]->{messages_to_send} }
sub actors_to_create { return $_[0]->{actors_to_create} }
sub stop             { return $_[0]->{stop}             }

# ============================================================================
# ActorSpec — blueprint for creating a new actor
# ============================================================================
#
# Returned inside ActorResult::actors_to_create when a behavior wants to
# spawn a child actor.
#
# Fields
# ------
# actor_id      — unique string identifier (must not already exist)
# initial_state — the state the new actor starts with
# behavior      — coderef: sub { my ($state, $msg) = @_; ... return ActorResult }

package CodingAdventures::Actor::ActorSpec;

sub new {
    my ($class, %args) = @_;
    return bless {
        actor_id      => $args{actor_id},
        initial_state => $args{initial_state},
        behavior      => $args{behavior},
    }, $class;
}

sub actor_id      { return $_[0]->{actor_id}      }
sub initial_state { return $_[0]->{initial_state} }
sub behavior      { return $_[0]->{behavior}      }

# ============================================================================
# ActorSystem — the runtime
# ============================================================================
#
# Holds:
#   actors       — hashref mapping actor_id -> actor record
#   queue        — arrayref of [$target_id, $message] pending deliveries
#   dead_letters — arrayref of [$target_id, $message] that could not
#                  be delivered (actor stopped or unknown)
#
# Actor record (internal):
#   {
#     id       => $id,
#     state    => $state,
#     behavior => $coderef,
#     stopped  => 0 | 1,
#   }

package CodingAdventures::Actor::ActorSystem;

sub new {
    my ($class) = @_;
    return bless {
        actors       => {},
        queue        => [],
        dead_letters => [],
    }, $class;
}

# spawn($id, $initial_state, $behavior_coderef)
# ---------------------------------------------
# Register a new actor.  Dies if an actor with that ID already exists.
# Returns $id so callers can chain: my $id = $system->spawn(...);
sub spawn {
    my ($self, $id, $initial_state, $behavior) = @_;
    die "Actor '$id' already exists" if exists $self->{actors}{$id};
    $self->{actors}{$id} = {
        id       => $id,
        state    => $initial_state,
        behavior => $behavior,
        stopped  => 0,
    };
    return $id;
}

# send($target_id, $message)
# --------------------------
# Enqueue a message for delivery.  The message is not processed until
# run() is called.  $message can be any Perl value (hashref, string, etc.)
sub send {
    my ($self, $target_id, $message) = @_;
    push @{$self->{queue}}, [$target_id, $message];
}

# run()
# -----
# Process all enqueued messages until the queue is empty.
#
# Algorithm:
#   while queue is not empty:
#     shift first message from queue
#     look up target actor
#     if actor does not exist or is stopped:
#       append to dead_letters, continue
#     call behavior(state, message) -> ActorResult
#     update actor state
#     enqueue messages_to_send
#     spawn actors_to_create
#     if result.stop: mark actor stopped
#
# This FIFO processing order means messages sent earlier are always
# processed before messages sent later — the same guarantee as Erlang's
# mailbox ordering within a single sender.
sub run {
    my ($self) = @_;
    while (@{$self->{queue}}) {
        my $item      = shift @{$self->{queue}};
        my $target_id = $item->[0];
        my $message   = $item->[1];

        my $actor = $self->{actors}{$target_id};

        # Unknown or stopped actor → dead letter
        if (!$actor || $actor->{stopped}) {
            push @{$self->{dead_letters}}, [$target_id, $message];
            next;
        }

        # Invoke the behavior
        my $result = $actor->{behavior}->($actor->{state}, $message);

        # Apply new state
        $actor->{state} = $result->new_state;

        # Enqueue outgoing messages
        for my $pair (@{$result->messages_to_send}) {
            push @{$self->{queue}}, $pair;
        }

        # Spawn child actors
        for my $spec (@{$result->actors_to_create}) {
            $self->spawn(
                $spec->actor_id,
                $spec->initial_state,
                $spec->behavior,
            );
        }

        # Stop if requested
        if ($result->stop) {
            $actor->{stopped} = 1;
        }
    }
}

# get_state($id) — return the current state of an actor
sub get_state {
    my ($self, $id) = @_;
    die "Actor '$id' does not exist" unless exists $self->{actors}{$id};
    return $self->{actors}{$id}{state};
}

# is_stopped($id) — return 1 if the actor is stopped, 0 otherwise
sub is_stopped {
    my ($self, $id) = @_;
    die "Actor '$id' does not exist" unless exists $self->{actors}{$id};
    return $self->{actors}{$id}{stopped} ? 1 : 0;
}

# dead_letters() — return the arrayref of undeliverable messages
sub dead_letters {
    my ($self) = @_;
    return $self->{dead_letters};
}

# actor_ids() — return a sorted list of all actor IDs
sub actor_ids {
    my ($self) = @_;
    return sort keys %{$self->{actors}};
}

# ============================================================================
# Back to the top-level package
# ============================================================================

package CodingAdventures::Actor;

1;

__END__

=head1 NAME

CodingAdventures::Actor - Actor model implementation with message passing and supervised execution

=head1 SYNOPSIS

    use CodingAdventures::Actor;

    my $system = CodingAdventures::Actor::ActorSystem->new();

    my $id = $system->spawn("counter", 0, sub {
        my ($state, $msg) = @_;
        if ($msg->{type} eq 'increment') {
            return CodingAdventures::Actor::ActorResult->new(
                new_state => $state + 1,
            );
        }
        return CodingAdventures::Actor::ActorResult->new(new_state => $state);
    });

    $system->send($id, { type => 'increment' });
    $system->send($id, { type => 'increment' });
    $system->run();

    print $system->get_state($id);   # 2

=head1 DESCRIPTION

A synchronous, single-threaded implementation of the Actor model.  Actors
communicate by passing messages; state is private and updated only by the
actor's own behavior function.

=head2 Classes

=over 4

=item CodingAdventures::Actor::ActorResult

Return value of a behavior function.  Fields: C<new_state>,
C<messages_to_send>, C<actors_to_create>, C<stop>.

=item CodingAdventures::Actor::ActorSpec

Blueprint for a new actor.  Fields: C<actor_id>, C<initial_state>,
C<behavior>.

=item CodingAdventures::Actor::ActorSystem

The runtime.  Methods: C<spawn>, C<send>, C<run>, C<get_state>,
C<is_stopped>, C<dead_letters>, C<actor_ids>.

=back

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
