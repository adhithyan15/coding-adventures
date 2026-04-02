use strict;
use warnings;
use Test2::V0;

use CodingAdventures::ProcessManager;

my $PM  = 'CodingAdventures::ProcessManager';
my $PCB = 'CodingAdventures::ProcessManager::PCB';
my $Mgr = 'CodingAdventures::ProcessManager::Manager';

# ============================================================================
# Constants
# ============================================================================

subtest 'constants — states' => sub {
    is($PM->STATE_READY,      0, 'READY=0');
    is($PM->STATE_RUNNING,    1, 'RUNNING=1');
    is($PM->STATE_BLOCKED,    2, 'BLOCKED=2');
    is($PM->STATE_TERMINATED, 3, 'TERMINATED=3');
    is($PM->STATE_ZOMBIE,     4, 'ZOMBIE=4');
};

subtest 'constants — signals' => sub {
    is($PM->SIGINT,  2,  'SIGINT=2');
    is($PM->SIGKILL, 9,  'SIGKILL=9');
    is($PM->SIGTERM, 15, 'SIGTERM=15');
    is($PM->SIGCHLD, 17, 'SIGCHLD=17');
    is($PM->SIGCONT, 18, 'SIGCONT=18');
    is($PM->SIGSTOP, 19, 'SIGSTOP=19');
};

subtest 'constants — priority' => sub {
    is($PM->DEFAULT_PRIORITY, 20, 'default 20');
    is($PM->MIN_PRIORITY,      0, 'min 0');
    is($PM->MAX_PRIORITY,     39, 'max 39');
};

# ============================================================================
# PCB tests
# ============================================================================

subtest 'PCB — new with defaults' => sub {
    my $pcb = $PCB->new(1, 'init');
    is($pcb->{pid},       1,              'pid');
    is($pcb->{name},      'init',         'name');
    is($pcb->{state},     $PM->STATE_READY, 'ready');
    is($pcb->{priority},  20,             'priority 20');
    is($pcb->{cpu_time},  0,              'cpu_time 0');
    is($pcb->{exit_code}, 0,              'exit_code 0');
    is($pcb->{parent_pid}, 0,             'parent_pid 0');
    is(scalar @{ $pcb->{children} }, 0,   'no children');
    is(scalar @{ $pcb->{registers} }, 32, '32 registers');
};

subtest 'PCB — new with options' => sub {
    my $pcb = $PCB->new(2, 'ls', { priority => 5, parent_pid => 1 });
    is($pcb->{priority},   5, 'priority 5');
    is($pcb->{parent_pid}, 1, 'parent 1');
};

subtest 'PCB — set_state' => sub {
    my $pcb = $PCB->new(1, 'p');
    $pcb->set_state($PM->STATE_RUNNING);
    is($pcb->{state}, $PM->STATE_RUNNING, 'running');
};

subtest 'PCB — save_context' => sub {
    my $pcb = $PCB->new(1, 'p');
    my @regs = map { $_ * 2 } 1..32;
    $pcb->save_context(\@regs, 0xCAFE, 0xFF00);
    is($pcb->{pc},           0xCAFE, 'pc saved');
    is($pcb->{sp},           0xFF00, 'sp saved');
    is($pcb->{registers}[0], 2,      'register saved');
};

subtest 'PCB — add_signal queues signal' => sub {
    my $pcb = $PCB->new(1, 'p');
    $pcb->add_signal($PM->SIGTERM);
    is(scalar @{ $pcb->{pending_signals} }, 1, 'one signal');
    is($pcb->{pending_signals}[0], $PM->SIGTERM, 'SIGTERM');
};

subtest 'PCB — add_signal idempotent' => sub {
    my $pcb = $PCB->new(1, 'p');
    $pcb->add_signal($PM->SIGTERM);
    $pcb->add_signal($PM->SIGTERM);
    is(scalar @{ $pcb->{pending_signals} }, 1, 'still 1');
};

subtest 'PCB — mask/unmask/is_masked' => sub {
    my $pcb = $PCB->new(1, 'p');
    ok(!$pcb->is_masked($PM->SIGTERM), 'not masked initially');
    $pcb->mask_signal($PM->SIGTERM);
    ok($pcb->is_masked($PM->SIGTERM), 'masked');
    $pcb->unmask_signal($PM->SIGTERM);
    ok(!$pcb->is_masked($PM->SIGTERM), 'unmasked');
};

subtest 'PCB — set_handler' => sub {
    my $pcb = $PCB->new(1, 'p');
    $pcb->set_handler($PM->SIGTERM, 0xDEAD);
    is($pcb->{signal_handlers}{$PM->SIGTERM}, 0xDEAD, 'handler set');
};

subtest 'PCB — tick_cpu' => sub {
    my $pcb = $PCB->new(1, 'p');
    $pcb->tick_cpu(100);
    is($pcb->{cpu_time}, 100, '100 cycles');
    $pcb->tick_cpu(50);
    is($pcb->{cpu_time}, 150, '150 total');
    $pcb->tick_cpu();
    is($pcb->{cpu_time}, 151, 'defaults to 1');
};

# ============================================================================
# Manager tests
# ============================================================================

subtest 'Manager — new creates empty manager' => sub {
    my $m = $Mgr->new();
    is($m->{next_pid}, 1,     'next_pid 1');
    is($m->total_processes(), 0, 'no processes');
};

subtest 'Manager — spawn creates process' => sub {
    my $m   = $Mgr->new();
    my $pid = $m->spawn('init');
    is($pid, 1, 'pid 1');
    is($m->{next_pid}, 2, 'next_pid 2');
    is($m->total_processes(), 1, '1 process');
    ok(defined $m->get($pid), 'pcb exists');
    is($m->get($pid)->{name}, 'init', 'name init');
};

subtest 'Manager — spawn adds to run queue' => sub {
    my $m   = $Mgr->new();
    my $pid = $m->spawn('p');
    is(scalar @{ $m->{run_queue} }, 1, '1 in queue');
    is($m->{run_queue}[0], $pid, 'pid in queue');
};

subtest 'Manager — spawn assigns sequential PIDs' => sub {
    my $m  = $Mgr->new();
    my $p1 = $m->spawn('p1');
    my $p2 = $m->spawn('p2');
    isnt($p1, $p2, 'different PIDs');
    is($p2, $p1 + 1, 'sequential');
};

subtest 'Manager — fork clones parent' => sub {
    my $m    = $Mgr->new();
    my $ppid = $m->spawn('shell');
    my $cpid = $m->fork($ppid);
    isnt($ppid, $cpid, 'different PIDs');
    is($m->get($cpid)->{parent_pid}, $ppid, 'parent_pid set');
    is($m->get($cpid)->{state}, $PM->STATE_READY, 'child ready');
    is(scalar @{ $m->get($ppid)->{children} }, 1, 'parent has 1 child');
    is($m->get($ppid)->{children}[0], $cpid, 'child recorded in parent');
};

subtest 'Manager — fork dies for missing parent' => sub {
    my $m = $Mgr->new();
    ok(dies { $m->fork(999) }, 'dies on missing parent');
};

subtest 'Manager — exec replaces program' => sub {
    my $m   = $Mgr->new();
    my $pid = $m->spawn('shell');
    $m->exec($pid, 'ls', { pc => 0x4000 });
    is($m->get($pid)->{name},       'ls',    'name changed');
    is($m->get($pid)->{pc},         0x4000,  'pc set');
    is($m->get($pid)->{registers}[0], 0,     'registers zeroed');
};

subtest 'Manager — exec dies for missing process' => sub {
    my $m = $Mgr->new();
    ok(dies { $m->exec(999, 'ls') }, 'dies');
};

subtest 'Manager — schedule picks ready process' => sub {
    my $m   = $Mgr->new();
    my $pid = $m->spawn('p');
    my $chosen = $m->schedule();
    is($chosen, $pid, 'chose pid');
    is($m->get($pid)->{state}, $PM->STATE_RUNNING, 'now running');
    is($m->{current_pid}, $pid, 'current_pid set');
};

subtest 'Manager — schedule returns undef when no ready processes' => sub {
    my $m      = $Mgr->new();
    my $chosen = $m->schedule();
    is($chosen, undef, 'undef when empty');
};

subtest 'Manager — schedule respects priority' => sub {
    my $m    = $Mgr->new();
    my $high = $m->spawn('high', { priority => 5 });
    my $low  = $m->spawn('low',  { priority => 30 });
    my $chosen = $m->schedule();
    is($chosen, $high, 'high priority chosen first');
};

subtest 'Manager — block moves process out of queue' => sub {
    my $m   = $Mgr->new();
    my $pid = $m->spawn('p');
    $m->schedule();
    $m->block($pid);
    is($m->get($pid)->{state}, $PM->STATE_BLOCKED, 'blocked');
    is(scalar @{ $m->{run_queue} }, 0, 'run queue empty');
};

subtest 'Manager — unblock restores to ready' => sub {
    my $m   = $Mgr->new();
    my $pid = $m->spawn('p');
    $m->schedule();
    $m->block($pid);
    $m->unblock($pid);
    is($m->get($pid)->{state}, $PM->STATE_READY, 'ready again');
    is(scalar @{ $m->{run_queue} }, 1, 'back in queue');
};

subtest 'Manager — unblock no-ops for non-blocked process' => sub {
    my $m   = $Mgr->new();
    my $pid = $m->spawn('p');
    $m->unblock($pid);  # already ready, not blocked
    is($m->get($pid)->{state}, $PM->STATE_READY, 'still ready');
};

subtest 'Manager — exit_process sets zombie' => sub {
    my $m   = $Mgr->new();
    my $pid = $m->spawn('p');
    $m->exit_process($pid, 42);
    is($m->get($pid)->{state},     $PM->STATE_ZOMBIE, 'zombie');
    is($m->get($pid)->{exit_code}, 42,                'exit code 42');
};

subtest 'Manager — exit_process sends SIGCHLD to parent' => sub {
    my $m    = $Mgr->new();
    my $ppid = $m->spawn('shell');
    my $cpid = $m->fork($ppid);
    $m->exit_process($cpid, 0);
    my $parent = $m->get($ppid);
    ok(grep { $_ == $PM->SIGCHLD } @{ $parent->{pending_signals} }, 'SIGCHLD queued');
};

subtest 'Manager — wait_child reaps zombie' => sub {
    my $m    = $Mgr->new();
    my $ppid = $m->spawn('shell');
    my $cpid = $m->fork($ppid);
    $m->exit_process($cpid, 99);
    my ($st, $ec) = $m->wait_child($ppid, $cpid);
    is($st, 'ok', 'ok');
    is($ec, 99,   'exit code 99');
    ok(!defined $m->get($cpid), 'child removed');
    is(scalar @{ $m->get($ppid)->{children} }, 0, 'child removed from parent');
};

subtest 'Manager — wait_child not_exited when still running' => sub {
    my $m    = $Mgr->new();
    my $ppid = $m->spawn('shell');
    my $cpid = $m->fork($ppid);
    my ($st, $ignored) = $m->wait_child($ppid, $cpid);
    is($st, 'not_exited', 'not exited');
};

subtest 'Manager — wait_child no_child for non-child' => sub {
    my $m  = $Mgr->new();
    my $p1 = $m->spawn('p1');
    my $p2 = $m->spawn('p2');
    my ($st, $ignored) = $m->wait_child($p1, $p2);
    is($st, 'no_child', 'no child');
};

subtest 'Manager — kill SIGKILL terminates immediately' => sub {
    my $m   = $Mgr->new();
    my $pid = $m->spawn('p');
    $m->kill($pid, $PM->SIGKILL);
    is($m->get($pid)->{state}, $PM->STATE_ZOMBIE, 'zombie');
};

subtest 'Manager — kill SIGSTOP blocks process' => sub {
    my $m   = $Mgr->new();
    my $pid = $m->spawn('p');
    $m->kill($pid, $PM->SIGSTOP);
    is($m->get($pid)->{state}, $PM->STATE_BLOCKED, 'blocked');
    is(scalar @{ $m->{run_queue} }, 0, 'removed from queue');
};

subtest 'Manager — kill SIGCONT resumes blocked process' => sub {
    my $m   = $Mgr->new();
    my $pid = $m->spawn('p');
    $m->kill($pid, $PM->SIGSTOP);
    $m->kill($pid, $PM->SIGCONT);
    is($m->get($pid)->{state}, $PM->STATE_READY, 'ready again');
    is(scalar @{ $m->{run_queue} }, 1, 'back in queue');
};

subtest 'Manager — kill SIGTERM queues signal' => sub {
    my $m   = $Mgr->new();
    my $pid = $m->spawn('p');
    $m->kill($pid, $PM->SIGTERM);
    ok(grep { $_ == $PM->SIGTERM } @{ $m->get($pid)->{pending_signals} }, 'SIGTERM queued');
};

subtest 'Manager — kill no-ops for missing process' => sub {
    my $m = $Mgr->new();
    $m->kill(999, $PM->SIGTERM);
    is($m->total_processes(), 0, 'no processes');
};

subtest 'Manager — count_in_state' => sub {
    my $m  = $Mgr->new();
    my $p1 = $m->spawn('p1');
    my $p2 = $m->spawn('p2');
    $m->schedule();
    is($m->count_in_state($PM->STATE_RUNNING), 1, '1 running');
    is($m->count_in_state($PM->STATE_READY),   1, '1 ready');
    is($m->count_in_state($PM->STATE_BLOCKED),  0, '0 blocked');
};

subtest 'Manager — full fork-exec-wait lifecycle' => sub {
    my $m    = $Mgr->new();
    my $init = $m->spawn('init');
    my $ls   = $m->fork($init);
    $m->exec($ls, 'ls', { pc => 0x4000 });
    is($m->get($ls)->{name}, 'ls', 'exec applied');
    $m->exit_process($ls, 0);
    is($m->get($ls)->{state}, $PM->STATE_ZOMBIE, 'zombie');
    my ($st, $ec) = $m->wait_child($init, $ls);
    is($st, 'ok', 'wait ok');
    is($ec, 0,    'exit code 0');
    ok(!defined $m->get($ls), 'child reaped');
};

done_testing();
