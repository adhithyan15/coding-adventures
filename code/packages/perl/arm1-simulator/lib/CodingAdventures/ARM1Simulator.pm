package CodingAdventures::ARM1Simulator;

# ===========================================================================
# CodingAdventures::ARM1Simulator — ARM1 Behavioral Simulator in Pure Perl
# ===========================================================================
#
# The ARM1 was the first commercial ARM processor, designed by Sophie Wilson
# and Steve Furber at Acorn Computers, Cambridge. It first powered on April
# 26, 1985, and worked correctly on its very first attempt. Sophie Wilson
# tested it by typing `PRINT PI` at a BBC Micro and got the right answer.
#
# This module implements a complete behavioral simulator of the ARMv1
# instruction set — the same ISA running in 250+ billion devices today.
#
# # Key ARM1 Characteristics
#
#   * 32-bit RISC processor with fixed 32-bit instructions
#   * 25,000 transistors (vs. 275,000 for the Intel 386 released same year)
#   * ~0.1W power (vs. 2W for the 386 — the "accidental" power efficiency
#     that later made ARM dominate mobile computing)
#   * 16 visible registers, 25 physical (banked modes)
#   * R15 = PC + Status Register (flags + mode bits combined)
#   * Conditional execution on EVERY instruction
#   * Free barrel shifter on every data processing instruction
#   * 26-bit address space (64 MiB)
#
# # R15 Layout (unique to ARMv1)
#
#  31  30  29  28  27  26  25                    2   1   0
# ┌───┬───┬───┬───┬───┬───┬──────────────────────┬───┬───┐
# │ N │ Z │ C │ V │ I │ F │   24-bit PC           │M1 │M0 │
# └───┴───┴───┴───┴───┴───┴──────────────────────┴───┴───┘
#
# # Perl Integer Notes
#
# Perl uses platform-native integers (typically 64-bit on modern systems).
# We mask with 0xFFFFFFFF to simulate 32-bit unsigned registers. The &, |,
# ^, ~, >>, << operators work on Perl integers. Note: ~ in Perl does NOT
# produce 32-bit NOT — we use ~$v & 0xFFFFFFFF.
#
# ===========================================================================

use strict;
use warnings;
use Carp qw(croak);
use POSIX qw(floor);

our $VERSION = '0.01';

# ===========================================================================
# Constants — Processor Modes
# ===========================================================================

use constant {
    MODE_USR => 0,   # User mode — normal operation
    MODE_FIQ => 1,   # Fast Interrupt — banks R8-R14
    MODE_IRQ => 2,   # Normal Interrupt — banks R13, R14
    MODE_SVC => 3,   # Supervisor — OS mode, banks R13, R14
};

my %MODE_NAMES = ( 0 => 'USR', 1 => 'FIQ', 2 => 'IRQ', 3 => 'SVC' );

# ===========================================================================
# Constants — Condition Codes
# ===========================================================================
#
# ARM's signature feature: every instruction has a 4-bit condition field.
# The instruction only executes if the condition is satisfied.
#
#   Code  Suffix  Test
#   ----  ------  ----
#   0000  EQ      Z=1 (equal after CMP)
#   0001  NE      Z=0
#   0010  CS      C=1 (carry set / unsigned higher-or-same)
#   0011  CC      C=0 (carry clear / unsigned lower)
#   0100  MI      N=1 (minus/negative)
#   0101  PL      N=0 (plus/non-negative)
#   0110  VS      V=1 (overflow)
#   0111  VC      V=0 (no overflow)
#   1000  HI      C=1 AND Z=0 (unsigned higher)
#   1001  LS      C=0 OR Z=1  (unsigned lower or same)
#   1010  GE      N=V (signed greater or equal)
#   1011  LT      N≠V (signed less than)
#   1100  GT      Z=0 AND N=V (signed greater than)
#   1101  LE      Z=1 OR N≠V  (signed less or equal)
#   1110  AL      always (unconditional)
#   1111  NV      never (reserved — always fails)

use constant {
    COND_EQ => 0x0, COND_NE => 0x1, COND_CS => 0x2, COND_CC => 0x3,
    COND_MI => 0x4, COND_PL => 0x5, COND_VS => 0x6, COND_VC => 0x7,
    COND_HI => 0x8, COND_LS => 0x9, COND_GE => 0xA, COND_LT => 0xB,
    COND_GT => 0xC, COND_LE => 0xD, COND_AL => 0xE, COND_NV => 0xF,
};

my @COND_NAMES = qw(EQ NE CS CC MI PL VS VC HI LS GE LT GT LE "" NV);

# ===========================================================================
# Constants — ALU Opcodes
# ===========================================================================
#
# 16 ALU operations selected by bits 24:21 of data processing instructions.
# Note: TST, TEQ, CMP, CMN are "test" ops — they set flags but write no Rd.

use constant {
    OP_AND => 0x0,  # Rd = Rn AND Op2
    OP_EOR => 0x1,  # Rd = Rn XOR Op2
    OP_SUB => 0x2,  # Rd = Rn - Op2
    OP_RSB => 0x3,  # Rd = Op2 - Rn  (Reverse Subtract)
    OP_ADD => 0x4,  # Rd = Rn + Op2
    OP_ADC => 0x5,  # Rd = Rn + Op2 + C
    OP_SBC => 0x6,  # Rd = Rn - Op2 - NOT(C)
    OP_RSC => 0x7,  # Rd = Op2 - Rn - NOT(C)
    OP_TST => 0x8,  # flags = Rn AND Op2 (no Rd write)
    OP_TEQ => 0x9,  # flags = Rn XOR Op2
    OP_CMP => 0xA,  # flags = Rn - Op2
    OP_CMN => 0xB,  # flags = Rn + Op2
    OP_ORR => 0xC,  # Rd = Rn OR Op2
    OP_MOV => 0xD,  # Rd = Op2 (Rn ignored)
    OP_BIC => 0xE,  # Rd = Rn AND NOT(Op2)  (Bit Clear)
    OP_MVN => 0xF,  # Rd = NOT(Op2)         (Move Negated)
};

my @OP_NAMES = qw(AND EOR SUB RSB ADD ADC SBC RSC TST TEQ CMP CMN ORR MOV BIC MVN);

# ===========================================================================
# Constants — Shift Types (Barrel Shifter)
# ===========================================================================
#
# The barrel shifter is ARM's most distinctive feature. Every data processing
# instruction passes its second operand through the barrel shifter for free.
#
#   00  LSL  Logical Shift Left   (fill with 0)
#   01  LSR  Logical Shift Right  (fill with 0)
#   10  ASR  Arithmetic Shift Right (fill with sign bit)
#   11  ROR  Rotate Right         (circular rotation)
#       RRX  ROR #0 = Rotate Right Extended through carry (33-bit)

use constant {
    SHIFT_LSL => 0,
    SHIFT_LSR => 1,
    SHIFT_ASR => 2,
    SHIFT_ROR => 3,
};

my @SHIFT_NAMES = qw(LSL LSR ASR ROR);

# ===========================================================================
# Constants — R15 Bit Fields
# ===========================================================================

use constant {
    FLAG_N    => 0x80000000,  # bit 31: Negative
    FLAG_Z    => 0x40000000,  # bit 30: Zero
    FLAG_C    => 0x20000000,  # bit 29: Carry
    FLAG_V    => 0x10000000,  # bit 28: Overflow
    FLAG_I    => 0x08000000,  # bit 27: IRQ disable
    FLAG_F    => 0x04000000,  # bit 26: FIQ disable
    PC_MASK   => 0x03FFFFFC,  # bits 25:2: Program Counter
    MODE_MASK => 0x3,         # bits 1:0: Processor Mode
    MASK32    => 0xFFFFFFFF,  # 32-bit unsigned mask
    HALT_SWI  => 0x123456,   # our pseudo-halt SWI number
};

# ===========================================================================
# Constants — Instruction Types
# ===========================================================================

use constant {
    INST_DATA_PROCESSING => 0,
    INST_LOAD_STORE      => 1,
    INST_BLOCK_TRANSFER  => 2,
    INST_BRANCH          => 3,
    INST_SWI             => 4,
    INST_COPROCESSOR     => 5,
    INST_UNDEFINED       => 6,
};

# ===========================================================================
# Helpers — 32-bit Arithmetic
# ===========================================================================

# Mask a Perl integer to 32 unsigned bits
sub _mask32 { return $_[0] & MASK32 }

# Bitwise NOT, constrained to 32 bits
sub _bnot32 { return (~$_[0]) & MASK32 }

# Arithmetic right shift (fill upper bits with sign bit)
sub _asr {
    my ($v, $amount) = @_;
    return ($v & 0x80000000) ? MASK32 : 0 if $amount >= 32;
    return $v if $amount == 0;
    my $sign = ($v & 0x80000000) ? 1 : 0;
    my $result = $v >> $amount;
    if ($sign) {
        my $fill = ((1 << $amount) - 1) << (32 - $amount);
        $result |= $fill;
    }
    return _mask32($result);
}

# Rotate right by $amount bits (32-bit circular)
sub _ror32 {
    my ($v, $amount) = @_;
    $amount &= 31;
    return _mask32($v) if $amount == 0;
    return _mask32(($v >> $amount) | ($v << (32 - $amount)));
}

# ===========================================================================
# Constructor
# ===========================================================================

=head1 new

  my $cpu = CodingAdventures::ARM1Simulator->new($memory_size);

Creates a new ARM1 simulator. $memory_size defaults to 1 MiB.

=cut

sub new {
    my ($class, $memory_size) = @_;
    $memory_size //= 1024 * 1024;
    $memory_size = 1024 * 1024 if $memory_size <= 0;

    my $self = bless {
        # 27 physical registers:
        #   0-15:  base R0-R15
        #   16-22: FIQ banked R8_fiq..R14_fiq
        #   23-24: IRQ banked R13_irq, R14_irq
        #   25-26: SVC banked R13_svc, R14_svc
        regs        => [ (0) x 27 ],
        memory      => [ (0) x $memory_size ],
        memory_size => $memory_size,
        halted      => 0,
    }, $class;

    $self->reset();
    return $self;
}

# ===========================================================================
# Reset
# ===========================================================================

=head2 reset

  $cpu->reset();

Resets to power-on state: SVC mode, IRQ/FIQ disabled, PC=0, all regs=0.

=cut

sub reset {
    my ($self) = @_;
    $self->{regs}[$_] = 0 for 0..26;
    # R15: SVC mode (bits 1:0=11), IRQ disabled (bit 27), FIQ disabled (bit 26)
    $self->{regs}[15] = FLAG_I | FLAG_F | MODE_SVC;
    $self->{halted} = 0;
}

# ===========================================================================
# Register Access (with mode banking)
# ===========================================================================

# Physical register index based on current mode
sub _physical_reg {
    my ($self, $index) = @_;
    my $mode = $self->{regs}[15] & MODE_MASK;
    if ($mode == MODE_FIQ && $index >= 8 && $index <= 14) {
        return 16 + ($index - 8);
    } elsif ($mode == MODE_IRQ && $index >= 13 && $index <= 14) {
        return 23 + ($index - 13);
    } elsif ($mode == MODE_SVC && $index >= 13 && $index <= 14) {
        return 25 + ($index - 13);
    }
    return $index;
}

=head2 read_register

  my $value = $cpu->read_register($index);

Reads R0-R15, respecting mode banking.

=cut

sub read_register {
    my ($self, $index) = @_;
    return $self->{regs}[ $self->_physical_reg($index) ];
}

=head2 write_register

  $cpu->write_register($index, $value);

Writes R0-R15, respecting mode banking.

=cut

sub write_register {
    my ($self, $index, $value) = @_;
    $self->{regs}[ $self->_physical_reg($index) ] = _mask32($value);
}

=head2 get_pc

  my $pc = $cpu->get_pc();

Returns the current program counter (26-bit, word-aligned).

=cut

sub get_pc { return $_[0]->{regs}[15] & PC_MASK }

=head2 set_pc

  $cpu->set_pc($address);

Sets the PC portion of R15 without disturbing flags or mode.

=cut

sub set_pc {
    my ($self, $addr) = @_;
    my $r15 = $self->{regs}[15];
    $self->{regs}[15] = _mask32(($r15 & _bnot32(PC_MASK)) | ($addr & PC_MASK));
}

=head2 get_flags

  my $flags = $cpu->get_flags();  # returns hashref {n, z, c, v}

=cut

sub get_flags {
    my ($self) = @_;
    my $r15 = $self->{regs}[15];
    return {
        n => ($r15 & FLAG_N) ? 1 : 0,
        z => ($r15 & FLAG_Z) ? 1 : 0,
        c => ($r15 & FLAG_C) ? 1 : 0,
        v => ($r15 & FLAG_V) ? 1 : 0,
    };
}

=head2 set_flags

  $cpu->set_flags({n => 1, z => 0, c => 1, v => 0});

=cut

sub set_flags {
    my ($self, $f) = @_;
    my $r15 = $self->{regs}[15] & _bnot32(FLAG_N | FLAG_Z | FLAG_C | FLAG_V);
    $r15 |= FLAG_N if $f->{n};
    $r15 |= FLAG_Z if $f->{z};
    $r15 |= FLAG_C if $f->{c};
    $r15 |= FLAG_V if $f->{v};
    $self->{regs}[15] = _mask32($r15);
}

=head2 get_mode

  my $mode = $cpu->get_mode();  # 0=USR, 1=FIQ, 2=IRQ, 3=SVC

=cut

sub get_mode { return $_[0]->{regs}[15] & MODE_MASK }

# ===========================================================================
# Memory Access
# ===========================================================================

=head2 read_word

  my $word = $cpu->read_word($address);

Reads a 32-bit little-endian word (word-aligned).

=cut

sub read_word {
    my ($self, $addr) = @_;
    $addr = ($addr & PC_MASK) & ~3;
    return 0 if $addr + 3 >= $self->{memory_size};
    my $mem = $self->{memory};
    return _mask32(
        $mem->[$addr]       |
        ($mem->[$addr + 1] << 8)  |
        ($mem->[$addr + 2] << 16) |
        ($mem->[$addr + 3] << 24)
    );
}

=head2 write_word

  $cpu->write_word($address, $value);

Writes a 32-bit little-endian word (word-aligned).

=cut

sub write_word {
    my ($self, $addr, $value) = @_;
    $addr = ($addr & PC_MASK) & ~3;
    return if $addr + 3 >= $self->{memory_size};
    $value = _mask32($value);
    my $mem = $self->{memory};
    $mem->[$addr]     =  $value        & 0xFF;
    $mem->[$addr + 1] = ($value >>  8) & 0xFF;
    $mem->[$addr + 2] = ($value >> 16) & 0xFF;
    $mem->[$addr + 3] = ($value >> 24) & 0xFF;
}

=head2 read_byte

  my $byte = $cpu->read_byte($address);

=cut

sub read_byte {
    my ($self, $addr) = @_;
    $addr &= 0x03FFFFFF;  # 26-bit address space, all byte positions valid
    return 0 if $addr >= $self->{memory_size};
    return $self->{memory}[$addr] // 0;
}

=head2 write_byte

  $cpu->write_byte($address, $value);

=cut

sub write_byte {
    my ($self, $addr, $value) = @_;
    $addr &= 0x03FFFFFF;  # 26-bit address space, all byte positions valid
    return if $addr >= $self->{memory_size};
    $self->{memory}[$addr] = $value & 0xFF;
}

=head2 load_instructions

  $cpu->load_instructions(\@words, $start_address);

Loads an array of 32-bit instruction words into memory.

=cut

sub load_instructions {
    my ($self, $instructions, $start_address) = @_;
    $start_address //= 0;
    for my $i (0 .. $#$instructions) {
        $self->write_word($start_address + $i * 4, $instructions->[$i]);
    }
}

# ===========================================================================
# Condition Evaluation
# ===========================================================================

sub _evaluate_condition {
    my ($cond, $n, $z, $c, $v) = @_;
    return $z           if $cond == COND_EQ;
    return !$z          if $cond == COND_NE;
    return $c           if $cond == COND_CS;
    return !$c          if $cond == COND_CC;
    return $n           if $cond == COND_MI;
    return !$n          if $cond == COND_PL;
    return $v           if $cond == COND_VS;
    return !$v          if $cond == COND_VC;
    return $c && !$z    if $cond == COND_HI;
    return !$c || $z    if $cond == COND_LS;
    return $n == $v     if $cond == COND_GE;
    return $n != $v     if $cond == COND_LT;
    return !$z && $n == $v  if $cond == COND_GT;
    return $z  || $n != $v  if $cond == COND_LE;
    return 1            if $cond == COND_AL;
    return 0;  # COND_NV and unknown
}

# ===========================================================================
# Barrel Shifter
# ===========================================================================
#
# Returns ($result, $carry_out).
#
# The ARM barrel shifter is a core part of the ARM architecture's elegance.
# Every data processing instruction gets a "free" shift on its second
# operand. On the real ARM1, this was a 32x32 crossbar of pass transistors.

sub _barrel_shift {
    my ($value, $shift_type, $amount, $carry_in, $by_register) = @_;

    # Register shift with amount=0: pass through unchanged
    return ($value, $carry_in) if $by_register && $amount == 0;

    if ($shift_type == SHIFT_LSL) {
        return ($value, $carry_in) if $amount == 0;
        return (0, ($value & 1) ? 1 : 0) if $amount == 32;
        return (0, 0) if $amount > 32;
        my $carry = (($value >> (32 - $amount)) & 1) ? 1 : 0;
        return (_mask32($value << $amount), $carry);

    } elsif ($shift_type == SHIFT_LSR) {
        # Immediate LSR #0 encodes LSR #32
        return (0, ($value >> 31) & 1) if !$by_register && $amount == 0;
        return ($value, $carry_in) if $amount == 0;
        return (0, ($value >> 31) & 1) if $amount == 32;
        return (0, 0) if $amount > 32;
        my $carry = (($value >> ($amount - 1)) & 1) ? 1 : 0;
        return ($value >> $amount, $carry);

    } elsif ($shift_type == SHIFT_ASR) {
        # Immediate ASR #0 encodes ASR #32
        if (!$by_register && $amount == 0) {
            return ($value & 0x80000000) ? (MASK32, 1) : (0, 0);
        }
        return ($value, $carry_in) if $amount == 0;
        if ($amount >= 32) {
            return ($value & 0x80000000) ? (MASK32, 1) : (0, 0);
        }
        my $carry = (($value >> ($amount - 1)) & 1) ? 1 : 0;
        return (_asr($value, $amount), $carry);

    } elsif ($shift_type == SHIFT_ROR) {
        # Immediate ROR #0 encodes RRX (rotate right extended through carry)
        if (!$by_register && $amount == 0) {
            my $carry = $value & 1;
            my $result = $value >> 1;
            $result |= 0x80000000 if $carry_in;
            return (_mask32($result), $carry ? 1 : 0);
        }
        return ($value, $carry_in) if $amount == 0;
        my $result = _ror32($value, $amount);
        my $carry  = ($result >> 31) & 1;
        return ($result, $carry);
    }

    return ($value, $carry_in);
}

# Decode rotated immediate (I=1 data processing)
sub _decode_immediate {
    my ($imm8, $rotate_field) = @_;
    return ($imm8, 0) if $rotate_field == 0;
    my $rotate_amount = $rotate_field * 2;
    my $value = _ror32($imm8, $rotate_amount);
    my $carry = ($value >> 31) & 1;
    return ($value, $carry);
}

# ===========================================================================
# ALU
# ===========================================================================

# 32-bit add with carry — returns (result, carry, overflow)
sub _add32 {
    my ($a, $b, $carry_in) = @_;
    my $cin = $carry_in ? 1 : 0;
    # Use floating point to handle overflow safely (Perl ints may be 64-bit)
    my $sum = $a + $b + $cin;
    my $result = _mask32($sum);
    my $carry = ($sum > MASK32) ? 1 : 0;
    # Overflow: same-sign inputs, different-sign result
    my $overflow = ((($a ^ $result) & ($b ^ $result)) >> 31) & 1;
    return ($result, $carry, $overflow);
}

sub _is_test_op { my $op = shift; return $op >= OP_TST && $op <= OP_CMN }
sub _is_logical_op {
    my $op = shift;
    return $op == OP_AND || $op == OP_EOR || $op == OP_TST || $op == OP_TEQ
        || $op == OP_ORR || $op == OP_MOV || $op == OP_BIC || $op == OP_MVN;
}

# Returns hashref {result, n, z, c, v, write_result}
sub _alu_execute {
    my ($opcode, $a, $b, $carry_in, $shifter_carry, $old_v) = @_;
    my $write_result = !_is_test_op($opcode);
    my ($result, $carry, $overflow);

    if ($opcode == OP_AND || $opcode == OP_TST) {
        $result = ($a & $b) & MASK32;
        ($carry, $overflow) = ($shifter_carry, $old_v);
    } elsif ($opcode == OP_EOR || $opcode == OP_TEQ) {
        $result = ($a ^ $b) & MASK32;
        ($carry, $overflow) = ($shifter_carry, $old_v);
    } elsif ($opcode == OP_ORR) {
        $result = ($a | $b) & MASK32;
        ($carry, $overflow) = ($shifter_carry, $old_v);
    } elsif ($opcode == OP_MOV) {
        $result = $b & MASK32;
        ($carry, $overflow) = ($shifter_carry, $old_v);
    } elsif ($opcode == OP_BIC) {
        $result = ($a & _bnot32($b)) & MASK32;
        ($carry, $overflow) = ($shifter_carry, $old_v);
    } elsif ($opcode == OP_MVN) {
        $result = _bnot32($b);
        ($carry, $overflow) = ($shifter_carry, $old_v);
    } elsif ($opcode == OP_ADD || $opcode == OP_CMN) {
        ($result, $carry, $overflow) = _add32($a, $b, 0);
    } elsif ($opcode == OP_ADC) {
        ($result, $carry, $overflow) = _add32($a, $b, $carry_in);
    } elsif ($opcode == OP_SUB || $opcode == OP_CMP) {
        ($result, $carry, $overflow) = _add32($a, _bnot32($b), 1);
    } elsif ($opcode == OP_SBC) {
        ($result, $carry, $overflow) = _add32($a, _bnot32($b), $carry_in);
    } elsif ($opcode == OP_RSB) {
        ($result, $carry, $overflow) = _add32($b, _bnot32($a), 1);
    } elsif ($opcode == OP_RSC) {
        ($result, $carry, $overflow) = _add32($b, _bnot32($a), $carry_in);
    } else {
        ($result, $carry, $overflow) = (0, 0, 0);
    }

    $result = _mask32($result);
    return {
        result       => $result,
        n            => ($result >> 31) & 1,
        z            => ($result == 0) ? 1 : 0,
        c            => $carry ? 1 : 0,
        v            => $overflow ? 1 : 0,
        write_result => $write_result,
    };
}

# ===========================================================================
# Decoder
# ===========================================================================

sub _decode {
    my ($instruction) = @_;
    my $d = {
        raw           => $instruction,
        condition     => ($instruction >> 28) & 0xF,
        type          => INST_UNDEFINED,
        # Data processing fields
        opcode => 0, s => 0, rn => 0, rd => 0,
        immediate => 0, imm8 => 0, rotate => 0,
        rm => 0, shift_type => 0, shift_by_reg => 0,
        shift_imm => 0, rs => 0,
        # Load/Store fields
        load => 0, byte_access => 0, pre_index => 0,
        up => 0, write_back => 0, offset12 => 0,
        # Block Transfer fields
        register_list => 0, force_user => 0,
        # Branch fields
        link => 0, branch_offset => 0,
        # SWI
        swi_comment => 0,
    };

    my $bits2726 = ($instruction >> 26) & 0x3;
    my $bit25    = ($instruction >> 25) & 0x1;

    if ($bits2726 == 0) {
        # Data Processing
        $d->{type}      = INST_DATA_PROCESSING;
        my $is_imm = (($instruction >> 25) & 1);
        $d->{immediate} = $is_imm;
        $d->{opcode}    = ($instruction >> 21) & 0xF;
        $d->{s}         = ($instruction >> 20) & 1;
        $d->{rn}        = ($instruction >> 16) & 0xF;
        $d->{rd}        = ($instruction >> 12) & 0xF;

        if ($is_imm) {
            $d->{imm8}   = $instruction & 0xFF;
            $d->{rotate} = ($instruction >> 8) & 0xF;
        } else {
            my $shift_by_reg = ($instruction >> 4) & 1;
            $d->{rm}           = $instruction & 0xF;
            $d->{shift_type}   = ($instruction >> 5) & 0x3;
            $d->{shift_by_reg} = $shift_by_reg;
            if ($shift_by_reg) {
                $d->{rs} = ($instruction >> 8) & 0xF;
            } else {
                $d->{shift_imm} = ($instruction >> 7) & 0x1F;
            }
        }

    } elsif ($bits2726 == 1) {
        # Single Data Transfer
        $d->{type}        = INST_LOAD_STORE;
        $d->{immediate}   = ($instruction >> 25) & 1;
        $d->{pre_index}   = ($instruction >> 24) & 1;
        $d->{up}          = ($instruction >> 23) & 1;
        $d->{byte_access} = ($instruction >> 22) & 1;
        $d->{write_back}  = ($instruction >> 21) & 1;
        $d->{load}        = ($instruction >> 20) & 1;
        $d->{rn}          = ($instruction >> 16) & 0xF;
        $d->{rd}          = ($instruction >> 12) & 0xF;
        $d->{rm}          = $instruction & 0xF;
        $d->{shift_type}  = ($instruction >> 5) & 0x3;
        $d->{shift_imm}   = ($instruction >> 7) & 0x1F;
        $d->{offset12}    = $instruction & 0xFFF;

    } elsif ($bits2726 == 2 && $bit25 == 0) {
        # Block Data Transfer
        $d->{type}          = INST_BLOCK_TRANSFER;
        $d->{pre_index}     = ($instruction >> 24) & 1;
        $d->{up}            = ($instruction >> 23) & 1;
        $d->{force_user}    = ($instruction >> 22) & 1;
        $d->{write_back}    = ($instruction >> 21) & 1;
        $d->{load}          = ($instruction >> 20) & 1;
        $d->{rn}            = ($instruction >> 16) & 0xF;
        $d->{register_list} = $instruction & 0xFFFF;

    } elsif ($bits2726 == 2 && $bit25 == 1) {
        # Branch / Branch with Link
        $d->{type}  = INST_BRANCH;
        $d->{link}  = ($instruction >> 24) & 1;
        my $raw_off = $instruction & 0x00FFFFFF;
        # Sign-extend from 24 bits
        if ($raw_off >> 23) {
            $raw_off -= (1 << 24);  # sign extend from 24 bits (portable)
        }
        # Convert to signed and shift left by 2
        # In Perl: use the fact that the value is already signed 64-bit
        # We need to handle wrap-around of Perl integers
        my $signed = $raw_off;
        # If bit 23 of raw was set, we have a negative offset
        # raw_off is now a large positive Perl int due to sign extension
        # Convert: if raw_off >= 2^63 it's negative, subtract 2^64
        if ($raw_off >= (1 << 63)) {
            $signed = $raw_off - (1 << 64);
        }
        $d->{branch_offset} = $signed * 4;

    } elsif ($bits2726 == 3) {
        # Coprocessor / SWI
        if ((($instruction >> 24) & 0xF) == 0xF) {
            $d->{type}        = INST_SWI;
            $d->{swi_comment} = $instruction & 0x00FFFFFF;
        } else {
            $d->{type} = INST_COPROCESSOR;
        }
    }

    return $d;
}

# ===========================================================================
# Disassembly
# ===========================================================================

sub _disasm_reg_list {
    my ($reg_list) = @_;
    my @regs;
    for my $i (0..15) {
        next unless (($reg_list >> $i) & 1);
        push @regs, ($i == 15 ? 'PC' : $i == 14 ? 'LR' : $i == 13 ? 'SP' : "R$i");
    }
    return join(', ', @regs);
}

sub _disasm_operand2 {
    my ($d) = @_;
    if ($d->{immediate}) {
        my ($val) = _decode_immediate($d->{imm8}, $d->{rotate});
        return "#$val";
    }
    if (!$d->{shift_by_reg} && $d->{shift_imm} == 0 && $d->{shift_type} == SHIFT_LSL) {
        return "R$d->{rm}";
    }
    if ($d->{shift_by_reg}) {
        return sprintf("R%d, %s R%d", $d->{rm}, $SHIFT_NAMES[$d->{shift_type}], $d->{rs});
    }
    my $amount = $d->{shift_imm};
    if (($d->{shift_type} == SHIFT_LSR || $d->{shift_type} == SHIFT_ASR) && $amount == 0) {
        $amount = 32;
    } elsif ($d->{shift_type} == SHIFT_ROR && $amount == 0) {
        return sprintf("R%d, RRX", $d->{rm});
    }
    return sprintf("R%d, %s #%d", $d->{rm}, $SHIFT_NAMES[$d->{shift_type}], $amount);
}

sub disassemble {
    my ($d) = @_;
    my $cond = $COND_NAMES[$d->{condition}] // '??';

    if ($d->{type} == INST_DATA_PROCESSING) {
        my $op  = $OP_NAMES[$d->{opcode}] // '???';
        my $suf = ($d->{s} && !_is_test_op($d->{opcode})) ? 'S' : '';
        my $op2 = _disasm_operand2($d);
        if ($d->{opcode} == OP_MOV || $d->{opcode} == OP_MVN) {
            return "${op}${cond}${suf} R$d->{rd}, $op2";
        } elsif (_is_test_op($d->{opcode})) {
            return "${op}${cond} R$d->{rn}, $op2";
        } else {
            return "${op}${cond}${suf} R$d->{rd}, R$d->{rn}, $op2";
        }
    } elsif ($d->{type} == INST_LOAD_STORE) {
        my $op   = $d->{load} ? 'LDR' : 'STR';
        my $bsuf = $d->{byte_access} ? 'B' : '';
        my $offset;
        if ($d->{immediate}) {
            $offset = $d->{shift_imm} ? sprintf("R%d, %s #%d", $d->{rm}, $SHIFT_NAMES[$d->{shift_type}], $d->{shift_imm}) : "R$d->{rm}";
        } else {
            $offset = "#$d->{offset12}";
        }
        my $sign = $d->{up} ? '' : '-';
        if ($d->{pre_index}) {
            my $wb = $d->{write_back} ? '!' : '';
            return "${op}${cond}${bsuf} R$d->{rd}, [R$d->{rn}, ${sign}${offset}]${wb}";
        } else {
            return "${op}${cond}${bsuf} R$d->{rd}, [R$d->{rn}], ${sign}${offset}";
        }
    } elsif ($d->{type} == INST_BLOCK_TRANSFER) {
        my $op = $d->{load} ? 'LDM' : 'STM';
        my $mode = (!$d->{pre_index} && $d->{up})  ? 'IA'
                 : ($d->{pre_index}  && $d->{up})  ? 'IB'
                 : (!$d->{pre_index} && !$d->{up}) ? 'DA'
                 :                                    'DB';
        my $wb   = $d->{write_back} ? '!' : '';
        my $regs = _disasm_reg_list($d->{register_list});
        return "${op}${cond}${mode} R$d->{rn}${wb}, {$regs}";
    } elsif ($d->{type} == INST_BRANCH) {
        my $op = $d->{link} ? 'BL' : 'B';
        return "${op}${cond} #$d->{branch_offset}";
    } elsif ($d->{type} == INST_SWI) {
        return ($d->{swi_comment} == HALT_SWI)
            ? "HLT${cond}"
            : sprintf("SWI${cond} #0x%06X", $d->{swi_comment});
    } elsif ($d->{type} == INST_COPROCESSOR) {
        return "CDP${cond} (coprocessor)";
    }
    return sprintf("UND${cond} #0x%08X", $d->{raw});
}

# ===========================================================================
# Execution — Step
# ===========================================================================

sub _capture_regs {
    my ($self) = @_;
    my %regs;
    $regs{$_} = $self->read_register($_) for 0..15;
    return \%regs;
}

# Read register as seen during execution (R15 = PC+8 due to pipeline)
sub _read_reg_for_exec {
    my ($self, $index) = @_;
    return _mask32($self->{regs}[15] + 4) if $index == 15;
    return $self->read_register($index);
}

sub _trap_undefined {
    my ($self) = @_;
    my $r15_val = $self->{regs}[15];
    $self->{regs}[26] = $r15_val;  # R14_svc
    my $r15 = $self->{regs}[15];
    $r15 = ($r15 & _bnot32(MODE_MASK)) | MODE_SVC;
    $r15 |= FLAG_I;
    $self->{regs}[15] = _mask32($r15);
    $self->set_pc(0x04);
}

sub _execute_data_processing {
    my ($self, $d) = @_;
    my $a = 0;
    $a = $self->_read_reg_for_exec($d->{rn})
        if $d->{opcode} != OP_MOV && $d->{opcode} != OP_MVN;

    my $flags = $self->get_flags();
    my ($b, $shifter_carry);

    if ($d->{immediate}) {
        ($b, $shifter_carry) = _decode_immediate($d->{imm8}, $d->{rotate});
        $shifter_carry = $flags->{c} if $d->{rotate} == 0;
    } else {
        my $rm_val = $self->_read_reg_for_exec($d->{rm});
        my $shift_amount;
        if ($d->{shift_by_reg}) {
            $shift_amount = $self->_read_reg_for_exec($d->{rs}) & 0xFF;
        } else {
            $shift_amount = $d->{shift_imm};
        }
        ($b, $shifter_carry) = _barrel_shift($rm_val, $d->{shift_type},
            $shift_amount, $flags->{c}, $d->{shift_by_reg});
    }

    my $alu = _alu_execute($d->{opcode}, $a, $b, $flags->{c}, $shifter_carry, $flags->{v});

    if ($alu->{write_result}) {
        if ($d->{rd} == 15) {
            if ($d->{s}) {
                $self->{regs}[15] = _mask32($alu->{result});
            } else {
                $self->set_pc($alu->{result} & PC_MASK);
            }
        } else {
            $self->write_register($d->{rd}, $alu->{result});
        }
    }

    if (($d->{s} && $d->{rd} != 15) || _is_test_op($d->{opcode})) {
        $self->set_flags({ n => $alu->{n}, z => $alu->{z}, c => $alu->{c}, v => $alu->{v} });
    }
}

sub _execute_load_store {
    my ($self, $d, $mem_reads, $mem_writes) = @_;
    my $offset;
    if ($d->{immediate}) {
        my $rm_val = $self->_read_reg_for_exec($d->{rm});
        if ($d->{shift_imm} != 0) {
            ($offset) = _barrel_shift($rm_val, $d->{shift_type},
                $d->{shift_imm}, $self->get_flags()->{c}, 0);
        } else {
            $offset = $rm_val;
        }
    } else {
        $offset = $d->{offset12};
    }

    my $base = $self->_read_reg_for_exec($d->{rn});
    my $addr = $d->{up} ? _mask32($base + $offset) : _mask32($base - $offset);
    my $transfer_addr = $d->{pre_index} ? $addr : $base;

    if ($d->{load}) {
        my $value;
        if ($d->{byte_access}) {
            $value = $self->read_byte($transfer_addr);
        } else {
            my $word = $self->read_word($transfer_addr);
            my $rotation = ($transfer_addr & 3) * 8;
            $word = _ror32($word, $rotation) if $rotation;
            $value = $word;
        }
        push @$mem_reads, { address => $transfer_addr, value => $value };
        if ($d->{rd} == 15) {
            $self->{regs}[15] = _mask32($value);
        } else {
            $self->write_register($d->{rd}, $value);
        }
    } else {
        my $value = $self->_read_reg_for_exec($d->{rd});
        if ($d->{byte_access}) {
            $self->write_byte($transfer_addr, $value & 0xFF);
        } else {
            $self->write_word($transfer_addr, $value);
        }
        push @$mem_writes, { address => $transfer_addr, value => $value };
    }

    if ($d->{write_back} || !$d->{pre_index}) {
        $self->write_register($d->{rn}, $addr) if $d->{rn} != 15;
    }
}

sub _execute_block_transfer {
    my ($self, $d, $mem_reads, $mem_writes) = @_;
    my $base     = $self->read_register($d->{rn});
    my $reg_list = $d->{register_list};

    my $count = 0;
    $count++ for grep { ($reg_list >> $_) & 1 } 0..15;
    return unless $count;

    my $start_addr;
    if (!$d->{pre_index} && $d->{up}) {
        $start_addr = $base;
    } elsif ($d->{pre_index} && $d->{up}) {
        $start_addr = _mask32($base + 4);
    } elsif (!$d->{pre_index} && !$d->{up}) {
        $start_addr = _mask32($base - $count * 4 + 4);
    } else {
        $start_addr = _mask32($base - $count * 4);
    }

    my $addr = $start_addr;
    for my $i (0..15) {
        next unless (($reg_list >> $i) & 1);
        if ($d->{load}) {
            my $value = $self->read_word($addr);
            push @$mem_reads, { address => $addr, value => $value };
            if ($i == 15) {
                $self->{regs}[15] = _mask32($value);
            } else {
                $self->write_register($i, $value);
            }
        } else {
            my $value = ($i == 15) ? _mask32($self->{regs}[15] + 4) : $self->read_register($i);
            $self->write_word($addr, $value);
            push @$mem_writes, { address => $addr, value => $value };
        }
        $addr = _mask32($addr + 4);
    }

    if ($d->{write_back}) {
        my $new_base = $d->{up} ? _mask32($base + $count * 4) : _mask32($base - $count * 4);
        $self->write_register($d->{rn}, $new_base);
    }
}

sub _execute_branch {
    my ($self, $d) = @_;
    my $branch_base = _mask32($self->get_pc() + 4);

    if ($d->{link}) {
        $self->write_register(14, $self->{regs}[15]);
    }

    my $target = ($branch_base + $d->{branch_offset}) & 0x3FFFFFFF;
    $self->set_pc($target & PC_MASK);
}

sub _execute_swi {
    my ($self, $d) = @_;
    if ($d->{swi_comment} == HALT_SWI) {
        $self->{halted} = 1;
    } else {
        my $r15_val = $self->{regs}[15];
        $self->{regs}[25] = $r15_val;  # R13_svc
        $self->{regs}[26] = $r15_val;  # R14_svc
        my $r15 = $self->{regs}[15];
        $r15 = ($r15 & _bnot32(MODE_MASK)) | MODE_SVC;
        $r15 |= FLAG_I;
        $self->{regs}[15] = _mask32($r15);
        $self->set_pc(0x08);
    }
}

=head2 step

  my $trace = $cpu->step();

Executes one instruction and returns a trace hashref.

=cut

sub step {
    my ($self) = @_;
    my $current_pc  = $self->get_pc();
    my $regs_before = $self->_capture_regs();
    my $flags_before = $self->get_flags();

    my $instruction = $self->read_word($current_pc);
    my $decoded     = _decode($instruction);

    my $cond_met = _evaluate_condition(
        $decoded->{condition},
        $flags_before->{n}, $flags_before->{z},
        $flags_before->{c}, $flags_before->{v},
    );

    my @mem_reads;
    my @mem_writes;

    # Advance PC (pipeline: PC+4 before execute)
    $self->set_pc(_mask32($current_pc + 4));

    if ($cond_met) {
        my $type = $decoded->{type};
        if    ($type == INST_DATA_PROCESSING) { $self->_execute_data_processing($decoded) }
        elsif ($type == INST_LOAD_STORE)      { $self->_execute_load_store($decoded, \@mem_reads, \@mem_writes) }
        elsif ($type == INST_BLOCK_TRANSFER)  { $self->_execute_block_transfer($decoded, \@mem_reads, \@mem_writes) }
        elsif ($type == INST_BRANCH)          { $self->_execute_branch($decoded) }
        elsif ($type == INST_SWI)             { $self->_execute_swi($decoded) }
        else                                  { $self->_trap_undefined() }
    }

    return {
        address       => $current_pc,
        raw           => $instruction,
        mnemonic      => disassemble($decoded),
        condition     => $COND_NAMES[$decoded->{condition}] // '??',
        condition_met => $cond_met ? 1 : 0,
        regs_before   => $regs_before,
        regs_after    => $self->_capture_regs(),
        flags_before  => $flags_before,
        flags_after   => $self->get_flags(),
        memory_reads  => \@mem_reads,
        memory_writes => \@mem_writes,
    };
}

=head2 run

  my $traces = $cpu->run($max_steps);

Runs until halted or max_steps reached. Returns arrayref of traces.

=cut

sub run {
    my ($self, $max_steps) = @_;
    $max_steps //= 100_000;
    my @traces;
    while (!$self->{halted} && @traces < $max_steps) {
        push @traces, $self->step();
    }
    return \@traces;
}

# ===========================================================================
# Encoding Helpers
# ===========================================================================

=head2 encode_mov_imm

  my $inst = CodingAdventures::ARM1Simulator::encode_mov_imm($cond, $rd, $imm8);

Creates a MOV Rd, #imm8 instruction word.

=cut

sub encode_mov_imm {
    shift if !ref($_[0]) && defined $_[0] && index($_[0], '::') >= 0;  # discard class invocant
    my ($condition, $rd, $imm8) = @_;
    my $inst = ($condition << 28) | 0x03A00000;
    $inst |= ($rd << 12) | $imm8;
    return _mask32($inst);
}

=head2 encode_alu_reg

  my $inst = CodingAdventures::ARM1Simulator::encode_alu_reg($cond, $opcode, $s, $rd, $rn, $rm);

Creates a data processing instruction with register operand.

=cut

sub encode_alu_reg {
    shift if !ref($_[0]) && defined $_[0] && index($_[0], '::') >= 0;  # discard class invocant
    my ($condition, $opcode, $s, $rd, $rn, $rm) = @_;
    my $s_bit = $s ? 1 : 0;
    my $inst = ($condition << 28) | ($opcode << 21) | ($s_bit << 20);
    $inst |= ($rn << 16) | ($rd << 12) | $rm;
    return _mask32($inst);
}

=head2 encode_alu_reg_shift

  my $inst = CodingAdventures::ARM1Simulator::encode_alu_reg_shift($cond, $opcode, $s, $rd, $rn, $rm, $shift_type, $shift_imm);

Creates a data processing instruction with shifted register operand.

=cut

sub encode_alu_reg_shift {
    shift if !ref($_[0]) && defined $_[0] && index($_[0], '::') >= 0;  # discard class invocant
    my ($condition, $opcode, $s, $rd, $rn, $rm, $shift_type, $shift_imm) = @_;
    my $s_bit = $s ? 1 : 0;
    my $inst = ($condition << 28) | ($opcode << 21) | ($s_bit << 20);
    $inst |= ($rn << 16) | ($rd << 12) | ($shift_imm << 7) | ($shift_type << 5) | $rm;
    return _mask32($inst);
}

=head2 encode_branch

  my $inst = CodingAdventures::ARM1Simulator::encode_branch($cond, $link, $offset_bytes);

Creates a B or BL instruction with the given byte offset.

=cut

sub encode_branch {
    shift if !ref($_[0]) && defined $_[0] && index($_[0], '::') >= 0;  # discard class invocant
    my ($condition, $link, $offset) = @_;
    my $inst = ($condition << 28) | 0x0A000000;
    $inst |= 0x01000000 if $link;
    my $encoded = int($offset / 4) & 0x00FFFFFF;
    return _mask32($inst | $encoded);
}

=head2 encode_halt

  my $inst = CodingAdventures::ARM1Simulator::encode_halt();

Creates the pseudo-halt instruction (SWI 0x123456).

=cut

sub encode_halt {
    shift if !ref($_[0]) && defined $_[0] && index($_[0], '::') >= 0;  # discard class invocant
    return _mask32((COND_AL << 28) | 0x0F000000 | HALT_SWI);
}

=head2 encode_ldr

  my $inst = CodingAdventures::ARM1Simulator::encode_ldr($cond, $rd, $rn, $offset, $pre_index);

=cut

sub encode_ldr {
    shift if !ref($_[0]) && defined $_[0] && index($_[0], '::') >= 0;  # discard class invocant
    my ($condition, $rd, $rn, $offset, $pre_index) = @_;
    my $inst = ($condition << 28) | 0x04100000;
    $inst |= ($rd << 12) | ($rn << 16);
    $inst |= (1 << 24) if $pre_index;
    if ($offset >= 0) {
        $inst |= (1 << 23) | ($offset & 0xFFF);
    } else {
        $inst |= ((-$offset) & 0xFFF);
    }
    return _mask32($inst);
}

=head2 encode_str

  my $inst = CodingAdventures::ARM1Simulator::encode_str($cond, $rd, $rn, $offset, $pre_index);

=cut

sub encode_str {
    shift if !ref($_[0]) && defined $_[0] && index($_[0], '::') >= 0;  # discard class invocant
    my ($condition, $rd, $rn, $offset, $pre_index) = @_;
    my $inst = ($condition << 28) | 0x04000000;
    $inst |= ($rd << 12) | ($rn << 16);
    $inst |= (1 << 24) if $pre_index;
    if ($offset >= 0) {
        $inst |= (1 << 23) | ($offset & 0xFFF);
    } else {
        $inst |= ((-$offset) & 0xFFF);
    }
    return _mask32($inst);
}

=head2 encode_ldm

  my $inst = CodingAdventures::ARM1Simulator::encode_ldm($cond, $rn, $reg_list, $write_back, $mode);

Creates an LDM instruction. $mode is "IA", "IB", "DA", or "DB".

=cut

sub encode_ldm {
    shift if !ref($_[0]) && defined $_[0] && index($_[0], '::') >= 0;  # discard class invocant
    my ($condition, $rn, $reg_list, $write_back, $bt_mode) = @_;
    my $inst = ($condition << 28) | 0x08100000;
    $inst |= ($rn << 16) | $reg_list;
    $inst |= (1 << 21) if $write_back;
    if    (defined $bt_mode && $bt_mode eq 'IA') { $inst |= (1 << 23) }
    elsif (defined $bt_mode && $bt_mode eq 'IB') { $inst |= (1 << 24) | (1 << 23) }
    elsif (defined $bt_mode && $bt_mode eq 'DB') { $inst |= (1 << 24) }
    # DA or undef: no P or U bits
    return _mask32($inst);
}

=head2 encode_stm

  my $inst = CodingAdventures::ARM1Simulator::encode_stm($cond, $rn, $reg_list, $write_back, $mode);

=cut

sub encode_stm {
    shift if !ref($_[0]) && defined $_[0] && index($_[0], '::') >= 0;  # discard class invocant
    my ($condition, $rn, $reg_list, $write_back, $bt_mode) = @_;
    # Build STM directly (same as encode_ldm but with L=0 instead of L=1)
    my $inst = ($condition << 28) | 0x08000000;  # bits 27:25=100, L=0
    $inst |= ($rn << 16) | $reg_list;
    $inst |= (1 << 21) if $write_back;
    if    (defined $bt_mode && $bt_mode eq 'IA') { $inst |= (1 << 23) }
    elsif (defined $bt_mode && $bt_mode eq 'IB') { $inst |= (1 << 24) | (1 << 23) }
    elsif (defined $bt_mode && $bt_mode eq 'DB') { $inst |= (1 << 24) }
    # DA or undef: no P or U bits
    return _mask32($inst);
}

1;
