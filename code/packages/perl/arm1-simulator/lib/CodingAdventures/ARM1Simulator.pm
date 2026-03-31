package CodingAdventures::ARM1Simulator;

# =============================================================================
# CodingAdventures::ARM1Simulator — ARM1 (ARMv1) Behavioral Simulator
# =============================================================================
#
# The ARM1 was designed by Sophie Wilson and Steve Furber at Acorn Computers
# in Cambridge, UK. First silicon powered on April 26, 1985 — and worked
# correctly on the very first attempt. The ARM1 had just 25,000 transistors
# and a 26-bit address space (64 MiB). Its accidentally low power consumption
# (~0.1W) later made the ARM architecture dominant in mobile computing.
#
# # Instruction Format
#
# Every ARM instruction is 32 bits wide:
#
#   31:28  27:26  25  24:21     20  19:16  15:12  11:0
#   Cond   Type   I   Opcode    S   Rn     Rd     Operand2
#
# # Register File (26-bit architecture, 27 physical registers)
#
#   R0-R15:       base set (R15 = PC + flags + mode)
#   R16-R22 (FIQ): banked R8-R14 for FIQ mode
#   R23-R24 (IRQ): banked R13-R14 for IRQ mode
#   R25-R26 (SVC): banked R13-R14 for SVC mode
#
# # R15 Layout
#
#   Bit 31: N (Negative)       Bit 27: I (IRQ disable)
#   Bit 30: Z (Zero)           Bit 26: F (FIQ disable)
#   Bit 29: C (Carry)          Bits 25:2: PC (26-bit address, word-aligned)
#   Bit 28: V (Overflow)       Bits 1:0: Processor Mode

use strict;
use warnings;

our $VERSION = '0.01';

# =============================================================================
# Constants
# =============================================================================

# Processor modes
use constant MODE_USR => 0;
use constant MODE_FIQ => 1;
use constant MODE_IRQ => 2;
use constant MODE_SVC => 3;

# Condition codes
use constant COND_EQ  => 0x0;
use constant COND_NE  => 0x1;
use constant COND_CS  => 0x2;
use constant COND_CC  => 0x3;
use constant COND_MI  => 0x4;
use constant COND_PL  => 0x5;
use constant COND_VS  => 0x6;
use constant COND_VC  => 0x7;
use constant COND_HI  => 0x8;
use constant COND_LS  => 0x9;
use constant COND_GE  => 0xA;
use constant COND_LT  => 0xB;
use constant COND_GT  => 0xC;
use constant COND_LE  => 0xD;
use constant COND_AL  => 0xE;
use constant COND_NV  => 0xF;

# ALU opcodes
use constant OP_AND => 0x0;
use constant OP_EOR => 0x1;
use constant OP_SUB => 0x2;
use constant OP_RSB => 0x3;
use constant OP_ADD => 0x4;
use constant OP_ADC => 0x5;
use constant OP_SBC => 0x6;
use constant OP_RSC => 0x7;
use constant OP_TST => 0x8;
use constant OP_TEQ => 0x9;
use constant OP_CMP => 0xA;
use constant OP_CMN => 0xB;
use constant OP_ORR => 0xC;
use constant OP_MOV => 0xD;
use constant OP_BIC => 0xE;
use constant OP_MVN => 0xF;

# Shift types
use constant SHIFT_LSL => 0;
use constant SHIFT_LSR => 1;
use constant SHIFT_ASR => 2;
use constant SHIFT_ROR => 3;

# R15 bit masks
use constant FLAG_N    => 0x80000000;
use constant FLAG_Z    => 0x40000000;
use constant FLAG_C    => 0x20000000;
use constant FLAG_V    => 0x10000000;
use constant FLAG_I    => 0x08000000;
use constant FLAG_F    => 0x04000000;
use constant PC_MASK   => 0x03FFFFFC;
use constant MODE_MASK => 0x3;
use constant MASK32    => 0xFFFFFFFF;
use constant HALT_SWI  => 0x123456;

# Instruction types
use constant INST_DATA_PROCESSING => 0;
use constant INST_LOAD_STORE      => 1;
use constant INST_BLOCK_TRANSFER  => 2;
use constant INST_BRANCH          => 3;
use constant INST_SWI             => 4;
use constant INST_COPROCESSOR     => 5;
use constant INST_UNDEFINED       => 6;

my @OP_NAMES = qw(AND EOR SUB RSB ADD ADC SBC RSC TST TEQ CMP CMN ORR MOV BIC MVN);
my @SHIFT_NAMES = qw(LSL LSR ASR ROR);
my @COND_STRINGS = ('EQ','NE','CS','CC','MI','PL','VS','VC','HI','LS','GE','LT','GT','LE','','NV');

# =============================================================================
# Constructor
# =============================================================================

sub new {
    my ($class, $memory_size) = @_;
    $memory_size //= 1024 * 1024;
    $memory_size = 1024 * 1024 if $memory_size <= 0;

    my $self = bless {
        regs     => [(0) x 27],
        memory   => [(0) x $memory_size],
        mem_size => $memory_size,
        halted   => 0,
    }, $class;

    return $self->reset();
}

sub reset {
    my ($self) = @_;
    for my $i (0..26) { $self->{regs}[$i] = 0; }
    # SVC mode (bits 1:0 = 11), IRQ disabled (bit 27), FIQ disabled (bit 26)
    $self->{regs}[15] = FLAG_I | FLAG_F | MODE_SVC;
    $self->{halted}   = 0;
    return $self;
}

# =============================================================================
# Register Access
# =============================================================================

# Maps logical register index (0-15) to physical register index.
sub _physical_reg {
    my ($self, $index) = @_;
    my $mode = $self->{regs}[15] & MODE_MASK;

    if ($mode == MODE_FIQ && $index >= 8 && $index <= 14) {
        return 16 + ($index - 8);
    }
    elsif ($mode == MODE_IRQ && $index >= 13 && $index <= 14) {
        return 23 + ($index - 13);
    }
    elsif ($mode == MODE_SVC && $index >= 13 && $index <= 14) {
        return 25 + ($index - 13);
    }
    return $index;
}

sub read_register {
    my ($self, $n) = @_;
    return $self->{regs}[ $self->_physical_reg($n) ];
}

sub write_register {
    my ($self, $n, $value) = @_;
    $self->{regs}[ $self->_physical_reg($n) ] = $value & MASK32;
}

sub get_pc {
    my ($self) = @_;
    return $self->{regs}[15] & PC_MASK;
}

sub set_pc {
    my ($self, $addr) = @_;
    my $r15 = $self->{regs}[15];
    $self->{regs}[15] = (($r15 & (~PC_MASK & MASK32)) | ($addr & PC_MASK)) & MASK32;
}

sub get_flags {
    my ($self) = @_;
    my $r15 = $self->{regs}[15];
    return {
        n => (($r15 & FLAG_N) != 0) ? 1 : 0,
        z => (($r15 & FLAG_Z) != 0) ? 1 : 0,
        c => (($r15 & FLAG_C) != 0) ? 1 : 0,
        v => (($r15 & FLAG_V) != 0) ? 1 : 0,
    };
}

sub set_flags {
    my ($self, $n, $z, $c, $v) = @_;
    my $r15 = $self->{regs}[15];
    my $mask = ~(FLAG_N | FLAG_Z | FLAG_C | FLAG_V) & MASK32;
    $r15 &= $mask;
    $r15 |= FLAG_N if $n;
    $r15 |= FLAG_Z if $z;
    $r15 |= FLAG_C if $c;
    $r15 |= FLAG_V if $v;
    $self->{regs}[15] = $r15 & MASK32;
}

sub get_mode {
    my ($self) = @_;
    return $self->{regs}[15] & MODE_MASK;
}

# =============================================================================
# Memory Access
# =============================================================================

sub read_word {
    my ($self, $addr) = @_;
    $addr &= PC_MASK;
    my $a = $addr & (~3 & MASK32);
    return 0 if $a + 3 >= $self->{mem_size};
    my $b0 = $self->{memory}[$a]   // 0;
    my $b1 = $self->{memory}[$a+1] // 0;
    my $b2 = $self->{memory}[$a+2] // 0;
    my $b3 = $self->{memory}[$a+3] // 0;
    return ($b0 | ($b1 << 8) | ($b2 << 16) | ($b3 << 24)) & MASK32;
}

sub write_word {
    my ($self, $addr, $value) = @_;
    $addr &= PC_MASK;
    my $a = $addr & (~3 & MASK32);
    return if $a + 3 >= $self->{mem_size};
    $value &= MASK32;
    $self->{memory}[$a]   = $value & 0xFF;
    $self->{memory}[$a+1] = ($value >> 8)  & 0xFF;
    $self->{memory}[$a+2] = ($value >> 16) & 0xFF;
    $self->{memory}[$a+3] = ($value >> 24) & 0xFF;
}

sub read_byte {
    my ($self, $addr) = @_;
    $addr &= MASK32;
    return 0 if $addr >= $self->{mem_size};
    return $self->{memory}[$addr] // 0;
}

sub write_byte {
    my ($self, $addr, $value) = @_;
    $addr &= MASK32;
    return if $addr >= $self->{mem_size};
    $self->{memory}[$addr] = $value & 0xFF;
}

sub load_instructions {
    my ($self, $instructions) = @_;
    my $addr = 0;
    for my $inst (@$instructions) {
        $self->write_word($addr, $inst);
        $addr += 4;
    }
}

# =============================================================================
# Condition Evaluation
# =============================================================================

sub evaluate_condition {
    my ($self, $cond, $flags) = @_;
    my ($n, $z, $c, $v) = @{$flags}{qw(n z c v)};

    return $z             if $cond == COND_EQ;
    return !$z            if $cond == COND_NE;
    return $c             if $cond == COND_CS;
    return !$c            if $cond == COND_CC;
    return $n             if $cond == COND_MI;
    return !$n            if $cond == COND_PL;
    return $v             if $cond == COND_VS;
    return !$v            if $cond == COND_VC;
    return ($c && !$z)    if $cond == COND_HI;
    return (!$c || $z)    if $cond == COND_LS;
    return ($n == $v)     if $cond == COND_GE;
    return ($n != $v)     if $cond == COND_LT;
    return (!$z && ($n == $v))  if $cond == COND_GT;
    return ($z  || ($n != $v)) if $cond == COND_LE;
    return 1              if $cond == COND_AL;
    return 0              if $cond == COND_NV;
    return 0;
}

# =============================================================================
# Barrel Shifter
# =============================================================================

# Logical Shift Left
sub _shift_lsl {
    my ($value, $amount, $carry_in) = @_;
    return ($value, $carry_in) if $amount == 0;
    if ($amount >= 32) {
        if ($amount == 32) { return (0, ($value & 1) ? 1 : 0); }
        return (0, 0);
    }
    my $carry = (($value >> (32 - $amount)) & 1) ? 1 : 0;
    my $result = ($value << $amount) & MASK32;
    return ($result, $carry);
}

# Logical Shift Right
sub _shift_lsr {
    my ($value, $amount, $carry_in, $by_register) = @_;
    if ($amount == 0) {
        # Immediate LSR #0 = LSR #32
        return (0, ($value >> 31) & 1) unless $by_register;
        return ($value, $carry_in);
    }
    if ($amount >= 32) {
        return (0, ($value >> 31) & 1) if $amount == 32;
        return (0, 0);
    }
    my $carry = (($value >> ($amount - 1)) & 1) ? 1 : 0;
    return ($value >> $amount, $carry);
}

# Arithmetic Shift Right (sign-extending)
sub _shift_asr {
    my ($value, $amount, $carry_in, $by_register) = @_;
    if ($amount == 0) {
        unless ($by_register) {
            # Immediate ASR #0 = ASR #32
            return (($value >> 31) ? (MASK32, 1) : (0, 0));
        }
        return ($value, $carry_in);
    }
    if ($amount >= 32) {
        return (($value >> 31) ? (MASK32, 1) : (0, 0));
    }
    my $carry = (($value >> ($amount - 1)) & 1) ? 1 : 0;
    if ($value >> 31) {
        my $fill   = (MASK32 << (32 - $amount)) & MASK32;
        my $result = (($value >> $amount) | $fill) & MASK32;
        return ($result, $carry);
    }
    return ($value >> $amount, $carry);
}

# Rotate Right (and RRX)
sub _shift_ror {
    my ($value, $amount, $carry_in, $by_register) = @_;
    if ($amount == 0) {
        unless ($by_register) {
            # RRX: rotate right through carry
            my $carry  = $value & 1;
            my $result = ($value >> 1) | ($carry_in ? 0x80000000 : 0);
            return ($result & MASK32, $carry);
        }
        return ($value, $carry_in);
    }
    $amount &= 31;
    if ($amount == 0) {
        return ($value, ($value >> 31) & 1);
    }
    my $result = (($value >> $amount) | ($value << (32 - $amount))) & MASK32;
    my $carry  = ($result >> 31) & 1;
    return ($result, $carry);
}

# Main barrel shift dispatcher.
# Returns ($result, $carry_out).
sub barrel_shift {
    my ($self, $value, $shift_type, $amount, $carry_in, $by_register) = @_;

    # Register shift by 0 = no change
    if ($amount == 0 && $by_register) {
        return ($value, $carry_in);
    }

    if ($shift_type == SHIFT_LSL) { return _shift_lsl($value, $amount, $carry_in); }
    if ($shift_type == SHIFT_LSR) { return _shift_lsr($value, $amount, $carry_in, $by_register); }
    if ($shift_type == SHIFT_ASR) { return _shift_asr($value, $amount, $carry_in, $by_register); }
    if ($shift_type == SHIFT_ROR) { return _shift_ror($value, $amount, $carry_in, $by_register); }

    return ($value, $carry_in);
}

# Decodes rotated 8-bit immediate. Returns ($value, $carry_out).
sub decode_immediate {
    my ($self, $imm8, $rotate_field) = @_;
    return ($imm8, 0) if $rotate_field == 0;
    my $rotate = $rotate_field * 2;
    my $value  = (($imm8 >> $rotate) | ($imm8 << (32 - $rotate))) & MASK32;
    my $carry  = ($value >> 31) & 1;
    return ($value, $carry);
}

# =============================================================================
# ALU
# =============================================================================

# 32-bit addition with carry. Returns ($result, $carry, $overflow).
sub _add32 {
    my ($a, $b, $carry_in) = @_;
    my $cin = $carry_in ? 1 : 0;
    # Use 64-bit Perl integer arithmetic
    my $sum    = $a + $b + $cin;
    my $result = $sum & MASK32;
    my $carry  = (($sum >> 32) != 0) ? 1 : 0;
    # Overflow: inputs same sign but result differs
    my $overflow = ((($a ^ $result) & ($b ^ $result)) >> 31) & 1;
    return ($result, $carry, $overflow);
}

sub _test_op {
    my ($opcode) = @_;
    return ($opcode >= OP_TST && $opcode <= OP_CMN) ? 1 : 0;
}

# Executes one ALU operation. Returns a hashref.
sub alu_execute {
    my ($self, $opcode, $a, $b, $carry_in, $shifter_carry, $old_v) = @_;
    my ($result, $carry, $overflow);
    my $write_result = _test_op($opcode) ? 0 : 1;

    if ($opcode == OP_AND || $opcode == OP_TST) {
        $result = $a & $b;
        ($carry, $overflow) = ($shifter_carry, $old_v);
    }
    elsif ($opcode == OP_EOR || $opcode == OP_TEQ) {
        $result = $a ^ $b;
        ($carry, $overflow) = ($shifter_carry, $old_v);
    }
    elsif ($opcode == OP_ORR) {
        $result = $a | $b;
        ($carry, $overflow) = ($shifter_carry, $old_v);
    }
    elsif ($opcode == OP_MOV) {
        $result = $b;
        ($carry, $overflow) = ($shifter_carry, $old_v);
    }
    elsif ($opcode == OP_BIC) {
        $result = $a & (~$b & MASK32);
        ($carry, $overflow) = ($shifter_carry, $old_v);
    }
    elsif ($opcode == OP_MVN) {
        $result = ~$b & MASK32;
        ($carry, $overflow) = ($shifter_carry, $old_v);
    }
    elsif ($opcode == OP_ADD || $opcode == OP_CMN) {
        ($result, $carry, $overflow) = _add32($a, $b, 0);
    }
    elsif ($opcode == OP_ADC) {
        ($result, $carry, $overflow) = _add32($a, $b, $carry_in);
    }
    elsif ($opcode == OP_SUB || $opcode == OP_CMP) {
        ($result, $carry, $overflow) = _add32($a, ~$b & MASK32, 1);
    }
    elsif ($opcode == OP_SBC) {
        ($result, $carry, $overflow) = _add32($a, ~$b & MASK32, $carry_in);
    }
    elsif ($opcode == OP_RSB) {
        ($result, $carry, $overflow) = _add32($b, ~$a & MASK32, 1);
    }
    elsif ($opcode == OP_RSC) {
        ($result, $carry, $overflow) = _add32($b, ~$a & MASK32, $carry_in);
    }
    else {
        $result   = 0;
        ($carry, $overflow) = ($shifter_carry, $old_v);
    }

    $result &= MASK32;
    return {
        result       => $result,
        n            => ($result >> 31) & 1,
        z            => ($result == 0) ? 1 : 0,
        c            => $carry ? 1 : 0,
        v            => $overflow ? 1 : 0,
        write_result => $write_result,
    };
}

# =============================================================================
# Decoder
# =============================================================================

sub _decode_data_processing {
    my ($d, $inst) = @_;
    my $is_imm = (($inst >> 25) & 1) == 1;
    $d->{immediate}  = $is_imm;
    $d->{opcode}     = ($inst >> 21) & 0xF;
    $d->{s}          = (($inst >> 20) & 1) == 1;
    $d->{rn}         = ($inst >> 16) & 0xF;
    $d->{rd}         = ($inst >> 12) & 0xF;
    if ($is_imm) {
        $d->{imm8}   = $inst & 0xFF;
        $d->{rotate} = ($inst >> 8) & 0xF;
    } else {
        $d->{shift_by_reg} = (($inst >> 4) & 1) == 1;
        $d->{rm}            = $inst & 0xF;
        $d->{shift_type}    = ($inst >> 5) & 0x3;
        if ($d->{shift_by_reg}) {
            $d->{rs} = ($inst >> 8) & 0xF;
        } else {
            $d->{shift_imm} = ($inst >> 7) & 0x1F;
        }
    }
    return $d;
}

sub _decode_load_store {
    my ($d, $inst) = @_;
    $d->{immediate}  = (($inst >> 25) & 1) == 1;
    $d->{pre_index}  = (($inst >> 24) & 1) == 1;
    $d->{up}         = (($inst >> 23) & 1) == 1;
    $d->{byte}       = (($inst >> 22) & 1) == 1;
    $d->{write_back} = (($inst >> 21) & 1) == 1;
    $d->{load}       = (($inst >> 20) & 1) == 1;
    $d->{rn}         = ($inst >> 16) & 0xF;
    $d->{rd}         = ($inst >> 12) & 0xF;
    $d->{rm}         = $inst & 0xF;
    $d->{shift_type} = ($inst >> 5) & 0x3;
    $d->{shift_imm}  = ($inst >> 7) & 0x1F;
    $d->{offset12}   = $inst & 0xFFF;
    return $d;
}

sub _decode_block_transfer {
    my ($d, $inst) = @_;
    $d->{pre_index}     = (($inst >> 24) & 1) == 1;
    $d->{up}            = (($inst >> 23) & 1) == 1;
    $d->{force_user}    = (($inst >> 22) & 1) == 1;
    $d->{write_back}    = (($inst >> 21) & 1) == 1;
    $d->{load}          = (($inst >> 20) & 1) == 1;
    $d->{rn}            = ($inst >> 16) & 0xF;
    $d->{register_list} = $inst & 0xFFFF;
    return $d;
}

sub _decode_branch {
    my ($d, $inst) = @_;
    $d->{link}   = (($inst >> 24) & 1) == 1;
    my $offset   = $inst & 0x00FFFFFF;
    # Sign-extend from 24 bits
    $offset |= 0xFF000000 if ($offset >> 23) != 0;
    # Treat as signed 32-bit
    $offset -= (1 << 32) if $offset >= 0x80000000;
    $d->{branch_offset} = $offset * 4;
    return $d;
}

sub decode {
    my ($self, $instruction) = @_;
    my $d = {
        raw           => $instruction,
        condition     => ($instruction >> 28) & 0xF,
        type          => INST_UNDEFINED,
        opcode        => 0, s => 0, rn => 0, rd => 0,
        immediate     => 0, imm8 => 0, rotate => 0,
        rm => 0, shift_type => 0, shift_by_reg => 0, shift_imm => 0, rs => 0,
        load => 0, byte => 0, pre_index => 0, up => 0, write_back => 0, offset12 => 0,
        register_list => 0, force_user => 0,
        link => 0, branch_offset => 0,
        swi_comment => 0,
    };

    my $bits2726 = ($instruction >> 26) & 0x3;
    my $bit25    = ($instruction >> 25) & 0x1;

    if ($bits2726 == 0) {
        $d->{type} = INST_DATA_PROCESSING;
        return _decode_data_processing($d, $instruction);
    }
    elsif ($bits2726 == 1) {
        $d->{type} = INST_LOAD_STORE;
        return _decode_load_store($d, $instruction);
    }
    elsif ($bits2726 == 2 && $bit25 == 0) {
        $d->{type} = INST_BLOCK_TRANSFER;
        return _decode_block_transfer($d, $instruction);
    }
    elsif ($bits2726 == 2 && $bit25 == 1) {
        $d->{type} = INST_BRANCH;
        return _decode_branch($d, $instruction);
    }
    elsif ($bits2726 == 3) {
        if ((($instruction >> 24) & 0xF) == 0xF) {
            $d->{type}        = INST_SWI;
            $d->{swi_comment} = $instruction & 0x00FFFFFF;
        } else {
            $d->{type} = INST_COPROCESSOR;
        }
    }
    return $d;
}

# =============================================================================
# Disassembly
# =============================================================================

sub _disasm_reg_list {
    my ($list) = @_;
    my @regs;
    for my $i (0..15) {
        if (($list >> $i) & 1) {
            push @regs, ($i == 15 ? 'PC' : $i == 14 ? 'LR' : $i == 13 ? 'SP' : "R$i");
        }
    }
    return join(', ', @regs);
}

sub _disasm_operand2 {
    my ($self, $d) = @_;
    if ($d->{immediate}) {
        my ($val) = $self->decode_immediate($d->{imm8}, $d->{rotate});
        return "#$val";
    }
    if (!$d->{shift_by_reg} && $d->{shift_imm} == 0 && $d->{shift_type} == SHIFT_LSL) {
        return "R$d->{rm}";
    }
    if ($d->{shift_by_reg}) {
        return "R$d->{rm}, $SHIFT_NAMES[$d->{shift_type}] R$d->{rs}";
    }
    my $amount = $d->{shift_imm};
    my $is_rrx = 0;
    if (($d->{shift_type} == SHIFT_LSR || $d->{shift_type} == SHIFT_ASR) && $amount == 0) {
        $amount = 32;
    }
    elsif ($d->{shift_type} == SHIFT_ROR && $amount == 0) {
        $is_rrx = 1;
    }
    return "R$d->{rm}, RRX" if $is_rrx;
    return "R$d->{rm}, $SHIFT_NAMES[$d->{shift_type}] #$amount";
}

sub disassemble {
    my ($self, $d) = @_;
    my $cond = $COND_STRINGS[$d->{condition}] // '??';

    if ($d->{type} == INST_DATA_PROCESSING) {
        my $op  = $OP_NAMES[$d->{opcode}] // '???';
        my $suf = ($d->{s} && !_test_op($d->{opcode})) ? 'S' : '';
        my $op2 = $self->_disasm_operand2($d);
        if ($d->{opcode} == OP_MOV || $d->{opcode} == OP_MVN) {
            return "${op}${cond}${suf} R$d->{rd}, $op2";
        }
        elsif (_test_op($d->{opcode})) {
            return "${op}${cond} R$d->{rn}, $op2";
        }
        return "${op}${cond}${suf} R$d->{rd}, R$d->{rn}, $op2";
    }
    elsif ($d->{type} == INST_LOAD_STORE) {
        my $op   = $d->{load} ? 'LDR' : 'STR';
        my $bsuf = $d->{byte} ? 'B' : '';
        my $sign = $d->{up} ? '' : '-';
        my $offset;
        if ($d->{immediate}) {
            $offset = ($d->{shift_imm} != 0)
                ? "R$d->{rm}, $SHIFT_NAMES[$d->{shift_type}] #$d->{shift_imm}"
                : "R$d->{rm}";
        } else {
            $offset = "#$d->{offset12}";
        }
        if ($d->{pre_index}) {
            my $wb = $d->{write_back} ? '!' : '';
            return "${op}${cond}${bsuf} R$d->{rd}, [R$d->{rn}, ${sign}${offset}]${wb}";
        }
        return "${op}${cond}${bsuf} R$d->{rd}, [R$d->{rn}], ${sign}${offset}";
    }
    elsif ($d->{type} == INST_BLOCK_TRANSFER) {
        my $op = $d->{load} ? 'LDM' : 'STM';
        my $bt_mode;
        if    (!$d->{pre_index} &&  $d->{up}) { $bt_mode = 'IA'; }
        elsif ( $d->{pre_index} &&  $d->{up}) { $bt_mode = 'IB'; }
        elsif (!$d->{pre_index} && !$d->{up}) { $bt_mode = 'DA'; }
        else                                   { $bt_mode = 'DB'; }
        my $wb   = $d->{write_back} ? '!' : '';
        my $regs = _disasm_reg_list($d->{register_list});
        return "${op}${cond}${bt_mode} R$d->{rn}${wb}, {${regs}}";
    }
    elsif ($d->{type} == INST_BRANCH) {
        my $op = $d->{link} ? 'BL' : 'B';
        return "${op}${cond} #$d->{branch_offset}";
    }
    elsif ($d->{type} == INST_SWI) {
        return "HLT${cond}" if $d->{swi_comment} == HALT_SWI;
        return sprintf("SWI%s #0x%X", $cond, $d->{swi_comment});
    }
    elsif ($d->{type} == INST_COPROCESSOR) {
        return "CDP${cond} (coprocessor)";
    }
    return sprintf("UND%s #0x%08X", $cond, $d->{raw});
}

# =============================================================================
# Execution Helpers
# =============================================================================

# Read register as seen during execution (R15 = PC+8 due to pipeline).
# step() has already advanced PC by 4, so add 4 more.
sub _read_reg_exec {
    my ($self, $n) = @_;
    return ($self->{regs}[15] + 4) & MASK32 if $n == 15;
    return $self->read_register($n);
}

# =============================================================================
# Data Processing
# =============================================================================

sub _exec_data_processing {
    my ($self, $d) = @_;

    my $a = ($d->{opcode} != OP_MOV && $d->{opcode} != OP_MVN)
            ? $self->_read_reg_exec($d->{rn}) : 0;
    my $flags = $self->get_flags();
    my ($b, $shifter_carry);

    if ($d->{immediate}) {
        ($b, $shifter_carry) = $self->decode_immediate($d->{imm8}, $d->{rotate});
        $shifter_carry = $flags->{c} if $d->{rotate} == 0;
    } else {
        my $rm_val = $self->_read_reg_exec($d->{rm});
        my $amount;
        if ($d->{shift_by_reg}) {
            $amount = $self->_read_reg_exec($d->{rs}) & 0xFF;
        } else {
            $amount = $d->{shift_imm};
        }
        ($b, $shifter_carry) = $self->barrel_shift($rm_val, $d->{shift_type}, $amount, $flags->{c}, $d->{shift_by_reg});
    }

    my $alu = $self->alu_execute($d->{opcode}, $a, $b, $flags->{c}, $shifter_carry, $flags->{v});

    if ($alu->{write_result}) {
        if ($d->{rd} == 15) {
            if ($d->{s}) {
                # MOVS PC: restore entire R15
                $self->{regs}[15] = $alu->{result} & MASK32;
            } else {
                $self->set_pc($alu->{result} & PC_MASK);
            }
        } else {
            $self->write_register($d->{rd}, $alu->{result});
        }
    }

    if ($d->{s} && $d->{rd} != 15) {
        $self->set_flags($alu->{n}, $alu->{z}, $alu->{c}, $alu->{v});
    }
    if (_test_op($d->{opcode})) {
        $self->set_flags($alu->{n}, $alu->{z}, $alu->{c}, $alu->{v});
    }
}

# =============================================================================
# Load/Store
# =============================================================================

sub _exec_load_store {
    my ($self, $d) = @_;
    my $base = $self->_read_reg_exec($d->{rn});
    my $offset;

    if ($d->{immediate}) {
        my $rm_val = $self->_read_reg_exec($d->{rm});
        if ($d->{shift_imm} != 0) {
            ($offset) = $self->barrel_shift($rm_val, $d->{shift_type}, $d->{shift_imm}, $self->get_flags()->{c}, 0);
        } else {
            $offset = $rm_val;
        }
    } else {
        $offset = $d->{offset12};
    }

    my $addr = $d->{up}
        ? ($base + $offset) & MASK32
        : ($base - $offset) & MASK32;

    my $transfer_addr = $d->{pre_index} ? $addr : $base;
    my (@reads, @writes);

    if ($d->{load}) {
        my $value;
        if ($d->{byte}) {
            $value = $self->read_byte($transfer_addr);
        } else {
            $value = $self->read_word($transfer_addr);
            my $rotation = ($transfer_addr & 3) * 8;
            if ($rotation) {
                $value = (($value >> $rotation) | ($value << (32 - $rotation))) & MASK32;
            }
        }
        push @reads, { address => $transfer_addr, value => $value };
        if ($d->{rd} == 15) {
            $self->{regs}[15] = $value & MASK32;
        } else {
            $self->write_register($d->{rd}, $value);
        }
    } else {
        my $value = $self->_read_reg_exec($d->{rd});
        if ($d->{byte}) {
            $self->write_byte($transfer_addr, $value & 0xFF);
        } else {
            $self->write_word($transfer_addr, $value);
        }
        push @writes, { address => $transfer_addr, value => $value };
    }

    # Write-back
    if ($d->{write_back} || !$d->{pre_index}) {
        $self->write_register($d->{rn}, $addr) if $d->{rn} != 15;
    }

    return (\@reads, \@writes);
}

# =============================================================================
# Block Transfer
# =============================================================================

sub _exec_block_transfer {
    my ($self, $d) = @_;
    my $base = $self->read_register($d->{rn});
    my $list = $d->{register_list};
    my (@reads, @writes);

    my $count = 0;
    for my $i (0..15) { $count++ if ($list >> $i) & 1; }
    return (\@reads, \@writes) if $count == 0;

    my $start_addr;
    if    (!$d->{pre_index} &&  $d->{up}) { $start_addr = $base; }
    elsif ( $d->{pre_index} &&  $d->{up}) { $start_addr = ($base + 4) & MASK32; }
    elsif (!$d->{pre_index} && !$d->{up}) { $start_addr = ($base - $count * 4 + 4) & MASK32; }
    else                                   { $start_addr = ($base - $count * 4) & MASK32; }

    my $addr = $start_addr;
    for my $i (0..15) {
        next unless ($list >> $i) & 1;
        if ($d->{load}) {
            my $value = $self->read_word($addr);
            push @reads, { address => $addr, value => $value };
            if ($i == 15) {
                $self->{regs}[15] = $value & MASK32;
            } else {
                $self->write_register($i, $value);
            }
        } else {
            my $value = ($i == 15)
                ? ($self->{regs}[15] + 4) & MASK32
                : $self->read_register($i);
            $self->write_word($addr, $value);
            push @writes, { address => $addr, value => $value };
        }
        $addr = ($addr + 4) & MASK32;
    }

    if ($d->{write_back}) {
        my $new_base = $d->{up}
            ? ($base + $count * 4) & MASK32
            : ($base - $count * 4) & MASK32;
        $self->write_register($d->{rn}, $new_base);
    }

    return (\@reads, \@writes);
}

# =============================================================================
# Branch
# =============================================================================

sub _exec_branch {
    my ($self, $d) = @_;
    # PC already advanced by 4; branch_base = current PC + 4
    my $branch_base = ($self->get_pc() + 4) & MASK32;

    if ($d->{link}) {
        $self->write_register(14, $self->{regs}[15]);
    }

    my $target = ($branch_base + $d->{branch_offset}) & MASK32;
    $self->set_pc($target & PC_MASK);
}

# =============================================================================
# SWI
# =============================================================================

sub _exec_swi {
    my ($self, $d) = @_;
    if ($d->{swi_comment} == HALT_SWI) {
        $self->{halted} = 1;
        return;
    }
    my $r15_val = $self->{regs}[15];
    $self->{regs}[25] = $r15_val;  # R13_svc
    $self->{regs}[26] = $r15_val;  # R14_svc
    my $r15 = $self->{regs}[15];
    $r15 = ($r15 & (~MODE_MASK & MASK32)) | MODE_SVC;
    $r15 |= FLAG_I;
    $self->{regs}[15] = $r15 & MASK32;
    $self->set_pc(0x08);
}

sub _trap_undefined {
    my ($self) = @_;
    my $r15_val = $self->{regs}[15];
    $self->{regs}[26] = $r15_val;  # R14_svc
    my $r15 = $self->{regs}[15];
    $r15 = ($r15 & (~MODE_MASK & MASK32)) | MODE_SVC;
    $r15 |= FLAG_I;
    $self->{regs}[15] = $r15 & MASK32;
    $self->set_pc(0x04);
}

# =============================================================================
# Step and Run
# =============================================================================

sub step {
    my ($self) = @_;
    return { halted => 1 } if $self->{halted};

    my $current_pc    = $self->get_pc();
    my $flags_before  = $self->get_flags();
    my $instruction   = $self->read_word($current_pc);
    my $d             = $self->decode($instruction);
    my $cond_met      = $self->evaluate_condition($d->{condition}, $flags_before);

    # Advance PC
    $self->set_pc(($current_pc + 4) & PC_MASK);

    my ($reads, $writes) = ([], []);

    if ($cond_met) {
        my $type = $d->{type};
        if    ($type == INST_DATA_PROCESSING) { $self->_exec_data_processing($d); }
        elsif ($type == INST_LOAD_STORE)      { ($reads, $writes) = $self->_exec_load_store($d); }
        elsif ($type == INST_BLOCK_TRANSFER)  { ($reads, $writes) = $self->_exec_block_transfer($d); }
        elsif ($type == INST_BRANCH)          { $self->_exec_branch($d); }
        elsif ($type == INST_SWI)             { $self->_exec_swi($d); }
        else                                   { $self->_trap_undefined(); }
    }

    return {
        address       => $current_pc,
        raw           => $instruction,
        mnemonic      => $self->disassemble($d),
        condition_met => $cond_met ? 1 : 0,
        memory_reads  => $reads,
        memory_writes => $writes,
    };
}

sub run {
    my ($self, $max_steps) = @_;
    my @traces;
    for my $discard (1..$max_steps) {
        last if $self->{halted};
        my $trace = $self->step();
        push @traces, $trace;
        last if $self->{halted};
    }
    return \@traces;
}

# =============================================================================
# Encoding Helpers
# =============================================================================

sub encode_data_processing {
    my ($self, $condition, $opcode, $s, $rn, $rd, $operand2) = @_;
    return (($condition << 28) | $operand2 | ($opcode << 21) | ($s << 20)
            | ($rn << 16) | ($rd << 12)) & MASK32;
}

sub encode_mov_imm {
    my ($self, $condition, $rd, $imm8) = @_;
    return $self->encode_data_processing($condition, OP_MOV, 0, 0, $rd, (1 << 25) | $imm8);
}

sub encode_alu_reg {
    my ($self, $condition, $opcode, $s, $rd, $rn, $rm) = @_;
    return $self->encode_data_processing($condition, $opcode, $s, $rn, $rd, $rm);
}

sub encode_branch {
    my ($self, $condition, $link, $offset) = @_;
    my $inst = ($condition << 28) | 0x0A000000;
    $inst |= 0x01000000 if $link;
    my $encoded = int($offset / 4) & 0x00FFFFFF;
    return ($inst | $encoded) & MASK32;
}

sub encode_halt {
    my ($self) = @_;
    return ((COND_AL << 28) | 0x0F000000 | HALT_SWI) & MASK32;
}

sub encode_ldr {
    my ($self, $condition, $rd, $rn, $offset, $pre_index) = @_;
    my $inst = ($condition << 28) | 0x04100000;
    $inst |= ($rd << 12) | ($rn << 16);
    $inst |= (1 << 24) if $pre_index;
    if ($offset >= 0) {
        $inst |= (1 << 23) | ($offset & 0xFFF);
    } else {
        $inst |= ((-$offset) & 0xFFF);
    }
    return $inst & MASK32;
}

sub encode_str {
    my ($self, $condition, $rd, $rn, $offset, $pre_index) = @_;
    my $inst = ($condition << 28) | 0x04000000;
    $inst |= ($rd << 12) | ($rn << 16);
    $inst |= (1 << 24) if $pre_index;
    if ($offset >= 0) {
        $inst |= (1 << 23) | ($offset & 0xFFF);
    } else {
        $inst |= ((-$offset) & 0xFFF);
    }
    return $inst & MASK32;
}

sub encode_ldm {
    my ($self, $condition, $rn, $reg_list, $write_back, $bt_mode) = @_;
    my $inst = ($condition << 28) | 0x08100000;
    $inst |= ($rn << 16) | $reg_list;
    $inst |= (1 << 21) if $write_back;
    if    ($bt_mode eq 'IA') { $inst |= (1 << 23); }
    elsif ($bt_mode eq 'IB') { $inst |= (1 << 24) | (1 << 23); }
    elsif ($bt_mode eq 'DB') { $inst |= (1 << 24); }
    # DA: no extra bits
    return $inst & MASK32;
}

sub encode_stm {
    my ($self, $condition, $rn, $reg_list, $write_back, $bt_mode) = @_;
    my $inst = $self->encode_ldm($condition, $rn, $reg_list, $write_back, $bt_mode);
    return ($inst & ~(1 << 20)) & MASK32;
}

# String helpers (for convenience)
sub cond_string { return $COND_STRINGS[$_[1]] // '??' }
sub op_string   { return $OP_NAMES[$_[1]]    // '???' }
sub mode_string {
    my $m = $_[1];
    return ($m == MODE_USR ? 'USR' : $m == MODE_FIQ ? 'FIQ' :
            $m == MODE_IRQ ? 'IRQ' : $m == MODE_SVC ? 'SVC' : '???');
}

1;

__END__

=head1 NAME

CodingAdventures::ARM1Simulator - ARM1 (ARMv1) behavioral instruction set simulator

=head1 SYNOPSIS

  use CodingAdventures::ARM1Simulator;
  my $cpu = CodingAdventures::ARM1Simulator->new(4096);
  $cpu->load_instructions([$cpu->encode_mov_imm($cpu->COND_AL, 0, 42),
                            $cpu->encode_halt()]);
  $cpu->run(100);
  print $cpu->read_register(0);  # 42

=head1 DESCRIPTION

Complete ARMv1 instruction set simulator implementing all 16 condition codes,
barrel shifter, ALU operations, data processing, load/store, block transfer,
branch, and SWI instructions.
