# ipc (Perl)

Inter-process communication for the coding-adventures simulated OS.

## What It Does

- `Pipe` — circular buffer byte stream (broken-pipe and EOF detection)
- `MessageQueue` — FIFO of typed messages with type-filtered receive
- `SharedMemory` — named byte region with attach/detach and bounds checking
- `Manager` — kernel IPC coordinator

## Usage

```perl
use CodingAdventures::IPC;

# Pipe
my $pipe = CodingAdventures::IPC::Pipe->new(64);
$pipe->write("hello");
my ($status, $data) = $pipe->read(5);  # $data eq "hello"

# Message Queue
my $mq = CodingAdventures::IPC::MessageQueue->new();
$mq->send(1, "ping");
my ($st, $msg) = $mq->receive(1);  # $msg->{body} eq "ping"

# Shared Memory
my $shm = CodingAdventures::IPC::SharedMemory->new("seg", 64, 1);
$shm->write(0, "data");
my ($rs, $bytes) = $shm->read(0, 4);  # $bytes eq "data"
```
