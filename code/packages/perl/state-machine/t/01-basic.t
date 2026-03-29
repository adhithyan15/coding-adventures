use strict;
use warnings;
use Test2::V0;

ok(eval { require CodingAdventures::StateMachine; 1 }, 'CodingAdventures::StateMachine loads');

# ============================================================================
# Helper: build the classic turnstile state machine
#
#   States:      locked (initial), unlocked (accepting)
#   Transitions: locked+coin → unlocked
#                locked+push → locked
#                unlocked+push → locked
#                unlocked+coin → unlocked
# ============================================================================

sub make_turnstile {
    my $sm = CodingAdventures::StateMachine->new();
    $sm->add_state('locked',   { initial => 1 });
    $sm->add_state('unlocked', { accepting => 1 });
    $sm->add_transition('locked',   'coin', 'unlocked');
    $sm->add_transition('locked',   'push', 'locked');
    $sm->add_transition('unlocked', 'push', 'locked');
    $sm->add_transition('unlocked', 'coin', 'unlocked');
    return $sm;
}

# ============================================================================
# Construction and initial state
# ============================================================================

subtest 'construction' => sub {
    my $sm = CodingAdventures::StateMachine->new();
    ok($sm, 'StateMachine created');
    is($sm->current_state(), undef, 'no initial state before add_state');
};

# ============================================================================
# Turnstile — basic operation
# ============================================================================

subtest 'turnstile — initial state' => sub {
    my $sm = make_turnstile();
    is($sm->current_state(), 'locked', 'initial state is locked');
    is($sm->is_accepting(),  0,        'locked is not accepting');
};

subtest 'turnstile — coin unlocks' => sub {
    my $sm = make_turnstile();
    my $new = $sm->process('coin');
    is($new,                   'unlocked', 'process(coin) returns unlocked');
    is($sm->current_state(),   'unlocked', 'current state is unlocked');
    is($sm->is_accepting(),    1,          'unlocked is accepting');
};

subtest 'turnstile — push while locked stays locked' => sub {
    my $sm = make_turnstile();
    my $new = $sm->process('push');
    is($new,                 'locked', 'push while locked stays locked');
    is($sm->current_state(), 'locked', 'current state unchanged');
};

subtest 'turnstile — push while unlocked relocks' => sub {
    my $sm = make_turnstile();
    $sm->process('coin');    # unlock first
    my $new = $sm->process('push');
    is($new,                 'locked', 'push while unlocked relocks');
    is($sm->current_state(), 'locked', 'back to locked');
    is($sm->is_accepting(),  0,        'locked is not accepting');
};

subtest 'turnstile — extra coin while unlocked stays unlocked' => sub {
    my $sm = make_turnstile();
    $sm->process('coin');
    my $new = $sm->process('coin');
    is($new,                 'unlocked', 'extra coin stays unlocked');
    is($sm->current_state(), 'unlocked', 'still unlocked');
};

subtest 'turnstile — reset' => sub {
    my $sm = make_turnstile();
    $sm->process('coin');
    is($sm->current_state(), 'unlocked', 'after coin: unlocked');
    $sm->reset();
    is($sm->current_state(), 'locked', 'after reset: back to locked');
    is($sm->is_accepting(),  0,        'locked not accepting after reset');
};

subtest 'turnstile — unknown event returns undef' => sub {
    my $sm = make_turnstile();
    my $result = $sm->process('explode');
    is($result,              undef,    'unknown event returns undef');
    is($sm->current_state(), 'locked', 'state unchanged after unknown event');
};

# ============================================================================
# set_initial
# ============================================================================

subtest 'set_initial sets current state' => sub {
    my $sm = CodingAdventures::StateMachine->new();
    $sm->add_state('start');
    $sm->add_state('end', { accepting => 1 });
    $sm->set_initial('start');
    is($sm->current_state(), 'start', 'set_initial works');
    is($sm->is_accepting(),  0,       'start not accepting');
};

# ============================================================================
# Transition action callbacks
# ============================================================================

subtest 'transition action callback fires' => sub {
    my @log;
    my $sm = CodingAdventures::StateMachine->new();
    $sm->add_state('a', { initial => 1 });
    $sm->add_state('b');
    $sm->add_transition('a', 'go', 'b', {
        action => sub {
            my ($from, $event, $to) = @_;
            push @log, "$from--$event-->$to";
        }
    });
    $sm->process('go');
    is(\@log, ['a--go-->b'], 'transition action fires with correct args');
};

# ============================================================================
# Entry and exit callbacks
# ============================================================================

subtest 'entry and exit actions fire in correct order' => sub {
    my @log;
    my $sm = CodingAdventures::StateMachine->new();
    $sm->add_state('off', {
        initial => 1,
        exit    => sub { push @log, 'exit:off' },
    });
    $sm->add_state('on', {
        accepting => 1,
        entry     => sub { push @log, 'entry:on' },
    });
    $sm->add_transition('off', 'flip', 'on');
    $sm->process('flip');
    # exit fires before entry (standard HSM convention)
    is(\@log, ['exit:off', 'entry:on'], 'exit fires before entry');
};

# ============================================================================
# Guard conditions
# ============================================================================

subtest 'guard condition blocks transition when false' => sub {
    my $allowed = 0;   # gate starts closed
    my $sm = CodingAdventures::StateMachine->new();
    $sm->add_state('closed', { initial => 1 });
    $sm->add_state('open',   { accepting => 1 });
    $sm->add_transition('closed', 'open', 'open', {
        guard => sub { $allowed },
    });

    # Guard is false → transition blocked
    $sm->process('open');
    is($sm->current_state(), 'closed', 'guard=false keeps machine in closed state');

    # Open the gate
    $allowed = 1;
    $sm->process('open');
    is($sm->current_state(), 'open', 'guard=true allows transition');
    is($sm->is_accepting(),  1,      'open state is accepting');
};

# ============================================================================
# states() returns all state names
# ============================================================================

subtest 'states() lists all defined state names' => sub {
    my $sm = make_turnstile();
    my @names = $sm->states();
    is(\@names, bag { item 'locked'; item 'unlocked'; end }, 'states() returns both states');
};

# ============================================================================
# Error cases
# ============================================================================

subtest 'duplicate state dies' => sub {
    my $sm = CodingAdventures::StateMachine->new();
    $sm->add_state('a');
    ok(!eval { $sm->add_state('a'); 1 }, 'duplicate state raises error');
};

subtest 'transition with unknown source state dies' => sub {
    my $sm = CodingAdventures::StateMachine->new();
    $sm->add_state('a');
    ok(!eval { $sm->add_transition('x', 'e', 'a'); 1 },
       'undefined source state raises error');
};

subtest 'transition with unknown target state dies' => sub {
    my $sm = CodingAdventures::StateMachine->new();
    $sm->add_state('a');
    ok(!eval { $sm->add_transition('a', 'e', 'z'); 1 },
       'undefined target state raises error');
};

subtest 'process without initial state dies' => sub {
    my $sm = CodingAdventures::StateMachine->new();
    $sm->add_state('a');
    ok(!eval { $sm->process('event'); 1 }, 'process without initial state dies');
};

# ============================================================================
# Multi-step sequence
# ============================================================================

subtest 'multi-step sequence coin-push-coin-push' => sub {
    my $sm = make_turnstile();
    my @states;
    for my $ev (qw(coin push coin push)) {
        push @states, scalar $sm->process($ev);
    }
    is(\@states, ['unlocked', 'locked', 'unlocked', 'locked'],
       'coin-push-coin-push sequence correct');
};

done_testing;
