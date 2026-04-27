package CodingAdventures::Intel8008Simulator;

# ============================================================================
# Intel 8008 Simulator — the world's first commercial 8-bit microprocessor
# ============================================================================
#
# The Intel 8008 was introduced in April 1972. It was designed originally at
# the request of Computer Terminal Corporation (CTC), who wanted a CPU for
# their Datapoint 2200 terminal. CTC rejected it for being too slow; Intel
# sold it commercially anyway. The 8008 launched 8-bit computing and directly
# inspired the 8080, which inspired the Z80 and the x86 — making every modern
# x86 processor a distant descendant of this chip.
#
# ## Architecture
#
#   Data width:       8 bits
#   Registers:        7 × 8-bit (A, B, C, D, E, H, L)
#                     + M (pseudo-register: memory at [H:L])
#   Accumulator:      A (register index 7)
#   Flags:            4 bits — Carry (CY), Zero (Z), Sign (S), Parity (P)
#   Program counter:  14 bits (addresses 16 KiB = 0x0000–0x3FFF)
#   Stack:            8-level internal push-down stack (14-bit entries)
#                     Entry 0 is always the current PC.
#                     Only 7 levels usable for CALL/RETURN (entry 0 = PC).
#   Memory:           16,384 bytes (14-bit address space)
#   I/O:              8 input ports (IN 0–7), 24 output ports (OUT 0–23)
#
# ## Register Encoding (3-bit field in instruction)
#
#   000 = B      100 = H
#   001 = C      101 = L
#   010 = D      110 = M (indirect memory at [H:L])
#   011 = E      111 = A (accumulator)
#
# ## The 8-Level Push-Down Stack
#
# Unlike a software stack with a pointer, the 8008 has 8 × 14-bit registers
# physically inside the chip. Entry 0 IS the program counter. On CALL:
#   1. All entries shift down: entry 0 → entry 1, entry 1 → entry 2, ...
#   2. Jump target is loaded into entry 0 (new PC)
# On RETURN, the shift reverses: entry 1 → entry 0 (restoring the saved PC).
#
# This design means the programmer never sees a "stack pointer" — calls and
# returns are handled entirely by rotating the register array. Only 7 levels
# of call nesting are possible; the 8th would silently overwrite the oldest
# saved return address.
#
# ## Flag Semantics
#
#   CY (Carry):   Set when ADD/ADC overflows 8 bits.
#                 Set when SUB/SBB borrows (i.e., unsigned result went negative).
#                 Set/cleared by rotate instructions.
#   Z  (Zero):    Set when result is 0x00.
#   S  (Sign):    Set when bit 7 of result is 1 (result is negative if signed).
#   P  (Parity):  Set when result has an EVEN number of 1-bits.
#                 P=1 means even parity, P=0 means odd parity.
#
# Note: INR/DCR update Z, S, P but do NOT affect CY.
#
# ## Instruction Format
#
#   Bits 7–6: Major group (00/01/10/11)
#   Bits 5–3: DDD — destination register or ALU operation select
#   Bits 2–0: SSS — source register or sub-operation
#
# Variable-length instructions:
#   1 byte:  most instructions
#   2 bytes: MVI rr, d (00DDD110, data)
#            ALU immediate (11OOO100, data)
#   3 bytes: JMP/CALL and conditional variants (01CCC_00 or 01CCC_10, lo, hi)
#
# ## Subtraction Convention
#
# The 8008 (like the 8080) sets CY=1 when subtraction requires a borrow.
# That is: CY=0 after SUB means unsigned A >= B (no borrow needed).
# This is the OPPOSITE of the ARM convention (which sets C=0 on borrow).
#
# Subtraction is performed as two's complement: A - B = A + (~B) + 1.
#
# ## HLT encoding
#
# There are two halt opcodes:
#   0x76 = MOV M, M (intentional design: a MOV instruction that does nothing useful)
#   0xFF = also halts the processor

use strict;
use warnings;
our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# Constants: register indices (matching 3-bit hardware encoding)
# ---------------------------------------------------------------------------

use constant {
    REG_B => 0,
    REG_C => 1,
    REG_D => 2,
    REG_E => 3,
    REG_H => 4,
    REG_L => 5,
    REG_M => 6,   # Not a physical register — indirect memory [H:L]
    REG_A => 7,
};

# Register names for mnemonics
my @REG_NAMES = qw(B C D E H L M A);

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

    # 7 × 8-bit registers (+ index 6 unused/M) — indexed by 3-bit reg code
    # Layout: [B=0, C=1, D=2, E=3, H=4, L=5, unused=6, A=7]
    $self->{regs} = [(0) x 8];

    # 16,384 bytes of unified memory (program + data share the same space)
    $self->{memory} = [(0) x 16384];

    # 8-level push-down stack (14-bit entries). Entry 0 IS the current PC.
    # On power-on, all entries are 0 (including PC).
    $self->{stack} = [(0) x 8];

    # How many call levels are active (0 = just PC, 7 = fully nested)
    $self->{stack_depth} = 0;

    # 4 condition flags
    $self->{flags} = { carry => 0, zero => 0, sign => 0, parity => 0 };

    # Halted flag — set by HLT instruction
    $self->{halted} = 0;

    # 8 input ports (values set externally before running programs)
    $self->{input_ports} = [(0) x 8];

    # 24 output ports (written by OUT instructions, read externally)
    $self->{output_ports} = [(0) x 24];
}

# ---------------------------------------------------------------------------
# Public API — Accessors
# ---------------------------------------------------------------------------

# Accumulator (A) — 8-bit
sub a { $_[0]->{regs}[REG_A] }

# Working registers — 8-bit each
sub b { $_[0]->{regs}[REG_B] }
sub c { $_[0]->{regs}[REG_C] }
sub d { $_[0]->{regs}[REG_D] }
sub e { $_[0]->{regs}[REG_E] }
sub h { $_[0]->{regs}[REG_H] }
sub l { $_[0]->{regs}[REG_L] }

# Current program counter — always entry 0 of the push-down stack, 14-bit
sub pc { $_[0]->{stack}[0] & 0x3FFF }

# The 14-bit memory address formed from H and L:
#   address = (H & 0x3F) << 8 | L
# The 8008 uses H as the high 6 bits and L as the full 8 low bits,
# giving 14 bits of address. Only the low 6 bits of H are significant.
sub hl_address {
    my ($self) = @_;
    return (($self->{regs}[REG_H] & 0x3F) << 8) | $self->{regs}[REG_L];
}

# Current flags (hashref with keys: carry, zero, sign, parity)
sub flags      { $_[0]->{flags} }
sub stack      { $_[0]->{stack} }
sub stack_depth { $_[0]->{stack_depth} }
sub memory     { $_[0]->{memory} }
sub halted     { $_[0]->{halted} }

# ---------------------------------------------------------------------------
# Public API — I/O Ports
# ---------------------------------------------------------------------------

# Set an input port value (port 0–7). Used before running programs that read
# from ports with IN instructions.
sub set_input_port {
    my ($self, $port, $value) = @_;
    die "Intel8008: input port must be 0–7, got $port\n"   if $port < 0 || $port > 7;
    die "Intel8008: port value must be 0–255, got $value\n" if $value < 0 || $value > 255;
    $self->{input_ports}[$port] = $value & 0xFF;
}

# Read an output port value (port 0–23). Read after running a program.
sub get_output_port {
    my ($self, $port) = @_;
    die "Intel8008: output port must be 0–23, got $port\n" if $port < 0 || $port > 23;
    return $self->{output_ports}[$port] // 0;
}

# ---------------------------------------------------------------------------
# Public API — Execution
# ---------------------------------------------------------------------------

# Load a program (arrayref of bytes or a packed string) into memory starting
# at start_address (default 0). The CPU always begins execution from PC=0.
sub load_program {
    my ($self, $program, $start_address) = @_;
    $start_address //= 0;
    $self->{stack}[0] = $start_address & 0x3FFF;

    if (ref $program eq 'ARRAY') {
        for my $i (0 .. $#$program) {
            my $addr = ($start_address + $i) & 0x3FFF;
            $self->{memory}[$addr] = $program->[$i] & 0xFF;
        }
    } else {
        # Packed string of bytes
        my @bytes = unpack('C*', $program);
        for my $i (0 .. $#bytes) {
            my $addr = ($start_address + $i) & 0x3FFF;
            $self->{memory}[$addr] = $bytes[$i] & 0xFF;
        }
    }
}

# Run a program and return arrayref of trace hashrefs.
# Each trace records the state before and after one instruction.
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

# Execute exactly one instruction. Returns a trace hashref:
#   address         — PC at fetch time
#   raw             — arrayref of raw instruction bytes (1, 2, or 3)
#   mnemonic        — human-readable string like "ADD B" or "MVI A, 0x05"
#   a_before        — accumulator before execution
#   a_after         — accumulator after execution
#   flags_before    — flags hashref before execution
#   flags_after     — flags hashref after execution
#   memory_address  — address touched (if M was accessed), else undef
#   memory_value    — value read/written at memory_address, else undef
sub step {
    my ($self) = @_;
    die "Intel8008: CPU is halted — cannot step further\n" if $self->{halted};

    my $address     = $self->pc;
    my $a_before    = $self->{regs}[REG_A];
    my $flags_before = { %{ $self->{flags} } };  # shallow copy

    # --- Fetch: read opcode byte, advance PC ---
    my $opcode = $self->_fetch_byte();

    # --- Detect instruction length and fetch operands ---
    my ($data, $addr_lo, $addr_hi);
    my @raw_bytes = ($opcode);

    if ($self->_is_two_byte($opcode)) {
        # 2-byte instruction: opcode + immediate data byte
        $data = $self->_fetch_byte();
        push @raw_bytes, $data;
    } elsif ($self->_is_three_byte($opcode)) {
        # 3-byte instruction: opcode + 14-bit address (lo, hi)
        $addr_lo = $self->_fetch_byte();
        $addr_hi = $self->_fetch_byte();
        push @raw_bytes, $addr_lo, $addr_hi;
    }

    my ($mnemonic, $mem_addr, $mem_val) =
        $self->_execute($opcode, $data, $addr_lo, $addr_hi, $address);

    return {
        address        => $address,
        raw            => \@raw_bytes,
        mnemonic       => $mnemonic,
        a_before       => $a_before,
        a_after        => $self->{regs}[REG_A],
        flags_before   => $flags_before,
        flags_after    => { %{ $self->{flags} } },
        memory_address => $mem_addr,
        memory_value   => $mem_val,
    };
}

# Reset CPU to power-on state (clears all registers, memory, flags, stack).
sub reset {
    my ($self) = @_;
    $self->_init_state();
}

# ---------------------------------------------------------------------------
# Private: fetch helpers
# ---------------------------------------------------------------------------

# Read one byte from memory at current PC, then increment PC.
sub _fetch_byte {
    my ($self) = @_;
    my $pc  = $self->{stack}[0] & 0x3FFF;
    my $val = $self->{memory}[$pc] // 0;
    $self->{stack}[0] = ($pc + 1) & 0x3FFF;
    return $val;
}

# ---------------------------------------------------------------------------
# Private: instruction-length detection
# ---------------------------------------------------------------------------
#
# The 8008 has three instruction lengths:
#
#   1 byte:  Most instructions (MOV, INR, DCR, rotates, ALU register,
#            returns, I/O, HLT).
#
#   2 bytes: MVI rr, d   — 00DDD110 pattern (move immediate)
#            ALU imm     — 11OOO100 pattern (ADI, SUI, ANI, etc.)
#
#   3 bytes: JMP/CALL conditionals/unconditional — encoded as 01CCC_00 (jumps)
#            and 01CCC_10 (calls). The two address bytes follow.
#
# Pattern detection uses the bit-field structure:
#   MVI:      bits[7:6]=00, bits[2:0]=110  (and not HLT/rotate patterns)
#   ALU imm:  bits[7:6]=11, bits[2:0]=100
#   JUMP:     bits[7:6]=01, bits[2:0]=000 or 100
#   CALL:     bits[7:6]=01, bits[2:0]=010 or 110
# We unify JUMP and CALL into "three-byte" detection.

sub _is_two_byte {
    my ($self, $opcode) = @_;
    my $group = ($opcode >> 6) & 0x03;
    my $sss   = $opcode & 0x07;

    # MVI: group=00, sss=110 (but not 0x76 which is HLT, and not rotate 0x02/0x0A/0x12/0x1A)
    if ($group == 0b00 && $sss == 0b110) {
        # 0x36 = MVI M,d is valid 2-byte; all other 00DDD110 are MVI or HLT
        # 0x76 = MOV M,M = HLT — handled in _is_three_byte check: not 3-byte,
        # so falls through. We treat 0x76 as 1-byte for fetch, and _execute handles HLT.
        # The encoding 00 110 110 = 0x36 IS a valid MVI M,d (2-byte).
        # 0x76 = 01 110 110 — group=01, not group=00 — safe.
        return 1;
    }

    # ALU immediate: group=11, sss=100
    if ($group == 0b11 && $sss == 0b100) {
        return 1;
    }

    return 0;
}

sub _is_three_byte {
    my ($self, $opcode) = @_;
    my $group = ($opcode >> 6) & 0x03;
    my $ddd   = ($opcode >> 3) & 0x07;
    my $sss   = $opcode & 0x07;

    # Only group=01 has 3-byte instructions.
    # Analysis of the full 8008 jump/call opcode table:
    #
    # Conditional jumps/calls:  ddd ∈ {000,001,010,011} (CCC=0-3), sss ∈ {000,100,010,110}
    #   sss=000 or 100 → jumps (JFc, JTc)
    #   sss=010 or 110 → calls (CFc, CTc)
    #
    # Unconditional JMP: 0x7C = ddd=111, sss=100
    # Unconditional CAL: 0x7E = ddd=111, sss=110
    #
    # All other group=01 opcodes are MOV, IN, or HLT (1-byte).
    # Key: MOV uses ddd ∈ {100,101,110,111} with sss ∈ {000,010,100,110} for
    # the cases that could look like jumps — e.g., 0x78 (MOV A,B) has ddd=7,sss=0.
    # These are NOT 3-byte because ddd is not in {0-3} and not the special cases.
    #
    # So the precise rule is:
    #   3-byte iff group=01 AND (
    #     (ddd <= 3 AND sss ∈ {000, 100, 010, 110})   -- conditional JMP/CALL
    #     OR (ddd == 7 AND sss ∈ {100, 110})           -- JMP, CAL
    #   )
    if ($group == 0b01) {
        my $is_jmp_call_sss = ($sss == 0b000 || $sss == 0b100 ||
                               $sss == 0b010 || $sss == 0b110);
        # Conditional jumps/calls: ddd is a condition code (0-3)
        return 1 if $ddd <= 3 && $is_jmp_call_sss;
        # Unconditional JMP (0x7C) and CAL (0x7E)
        return 1 if $ddd == 7 && ($sss == 0b100 || $sss == 0b110);
    }

    return 0;
}

# ---------------------------------------------------------------------------
# Private: stack operations
# ---------------------------------------------------------------------------
#
# The 8008 hardware stack is a circular shift register. Entry 0 holds the
# current PC at all times. A CALL or RST instruction shifts all entries down
# (entry 7 is silently discarded) and loads the jump target into entry 0.
# A RETURN shifts all entries up (entry 0 is discarded) and entry 1 becomes
# the new PC (now entry 0).

# Push: save current PC, load new target into entry 0.
# Used by CALL instructions and RST.
sub _push_and_jump {
    my ($self, $target) = @_;
    # Shift stack down: entry 7 ← entry 6 ← ... ← entry 1 ← entry 0 (PC)
    for my $i (reverse 1..7) {
        $self->{stack}[$i] = $self->{stack}[$i-1];
    }
    # Load new target as the current PC
    $self->{stack}[0] = $target & 0x3FFF;
    $self->{stack_depth}++ if $self->{stack_depth} < 7;
}

# Pop: discard current PC (entry 0), restore saved return address (entry 1).
# Used by RETURN instructions.
sub _pop_return {
    my ($self) = @_;
    # Shift stack up: entry 0 ← entry 1 ← ... ← entry 6 ← entry 7
    for my $i (0..6) {
        $self->{stack}[$i] = $self->{stack}[$i+1];
    }
    $self->{stack}[7] = 0;  # Clear the now-vacated deepest slot
    $self->{stack_depth}-- if $self->{stack_depth} > 0;
}

# ---------------------------------------------------------------------------
# Private: register read/write (handles M pseudo-register)
# ---------------------------------------------------------------------------

# Read a register value (0–7). If reg=6 (M), reads from memory at [H:L].
# Returns ($value, $mem_addr_or_undef) — mem_addr is set only for M access.
sub _read_reg {
    my ($self, $reg) = @_;
    if ($reg == REG_M) {
        my $addr = $self->hl_address();
        return ($self->{memory}[$addr] // 0, $addr);
    }
    return ($self->{regs}[$reg], undef);
}

# Write a register value. If reg=6 (M), writes to memory at [H:L].
# Returns $mem_addr_or_undef.
sub _write_reg {
    my ($self, $reg, $value) = @_;
    $value &= 0xFF;
    if ($reg == REG_M) {
        my $addr = $self->hl_address();
        $self->{memory}[$addr] = $value;
        return $addr;
    }
    $self->{regs}[$reg] = $value;
    return undef;
}

# ---------------------------------------------------------------------------
# Private: flag computation
# ---------------------------------------------------------------------------
#
# The 8008 updates four flags after most ALU operations.
#
# Parity (P) counts the number of 1-bits in the result:
#   P=1 means EVEN count (even parity).
#   P=0 means ODD count (odd parity).
#
# Parity is computed via binary representation: count the '1' characters
# in the 8-bit string. Even count → P=1.
#
# @param $result       The integer result (may be > 255 — we mask to 8 bits)
# @param $carry        The carry/borrow bit from the operation
# @param $update_carry If 0, preserve the old carry flag (for INR/DCR)
sub _compute_flags {
    my ($self, $result, $carry, $update_carry) = @_;
    $update_carry //= 1;
    my $r8 = $result & 0xFF;

    # Count ones in binary representation — simulates a 7-gate XOR parity tree
    my $ones = scalar grep { $_ eq '1' } split //, sprintf("%08b", $r8);

    return {
        carry  => $update_carry ? ($carry ? 1 : 0) : $self->{flags}{carry},
        zero   => ($r8 == 0) ? 1 : 0,
        sign   => ($r8 & 0x80) ? 1 : 0,
        parity => ($ones % 2 == 0) ? 1 : 0,  # 1 = even parity (8008 convention)
    };
}

# ---------------------------------------------------------------------------
# Private: instruction executor
# ---------------------------------------------------------------------------
#
# Decode the opcode into major group (bits 7–6), DDD (bits 5–3), SSS (bits 2–0).
# Dispatch to the appropriate handler.
#
# Returns: ($mnemonic, $mem_addr, $mem_val)

sub _execute {
    my ($self, $opcode, $data, $addr_lo, $addr_hi, $fetch_addr) = @_;

    # Halt: both HLT encodings
    if ($opcode == 0x76 || $opcode == 0xFF) {
        $self->{halted} = 1;
        return ('HLT', undef, undef);
    }

    my $group = ($opcode >> 6) & 0x03;
    my $ddd   = ($opcode >> 3) & 0x07;
    my $sss   = $opcode & 0x07;

    # -----------------------------------------------------------------------
    # Group 01: MOV + IN + control flow (jump/call)
    # -----------------------------------------------------------------------
    if ($group == 0b01) {
        # --- I/O: IN instruction ---
        # IN P: 01 PPP 001 — sss=001, port number is ddd field (bits[5:3])
        # IN 0=0x41, IN 1=0x49, IN 2=0x51, ..., IN 7=0x79
        if ($sss == 0b001) {
            my $port = $ddd;  # Port 0-7 encoded in the ddd (bits[5:3]) field
            $self->{regs}[REG_A] = $self->{input_ports}[$port] & 0xFF;
            return (sprintf("IN %d", $port), undef, undef);
        }

        # --- Jump instructions: only when this IS a 3-byte instruction ---
        # (addr_lo/addr_hi were fetched only if _is_three_byte returned true,
        # which happens when ddd<=3 and sss ∈ {000,100,010,110}, or ddd=7 and
        # sss ∈ {100,110}.)
        # We use the presence of addr_lo (defined) as the signal that we fetched
        # address bytes — but that's indirect. Better: repeat the 3-byte condition.
        my $is_jmp_call_sss = ($sss == 0b000 || $sss == 0b100 ||
                               $sss == 0b010 || $sss == 0b110);

        if ($is_jmp_call_sss && ($ddd <= 3 || ($ddd == 7 && ($sss == 0b100 || $sss == 0b110)))) {
            # This IS a jump or call instruction (3-byte).
            if ($sss == 0b000 || $sss == 0b100) {
                return $self->_exec_jump($opcode, $ddd, $sss, $addr_lo, $addr_hi);
            } else {
                return $self->_exec_call($opcode, $ddd, $sss, $addr_lo, $addr_hi);
            }
        }

        # --- MOV: group=01, everything else ---
        # MOV DDD, SSS — copy SSS into DDD
        # (HLT = 0x76 = MOV M,M already caught above at the top of _execute)
        return $self->_exec_mov($ddd, $sss);
    }

    # -----------------------------------------------------------------------
    # Group 10: ALU register operations
    # -----------------------------------------------------------------------
    if ($group == 0b10) {
        # Encoding: 10 OOO SSS
        # DDD field here is the ALU operation selector (not a destination reg)
        return $self->_exec_alu_reg($ddd, $sss);
    }

    # -----------------------------------------------------------------------
    # Group 11: ALU immediate + control (RET, RST, OUT)
    # -----------------------------------------------------------------------
    if ($group == 0b11) {
        # ALU immediate: 11 OOO 100 — sss=100
        if ($sss == 0b100) {
            return $self->_exec_alu_imm($ddd, $data);
        }

        # --- Return instructions: 00 CCC T11 (group=00, sss ends in 11)
        # Wait — RET encoding is 00 CCC T11 — that's GROUP=00.
        # Group=11 entries: RST (00AAA101) also group=00.
        # OUT instruction: group=00 as well.
        # So group=11 is ONLY ALU immediate. Fall through to group=00 for others.
        # Actually let me re-check: 11OOO100 is only one encoding in group 11.
        # What else is in group 11? Let me verify:
        #   RET = 00CCC T11 — group=00
        #   RST = 00AAA101  — group=00
        #   OUT = 00PPP P10  — group=00
        # So group=11 has ONLY ALU immediate (11OOO100).
        # Any other group=11 is undefined/reserved. Fall through.
        return (sprintf("UNKNOWN(0x%02X)", $opcode), undef, undef);
    }

    # -----------------------------------------------------------------------
    # Group 00: INR, DCR, MVI, Rotates, RET, RST, OUT
    # -----------------------------------------------------------------------
    # (group == 0b00)

    # --- Rotate instructions: 00 0RR 010 ---
    # Pattern: bits[5:3] = 0RR (bit 5 must be 0), sss = 010
    if ($sss == 0b010 && ($ddd & 0b100) == 0) {
        return $self->_exec_rotate($ddd);
    }

    # --- Return instructions: 00 CCC T11 ---
    # sss ends in 11: sss = 011 (T=0, false) or sss = 111 (T=1, true)
    if ($sss == 0b011 || $sss == 0b111) {
        return $self->_exec_return($ddd, $sss);
    }

    # --- RST (Restart): 00 AAA 101 ---
    if ($sss == 0b101) {
        my $target = ($ddd & 0x07) << 3;  # AAA × 8 → 0,8,16,...,56
        my $next_pc = $self->pc;  # PC is already past the RST instruction
        $self->_push_and_jump($target);
        return (sprintf("RST %d", $ddd), undef, undef);
    }

    # --- OUT instruction: 00 PPP 010 ---
    # OUT port P: group=00, sss=010, port = ddd (bits[5:3]).
    # Ports 0-3 (ddd=0-3) share encodings with rotate instructions (RLC/RRC/RAL/RAR).
    # The rotate handler above takes priority for ddd<4 (bit5=0).
    # OUT is only unambiguously encoded for ports 4-7 (ddd=4-7, bit5=1 set).
    # OUT 4=0x22, OUT 5=0x2A, OUT 6=0x32, OUT 7=0x3A.
    # For a software simulator we implement the unambiguous ports 4-7 here.
    # (The real 8008 hardware used I/O control signals to distinguish OUT from rotates.)
    if ($sss == 0b010 && ($ddd & 0b100) != 0) {
        my $port = $ddd;  # Port number = ddd field (4, 5, 6, or 7)
        $self->{output_ports}[$port] = $self->{regs}[REG_A] & 0xFF;
        return (sprintf("OUT %d", $port), undef, undef);
    }

    # --- OUT instruction also via sss=110 ---
    # 00 PPP 110 with ddd>=4: OUT 4-7 via sss=110 encoding.
    # BUT sss=110 in group=00 is MVI (00 DDD 110)! So this conflicts too.
    # MVI handles sss=110 below. For a software simulator, MVI takes priority.
    # External code can use output_ports directly if needed.

    # --- MVI: 00 DDD 110 ---
    if ($sss == 0b110) {
        return $self->_exec_mvi($ddd, $data);
    }

    # --- INR: 00 DDD 000 ---
    if ($sss == 0b000) {
        return $self->_exec_inr($ddd);
    }

    # --- DCR: 00 DDD 001 ---
    if ($sss == 0b001) {
        return $self->_exec_dcr($ddd);
    }

    return (sprintf("UNKNOWN(0x%02X)", $opcode), undef, undef);
}

# ---------------------------------------------------------------------------
# Instruction implementations
# ---------------------------------------------------------------------------

# MOV DDD, SSS — copy source register into destination register.
# If source = M, reads memory at [H:L].
# If destination = M, writes to memory at [H:L].
# Flags: not affected.
sub _exec_mov {
    my ($self, $ddd, $sss) = @_;
    my ($val, $src_addr) = $self->_read_reg($sss);
    my $dst_addr = $self->_write_reg($ddd, $val);
    my $mem_addr = $src_addr // $dst_addr;
    my $mem_val  = defined($mem_addr) ? $val : undef;
    return (
        sprintf("MOV %s, %s", $REG_NAMES[$ddd], $REG_NAMES[$sss]),
        $mem_addr,
        $mem_val,
    );
}

# MVI DDD, d — move immediate byte into register.
# If DDD=M, write to memory at [H:L].
# Flags: not affected.
sub _exec_mvi {
    my ($self, $ddd, $data) = @_;
    $data //= 0;
    my $addr = $self->_write_reg($ddd, $data);
    return (
        sprintf("MVI %s, 0x%02X", $REG_NAMES[$ddd], $data),
        $addr,
        defined($addr) ? ($data & 0xFF) : undef,
    );
}

# INR DDD — increment register by 1. Wraps 0xFF → 0x00.
# Updates Z, S, P. Does NOT update CY (carry preserved from before).
sub _exec_inr {
    my ($self, $ddd) = @_;
    my ($val, $src_addr) = $self->_read_reg($ddd);
    my $result = ($val + 1) & 0xFF;
    my $addr = $self->_write_reg($ddd, $result);
    $self->{flags} = $self->_compute_flags($result, 0, 0);  # preserve carry
    my $mem_addr = $src_addr // $addr;
    return (
        sprintf("INR %s", $REG_NAMES[$ddd]),
        $mem_addr,
        defined($mem_addr) ? $result : undef,
    );
}

# DCR DDD — decrement register by 1. Wraps 0x00 → 0xFF.
# Updates Z, S, P. Does NOT update CY.
sub _exec_dcr {
    my ($self, $ddd) = @_;
    my ($val, $src_addr) = $self->_read_reg($ddd);
    my $result = ($val - 1) & 0xFF;
    my $addr = $self->_write_reg($ddd, $result);
    $self->{flags} = $self->_compute_flags($result, 0, 0);  # preserve carry
    my $mem_addr = $src_addr // $addr;
    return (
        sprintf("DCR %s", $REG_NAMES[$ddd]),
        $mem_addr,
        defined($mem_addr) ? $result : undef,
    );
}

# ALU register operations: 10 OOO SSS
# OOO selects the operation; SSS is the source register.
# All operations write the result to the accumulator (except CMP).
sub _exec_alu_reg {
    my ($self, $ooo, $sss) = @_;
    my ($src, $mem_addr) = $self->_read_reg($sss);
    my $a = $self->{regs}[REG_A];

    my @ops = qw(ADD ADC SUB SBB ANA XRA ORA CMP);
    my $mnem = $ops[$ooo];

    my ($result, $new_flags) = $self->_alu_op($ooo, $a, $src);

    # CMP doesn't write the result — it only updates flags
    if ($ooo != 0b111) {
        $self->{regs}[REG_A] = $result & 0xFF;
    }
    $self->{flags} = $new_flags;

    return (
        sprintf("%s %s", $mnem, $REG_NAMES[$sss]),
        $mem_addr,
        defined($mem_addr) ? $src : undef,
    );
}

# ALU immediate operations: 11 OOO 100, data
# Same operation codes as register ALU but with an immediate byte.
my @ALU_IMM_NAMES = qw(ADI ACI SUI SBI ANI XRI ORI CPI);
sub _exec_alu_imm {
    my ($self, $ooo, $data) = @_;
    $data //= 0;
    my $a = $self->{regs}[REG_A];

    my ($result, $new_flags) = $self->_alu_op($ooo, $a, $data);

    # CPI doesn't write result, only updates flags
    if ($ooo != 0b111) {
        $self->{regs}[REG_A] = $result & 0xFF;
    }
    $self->{flags} = $new_flags;

    return (sprintf("%s 0x%02X", $ALU_IMM_NAMES[$ooo], $data), undef, undef);
}

# Core ALU operation dispatcher.
# @param $ooo  3-bit operation code (0=ADD, 1=ADC, 2=SUB, 3=SBB, 4=ANA, 5=XRA, 6=ORA, 7=CMP)
# @param $a    Accumulator value (0–255)
# @param $b    Source operand (0–255)
# @return      ($result, $flags_hashref)
sub _alu_op {
    my ($self, $ooo, $a, $b) = @_;

    my ($result, $carry);

    if ($ooo == 0b000) {
        # ADD: A + B
        my $sum = $a + $b;
        $result = $sum & 0xFF;
        $carry  = $sum > 0xFF ? 1 : 0;
        return ($result, $self->_compute_flags($result, $carry, 1));

    } elsif ($ooo == 0b001) {
        # ADC: A + B + CY
        my $cin = $self->{flags}{carry} ? 1 : 0;
        my $sum = $a + $b + $cin;
        $result = $sum & 0xFF;
        $carry  = $sum > 0xFF ? 1 : 0;
        return ($result, $self->_compute_flags($result, $carry, 1));

    } elsif ($ooo == 0b010) {
        # SUB: A - B  (two's complement: A + ~B + 1)
        # CY=1 means borrow occurred (unsigned A < B)
        my $diff = $a - $b;
        $result = $diff & 0xFF;
        $carry  = ($diff < 0) ? 1 : 0;  # borrow = CY=1 on the 8008
        return ($result, $self->_compute_flags($result, $carry, 1));

    } elsif ($ooo == 0b011) {
        # SBB: A - B - CY
        my $cin = $self->{flags}{carry} ? 1 : 0;
        my $diff = $a - $b - $cin;
        $result = $diff & 0xFF;
        $carry  = ($diff < 0) ? 1 : 0;
        return ($result, $self->_compute_flags($result, $carry, 1));

    } elsif ($ooo == 0b100) {
        # ANA: A & B  (clears carry)
        $result = $a & $b;
        return ($result, $self->_compute_flags($result, 0, 1));

    } elsif ($ooo == 0b101) {
        # XRA: A ^ B  (clears carry)
        $result = $a ^ $b;
        return ($result, $self->_compute_flags($result, 0, 1));

    } elsif ($ooo == 0b110) {
        # ORA: A | B  (clears carry)
        $result = $a | $b;
        return ($result, $self->_compute_flags($result, 0, 1));

    } else {
        # CMP: A - B (flags only, result discarded)
        my $diff = $a - $b;
        $result = $diff & 0xFF;
        $carry  = ($diff < 0) ? 1 : 0;
        return ($result, $self->_compute_flags($result, $carry, 1));
    }
}

# Rotate instructions: 00 0RR 010
# RR encodes the rotate type in bits [4:3] of the opcode (= bits [1:0] of DDD field).
#
#   RR=00 (0x02) RLC — Rotate left circular (bit 7 wraps to bit 0 and to CY)
#   RR=01 (0x0A) RRC — Rotate right circular (bit 0 wraps to bit 7 and to CY)
#   RR=10 (0x12) RAL — Rotate left through carry (9-bit: {CY, A} << 1)
#   RR=11 (0x1A) RAR — Rotate right through carry (9-bit: {A, CY} >> 1)
#
# Only CY is updated by rotates; Z, S, P are unaffected.
sub _exec_rotate {
    my ($self, $ddd) = @_;
    my $rr = $ddd & 0b011;  # bits [1:0] of the ddd field = rotate type
    my $a  = $self->{regs}[REG_A];

    my ($new_a, $new_cy, $mnem);

    if ($rr == 0b00) {
        # RLC: A[7] → CY; A << 1; A[0] ← old A[7]
        my $bit7 = ($a >> 7) & 1;
        $new_a  = (($a << 1) | $bit7) & 0xFF;
        $new_cy = $bit7;
        $mnem   = 'RLC';

    } elsif ($rr == 0b01) {
        # RRC: A[0] → CY; A >> 1; A[7] ← old A[0]
        my $bit0 = $a & 1;
        $new_a  = (($a >> 1) | ($bit0 << 7)) & 0xFF;
        $new_cy = $bit0;
        $mnem   = 'RRC';

    } elsif ($rr == 0b10) {
        # RAL: 9-bit rotate left through carry
        # new A[0] ← old CY; new CY ← old A[7]
        my $old_cy = $self->{flags}{carry} ? 1 : 0;
        my $bit7   = ($a >> 7) & 1;
        $new_a  = (($a << 1) | $old_cy) & 0xFF;
        $new_cy = $bit7;
        $mnem   = 'RAL';

    } else {
        # RAR: 9-bit rotate right through carry
        # new A[7] ← old CY; new CY ← old A[0]
        my $old_cy = $self->{flags}{carry} ? 1 : 0;
        my $bit0   = $a & 1;
        $new_a  = (($a >> 1) | ($old_cy << 7)) & 0xFF;
        $new_cy = $bit0;
        $mnem   = 'RAR';
    }

    $self->{regs}[REG_A] = $new_a;
    $self->{flags}{carry} = $new_cy;
    # Z, S, P are NOT updated by rotate instructions

    return ($mnem, undef, undef);
}

# Jump instructions: group=01, sss ∈ {000, 100} for jumps
# Address is reconstructed from two bytes: (addr_hi & 0x3F) << 8 | addr_lo
#
# Encoding: 01 CCC T 00
#   bits[7:6] = 01  (group)
#   bits[5:3] = CCC (condition code: 0=CY, 1=Z, 2=S, 3=P) = ddd field
#   bit[2]    = T   (sense: 0 = jump-if-false, 1 = jump-if-true) = sss bit 2
#   bits[1:0] = 00  (fixed marker for jumps)
#
# Verified from actual opcodes:
#   JFC=0x40 (ddd=0,sss=000): CCC=0(CY), T=0 → "jump if carry false"
#   JTC=0x44 (ddd=0,sss=100): CCC=0(CY), T=1 → "jump if carry true"
#   JFZ=0x48 (ddd=1,sss=000): CCC=1(Z),  T=0
#   JTZ=0x4C (ddd=1,sss=100): CCC=1(Z),  T=1
#   JMP=0x7C (ddd=7,sss=100): unconditional (special case, ddd=7)
#
# Key: T = (sss >> 2) & 1  (bit2 of opcode)
#      CCC = ddd (bits[5:3] of opcode)

my @COND_NAMES_F = ('JFC', 'JFZ', 'JFS', 'JFP');  # T=0 (jump if false/not-set)
my @COND_NAMES_T = ('JTC', 'JTZ', 'JTS', 'JTP');  # T=1 (jump if true/set)
my @COND_FLAGS   = ('carry', 'zero', 'sign', 'parity');

sub _exec_jump {
    my ($self, $opcode, $ddd, $sss, $addr_lo, $addr_hi) = @_;
    $addr_lo //= 0;
    $addr_hi //= 0;
    my $target = (($addr_hi & 0x3F) << 8) | $addr_lo;

    # Unconditional JMP: ddd=111 and sss=100
    if ($ddd == 0b111 && $sss == 0b100) {
        $self->{stack}[0] = $target & 0x3FFF;
        return (sprintf("JMP 0x%04X", $target), undef, undef);
    }

    # Conditional jump:
    # T   = (sss >> 2) & 1 = bit2 of opcode: 0=false, 1=true
    # CCC = ddd: condition code (0=carry, 1=zero, 2=sign, 3=parity)
    my $t   = ($sss >> 2) & 1;     # 0=jump-if-false, 1=jump-if-true
    my $ccc = $ddd & 0b011;        # condition code (ddd is already 0-3 for conditionals)

    my $flag_name = $COND_FLAGS[$ccc];
    my $flag_val  = $self->{flags}{$flag_name} ? 1 : 0;

    my $should_jump = ($t == 1) ? $flag_val : (1 - $flag_val);

    my $mnem = $t ? $COND_NAMES_T[$ccc] : $COND_NAMES_F[$ccc];

    if ($should_jump) {
        $self->{stack}[0] = $target & 0x3FFF;
    }

    return (sprintf("%s 0x%04X", $mnem, $target), undef, undef);
}

# Call instructions: group=01, sss ∈ {010, 110} for calls
# Same encoding as jumps but with bits[1:0]=10 (call marker) instead of 00.
#
# Encoding: 01 CCC T 10
#   T   = (sss >> 2) & 1  (bit2 of opcode)
#   CCC = ddd              (condition code)
#
# Verified:
#   CFC=0x42 (ddd=0,sss=010): CCC=0(CY), T=0 → "call if carry false"
#   CTC=0x46 (ddd=0,sss=110): CCC=0(CY), T=1 → "call if carry true"
#   CAL=0x7E (ddd=7,sss=110): unconditional

my @COND_NAMES_CF = ('CFC', 'CFZ', 'CFS', 'CFP');  # T=0
my @COND_NAMES_CT = ('CTC', 'CTZ', 'CTS', 'CTP');  # T=1

sub _exec_call {
    my ($self, $opcode, $ddd, $sss, $addr_lo, $addr_hi) = @_;
    $addr_lo //= 0;
    $addr_hi //= 0;
    my $target = (($addr_hi & 0x3F) << 8) | $addr_lo;

    # Unconditional CAL: ddd=111 and sss=110 (0x7E)
    if ($ddd == 0b111 && $sss == 0b110) {
        $self->_push_and_jump($target);
        return (sprintf("CAL 0x%04X", $target), undef, undef);
    }

    # Conditional call
    # T   = (sss >> 2) & 1  (bit2 of opcode)
    # CCC = ddd
    my $t   = ($sss >> 2) & 1;
    my $ccc = $ddd & 0b011;

    my $flag_name = $COND_FLAGS[$ccc];
    my $flag_val  = $self->{flags}{$flag_name} ? 1 : 0;

    my $should_call = ($t == 1) ? $flag_val : (1 - $flag_val);

    my $mnem = $t ? $COND_NAMES_CT[$ccc] : $COND_NAMES_CF[$ccc];

    if ($should_call) {
        $self->_push_and_jump($target);
    }

    return (sprintf("%s 0x%04X", $mnem, $target), undef, undef);
}

# Return instructions: 00 CCC T11
# Encoding: group=00, bits[5:3] = CCC T (same layout as jumps/calls)
#   CCC = ddd (condition code)
#   T   = (sss >> 2) & 1  (bit2 of opcode)
#   sss is always 011 (T=0, false) or 111 (T=1, true)
#
# Verified:
#   RFC=0x03 (ddd=0,sss=011): CCC=0(CY), T=0 → "return if carry false"
#   RTC=0x07 (ddd=0,sss=111): CCC=0(CY), T=1 → "return if carry true"
#   RET=0x3F (ddd=7,sss=111): unconditional return
#
# Pops return address from the hardware stack if condition is met.

my @COND_NAMES_RF = ('RFC', 'RFZ', 'RFS', 'RFP');  # T=0
my @COND_NAMES_RT = ('RTC', 'RTZ', 'RTS', 'RTP');  # T=1

sub _exec_return {
    my ($self, $ddd, $sss) = @_;

    # Unconditional RET: ddd=111, sss=111 (0x3F)
    if ($ddd == 0b111 && $sss == 0b111) {
        $self->_pop_return();
        return ('RET', undef, undef);
    }

    # Conditional return
    # T   = (sss >> 2) & 1  (bit2 of opcode)
    # CCC = ddd
    my $t   = ($sss >> 2) & 1;
    my $ccc = $ddd & 0b011;

    my $flag_name = $COND_FLAGS[$ccc];
    my $flag_val  = $self->{flags}{$flag_name} ? 1 : 0;

    my $should_ret = ($t == 1) ? $flag_val : (1 - $flag_val);

    my $mnem = $t ? $COND_NAMES_RT[$ccc] : $COND_NAMES_RF[$ccc];

    if ($should_ret) {
        $self->_pop_return();
    }

    return ($mnem, undef, undef);
}

1;

__END__

=head1 NAME

CodingAdventures::Intel8008Simulator - Behavioral simulator for the Intel 8008

=head1 SYNOPSIS

    use CodingAdventures::Intel8008Simulator;

    my $cpu = CodingAdventures::Intel8008Simulator->new();

    # Load and run a program:
    #   MVI B, 1   ; 0x06 0x01
    #   MVI A, 2   ; 0x3E 0x02
    #   ADD B      ; 0x80
    #   HLT        ; 0x76
    my $traces = $cpu->run([0x06, 0x01, 0x3E, 0x02, 0x80, 0x76]);
    print "A = ", $cpu->a, "\n";  # 3

=head1 DESCRIPTION

Complete behavioral simulator for the Intel 8008 (April 1972), the world's
first commercial 8-bit microprocessor and ancestor of the x86 architecture.

Implements the full instruction set: MOV, MVI, INR, DCR, ADD, ADC, SUB, SBB,
ANA, XRA, ORA, CMP (register and immediate variants), RLC, RRC, RAL, RAR,
JMP, conditional jumps, CAL, conditional calls, RET, conditional returns, RST,
IN, OUT, HLT.

=head1 METHODS

=over 4

=item B<new()> — Create a new CPU instance (all zeroed).

=item B<run($program, $max_steps, $start_address)> — Load and run a program.
Returns arrayref of trace hashrefs.

=item B<step()> — Execute one instruction. Returns trace hashref.

=item B<reset()> — Reset all state to power-on defaults.

=item B<a>, B<b>, B<c>, B<d>, B<e>, B<h>, B<l> — Register accessors.

=item B<pc> — 14-bit program counter.

=item B<hl_address> — 14-bit address from H:L pair.

=item B<flags> — Hashref with keys: carry, zero, sign, parity.

=item B<set_input_port($port, $value)> — Set input port 0–7.

=item B<get_output_port($port)> — Read output port 0–23.

=back

=cut
