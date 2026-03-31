package CodingAdventures::Intel4004Simulator;

# ============================================================================
# Intel 4004 Simulator — the world's first commercial microprocessor
# ============================================================================
#
# The Intel 4004 was released in November 1971, designed by Federico Faggin,
# Ted Hoff, and Stanley Mazor for the Busicom 141-PF calculator. It contained
# just 2,300 transistors and ran at 740 kHz — roughly one million times slower
# than a modern CPU core. Yet it proved that a general-purpose processor could
# be built on a single chip, launching the microprocessor revolution.
#
# ## Architecture
#
#   Data width:      4 bits (values 0-15)
#   Instructions:    8 bits (some are 2 bytes)
#   Registers:       16 x 4-bit (R0-R15), organized as 8 pairs (P0-P7)
#   Accumulator:     4-bit (A) — most arithmetic goes through here
#   Carry flag:      1 bit — set on overflow/borrow
#   Program counter: 12 bits (addresses 4096 bytes of ROM)
#   Stack:           3-level hardware stack (12-bit return addresses)
#   RAM:             4 banks x 4 registers x (16 main + 4 status) nibbles
#   Clock:           740 kHz (original hardware)
#
# ## Register pairs
#
# The 16 registers are organized as 8 pairs for certain instructions:
#   P0: R0 (high nibble), R1 (low nibble)
#   P1: R2 (high nibble), R3 (low nibble)
#   ...
#   P7: R14 (high nibble), R15 (low nibble)
#
# Pair value = (R_high << 4) | R_low  (8-bit combined value)
#
# ## 3-Level Hardware Stack
#
# The 4004 has a 3-deep hardware stack for subroutine calls. It is NOT in
# RAM — it uses dedicated registers inside the chip. Stack wraps silently
# on overflow (4th push overwrites the oldest entry). This is a real hardware
# constraint that severely limits recursion depth.
#
# ## Complete Instruction Set (46 + HLT)
#
#   0x00       NOP          No operation
#   0x01       HLT          Halt (simulator-only)
#   0x1_       JCN c,a *    Conditional jump
#   0x2_ even  FIM Pp,d *   Fetch immediate to pair
#   0x2_ odd   SRC Pp       Send register control
#   0x3_ even  FIN Pp       Fetch indirect from ROM via P0
#   0x3_ odd   JIN Pp       Jump indirect via pair
#   0x4_       JUN a   *    Unconditional jump
#   0x5_       JMS a   *    Jump to subroutine
#   0x6_       INC Rn       Increment register
#   0x7_       ISZ Rn,a *   Increment and skip if zero
#   0x8_       ADD Rn       Add register to accumulator with carry
#   0x9_       SUB Rn       Subtract register with borrow
#   0xA_       LD Rn        Load register into accumulator
#   0xB_       XCH Rn       Exchange accumulator and register
#   0xC_       BBL n        Branch back and load
#   0xD_       LDM n        Load immediate
#   0xE0-0xEF  I/O ops      RAM/ROM read/write
#   0xF0-0xFD  Accum ops    Accumulator manipulation
#
#   (* = 2-byte instruction)

use strict;
use warnings;
our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# Constructor
# ---------------------------------------------------------------------------

sub new {
    my ($class) = @_;
    my $self = bless {}, $class;
    $self->_init_state();
    return $self;
}

sub _init_state {
    my ($self) = @_;

    # 4-bit accumulator (0-15)
    $self->{accumulator} = 0;

    # 16 x 4-bit registers (R0-R15), 0-indexed internally
    $self->{registers} = [(0) x 16];

    # 1-bit carry flag
    $self->{carry} = 0;

    # 12-bit program counter
    $self->{pc} = 0;

    # Halted flag
    $self->{halted} = 0;

    # 3-level hardware stack (12-bit return addresses, 0-indexed slots)
    $self->{hw_stack}      = [0, 0, 0];
    $self->{stack_pointer} = 0;   # points to next-write slot (0-2)

    # RAM: [bank][reg][char] = nibble
    # Organized as 4 banks x 4 registers x 16 main chars
    $self->{ram}        = [];
    $self->{ram_status} = [];
    $self->{ram_output} = [0, 0, 0, 0];
    for my $b (0..3) {
        for my $r (0..3) {
            $self->{ram}[$b][$r]        = [(0) x 16];
            $self->{ram_status}[$b][$r] = [(0) x 4];
        }
    }

    # Current RAM addressing (set by SRC instruction; 0-indexed internally)
    $self->{ram_bank}      = 0;
    $self->{ram_register}  = 0;
    $self->{ram_character} = 0;

    # ROM (4096 bytes, 0-indexed)
    $self->{rom} = [(0) x 4096];

    # ROM I/O port (WRR/RDR)
    $self->{rom_port} = 0;
}

# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

# Load a program (arrayref of bytes or a string) into ROM at address 0.
sub load_program {
    my ($self, $program) = @_;
    $self->{pc} = 0;
    if (ref $program eq 'ARRAY') {
        for my $i (0 .. $#$program) {
            $self->{rom}[$i] = $program->[$i] & 0xFF;
        }
    } else {
        # String of bytes
        my @bytes = unpack('C*', $program);
        for my $i (0 .. $#bytes) {
            $self->{rom}[$i] = $bytes[$i] & 0xFF;
        }
    }
}

# Run a program and return arrayref of trace hashrefs.
# $program: arrayref of bytes
# $max_steps: maximum instructions to execute (default 10000)
sub run {
    my ($self, $program, $max_steps) = @_;
    $max_steps //= 10_000;
    $self->load_program($program);
    my @traces;
    my $steps = 0;
    while (!$self->{halted} && $self->{pc} < 4096 && $steps < $max_steps) {
        push @traces, $self->step();
        $steps++;
    }
    return \@traces;
}

# Execute one instruction. Returns a trace hashref:
#   {address, raw, raw2, mnemonic,
#    accumulator_before, accumulator_after,
#    carry_before, carry_after}
sub step {
    my ($self) = @_;
    die "CPU is halted — cannot step further\n" if $self->{halted};

    my $address = $self->{pc};
    my $raw     = $self->{rom}[$self->{pc}] // 0;
    $self->{pc}++;

    my $raw2;
    if ($self->_is_two_byte($raw)) {
        $raw2 = $self->{rom}[$self->{pc}] // 0;
        $self->{pc}++;
    }

    my $acc_before   = $self->{accumulator};
    my $carry_before = $self->{carry};

    my $mnemonic = $self->_execute($raw, $raw2, $address);

    return {
        address            => $address,
        raw                => $raw,
        raw2               => $raw2,
        mnemonic           => $mnemonic,
        accumulator_before => $acc_before,
        accumulator_after  => $self->{accumulator},
        carry_before       => $carry_before,
        carry_after        => $self->{carry},
    };
}

# Reset CPU to initial state, preserving nothing.
sub reset {
    my ($self) = @_;
    $self->_init_state();
}

# Accessors for testing
sub accumulator  { $_[0]->{accumulator} }
sub carry        { $_[0]->{carry} }
sub pc           { $_[0]->{pc} }
sub halted       { $_[0]->{halted} }

sub get_register {
    my ($self, $reg) = @_;
    return $self->{registers}[$reg] // 0;
}

sub get_ram {
    my ($self, $bank, $reg, $char) = @_;
    return $self->{ram}[$bank][$reg][$char] // 0;
}

sub get_ram_status {
    my ($self, $bank, $reg, $idx) = @_;
    return $self->{ram_status}[$bank][$reg][$idx] // 0;
}

# ---------------------------------------------------------------------------
# Private: 2-byte instruction detection
# ---------------------------------------------------------------------------

# Returns true if this opcode is the first byte of a 2-byte instruction.
#
# 2-byte instructions:
#   0x1_  JCN — conditional jump
#   0x2_ even FIM — fetch immediate
#   0x4_  JUN — unconditional jump (12-bit address)
#   0x5_  JMS — jump to subroutine
#   0x7_  ISZ — increment and skip if zero
sub _is_two_byte {
    my ($self, $raw) = @_;
    my $upper = ($raw >> 4) & 0xF;
    return 1 if $upper == 0x1 || $upper == 0x4 || $upper == 0x5 || $upper == 0x7;
    # FIM: 0x2_ with even low nibble
    return 1 if $upper == 0x2 && ($raw & 0x1) == 0;
    return 0;
}

# ---------------------------------------------------------------------------
# Private: instruction dispatcher
# ---------------------------------------------------------------------------

sub _execute {
    my ($self, $raw, $raw2, $addr) = @_;

    return 'NOP' if $raw == 0x00;
    if ($raw == 0x01) {
        $self->{halted} = 1;
        return 'HLT';
    }

    my $upper = ($raw >> 4) & 0xF;
    my $lower = $raw & 0xF;

    if    ($upper == 0x1) { return $self->_exec_jcn($lower, $raw2, $addr) }
    elsif ($upper == 0x2 && ($raw & 1) == 0) { return $self->_exec_fim($lower >> 1, $raw2) }
    elsif ($upper == 0x2) { return $self->_exec_src($lower >> 1) }
    elsif ($upper == 0x3 && ($raw & 1) == 0) { return $self->_exec_fin($lower >> 1, $addr) }
    elsif ($upper == 0x3) { return $self->_exec_jin($lower >> 1, $addr) }
    elsif ($upper == 0x4) { return $self->_exec_jun($lower, $raw2) }
    elsif ($upper == 0x5) { return $self->_exec_jms($lower, $raw2, $addr) }
    elsif ($upper == 0x6) { return $self->_exec_inc($lower) }
    elsif ($upper == 0x7) { return $self->_exec_isz($lower, $raw2, $addr) }
    elsif ($upper == 0x8) { return $self->_exec_add($lower) }
    elsif ($upper == 0x9) { return $self->_exec_sub($lower) }
    elsif ($upper == 0xA) { return $self->_exec_ld($lower) }
    elsif ($upper == 0xB) { return $self->_exec_xch($lower) }
    elsif ($upper == 0xC) { return $self->_exec_bbl($lower) }
    elsif ($upper == 0xD) { return $self->_exec_ldm($lower) }
    elsif ($upper == 0xE) { return $self->_exec_io($raw) }
    elsif ($upper == 0xF) { return $self->_exec_accum($raw) }
    else { return sprintf "UNKNOWN(0x%02X)", $raw }
}

# ---------------------------------------------------------------------------
# Instruction implementations
# ---------------------------------------------------------------------------

# LDM: Load immediate nibble into accumulator.
# A = N (lower nibble of opcode byte)
sub _exec_ldm {
    my ($self, $n) = @_;
    $self->{accumulator} = $n & 0xF;
    return "LDM $n";
}

# LD: Load register into accumulator.
# A = Rn (non-destructive read)
sub _exec_ld {
    my ($self, $reg) = @_;
    $self->{accumulator} = $self->{registers}[$reg] & 0xF;
    return "LD R$reg";
}

# XCH: Exchange accumulator and register.
# tmp = A; A = Rn; Rn = tmp
sub _exec_xch {
    my ($self, $reg) = @_;
    my $old_a = $self->{accumulator};
    $self->{accumulator}    = $self->{registers}[$reg] & 0xF;
    $self->{registers}[$reg] = $old_a & 0xF;
    return "XCH R$reg";
}

# INC: Increment register (modulo 16, no carry effect).
# Rn = (Rn + 1) & 0xF
sub _exec_inc {
    my ($self, $reg) = @_;
    $self->{registers}[$reg] = ($self->{registers}[$reg] + 1) & 0xF;
    return "INC R$reg";
}

# ADD: Add register to accumulator with carry.
# A = A + Rn + carry_in. Sets carry if result > 15.
# The carry participates — this enables multi-digit BCD arithmetic.
sub _exec_add {
    my ($self, $reg) = @_;
    my $carry_in = $self->{carry} ? 1 : 0;
    my $result   = $self->{accumulator} + $self->{registers}[$reg] + $carry_in;
    $self->{accumulator} = $result & 0xF;
    $self->{carry}       = $result > 0xF ? 1 : 0;
    return "ADD R$reg";
}

# SUB: Subtract register from accumulator (complement-add method).
# A = A + (~Rn) + CY   where CY (carry) = 1 means no borrow (MCS-4 carry semantics).
sub _exec_sub {
    my ($self, $reg) = @_;
    my $reg_val    = $self->{registers}[$reg];
    my $complement = (~$reg_val) & 0xF;
    my $borrow_in  = $self->{carry} ? 1 : 0;
    my $result     = $self->{accumulator} + $complement + $borrow_in;
    $self->{accumulator} = $result & 0xF;
    $self->{carry}       = $result > 0xF ? 1 : 0;
    return "SUB R$reg";
}

# JUN: Unconditional jump to 12-bit address.
# Target = (lower_nibble << 8) | second_byte
sub _exec_jun {
    my ($self, $lower, $raw2) = @_;
    my $target = ($lower << 8) | $raw2;
    $self->{pc} = $target;
    return sprintf "JUN 0x%03X", $target;
}

# JCN: Conditional jump (page-relative).
# Condition nibble bits:
#   bit 3 (0x8): invert the test result
#   bit 2 (0x4): test accumulator == 0
#   bit 1 (0x2): test carry == 1
#   bit 0 (0x1): test input pin (always 0 in simulator)
# Multiple test bits are OR'd. Target is same-page (upper 4 bits of PC).
sub _exec_jcn {
    my ($self, $cond, $raw2, $addr) = @_;
    my $test_zero  = ($cond & 0x4) && $self->{accumulator} == 0;
    my $test_carry = ($cond & 0x2) && $self->{carry};
    my $test_pin   = 0;   # input pin always 0 in simulator
    my $result     = $test_zero || $test_carry || $test_pin;
    $result = !$result if $cond & 0x8;

    # Target is page-relative: upper 4 bits from (addr+2), lower 8 from raw2
    my $page   = ($addr + 2) & 0xF00;
    my $target = $page | $raw2;

    $self->{pc} = $target if $result;
    return sprintf "JCN %d,0x%02X", $cond, $raw2;
}

# JMS: Jump to subroutine (push return address, jump to 12-bit address).
# Push (addr+2) onto 3-level stack, then set PC.
sub _exec_jms {
    my ($self, $lower, $raw2, $addr) = @_;
    my $target      = ($lower << 8) | $raw2;
    my $return_addr = $addr + 2;
    $self->_stack_push($return_addr);
    $self->{pc} = $target;
    return sprintf "JMS 0x%03X", $target;
}

# BBL: Branch back and load.
# Pop return address from stack. Set A = immediate nibble (only if n != 0).
# BBL 0 is used to return while preserving the accumulator's current value.
sub _exec_bbl {
    my ($self, $n) = @_;
    my $return_addr  = $self->_stack_pop();
    $self->{accumulator} = $n & 0xF if $n != 0;
    $self->{pc}          = $return_addr;
    return "BBL $n";
}

# ISZ: Increment register and skip next instruction if zero.
# Rn = (Rn + 1) & 0xF.  If Rn != 0: jump to page-relative target.
# Used as a loop counter: the register counts up from some starting value;
# when it wraps to 0 the jump is skipped (loop exit).
sub _exec_isz {
    my ($self, $reg, $raw2, $addr) = @_;
    my $val = ($self->{registers}[$reg] + 1) & 0xF;
    $self->{registers}[$reg] = $val;

    if ($val != 0) {
        my $page   = ($addr + 2) & 0xF00;
        my $target = $page | $raw2;
        $self->{pc} = $target;
    }
    return sprintf "ISZ R%d,0x%02X", $reg, $raw2;
}

# FIM: Fetch immediate byte into register pair.
# R_high = (data >> 4) & 0xF,  R_low = data & 0xF
sub _exec_fim {
    my ($self, $pair, $data) = @_;
    my $high_reg = $pair * 2;
    my $low_reg  = $high_reg + 1;
    $self->{registers}[$high_reg] = ($data >> 4) & 0xF;
    $self->{registers}[$low_reg]  = $data & 0xF;
    return sprintf "FIM P%d,0x%02X", $pair, $data;
}

# SRC: Send register pair as address for RAM/ROM operations.
# High nibble of pair selects RAM register (mod 4, 0-indexed).
# Low nibble selects character within that register (0-indexed).
sub _exec_src {
    my ($self, $pair) = @_;
    my $pair_val = $self->_read_pair($pair);
    $self->{ram_register}  = (($pair_val >> 4) & 0xF) % 4;
    $self->{ram_character} = $pair_val & 0xF;
    return "SRC P$pair";
}

# FIN: Fetch indirect from ROM via pair P0.
# ROM address = (current_page) | value_of_P0
# Loads the byte at that ROM address into register pair Pp.
sub _exec_fin {
    my ($self, $pair, $addr) = @_;
    my $p0_val   = $self->_read_pair(0);
    my $page     = $addr & 0xF00;
    my $rom_addr = $page | $p0_val;
    my $rom_byte = $self->{rom}[$rom_addr] // 0;
    $self->_write_pair($pair, $rom_byte);
    return "FIN P$pair";
}

# JIN: Jump indirect via register pair (page-relative).
# PC = (current_page) | (pair_high << 4) | pair_low
sub _exec_jin {
    my ($self, $pair, $addr) = @_;
    my $pair_val = $self->_read_pair($pair);
    my $page     = $addr & 0xF00;
    $self->{pc}  = $page | $pair_val;
    return "JIN P$pair";
}

# ---------------------------------------------------------------------------
# I/O instructions (0xE0-0xEF)
# ---------------------------------------------------------------------------

sub _exec_io {
    my ($self, $raw) = @_;

    if ($raw == 0xE0) {
        # WRM: Write accumulator to RAM main character
        my ($b, $r, $c) = @{$self}{qw(ram_bank ram_register ram_character)};
        $self->{ram}[$b][$r][$c] = $self->{accumulator} & 0xF;
        return "WRM";

    } elsif ($raw == 0xE1) {
        # WMP: Write accumulator to RAM output port
        $self->{ram_output}[$self->{ram_bank}] = $self->{accumulator} & 0xF;
        return "WMP";

    } elsif ($raw == 0xE2) {
        # WRR: Write accumulator to ROM I/O port
        $self->{rom_port} = $self->{accumulator} & 0xF;
        return "WRR";

    } elsif ($raw == 0xE3) {
        # WPM: Write program RAM (not simulated)
        return "WPM";

    } elsif ($raw >= 0xE4 && $raw <= 0xE7) {
        # WR0-WR3: Write accumulator to RAM status character 0-3
        my $idx = $raw - 0xE4;
        my ($b, $r) = @{$self}{qw(ram_bank ram_register)};
        $self->{ram_status}[$b][$r][$idx] = $self->{accumulator} & 0xF;
        return "WR" . ($raw - 0xE4);

    } elsif ($raw == 0xE8) {
        # SBM: Subtract RAM main character from accumulator (complement-add)
        my ($b, $r, $c) = @{$self}{qw(ram_bank ram_register ram_character)};
        my $ram_val    = $self->{ram}[$b][$r][$c];
        my $complement = (~$ram_val) & 0xF;
        my $borrow_in  = $self->{carry} ? 1 : 0;
        my $result     = $self->{accumulator} + $complement + $borrow_in;
        $self->{accumulator} = $result & 0xF;
        $self->{carry}       = $result > 0xF ? 1 : 0;
        return "SBM";

    } elsif ($raw == 0xE9) {
        # RDM: Read RAM main character into accumulator
        my ($b, $r, $c) = @{$self}{qw(ram_bank ram_register ram_character)};
        $self->{accumulator} = $self->{ram}[$b][$r][$c] & 0xF;
        return "RDM";

    } elsif ($raw == 0xEA) {
        # RDR: Read ROM I/O port into accumulator
        $self->{accumulator} = $self->{rom_port} & 0xF;
        return "RDR";

    } elsif ($raw == 0xEB) {
        # ADM: Add RAM main character to accumulator with carry
        my ($b, $r, $c) = @{$self}{qw(ram_bank ram_register ram_character)};
        my $ram_val  = $self->{ram}[$b][$r][$c];
        my $carry_in = $self->{carry} ? 1 : 0;
        my $result   = $self->{accumulator} + $ram_val + $carry_in;
        $self->{accumulator} = $result & 0xF;
        $self->{carry}       = $result > 0xF ? 1 : 0;
        return "ADM";

    } elsif ($raw >= 0xEC && $raw <= 0xEF) {
        # RD0-RD3: Read RAM status character 0-3 into accumulator
        my $idx = $raw - 0xEC;
        my ($b, $r) = @{$self}{qw(ram_bank ram_register)};
        $self->{accumulator} = $self->{ram_status}[$b][$r][$idx] & 0xF;
        return "RD" . ($raw - 0xEC);

    } else {
        return sprintf "UNKNOWN(0x%02X)", $raw;
    }
}

# ---------------------------------------------------------------------------
# Accumulator instructions (0xF0-0xFD)
# ---------------------------------------------------------------------------

sub _exec_accum {
    my ($self, $raw) = @_;

    if ($raw == 0xF0) {
        # CLB: Clear both accumulator and carry
        $self->{accumulator} = 0;
        $self->{carry}       = 0;
        return "CLB";

    } elsif ($raw == 0xF1) {
        # CLC: Clear carry flag
        $self->{carry} = 0;
        return "CLC";

    } elsif ($raw == 0xF2) {
        # IAC: Increment accumulator
        my $result = $self->{accumulator} + 1;
        $self->{accumulator} = $result & 0xF;
        $self->{carry}       = $result > 0xF ? 1 : 0;
        return "IAC";

    } elsif ($raw == 0xF3) {
        # CMC: Complement (invert) carry flag
        $self->{carry} = $self->{carry} ? 0 : 1;
        return "CMC";

    } elsif ($raw == 0xF4) {
        # CMA: Complement (bitwise NOT) accumulator in 4 bits
        $self->{accumulator} = (~$self->{accumulator}) & 0xF;
        return "CMA";

    } elsif ($raw == 0xF5) {
        # RAL: Rotate accumulator left through carry.
        # [carry | A3 A2 A1 A0] -> [A3 | A2 A1 A0 old_carry]
        my $old_carry = $self->{carry} ? 1 : 0;
        my $new_carry = ($self->{accumulator} & 0x8) ? 1 : 0;
        $self->{accumulator} = (($self->{accumulator} << 1) | $old_carry) & 0xF;
        $self->{carry}       = $new_carry;
        return "RAL";

    } elsif ($raw == 0xF6) {
        # RAR: Rotate accumulator right through carry.
        # [carry | A3 A2 A1 A0] -> [A0 | old_carry A3 A2 A1]
        my $old_carry = $self->{carry} ? 1 : 0;
        my $new_carry = ($self->{accumulator} & 0x1) ? 1 : 0;
        $self->{accumulator} = (($self->{accumulator} >> 1) | ($old_carry << 3)) & 0xF;
        $self->{carry}       = $new_carry;
        return "RAR";

    } elsif ($raw == 0xF7) {
        # TCC: Transfer carry to accumulator, clear carry
        $self->{accumulator} = $self->{carry} ? 1 : 0;
        $self->{carry}       = 0;
        return "TCC";

    } elsif ($raw == 0xF8) {
        # DAC: Decrement accumulator.
        # carry = 1 if no borrow (A > 0), 0 if borrow (A was 0 and wrapped).
        my $new_carry = $self->{accumulator} > 0 ? 1 : 0;
        $self->{accumulator} = ($self->{accumulator} - 1) & 0xF;
        $self->{carry}       = $new_carry;
        return "DAC";

    } elsif ($raw == 0xF9) {
        # TCS: Transfer carry subtract.
        # A = 10 if carry set, 9 if carry clear. Carry is always cleared.
        # Used in BCD subtraction to provide the complement correction factor.
        $self->{accumulator} = $self->{carry} ? 10 : 9;
        $self->{carry}       = 0;
        return "TCS";

    } elsif ($raw == 0xFA) {
        # STC: Set carry flag
        $self->{carry} = 1;
        return "STC";

    } elsif ($raw == 0xFB) {
        # DAA: Decimal adjust accumulator (BCD correction after addition).
        # If A > 9 or carry is set, add 6 to A.
        # If that addition overflows past 0xF, set carry.
        if ($self->{accumulator} > 9 || $self->{carry}) {
            my $result  = $self->{accumulator} + 6;
            my $new_carry = ($result > 0xF) ? 1 : $self->{carry};
            $self->{accumulator} = $result & 0xF;
            $self->{carry}       = $new_carry;
        }
        return "DAA";

    } elsif ($raw == 0xFC) {
        # KBP: Keyboard process — converts 1-hot encoding to binary.
        # 0->0, 1->1, 2->2, 4->3, 8->4, anything else->15 (error indicator).
        # This was used by the 4004 to read a decimal keypad where only one
        # key could be pressed at a time (one-hot encoding).
        my %kbp = (0=>0, 1=>1, 2=>2, 4=>3, 8=>4);
        my $acc = $self->{accumulator};
        $self->{accumulator} = exists $kbp{$acc} ? $kbp{$acc} : 15;
        return "KBP";

    } elsif ($raw == 0xFD) {
        # DCL: Designate command line — select RAM bank based on A bits 0-2.
        # A values 0-3 select banks 0-3 (0-indexed in our implementation).
        my $bank_bits = $self->{accumulator} & 0x7;
        $bank_bits = $bank_bits & 0x3 if $bank_bits > 3;
        $self->{ram_bank} = $bank_bits;
        return "DCL";

    } else {
        return sprintf "UNKNOWN(0x%02X)", $raw;
    }
}

# ---------------------------------------------------------------------------
# Private helpers: register pairs
# ---------------------------------------------------------------------------

# Read a register pair value (8-bit: high_nibble<<4 | low_nibble).
# pair: 0-7
sub _read_pair {
    my ($self, $pair) = @_;
    my $high_reg = $pair * 2;
    my $low_reg  = $high_reg + 1;
    my $high = $self->{registers}[$high_reg] // 0;
    my $low  = $self->{registers}[$low_reg]  // 0;
    return ($high << 4) | $low;
}

# Write an 8-bit value into a register pair.
sub _write_pair {
    my ($self, $pair, $value) = @_;
    my $high_reg = $pair * 2;
    my $low_reg  = $high_reg + 1;
    $self->{registers}[$high_reg] = ($value >> 4) & 0xF;
    $self->{registers}[$low_reg]  = $value & 0xF;
}

# ---------------------------------------------------------------------------
# Private helpers: 3-level hardware stack
# ---------------------------------------------------------------------------

# Push an address onto the hardware stack (wraps on overflow).
sub _stack_push {
    my ($self, $addr) = @_;
    $self->{hw_stack}[$self->{stack_pointer}] = $addr & 0xFFF;
    $self->{stack_pointer} = ($self->{stack_pointer} + 1) % 3;
}

# Pop an address from the hardware stack.
sub _stack_pop {
    my ($self) = @_;
    $self->{stack_pointer} = ($self->{stack_pointer} + 2) % 3;
    return $self->{hw_stack}[$self->{stack_pointer}] // 0;
}

1;
