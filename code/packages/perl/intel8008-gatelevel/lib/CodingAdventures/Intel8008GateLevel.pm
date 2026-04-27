package CodingAdventures::Intel8008GateLevel;

# ============================================================================
# Intel 8008 Gate-Level Simulator
# ============================================================================
#
# Every arithmetic and logical operation in this simulator routes through
# actual logic gate functions from CodingAdventures::LogicGates and
# CodingAdventures::Arithmetic, chained into an 8-bit ripple-carry adder,
# then a full ALU.
#
# ## How it differs from the behavioral simulator
#
# The behavioral simulator (Intel8008Simulator) computes A + B using Perl's
# native '+' operator. This simulator computes A + B by:
#
#   1. Converting A and B to 8-element bit arrays (int_to_bits)
#   2. Passing them through ripple_carry_adder() from the arithmetic package
#      (which chains 8 full_adder() calls, each using XOR and AND gates)
#   3. Converting the result bit array back to an integer (bits_to_int)
#
# The parity flag uses XORn() (7 XOR gates chained) followed by NOT().
#
# ## The Instruction Execution Path
#
# Every ADD B (opcode 0x80) executes this path:
#
#   opcode → Decoder (gate trees) → control signals
#   registers[B] → read_reg → integer
#   registers[A] → read_reg → integer
#   (A, B) → int_to_bits(A, 8), int_to_bits(B, 8) → bit arrays
#   bit arrays → ripple_carry_adder (40 AND/XOR/OR gates) → result bits
#   result bits → bits_to_int → integer
#   result bits → compute_parity (7 XOR gates + NOT) → parity flag
#   result → write_reg → register (flip-flop update)
#
# ## Dependencies
#
#   CodingAdventures::LogicGates   — AND, OR, NOT, XOR, XORn, Register
#   CodingAdventures::Arithmetic   — half_adder, ripple_carry_adder
#   Intel8008GateLevel::Bits       — int_to_bits, bits_to_int, compute_parity
#   Intel8008GateLevel::ALU        — 8-bit ALU operations
#   Intel8008GateLevel::Registers  — 7×8-bit register file
#   Intel8008GateLevel::Decoder    — opcode → control signals
#   Intel8008GateLevel::Stack      — 8-level push-down stack

use strict;
use warnings;
our $VERSION = '0.01';

use CodingAdventures::LogicGates   qw(AND OR NOT XOR XORn Register new_flip_flop_state);
use CodingAdventures::Arithmetic;

use CodingAdventures::Intel8008GateLevel::Bits     qw(int_to_bits bits_to_int compute_parity);
use CodingAdventures::Intel8008GateLevel::ALU      qw(
    alu_add alu_sub alu_and alu_or alu_xor
    alu_inr alu_dcr
    alu_rlc alu_rrc alu_ral alu_rar
    compute_flags
);
use CodingAdventures::Intel8008GateLevel::Registers qw(
    new_register_file read_reg write_reg reg_a reg_h reg_l
);
use CodingAdventures::Intel8008GateLevel::Decoder  qw(decode);
use CodingAdventures::Intel8008GateLevel::Stack    qw(
    new_stack push_stack pop_stack stack_pc set_pc
);

# Register indices
use constant {
    REG_B => 0, REG_C => 1, REG_D => 2, REG_E => 3,
    REG_H => 4, REG_L => 5, REG_M => 6, REG_A => 7,
};

my @REG_NAMES = qw(B C D E H L M A);
my @COND_FLAGS = ('carry', 'zero', 'sign', 'parity');
my @COND_NAMES_F = ('JFC', 'JFZ', 'JFS', 'JFP');
my @COND_NAMES_T = ('JTC', 'JTZ', 'JTS', 'JTP');
my @COND_NAMES_CF = ('CFC', 'CFZ', 'CFS', 'CFP');
my @COND_NAMES_CT = ('CTC', 'CTZ', 'CTS', 'CTP');
my @COND_NAMES_RF = ('RFC', 'RFZ', 'RFS', 'RFP');
my @COND_NAMES_RT = ('RTC', 'RTZ', 'RTS', 'RTP');
my @ALU_IMM_NAMES = qw(ADI ACI SUI SBI ANI XRI ORI CPI);
my @ALU_REG_NAMES = qw(ADD ADC SUB SBB ANA XRA ORA CMP);

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
    $self->{regs}    = new_register_file();
    $self->{memory}  = [(0) x 16384];
    $self->{stack}   = new_stack();
    $self->{flags}   = { carry => 0, zero => 0, sign => 0, parity => 0 };
    $self->{halted}  = 0;
    $self->{input_ports}  = [(0) x 8];
    $self->{output_ports} = [(0) x 24];
}

# ---------------------------------------------------------------------------
# Public API — Accessors (same interface as behavioral simulator)
# ---------------------------------------------------------------------------

sub a  { read_reg($_[0]->{regs}, REG_A) }
sub b  { read_reg($_[0]->{regs}, REG_B) }
sub c  { read_reg($_[0]->{regs}, REG_C) }
sub d  { read_reg($_[0]->{regs}, REG_D) }
sub e  { read_reg($_[0]->{regs}, REG_E) }
sub h  { read_reg($_[0]->{regs}, REG_H) }
sub l  { read_reg($_[0]->{regs}, REG_L) }
sub pc { stack_pc($_[0]->{stack}) }
sub hl_address { CodingAdventures::Intel8008GateLevel::Registers::hl_address($_[0]->{regs}) }
sub flags  { $_[0]->{flags} }
sub stack  { $_[0]->{stack}{entries} }
sub stack_depth { $_[0]->{stack}{depth} }
sub memory { $_[0]->{memory} }
sub halted { $_[0]->{halted} }

# ---------------------------------------------------------------------------
# Public API — I/O Ports
# ---------------------------------------------------------------------------

sub set_input_port {
    my ($self, $port, $value) = @_;
    die "Intel8008GateLevel: input port must be 0–7\n"   if $port < 0 || $port > 7;
    $self->{input_ports}[$port] = $value & 0xFF;
}

sub get_output_port {
    my ($self, $port) = @_;
    die "Intel8008GateLevel: output port must be 0–23\n" if $port < 0 || $port > 23;
    return $self->{output_ports}[$port] // 0;
}

# ---------------------------------------------------------------------------
# Public API — Execution
# ---------------------------------------------------------------------------

sub load_program {
    my ($self, $program, $start_address) = @_;
    $start_address //= 0;
    set_pc($self->{stack}, $start_address);
    if (ref $program eq 'ARRAY') {
        for my $i (0 .. $#$program) {
            my $addr = ($start_address + $i) & 0x3FFF;
            $self->{memory}[$addr] = $program->[$i] & 0xFF;
        }
    } else {
        my @bytes = unpack('C*', $program);
        for my $i (0 .. $#bytes) {
            my $addr = ($start_address + $i) & 0x3FFF;
            $self->{memory}[$addr] = $bytes[$i] & 0xFF;
        }
    }
}

sub run {
    my ($self, $program, $max_steps, $start_address) = @_;
    $max_steps    //= 100_000;
    $start_address //= 0;
    $self->load_program($program, $start_address);
    my @traces;
    my $steps = 0;
    while (!$self->{halted} && $steps < $max_steps) {
        push @traces, $self->step();
        $steps++;
    }
    return \@traces;
}

sub step {
    my ($self) = @_;
    die "Intel8008GateLevel: CPU is halted\n" if $self->{halted};

    my $address     = $self->pc;
    my $a_before    = $self->a;
    my $flags_before = { %{ $self->{flags} } };

    # Fetch opcode
    my $opcode = $self->_fetch_byte();
    my @raw_bytes = ($opcode);

    # Decode: get control signals via gate trees
    my $ctrl = decode($opcode);

    # Fetch additional bytes based on instruction length
    my ($data, $addr_lo, $addr_hi);
    if ($ctrl->{instr_bytes} == 2) {
        $data = $self->_fetch_byte();
        push @raw_bytes, $data;
    } elsif ($ctrl->{instr_bytes} == 3) {
        $addr_lo = $self->_fetch_byte();
        $addr_hi = $self->_fetch_byte();
        push @raw_bytes, $addr_lo, $addr_hi;
    }

    # Execute instruction using gate-level ALU/register operations
    my ($mnemonic, $mem_addr, $mem_val) =
        $self->_execute($ctrl, $opcode, $data, $addr_lo, $addr_hi);

    return {
        address      => $address,
        raw          => \@raw_bytes,
        mnemonic     => $mnemonic,
        a_before     => $a_before,
        a_after      => $self->a,
        flags_before => $flags_before,
        flags_after  => { %{ $self->{flags} } },
        memory_address => $mem_addr,
        memory_value   => $mem_val,
    };
}

sub reset {
    my ($self) = @_;
    $self->_init_state();
}

# ---------------------------------------------------------------------------
# Private: fetch helper
# ---------------------------------------------------------------------------

sub _fetch_byte {
    my ($self) = @_;
    my $pc  = stack_pc($self->{stack});
    my $val = $self->{memory}[$pc] // 0;
    set_pc($self->{stack}, ($pc + 1) & 0x3FFF);
    return $val;
}

# ---------------------------------------------------------------------------
# Private: register read/write (handles M pseudo-register)
# ---------------------------------------------------------------------------

sub _read_reg_or_mem {
    my ($self, $reg) = @_;
    if ($reg == REG_M) {
        my $addr = CodingAdventures::Intel8008GateLevel::Registers::hl_address($self->{regs});
        return ($self->{memory}[$addr] // 0, $addr);
    }
    return (read_reg($self->{regs}, $reg), undef);
}

sub _write_reg_or_mem {
    my ($self, $reg, $value) = @_;
    $value &= 0xFF;
    if ($reg == REG_M) {
        my $addr = CodingAdventures::Intel8008GateLevel::Registers::hl_address($self->{regs});
        $self->{memory}[$addr] = $value;
        return $addr;
    }
    write_reg($self->{regs}, $reg, $value);
    return undef;
}

# ---------------------------------------------------------------------------
# Private: instruction executor
# ---------------------------------------------------------------------------

sub _execute {
    my ($self, $ctrl, $opcode, $data, $addr_lo, $addr_hi) = @_;

    my $ddd = $ctrl->{ddd};
    my $sss = $ctrl->{sss};

    # HLT
    if ($ctrl->{is_hlt}) {
        $self->{halted} = 1;
        return ('HLT', undef, undef);
    }

    # -----------------------------------------------------------------------
    # Group 01: MOV, IN, JMP/CALL
    # -----------------------------------------------------------------------
    if ($ctrl->{group_01}) {

        # IN: sss=001
        if ($sss == 0b001) {
            my $port = $ddd;
            write_reg($self->{regs}, REG_A, $self->{input_ports}[$port] & 0xFF);
            return (sprintf("IN %d", $port), undef, undef);
        }

        # Jumps and calls (3-byte instructions)
        my $is_jcc_sss = ($sss == 0b000 || $sss == 0b100 ||
                          $sss == 0b010 || $sss == 0b110);
        if ($is_jcc_sss && ($ddd <= 3 || ($ddd == 7 && ($sss == 0b100 || $sss == 0b110)))) {
            if ($sss == 0b000 || $sss == 0b100) {
                return $self->_exec_jump($ddd, $sss, $addr_lo, $addr_hi);
            } else {
                return $self->_exec_call($ddd, $sss, $addr_lo, $addr_hi);
            }
        }

        # MOV DDD, SSS
        return $self->_exec_mov($ddd, $sss);
    }

    # -----------------------------------------------------------------------
    # Group 10: ALU register operations
    # -----------------------------------------------------------------------
    if ($ctrl->{group_10}) {
        return $self->_exec_alu_reg($ddd, $sss);
    }

    # -----------------------------------------------------------------------
    # Group 11: ALU immediate
    # -----------------------------------------------------------------------
    if ($ctrl->{group_11}) {
        if ($sss == 0b100) {
            return $self->_exec_alu_imm($ddd, $data);
        }
        return (sprintf("UNKNOWN(0x%02X)", $opcode), undef, undef);
    }

    # -----------------------------------------------------------------------
    # Group 00: INR, DCR, MVI, Rotates, RET, RST, OUT
    # -----------------------------------------------------------------------

    # Rotate: group=00, sss=010, bit5=0
    if ($ctrl->{is_rot}) {
        return $self->_exec_rotate($ddd);
    }

    # Return family: group=00, sss ∈ {011, 111}
    if ($sss == 0b011 || $sss == 0b111) {
        return $self->_exec_return($ddd, $sss);
    }

    # RST: group=00, sss=101
    if ($sss == 0b101) {
        my $target = ($ddd & 7) << 3;
        push_stack($self->{stack}, $target);
        return (sprintf("RST %d", $ddd), undef, undef);
    }

    # OUT: group=00, sss=010, bit5=1 (ddd >= 4)
    if ($sss == 0b010 && ($ddd & 0b100) != 0) {
        my $port = $ddd;
        $self->{output_ports}[$port] = read_reg($self->{regs}, REG_A);
        return (sprintf("OUT %d", $port), undef, undef);
    }

    # MVI: group=00, sss=110
    if ($sss == 0b110) {
        return $self->_exec_mvi($ddd, $data);
    }

    # INR: group=00, sss=000
    if ($sss == 0b000) {
        return $self->_exec_inr($ddd);
    }

    # DCR: group=00, sss=001
    if ($sss == 0b001) {
        return $self->_exec_dcr($ddd);
    }

    return (sprintf("UNKNOWN(0x%02X)", $opcode), undef, undef);
}

# ---------------------------------------------------------------------------
# Instruction handlers
# ---------------------------------------------------------------------------

sub _exec_mov {
    my ($self, $ddd, $sss) = @_;
    my ($val, $src_addr) = $self->_read_reg_or_mem($sss);
    my $dst_addr = $self->_write_reg_or_mem($ddd, $val);
    my $mem_addr = $src_addr // $dst_addr;
    return (
        sprintf("MOV %s, %s", $REG_NAMES[$ddd], $REG_NAMES[$sss]),
        $mem_addr,
        defined($mem_addr) ? $val : undef,
    );
}

sub _exec_mvi {
    my ($self, $ddd, $data) = @_;
    $data //= 0;
    my $addr = $self->_write_reg_or_mem($ddd, $data);
    return (
        sprintf("MVI %s, 0x%02X", $REG_NAMES[$ddd], $data),
        $addr,
        defined($addr) ? ($data & 0xFF) : undef,
    );
}

sub _exec_inr {
    my ($self, $ddd) = @_;
    my ($val, $src_addr) = $self->_read_reg_or_mem($ddd);
    my $old_carry = $self->{flags}{carry};
    my ($result, $new_flags) = alu_inr($val, $old_carry);
    my $addr = $self->_write_reg_or_mem($ddd, $result);
    $self->{flags} = $new_flags;
    my $mem_addr = $src_addr // $addr;
    return (
        sprintf("INR %s", $REG_NAMES[$ddd]),
        $mem_addr,
        defined($mem_addr) ? $result : undef,
    );
}

sub _exec_dcr {
    my ($self, $ddd) = @_;
    my ($val, $src_addr) = $self->_read_reg_or_mem($ddd);
    my $old_carry = $self->{flags}{carry};
    my ($result, $new_flags) = alu_dcr($val, $old_carry);
    my $addr = $self->_write_reg_or_mem($ddd, $result);
    $self->{flags} = $new_flags;
    my $mem_addr = $src_addr // $addr;
    return (
        sprintf("DCR %s", $REG_NAMES[$ddd]),
        $mem_addr,
        defined($mem_addr) ? $result : undef,
    );
}

sub _exec_alu_reg {
    my ($self, $ooo, $sss) = @_;
    my ($src, $mem_addr) = $self->_read_reg_or_mem($sss);
    my $a = read_reg($self->{regs}, REG_A);
    my ($result, $new_flags) = $self->_alu_op($ooo, $a, $src);
    if ($ooo != 0b111) {
        write_reg($self->{regs}, REG_A, $result);
    }
    $self->{flags} = $new_flags;
    return (
        sprintf("%s %s", $ALU_REG_NAMES[$ooo], $REG_NAMES[$sss]),
        $mem_addr,
        defined($mem_addr) ? $src : undef,
    );
}

sub _exec_alu_imm {
    my ($self, $ooo, $data) = @_;
    $data //= 0;
    my $a = read_reg($self->{regs}, REG_A);
    my ($result, $new_flags) = $self->_alu_op($ooo, $a, $data);
    if ($ooo != 0b111) {
        write_reg($self->{regs}, REG_A, $result);
    }
    $self->{flags} = $new_flags;
    return (sprintf("%s 0x%02X", $ALU_IMM_NAMES[$ooo], $data), undef, undef);
}

# Gate-level ALU dispatch (routes through actual gate functions)
# All ALU functions return ($result, $carry_out, $flags_hashref).
# This dispatcher extracts only ($result, $flags_hashref) — the carry is
# already embedded in the flags hashref, so we discard the standalone carry.
sub _alu_op {
    my ($self, $ooo, $a, $b) = @_;

    my ($result, undef, $flags);
    if    ($ooo == 0) { ($result, undef, $flags) = alu_add($a, $b, 0) }
    elsif ($ooo == 1) { ($result, undef, $flags) = alu_add($a, $b, $self->{flags}{carry} ? 1 : 0) }
    elsif ($ooo == 2) { ($result, undef, $flags) = alu_sub($a, $b, 0) }
    elsif ($ooo == 3) { ($result, undef, $flags) = alu_sub($a, $b, $self->{flags}{carry} ? 1 : 0) }
    elsif ($ooo == 4) { ($result, undef, $flags) = alu_and($a, $b) }
    elsif ($ooo == 5) { ($result, undef, $flags) = alu_xor($a, $b) }
    elsif ($ooo == 6) { ($result, undef, $flags) = alu_or($a, $b) }
    else {             ($result, undef, $flags) = alu_sub($a, $b, 0) }  # CMP

    return ($result, $flags);
}

sub _exec_rotate {
    my ($self, $ddd) = @_;
    my $rr = $ddd & 3;
    my $a  = read_reg($self->{regs}, REG_A);
    my ($new_a, $new_cy, $mnem);

    if    ($rr == 0) { ($new_a, $new_cy) = alu_rlc($a); $mnem = 'RLC' }
    elsif ($rr == 1) { ($new_a, $new_cy) = alu_rrc($a); $mnem = 'RRC' }
    elsif ($rr == 2) { ($new_a, $new_cy) = alu_ral($a, $self->{flags}{carry}); $mnem = 'RAL' }
    else             { ($new_a, $new_cy) = alu_rar($a, $self->{flags}{carry}); $mnem = 'RAR' }

    write_reg($self->{regs}, REG_A, $new_a);
    $self->{flags}{carry} = $new_cy;
    return ($mnem, undef, undef);
}

sub _exec_jump {
    my ($self, $ddd, $sss, $addr_lo, $addr_hi) = @_;
    $addr_lo //= 0;
    $addr_hi //= 0;
    my $target = (($addr_hi & 0x3F) << 8) | $addr_lo;

    if ($ddd == 7 && $sss == 0b100) {
        set_pc($self->{stack}, $target);
        return (sprintf("JMP 0x%04X", $target), undef, undef);
    }

    my $t   = ($sss >> 2) & 1;
    my $ccc = $ddd & 3;
    my $flag_val = $self->{flags}{$COND_FLAGS[$ccc]} ? 1 : 0;
    my $should_jump = ($t == 1) ? $flag_val : (1 - $flag_val);
    my $mnem = $t ? $COND_NAMES_T[$ccc] : $COND_NAMES_F[$ccc];

    if ($should_jump) { set_pc($self->{stack}, $target) }
    return (sprintf("%s 0x%04X", $mnem, $target), undef, undef);
}

sub _exec_call {
    my ($self, $ddd, $sss, $addr_lo, $addr_hi) = @_;
    $addr_lo //= 0;
    $addr_hi //= 0;
    my $target = (($addr_hi & 0x3F) << 8) | $addr_lo;

    if ($ddd == 7 && $sss == 0b110) {
        push_stack($self->{stack}, $target);
        return (sprintf("CAL 0x%04X", $target), undef, undef);
    }

    my $t   = ($sss >> 2) & 1;
    my $ccc = $ddd & 3;
    my $flag_val = $self->{flags}{$COND_FLAGS[$ccc]} ? 1 : 0;
    my $should_call = ($t == 1) ? $flag_val : (1 - $flag_val);
    my $mnem = $t ? $COND_NAMES_CT[$ccc] : $COND_NAMES_CF[$ccc];

    if ($should_call) { push_stack($self->{stack}, $target) }
    return (sprintf("%s 0x%04X", $mnem, $target), undef, undef);
}

sub _exec_return {
    my ($self, $ddd, $sss) = @_;

    if ($ddd == 7 && $sss == 7) {
        pop_stack($self->{stack});
        return ('RET', undef, undef);
    }

    my $t   = ($sss >> 2) & 1;
    my $ccc = $ddd & 3;
    my $flag_val = $self->{flags}{$COND_FLAGS[$ccc]} ? 1 : 0;
    my $should_ret = ($t == 1) ? $flag_val : (1 - $flag_val);
    my $mnem = $t ? $COND_NAMES_RT[$ccc] : $COND_NAMES_RF[$ccc];

    if ($should_ret) { pop_stack($self->{stack}) }
    return ($mnem, undef, undef);
}

1;

__END__

=head1 NAME

CodingAdventures::Intel8008GateLevel - Gate-level Intel 8008 simulator

=head1 SYNOPSIS

    use CodingAdventures::Intel8008GateLevel;

    my $cpu = CodingAdventures::Intel8008GateLevel->new();
    my $traces = $cpu->run([0x06, 0x01, 0x3E, 0x02, 0x80, 0x76]);
    print "A = ", $cpu->a, "\n";  # 3

=head1 DESCRIPTION

Gate-level Intel 8008 simulator. Every arithmetic operation routes through
actual AND/OR/XOR/NOT gate functions via the arithmetic package's
ripple_carry_adder, and the parity flag is computed by XORn() + NOT().

Implements the same interface as CodingAdventures::Intel8008Simulator so
both can run the same programs and produce identical traces.

=cut
