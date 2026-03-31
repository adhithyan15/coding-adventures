use strict;
use warnings;
use Test2::V0;

use CodingAdventures::EventLoop;

sub near {
    my ($got, $exp, $tol) = @_;
    $tol //= 1e-9;
    return abs($got - $exp) < $tol;
}

# ---------------------------------------------------------------------------
# new()
# ---------------------------------------------------------------------------

subtest 'new() — initial state' => sub {
    my $loop = CodingAdventures::EventLoop->new();
    ok(near($loop->{elapsed_time}, 0.0), 'elapsed_time = 0');
    is($loop->{tick_count}, 0, 'tick_count = 0');
};

# ---------------------------------------------------------------------------
# on() / emit()
# ---------------------------------------------------------------------------

subtest 'on() + emit() — basic fire' => sub {
    my $loop  = CodingAdventures::EventLoop->new();
    my $fired = 0;
    $loop->on("test", sub { $fired = 1 });
    $loop->emit("test", {});
    ok($fired, 'handler was fired');
};

subtest 'emit() — passes data to handler' => sub {
    my $loop = CodingAdventures::EventLoop->new();
    my $received;
    $loop->on("msg", sub { $received = shift->{value} });
    $loop->emit("msg", { value => 42 });
    is($received, 42, 'data passed correctly');
};

subtest 'emit() — no handlers does nothing' => sub {
    my $loop = CodingAdventures::EventLoop->new();
    ok(lives { $loop->emit("no_such", {}) }, 'no error on missing event');
};

subtest 'emit() — multiple handlers fire in order' => sub {
    my $loop  = CodingAdventures::EventLoop->new();
    my @calls;
    $loop->on("ev", sub { push @calls, "first"  });
    $loop->on("ev", sub { push @calls, "second" });
    $loop->on("ev", sub { push @calls, "third"  });
    $loop->emit("ev", undef);
    is(\@calls, ["first","second","third"], 'handlers fire in order');
};

subtest 'emit() — fires each time called' => sub {
    my $loop  = CodingAdventures::EventLoop->new();
    my $count = 0;
    $loop->on("ev", sub { $count++ });
    $loop->emit("ev", undef) for 1..3;
    is($count, 3, 'handler fired 3 times');
};

subtest 'emit() — different events are independent' => sub {
    my $loop = CodingAdventures::EventLoop->new();
    my ($a, $b) = (0, 0);
    $loop->on("a", sub { $a++ });
    $loop->on("b", sub { $b++ });
    $loop->emit("a", undef);
    $loop->emit("a", undef);
    $loop->emit("b", undef);
    is($a, 2, 'a fired twice');
    is($b, 1, 'b fired once');
};

# ---------------------------------------------------------------------------
# once()
# ---------------------------------------------------------------------------

subtest 'once() — fires exactly once' => sub {
    my $loop  = CodingAdventures::EventLoop->new();
    my $count = 0;
    $loop->once("ev", sub { $count++ });
    $loop->emit("ev", undef) for 1..3;
    is($count, 1, 'once handler fired exactly once');
};

subtest 'once() — data passed correctly' => sub {
    my $loop = CodingAdventures::EventLoop->new();
    my $got;
    $loop->once("init", sub { $got = shift->{v} });
    $loop->emit("init", { v => 99 });
    is($got, 99, 'data passed to once handler');
};

subtest 'once() — does not affect persistent on() handlers' => sub {
    my $loop = CodingAdventures::EventLoop->new();
    my ($once_count, $always_count) = (0, 0);
    $loop->once("ev", sub { $once_count++ });
    $loop->on("ev",   sub { $always_count++ });
    $loop->emit("ev", undef) for 1..3;
    is($once_count,   1, 'once fired once');
    is($always_count, 3, 'on fired three times');
};

# ---------------------------------------------------------------------------
# off()
# ---------------------------------------------------------------------------

subtest 'off() — removes all handlers' => sub {
    my $loop  = CodingAdventures::EventLoop->new();
    my $count = 0;
    $loop->on("ev", sub { $count++ });
    $loop->on("ev", sub { $count++ });
    $loop->off("ev");
    $loop->emit("ev", undef);
    is($count, 0, 'no handlers remain after off()');
};

subtest 'off() — removes specific handler' => sub {
    my $loop = CodingAdventures::EventLoop->new();
    my ($a, $b) = (0, 0);
    my $ha = sub { $a++ };
    my $hb = sub { $b++ };
    $loop->on("ev", $ha);
    $loop->on("ev", $hb);
    $loop->off("ev", $ha);
    $loop->emit("ev", undef);
    is($a, 0, 'ha removed');
    is($b, 1, 'hb still fires');
};

subtest 'off() — nonexistent event does not error' => sub {
    my $loop = CodingAdventures::EventLoop->new();
    ok(lives { $loop->off("no_such") }, 'no error');
};

subtest 'off() — removes only first occurrence of duplicate handler' => sub {
    my $loop  = CodingAdventures::EventLoop->new();
    my $count = 0;
    my $h     = sub { $count++ };
    $loop->on("ev", $h);
    $loop->on("ev", $h);
    $loop->off("ev", $h);
    $loop->emit("ev", undef);
    is($count, 1, 'one occurrence remains');
};

# ---------------------------------------------------------------------------
# on_tick() / tick()
# ---------------------------------------------------------------------------

subtest 'on_tick() + tick() — fires on each tick' => sub {
    my $loop  = CodingAdventures::EventLoop->new();
    my $count = 0;
    $loop->on_tick(sub { $count++ });
    $loop->tick() for 1..3;
    is($count, 3, 'tick handler fired 3 times');
};

subtest 'tick() — passes delta_time to handlers' => sub {
    my $loop = CodingAdventures::EventLoop->new();
    my @dts;
    $loop->on_tick(sub { push @dts, shift });
    $loop->tick(0.5);
    $loop->tick(1.0);
    $loop->tick(0.25);
    is(\@dts, [0.5, 1.0, 0.25], 'delta_times passed correctly');
};

subtest 'tick() — default delta_time is 1.0' => sub {
    my $loop = CodingAdventures::EventLoop->new();
    my $got;
    $loop->on_tick(sub { $got = shift });
    $loop->tick();
    ok(near($got, 1.0), 'default dt = 1.0');
};

subtest 'tick() — multiple tick handlers fire in order' => sub {
    my $loop  = CodingAdventures::EventLoop->new();
    my @order;
    $loop->on_tick(sub { push @order, "A" });
    $loop->on_tick(sub { push @order, "B" });
    $loop->tick();
    is(\@order, ["A","B"], 'tick handlers fire in registration order');
};

subtest 'tick() — updates elapsed_time' => sub {
    my $loop = CodingAdventures::EventLoop->new();
    $loop->tick(0.5);
    $loop->tick(0.25);
    $loop->tick(1.0);
    ok(near($loop->{elapsed_time}, 1.75), 'elapsed_time = 1.75');
};

subtest 'tick() — increments tick_count' => sub {
    my $loop = CodingAdventures::EventLoop->new();
    $loop->tick() for 1..5;
    is($loop->{tick_count}, 5, 'tick_count = 5');
};

# ---------------------------------------------------------------------------
# run()
# ---------------------------------------------------------------------------

subtest 'run() — fires the right number of ticks' => sub {
    my $loop  = CodingAdventures::EventLoop->new();
    my $count = 0;
    $loop->on_tick(sub { $count++ });
    $loop->run(7);
    is($count, 7, 'run(7) fires 7 ticks');
};

subtest 'run() — uses provided delta_time' => sub {
    my $loop = CodingAdventures::EventLoop->new();
    $loop->run(4, 0.25);
    ok(near($loop->{elapsed_time}, 1.0), 'elapsed_time = 1.0');
    is($loop->{tick_count}, 4, 'tick_count = 4');
};

subtest 'run() — defaults to 1 tick' => sub {
    my $loop = CodingAdventures::EventLoop->new();
    $loop->run();
    is($loop->{tick_count}, 1, 'run() with no args = 1 tick');
};

# ---------------------------------------------------------------------------
# step()
# ---------------------------------------------------------------------------

subtest 'step() — one tick' => sub {
    my $loop  = CodingAdventures::EventLoop->new();
    my $count = 0;
    $loop->on_tick(sub { $count++ });
    $loop->step();
    is($count, 1, 'step fires one tick');
};

subtest 'step() — passes delta_time' => sub {
    my $loop = CodingAdventures::EventLoop->new();
    my $got;
    $loop->on_tick(sub { $got = shift });
    $loop->step(0.016);
    ok(near($got, 0.016), 'step dt = 0.016');
};

# ---------------------------------------------------------------------------
# Cross-concerns
# ---------------------------------------------------------------------------

subtest 'emit and tick coexist' => sub {
    my $loop = CodingAdventures::EventLoop->new();
    my ($tick_count, $event_count) = (0, 0);
    $loop->on_tick(sub { $tick_count++ });
    $loop->on("hit",  sub { $event_count++ });
    $loop->run(3);
    $loop->emit("hit", undef) for 1..2;
    is($tick_count,  3, '3 ticks');
    is($event_count, 2, '2 events');
};

done_testing();
