package CodingAdventures::InterruptHandler;

# ============================================================================
# CodingAdventures::InterruptHandler — Hardware Interrupt Controller
# ============================================================================
#
# Without interrupts, a CPU can only execute instructions sequentially.
# It cannot respond to a timer tick, a keystroke, or a disk completion.
# Interrupts transform a calculator into a computer.
#
# ## Analogy
#
# Imagine cooking while waiting for a phone call.  You are focused on your
# recipe (the main program).  When the phone rings (interrupt), you:
#   1. Put down your spoon and note what step you were on (save context).
#   2. Answer the phone and handle the call (Interrupt Service Routine).
#   3. Hang up and return to the exact step you were on (restore context).
#
# ## Three Types of Interrupts
#
#   Type       Trigger                Examples
#   ─────────────────────────────────────────────────────────────
#   Hardware   External device        Timer tick, keyboard, disk I/O
#   Software   Trap instruction       System call, debug breakpoint
#   Exception  CPU detects error      Divide-by-zero, page fault
#
# ## Module Structure
#
#   InterruptHandler
#   ├── IDT              — maps interrupt numbers (0..255) to ISR addresses
#   ├── ISRRegistry      — maps interrupt numbers to Perl callback subs
#   ├── Controller       — pending queue, mask register, enable/disable
#   └── Frame            — saved CPU context at time of interrupt
#
# ## Usage
#
#   use CodingAdventures::InterruptHandler;
#
#   my $ctrl = CodingAdventures::InterruptHandler::Controller->new();
#   $ctrl->register(32, sub {
#       my ($frame, $kernel) = @_;
#       $kernel->{ticks}++;
#       return $kernel;
#   });
#   $ctrl->raise(32);
#   my $frame  = CodingAdventures::InterruptHandler::Frame->new(0x1000, {}, 0, 32);
#   my $kernel = { ticks => 0 };
#   $ctrl->dispatch($frame, $kernel);  # $kernel->{ticks} == 1
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# IDT — Interrupt Descriptor Table
# ============================================================================
#
# The IDT maps interrupt numbers (0..255) to ISR entry descriptors.
# Each entry has three fields:
#   isr_address     — address of the handler in memory (simulation only)
#   present         — is this entry active?
#   privilege_level — 0 = kernel, 3 = user
#
# Interrupt numbers:
#   0..31   — CPU exceptions (divide-by-zero, page fault, ...)
#   32..47  — Hardware IRQs (timer=32, keyboard=33, disk=34, ...)
#   48..255 — Software interrupts (system calls, user-defined)

package CodingAdventures::InterruptHandler::IDT;

sub new {
    my ($class) = @_;
    return bless { entries => {} }, $class;
}

sub set_entry {
    my ($self, $number, $entry) = @_;
    die "IDT entry number must be 0..255" if $number < 0 || $number > 255;
    $self->{entries}{$number} = {
        isr_address     => $entry->{isr_address}     // 0,
        present         => $entry->{present}         // 1,
        privilege_level => $entry->{privilege_level} // 0,
    };
    return $self;
}

sub get_entry {
    my ($self, $number) = @_;
    die "IDT entry number must be 0..255" if $number < 0 || $number > 255;
    return $self->{entries}{$number}
        // { isr_address => 0, present => 0, privilege_level => 0 };
}

# ============================================================================
# ISRRegistry — Interrupt Service Routine Registry
# ============================================================================
#
# Maps interrupt numbers to Perl subroutines (callbacks).
# When an interrupt fires: $handler->($frame, $kernel) → $new_kernel

package CodingAdventures::InterruptHandler::ISRRegistry;

sub new {
    my ($class) = @_;
    return bless { handlers => {} }, $class;
}

sub register {
    my ($self, $number, $handler) = @_;
    $self->{handlers}{$number} = $handler;
    return $self;
}

sub dispatch {
    my ($self, $number, $frame, $kernel) = @_;
    my $handler = $self->{handlers}{$number}
        or die "no ISR handler registered for interrupt $number";
    return $handler->($frame, $kernel);
}

sub has_handler {
    my ($self, $number) = @_;
    return exists $self->{handlers}{$number};
}

# ============================================================================
# Frame — Interrupt Frame (Saved CPU Context)
# ============================================================================
#
# When an interrupt fires, the CPU saves its register state here so it can
# resume after the ISR returns.
#
#   pc         — Program counter at the time of interrupt
#   registers  — Hashref of register name → value
#   mstatus    — Machine status register (RISC-V) / EFLAGS (x86)
#   mcause     — Machine cause register — which interrupt fired

package CodingAdventures::InterruptHandler::Frame;

sub new {
    my ($class, $pc, $registers, $mstatus, $mcause) = @_;
    return bless {
        pc        => $pc        // 0,
        registers => $registers // {},
        mstatus   => $mstatus   // 0,
        mcause    => $mcause    // 0,
    }, $class;
}

sub save_context {
    my ($class, $registers, $pc, $mstatus, $mcause) = @_;
    return $class->new($pc, $registers, $mstatus, $mcause);
}

sub restore_context {
    my ($self) = @_;
    return ($self->{registers}, $self->{pc}, $self->{mstatus});
}

# ============================================================================
# Controller — Interrupt Controller
# ============================================================================
#
# The controller (PIC/APIC) sits between hardware and the CPU.  It:
#   1. Receives interrupt signals and queues them (sorted by number).
#   2. Masks individual IRQs via a 32-bit mask_register bitmask.
#   3. Respects a global enabled flag (cli/sti in x86).
#   4. Dispatches interrupts in priority order (lowest number first).
#
# ## Mask Register
#
# A 32-bit integer where bit N = 1 means interrupt N is suppressed.
#   mask = 1       ← interrupt 0 masked
#   mask = 3       ← interrupts 0 and 1 masked

package CodingAdventures::InterruptHandler::Controller;

sub new {
    my ($class) = @_;
    return bless {
        idt           => CodingAdventures::InterruptHandler::IDT->new(),
        registry      => CodingAdventures::InterruptHandler::ISRRegistry->new(),
        pending       => [],        # sorted list of pending IRQ numbers
        mask_register => 0,         # 32-bit bitmask of masked lines
        enabled       => 1,         # global interrupt enable
    }, $class;
}

sub register {
    my ($self, $number, $handler) = @_;
    $self->{registry}->register($number, $handler);
    return $self;
}

sub raise {
    my ($self, $number) = @_;
    # Idempotent: don't add if already pending
    unless (grep { $_ == $number } @{ $self->{pending} }) {
        push @{ $self->{pending} }, $number;
        @{ $self->{pending} } = sort { $a <=> $b } @{ $self->{pending} };
    }
    return $self;
}

sub has_pending {
    my ($self) = @_;
    return 0 unless $self->{enabled};
    for my $irq (@{ $self->{pending} }) {
        return 1 unless $self->is_masked($irq);
    }
    return 0;
}

sub next_pending {
    my ($self) = @_;
    return -1 unless $self->{enabled};
    for my $irq (@{ $self->{pending} }) {
        return $irq unless $self->is_masked($irq);
    }
    return -1;
}

sub acknowledge {
    my ($self, $number) = @_;
    @{ $self->{pending} } = grep { $_ != $number } @{ $self->{pending} };
    return $self;
}

sub set_mask {
    my ($self, $number, $masked) = @_;
    return $self if $number < 0 || $number > 31;
    if ($masked) {
        $self->{mask_register} |= (1 << $number);
    } else {
        $self->{mask_register} &= ~(1 << $number);
    }
    return $self;
}

sub is_masked {
    my ($self, $number) = @_;
    return 0 if $number < 0 || $number > 31;
    return ($self->{mask_register} & (1 << $number)) ? 1 : 0;
}

sub enable {
    my ($self) = @_;
    $self->{enabled} = 1;
    return $self;
}

sub disable {
    my ($self) = @_;
    $self->{enabled} = 0;
    return $self;
}

sub pending_count {
    my ($self) = @_;
    return scalar @{ $self->{pending} };
}

sub clear_all {
    my ($self) = @_;
    $self->{pending} = [];
    return $self;
}

sub dispatch {
    my ($self, $frame, $kernel) = @_;
    my $irq = $self->next_pending();
    return $kernel if $irq == -1;
    $self->acknowledge($irq);
    return $self->{registry}->dispatch($irq, $frame, $kernel);
}

# ============================================================================
# Top-level package (re-export convenience)
# ============================================================================

package CodingAdventures::InterruptHandler;

=head1 NAME

CodingAdventures::InterruptHandler - Hardware interrupt controller and handler

=head1 SYNOPSIS

  use CodingAdventures::InterruptHandler;

  my $ctrl = CodingAdventures::InterruptHandler::Controller->new();
  $ctrl->register(32, sub { my ($frame, $k) = @_; $k->{ticks}++; $k });
  $ctrl->raise(32);
  my $frame = CodingAdventures::InterruptHandler::Frame->new(0x1000, {}, 0, 32);
  my $kernel = { ticks => 0 };
  $ctrl->dispatch($frame, $kernel);

=head1 DESCRIPTION

Full interrupt lifecycle: IDT, ISR registry, interrupt controller, and
saved-CPU-context frames.

=cut

1;
