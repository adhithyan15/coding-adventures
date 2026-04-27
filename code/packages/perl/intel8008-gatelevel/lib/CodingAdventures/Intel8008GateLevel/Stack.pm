package CodingAdventures::Intel8008GateLevel::Stack;

# ============================================================================
# Stack.pm — 8-Level Push-Down Stack (14-bit entries)
# ============================================================================
#
# The Intel 8008's hardware stack is 8 × 14-bit registers arranged as a
# circular push-down stack. Entry 0 always holds the current program counter.
# This is fundamentally different from a software stack — there is no "stack
# pointer" visible to the programmer. The entire stack shifts on every call
# and return.
#
# ## Physical Implementation
#
# 8 registers × 14 bits = 112 D flip-flops.
# Each flip-flop = 2 SR latches = 4 NOR gates.
# Total: 112 × 4 = 448 NOR gates for the stack.
# (Compare: 4004's 3-level stack = 3 × 12 = 36 D flip-flops = 144 NOR gates)
#
# ## Push (CALL/RST): rotate down and load target
#
#   Before CALL target_addr, with PC = current_pc:
#     stack = [current_pc, ret1, ret2, ..., ret7]
#
#   After CALL target_addr:
#     stack = [target_addr, current_pc, ret1, ..., ret6]  ← ret7 is discarded
#
# ## Pop (RETURN): rotate up
#
#   After RETURN:
#     stack = [current_pc, ret2, ..., ret7, 0]  ← entry 0 gets old entry 1
#
# ## Note on stack_depth
#
# Since entry 0 is always the PC, "depth" means how many nested calls are
# active. Depth 0 means no calls have been made (only PC in the stack).
# Maximum useful depth: 7 (after 7 nested CAL instructions).
# Depth 8 would silently overwrite the oldest saved return address.

use strict;
use warnings;

use Exporter 'import';
our @EXPORT_OK = qw(new_stack push_stack pop_stack stack_pc set_pc);

# new_stack — create a zeroed 8-level stack.
# Returns an arrayref of 8 integer entries (all 0), plus a depth counter.
sub new_stack {
    return {
        entries => [(0) x 8],
        depth   => 0,
    };
}

# stack_pc — read the current program counter (always entry 0).
sub stack_pc {
    my ($stack) = @_;
    return $stack->{entries}[0] & 0x3FFF;
}

# set_pc — write a new value to the PC (entry 0).
sub set_pc {
    my ($stack, $pc) = @_;
    $stack->{entries}[0] = $pc & 0x3FFF;
}

# push_stack — save current PC and jump to target.
# Rotates all entries down by one position; entry 7 is lost.
# Loads $target into entry 0 (new PC).
sub push_stack {
    my ($stack, $target) = @_;

    # Rotate down: entry 7 ← entry 6 ← ... ← entry 1 ← entry 0
    # In hardware: 7 × 14 = 98 AND gates to route each bit to the next register.
    for my $i (reverse 1..7) {
        $stack->{entries}[$i] = $stack->{entries}[$i-1];
    }
    # Load target into entry 0 (this becomes the new PC)
    $stack->{entries}[0] = $target & 0x3FFF;

    # Track depth (capped at 7 — deeper nesting silently wraps)
    $stack->{depth}++ if $stack->{depth} < 7;
}

# pop_stack — restore saved return address.
# Rotates all entries up by one position; entry 0 is discarded.
# Entry 1 becomes entry 0 (the restored PC).
sub pop_stack {
    my ($stack) = @_;

    # Rotate up: entry 0 ← entry 1 ← ... ← entry 6 ← entry 7
    for my $i (0..6) {
        $stack->{entries}[$i] = $stack->{entries}[$i+1];
    }
    $stack->{entries}[7] = 0;  # Clear vacated deepest slot

    $stack->{depth}-- if $stack->{depth} > 0;
}

1;
