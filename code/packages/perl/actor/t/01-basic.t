use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Actor;

# Shorthand aliases
my $AR  = 'CodingAdventures::Actor::ActorResult';
my $AS  = 'CodingAdventures::Actor::ActorSpec';
my $SYS = 'CodingAdventures::Actor::ActorSystem';

# ---------------------------------------------------------------------------
# 1. VERSION
# ---------------------------------------------------------------------------
is(CodingAdventures::Actor->VERSION, '0.01', 'has VERSION 0.01');

# ---------------------------------------------------------------------------
# 2. ActorResult — defaults
# ---------------------------------------------------------------------------
my $result = $AR->new(new_state => 42);
is($result->new_state,        42, 'ActorResult new_state');
is(ref $result->messages_to_send, 'ARRAY', 'ActorResult messages_to_send default is arrayref');
is(scalar @{$result->messages_to_send}, 0,  'ActorResult messages_to_send default is empty');
is(ref $result->actors_to_create, 'ARRAY',  'ActorResult actors_to_create default is arrayref');
is(scalar @{$result->actors_to_create}, 0,  'ActorResult actors_to_create default is empty');
is($result->stop, 0, 'ActorResult stop defaults to 0');

# ---------------------------------------------------------------------------
# 3. ActorResult — explicit fields
# ---------------------------------------------------------------------------
my $result2 = $AR->new(
    new_state        => 'hello',
    messages_to_send => [['id1', { type => 'ping' }]],
    stop             => 1,
);
is($result2->new_state, 'hello', 'ActorResult explicit new_state');
is(scalar @{$result2->messages_to_send}, 1, 'ActorResult messages_to_send count');
is($result2->messages_to_send->[0][0], 'id1', 'ActorResult outgoing target id');
is($result2->stop, 1, 'ActorResult stop=1');

# ---------------------------------------------------------------------------
# 4. ActorSpec
# ---------------------------------------------------------------------------
my $spec = $AS->new(
    actor_id      => 'worker',
    initial_state => 0,
    behavior      => sub { $AR->new(new_state => $_[0] + 1) },
);
is($spec->actor_id,      'worker', 'ActorSpec actor_id');
is($spec->initial_state, 0,        'ActorSpec initial_state');
is(ref $spec->behavior,  'CODE',   'ActorSpec behavior is coderef');

# ---------------------------------------------------------------------------
# 5. ActorSystem — spawn and get_state
# ---------------------------------------------------------------------------
my $sys = $SYS->new();
my $id  = $sys->spawn('a', 10, sub { $AR->new(new_state => $_[0]) });
is($id, 'a', 'spawn returns actor id');
is($sys->get_state('a'), 10, 'get_state returns initial state');

# ---------------------------------------------------------------------------
# 6. ActorSystem — duplicate spawn dies
# ---------------------------------------------------------------------------
ok(dies { $sys->spawn('a', 0, sub {}) }, 'spawning duplicate id dies');

# ---------------------------------------------------------------------------
# 7. ActorSystem — get_state unknown actor dies
# ---------------------------------------------------------------------------
ok(dies { $sys->get_state('nonexistent') }, 'get_state on unknown id dies');

# ---------------------------------------------------------------------------
# 8. Simple counter — send + run
# ---------------------------------------------------------------------------
my $sys2 = $SYS->new();
$sys2->spawn('counter', 0, sub {
    my ($state, $msg) = @_;
    if ($msg->{type} eq 'increment') {
        return $AR->new(new_state => $state + 1);
    }
    return $AR->new(new_state => $state);
});

$sys2->send('counter', { type => 'increment' });
$sys2->send('counter', { type => 'increment' });
$sys2->send('counter', { type => 'noop' });
$sys2->run();

is($sys2->get_state('counter'), 2, 'counter after 2 increments and 1 noop is 2');

# ---------------------------------------------------------------------------
# 9. Message ordering is FIFO
# ---------------------------------------------------------------------------
my $sys3 = $SYS->new();
my @log;
$sys3->spawn('logger', [], sub {
    my ($state, $msg) = @_;
    push @$state, $msg->{val};
    return $AR->new(new_state => $state);
});

$sys3->send('logger', { val => 1 });
$sys3->send('logger', { val => 2 });
$sys3->send('logger', { val => 3 });
$sys3->run();

is($sys3->get_state('logger'), [1, 2, 3], 'messages processed in FIFO order');

# ---------------------------------------------------------------------------
# 10. Messages to other actors (messages_to_send)
# ---------------------------------------------------------------------------
my $sys4 = $SYS->new();
$sys4->spawn('pinger', 0, sub {
    my ($state, $msg) = @_;
    if ($msg->{type} eq 'start') {
        return $AR->new(
            new_state        => $state,
            messages_to_send => [['ponger', { type => 'pong_me' }]],
        );
    }
    return $AR->new(new_state => $state);
});
$sys4->spawn('ponger', 0, sub {
    my ($state, $msg) = @_;
    if ($msg->{type} eq 'pong_me') {
        return $AR->new(new_state => $state + 1);
    }
    return $AR->new(new_state => $state);
});

$sys4->send('pinger', { type => 'start' });
$sys4->run();

is($sys4->get_state('ponger'), 1, 'pinger forwarded message to ponger');

# ---------------------------------------------------------------------------
# 11. Actors can spawn children (actors_to_create)
# ---------------------------------------------------------------------------
my $sys5 = $SYS->new();
$sys5->spawn('parent', 0, sub {
    my ($state, $msg) = @_;
    if ($msg->{type} eq 'create_child') {
        my $child_spec = $AS->new(
            actor_id      => 'child',
            initial_state => 99,
            behavior      => sub { $AR->new(new_state => $_[0]) },
        );
        return $AR->new(
            new_state        => $state + 1,
            actors_to_create => [$child_spec],
        );
    }
    return $AR->new(new_state => $state);
});

$sys5->send('parent', { type => 'create_child' });
$sys5->run();

is($sys5->get_state('parent'), 1,  'parent state incremented after creating child');
is($sys5->get_state('child'),  99, 'child spawned with correct initial state');

# ---------------------------------------------------------------------------
# 12. stop flag
# ---------------------------------------------------------------------------
my $sys6 = $SYS->new();
$sys6->spawn('dying', 0, sub {
    my ($state, $msg) = @_;
    if ($msg->{type} eq 'die') {
        return $AR->new(new_state => $state, stop => 1);
    }
    return $AR->new(new_state => $state + 1);
});

$sys6->send('dying', { type => 'inc' });
$sys6->send('dying', { type => 'die' });
$sys6->send('dying', { type => 'inc' });   # should go to dead_letters
$sys6->run();

is($sys6->get_state('dying'), 1, 'state is 1 (one inc before die)');
is($sys6->is_stopped('dying'), 1, 'actor is marked stopped');
is(scalar @{$sys6->dead_letters}, 1, 'post-stop message goes to dead_letters');
is($sys6->dead_letters->[0][0], 'dying', 'dead letter target is dying');

# ---------------------------------------------------------------------------
# 13. is_stopped on a live actor
# ---------------------------------------------------------------------------
my $sys7 = $SYS->new();
$sys7->spawn('alive', 0, sub { $AR->new(new_state => $_[0]) });
is($sys7->is_stopped('alive'), 0, 'live actor is_stopped returns 0');

# ---------------------------------------------------------------------------
# 14. is_stopped on unknown actor dies
# ---------------------------------------------------------------------------
ok(dies { $sys7->is_stopped('ghost') }, 'is_stopped on unknown id dies');

# ---------------------------------------------------------------------------
# 15. Message to unknown actor goes to dead_letters
# ---------------------------------------------------------------------------
my $sys8 = $SYS->new();
$sys8->send('nobody', { type => 'hello' });
$sys8->run();

is(scalar @{$sys8->dead_letters}, 1, 'message to unknown actor goes to dead_letters');
is($sys8->dead_letters->[0][0], 'nobody', 'dead letter target is nobody');

# ---------------------------------------------------------------------------
# 16. run() is idempotent when queue is already empty
# ---------------------------------------------------------------------------
my $sys9 = $SYS->new();
$sys9->spawn('idle', 5, sub { $AR->new(new_state => $_[0]) });
$sys9->run();   # queue empty, should be a no-op
is($sys9->get_state('idle'), 5, 'run() on empty queue changes nothing');

# ---------------------------------------------------------------------------
# 17. actor_ids()
# ---------------------------------------------------------------------------
my $sys10 = $SYS->new();
$sys10->spawn('b', 0, sub { $AR->new(new_state => $_[0]) });
$sys10->spawn('a', 0, sub { $AR->new(new_state => $_[0]) });
$sys10->spawn('c', 0, sub { $AR->new(new_state => $_[0]) });
is([$sys10->actor_ids()], ['a', 'b', 'c'], 'actor_ids returns sorted ids');

# ---------------------------------------------------------------------------
# 18. Chain of forwarded messages
# ---------------------------------------------------------------------------
# a -> b -> c, each incrementing state by 1
my $sys11 = $SYS->new();
my $make_forwarding = sub {
    my $next = shift;
    return sub {
        my ($state, $msg) = @_;
        my @fwd = $next ? ([$next, { type => 'inc' }]) : ();
        return $AR->new(
            new_state        => $state + 1,
            messages_to_send => \@fwd,
        );
    };
};
$sys11->spawn('c', 0, $make_forwarding->(undef));
$sys11->spawn('b', 0, $make_forwarding->('c'));
$sys11->spawn('a', 0, $make_forwarding->('b'));

$sys11->send('a', { type => 'inc' });
$sys11->run();

is($sys11->get_state('a'), 1, 'chain: a state is 1');
is($sys11->get_state('b'), 1, 'chain: b state is 1');
is($sys11->get_state('c'), 1, 'chain: c state is 1');

# ---------------------------------------------------------------------------
# 19. Multiple dead letters accumulate
# ---------------------------------------------------------------------------
my $sys12 = $SYS->new();
$sys12->send('x', { n => 1 });
$sys12->send('x', { n => 2 });
$sys12->send('y', { n => 3 });
$sys12->run();

is(scalar @{$sys12->dead_letters}, 3, 'all messages to unknown actors become dead letters');

# ---------------------------------------------------------------------------
# 20. State can be any Perl value (hashref)
# ---------------------------------------------------------------------------
my $sys13 = $SYS->new();
$sys13->spawn('store', {}, sub {
    my ($state, $msg) = @_;
    if ($msg->{type} eq 'set') {
        my %new = %$state;
        $new{$msg->{key}} = $msg->{value};
        return $AR->new(new_state => \%new);
    }
    return $AR->new(new_state => $state);
});

$sys13->send('store', { type => 'set', key => 'x', value => 42 });
$sys13->send('store', { type => 'set', key => 'y', value => 'hello' });
$sys13->run();

is($sys13->get_state('store')->{x}, 42,      'store state x=42');
is($sys13->get_state('store')->{y}, 'hello', 'store state y=hello');

done_testing;
