# interrupt-handler (Perl)

Hardware interrupt controller and handler for the coding-adventures simulated computer.

## What It Does

- `IDT` — Interrupt Descriptor Table mapping 0..255 to ISR entries
- `ISRRegistry` — Perl sub registry for interrupt handlers
- `Controller` — pending queue, mask register, global enable/disable, priority dispatch
- `Frame` — saved CPU context (PC, registers, mstatus, mcause)

## Usage

```perl
use CodingAdventures::InterruptHandler;

my $ctrl = CodingAdventures::InterruptHandler::Controller->new();
$ctrl->register(32, sub {
    my ($frame, $kernel) = @_;
    $kernel->{ticks}++;
    return $kernel;
});
$ctrl->raise(32);
my $frame  = CodingAdventures::InterruptHandler::Frame->new(0x1000, {}, 0, 32);
my $kernel = { ticks => 0 };
$ctrl->dispatch($frame, $kernel);
# $kernel->{ticks} == 1
```
