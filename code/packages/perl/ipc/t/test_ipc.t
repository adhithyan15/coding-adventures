use strict;
use warnings;
use Test2::V0;

use CodingAdventures::IPC;

my $Pipe    = 'CodingAdventures::IPC::Pipe';
my $MQ      = 'CodingAdventures::IPC::MessageQueue';
my $SHM     = 'CodingAdventures::IPC::SharedMemory';
my $Manager = 'CodingAdventures::IPC::Manager';

# ============================================================================
# Constants
# ============================================================================

subtest 'constants' => sub {
    is(CodingAdventures::IPC::DEFAULT_PIPE_CAPACITY,    4096, 'pipe capacity');
    is(CodingAdventures::IPC::DEFAULT_MAX_MESSAGES,      256, 'max messages');
    is(CodingAdventures::IPC::DEFAULT_MAX_MESSAGE_SIZE, 4096, 'max msg size');
};

# ============================================================================
# Pipe tests
# ============================================================================

subtest 'Pipe — new creates pipe' => sub {
    my $p = $Pipe->new(16);
    is($p->{capacity},     16, 'capacity set');
    is($p->available(),     0, 'empty initially');
    ok($p->is_empty(),        'is_empty true');
    ok(!$p->is_full(),        'is_full false');
    is($p->{reader_count}, 1, 'one reader');
    is($p->{writer_count}, 1, 'one writer');
};

subtest 'Pipe — new rejects non-positive capacity' => sub {
    ok(dies { $Pipe->new(0) }, 'dies on 0');
};

subtest 'Pipe — write and read round-trip' => sub {
    my $p = $Pipe->new(16);
    my ($ws, $n) = $p->write("hello");
    is($ws, 'ok', 'write ok');
    is($n,  5,    'wrote 5 bytes');
    is($p->available(), 5, '5 bytes available');
    my ($rs, $data) = $p->read(5);
    is($rs,   'ok',   'read ok');
    is($data, 'hello', 'got hello');
    is($p->available(), 0, 'buffer empty after read');
};

subtest 'Pipe — read returns empty when buffer empty and writers open' => sub {
    my $p = $Pipe->new(8);
    my ($st, $d) = $p->read(4);
    is($st, 'empty', 'empty status');
    is($d,  '',      'empty string');
};

subtest 'Pipe — read returns eof when writers closed and buffer empty' => sub {
    my $p = $Pipe->new(8);
    $p->close_write();
    my ($st, $d) = $p->read(4);
    is($st, 'eof', 'eof status');
    is($d,  '',    'empty string');
};

subtest 'Pipe — write returns broken_pipe when readers closed' => sub {
    my $p = $Pipe->new(8);
    $p->close_read();
    my ($st, $n) = $p->write("hi");
    is($st, 'broken_pipe', 'broken pipe');
    is($n,  0,             'zero written');
};

subtest 'Pipe — write returns full on overflow' => sub {
    my $p = $Pipe->new(4);
    my ($st, $n) = $p->write("hello world");
    is($st, 'full', 'full status');
    is($n,  4,      '4 bytes written');
    ok($p->is_full(), 'is_full true');
};

subtest 'Pipe — circular buffer wraps correctly' => sub {
    my $p = $Pipe->new(8);
    $p->write("abcde");
    $p->read(3);          # consume "abc", 2 remain
    $p->write("fghij");   # 5 more → wraps
    my ($st, $data) = $p->read(7);
    is($st,   'ok',      'read ok');
    is($data, 'defghij', 'correct wrapped data');
};

subtest 'Pipe — partial read' => sub {
    my $p = $Pipe->new(16);
    $p->write("abcdef");
    my ($st, $data) = $p->read(3);
    is($st,   'ok',  'ok');
    is($data, 'abc', '3 bytes');
    is($p->available(), 3, '3 remaining');
};

subtest 'Pipe — multiple writes' => sub {
    my $p = $Pipe->new(64);
    $p->write("first ");
    $p->write("second");
    my ($st, $data) = $p->read(12);
    is($data, 'first second', 'concatenated');
};

# ============================================================================
# MessageQueue tests
# ============================================================================

subtest 'MessageQueue — new creates empty queue' => sub {
    my $q = $MQ->new();
    ok($q->is_empty(),   'empty');
    ok(!$q->is_full(),   'not full');
    is($q->{message_count}, 0, 'count 0');
};

subtest 'MessageQueue — send and receive' => sub {
    my $q = $MQ->new();
    is($q->send(1, 'hello'), 'ok', 'send ok');
    my ($st, $msg) = $q->receive(0);
    is($st,           'ok',   'receive ok');
    is($msg->{body},  'hello', 'body correct');
    is($msg->{msg_type}, 1,   'type correct');
    ok($q->is_empty(), 'empty after receive');
};

subtest 'MessageQueue — FIFO order' => sub {
    my $q = $MQ->new();
    $q->send(1, 'first');
    $q->send(1, 'second');
    $q->send(1, 'third');
    my (undef, $m1) = $q->receive(0);
    my (undef, $m2) = $q->receive(0);
    my (undef, $m3) = $q->receive(0);
    is($m1->{body}, 'first',  'first');
    is($m2->{body}, 'second', 'second');
    is($m3->{body}, 'third',  'third');
};

subtest 'MessageQueue — type-filtered receive' => sub {
    my $q = $MQ->new();
    $q->send(1, 'type1');
    $q->send(2, 'type2');
    $q->send(1, 'type1-again');
    my ($st, $msg) = $q->receive(2);
    is($st,          'ok',    'ok');
    is($msg->{body}, 'type2', 'got type2');
    is($q->{message_count}, 2, '2 remaining');
};

subtest 'MessageQueue — receive from empty returns empty' => sub {
    my $q = $MQ->new();
    my ($st, $msg) = $q->receive(0);
    is($st, 'empty', 'empty status');
    is($msg, undef,  'undef msg');
};

subtest 'MessageQueue — receive type not found returns empty' => sub {
    my $q = $MQ->new();
    $q->send(1, 'a');
    my ($st, $msg) = $q->receive(99);
    is($st, 'empty', 'empty for missing type');
};

subtest 'MessageQueue — oversized message rejected' => sub {
    my $q = $MQ->new(10, 5);
    is($q->send(1, 'toolong'), 'oversized', 'oversized rejected');
    is($q->{message_count}, 0, 'count unchanged');
};

subtest 'MessageQueue — full queue rejects new messages' => sub {
    my $q = $MQ->new(2, 4096);
    $q->send(1, 'a');
    $q->send(1, 'b');
    ok($q->is_full(), 'is_full');
    is($q->send(1, 'c'), 'full', 'full status');
};

# ============================================================================
# SharedMemory tests
# ============================================================================

subtest 'SharedMemory — new creates zero-initialized region' => sub {
    my $s = $SHM->new('seg1', 16, 100);
    is($s->{region_name},  'seg1', 'name');
    is($s->{region_size},  16,     'size');
    is($s->{owner_pid},    100,    'owner');
    is($s->attached_count(), 0,   'no attached');
};

subtest 'SharedMemory — new rejects non-positive size' => sub {
    ok(dies { $SHM->new('x', 0, 1) }, 'dies on size 0');
};

subtest 'SharedMemory — write and read round-trip' => sub {
    my $s = $SHM->new('test', 32, 1);
    my ($ws, $n) = $s->write(0, 'hello');
    is($ws, 'ok', 'write ok');
    is($n,  5,    '5 bytes written');
    my ($rs, $data) = $s->read(0, 5);
    is($rs,   'ok',   'read ok');
    is($data, 'hello', 'data correct');
};

subtest 'SharedMemory — write at offset' => sub {
    my $s = $SHM->new('test', 32, 1);
    $s->write(10, 'world');
    my (undef, $data) = $s->read(10, 5);
    is($data, 'world', 'offset write/read');
};

subtest 'SharedMemory — read out of bounds' => sub {
    my $s = $SHM->new('test', 8, 1);
    my ($st, $d) = $s->read(6, 5);  # 6+5=11 > 8
    is($st, 'out_of_bounds', 'out_of_bounds');
    is($d,  undef,           'undef data');
};

subtest 'SharedMemory — write out of bounds' => sub {
    my $s = $SHM->new('test', 8, 1);
    my ($st, $n) = $s->write(6, 'toolong');
    is($st, 'out_of_bounds', 'out_of_bounds');
    is($n,  0,               '0 bytes written');
};

subtest 'SharedMemory — attach and detach' => sub {
    my $s = $SHM->new('seg', 64, 1);
    is($s->attach(200), 'ok',  'attached');
    ok($s->is_attached(200),   'is_attached true');
    is($s->attached_count(), 1, 'count 1');
    is($s->detach(200), 'ok',  'detached');
    ok(!$s->is_attached(200),  'is_attached false');
    is($s->attached_count(), 0, 'count 0');
};

subtest 'SharedMemory — attach already attached returns error' => sub {
    my $s = $SHM->new('seg', 64, 1);
    $s->attach(200);
    is($s->attach(200), 'already_attached', 'double attach');
    is($s->attached_count(), 1, 'still 1');
};

subtest 'SharedMemory — detach not attached returns error' => sub {
    my $s = $SHM->new('seg', 64, 1);
    is($s->detach(999), 'not_attached', 'not attached');
};

subtest 'SharedMemory — multiple pids attach' => sub {
    my $s = $SHM->new('seg', 64, 1);
    $s->attach(10);
    $s->attach(20);
    $s->attach(30);
    is($s->attached_count(), 3,   'count 3');
    ok($s->is_attached(10),      'pid 10');
    ok($s->is_attached(20),      'pid 20');
    ok($s->is_attached(30),      'pid 30');
};

# ============================================================================
# Manager tests
# ============================================================================

subtest 'Manager — new creates empty manager' => sub {
    my $m = $Manager->new();
    is($m->{next_pipe_id}, 0,   'next_pipe_id 0');
    is($m->{next_fd},      100, 'next_fd 100');
};

subtest 'Manager — create_pipe returns handle' => sub {
    my $m = $Manager->new();
    my $h = $m->create_pipe();
    is($h->{pipe_id},  0,   'pipe_id 0');
    is($h->{read_fd},  100, 'read_fd 100');
    is($h->{write_fd}, 101, 'write_fd 101');
    is($m->{next_pipe_id}, 1, 'next_pipe_id incremented');
};

subtest 'Manager — get_pipe returns pipe' => sub {
    my $m = $Manager->new();
    my $h = $m->create_pipe();
    my ($st, $pipe) = $m->get_pipe($h->{pipe_id});
    is($st, 'ok', 'ok');
    ok(defined $pipe, 'pipe defined');
};

subtest 'Manager — get_pipe returns not_found' => sub {
    my $m = $Manager->new();
    my ($st, $p) = $m->get_pipe(999);
    is($st, 'not_found', 'not found');
    is($p,  undef,       'undef');
};

subtest 'Manager — destroy_pipe' => sub {
    my $m = $Manager->new();
    my $h = $m->create_pipe();
    is($m->destroy_pipe($h->{pipe_id}), 'ok', 'destroyed');
    my ($st, $_) = $m->get_pipe($h->{pipe_id});
    is($st, 'not_found', 'gone');
};

subtest 'Manager — destroy_pipe not found' => sub {
    my $m = $Manager->new();
    is($m->destroy_pipe(42), 'not_found', 'not found');
};

subtest 'Manager — message queue lifecycle' => sub {
    my $m = $Manager->new();
    my $q = $m->create_message_queue('q');
    ok(defined $q, 'queue created');
    my ($st, $q2) = $m->get_message_queue('q');
    is($st, 'ok', 'found');
    is($m->destroy_message_queue('q'), 'ok', 'destroyed');
    my ($rs, $_) = $m->get_message_queue('q');
    is($rs, 'not_found', 'gone');
};

subtest 'Manager — shared memory lifecycle' => sub {
    my $m = $Manager->new();
    my $s = $m->create_shared_memory('seg', 64, 1);
    ok(defined $s, 'shm created');
    my ($st, $s2) = $m->get_shared_memory('seg');
    is($st, 'ok', 'found');
    is($m->destroy_shared_memory('seg'), 'ok', 'destroyed');
    my ($rs, $_) = $m->get_shared_memory('seg');
    is($rs, 'not_found', 'gone');
};

subtest 'Manager — end-to-end pipe workflow' => sub {
    my $m = $Manager->new();
    my $h = $m->create_pipe(64);
    my (undef, $pipe) = $m->get_pipe($h->{pipe_id});
    $pipe->write("ping");
    my ($st, $data) = $pipe->read(4);
    is($st,   'ok',  'read ok');
    is($data, 'ping', 'got ping');
};

subtest 'Manager — create_message_queue idempotent' => sub {
    my $m = $Manager->new();
    my $q1 = $m->create_message_queue('q');
    $q1->send(1, 'msg');
    my $q2 = $m->create_message_queue('q');  # should return existing
    is($q2->{message_count}, 1, 'existing queue returned');
};

done_testing();
