package CodingAdventures::Intel4004GateLevel;

# ============================================================================
# Intel 4004 Gate-Level Simulator
# ============================================================================
#
# Every computation in this simulator routes through actual logic gate
# functions — AND, OR, XOR, NOT — chained into adders, then into a
# 4-bit ALU. Registers are built from D flip-flops via the Register()
# function from CodingAdventures::LogicGates. The program counter uses a
# half-adder chain for incrementing.
#
# This is NOT the same as the behavioral simulator (Intel4004Simulator).
# The behavioral simulator executes instructions directly with host-language
# integers. This simulator routes everything through the gate abstractions
# built from scratch.
#
# ## Why gate-level?
#
#   1. Count gates: how many AND/OR/NOT ops does ADD R3 actually require?
#   2. Trace signals: follow a bit from register R3 through the ALU
#   3. Understand timing: a ripple-carry add takes 4 gate delays
#   4. Appreciate constraints: 2,300 transistors is incredibly few
#
# ## Every ADD instruction traverses:
#
#   a_bits = int_to_bits(A, 4)
#   b_bits = int_to_bits(Rn, 4)
#   ripple_carry_adder(\@a_bits, \@b_bits, $cin):
#     full_adder(a[0], b[0], cin)   -> sum[0], carry1
#     full_adder(a[1], b[1], carry1) -> sum[1], carry2
#     full_adder(a[2], b[2], carry2) -> sum[2], carry3
#     full_adder(a[3], b[3], carry3) -> sum[3], carry_out
#
# ## Flip-flop two-phase write
#
# The Register() function models a master-slave D flip-flop:
#   clock=0: data is loaded into the master latch (not yet visible at output)
#   clock=1: master latches to slave (data appears at output)
# We perform both phases for every write to get the updated output state.
#
# ## Dependencies
#
#   CodingAdventures::LogicGates   — AND, OR, XOR, NOT, Register, new_flip_flop_state
#   CodingAdventures::Arithmetic   — half_adder, ripple_carry_adder

use strict;
use warnings;
our $VERSION = '0.01';

use CodingAdventures::LogicGates   qw(AND OR NOT XOR Register new_flip_flop_state);
use CodingAdventures::Arithmetic;

# Use fully-qualified names for Arithmetic functions (module doesn't export by default).
sub half_adder        { CodingAdventures::Arithmetic::half_adder(@_) }
sub ripple_carry_adder { CodingAdventures::Arithmetic::ripple_carry_adder(@_) }

# ---------------------------------------------------------------------------
# Bit conversion helpers
# ---------------------------------------------------------------------------

# Convert an integer to an LSB-first bit arrayref of given width.
# Example: int_to_bits(5, 4) = [1, 0, 1, 0]  (5 = 0101 in binary)
sub _int_to_bits {
    my ($value, $width) = @_;
    my $mask = ($width >= 32) ? 0xFFFFFFFF : ((1 << $width) - 1);
    $value = $value & $mask;
    my @bits;
    for my $i (0 .. $width - 1) {
        push @bits, ($value >> $i) & 1;
    }
    return \@bits;
}

# Convert an LSB-first bit arrayref to an integer.
# Example: bits_to_int([1, 0, 1, 0]) = 5
sub _bits_to_int {
    my ($bits) = @_;
    my $value = 0;
    for my $i (0 .. $#$bits) {
        $value |= ($bits->[$i] << $i);
    }
    return $value;
}

# ---------------------------------------------------------------------------
# ALU helpers — wrap the arithmetic package's ripple_carry_adder
# ---------------------------------------------------------------------------

# Add two 4-bit integers with carry, using the gate-level adder.
# Returns: ($result, $carry_out) where carry_out is 0 or 1.
sub _gate_add {
    my ($self, $a, $b, $carry_in) = @_;
    my $a_bits = _int_to_bits($a, 4);
    my $b_bits = _int_to_bits($b, 4);
    my $cin    = $carry_in ? 1 : 0;
    my ($result_bits, $carry_out) = ripple_carry_adder($a_bits, $b_bits, $cin);
    return (_bits_to_int($result_bits), $carry_out);
}

# Bitwise NOT of a 4-bit value using NOT gates.
sub _gate_not4 {
    my ($self, $a) = @_;
    my $bits = _int_to_bits($a, 4);
    my @out;
    for my $b (@$bits) { push @out, NOT($b) }
    return _bits_to_int(\@out);
}

# ---------------------------------------------------------------------------
# Flip-flop state helpers
# ---------------------------------------------------------------------------

# Create initial flip-flop state for a width-bit register.
# Returns an arrayref of flip-flop state hashrefs.
sub _new_ff_state {
    my ($width) = @_;
    return [ map { new_flip_flop_state() } 1 .. $width ];
}

# Read an integer from a flip-flop register state.
# We use clock=0 to read the slave output without latching new data.
sub _read_ff {
    my ($state, $width) = @_;
    my $zero_bits = [ (0) x $width ];
    my ($output, undef) = Register($zero_bits, 0, $state);
    return _bits_to_int($output);
}

# Write an integer to a flip-flop register state.
# Two-phase write:
#   Phase 1 (clock=0): load data into master latch
#   Phase 2 (clock=1): latch master to slave (data appears at output)
sub _write_ff {
    my ($value, $width, $state) = @_;
    my $bits = _int_to_bits($value, $width);
    my (undef, $state1) = Register($bits, 0, $state);
    my (undef, $new_state) = Register($bits, 1, $state1);
    return $new_state;
}

# ---------------------------------------------------------------------------
# Constructor
# ---------------------------------------------------------------------------

sub new {
    my ($class) = @_;
    my $self = bless {}, $class;
    $self->_init();
    return $self;
}

sub _init {
    my ($self) = @_;

    # 16 × 4-bit registers (each stored as a flip-flop state arrayref)
    $self->{reg_states} = [ map { _new_ff_state(4) } 0..15 ];

    # 4-bit accumulator flip-flop state
    $self->{acc_state} = _new_ff_state(4);

    # 1-bit carry flag flip-flop state
    $self->{carry_state} = _new_ff_state(1);

    # 12-bit program counter flip-flop state
    $self->{pc_state} = _new_ff_state(12);

    # 3-level hardware stack (3 × 12-bit registers as flip-flop states)
    $self->{stack_states}   = [ map { _new_ff_state(12) } 0..2 ];
    $self->{stack_pointer}  = 0;    # 0-indexed, next-write slot (0-2)

    # RAM: stored as flip-flop state hashrefs in a flat keyed hash.
    # Key: bank*10000 + reg*100 + char (all 0-indexed)
    $self->{ram_states}        = {};
    $self->{ram_status_states} = {};
    $self->{ram_output}        = [0, 0, 0, 0];

    # ROM (4096 bytes, 0-indexed)
    $self->{rom} = [(0) x 4096];

    # RAM addressing (0-indexed, set by SRC instruction)
    $self->{ram_bank}      = 0;
    $self->{ram_register}  = 0;
    $self->{ram_character} = 0;

    # ROM I/O port
    $self->{rom_port} = 0;

    # Control
    $self->{halted} = 0;
}

# ---------------------------------------------------------------------------
# Public API (same interface as behavioral simulator)
# ---------------------------------------------------------------------------

# Load a program (arrayref of bytes or a string) into ROM at address 0.
sub load_program {
    my ($self, $program) = @_;
    $self->_write_pc_val(0);
    if (ref $program eq 'ARRAY') {
        for my $i (0 .. $#$program) {
            $self->{rom}[$i] = $program->[$i] & 0xFF;
        }
    } else {
        my @bytes = unpack('C*', $program);
        for my $i (0 .. $#bytes) {
            $self->{rom}[$i] = $bytes[$i] & 0xFF;
        }
    }
}

# Run a program, return arrayref of trace hashrefs.
sub run {
    my ($self, $program, $max_steps) = @_;
    $max_steps //= 10_000;
    $self->load_program($program);
    my @traces;
    my $steps = 0;
    while (!$self->{halted} && $self->_read_pc_val() < 4096 && $steps < $max_steps) {
        push @traces, $self->step();
        $steps++;
    }
    return \@traces;
}

# Execute one instruction, routing all operations through gate functions.
# Returns a trace hashref.
sub step {
    my ($self) = @_;
    die "CPU is halted — cannot step further\n" if $self->{halted};

    my $address = $self->_read_pc_val();
    my $raw     = $self->{rom}[$address] // 0;
    $self->_inc_pc();

    my $raw2;
    if ($self->_is_two_byte($raw)) {
        $raw2 = $self->{rom}[$self->_read_pc_val()] // 0;
        $self->_inc_pc();
    }

    my $acc_before   = $self->_read_acc();
    my $carry_before = $self->_read_carry();

    my $mnemonic = $self->_execute($raw, $raw2, $address);

    return {
        address            => $address,
        raw                => $raw,
        raw2               => $raw2,
        mnemonic           => $mnemonic,
        accumulator_before => $acc_before,
        accumulator_after  => $self->_read_acc(),
        carry_before       => $carry_before,
        carry_after        => $self->_read_carry(),
    };
}

# Reset CPU to initial state.
sub reset {
    my ($self) = @_;
    $self->_init();
}

# Return gate count estimates per component (educational).
# These approximate the 4004's real transistor budget.
sub gate_count {
    return {
        alu       => 80,    # 4 full adders x ~20 gates each
        registers => 256,   # 16 regs x 4 bits x 4 gates per flip-flop
        acc       => 16,
        carry     => 4,
        decoder   => 120,   # AND/OR/NOT tree for all opcodes
        pc        => 96,    # 12 half-adders for increment
        stack     => 144,   # 3 x 12 bits x 4 gates
        total     => 716,   # close to 4004's ~786 estimated gates
    };
}

# Accessors for testing
sub halted { $_[0]->{halted} }
sub accumulator { $_[0]->_read_acc() }
sub carry { $_[0]->_read_carry() }
sub pc { $_[0]->_read_pc_val() }

sub get_register {
    my ($self, $reg) = @_;
    return $self->_read_reg($reg);
}

# ---------------------------------------------------------------------------
# Private: PC register operations (gate-level via flip-flops)
# ---------------------------------------------------------------------------

sub _read_pc_val {
    my ($self) = @_;
    return _read_ff($self->{pc_state}, 12);
}

sub _write_pc_val {
    my ($self, $value) = @_;
    $self->{pc_state} = _write_ff($value & 0xFFF, 12, $self->{pc_state});
}

# Increment PC by 1 using a chain of half-adders.
# Each half_adder(bit, carry) -> (sum, carry_out).
# This models the ripple-carry incrementer in the real 4004.
sub _inc_pc {
    my ($self) = @_;
    my $bits  = _int_to_bits($self->_read_pc_val(), 12);
    my $carry = 1;   # adding 1 = initial carry into bit 0
    my @new_bits;
    for my $b (@$bits) {
        my ($sum, $cout) = half_adder($b, $carry);
        push @new_bits, $sum;
        $carry = $cout;
    }
    $self->_write_pc_val(_bits_to_int(\@new_bits));
}

# ---------------------------------------------------------------------------
# Private: accumulator and carry via flip-flops
# ---------------------------------------------------------------------------

sub _read_acc {
    my ($self) = @_;
    return _read_ff($self->{acc_state}, 4);
}

sub _write_acc {
    my ($self, $value) = @_;
    $self->{acc_state} = _write_ff($value & 0xF, 4, $self->{acc_state});
}

sub _read_carry {
    my ($self) = @_;
    return _read_ff($self->{carry_state}, 1) ? 1 : 0;
}

sub _write_carry {
    my ($self, $value) = @_;
    my $bit = $value ? 1 : 0;
    $self->{carry_state} = _write_ff($bit, 1, $self->{carry_state});
}

# ---------------------------------------------------------------------------
# Private: general registers via flip-flops
# ---------------------------------------------------------------------------

sub _read_reg {
    my ($self, $index) = @_;
    return _read_ff($self->{reg_states}[$index], 4);
}

sub _write_reg {
    my ($self, $index, $value) = @_;
    $self->{reg_states}[$index] = _write_ff($value & 0xF, 4, $self->{reg_states}[$index]);
}

# ---------------------------------------------------------------------------
# Private: register pairs
# ---------------------------------------------------------------------------

sub _read_pair {
    my ($self, $pair) = @_;
    my $high = $self->_read_reg($pair * 2);
    my $low  = $self->_read_reg($pair * 2 + 1);
    return ($high << 4) | $low;
}

sub _write_pair {
    my ($self, $pair, $value) = @_;
    $self->_write_reg($pair * 2,     ($value >> 4) & 0xF);
    $self->_write_reg($pair * 2 + 1, $value & 0xF);
}

# ---------------------------------------------------------------------------
# Private: 3-level hardware stack via flip-flops
# ---------------------------------------------------------------------------

sub _stack_push {
    my ($self, $addr) = @_;
    $self->{stack_states}[$self->{stack_pointer}] =
        _write_ff($addr & 0xFFF, 12, $self->{stack_states}[$self->{stack_pointer}]);
    $self->{stack_pointer} = ($self->{stack_pointer} + 1) % 3;
}

sub _stack_pop {
    my ($self) = @_;
    $self->{stack_pointer} = ($self->{stack_pointer} + 2) % 3;
    return _read_ff($self->{stack_states}[$self->{stack_pointer}], 12);
}

# ---------------------------------------------------------------------------
# Private: RAM via flip-flop states (flat keyed map)
# ---------------------------------------------------------------------------

sub _ram_key {
    my ($bank, $reg, $char) = @_;
    return $bank * 10000 + $reg * 100 + $char;
}

sub _ram_read_main {
    my ($self) = @_;
    my $key   = _ram_key(@{$self}{qw(ram_bank ram_register ram_character)});
    my $state = $self->{ram_states}{$key};
    return 0 unless defined $state;
    return _read_ff($state, 4);
}

sub _ram_write_main {
    my ($self, $value) = @_;
    my $key   = _ram_key(@{$self}{qw(ram_bank ram_register ram_character)});
    my $state = $self->{ram_states}{$key} // _new_ff_state(4);
    $self->{ram_states}{$key} = _write_ff($value & 0xF, 4, $state);
}

sub _ram_read_status {
    my ($self, $idx) = @_;
    my $key   = _ram_key($self->{ram_bank}, $self->{ram_register}, $idx + 100);
    my $state = $self->{ram_status_states}{$key};
    return 0 unless defined $state;
    return _read_ff($state, 4);
}

sub _ram_write_status {
    my ($self, $idx, $value) = @_;
    my $key   = _ram_key($self->{ram_bank}, $self->{ram_register}, $idx + 100);
    my $state = $self->{ram_status_states}{$key} // _new_ff_state(4);
    $self->{ram_status_states}{$key} = _write_ff($value & 0xF, 4, $state);
}

# ---------------------------------------------------------------------------
# Private: 2-byte detection and instruction dispatcher
# ---------------------------------------------------------------------------

sub _is_two_byte {
    my ($self, $raw) = @_;
    my $upper = ($raw >> 4) & 0xF;
    return 1 if $upper == 0x1 || $upper == 0x4 || $upper == 0x5 || $upper == 0x7;
    return 1 if $upper == 0x2 && ($raw & 1) == 0;
    return 0;
}

sub _execute {
    my ($self, $raw, $raw2, $addr) = @_;

    return 'NOP' if $raw == 0x00;
    if ($raw == 0x01) { $self->{halted} = 1; return 'HLT' }

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
    return sprintf "UNKNOWN(0x%02X)", $raw;
}

# ---------------------------------------------------------------------------
# Instructions — all arithmetic routed through gate functions
# ---------------------------------------------------------------------------

sub _exec_ldm {
    my ($self, $n) = @_;
    $self->_write_acc($n & 0xF);
    return "LDM $n";
}

sub _exec_ld {
    my ($self, $reg) = @_;
    $self->_write_acc($self->_read_reg($reg));
    return "LD R$reg";
}

sub _exec_xch {
    my ($self, $reg) = @_;
    my $old_a   = $self->_read_acc();
    my $reg_val = $self->_read_reg($reg);
    $self->_write_acc($reg_val);
    $self->_write_reg($reg, $old_a);
    return "XCH R$reg";
}

# INC: increment via half-adder chain (models the real incrementer circuit)
sub _exec_inc {
    my ($self, $reg) = @_;
    my $bits  = _int_to_bits($self->_read_reg($reg), 4);
    my $carry = 1;
    my @new_bits;
    for my $b (@$bits) {
        my ($sum, $cout) = half_adder($b, $carry);
        push @new_bits, $sum;
        $carry = $cout;
    }
    $self->_write_reg($reg, _bits_to_int(\@new_bits) & 0xF);
    return "INC R$reg";
}

# ADD: uses the gate-level ripple-carry adder
# A = A + Rn + carry_in; carry set if overflow
sub _exec_add {
    my ($self, $reg) = @_;
    my ($result, $cout) = $self->_gate_add(
        $self->_read_acc(), $self->_read_reg($reg), $self->_read_carry()
    );
    $self->_write_acc($result & 0xF);
    $self->_write_carry($cout);
    return "ADD R$reg";
}

# SUB: complement-add subtraction through gates
# A = A + NOT(Rn) + borrow_in; carry=1 means no borrow
sub _exec_sub {
    my ($self, $reg) = @_;
    my $a        = $self->_read_acc();
    my $rn       = $self->_read_reg($reg);
    my $carry    = $self->_read_carry();
    my $compl_rn = $self->_gate_not4($rn);
    my $borrow_in = !$carry;   # carry=1 means no borrow -> borrow_in=0
    my ($result, $cout) = $self->_gate_add($a, $compl_rn, $borrow_in);
    $self->_write_acc($result & 0xF);
    $self->_write_carry($cout);
    return "SUB R$reg";
}

sub _exec_jun {
    my ($self, $lower, $raw2) = @_;
    my $target = ($lower << 8) | $raw2;
    $self->_write_pc_val($target);
    return sprintf "JUN 0x%03X", $target;
}

sub _exec_jcn {
    my ($self, $cond, $raw2, $addr) = @_;
    my $acc   = $self->_read_acc();
    my $carry = $self->_read_carry();
    my $test_zero  = ($cond & 0x4) && $acc == 0;
    my $test_carry = ($cond & 0x2) && $carry;
    my $result     = $test_zero || $test_carry;
    $result = !$result if $cond & 0x8;
    my $page   = ($addr + 2) & 0xF00;
    my $target = $page | $raw2;
    $self->_write_pc_val($target) if $result;
    return sprintf "JCN %d,0x%02X", $cond, $raw2;
}

sub _exec_jms {
    my ($self, $lower, $raw2, $addr) = @_;
    my $target = ($lower << 8) | $raw2;
    $self->_stack_push($addr + 2);
    $self->_write_pc_val($target);
    return sprintf "JMS 0x%03X", $target;
}

sub _exec_bbl {
    my ($self, $n) = @_;
    my $ret = $self->_stack_pop();
    $self->_write_acc($n & 0xF);
    $self->_write_pc_val($ret);
    return "BBL $n";
}

# ISZ: increment via half-adder chain, then skip if zero
sub _exec_isz {
    my ($self, $reg, $raw2, $addr) = @_;
    my $bits  = _int_to_bits($self->_read_reg($reg), 4);
    my $carry = 1;
    my @new_bits;
    for my $b (@$bits) {
        my ($sum, $cout) = half_adder($b, $carry);
        push @new_bits, $sum;
        $carry = $cout;
    }
    my $new_val = _bits_to_int(\@new_bits) & 0xF;
    $self->_write_reg($reg, $new_val);
    if ($new_val != 0) {
        $self->_write_pc_val((($addr + 2) & 0xF00) | $raw2);
    }
    return sprintf "ISZ R%d,0x%02X", $reg, $raw2;
}

sub _exec_fim {
    my ($self, $pair, $data) = @_;
    $self->_write_pair($pair, $data);
    return sprintf "FIM P%d,0x%02X", $pair, $data;
}

sub _exec_src {
    my ($self, $pair) = @_;
    my $pair_val = $self->_read_pair($pair);
    $self->{ram_register}  = (($pair_val >> 4) & 0xF) % 4;
    $self->{ram_character} = $pair_val & 0xF;
    return "SRC P$pair";
}

sub _exec_fin {
    my ($self, $pair, $addr) = @_;
    my $p0_val   = $self->_read_pair(0);
    my $page     = $addr & 0xF00;
    my $rom_addr = $page | $p0_val;
    my $byte     = $self->{rom}[$rom_addr] // 0;
    $self->_write_pair($pair, $byte);
    return "FIN P$pair";
}

sub _exec_jin {
    my ($self, $pair, $addr) = @_;
    my $pair_val = $self->_read_pair($pair);
    $self->_write_pc_val(($addr & 0xF00) | $pair_val);
    return "JIN P$pair";
}

# ---------------------------------------------------------------------------
# I/O instructions (0xE0-0xEF)
# ---------------------------------------------------------------------------

sub _exec_io {
    my ($self, $raw) = @_;
    my $acc = $self->_read_acc();

    if ($raw == 0xE0) {
        $self->_ram_write_main($acc);
        return "WRM";
    } elsif ($raw == 0xE1) {
        $self->{ram_output}[$self->{ram_bank}] = $acc & 0xF;
        return "WMP";
    } elsif ($raw == 0xE2) {
        $self->{rom_port} = $acc & 0xF;
        return "WRR";
    } elsif ($raw == 0xE3) {
        return "WPM";
    } elsif ($raw >= 0xE4 && $raw <= 0xE7) {
        $self->_ram_write_status($raw - 0xE4, $acc);
        return "WR" . ($raw - 0xE4);
    } elsif ($raw == 0xE8) {
        # SBM: subtract RAM from accumulator through gates
        my $ram_val   = $self->_ram_read_main();
        my $compl_val = $self->_gate_not4($ram_val);
        my $borrow_in = !$self->_read_carry();
        my ($result, $cout) = $self->_gate_add($acc, $compl_val, $borrow_in);
        $self->_write_acc($result & 0xF);
        $self->_write_carry($cout);
        return "SBM";
    } elsif ($raw == 0xE9) {
        $self->_write_acc($self->_ram_read_main());
        return "RDM";
    } elsif ($raw == 0xEA) {
        $self->_write_acc($self->{rom_port} & 0xF);
        return "RDR";
    } elsif ($raw == 0xEB) {
        # ADM: add RAM to accumulator through gates
        my $ram_val  = $self->_ram_read_main();
        my $carry_in = $self->_read_carry();
        my ($result, $cout) = $self->_gate_add($acc, $ram_val, $carry_in);
        $self->_write_acc($result & 0xF);
        $self->_write_carry($cout);
        return "ADM";
    } elsif ($raw >= 0xEC && $raw <= 0xEF) {
        $self->_write_acc($self->_ram_read_status($raw - 0xEC));
        return "RD" . ($raw - 0xEC);
    }
    return sprintf "UNKNOWN(0x%02X)", $raw;
}

# ---------------------------------------------------------------------------
# Accumulator instructions (0xF0-0xFD)
# ---------------------------------------------------------------------------

sub _exec_accum {
    my ($self, $raw) = @_;
    my $acc   = $self->_read_acc();
    my $carry = $self->_read_carry();

    if ($raw == 0xF0) {
        $self->_write_acc(0); $self->_write_carry(0); return "CLB";
    } elsif ($raw == 0xF1) {
        $self->_write_carry(0); return "CLC";
    } elsif ($raw == 0xF2) {
        # IAC: increment via gate_add (carry_in=1 adds 1)
        my ($result, $cout) = $self->_gate_add($acc, 0, 1);
        $self->_write_acc($result & 0xF); $self->_write_carry($cout); return "IAC";
    } elsif ($raw == 0xF3) {
        # CMC: complement carry using NOT gate
        $self->_write_carry(NOT($carry ? 1 : 0)); return "CMC";
    } elsif ($raw == 0xF4) {
        # CMA: complement accumulator using NOT gates
        $self->_write_acc($self->_gate_not4($acc)); return "CMA";
    } elsif ($raw == 0xF5) {
        # RAL: rotate accumulator left through carry (using bit array operations)
        my $bits      = _int_to_bits($acc, 4);
        my $old_carry = $carry ? 1 : 0;
        my $new_carry = $bits->[3];   # MSB (index 3 in LSB-first array)
        my @new_bits  = ($old_carry, $bits->[0], $bits->[1], $bits->[2]);
        $self->_write_acc(_bits_to_int(\@new_bits));
        $self->_write_carry($new_carry);
        return "RAL";
    } elsif ($raw == 0xF6) {
        # RAR: rotate accumulator right through carry
        my $bits      = _int_to_bits($acc, 4);
        my $old_carry = $carry ? 1 : 0;
        my $new_carry = $bits->[0];   # LSB goes to carry
        my @new_bits  = ($bits->[1], $bits->[2], $bits->[3], $old_carry);
        $self->_write_acc(_bits_to_int(\@new_bits));
        $self->_write_carry($new_carry);
        return "RAR";
    } elsif ($raw == 0xF7) {
        $self->_write_acc($carry ? 1 : 0); $self->_write_carry(0); return "TCC";
    } elsif ($raw == 0xF8) {
        # DAC: decrement via gate_add(A, NOT(1), 1) — but carry semantics
        # are acc > 0 (no borrow), not derived from the adder output.
        my ($result, undef) = $self->_gate_add($acc, $self->_gate_not4(1), 1);
        my $no_borrow = $acc > 0 ? 1 : 0;
        $self->_write_acc($result & 0xF); $self->_write_carry($no_borrow); return "DAC";
    } elsif ($raw == 0xF9) {
        $self->_write_acc($carry ? 10 : 9); $self->_write_carry(0); return "TCS";
    } elsif ($raw == 0xFA) {
        $self->_write_carry(1); return "STC";
    } elsif ($raw == 0xFB) {
        # DAA: BCD adjust using gate_add
        if ($acc > 9 || $carry) {
            my ($result, $cout) = $self->_gate_add($acc, 6, 0);
            my $new_carry = $cout ? 1 : ($carry ? 1 : 0);
            $self->_write_acc($result & 0xF);
            $self->_write_carry($new_carry);
        }
        return "DAA";
    } elsif ($raw == 0xFC) {
        my %kbp = (0=>0, 1=>1, 2=>2, 4=>3, 8=>4);
        $self->_write_acc(exists $kbp{$acc} ? $kbp{$acc} : 15); return "KBP";
    } elsif ($raw == 0xFD) {
        my $bank_bits = $acc & 0x7;
        $bank_bits = $bank_bits & 0x3 if $bank_bits > 3;
        $self->{ram_bank} = $bank_bits; return "DCL";
    }
    return sprintf "UNKNOWN(0x%02X)", $raw;
}

1;
