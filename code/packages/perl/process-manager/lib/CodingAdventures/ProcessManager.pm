package CodingAdventures::ProcessManager;

# ============================================================================
# CodingAdventures::ProcessManager — Process Lifecycle Management
# ============================================================================
#
# Every program running on a computer is a process.  Unix created three
# elegant system calls to manage processes: fork, exec, and wait.
#
# ## The Restaurant Kitchen Analogy
#
#   fork()  — Clone the head chef.  Now there are two identical chefs.
#   exec()  — The clone throws away their recipe book and picks up a new one.
#             Same person (same PID), completely different work.
#   wait()  — The head chef pauses and watches the clone work.  Resumes when
#             the clone finishes.
#
# ## Process State Machine
#
#   fork() → ready ──[schedule()]──► running ──[exit()]──► zombie ──[wait()]──► removed
#              ▲                        │
#              │                        ▼
#              └────── blocked ◄──[block()]
#                       unblock()
#
# ## Signals
#
#   +----------+--------+-------------------+-------------+
#   | Name     | Number | Default Action    | Catchable?  |
#   +----------+--------+-------------------+-------------+
#   | SIGINT   |   2    | Terminate         | Yes         |
#   | SIGKILL  |   9    | Terminate         | NO          |
#   | SIGTERM  |  15    | Terminate         | Yes         |
#   | SIGCHLD  |  17    | Ignore            | Yes         |
#   | SIGCONT  |  18    | Continue          | Yes         |
#   | SIGSTOP  |  19    | Stop              | NO          |
#   +----------+--------+-------------------+-------------+
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# State constants
use constant STATE_READY      => 0;
use constant STATE_RUNNING    => 1;
use constant STATE_BLOCKED    => 2;
use constant STATE_TERMINATED => 3;
use constant STATE_ZOMBIE     => 4;

# Signal constants
use constant SIGINT  =>  2;
use constant SIGKILL =>  9;
use constant SIGTERM => 15;
use constant SIGCHLD => 17;
use constant SIGCONT => 18;
use constant SIGSTOP => 19;

use constant DEFAULT_PRIORITY => 20;
use constant MIN_PRIORITY     =>  0;
use constant MAX_PRIORITY     => 39;

# ============================================================================
# PCB — Process Control Block
# ============================================================================
#
# The PCB is the kernel's "passport" for each process.  It stores everything
# needed to suspend, resume, and identify the process.
#
#   pid              Unique process ID
#   name             Human-readable name ("ls", "init", "shell")
#   state            Current lifecycle state (0..4)
#   registers        Arrayref of 32 register values
#   pc               Program counter
#   sp               Stack pointer
#   memory_base      Base address of this process's memory region
#   memory_size      Size of memory region
#   parent_pid       PID of the creator (0 for init)
#   children         Arrayref of child PIDs
#   pending_signals  Arrayref of queued signal numbers
#   signal_handlers  Hashref of signal_number → handler address
#   signal_mask      Hashref of signal_number → 1 (blocked signals)
#   priority         Scheduling priority (0=highest, 39=lowest)
#   cpu_time         Total CPU cycles consumed
#   exit_code        Exit status (meaningful when state=zombie)

package CodingAdventures::ProcessManager::PCB;

sub new {
    my ($class, $pid, $name, $opts) = @_;
    $opts //= {};
    return bless {
        pid             => $pid,
        name            => $name,
        state           => CodingAdventures::ProcessManager::STATE_READY,
        registers       => [(0) x 32],
        pc              => 0,
        sp              => 0,
        memory_base     => $opts->{memory_base}  // 0,
        memory_size     => $opts->{memory_size}  // 0,
        parent_pid      => $opts->{parent_pid}   // 0,
        children        => [],
        pending_signals => [],
        signal_handlers => {},
        signal_mask     => {},
        priority        => $opts->{priority} // CodingAdventures::ProcessManager::DEFAULT_PRIORITY,
        cpu_time        => 0,
        exit_code       => 0,
    }, $class;
}

sub set_state {
    my ($self, $state) = @_;
    $self->{state} = $state;
    return $self;
}

sub save_context {
    my ($self, $registers, $pc, $sp) = @_;
    $self->{registers} = $registers if $registers;
    $self->{pc}        = $pc        if defined $pc;
    $self->{sp}        = $sp        if defined $sp;
    return $self;
}

sub add_signal {
    my ($self, $sig) = @_;
    unless (grep { $_ == $sig } @{ $self->{pending_signals} }) {
        push @{ $self->{pending_signals} }, $sig;
    }
    return $self;
}

sub is_masked {
    my ($self, $sig) = @_;
    return $self->{signal_mask}{$sig} ? 1 : 0;
}

sub mask_signal {
    my ($self, $sig) = @_;
    $self->{signal_mask}{$sig} = 1;
    return $self;
}

sub unmask_signal {
    my ($self, $sig) = @_;
    delete $self->{signal_mask}{$sig};
    return $self;
}

sub set_handler {
    my ($self, $sig, $addr) = @_;
    $self->{signal_handlers}{$sig} = $addr;
    return $self;
}

sub tick_cpu {
    my ($self, $delta) = @_;
    $delta //= 1;
    $self->{cpu_time} += $delta;
    return $self;
}

# ============================================================================
# Manager — Process Table and Scheduler
# ============================================================================

package CodingAdventures::ProcessManager::Manager;

sub new {
    my ($class) = @_;
    return bless {
        process_table => {},
        run_queue     => [],
        next_pid      => 1,
        current_pid   => undef,
    }, $class;
}

sub get { $_[0]->{process_table}{$_[1]} }

# Internal: insert PID into run queue sorted by priority
sub _enqueue {
    my ($self, $pid) = @_;
    my $priority = $self->{process_table}{$pid}
        ? $self->{process_table}{$pid}{priority}
        : CodingAdventures::ProcessManager::DEFAULT_PRIORITY;

    # Remove if already present
    my @rq = grep { $_ != $pid } @{ $self->{run_queue} };

    # Insert at correct sorted position (stable: after same-priority entries)
    my $inserted = 0;
    my @result;
    for my $v (@rq) {
        my $vp = $self->{process_table}{$v}
            ? $self->{process_table}{$v}{priority}
            : CodingAdventures::ProcessManager::DEFAULT_PRIORITY;
        if (!$inserted && $priority < $vp) {
            push @result, $pid;
            $inserted = 1;
        }
        push @result, $v;
    }
    push @result, $pid unless $inserted;
    $self->{run_queue} = \@result;
}

sub spawn {
    my ($self, $name, $opts) = @_;
    my $pid = $self->{next_pid}++;
    my $pcb = CodingAdventures::ProcessManager::PCB->new($pid, $name, $opts);
    $self->{process_table}{$pid} = $pcb;
    $self->_enqueue($pid);
    return $pid;
}

sub fork {
    my ($self, $parent_pid) = @_;
    my $parent = $self->{process_table}{$parent_pid}
        or die "fork: no such process $parent_pid";

    my $child_pid = $self->{next_pid}++;

    # Clone parent PCB
    my $child = CodingAdventures::ProcessManager::PCB->new($child_pid, $parent->{name}, {
        memory_base => $parent->{memory_base},
        memory_size => $parent->{memory_size},
        parent_pid  => $parent_pid,
        priority    => $parent->{priority},
    });
    $child->{registers} = [@{ $parent->{registers} }];
    $child->{pc}        = $parent->{pc};
    $child->{sp}        = $parent->{sp};

    # Record child in parent
    push @{ $parent->{children} }, $child_pid;

    $self->{process_table}{$child_pid} = $child;
    $self->_enqueue($child_pid);
    return $child_pid;
}

sub exec {
    my ($self, $pid, $name, $opts) = @_;
    $opts //= {};
    my $pcb = $self->{process_table}{$pid}
        or die "exec: no such process $pid";
    $pcb->{name}       = $name;
    $pcb->{registers}  = [(0) x 32];
    $pcb->{pc}         = $opts->{pc}          // 0;
    $pcb->{sp}         = 0;
    $pcb->{memory_base} = $opts->{memory_base} // $pcb->{memory_base};
    $pcb->{memory_size} = $opts->{memory_size} // $pcb->{memory_size};
    return $self;
}

sub wait_child {
    my ($self, $parent_pid, $child_pid) = @_;
    my $parent = $self->{process_table}{$parent_pid} or return ('no_child', 0);
    my $child  = $self->{process_table}{$child_pid}  or return ('no_child', 0);

    # Verify parentage
    unless (grep { $_ == $child_pid } @{ $parent->{children} }) {
        return ('no_child', 0);
    }

    return ('not_exited', 0)
        if $child->{state} != CodingAdventures::ProcessManager::STATE_ZOMBIE;

    # Reap
    my $exit_code = $child->{exit_code};
    delete $self->{process_table}{$child_pid};
    $parent->{children} = [grep { $_ != $child_pid } @{ $parent->{children} }];
    return ('ok', $exit_code);
}

sub exit_process {
    my ($self, $pid, $exit_code) = @_;
    my $pcb = $self->{process_table}{$pid} or return $self;
    $pcb->{state}     = CodingAdventures::ProcessManager::STATE_ZOMBIE;
    $pcb->{exit_code} = $exit_code // 0;

    # Remove from run queue
    $self->{run_queue} = [grep { $_ != $pid } @{ $self->{run_queue} }];

    # Send SIGCHLD to parent
    if ($pcb->{parent_pid}) {
        my $parent = $self->{process_table}{$pcb->{parent_pid}};
        $parent->add_signal(CodingAdventures::ProcessManager::SIGCHLD) if $parent;
    }

    $self->{current_pid} = undef if defined $self->{current_pid} && $self->{current_pid} == $pid;
    return $self;
}

sub kill {
    my ($self, $target_pid, $sig) = @_;
    my $pcb = $self->{process_table}{$target_pid} or return $self;

    if ($sig == CodingAdventures::ProcessManager::SIGKILL) {
        return $self->exit_process($target_pid, 137);
    }

    if ($sig == CodingAdventures::ProcessManager::SIGSTOP) {
        if ($pcb->{state} == CodingAdventures::ProcessManager::STATE_RUNNING
         || $pcb->{state} == CodingAdventures::ProcessManager::STATE_READY) {
            $pcb->{state} = CodingAdventures::ProcessManager::STATE_BLOCKED;
            $self->{run_queue} = [grep { $_ != $target_pid } @{ $self->{run_queue} }];
        }
        return $self;
    }

    if ($sig == CodingAdventures::ProcessManager::SIGCONT) {
        if ($pcb->{state} == CodingAdventures::ProcessManager::STATE_BLOCKED) {
            $pcb->{state} = CodingAdventures::ProcessManager::STATE_READY;
            $self->_enqueue($target_pid);
        }
        return $self;
    }

    # Queue signal if not masked
    $pcb->add_signal($sig) unless $pcb->is_masked($sig);
    return $self;
}

sub schedule {
    my ($self) = @_;
    return undef unless @{ $self->{run_queue} };

    # Move current running process back to ready
    if (defined $self->{current_pid}) {
        my $cur = $self->{process_table}{$self->{current_pid}};
        if ($cur && $cur->{state} == CodingAdventures::ProcessManager::STATE_RUNNING) {
            $cur->{state} = CodingAdventures::ProcessManager::STATE_READY;
            $self->_enqueue($self->{current_pid});
        }
    }

    my $chosen = shift @{ $self->{run_queue} };
    my $pcb    = $self->{process_table}{$chosen};
    $pcb->{state}     = CodingAdventures::ProcessManager::STATE_RUNNING;
    $self->{current_pid} = $chosen;
    return $chosen;
}

sub block {
    my ($self, $pid) = @_;
    my $pcb = $self->{process_table}{$pid} or return $self;
    $pcb->{state}      = CodingAdventures::ProcessManager::STATE_BLOCKED;
    $self->{run_queue} = [grep { $_ != $pid } @{ $self->{run_queue} }];
    $self->{current_pid} = undef if defined $self->{current_pid} && $self->{current_pid} == $pid;
    return $self;
}

sub unblock {
    my ($self, $pid) = @_;
    my $pcb = $self->{process_table}{$pid} or return $self;
    return $self unless $pcb->{state} == CodingAdventures::ProcessManager::STATE_BLOCKED;
    $pcb->{state} = CodingAdventures::ProcessManager::STATE_READY;
    $self->_enqueue($pid);
    return $self;
}

sub count_in_state {
    my ($self, $state) = @_;
    return scalar grep { $_->{state} == $state } values %{ $self->{process_table} };
}

sub total_processes {
    return scalar keys %{ $_[0]->{process_table} };
}

# ============================================================================
# Top-level package
# ============================================================================

package CodingAdventures::ProcessManager;

=head1 NAME

CodingAdventures::ProcessManager - Process lifecycle management

=head1 SYNOPSIS

  use CodingAdventures::ProcessManager;

  my $mgr  = CodingAdventures::ProcessManager::Manager->new();
  my $init = $mgr->spawn("init");
  my $ls   = $mgr->fork($init);
  $mgr->exec($ls, "ls", { pc => 0x4000 });
  $mgr->exit_process($ls, 0);
  my ($status, $ec) = $mgr->wait_child($init, $ls);

=cut

1;
