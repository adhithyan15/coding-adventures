package CodingAdventures::IPC;

# ============================================================================
# CodingAdventures::IPC — Inter-Process Communication
# ============================================================================
#
# Processes are isolated by design.  Each process has its own address space,
# file descriptors, and register state.  This isolation is essential for
# stability and security.
#
# But isolation creates a problem: how do processes collaborate?
#
#   - A web server forks workers that all need a shared request queue.
#   - A shell pipeline `ls | grep foo | wc -l` links three processes.
#   - A database uses shared memory so workers can read cached pages.
#
# IPC (Inter-Process Communication) provides kernel mechanisms for isolated
# processes to exchange data.
#
# ## Three Mechanisms
#
#   1. Pipes         — unidirectional byte streams
#   2. Message Queues — FIFO of typed messages
#   3. Shared Memory — named byte region mapped into multiple address spaces
#
# ## Analogy: Two Soundproofed Rooms
#
#   Pipe:          A pneumatic tube — bytes in one end, out the other.
#   Message Queue: A shared mailbox — labeled envelopes, picked up by type.
#   Shared Memory: A whiteboard through a window — fastest, no copy, but
#                  you must take turns writing.
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use constant DEFAULT_PIPE_CAPACITY    => 4096;
use constant DEFAULT_MAX_MESSAGES     => 256;
use constant DEFAULT_MAX_MESSAGE_SIZE => 4096;

# ============================================================================
# Pipe — Unidirectional Byte Stream
# ============================================================================
#
# A pipe is a circular buffer with read and write ends.
#
#   ┌───┬───┬───┬───┬───┬───┬───┬───┐
#   │ h │ e │ l │ l │ o │   │   │   │   capacity = 8
#   └───┴───┴───┴───┴───┴───┴───┴───┘
#     ▲ read_pos=0          ▲ write_pos=5   count=5
#
# EOF: all writers closed + buffer empty.
# Broken pipe: all readers closed.

package CodingAdventures::IPC::Pipe;

sub new {
    my ($class, $capacity) = @_;
    $capacity //= CodingAdventures::IPC::DEFAULT_PIPE_CAPACITY;
    die "Pipe capacity must be positive" if $capacity <= 0;
    return bless {
        buffer       => [(0) x $capacity],
        capacity     => $capacity,
        read_pos     => 0,
        write_pos    => 0,
        count        => 0,
        reader_count => 1,
        writer_count => 1,
    }, $class;
}

sub write {
    my ($self, $bytes) = @_;
    return ('broken_pipe', 0) if $self->{reader_count} <= 0;
    my $written = 0;
    for my $i (0 .. length($bytes) - 1) {
        last if $self->{count} >= $self->{capacity};
        my $idx = $self->{write_pos} % $self->{capacity};
        $self->{buffer}[$idx] = ord(substr($bytes, $i, 1));
        $self->{write_pos}++;
        $self->{count}++;
        $written++;
    }
    return $written < length($bytes) ? ('full', $written) : ('ok', $written);
}

sub read {
    my ($self, $max_bytes) = @_;
    if ($self->{count} == 0) {
        return $self->{writer_count} <= 0 ? ('eof', '') : ('empty', '');
    }
    my $to_read = $max_bytes < $self->{count} ? $max_bytes : $self->{count};
    my $result  = '';
    for (1 .. $to_read) {
        my $idx = $self->{read_pos} % $self->{capacity};
        $result .= chr($self->{buffer}[$idx]);
        $self->{read_pos}++;
        $self->{count}--;
    }
    return ('ok', $result);
}

sub close_read  { my $s = shift; $s->{reader_count} = $s->{reader_count} > 0 ? $s->{reader_count} - 1 : 0; $s }
sub close_write { my $s = shift; $s->{writer_count} = $s->{writer_count} > 0 ? $s->{writer_count} - 1 : 0; $s }
sub available   { $_[0]->{count} }
sub is_full     { $_[0]->{count} >= $_[0]->{capacity} }
sub is_empty    { $_[0]->{count} == 0 }

# ============================================================================
# MessageQueue — Typed FIFO Queue
# ============================================================================
#
# Messages have boundaries (unlike byte streams) and a type tag.
# Receivers can filter by type: receive(0) = any, receive(N) = type N only.

package CodingAdventures::IPC::MessageQueue;

sub new {
    my ($class, $max_messages, $max_message_size) = @_;
    return bless {
        messages         => [],
        message_count    => 0,
        max_messages     => $max_messages     // CodingAdventures::IPC::DEFAULT_MAX_MESSAGES,
        max_message_size => $max_message_size // CodingAdventures::IPC::DEFAULT_MAX_MESSAGE_SIZE,
    }, $class;
}

sub send {
    my ($self, $msg_type, $data) = @_;
    $data //= '';
    return 'oversized' if length($data) > $self->{max_message_size};
    return 'full'      if $self->{message_count} >= $self->{max_messages};
    push @{ $self->{messages} }, {
        msg_type => $msg_type,
        body     => $data,
        msg_size => length($data),
    };
    $self->{message_count}++;
    return 'ok';
}

sub receive {
    my ($self, $msg_type) = @_;
    $msg_type //= 0;
    return ('empty', undef) if $self->{message_count} == 0;
    if ($msg_type == 0) {
        my $msg = shift @{ $self->{messages} };
        $self->{message_count}--;
        return ('ok', $msg);
    }
    for my $i (0 .. $#{ $self->{messages} }) {
        if ($self->{messages}[$i]{msg_type} == $msg_type) {
            my $msg = splice(@{ $self->{messages} }, $i, 1);
            $self->{message_count}--;
            return ('ok', $msg);
        }
    }
    return ('empty', undef);
}

sub is_empty { $_[0]->{message_count} == 0 }
sub is_full  { $_[0]->{message_count} >= $_[0]->{max_messages} }

# ============================================================================
# SharedMemory — Zero-Copy Named Memory Region
# ============================================================================
#
# A region of bytes accessible by any process that attaches to it.
# No synchronization is provided — callers must coordinate externally.

package CodingAdventures::IPC::SharedMemory;

sub new {
    my ($class, $region_name, $region_size, $owner_pid) = @_;
    die "Shared memory size must be positive" if $region_size <= 0;
    return bless {
        region_name   => $region_name,
        region_size   => $region_size,
        data          => "\x00" x $region_size,
        owner_pid     => $owner_pid,
        attached_pids => {},
    }, $class;
}

sub attach {
    my ($self, $pid) = @_;
    return 'already_attached' if $self->{attached_pids}{$pid};
    $self->{attached_pids}{$pid} = 1;
    return 'ok';
}

sub detach {
    my ($self, $pid) = @_;
    return 'not_attached' unless $self->{attached_pids}{$pid};
    delete $self->{attached_pids}{$pid};
    return 'ok';
}

sub read {
    my ($self, $offset, $byte_count) = @_;
    return ('out_of_bounds', undef)
        if $offset < 0 || $byte_count < 0
        || ($offset + $byte_count) > $self->{region_size};
    return ('ok', substr($self->{data}, $offset, $byte_count));
}

sub write {
    my ($self, $offset, $bytes) = @_;
    my $len = length($bytes);
    return ('out_of_bounds', 0)
        if $offset < 0 || ($offset + $len) > $self->{region_size};
    substr($self->{data}, $offset, $len) = $bytes;
    return ('ok', $len);
}

sub attached_count {
    my ($self) = @_;
    return scalar keys %{ $self->{attached_pids} };
}

sub is_attached {
    my ($self, $pid) = @_;
    return $self->{attached_pids}{$pid} ? 1 : 0;
}

# ============================================================================
# Manager — Kernel IPC Coordinator
# ============================================================================

package CodingAdventures::IPC::Manager;

sub new {
    my ($class) = @_;
    return bless {
        pipes          => {},
        message_queues => {},
        shared_regions => {},
        next_pipe_id   => 0,
        next_fd        => 100,
    }, $class;
}

sub create_pipe {
    my ($self, $capacity) = @_;
    $capacity //= CodingAdventures::IPC::DEFAULT_PIPE_CAPACITY;
    my $pipe     = CodingAdventures::IPC::Pipe->new($capacity);
    my $pipe_id  = $self->{next_pipe_id}++;
    my $read_fd  = $self->{next_fd}++;
    my $write_fd = $self->{next_fd}++;
    $self->{pipes}{$pipe_id} = $pipe;
    return { pipe_id => $pipe_id, read_fd => $read_fd, write_fd => $write_fd };
}

sub get_pipe {
    my ($self, $pipe_id) = @_;
    return exists $self->{pipes}{$pipe_id}
        ? ('ok', $self->{pipes}{$pipe_id})
        : ('not_found', undef);
}

sub destroy_pipe {
    my ($self, $pipe_id) = @_;
    return 'not_found' unless exists $self->{pipes}{$pipe_id};
    delete $self->{pipes}{$pipe_id};
    return 'ok';
}

sub create_message_queue {
    my ($self, $name, $max_messages, $max_message_size) = @_;
    $self->{message_queues}{$name} //=
        CodingAdventures::IPC::MessageQueue->new($max_messages, $max_message_size);
    return $self->{message_queues}{$name};
}

sub get_message_queue {
    my ($self, $name) = @_;
    return exists $self->{message_queues}{$name}
        ? ('ok', $self->{message_queues}{$name})
        : ('not_found', undef);
}

sub destroy_message_queue {
    my ($self, $name) = @_;
    return 'not_found' unless exists $self->{message_queues}{$name};
    delete $self->{message_queues}{$name};
    return 'ok';
}

sub create_shared_memory {
    my ($self, $name, $size, $owner_pid) = @_;
    $self->{shared_regions}{$name} //=
        CodingAdventures::IPC::SharedMemory->new($name, $size, $owner_pid);
    return $self->{shared_regions}{$name};
}

sub get_shared_memory {
    my ($self, $name) = @_;
    return exists $self->{shared_regions}{$name}
        ? ('ok', $self->{shared_regions}{$name})
        : ('not_found', undef);
}

sub destroy_shared_memory {
    my ($self, $name) = @_;
    return 'not_found' unless exists $self->{shared_regions}{$name};
    delete $self->{shared_regions}{$name};
    return 'ok';
}

# ============================================================================
# Top-level package
# ============================================================================

package CodingAdventures::IPC;

=head1 NAME

CodingAdventures::IPC - Inter-process communication: pipes, message queues, shared memory

=head1 SYNOPSIS

  use CodingAdventures::IPC;

  my $pipe = CodingAdventures::IPC::Pipe->new(64);
  $pipe->write("hello");
  my ($status, $data) = $pipe->read(5);  # $data eq "hello"

  my $mq = CodingAdventures::IPC::MessageQueue->new();
  $mq->send(1, "ping");
  my ($st, $msg) = $mq->receive(1);     # $msg->{body} eq "ping"

  my $shm = CodingAdventures::IPC::SharedMemory->new("seg", 64, 1);
  $shm->write(0, "shared");
  my ($rs, $bytes) = $shm->read(0, 6);  # $bytes eq "shared"

=cut

1;
