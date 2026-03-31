use strict;
use warnings;
use Test2::V0;

use CodingAdventures::InterruptHandler;

my $IDT        = 'CodingAdventures::InterruptHandler::IDT';
my $ISRReg     = 'CodingAdventures::InterruptHandler::ISRRegistry';
my $Frame      = 'CodingAdventures::InterruptHandler::Frame';
my $Controller = 'CodingAdventures::InterruptHandler::Controller';

# ============================================================================
# IDT tests
# ============================================================================

subtest 'IDT — new creates empty table' => sub {
    my $idt = $IDT->new();
    my $e   = $idt->get_entry(0);
    is($e->{present},         0, 'entry 0 not present by default');
    is($e->{isr_address},     0, 'isr_address defaults to 0');
    is($e->{privilege_level}, 0, 'privilege_level defaults to 0');
};

subtest 'IDT — set_entry and get_entry round-trip' => sub {
    my $idt = $IDT->new();
    $idt->set_entry(32, { isr_address => 0x2000, present => 1, privilege_level => 0 });
    my $e = $idt->get_entry(32);
    is($e->{isr_address}, 0x2000, 'isr_address stored');
    is($e->{present},     1,       'present stored');
};

subtest 'IDT — set_entry rejects out-of-range' => sub {
    my $idt = $IDT->new();
    ok(dies { $idt->set_entry(256, { isr_address => 0 }) }, 'dies on 256');
    ok(dies { $idt->set_entry(-1,  { isr_address => 0 }) }, 'dies on -1');
};

subtest 'IDT — get_entry rejects out-of-range' => sub {
    my $idt = $IDT->new();
    ok(dies { $idt->get_entry(256) }, 'dies on 256');
};

subtest 'IDT — overwrite entry' => sub {
    my $idt = $IDT->new();
    $idt->set_entry(0, { isr_address => 0x100 });
    $idt->set_entry(0, { isr_address => 0x200 });
    is($idt->get_entry(0)->{isr_address}, 0x200, 'entry overwritten');
};

subtest 'IDT — multiple entries coexist' => sub {
    my $idt = $IDT->new();
    for my $i (0..5) {
        $idt->set_entry($i, { isr_address => $i * 16, present => 1 });
    }
    for my $i (0..5) {
        is($idt->get_entry($i)->{isr_address}, $i * 16, "entry $i ok");
    }
};

# ============================================================================
# ISRRegistry tests
# ============================================================================

subtest 'ISRRegistry — new has no handlers' => sub {
    my $r = $ISRReg->new();
    ok(!$r->has_handler(0), 'no handler for 0');
};

subtest 'ISRRegistry — register and has_handler' => sub {
    my $r = $ISRReg->new();
    $r->register(32, sub { });
    ok($r->has_handler(32),  'has handler for 32');
    ok(!$r->has_handler(33), 'no handler for 33');
};

subtest 'ISRRegistry — dispatch calls handler' => sub {
    my $r = $ISRReg->new();
    my $called = 0;
    $r->register(32, sub {
        my ($frame, $kernel) = @_;
        $called = 1;
        $kernel->{ticks}++;
        return $kernel;
    });
    my $frame  = $Frame->new(0x1000, {}, 0, 32);
    my $kernel = { ticks => 5 };
    my $result = $r->dispatch(32, $frame, $kernel);
    is($result->{ticks}, 6,  'handler incremented ticks');
    is($called,          1,  'handler was called');
};

subtest 'ISRRegistry — dispatch dies for missing handler' => sub {
    my $r = $ISRReg->new();
    ok(dies { $r->dispatch(99, $Frame->new(), {}) }, 'dies on missing handler');
};

subtest 'ISRRegistry — overwrite handler' => sub {
    my $r = $ISRReg->new();
    $r->register(5, sub { my ($f, $k) = @_; $k->{v} += 1;  $k });
    $r->register(5, sub { my ($f, $k) = @_; $k->{v} += 10; $k });
    my $result = $r->dispatch(5, $Frame->new(), { v => 0 });
    is($result->{v}, 10, 'second handler used');
};

# ============================================================================
# Frame tests
# ============================================================================

subtest 'Frame — new stores fields' => sub {
    my $f = $Frame->new(0xCAFE, { x1 => 42 }, 0xDEAD, 32);
    is($f->{pc},            0xCAFE, 'pc stored');
    is($f->{registers}{x1}, 42,     'registers stored');
    is($f->{mstatus},       0xDEAD, 'mstatus stored');
    is($f->{mcause},        32,     'mcause stored');
};

subtest 'Frame — defaults to zeros' => sub {
    my $f = $Frame->new();
    is($f->{pc},      0, 'pc defaults to 0');
    is($f->{mstatus}, 0, 'mstatus defaults to 0');
    is($f->{mcause},  0, 'mcause defaults to 0');
};

subtest 'Frame — save_context is class method alias' => sub {
    my $f = $Frame->save_context({ a0 => 1 }, 0x2000, 0x3, 32);
    is($f->{pc},     0x2000, 'pc from save_context');
    is($f->{mcause}, 32,     'mcause from save_context');
};

subtest 'Frame — restore_context returns registers, pc, mstatus' => sub {
    my $regs = { a0 => 7, a1 => 8 };
    my $f = $Frame->new(0x3000, $regs, 0xFF, 33);
    my ($r, $pc, $ms) = $f->restore_context();
    is($pc,      0x3000, 'pc restored');
    is($ms,      0xFF,   'mstatus restored');
    is($r->{a0}, 7,      'registers restored');
};

# ============================================================================
# Controller tests
# ============================================================================

subtest 'Controller — new has defaults' => sub {
    my $c = $Controller->new();
    is($c->pending_count(), 0,  'no pending');
    is($c->has_pending(),   0,  'has_pending false');
    is($c->next_pending(),  -1, 'next_pending = -1');
    is($c->{enabled},       1,  'enabled by default');
};

subtest 'Controller — raise adds to pending' => sub {
    my $c = $Controller->new();
    $c->raise(32);
    is($c->pending_count(), 1, 'one pending');
    is($c->has_pending(),   1, 'has_pending true');
    is($c->next_pending(),  32, 'next = 32');
};

subtest 'Controller — raise is idempotent' => sub {
    my $c = $Controller->new();
    $c->raise(32);
    $c->raise(32);
    is($c->pending_count(), 1, 'still one pending');
};

subtest 'Controller — raise sorts by priority' => sub {
    my $c = $Controller->new();
    $c->raise(33);
    $c->raise(32);
    $c->raise(35);
    is($c->next_pending(), 32, 'lowest number first');
};

subtest 'Controller — disable prevents dispatch' => sub {
    my $c = $Controller->new();
    $c->raise(5);
    $c->disable();
    is($c->has_pending(),  0,  'has_pending false when disabled');
    is($c->next_pending(), -1, 'next_pending = -1 when disabled');
};

subtest 'Controller — enable re-enables' => sub {
    my $c = $Controller->new();
    $c->raise(5);
    $c->disable();
    $c->enable();
    is($c->has_pending(), 1, 'has_pending true after enable');
};

subtest 'Controller — acknowledge removes from pending' => sub {
    my $c = $Controller->new();
    $c->raise(32);
    $c->raise(33);
    $c->acknowledge(32);
    is($c->pending_count(), 1,  'one remaining');
    is($c->next_pending(),  33, 'next = 33');
};

subtest 'Controller — set_mask masks an IRQ' => sub {
    my $c = $Controller->new();
    $c->raise(0);
    $c->set_mask(0, 1);
    is($c->has_pending(), 0, 'masked IRQ not pending');
};

subtest 'Controller — set_mask unmasks IRQ' => sub {
    my $c = $Controller->new();
    $c->raise(1);
    $c->set_mask(1, 1);
    is($c->has_pending(), 0, 'masked');
    $c->set_mask(1, 0);
    is($c->has_pending(), 1, 'unmasked');
};

subtest 'Controller — is_masked' => sub {
    my $c = $Controller->new();
    ok(!$c->is_masked(5),  'not masked initially');
    $c->set_mask(5, 1);
    ok($c->is_masked(5),   'now masked');
    ok(!$c->is_masked(32), 'numbers > 31 never masked');
};

subtest 'Controller — set_mask no-ops for number > 31' => sub {
    my $c = $Controller->new();
    my $before = $c->{mask_register};
    $c->set_mask(32, 1);
    is($c->{mask_register}, $before, 'mask unchanged');
};

subtest 'Controller — clear_all empties queue' => sub {
    my $c = $Controller->new();
    $c->raise(10)->raise(20)->raise(30);
    $c->clear_all();
    is($c->pending_count(), 0, 'queue empty');
};

subtest 'Controller — dispatch calls ISR and acknowledges' => sub {
    my $c = $Controller->new();
    $c->register(32, sub { my ($f, $k) = @_; $k->{ticks}++; $k });
    $c->raise(32);
    my $frame  = $Frame->new(0x1000, {}, 0, 32);
    my $kernel = { ticks => 0 };
    my $result = $c->dispatch($frame, $kernel);
    is($result->{ticks},     1, 'ISR called, ticks incremented');
    is($c->pending_count(),  0, 'interrupt acknowledged');
};

subtest 'Controller — dispatch returns kernel unchanged when no pending' => sub {
    my $c      = $Controller->new();
    my $frame  = $Frame->new(0, {}, 0, 0);
    my $kernel = { x => 99 };
    my $result = $c->dispatch($frame, $kernel);
    is($result->{x}, 99, 'unchanged');
};

subtest 'Controller — priority dispatch order' => sub {
    my @order;
    my $c = $Controller->new();
    $c->register(32, sub { push @order, 32; $_[1] });
    $c->register(33, sub { push @order, 33; $_[1] });
    $c->raise(33);
    $c->raise(32);
    my $frame = $Frame->new(0, {}, 0, 0);
    my $k = {};
    $c->dispatch($frame, $k);
    $c->dispatch($frame, $k);
    is(\@order, [32, 33], 'dispatched in priority order');
};

subtest 'Controller — nested interrupt scenario' => sub {
    my @handled;
    my $c = $Controller->new();
    $c->register(32, sub {
        my ($f, $k) = @_;
        push @handled, 32;
        $k->{ctrl}->raise(33);
        return $k;
    });
    $c->register(33, sub {
        my ($f, $k) = @_;
        push @handled, 33;
        return $k;
    });
    $c->raise(32);
    my $frame = $Frame->new(0, {}, 0, 0);
    my $k = { ctrl => $c };
    $c->dispatch($frame, $k);
    $k->{ctrl}->dispatch($frame, $k);
    is(\@handled, [32, 33], 'nested interrupts handled in order');
};

done_testing();
