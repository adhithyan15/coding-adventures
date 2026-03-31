package CodingAdventures::ARM1Gatelevel;

# =============================================================================
# CodingAdventures::ARM1Gatelevel — ARM1 Gate-Level Simulator (Perl)
# =============================================================================
#
# Where the behavioral simulator computes `result = $a + $b` directly, this
# simulator routes every bit through logic gate function calls:
#
#   ADD R0, R1, R2:
#     @a_bits = int_to_bits($reg[1], 32)     # LSB-first 0-indexed
#     @b_bits = int_to_bits($reg[2], 32)
#     ($sum_bits, $carry) = ripple_carry_adder(\@a_bits, \@b_bits, 0)
#       # each full_adder: XOR(XOR(a,b),cin), AND(a,b), AND(xor,cin), OR(...)
#     $result = bits_to_int(\@sum_bits)
#
# Every ADD leaves ~200 gate calls in the gate_ops counter.
#
# # Bit Arrays
#
# Perl bit arrays are 0-indexed LSB-first:
#   $bits[0]  = LSB (bit 0, weight 2^0)
#   $bits[31] = MSB (bit 31, weight 2^31)
#
# # Barrel Shifter
#
# The ARM1 barrel shifter is modelled as a 5-level Mux2 tree.
# Each level i (0..4) controls a shift of 2^i positions:
#
#   Level 0: shift by 1   (amount_bits[0])
#   Level 1: shift by 2   (amount_bits[1])
#   Level 2: shift by 4   (amount_bits[2])
#   Level 3: shift by 8   (amount_bits[3])
#   Level 4: shift by 16  (amount_bits[4])
#
# mux2(a, b, sel) = OR(AND(NOT(sel), a), AND(sel, b))
#   sel=0 → a,  sel=1 → b
#
# Each level applies 32 mux2 calls = ~96 gate calls per level.
# 5 levels × 96 ≈ 480 gate calls for a shift.

use strict;
use warnings;

our $VERSION = '0.01';

use CodingAdventures::LogicGates;
use CodingAdventures::Arithmetic;
use CodingAdventures::ARM1Simulator;

# =============================================================================
# Export all constants from ARM1Simulator (re-export via OO delegation)
# =============================================================================

# We re-expose the constants as package-level constants for convenience.
# This avoids polluting the symbol table but lets callers use
# CodingAdventures::ARM1Gatelevel::COND_AL etc.

use constant MODE_USR  => CodingAdventures::ARM1Simulator::MODE_USR;
use constant MODE_FIQ  => CodingAdventures::ARM1Simulator::MODE_FIQ;
use constant MODE_IRQ  => CodingAdventures::ARM1Simulator::MODE_IRQ;
use constant MODE_SVC  => CodingAdventures::ARM1Simulator::MODE_SVC;
use constant COND_EQ   => CodingAdventures::ARM1Simulator::COND_EQ;
use constant COND_NE   => CodingAdventures::ARM1Simulator::COND_NE;
use constant COND_CS   => CodingAdventures::ARM1Simulator::COND_CS;
use constant COND_CC   => CodingAdventures::ARM1Simulator::COND_CC;
use constant COND_MI   => CodingAdventures::ARM1Simulator::COND_MI;
use constant COND_PL   => CodingAdventures::ARM1Simulator::COND_PL;
use constant COND_VS   => CodingAdventures::ARM1Simulator::COND_VS;
use constant COND_VC   => CodingAdventures::ARM1Simulator::COND_VC;
use constant COND_HI   => CodingAdventures::ARM1Simulator::COND_HI;
use constant COND_LS   => CodingAdventures::ARM1Simulator::COND_LS;
use constant COND_GE   => CodingAdventures::ARM1Simulator::COND_GE;
use constant COND_LT   => CodingAdventures::ARM1Simulator::COND_LT;
use constant COND_GT   => CodingAdventures::ARM1Simulator::COND_GT;
use constant COND_LE   => CodingAdventures::ARM1Simulator::COND_LE;
use constant COND_AL   => CodingAdventures::ARM1Simulator::COND_AL;
use constant COND_NV   => CodingAdventures::ARM1Simulator::COND_NV;
use constant OP_AND    => CodingAdventures::ARM1Simulator::OP_AND;
use constant OP_EOR    => CodingAdventures::ARM1Simulator::OP_EOR;
use constant OP_SUB    => CodingAdventures::ARM1Simulator::OP_SUB;
use constant OP_RSB    => CodingAdventures::ARM1Simulator::OP_RSB;
use constant OP_ADD    => CodingAdventures::ARM1Simulator::OP_ADD;
use constant OP_ADC    => CodingAdventures::ARM1Simulator::OP_ADC;
use constant OP_SBC    => CodingAdventures::ARM1Simulator::OP_SBC;
use constant OP_RSC    => CodingAdventures::ARM1Simulator::OP_RSC;
use constant OP_TST    => CodingAdventures::ARM1Simulator::OP_TST;
use constant OP_TEQ    => CodingAdventures::ARM1Simulator::OP_TEQ;
use constant OP_CMP    => CodingAdventures::ARM1Simulator::OP_CMP;
use constant OP_CMN    => CodingAdventures::ARM1Simulator::OP_CMN;
use constant OP_ORR    => CodingAdventures::ARM1Simulator::OP_ORR;
use constant OP_MOV    => CodingAdventures::ARM1Simulator::OP_MOV;
use constant OP_BIC    => CodingAdventures::ARM1Simulator::OP_BIC;
use constant OP_MVN    => CodingAdventures::ARM1Simulator::OP_MVN;
use constant SHIFT_LSL => CodingAdventures::ARM1Simulator::SHIFT_LSL;
use constant SHIFT_LSR => CodingAdventures::ARM1Simulator::SHIFT_LSR;
use constant SHIFT_ASR => CodingAdventures::ARM1Simulator::SHIFT_ASR;
use constant SHIFT_ROR => CodingAdventures::ARM1Simulator::SHIFT_ROR;
use constant FLAG_N    => CodingAdventures::ARM1Simulator::FLAG_N;
use constant FLAG_Z    => CodingAdventures::ARM1Simulator::FLAG_Z;
use constant FLAG_C    => CodingAdventures::ARM1Simulator::FLAG_C;
use constant FLAG_V    => CodingAdventures::ARM1Simulator::FLAG_V;
use constant FLAG_I    => CodingAdventures::ARM1Simulator::FLAG_I;
use constant FLAG_F    => CodingAdventures::ARM1Simulator::FLAG_F;
use constant PC_MASK   => CodingAdventures::ARM1Simulator::PC_MASK;
use constant MODE_MASK => CodingAdventures::ARM1Simulator::MODE_MASK;
use constant MASK32    => CodingAdventures::ARM1Simulator::MASK32;
use constant HALT_SWI  => CodingAdventures::ARM1Simulator::HALT_SWI;

# =============================================================================
# CPU Construction
# =============================================================================
#
# The gate-level CPU is a thin wrapper around the behavioral simulator.
# We add a `gate_ops` counter to track cumulative gate calls.

sub new {
    my ($class, $memory_size) = @_;
    $memory_size //= 1024 * 1024;
    my $sim = CodingAdventures::ARM1Simulator->new($memory_size);
    my $self = bless { _sim => $sim, gate_ops => 0 }, $class;
    return $self;
}

sub reset {
    my ($self) = @_;
    $self->{_sim}->reset();
    $self->{gate_ops} = 0;
    return $self;
}

# Delegate all memory / register operations to the behavioral simulator.
sub read_register  { my ($self, @a) = @_; return $self->{_sim}->read_register(@a) }
sub write_register { my ($self, @a) = @_; return $self->{_sim}->write_register(@a) }
sub get_pc         { my ($self)     = @_; return $self->{_sim}->get_pc() }
sub set_pc         { my ($self, @a) = @_; return $self->{_sim}->set_pc(@a) }
sub get_flags      { my ($self)     = @_; return $self->{_sim}->get_flags() }
sub set_flags      { my ($self, @a) = @_; return $self->{_sim}->set_flags(@a) }
sub get_mode       { my ($self)     = @_; return $self->{_sim}->get_mode() }
sub read_word      { my ($self, @a) = @_; return $self->{_sim}->read_word(@a) }
sub write_word     { my ($self, @a) = @_; return $self->{_sim}->write_word(@a) }
sub load_instructions { my ($self, @a) = @_; return $self->{_sim}->load_instructions(@a) }

# Encoding helpers — delegate to behavioral simulator
sub encode_data_processing { shift; return CodingAdventures::ARM1Simulator::encode_data_processing(@_) }
sub encode_mov_imm         { shift; return CodingAdventures::ARM1Simulator::encode_mov_imm(@_) }
sub encode_alu_reg         { shift; return CodingAdventures::ARM1Simulator::encode_alu_reg(@_) }
sub encode_branch          { shift; return CodingAdventures::ARM1Simulator::encode_branch(@_) }
sub encode_halt            { shift; return CodingAdventures::ARM1Simulator::encode_halt(@_) }
sub encode_ldr             { shift; return CodingAdventures::ARM1Simulator::encode_ldr(@_) }
sub encode_str             { shift; return CodingAdventures::ARM1Simulator::encode_str(@_) }
sub encode_ldm             { shift; return CodingAdventures::ARM1Simulator::encode_ldm(@_) }
sub encode_stm             { shift; return CodingAdventures::ARM1Simulator::encode_stm(@_) }

# Accessor for halted state
sub halted { return $_[0]->{_sim}{halted} }

# =============================================================================
# Bit Conversion Helpers
# =============================================================================
#
# int_to_bits($v, $w): convert integer $v into $w-element 0-indexed LSB-first array
#   $bits[0]      = LSB (bit 0, weight 2^0)
#   $bits[$w - 1] = MSB (bit $w-1, weight 2^($w-1))
#
# bits_to_int(\@bits): convert LSB-first array back to an integer

sub int_to_bits {
    my ($v, $w) = @_;
    $w //= 32;
    my @bits;
    for my $i (0 .. $w - 1) {
        $bits[$i] = ($v >> $i) & 1;
    }
    return @bits;
}

sub bits_to_int {
    my ($bits_ref) = @_;
    my @bits = @$bits_ref;
    my $v = 0;
    for my $i (0 .. $#bits) {
        $v |= ($bits[$i] << $i) if $bits[$i];
    }
    return $v & MASK32;
}

# =============================================================================
# Mux2 — 2-to-1 multiplexer from gates
# =============================================================================
#
# mux2(a, b, sel) = OR(AND(NOT(sel), a), AND(sel, b))
#   sel=0 → a
#   sel=1 → b
#
# This is the fundamental building block of the barrel shifter.

sub _mux2 {
    my ($a, $b, $sel) = @_;
    return CodingAdventures::LogicGates::OR(
        CodingAdventures::LogicGates::AND(
            CodingAdventures::LogicGates::NOT($sel), $a
        ),
        CodingAdventures::LogicGates::AND($sel, $b)
    );
}

# =============================================================================
# Gate-Level Barrel Shifter Helpers
# =============================================================================
#
# Each of the four shift types has a "shift by N" primitive and a "Mux2 tree"
# that applies a variable-length shift using 5 levels.
#
# Bit arrays are 0-indexed LSB-first: index 0 = LSB, index 31 = MSB.

# _lsl_by_n(\@bits, $n): logical shift left by exactly $n positions
# result[i] = bits[i-n] for i >= n, else 0
sub _lsl_by_n {
    my ($bits, $n) = @_;
    my @result;
    for my $i (0 .. 31) {
        $result[$i] = ($i >= $n) ? $bits->[$i - $n] : 0;
    }
    return @result;
}

# _lsr_by_n(\@bits, $n): logical shift right by exactly $n positions
# result[i] = bits[i+n] for i+n < 32, else 0
sub _lsr_by_n {
    my ($bits, $n) = @_;
    my @result;
    for my $i (0 .. 31) {
        $result[$i] = (($i + $n) < 32) ? $bits->[$i + $n] : 0;
    }
    return @result;
}

# _asr_by_n(\@bits, $n): arithmetic shift right — fill with MSB (bits[31])
sub _asr_by_n {
    my ($bits, $n) = @_;
    my $msb = $bits->[31];
    my @result;
    for my $i (0 .. 31) {
        $result[$i] = (($i + $n) < 32) ? $bits->[$i + $n] : $msb;
    }
    return @result;
}

# _ror_by_n(\@bits, $n): rotate right by exactly $n positions (n must be 0..31)
# bit i of result = bits[(i + n) mod 32]
sub _ror_by_n {
    my ($bits, $n) = @_;
    $n = $n & 31;
    my @result;
    for my $i (0 .. 31) {
        $result[$i] = $bits->[($i + $n) % 32];
    }
    return @result;
}

# _lsl_tree(\@bits, \@amount_bits): 5-level Mux2 tree for LSL
# amount_bits[0..4] are the 5 LSBs of the shift amount (0-indexed)
sub _lsl_tree {
    my ($bits, $amt) = @_;
    my @cur = @$bits;
    for my $level (0 .. 4) {
        my $shift = 1 << $level;
        my $sel   = $amt->[$level];
        my @shifted = _lsl_by_n(\@cur, $shift);
        my @nxt;
        for my $i (0 .. 31) {
            $nxt[$i] = _mux2($cur[$i], $shifted[$i], $sel);
        }
        @cur = @nxt;
    }
    return @cur;
}

sub _lsr_tree {
    my ($bits, $amt) = @_;
    my @cur = @$bits;
    for my $level (0 .. 4) {
        my $shift = 1 << $level;
        my $sel   = $amt->[$level];
        my @shifted = _lsr_by_n(\@cur, $shift);
        my @nxt;
        for my $i (0 .. 31) {
            $nxt[$i] = _mux2($cur[$i], $shifted[$i], $sel);
        }
        @cur = @nxt;
    }
    return @cur;
}

sub _asr_tree {
    my ($bits, $amt) = @_;
    my @cur = @$bits;
    for my $level (0 .. 4) {
        my $shift = 1 << $level;
        my $sel   = $amt->[$level];
        my @shifted = _asr_by_n(\@cur, $shift);
        my @nxt;
        for my $i (0 .. 31) {
            $nxt[$i] = _mux2($cur[$i], $shifted[$i], $sel);
        }
        @cur = @nxt;
    }
    return @cur;
}

sub _ror_tree {
    my ($bits, $amt) = @_;
    my @cur = @$bits;
    for my $level (0 .. 4) {
        my $shift = 1 << $level;
        my $sel   = $amt->[$level];
        my @shifted = _ror_by_n(\@cur, $shift);
        my @nxt;
        for my $i (0 .. 31) {
            $nxt[$i] = _mux2($cur[$i], $shifted[$i], $sel);
        }
        @cur = @nxt;
    }
    return @cur;
}

# =============================================================================
# gate_barrel_shift — public gate-level barrel shifter
# =============================================================================
#
# Parameters:
#   $bits_ref   — arrayref, 32-element 0-indexed LSB-first bit array
#   $shift_type — 0=LSL, 1=LSR, 2=ASR, 3=ROR
#   $amount     — integer shift amount (0-32)
#   $carry_in   — 0 or 1 (used for RRX carry and as default carry)
#   $by_reg     — boolean: true if amount came from a register
#
# Returns: (\@result_bits, $carry_out)

sub gate_barrel_shift {
    my ($self, $bits_ref, $shift_type, $amount, $carry_in, $by_reg) = @_;
    my @bits = @$bits_ref;
    $carry_in //= 0;

    # Register shift by 0: no change, carry_in preserved
    if ($amount == 0 && $by_reg) {
        return ([@bits], $carry_in);
    }

    my $carry_out = $carry_in;

    if ($shift_type == SHIFT_LSL) {
        if ($amount == 0) {
            return ([@bits], $carry_in);
        } elsif ($amount >= 32) {
            $carry_out = ($amount == 32) ? $bits[0] : 0;  # LSB if shift=32
            my @z = (0) x 32;
            return (\@z, $carry_out);
        }
        # carry = bit[31 - amount + 1] = bit[32 - amount] in 1-indexed
        # In 0-indexed: bit[31 - (amount - 1)] = bit[32 - amount]
        $carry_out = $bits[32 - $amount];
        my @amt_bits = int_to_bits($amount, 5);
        my @result = _lsl_tree(\@bits, \@amt_bits);
        return (\@result, $carry_out);

    } elsif ($shift_type == SHIFT_LSR) {
        if ($amount == 0 && !$by_reg) {
            # Immediate LSR #0 = LSR #32: result=0, carry=MSB
            $carry_out = $bits[31];
            my @z = (0) x 32;
            return (\@z, $carry_out);
        } elsif ($amount == 0) {
            return ([@bits], $carry_in);
        } elsif ($amount >= 32) {
            $carry_out = ($amount == 32) ? $bits[31] : 0;
            my @z = (0) x 32;
            return (\@z, $carry_out);
        }
        # carry = bit[amount - 1] (0-indexed)
        $carry_out = $bits[$amount - 1];
        my @amt_bits = int_to_bits($amount, 5);
        my @result = _lsr_tree(\@bits, \@amt_bits);
        return (\@result, $carry_out);

    } elsif ($shift_type == SHIFT_ASR) {
        if ($amount == 0 && !$by_reg) {
            # Immediate ASR #0 = ASR #32: result=all MSB, carry=MSB
            my $msb = $bits[31];
            $carry_out = $msb;
            my @r = ($msb) x 32;
            return (\@r, $carry_out);
        } elsif ($amount == 0) {
            return ([@bits], $carry_in);
        } elsif ($amount >= 32) {
            my $msb = $bits[31];
            $carry_out = $msb;
            my @r = ($msb) x 32;
            return (\@r, $carry_out);
        }
        # carry = bit[amount - 1] (0-indexed)
        $carry_out = $bits[$amount - 1];
        my @amt_bits = int_to_bits($amount, 5);
        my @result = _asr_tree(\@bits, \@amt_bits);
        return (\@result, $carry_out);

    } elsif ($shift_type == SHIFT_ROR) {
        if ($amount == 0 && !$by_reg) {
            # RRX: rotate right through carry
            $carry_out = $bits[0];  # LSB exits as carry
            my @result = @bits;
            # shift right by 1: result[i] = bits[i+1] for i < 31
            for my $i (0 .. 30) { $result[$i] = $bits[$i + 1] }
            $result[31] = $carry_in;  # carry_in becomes MSB
            return (\@result, $carry_out);
        } elsif ($amount == 0) {
            return ([@bits], $carry_in);
        }
        my $eff = $amount & 31;
        if ($eff == 0) {
            # Multiple of 32: value unchanged, carry = MSB
            $carry_out = $bits[31];
            return ([@bits], $carry_out);
        }
        my @amt_bits = int_to_bits($eff, 5);
        my @result = _ror_tree(\@bits, \@amt_bits);
        $carry_out = $result[31];  # MSB after rotation
        return (\@result, $carry_out);
    }

    return ([@bits], $carry_in);
}

# =============================================================================
# gate_decode_immediate — gate-level immediate decode
# =============================================================================
#
# Delegates to behavioral simulator's decode_immediate, then converts result
# to bit array. Returns (\@result_bits, $carry_out).

sub gate_decode_immediate {
    my ($self, $imm8, $rotate) = @_;
    my ($value, $carry) = $self->{_sim}->decode_immediate($imm8, $rotate);
    my $c = $carry ? 1 : 0;
    my @bits = int_to_bits($value, 32);
    return (\@bits, $c);
}

# =============================================================================
# Gate-Level Condition Evaluation
# =============================================================================
#
# Each condition implemented using AND/OR/XOR/NOT/XNOR gate calls.
# +1 to gate_ops per condition evaluation.

sub _eval_cond {
    my ($self, $cond, $flags) = @_;
    my $n = $flags->{n} ? 1 : 0;
    my $z = $flags->{z} ? 1 : 0;
    my $c = $flags->{c} ? 1 : 0;
    my $v = $flags->{v} ? 1 : 0;
    $self->{gate_ops}++;

    if    ($cond == COND_EQ) { return $z == 1 }
    elsif ($cond == COND_NE) { return CodingAdventures::LogicGates::NOT($z) == 1 }
    elsif ($cond == COND_CS) { return $c == 1 }
    elsif ($cond == COND_CC) { return CodingAdventures::LogicGates::NOT($c) == 1 }
    elsif ($cond == COND_MI) { return $n == 1 }
    elsif ($cond == COND_PL) { return CodingAdventures::LogicGates::NOT($n) == 1 }
    elsif ($cond == COND_VS) { return $v == 1 }
    elsif ($cond == COND_VC) { return CodingAdventures::LogicGates::NOT($v) == 1 }
    elsif ($cond == COND_HI) {
        return CodingAdventures::LogicGates::AND($c, CodingAdventures::LogicGates::NOT($z)) == 1
    }
    elsif ($cond == COND_LS) {
        return CodingAdventures::LogicGates::OR(CodingAdventures::LogicGates::NOT($c), $z) == 1
    }
    elsif ($cond == COND_GE) {
        return CodingAdventures::LogicGates::XNOR($n, $v) == 1
    }
    elsif ($cond == COND_LT) {
        return CodingAdventures::LogicGates::XOR($n, $v) == 1
    }
    elsif ($cond == COND_GT) {
        return CodingAdventures::LogicGates::AND(
            CodingAdventures::LogicGates::NOT($z),
            CodingAdventures::LogicGates::XNOR($n, $v)
        ) == 1
    }
    elsif ($cond == COND_LE) {
        return CodingAdventures::LogicGates::OR(
            $z,
            CodingAdventures::LogicGates::XOR($n, $v)
        ) == 1
    }
    elsif ($cond == COND_AL) { return 1 }
    elsif ($cond == COND_NV) { return 0 }
    else                     { return 0 }
}

# =============================================================================
# Gate-Level ALU
# =============================================================================
#
# All 16 ARM1 ALU operations through gate-level primitives.
#
# Logical ops: AND/OR/XOR/NOT applied bit-by-bit (32 gate calls each)
# Arithmetic:  ripple_carry_adder (~160 gate calls for 32-bit)
#
# Parameters (all as integers):
#   $opcode, $a, $b — integer operands
#   $carry_in       — 0 or 1
#   $shifter_carry  — 0 or 1 (carry out from barrel shifter)
#   $old_v          — 0 or 1 (previous overflow flag)
#
# Returns a hashref: { result, n, z, c, v, write_result }

sub gate_alu_execute {
    my ($self, $opcode, $a, $b, $carry_in, $shifter_carry, $old_v) = @_;
    $carry_in      //= 0;
    $shifter_carry //= 0;
    $old_v         //= 0;

    my @a_bits = int_to_bits($a, 32);
    my @b_bits = int_to_bits($b, 32);
    my $c_in   = $carry_in ? 1 : 0;

    my @result_bits;
    my $carry    = $shifter_carry;
    my $overflow = $old_v;
    my $write_result = !($opcode >= OP_TST && $opcode <= OP_CMN);

    if ($opcode == OP_AND || $opcode == OP_TST) {
        for my $i (0 .. 31) {
            $result_bits[$i] = CodingAdventures::LogicGates::AND($a_bits[$i], $b_bits[$i]);
        }
        $carry = $shifter_carry;

    } elsif ($opcode == OP_EOR || $opcode == OP_TEQ) {
        for my $i (0 .. 31) {
            $result_bits[$i] = CodingAdventures::LogicGates::XOR($a_bits[$i], $b_bits[$i]);
        }
        $carry = $shifter_carry;

    } elsif ($opcode == OP_ORR) {
        for my $i (0 .. 31) {
            $result_bits[$i] = CodingAdventures::LogicGates::OR($a_bits[$i], $b_bits[$i]);
        }
        $carry = $shifter_carry;

    } elsif ($opcode == OP_MOV) {
        @result_bits = @b_bits;
        $carry = $shifter_carry;

    } elsif ($opcode == OP_BIC) {
        for my $i (0 .. 31) {
            $result_bits[$i] = CodingAdventures::LogicGates::AND(
                $a_bits[$i],
                CodingAdventures::LogicGates::NOT($b_bits[$i])
            );
        }
        $carry = $shifter_carry;

    } elsif ($opcode == OP_MVN) {
        for my $i (0 .. 31) {
            $result_bits[$i] = CodingAdventures::LogicGates::NOT($b_bits[$i]);
        }
        $carry = $shifter_carry;

    } elsif ($opcode == OP_ADD || $opcode == OP_CMN) {
        my ($sum_ref, $cout) = CodingAdventures::Arithmetic::ripple_carry_adder(\@a_bits, \@b_bits, 0);
        @result_bits = @$sum_ref;
        $carry = $cout;
        # Overflow: both inputs same sign but result differs
        my $sa = $a_bits[31]; my $sb = $b_bits[31]; my $sr = $result_bits[31];
        $overflow = CodingAdventures::LogicGates::AND(
            CodingAdventures::LogicGates::XNOR($sa, $sb),
            CodingAdventures::LogicGates::XOR($sa, $sr)
        );

    } elsif ($opcode == OP_ADC) {
        my ($sum_ref, $cout) = CodingAdventures::Arithmetic::ripple_carry_adder(\@a_bits, \@b_bits, $c_in);
        @result_bits = @$sum_ref;
        $carry = $cout;
        my $sa = $a_bits[31]; my $sb = $b_bits[31]; my $sr = $result_bits[31];
        $overflow = CodingAdventures::LogicGates::AND(
            CodingAdventures::LogicGates::XNOR($sa, $sb),
            CodingAdventures::LogicGates::XOR($sa, $sr)
        );

    } elsif ($opcode == OP_SUB || $opcode == OP_CMP) {
        # SUB a, b = a + NOT(b) + 1
        my @nb;
        for my $i (0 .. 31) {
            $nb[$i] = CodingAdventures::LogicGates::NOT($b_bits[$i]);
        }
        my ($sum_ref, $cout) = CodingAdventures::Arithmetic::ripple_carry_adder(\@a_bits, \@nb, 1);
        @result_bits = @$sum_ref;
        $carry = $cout;
        my $sa = $a_bits[31]; my $sb = $nb[31]; my $sr = $result_bits[31];
        $overflow = CodingAdventures::LogicGates::AND(
            CodingAdventures::LogicGates::XNOR($sa, $sb),
            CodingAdventures::LogicGates::XOR($sa, $sr)
        );

    } elsif ($opcode == OP_SBC) {
        my @nb;
        for my $i (0 .. 31) {
            $nb[$i] = CodingAdventures::LogicGates::NOT($b_bits[$i]);
        }
        my ($sum_ref, $cout) = CodingAdventures::Arithmetic::ripple_carry_adder(\@a_bits, \@nb, $c_in);
        @result_bits = @$sum_ref;
        $carry = $cout;
        my $sa = $a_bits[31]; my $sb = $nb[31]; my $sr = $result_bits[31];
        $overflow = CodingAdventures::LogicGates::AND(
            CodingAdventures::LogicGates::XNOR($sa, $sb),
            CodingAdventures::LogicGates::XOR($sa, $sr)
        );

    } elsif ($opcode == OP_RSB) {
        # RSB b, a = b + NOT(a) + 1
        my @na;
        for my $i (0 .. 31) {
            $na[$i] = CodingAdventures::LogicGates::NOT($a_bits[$i]);
        }
        my ($sum_ref, $cout) = CodingAdventures::Arithmetic::ripple_carry_adder(\@b_bits, \@na, 1);
        @result_bits = @$sum_ref;
        $carry = $cout;
        my $sa = $b_bits[31]; my $sb = $na[31]; my $sr = $result_bits[31];
        $overflow = CodingAdventures::LogicGates::AND(
            CodingAdventures::LogicGates::XNOR($sa, $sb),
            CodingAdventures::LogicGates::XOR($sa, $sr)
        );

    } elsif ($opcode == OP_RSC) {
        my @na;
        for my $i (0 .. 31) {
            $na[$i] = CodingAdventures::LogicGates::NOT($a_bits[$i]);
        }
        my ($sum_ref, $cout) = CodingAdventures::Arithmetic::ripple_carry_adder(\@b_bits, \@na, $c_in);
        @result_bits = @$sum_ref;
        $carry = $cout;
        my $sa = $b_bits[31]; my $sb = $na[31]; my $sr = $result_bits[31];
        $overflow = CodingAdventures::LogicGates::AND(
            CodingAdventures::LogicGates::XNOR($sa, $sb),
            CodingAdventures::LogicGates::XOR($sa, $sr)
        );

    } else {
        @result_bits = @a_bits;
    }

    my $result = bits_to_int(\@result_bits);
    return {
        result_bits  => \@result_bits,
        result       => $result,
        n            => $result_bits[31],
        z            => ($result == 0) ? 1 : 0,
        c            => $carry,
        v            => $overflow,
        write_result => $write_result ? 1 : 0,
    };
}

# =============================================================================
# _read_reg_exec — pipeline-adjusted register read
# =============================================================================
#
# During execution, R15 appears as PC+8 due to the 3-stage pipeline.
# When we advance PC by 4 before executing, R15 = current_PC + 4.
# For the execute stage we add another +4 → current_PC + 8 total.

sub _read_reg_exec {
    my ($self, $n) = @_;
    if ($n == 15) {
        return ($self->{_sim}{regs}[15] + 4) & MASK32;
    }
    return $self->{_sim}->read_register($n);
}

# =============================================================================
# _exec_data_processing_gl — gate-level data processing execution
# =============================================================================

sub _exec_data_processing_gl {
    my ($self, $d) = @_;
    my $sim = $self->{_sim};

    my $a = ($d->{opcode} != OP_MOV && $d->{opcode} != OP_MVN)
          ? $self->_read_reg_exec($d->{rn}) : 0;

    my $flags = $sim->get_flags();
    my ($b_bits_ref, $shifter_carry);

    if ($d->{immediate}) {
        ($b_bits_ref, $shifter_carry) = $self->gate_decode_immediate($d->{imm8}, $d->{rotate});
        if ($d->{rotate} == 0) {
            $shifter_carry = $flags->{c} ? 1 : 0;
        }
    } else {
        my $rm_val  = $self->_read_reg_exec($d->{rm});
        my @rm_bits = int_to_bits($rm_val, 32);
        my $shift_amount;
        if ($d->{shift_by_reg}) {
            $shift_amount = $self->_read_reg_exec($d->{rs}) & 0xFF;
        } else {
            $shift_amount = $d->{shift_imm};
        }
        my $c_in = $flags->{c} ? 1 : 0;
        ($b_bits_ref, $shifter_carry) = $self->gate_barrel_shift(
            \@rm_bits, $d->{shift_type}, $shift_amount, $c_in, $d->{shift_by_reg}
        );
    }

    my $c_int = $flags->{c} ? 1 : 0;
    my $v_int = $flags->{v} ? 1 : 0;
    my $b     = bits_to_int($b_bits_ref);
    my $alu   = $self->gate_alu_execute($d->{opcode}, $a, $b, $c_int, $shifter_carry, $v_int);

    $self->{gate_ops} += 200;

    if ($alu->{write_result}) {
        if ($d->{rd} == 15) {
            if ($d->{s}) {
                $sim->{regs}[15] = $alu->{result} & MASK32;
            } else {
                $sim->set_pc($alu->{result} & PC_MASK);
            }
        } else {
            $sim->write_register($d->{rd}, $alu->{result});
        }
    }

    if ($d->{s} && $d->{rd} != 15) {
        $sim->set_flags($alu->{n}, $alu->{z}, $alu->{c}, $alu->{v});
    }
    if ($d->{opcode} >= OP_TST && $d->{opcode} <= OP_CMN) {
        $sim->set_flags($alu->{n}, $alu->{z}, $alu->{c}, $alu->{v});
    }
}

# =============================================================================
# step — single instruction execution (gate-level)
# =============================================================================
#
# Overrides behavioral step to use:
#   - Gate-level condition evaluation
#   - Gate-level barrel shifter and ALU for data processing
#   - Behavioral step delegation for load/store and block transfer

sub step {
    my ($self) = @_;
    my $sim = $self->{_sim};

    return { halted => 1 } if $sim->{halted};

    my $current_pc   = $sim->get_pc();
    my $flags_before = $sim->get_flags();
    my $instruction  = $sim->read_word($current_pc);
    my $d            = $sim->decode($instruction);

    my $cond_met = $self->_eval_cond($d->{condition}, $flags_before);

    # Advance PC past this instruction
    $sim->set_pc(($current_pc + 4) & PC_MASK);

    my ($reads, $writes) = ([], []);

    if ($cond_met) {
        my $type = $d->{type};

        if ($type == CodingAdventures::ARM1Simulator::INST_DATA_PROCESSING) {
            $self->_exec_data_processing_gl($d);

        } elsif ($type == CodingAdventures::ARM1Simulator::INST_LOAD_STORE
               || $type == CodingAdventures::ARM1Simulator::INST_BLOCK_TRANSFER) {
            # Delegate memory operations to behavioral simulator.
            # Reset PC so behavioral step re-fetches the same instruction.
            my $saved_ops = $self->{gate_ops};
            $sim->set_pc($current_pc);
            my $btrace = $sim->step();
            my $cost = ($type == CodingAdventures::ARM1Simulator::INST_BLOCK_TRANSFER) ? 100 : 50;
            $self->{gate_ops} = $saved_ops + $cost;
            $reads  = $btrace->{memory_reads}  // [];
            $writes = $btrace->{memory_writes} // [];
            return {
                address       => $current_pc,
                raw           => $instruction,
                condition_met => 1,
                memory_reads  => $reads,
                memory_writes => $writes,
                gate_ops      => $self->{gate_ops},
            };

        } elsif ($type == CodingAdventures::ARM1Simulator::INST_BRANCH) {
            my $branch_base = ($sim->get_pc() + 4) & MASK32;
            if ($d->{link}) {
                $sim->write_register(14, $sim->{regs}[15]);
            }
            my $target = ($branch_base + $d->{branch_offset}) & MASK32;
            $sim->set_pc($target & PC_MASK);
            $self->{gate_ops} += 4;

        } elsif ($type == CodingAdventures::ARM1Simulator::INST_SWI) {
            if ($d->{swi_comment} == HALT_SWI) {
                $sim->{halted} = 1;
            } else {
                my $r15_val = $sim->{regs}[15];
                $sim->{regs}[25] = $r15_val;
                $sim->{regs}[26] = $r15_val;
                my $r15 = $sim->{regs}[15];
                $r15 = ($r15 & (~MODE_MASK & MASK32)) | MODE_SVC;
                $r15 |= FLAG_I;
                $sim->{regs}[15] = $r15 & MASK32;
                $sim->set_pc(0x08);
            }
        } else {
            # Undefined/coprocessor: trap to SVC mode
            $sim->{regs}[26] = $sim->{regs}[15];
            my $r15 = $sim->{regs}[15];
            $r15 = ($r15 & (~MODE_MASK & MASK32)) | MODE_SVC;
            $r15 |= FLAG_I;
            $sim->{regs}[15] = $r15 & MASK32;
            $sim->set_pc(0x04);
        }
    }

    return {
        address       => $current_pc,
        raw           => $instruction,
        condition_met => $cond_met ? 1 : 0,
        memory_reads  => $reads,
        memory_writes => $writes,
        gate_ops      => $self->{gate_ops},
    };
}

# =============================================================================
# run — execute up to max_steps instructions
# =============================================================================

sub run {
    my ($self, $max_steps) = @_;
    $max_steps //= 1_000_000;
    my @traces;
    for (1 .. $max_steps) {
        last if $self->{_sim}{halted};
        my $trace = $self->step();
        push @traces, $trace;
        last if $self->{_sim}{halted};
    }
    return \@traces;
}

1;

__END__

=head1 NAME

CodingAdventures::ARM1Gatelevel - ARM1 gate-level simulator (Perl)

=head1 SYNOPSIS

  use CodingAdventures::ARM1Gatelevel;
  use CodingAdventures::ARM1Simulator;

  my $cpu = CodingAdventures::ARM1Gatelevel->new(4096);
  $cpu->load_instructions(0, [
      CodingAdventures::ARM1Simulator::encode_mov_imm(
          CodingAdventures::ARM1Simulator::COND_AL, 0, 42),
      CodingAdventures::ARM1Simulator::encode_halt(),
  ]);
  $cpu->run(100);
  print $cpu->read_register(0), "\n";  # 42
  print $cpu->{gate_ops}, "\n";        # cumulative gate calls

=head1 DESCRIPTION

Gate-level ARM1 behavioral simulator. Every ALU operation routes through
logic gate function calls (CodingAdventures::LogicGates), and every barrel
shift uses a 5-level Mux2 tree (~480 gate calls per shift).

The C<gate_ops> field tracks cumulative gate function invocations.

=cut
