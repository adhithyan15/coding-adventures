package CodingAdventures::Intel8008GateLevel::Registers;

# ============================================================================
# Registers.pm — 7×8-bit Register File (Sequential Logic)
# ============================================================================
#
# The Intel 8008 has 7 working registers (A, B, C, D, E, H, L) plus a
# pseudo-register M (indirect memory). Each register is 8 bits wide.
#
# In the gate-level model, each register is implemented as 8 D flip-flops
# (one per bit) via the Register() function from CodingAdventures::LogicGates.
# Each D flip-flop is itself built from two SR latches (a master-slave pair).
#
# ## Flip-Flop Write Protocol
#
# The Register() function models a master-slave D flip-flop:
#   clock=0: data is loaded into the master latch (not yet visible at output)
#   clock=1: master latches to slave (data appears at the output)
# A complete write requires two phases: clock=0 then clock=1.
#
# ## Register File Layout
#
# Indices match the 8008's 3-bit register encoding:
#   0=B, 1=C, 2=D, 3=E, 4=H, 5=L, 6=(unused), 7=A
#
# Register 6 (M) is not a physical register — it maps to memory at [H:L].
# Attempting to read/write register 6 via this module raises an error.
#
# ## Flip-Flop Count
#
#   7 data registers × 8 bits = 56 D flip-flops
#   Each D flip-flop = 2 SR latches = 4 NOR gates
#   Total: 56 × 4 = 224 NOR gates for the register file
#
# ## HL Address Computation
#
# The 8008 memory address is formed from H (bits 13–8) and L (bits 7–0):
#   address = (H & 0x3F) << 8 | L
# The AND with 0x3F masks H to 6 bits — only the low 6 bits of H are
# significant for addressing. This is implemented as 6 AND gates (masking
# the top 2 bits of H to 0) followed by a bit-shift (wire routing).

use strict;
use warnings;

use CodingAdventures::LogicGates qw(AND Register new_flip_flop_state);
use CodingAdventures::Intel8008GateLevel::Bits qw(int_to_bits bits_to_int);

use Exporter 'import';
our @EXPORT_OK = qw(new_register_file read_reg write_reg reg_a reg_h reg_l hl_address);

# Register indices (matching 3-bit hardware encoding)
use constant {
    REG_B => 0, REG_C => 1, REG_D => 2, REG_E => 3,
    REG_H => 4, REG_L => 5, REG_M => 6, REG_A => 7,
};

# Create a new zeroed register file state.
# Returns an arrayref of 8 flip-flop state arrays (one per register index).
# Index 6 (M) is a placeholder (empty array) since M is not a physical register.
sub new_register_file {
    my @file;
    for my $i (0..7) {
        if ($i == REG_M) {
            $file[$i] = [];  # M is not a physical register
        } else {
            # 8-bit register: 8 D flip-flop state hashrefs
            $file[$i] = [map { new_flip_flop_state() } 1..8];
        }
    }
    return \@file;
}

# Read an 8-bit integer from register $reg_idx (0-7).
# Raises an error if reg_idx == 6 (M is not a physical register).
sub read_reg {
    my ($file, $reg_idx) = @_;
    die "Registers: cannot read M (index 6) — resolve to memory address first\n"
        if $reg_idx == REG_M;
    # Use clock=0 to read the current slave output without latching new data.
    my $zero_bits = int_to_bits(0, 8);
    my ($output, undef) = Register($zero_bits, 0, $file->[$reg_idx]);
    return bits_to_int($output);
}

# Write an 8-bit integer to register $reg_idx (0-7).
# Performs the two-phase flip-flop write (clock=0 then clock=1).
# Raises an error if reg_idx == 6 (M).
# Returns the updated file (caller should replace $file->[$reg_idx]).
sub write_reg {
    my ($file, $reg_idx, $value) = @_;
    die "Registers: cannot write M (index 6) — write to memory address instead\n"
        if $reg_idx == REG_M;
    $value &= 0xFF;
    my $bits = int_to_bits($value, 8);
    # Phase 1: load into master latch (clock=0)
    my (undef, $state1) = Register($bits, 0, $file->[$reg_idx]);
    # Phase 2: latch to slave (clock=1, data now visible at output)
    my (undef, $new_state) = Register($bits, 1, $state1);
    $file->[$reg_idx] = $new_state;
    return $file;
}

# Convenience: read the accumulator (A = index 7)
sub reg_a {
    my ($file) = @_;
    return read_reg($file, REG_A);
}

# Convenience: read H
sub reg_h {
    my ($file) = @_;
    return read_reg($file, REG_H);
}

# Convenience: read L
sub reg_l {
    my ($file) = @_;
    return read_reg($file, REG_L);
}

# Compute the 14-bit memory address from H and L.
#
# Hardware implementation:
#   1. Read H (8 bits) from the H register flip-flops
#   2. Mask H to 6 bits: h6 = H & 0x3F (6 AND gates masking bits 7-6 to 0)
#   3. Shift left by 8: address[13:8] = h6[5:0]  (wire routing, no gates)
#   4. Read L (8 bits) from the L register flip-flops
#   5. address[7:0] = L (wire routing)
#   6. Combine: address = h6 << 8 | L
#
# @return  14-bit integer (0–16383)
sub hl_address {
    my ($file) = @_;
    my $h = read_reg($file, REG_H);
    my $l = read_reg($file, REG_L);

    # Mask H to 6 bits using AND gates (hardware: 2 NOT gates to force bits 7-6 to 0)
    # We model this as: h_masked = (H & 0x3F) using 6 AND gates in hardware,
    # but since the top 2 bits are simply forced to 0, it's equivalent to masking.
    my $h_masked = $h & 0x3F;  # AND with 0x3F

    return ($h_masked << 8) | $l;
}

1;
