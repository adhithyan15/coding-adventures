package CodingAdventures::ClrSimulator;

# ============================================================================
# CodingAdventures::ClrSimulator — CLR IL bytecode simulator in Pure Perl
# ============================================================================
#
# # What is the CLR?
#
# The Common Language Runtime (CLR) is Microsoft's managed execution engine,
# introduced in 2002 with .NET Framework. It runs C#, F#, VB.NET, PowerShell,
# and many other languages. Like the JVM, it is a **stack-based virtual
# machine** — instructions operate on an operand stack.
#
# # CLR vs JVM: Type-Neutral vs Typed Opcodes
#
# The JVM has one arithmetic opcode per type: `iadd` for int, `ladd` for long,
# `fadd` for float. The CLR takes a different approach: just `add` — the
# runtime infers the type from what is on the stack.
#
#   JVM:    iconst_1  iconst_2  iadd     ← type embedded in opcode name
#   CLR:    ldc.i4.1  ldc.i4.2  add      ← type inferred from stack contents
#
# This means CLR bytecode is more compact (fewer distinct opcodes), but the VM
# must track type information on the stack at runtime.
#
# # Bytecode Encoding
#
# CLR bytecode is a sequence of variable-width bytes. Most instructions are
# 1 byte. Some have 1-byte or 4-byte operands. The compare instructions
# (ceq, cgt, clt) use a 2-byte opcode: 0xFE followed by a second byte.
#
# # Short Branch Offsets
#
# The `.s` suffix on branch instructions means "short" — the offset is a
# signed 8-bit integer (-128 to +127). The offset is relative to the
# instruction *after* the branch (next_pc = pc + width_of_branch_instruction).
#
# # Local Variables
#
# Each method has an array of local variable "slots." The first four slots
# have dedicated 1-byte opcodes (stloc.0-3, ldloc.0-3). Slots 4 and above
# use the `stloc.s N` and `ldloc.s N` two-byte forms.
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# Opcode Constants (real CLR IL hex values)
# ============================================================================

use constant {
    NOP        => 0x00,   # no operation
    LDNULL     => 0x01,   # push null
    LDLOC_0    => 0x06,   # push local[0]
    LDLOC_1    => 0x07,   # push local[1]
    LDLOC_2    => 0x08,   # push local[2]
    LDLOC_3    => 0x09,   # push local[3]
    STLOC_0    => 0x0A,   # pop → local[0]
    STLOC_1    => 0x0B,   # pop → local[1]
    STLOC_2    => 0x0C,   # pop → local[2]
    STLOC_3    => 0x0D,   # pop → local[3]
    LDLOC_S    => 0x11,   # push local[N] (1-byte operand)
    STLOC_S    => 0x13,   # pop → local[N] (1-byte operand)
    LDC_I4_0   => 0x16,   # push 0
    LDC_I4_1   => 0x17,   # push 1
    LDC_I4_2   => 0x18,   # push 2
    LDC_I4_3   => 0x19,   # push 3
    LDC_I4_4   => 0x1A,   # push 4
    LDC_I4_5   => 0x1B,   # push 5
    LDC_I4_6   => 0x1C,   # push 6
    LDC_I4_7   => 0x1D,   # push 7
    LDC_I4_8   => 0x1E,   # push 8
    LDC_I4_S   => 0x1F,   # push signed int8 (1-byte operand)
    LDC_I4     => 0x20,   # push int32 (4-byte little-endian operand)
    RET        => 0x2A,   # return (halts simulator)
    BR_S       => 0x2B,   # unconditional short branch (1-byte offset)
    BRFALSE_S  => 0x2C,   # branch if 0/null (short)
    BRTRUE_S   => 0x2D,   # branch if non-0/non-null (short)
    ADD        => 0x58,   # pop b, pop a, push a+b
    SUB        => 0x59,   # pop b, pop a, push a-b
    MUL        => 0x5A,   # pop b, pop a, push a*b
    DIV        => 0x5B,   # pop b, pop a, push trunc(a/b)
    PREFIX_FE  => 0xFE,   # two-byte opcode prefix (for ceq/cgt/clt)
    CEQ_BYTE   => 0x01,   # ceq second byte
    CGT_BYTE   => 0x02,   # cgt second byte
    CLT_BYTE   => 0x04,   # clt second byte
};

# ============================================================================
# Constructor
# ============================================================================

# ----------------------------------------------------------------------------
# new() → ClrSimulator instance
#
# Creates a new CLR simulator. The simulator is a mutable object — each
# step() call updates the simulator's state in-place.
#
# Fields:
#   $self->{stack}    — operand stack (array ref, last element = top)
#   $self->{locals}   — local variable array (array ref, 0-indexed)
#   $self->{pc}       — program counter (0-based byte index)
#   $self->{bytecode} — array ref of byte integers (0-255)
#   $self->{halted}   — true after ret executes
#
# @return blessed hashref
# ----------------------------------------------------------------------------
sub new {
    my ($class) = @_;
    return bless {
        stack    => [],
        locals   => [],
        pc       => 0,
        bytecode => [],
        halted   => 0,
    }, $class;
}

# ----------------------------------------------------------------------------
# load($bytecode_ref, %opts) → $self
#
# Load bytecode into the simulator and reset state.
#
# @param $bytecode_ref  Array ref of byte integers (0-255)
# @param %opts          Optional: num_locals => N (default 16)
# @return $self (for chaining)
# ----------------------------------------------------------------------------
sub load {
    my ($self, $bytecode_ref, %opts) = @_;
    my $num_locals = $opts{num_locals} // 16;
    $self->{bytecode} = $bytecode_ref;
    $self->{stack}    = [];
    $self->{locals}   = [(undef) x $num_locals];
    $self->{pc}       = 0;
    $self->{halted}   = 0;
    return $self;
}

# ============================================================================
# Byte Reading Helpers
# ============================================================================

sub _byte_at {
    my ($bytecode, $pos) = @_;
    return $bytecode->[$pos];
}

# Interpret a byte as a signed 8-bit integer (-128..127).
# CLR uses signed bytes for short branch offsets and ldc.i4.s operands.
sub _signed_byte {
    my ($val) = @_;
    return $val >= 128 ? $val - 256 : $val;
}

# Read a 32-bit signed integer from little-endian bytes at $pos.
sub _little_signed32 {
    my ($bytecode, $pos) = @_;
    my $v = $bytecode->[$pos]
          + $bytecode->[$pos+1] * 256
          + $bytecode->[$pos+2] * 65536
          + $bytecode->[$pos+3] * 16777216;
    $v -= 4294967296 if $v >= 2147483648;
    return $v;
}

# ============================================================================
# Stack Helpers
# ============================================================================

sub _pop {
    my ($self) = @_;
    die "Stack underflow\n" unless @{$self->{stack}};
    return pop @{$self->{stack}};
}

sub _push {
    my ($self, $val) = @_;
    push @{$self->{stack}}, $val;
}

# ============================================================================
# Step — Execute One Instruction
# ============================================================================

# ----------------------------------------------------------------------------
# step() → hashref (trace record)
#
# Execute the instruction at the current PC. Updates sim state and returns a
# trace record with:
#   pc           — program counter before execution
#   opcode       — mnemonic string (e.g. "ldc.i4.3")
#   stack_before — copy of stack before execution
#   stack_after  — copy of stack after execution
#   locals       — snapshot of local variables after execution
#   description  — plain-English description
#
# Dies if the simulator has halted or PC is out of range.
#
# @return hashref (trace)
# ----------------------------------------------------------------------------
sub step {
    my ($self) = @_;
    die "CLR simulator has halted\n" if $self->{halted};
    die sprintf("PC (%d) beyond end of bytecode\n", $self->{pc})
        if $self->{pc} >= scalar @{$self->{bytecode}};

    my @stack_before = @{$self->{stack}};
    my $opcode = _byte_at($self->{bytecode}, $self->{pc});

    # ---- nop ----------------------------------------------------------------
    if ($opcode == NOP) {
        my $pc = $self->{pc}++;
        return $self->_make_trace($pc, 'nop', \@stack_before, 'no operation');
    }

    # ---- ldnull -------------------------------------------------------------
    elsif ($opcode == LDNULL) {
        my $pc = $self->{pc}++;
        $self->_push(undef);
        return $self->_make_trace($pc, 'ldnull', \@stack_before, 'push null');
    }

    # ---- ldc.i4.0 through ldc.i4.8 -----------------------------------------
    # Opcodes 0x16-0x1E are consecutive: value = opcode - 0x16
    elsif ($opcode >= LDC_I4_0 && $opcode <= LDC_I4_8) {
        my $pc    = $self->{pc}++;
        my $value = $opcode - LDC_I4_0;
        $self->_push($value);
        return $self->_make_trace($pc, "ldc.i4.$value", \@stack_before,
            "push $value");
    }

    # ---- ldc.i4.s (signed byte operand) ------------------------------------
    elsif ($opcode == LDC_I4_S) {
        my $pc    = $self->{pc};
        my $value = _signed_byte(_byte_at($self->{bytecode}, $pc + 1));
        $self->{pc} += 2;
        $self->_push($value);
        return $self->_make_trace($pc, 'ldc.i4.s', \@stack_before,
            "push $value");
    }

    # ---- ldc.i4 (32-bit little-endian) -------------------------------------
    elsif ($opcode == LDC_I4) {
        my $pc    = $self->{pc};
        my $value = _little_signed32($self->{bytecode}, $pc + 1);
        $self->{pc} += 5;
        $self->_push($value);
        return $self->_make_trace($pc, 'ldc.i4', \@stack_before,
            "push $value");
    }

    # ---- ldloc.0 through ldloc.3 (opcodes 0x06-0x09) -----------------------
    elsif ($opcode >= LDLOC_0 && $opcode <= LDLOC_3) {
        my $pc   = $self->{pc}++;
        my $slot = $opcode - LDLOC_0;
        my $val  = $self->{locals}[$slot];
        die "Local variable $slot is uninitialized\n" unless defined $val;
        $self->_push($val);
        return $self->_make_trace($pc, "ldloc.$slot", \@stack_before,
            "push locals[$slot] = $val");
    }

    # ---- stloc.0 through stloc.3 (opcodes 0x0A-0x0D) -----------------------
    elsif ($opcode >= STLOC_0 && $opcode <= STLOC_3) {
        my $pc   = $self->{pc}++;
        my $slot = $opcode - STLOC_0;
        my $val  = $self->_pop();
        $self->{locals}[$slot] = $val;
        my $val_str = defined $val ? $val : 'null';
        return $self->_make_trace($pc, "stloc.$slot", \@stack_before,
            "pop $val_str, store in locals[$slot]");
    }

    # ---- ldloc.s (slot as 1-byte operand) ----------------------------------
    elsif ($opcode == LDLOC_S) {
        my $pc   = $self->{pc};
        my $slot = _byte_at($self->{bytecode}, $pc + 1);
        $self->{pc} += 2;
        my $val  = $self->{locals}[$slot];
        die "Local variable $slot is uninitialized\n" unless defined $val;
        $self->_push($val);
        return $self->_make_trace($pc, 'ldloc.s', \@stack_before,
            "push locals[$slot] = $val");
    }

    # ---- stloc.s (slot as 1-byte operand) ----------------------------------
    elsif ($opcode == STLOC_S) {
        my $pc   = $self->{pc};
        my $slot = _byte_at($self->{bytecode}, $pc + 1);
        $self->{pc} += 2;
        my $val  = $self->_pop();
        $self->{locals}[$slot] = $val;
        my $val_str = defined $val ? $val : 'null';
        return $self->_make_trace($pc, 'stloc.s', \@stack_before,
            "pop $val_str, store in locals[$slot]");
    }

    # ---- add / sub / mul ----------------------------------------------------
    elsif ($opcode == ADD) {
        my $pc = $self->{pc}++;
        my $b  = $self->_pop();
        my $a  = $self->_pop();
        $self->_push($a + $b);
        return $self->_make_trace($pc, 'add', \@stack_before,
            "pop $b and $a, push @{[$a+$b]}");
    }
    elsif ($opcode == SUB) {
        my $pc = $self->{pc}++;
        my $b  = $self->_pop();
        my $a  = $self->_pop();
        $self->_push($a - $b);
        return $self->_make_trace($pc, 'sub', \@stack_before,
            "pop $b and $a, push @{[$a-$b]}");
    }
    elsif ($opcode == MUL) {
        my $pc = $self->{pc}++;
        my $b  = $self->_pop();
        my $a  = $self->_pop();
        $self->_push($a * $b);
        return $self->_make_trace($pc, 'mul', \@stack_before,
            "pop $b and $a, push @{[$a*$b]}");
    }

    # ---- div (truncates toward zero) ----------------------------------------
    elsif ($opcode == DIV) {
        my $pc = $self->{pc}++;
        my $b  = $self->_pop();
        my $a  = $self->_pop();
        die "System.DivideByZeroException: division by zero\n" if $b == 0;
        # Truncate toward zero (like C's / for integers, not Perl's int())
        my $result = $a / $b;
        $result = $result > 0 ? int($result) : -int(-$result);
        $self->_push($result);
        return $self->_make_trace($pc, 'div', \@stack_before,
            "pop $b and $a, push $result");
    }

    # ---- ret ----------------------------------------------------------------
    elsif ($opcode == RET) {
        my $pc = $self->{pc}++;
        $self->{halted} = 1;
        return $self->_make_trace($pc, 'ret', \@stack_before, 'return');
    }

    # ---- br.s (unconditional short branch) ----------------------------------
    # The offset is signed and relative to the instruction AFTER the br.s
    # (next_pc = current_pc + 2, then target = next_pc + offset).
    elsif ($opcode == BR_S) {
        my $pc     = $self->{pc};
        my $offset = _signed_byte(_byte_at($self->{bytecode}, $pc + 1));
        my $next   = $pc + 2;
        my $target = $next + $offset;
        $self->{pc} = $target;
        my $sign = $offset >= 0 ? "+$offset" : "$offset";
        return $self->_make_trace($pc, 'br.s', \@stack_before,
            "branch to PC=$target (offset $sign)");
    }

    # ---- brfalse.s (branch if 0 or null) ------------------------------------
    elsif ($opcode == BRFALSE_S) {
        my $pc     = $self->{pc};
        my $offset = _signed_byte(_byte_at($self->{bytecode}, $pc + 1));
        my $next   = $pc + 2;
        my $val    = $self->_pop();
        my $num    = defined($val) ? $val : 0;
        if ($num == 0) {
            $self->{pc} = $next + $offset;
            my $val_str = defined $val ? $val : 'null';
            return $self->_make_trace($pc, 'brfalse.s', \@stack_before,
                "pop $val_str, branch taken to PC=$self->{pc}");
        } else {
            $self->{pc} = $next;
            return $self->_make_trace($pc, 'brfalse.s', \@stack_before,
                "pop $val, branch not taken");
        }
    }

    # ---- brtrue.s (branch if non-0 and non-null) ----------------------------
    elsif ($opcode == BRTRUE_S) {
        my $pc     = $self->{pc};
        my $offset = _signed_byte(_byte_at($self->{bytecode}, $pc + 1));
        my $next   = $pc + 2;
        my $val    = $self->_pop();
        my $num    = defined($val) ? $val : 0;
        if ($num != 0) {
            $self->{pc} = $next + $offset;
            return $self->_make_trace($pc, 'brtrue.s', \@stack_before,
                "pop $val, branch taken to PC=$self->{pc}");
        } else {
            $self->{pc} = $next;
            my $val_str = defined $val ? $val : 'null';
            return $self->_make_trace($pc, 'brtrue.s', \@stack_before,
                "pop $val_str, branch not taken");
        }
    }

    # ---- 0xFE prefix: ceq / cgt / clt ----------------------------------------
    # Two-byte compare instructions that push 1 (true) or 0 (false).
    # Note: a and b are on the stack as (a pushed first, b pushed second),
    # so we pop b first, then a.
    elsif ($opcode == PREFIX_FE) {
        my $pc     = $self->{pc};
        die sprintf("Incomplete two-byte opcode at PC=%d\n", $pc)
            if $pc + 1 >= scalar @{$self->{bytecode}};
        my $second = _byte_at($self->{bytecode}, $pc + 1);
        my $b = $self->_pop();
        my $a = $self->_pop();
        my ($mnemonic, $result, $desc);
        if ($second == CEQ_BYTE) {
            $mnemonic = 'ceq';
            $result   = ($a == $b) ? 1 : 0;
            $desc     = "pop $b and $a, push $result ($a == $b)";
        } elsif ($second == CGT_BYTE) {
            $mnemonic = 'cgt';
            $result   = ($a > $b) ? 1 : 0;
            $desc     = "pop $b and $a, push $result ($a > $b)";
        } elsif ($second == CLT_BYTE) {
            $mnemonic = 'clt';
            $result   = ($a < $b) ? 1 : 0;
            $desc     = "pop $b and $a, push $result ($a < $b)";
        } else {
            die sprintf("Unknown two-byte opcode: 0xFE 0x%02X at PC=%d\n",
                $second, $pc);
        }
        $self->{pc} += 2;
        $self->_push($result);
        return $self->_make_trace($pc, $mnemonic, \@stack_before, $desc);
    }

    else {
        die sprintf("Unknown CLR opcode: 0x%02X at PC=%d\n", $opcode, $self->{pc});
    }
}

# ============================================================================
# Run — Execute Until Halt
# ============================================================================

# ----------------------------------------------------------------------------
# run(%opts) → \@traces
#
# Execute instructions until the simulator halts or max_steps is reached.
#
# @param %opts  Optional: max_steps => N (default 10_000)
# @return arrayref of trace hashrefs
# ----------------------------------------------------------------------------
sub run {
    my ($self, %opts) = @_;
    my $max_steps = $opts{max_steps} // 10_000;
    my @traces;
    for (1 .. $max_steps) {
        last if $self->{halted};
        push @traces, $self->step();
    }
    return \@traces;
}

# ============================================================================
# Assembly Helpers
# ============================================================================

# ----------------------------------------------------------------------------
# encode_ldc_i4($n) → \@bytes
#
# Encode an integer constant in the most compact CLR form:
#   0-8:      ldc.i4.N     (1 byte, 0x16-0x1E)
#   -128..127: ldc.i4.s    (2 bytes)
#   otherwise: ldc.i4      (5 bytes, little-endian 32-bit)
# ----------------------------------------------------------------------------
sub encode_ldc_i4 {
    my ($n) = @_;
    if ($n >= 0 && $n <= 8) {
        return [LDC_I4_0 + $n];
    } elsif ($n >= -128 && $n <= 127) {
        my $b = $n < 0 ? $n + 256 : $n;
        return [LDC_I4_S, $b];
    } else {
        my $v = $n;
        $v += 4294967296 if $v < 0;
        my @bytes = (
            $v % 256,
            int($v / 256)     % 256,
            int($v / 65536)   % 256,
            int($v / 16777216) % 256,
        );
        return [LDC_I4, @bytes];
    }
}

# encode_stloc($slot) → \@bytes
sub encode_stloc {
    my ($slot) = @_;
    return $slot <= 3 ? [STLOC_0 + $slot] : [STLOC_S, $slot];
}

# encode_ldloc($slot) → \@bytes
sub encode_ldloc {
    my ($slot) = @_;
    return $slot <= 3 ? [LDLOC_0 + $slot] : [LDLOC_S, $slot];
}

# ----------------------------------------------------------------------------
# assemble(\@parts) → \@flat_bytes
#
# Flatten an array of byte-array refs into a single array ref.
#
# Example:
#   assemble([encode_ldc_i4(3), [ADD], [RET]])
# ----------------------------------------------------------------------------
sub assemble {
    my ($parts_ref) = @_;
    my @result;
    for my $part (@$parts_ref) {
        push @result, @$part;
    }
    return \@result;
}

# ============================================================================
# Private: Make Trace
# ============================================================================

sub _make_trace {
    my ($self, $pc, $opcode, $stack_before_ref, $description) = @_;
    return {
        pc           => $pc,
        opcode       => $opcode,
        stack_before => [@$stack_before_ref],
        stack_after  => [@{$self->{stack}}],
        locals       => [@{$self->{locals}}],
        description  => $description,
    };
}

1;

__END__

=head1 NAME

CodingAdventures::ClrSimulator - CLR Intermediate Language bytecode simulator

=head1 SYNOPSIS

    use CodingAdventures::ClrSimulator;

    my $sim = CodingAdventures::ClrSimulator->new();

    # Assemble: x = 1 + 2
    my $code = CodingAdventures::ClrSimulator::assemble([
        CodingAdventures::ClrSimulator::encode_ldc_i4(1),
        CodingAdventures::ClrSimulator::encode_ldc_i4(2),
        [CodingAdventures::ClrSimulator::ADD],
        [CodingAdventures::ClrSimulator::STLOC_0],
        [CodingAdventures::ClrSimulator::RET],
    ]);

    $sim->load($code);
    my $traces = $sim->run();
    print $sim->{locals}[0];  # 3

=head1 DESCRIPTION

Simulates a subset of the CLR Intermediate Language (CIL/MSIL). Supports
ldc.i4 variants, ldloc/stloc (short and long forms), arithmetic, compare
instructions (0xFE prefix), branching, and ret.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
