package CodingAdventures::JvmSimulator;

# ============================================================================
# CodingAdventures::JvmSimulator — JVM bytecode simulator in Pure Perl
# ============================================================================
#
# # What is the JVM?
#
# The Java Virtual Machine (1995) is the most widely deployed virtual machine
# in history. It runs Java, Kotlin, Scala, Clojure, and Groovy. It is a
# **stack-based machine** — instructions operate on an operand stack alongside
# a local variable array.
#
# # Typed Opcodes: The JVM's Design
#
# The JVM uses separate opcodes for each data type. For integers, all opcodes
# start with `i`:
#
#   iconst_N  — push small integer constant N
#   iadd      — add two integers
#   iload_N   — load integer from local variable N
#   istore_N  — store integer to local variable N
#   ireturn   — return an integer value
#
# This lets the JVM verify type safety at class-load time (bytecode
# verification), before execution even starts.
#
# # Local Variable Array
#
# Each JVM method frame has:
#   1. An operand stack  — for arithmetic and computation
#   2. A local variable array — for named variables (0-indexed; slot 0 = "this"
#      for instance methods)
#
# Short forms (iload_0-3, istore_0-3) avoid encoding a slot number for the
# four most common slots. Slots 4+ use iload N and istore N with a 1-byte
# slot operand.
#
# # Constant Pool
#
# JVM class files contain a "constant pool" — a table of constants (strings,
# numbers, class/method references). The `ldc` instruction loads a constant
# pool entry by 1-byte index. Our simulator uses a Perl array as the pool.
#
# # Branch Offsets
#
# JVM branches (`goto`, `if_icmpeq`, `if_icmpgt`) use **big-endian 16-bit
# signed offsets** relative to the branch instruction's own PC (not next PC).
#
#   target = instruction_pc + offset
#
# This differs from the CLR (which uses offsets relative to next_pc).
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# Opcode Constants (real JVM hex values)
# ============================================================================

use constant {
    ICONST_0   => 0x03,   # push int 0
    ICONST_1   => 0x04,   # push int 1
    ICONST_2   => 0x05,   # push int 2
    ICONST_3   => 0x06,   # push int 3
    ICONST_4   => 0x07,   # push int 4
    ICONST_5   => 0x08,   # push int 5
    BIPUSH     => 0x10,   # push signed byte as int (1-byte operand)
    SIPUSH     => 0x11,   # push signed short as int (2-byte big-endian)
    LDC        => 0x12,   # push from constant pool (1-byte index)
    ILOAD      => 0x15,   # load int from local[N] (1-byte slot)
    ILOAD_0    => 0x1A,   # load int from local[0]
    ILOAD_1    => 0x1B,   # load int from local[1]
    ILOAD_2    => 0x1C,   # load int from local[2]
    ILOAD_3    => 0x1D,   # load int from local[3]
    ISTORE     => 0x36,   # store int to local[N] (1-byte slot)
    ISTORE_0   => 0x3B,   # store int to local[0]
    ISTORE_1   => 0x3C,   # store int to local[1]
    ISTORE_2   => 0x3D,   # store int to local[2]
    ISTORE_3   => 0x3E,   # store int to local[3]
    IADD       => 0x60,   # pop b, pop a, push a+b (int32)
    ISUB       => 0x64,   # pop b, pop a, push a-b
    IMUL       => 0x68,   # pop b, pop a, push a*b
    IDIV       => 0x6C,   # pop b, pop a, push trunc(a/b); raises on b=0
    IF_ICMPEQ  => 0x9F,   # pop b, pop a; branch if a == b (2-byte offset)
    IF_ICMPGT  => 0xA3,   # pop b, pop a; branch if a > b
    GOTO       => 0xA7,   # unconditional branch (2-byte signed offset)
    IRETURN    => 0xAC,   # pop int, halt with return_value
    RETURN     => 0xB1,   # void return (halt)
};

# ============================================================================
# Constructor
# ============================================================================

# ----------------------------------------------------------------------------
# new() → JvmSimulator instance
#
# Creates a new JVM simulator. State is mutable — step() updates in-place.
#
# Fields:
#   $self->{stack}        — operand stack (array ref, last = top)
#   $self->{locals}       — local variable array (array ref, 0-indexed)
#   $self->{constants}    — constant pool (array ref, 0-indexed at JVM level)
#   $self->{pc}           — program counter (0-based byte index)
#   $self->{bytecode}     — array ref of byte integers (0-255)
#   $self->{halted}       — true after return/ireturn
#   $self->{return_value} — integer return value (from ireturn) or undef
# ----------------------------------------------------------------------------
sub new {
    my ($class) = @_;
    return bless {
        stack        => [],
        locals       => [],
        constants    => [],
        pc           => 0,
        bytecode     => [],
        halted       => 0,
        return_value => undef,
    }, $class;
}

# ----------------------------------------------------------------------------
# load($bytecode_ref, %opts) → $self
#
# Load bytecode and reset state.
# @param %opts  constants => \@pool, num_locals => N (default 16)
# ----------------------------------------------------------------------------
sub load {
    my ($self, $bytecode_ref, %opts) = @_;
    my $num_locals = $opts{num_locals} // 16;
    my $constants  = $opts{constants}  // [];
    $self->{bytecode}     = $bytecode_ref;
    $self->{stack}        = [];
    $self->{locals}       = [(undef) x $num_locals];
    $self->{constants}    = $constants;
    $self->{pc}           = 0;
    $self->{halted}       = 0;
    $self->{return_value} = undef;
    return $self;
}

# ============================================================================
# Byte Reading Helpers
# ============================================================================

sub _byte_at {
    my ($bytecode, $pos) = @_;
    return $bytecode->[$pos];
}

sub _signed_byte {
    my ($val) = @_;
    return $val >= 128 ? $val - 256 : $val;
}

# Big-endian signed 16-bit (used by goto, if_icmpeq, if_icmpgt).
# JVM convention: target = instruction_pc + offset (not next_pc + offset).
sub _big_signed16 {
    my ($bytecode, $pos) = @_;
    my $v = $bytecode->[$pos] * 256 + $bytecode->[$pos + 1];
    $v -= 65536 if $v >= 32768;
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
# Int32 Overflow Helper
# ============================================================================

# Clamp to signed 32-bit integer range. Perl uses 64-bit integers, so we
# must truncate after every arithmetic operation.
sub _to_i32 {
    my ($val) = @_;
    $val = $val % 4294967296;
    $val -= 4294967296 if $val >= 2147483648;
    return $val;
}

# ============================================================================
# Step — Execute One Instruction
# ============================================================================

# ----------------------------------------------------------------------------
# step() → hashref (trace)
#
# Returns: { pc, opcode, stack_before, stack_after, locals, description }
# ----------------------------------------------------------------------------
sub step {
    my ($self) = @_;
    die "JVM simulator has halted\n" if $self->{halted};
    die sprintf("PC (%d) past end of bytecode (%d bytes)\n",
        $self->{pc}, scalar @{$self->{bytecode}})
        if $self->{pc} >= scalar @{$self->{bytecode}};

    my @stack_before = @{$self->{stack}};
    my $opcode = _byte_at($self->{bytecode}, $self->{pc});

    # ---- iconst_0 through iconst_5 ------------------------------------------
    if ($opcode >= ICONST_0 && $opcode <= ICONST_5) {
        my $pc    = $self->{pc}++;
        my $value = $opcode - ICONST_0;
        $self->_push($value);
        return $self->_trace($pc, "iconst_$value", \@stack_before, "push $value");
    }

    # ---- bipush (signed byte) -----------------------------------------------
    elsif ($opcode == BIPUSH) {
        my $pc    = $self->{pc};
        my $value = _signed_byte(_byte_at($self->{bytecode}, $pc + 1));
        $self->{pc} += 2;
        $self->_push($value);
        return $self->_trace($pc, 'bipush', \@stack_before, "push $value");
    }

    # ---- sipush (big-endian signed 16-bit) ----------------------------------
    elsif ($opcode == SIPUSH) {
        my $pc    = $self->{pc};
        my $value = _big_signed16($self->{bytecode}, $pc + 1);
        $self->{pc} += 3;
        $self->_push($value);
        return $self->_trace($pc, 'sipush', \@stack_before, "push $value");
    }

    # ---- ldc (constant pool) ------------------------------------------------
    elsif ($opcode == LDC) {
        my $pc    = $self->{pc};
        my $index = _byte_at($self->{bytecode}, $pc + 1);
        die "Constant pool index $index out of range\n"
            if $index >= scalar @{$self->{constants}};
        my $value = $self->{constants}[$index];
        die "ldc: constant pool entry $index is not a number\n"
            unless defined $value && $value =~ /^-?\d+(\.\d+)?$/;
        $self->{pc} += 2;
        $self->_push($value);
        return $self->_trace($pc, 'ldc', \@stack_before,
            "push constant[$index] = $value");
    }

    # ---- iload_0 through iload_3 (0x1A-0x1D) --------------------------------
    elsif ($opcode >= ILOAD_0 && $opcode <= ILOAD_3) {
        my $pc   = $self->{pc}++;
        my $slot = $opcode - ILOAD_0;
        my $val  = $self->{locals}[$slot];
        die "Local variable $slot has not been initialized\n" unless defined $val;
        $self->_push($val);
        return $self->_trace($pc, "iload_$slot", \@stack_before,
            "push locals[$slot] = $val");
    }

    # ---- iload (with slot operand) ------------------------------------------
    elsif ($opcode == ILOAD) {
        my $pc   = $self->{pc};
        my $slot = _byte_at($self->{bytecode}, $pc + 1);
        $self->{pc} += 2;
        my $val  = $self->{locals}[$slot];
        die "Local variable $slot has not been initialized\n" unless defined $val;
        $self->_push($val);
        return $self->_trace($pc, 'iload', \@stack_before,
            "push locals[$slot] = $val");
    }

    # ---- istore_0 through istore_3 (0x3B-0x3E) ------------------------------
    elsif ($opcode >= ISTORE_0 && $opcode <= ISTORE_3) {
        my $pc   = $self->{pc}++;
        my $slot = $opcode - ISTORE_0;
        my $val  = $self->_pop();
        $self->{locals}[$slot] = $val;
        return $self->_trace($pc, "istore_$slot", \@stack_before,
            "pop $val, store in locals[$slot]");
    }

    # ---- istore (with slot operand) -----------------------------------------
    elsif ($opcode == ISTORE) {
        my $pc   = $self->{pc};
        my $slot = _byte_at($self->{bytecode}, $pc + 1);
        $self->{pc} += 2;
        my $val  = $self->_pop();
        $self->{locals}[$slot] = $val;
        return $self->_trace($pc, 'istore', \@stack_before,
            "pop $val, store in locals[$slot]");
    }

    # ---- iadd ---------------------------------------------------------------
    elsif ($opcode == IADD) {
        my $pc = $self->{pc}++;
        my $b  = $self->_pop();
        my $a  = $self->_pop();
        my $r  = _to_i32($a + $b);
        $self->_push($r);
        return $self->_trace($pc, 'iadd', \@stack_before,
            "pop $b and $a, push $r");
    }

    # ---- isub ---------------------------------------------------------------
    elsif ($opcode == ISUB) {
        my $pc = $self->{pc}++;
        my $b  = $self->_pop();
        my $a  = $self->_pop();
        my $r  = _to_i32($a - $b);
        $self->_push($r);
        return $self->_trace($pc, 'isub', \@stack_before,
            "pop $b and $a, push $r");
    }

    # ---- imul ---------------------------------------------------------------
    elsif ($opcode == IMUL) {
        my $pc = $self->{pc}++;
        my $b  = $self->_pop();
        my $a  = $self->_pop();
        my $r  = _to_i32($a * $b);
        $self->_push($r);
        return $self->_trace($pc, 'imul', \@stack_before,
            "pop $b and $a, push $r");
    }

    # ---- idiv ---------------------------------------------------------------
    elsif ($opcode == IDIV) {
        my $pc = $self->{pc}++;
        my $b  = $self->_pop();
        my $a  = $self->_pop();
        die "ArithmeticException: / by zero\n" if $b == 0;
        my $r = $a / $b;
        $r = $r > 0 ? int($r) : -int(-$r);
        $r = _to_i32($r);
        $self->_push($r);
        return $self->_trace($pc, 'idiv', \@stack_before,
            "pop $b and $a, push $r");
    }

    # ---- goto ---------------------------------------------------------------
    elsif ($opcode == GOTO) {
        my $pc     = $self->{pc};
        my $offset = _big_signed16($self->{bytecode}, $pc + 1);
        my $target = $pc + $offset;
        $self->{pc} = $target;
        my $sign = $offset >= 0 ? "+$offset" : "$offset";
        return $self->_trace($pc, 'goto', \@stack_before,
            "jump to PC=$target (offset $sign)");
    }

    # ---- if_icmpeq ----------------------------------------------------------
    elsif ($opcode == IF_ICMPEQ) {
        my $pc     = $self->{pc};
        my $offset = _big_signed16($self->{bytecode}, $pc + 1);
        my $b = $self->_pop();
        my $a = $self->_pop();
        if ($a == $b) {
            $self->{pc} = $pc + $offset;
            return $self->_trace($pc, 'if_icmpeq', \@stack_before,
                "pop $b and $a, $a == $b is true, jump to PC=$self->{pc}");
        } else {
            $self->{pc} = $pc + 3;
            return $self->_trace($pc, 'if_icmpeq', \@stack_before,
                "pop $b and $a, $a == $b is false, fall through");
        }
    }

    # ---- if_icmpgt ----------------------------------------------------------
    elsif ($opcode == IF_ICMPGT) {
        my $pc     = $self->{pc};
        my $offset = _big_signed16($self->{bytecode}, $pc + 1);
        my $b = $self->_pop();
        my $a = $self->_pop();
        if ($a > $b) {
            $self->{pc} = $pc + $offset;
            return $self->_trace($pc, 'if_icmpgt', \@stack_before,
                "pop $b and $a, $a > $b is true, jump to PC=$self->{pc}");
        } else {
            $self->{pc} = $pc + 3;
            return $self->_trace($pc, 'if_icmpgt', \@stack_before,
                "pop $b and $a, $a > $b is false, fall through");
        }
    }

    # ---- ireturn ------------------------------------------------------------
    elsif ($opcode == IRETURN) {
        my $pc = $self->{pc}++;
        my $val = $self->_pop();
        $self->{return_value} = $val;
        $self->{halted} = 1;
        return $self->_trace($pc, 'ireturn', \@stack_before, "return $val");
    }

    # ---- return (void) ------------------------------------------------------
    elsif ($opcode == RETURN) {
        my $pc = $self->{pc}++;
        $self->{halted} = 1;
        return $self->_trace($pc, 'return', \@stack_before, 'return void');
    }

    else {
        die sprintf("Unknown JVM opcode: 0x%02X at PC=%d\n",
            $opcode, $self->{pc});
    }
}

# ============================================================================
# Run
# ============================================================================

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

# encode_iconst($n) — most compact encoding for an integer push
sub encode_iconst {
    my ($n) = @_;
    if ($n >= 0 && $n <= 5) {
        return [ICONST_0 + $n];
    } elsif ($n >= -128 && $n <= 127) {
        my $b = $n < 0 ? $n + 256 : $n;
        return [BIPUSH, $b];
    } else {
        die "encode_iconst: $n outside bipush range; use sipush or ldc\n";
    }
}

# encode_istore($slot) — short or long form
sub encode_istore {
    my ($slot) = @_;
    return $slot <= 3 ? [ISTORE_0 + $slot] : [ISTORE, $slot];
}

# encode_iload($slot) — short or long form
sub encode_iload {
    my ($slot) = @_;
    return $slot <= 3 ? [ILOAD_0 + $slot] : [ILOAD, $slot];
}

# assemble(\@parts) — flatten array of byte-array refs
sub assemble {
    my ($parts_ref) = @_;
    my @result;
    push @result, @$_ for @$parts_ref;
    return \@result;
}

# ============================================================================
# Private
# ============================================================================

sub _trace {
    my ($self, $pc, $opcode, $stack_before, $description) = @_;
    return {
        pc           => $pc,
        opcode       => $opcode,
        stack_before => [@$stack_before],
        stack_after  => [@{$self->{stack}}],
        locals       => [@{$self->{locals}}],
        description  => $description,
    };
}

1;

__END__

=head1 NAME

CodingAdventures::JvmSimulator - JVM bytecode simulator in Pure Perl

=head1 SYNOPSIS

    use CodingAdventures::JvmSimulator;

    my $sim = CodingAdventures::JvmSimulator->new();
    my $code = CodingAdventures::JvmSimulator::assemble([
        CodingAdventures::JvmSimulator::encode_iconst(1),
        CodingAdventures::JvmSimulator::encode_iconst(2),
        [CodingAdventures::JvmSimulator::IADD],
        CodingAdventures::JvmSimulator::encode_istore(0),
        [CodingAdventures::JvmSimulator::RETURN],
    ]);
    $sim->load($code);
    $sim->run();
    print $sim->{locals}[0];  # 3

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
