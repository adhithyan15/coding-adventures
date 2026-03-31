package CodingAdventures::GpuCore;

# ============================================================================
# CodingAdventures::GpuCore — Generic Accelerator Processing Element
# ============================================================================
#
# This module simulates a single "processing element" — the smallest compute
# unit in any accelerator.  Despite different marketing names (CUDA Core,
# Stream Processor, Vector Engine, MAC Unit), every GPU/TPU/NPU shares the
# same pattern:
#
#   1. Local state: a floating-point register file.
#   2. Computation: FP add, multiply, fused-multiply-add.
#   3. Instruction stream: a program counter drives execution.
#   4. Local memory: a small scratchpad for intermediate values.
#
# By modelling this common pattern, we can simulate one CUDA core today and
# swap in a PTX instruction set tomorrow without touching the core.
#
# # Stack Position
# =================
#
#   Layer 11: Logic Gates (AND, OR, XOR, NAND)
#       │
#   Layer 10: FP Arithmetic (IEEE 754 add/mul/fma)
#       │
#   Layer 9:  Accelerator Core  ← YOU ARE HERE
#       │
#       ├──→ GPU: Warp/SIMT Engine (32 cores in lockstep)
#       ├──→ TPU: Systolic Array (NxN grid of PEs)
#       └──→ NPU: MAC Array (parallel MACs)
#
# # Execution Model
# =================
#
#   while not halted:
#     instruction = program[pc]
#     result = isa->execute(instruction, registers, memory)
#     pc = next_pc
#
# No branch prediction, no out-of-order, no pipeline.  GPUs achieve throughput
# through massive parallelism (thousands of simple cores running simultaneously),
# not per-core complexity.
#
# # Generic ISA
# =============
#
#   FADD  Rd, Rs1, Rs2         Rd = Rs1 + Rs2
#   FSUB  Rd, Rs1, Rs2         Rd = Rs1 - Rs2
#   FMUL  Rd, Rs1, Rs2         Rd = Rs1 * Rs2
#   FFMA  Rd, Rs1, Rs2, Rs3    Rd = Rs1 * Rs2 + Rs3  (fused multiply-add)
#   FNEG  Rd, Rs1              Rd = -Rs1
#   FABS  Rd, Rs1              Rd = |Rs1|
#   LOAD  Rd, Rs1, imm         Rd = Mem[Rs1 + imm]
#   STORE Rs1, Rs2, imm        Mem[Rs1 + imm] = Rs2
#   MOV   Rd, Rs1              Rd = Rs1
#   LIMM  Rd, imm              Rd = immediate float
#   BEQ   Rs1, Rs2, offset     if Rs1 == Rs2: PC += offset
#   BLT   Rs1, Rs2, offset     if Rs1 < Rs2:  PC += offset
#   BNE   Rs1, Rs2, offset     if Rs1 != Rs2: PC += offset
#   JMP   target               PC = target (absolute)
#   NOP                        no-op
#   HALT                       stop execution
#
# ============================================================================

use strict;
use warnings;
use Carp qw(croak confess);
use POSIX qw(floor);

our $VERSION = '0.01';

# ============================================================================
# Instruction constructors (exported as class methods)
# ============================================================================
#
# These helpers make programs readable.  Instead of:
#   {opcode=>"FFMA", rd=>3, rs1=>0, rs2=>1, rs3=>2, immediate=>0}
# you write:
#   GpuCore->ffma(3, 0, 1, 2)

sub _instr {
  my ($opcode, %args) = @_;
  return {
    opcode    => $opcode,
    rd        => $args{rd}        // 0,
    rs1       => $args{rs1}       // 0,
    rs2       => $args{rs2}       // 0,
    rs3       => $args{rs3}       // 0,
    immediate => $args{immediate} // 0,
  };
}

sub fadd  { _instr('FADD',  rd=>$_[1], rs1=>$_[2], rs2=>$_[3]) }
sub fsub  { _instr('FSUB',  rd=>$_[1], rs1=>$_[2], rs2=>$_[3]) }
sub fmul  { _instr('FMUL',  rd=>$_[1], rs1=>$_[2], rs2=>$_[3]) }
sub ffma  { _instr('FFMA',  rd=>$_[1], rs1=>$_[2], rs2=>$_[3], rs3=>$_[4]) }
sub fneg  { _instr('FNEG',  rd=>$_[1], rs1=>$_[2]) }
sub fabs  { _instr('FABS',  rd=>$_[1], rs1=>$_[2]) }
sub load  { _instr('LOAD',  rd=>$_[1], rs1=>$_[2], immediate=>$_[3]//0) }
sub store { _instr('STORE', rs1=>$_[1], rs2=>$_[2], immediate=>$_[3]//0) }
sub mov   { _instr('MOV',   rd=>$_[1], rs1=>$_[2]) }
sub limm  { _instr('LIMM',  rd=>$_[1], immediate=>$_[2]) }
sub beq   { _instr('BEQ',   rs1=>$_[1], rs2=>$_[2], immediate=>$_[3]) }
sub blt   { _instr('BLT',   rs1=>$_[1], rs2=>$_[2], immediate=>$_[3]) }
sub bne   { _instr('BNE',   rs1=>$_[1], rs2=>$_[2], immediate=>$_[3]) }
sub jmp   { _instr('JMP',   immediate=>$_[1]) }
sub nop   { _instr('NOP') }
sub halt  { _instr('HALT') }

# ============================================================================
# Example programs
# ============================================================================

sub saxpy_program {
  # SAXPY: R3 = a * x + y
  # The canonical GPU "hello world" — one thread computes one element.
  my ($class, $a, $x, $y) = @_;
  return [
    $class->limm(0, $a),         # R0 = a
    $class->limm(1, $x),         # R1 = x
    $class->limm(2, $y),         # R2 = y
    $class->ffma(3, 0, 1, 2),    # R3 = a * x + y
    $class->halt(),
  ];
}

sub dot_product_program {
  # Dot product: [1,2,3] · [4,5,6] = 32
  my ($class) = @_;
  return [
    $class->limm(0, 1.0),        # R0 = A[0]
    $class->limm(1, 2.0),        # R1 = A[1]
    $class->limm(2, 3.0),        # R2 = A[2]
    $class->limm(3, 4.0),        # R3 = B[0]
    $class->limm(4, 5.0),        # R4 = B[1]
    $class->limm(5, 6.0),        # R5 = B[2]
    $class->limm(6, 0.0),        # R6 = accumulator = 0.0
    $class->ffma(6, 0, 3, 6),    # R6 = 1.0 * 4.0 + 0.0 = 4.0
    $class->ffma(6, 1, 4, 6),    # R6 = 2.0 * 5.0 + 4.0 = 14.0
    $class->ffma(6, 2, 5, 6),    # R6 = 3.0 * 6.0 + 14.0 = 32.0
    $class->halt(),
  ];
}

# ============================================================================
# FPRegisterFile
# ============================================================================
#
# A flat array of 32 (or more) floating-point registers, all initialized to 0.
# Register access is bounds-checked to catch programming errors early.

package CodingAdventures::GpuCore::FPRegisterFile;

use strict;
use warnings;
use Carp qw(croak);

our $VERSION = '0.01';

sub new {
  my ($class, %args) = @_;
  my $num = $args{num_registers} // 32;
  my $self = bless {
    _num  => $num,
    _regs => [ (0.0) x $num ],
  }, $class;
  return $self;
}

sub read {
  my ($self, $index) = @_;
  croak "register index $index out of range [0,$self->{_num})"
    if $index < 0 || $index >= $self->{_num};
  return $self->{_regs}[$index];
}

sub write {
  my ($self, $index, $value) = @_;
  croak "register index $index out of range [0,$self->{_num})"
    if $index < 0 || $index >= $self->{_num};
  $self->{_regs}[$index] = $value;
}

sub reset {
  my ($self) = @_;
  $self->{_regs} = [ (0.0) x $self->{_num} ];
}

sub size { return $_[0]->{_num} }

# ============================================================================
# LocalMemory
# ============================================================================
#
# A simple scratchpad modelled as a hash indexed by integer address.
# Represents per-thread shared memory in GPU terminology.

package CodingAdventures::GpuCore::LocalMemory;

use strict;
use warnings;
use Carp qw(croak);

our $VERSION = '0.01';

sub new {
  my ($class, %args) = @_;
  return bless {
    _size => $args{size} // 4096,
    _mem  => {},
  }, $class;
}

sub load {
  my ($self, $addr) = @_;
  croak "memory address $addr out of range [0,$self->{_size})"
    if $addr < 0 || $addr >= $self->{_size};
  return $self->{_mem}{$addr} // 0.0;
}

sub store {
  my ($self, $addr, $value) = @_;
  croak "memory address $addr out of range [0,$self->{_size})"
    if $addr < 0 || $addr >= $self->{_size};
  $self->{_mem}{$addr} = $value;
}

sub reset {
  my ($self) = @_;
  $self->{_mem} = {};
}

# ============================================================================
# GenericISA
# ============================================================================
#
# The GenericISA implements the InstructionSet protocol for the teaching ISA.
# To add a vendor-specific ISA (PTX, GCN), create a new class with an
# execute() method with the same signature.

package CodingAdventures::GpuCore::GenericISA;

use strict;
use warnings;
use Carp qw(croak);
use POSIX qw(floor);

our $VERSION = '0.01';

sub new {
  my ($class) = @_;
  return bless { name => 'Generic' }, $class;
}

sub name { return $_[0]->{name} }

# execute($instr, $registers, $memory) → \%result
#
# Returns a hash-ref with:
#   registers_changed  => { reg_index => new_value, ... }
#   memory_changed     => { addr      => new_value, ... }
#   description        => "human readable string"
#   next_pc_offset     => integer (added to pc+1 for branch; 0 for sequential)
#   jmp_target         => integer or undef  (absolute jump target)
#   halted             => boolean
sub execute {
  my ($self, $instr, $registers, $memory) = @_;
  my $op = $instr->{opcode};

  my $result = {
    registers_changed => {},
    memory_changed    => {},
    description       => '',
    next_pc_offset    => 0,
    jmp_target        => undef,
    halted            => 0,
  };

  if ($op eq 'FADD') {
    my $a = $registers->read($instr->{rs1});
    my $b = $registers->read($instr->{rs2});
    my $v = $a + $b;
    $registers->write($instr->{rd}, $v);
    $result->{registers_changed}{$instr->{rd}} = $v;
    $result->{description} = sprintf("R%d = R%d + R%d = %g + %g = %g",
      $instr->{rd}, $instr->{rs1}, $instr->{rs2}, $a, $b, $v);

  } elsif ($op eq 'FSUB') {
    my $a = $registers->read($instr->{rs1});
    my $b = $registers->read($instr->{rs2});
    my $v = $a - $b;
    $registers->write($instr->{rd}, $v);
    $result->{registers_changed}{$instr->{rd}} = $v;
    $result->{description} = sprintf("R%d = R%d - R%d = %g - %g = %g",
      $instr->{rd}, $instr->{rs1}, $instr->{rs2}, $a, $b, $v);

  } elsif ($op eq 'FMUL') {
    my $a = $registers->read($instr->{rs1});
    my $b = $registers->read($instr->{rs2});
    my $v = $a * $b;
    $registers->write($instr->{rd}, $v);
    $result->{registers_changed}{$instr->{rd}} = $v;
    $result->{description} = sprintf("R%d = R%d * R%d = %g * %g = %g",
      $instr->{rd}, $instr->{rs1}, $instr->{rs2}, $a, $b, $v);

  } elsif ($op eq 'FFMA') {
    # Fused multiply-add: Rd = Rs1 * Rs2 + Rs3
    # In hardware: one operation, one rounding step.  More accurate than FMUL+FADD.
    my $a = $registers->read($instr->{rs1});
    my $b = $registers->read($instr->{rs2});
    my $c = $registers->read($instr->{rs3});
    my $v = $a * $b + $c;
    $registers->write($instr->{rd}, $v);
    $result->{registers_changed}{$instr->{rd}} = $v;
    $result->{description} = sprintf("R%d = R%d * R%d + R%d = %g * %g + %g = %g",
      $instr->{rd}, $instr->{rs1}, $instr->{rs2}, $instr->{rs3}, $a, $b, $c, $v);

  } elsif ($op eq 'FNEG') {
    my $a = $registers->read($instr->{rs1});
    my $v = -$a;
    $registers->write($instr->{rd}, $v);
    $result->{registers_changed}{$instr->{rd}} = $v;
    $result->{description} = sprintf("R%d = -R%d = -%g = %g",
      $instr->{rd}, $instr->{rs1}, $a, $v);

  } elsif ($op eq 'FABS') {
    my $a = $registers->read($instr->{rs1});
    my $v = abs($a);
    $registers->write($instr->{rd}, $v);
    $result->{registers_changed}{$instr->{rd}} = $v;
    $result->{description} = sprintf("R%d = |R%d| = |%g| = %g",
      $instr->{rd}, $instr->{rs1}, $a, $v);

  } elsif ($op eq 'LOAD') {
    my $base = $registers->read($instr->{rs1});
    my $addr = floor($base + $instr->{immediate});
    my $v    = $memory->load($addr);
    $registers->write($instr->{rd}, $v);
    $result->{registers_changed}{$instr->{rd}} = $v;
    $result->{description} = sprintf("R%d = Mem[R%d + %g] = Mem[%d] = %g",
      $instr->{rd}, $instr->{rs1}, $instr->{immediate}, $addr, $v);

  } elsif ($op eq 'STORE') {
    my $base = $registers->read($instr->{rs1});
    my $addr = floor($base + $instr->{immediate});
    my $v    = $registers->read($instr->{rs2});
    $memory->store($addr, $v);
    $result->{memory_changed}{$addr} = $v;
    $result->{description} = sprintf("Mem[R%d + %g] = R%d → Mem[%d] = %g",
      $instr->{rs1}, $instr->{immediate}, $instr->{rs2}, $addr, $v);

  } elsif ($op eq 'MOV') {
    my $v = $registers->read($instr->{rs1});
    $registers->write($instr->{rd}, $v);
    $result->{registers_changed}{$instr->{rd}} = $v;
    $result->{description} = sprintf("R%d = R%d = %g", $instr->{rd}, $instr->{rs1}, $v);

  } elsif ($op eq 'LIMM') {
    my $v = $instr->{immediate};
    $registers->write($instr->{rd}, $v);
    $result->{registers_changed}{$instr->{rd}} = $v;
    $result->{description} = sprintf("R%d = %g (immediate)", $instr->{rd}, $v);

  } elsif ($op eq 'BEQ') {
    my $a = $registers->read($instr->{rs1});
    my $b = $registers->read($instr->{rs2});
    if ($a == $b) {
      $result->{next_pc_offset} = floor($instr->{immediate}) - 1;
      $result->{description} = sprintf("BEQ R%d, R%d: %g == %g → taken (offset %d)",
        $instr->{rs1}, $instr->{rs2}, $a, $b, $result->{next_pc_offset});
    } else {
      $result->{description} = sprintf("BEQ R%d, R%d: %g != %g → not taken",
        $instr->{rs1}, $instr->{rs2}, $a, $b);
    }

  } elsif ($op eq 'BLT') {
    my $a = $registers->read($instr->{rs1});
    my $b = $registers->read($instr->{rs2});
    if ($a < $b) {
      $result->{next_pc_offset} = floor($instr->{immediate}) - 1;
      $result->{description} = sprintf("BLT R%d, R%d: %g < %g → taken (offset %d)",
        $instr->{rs1}, $instr->{rs2}, $a, $b, $result->{next_pc_offset});
    } else {
      $result->{description} = sprintf("BLT R%d, R%d: %g >= %g → not taken",
        $instr->{rs1}, $instr->{rs2}, $a, $b);
    }

  } elsif ($op eq 'BNE') {
    my $a = $registers->read($instr->{rs1});
    my $b = $registers->read($instr->{rs2});
    if ($a != $b) {
      $result->{next_pc_offset} = floor($instr->{immediate}) - 1;
      $result->{description} = sprintf("BNE R%d, R%d: %g != %g → taken (offset %d)",
        $instr->{rs1}, $instr->{rs2}, $a, $b, $result->{next_pc_offset});
    } else {
      $result->{description} = sprintf("BNE R%d, R%d: %g == %g → not taken",
        $instr->{rs1}, $instr->{rs2}, $a, $b);
    }

  } elsif ($op eq 'JMP') {
    $result->{jmp_target} = floor($instr->{immediate});
    $result->{description} = sprintf("JMP %d", $result->{jmp_target});

  } elsif ($op eq 'NOP') {
    $result->{description} = 'NOP';

  } elsif ($op eq 'HALT') {
    $result->{halted}      = 1;
    $result->{description} = 'HALT';

  } else {
    croak "Unknown opcode: $op";
  }

  return $result;
}

# ============================================================================
# GPUCore
# ============================================================================
#
# The main simulation object.  One GPUCore = one CUDA core / stream processor
# / vector engine — whichever vendor ISA you plug in.
#
# Usage:
#
#   use CodingAdventures::GpuCore;
#   my $isa  = CodingAdventures::GpuCore::GenericISA->new;
#   my $core = CodingAdventures::GpuCore::GPUCore->new(isa => $isa);
#   $core->load_program(CodingAdventures::GpuCore->saxpy_program(2.0, 3.0, 1.0));
#   $core->run;
#   print $core->registers->read(3);   # 7.0

package CodingAdventures::GpuCore::GPUCore;

use strict;
use warnings;
use Carp qw(croak);

our $VERSION = '0.01';

sub new {
  my ($class, %args) = @_;
  my $isa     = $args{isa}           // CodingAdventures::GpuCore::GenericISA->new;
  my $num_reg = $args{num_registers} // 32;
  my $mem_sz  = $args{memory_size}   // 4096;

  return bless {
    isa       => $isa,
    registers => CodingAdventures::GpuCore::FPRegisterFile->new(num_registers => $num_reg),
    memory    => CodingAdventures::GpuCore::LocalMemory->new(size => $mem_sz),
    program   => [],
    pc        => 0,
    cycle     => 0,
    halted    => 0,
  }, $class;
}

sub registers { $_[0]->{registers} }
sub memory    { $_[0]->{memory}    }
sub pc        { $_[0]->{pc}        }
sub cycle     { $_[0]->{cycle}     }
sub halted    { $_[0]->{halted}    }

sub load_program {
  my ($self, $program) = @_;
  $self->{program} = $program;
  $self->{pc}      = 0;
  $self->{cycle}   = 0;
  $self->{halted}  = 0;
}

# Execute one clock cycle.  Returns a trace hash-ref.
sub step {
  my ($self) = @_;

  if ($self->{halted}) {
    return {
      cycle             => $self->{cycle},
      pc                => $self->{pc},
      instruction       => undef,
      description       => 'already halted',
      registers_changed => {},
      memory_changed    => {},
      next_pc           => $self->{pc},
      halted            => 1,
    };
  }

  my $n = scalar @{ $self->{program} };
  if ($self->{pc} < 0 || $self->{pc} >= $n) {
    $self->{halted} = 1;
    return {
      cycle             => $self->{cycle},
      pc                => $self->{pc},
      instruction       => undef,
      description       => sprintf("PC %d out of range — implicit HALT", $self->{pc}),
      registers_changed => {},
      memory_changed    => {},
      next_pc           => $self->{pc},
      halted            => 1,
    };
  }

  my $instr      = $self->{program}[$self->{pc}];
  my $current_pc = $self->{pc};

  my $result = $self->{isa}->execute($instr, $self->{registers}, $self->{memory});

  my $next_pc;
  if (defined $result->{jmp_target}) {
    $next_pc = $result->{jmp_target};
  } else {
    $next_pc = $current_pc + 1 + $result->{next_pc_offset};
  }

  $self->{pc}    = $next_pc;
  $self->{cycle} = $self->{cycle} + 1;
  $self->{halted} = 1 if $result->{halted};

  return {
    cycle             => $self->{cycle} - 1,
    pc                => $current_pc,
    instruction       => $instr,
    description       => $result->{description},
    registers_changed => $result->{registers_changed},
    memory_changed    => $result->{memory_changed},
    next_pc           => $next_pc,
    halted            => $result->{halted} ? 1 : 0,
  };
}

# Run the program until HALT or max_steps is reached.
# Returns an array-ref of trace hash-refs.
sub run {
  my ($self, $max_steps) = @_;
  $max_steps //= 10_000;
  my @traces;
  for (1 .. $max_steps) {
    my $trace = $self->step;
    push @traces, $trace;
    last if $trace->{halted};
  }
  return \@traces;
}

sub reset {
  my ($self) = @_;
  $self->{registers}->reset;
  $self->{memory}->reset;
  $self->{pc}     = 0;
  $self->{cycle}  = 0;
  $self->{halted} = 0;
}

1;
__END__

=head1 NAME

CodingAdventures::GpuCore - Generic accelerator processing element

=head1 SYNOPSIS

  use CodingAdventures::GpuCore;

  my $isa  = CodingAdventures::GpuCore::GenericISA->new;
  my $core = CodingAdventures::GpuCore::GPUCore->new(isa => $isa);

  $core->load_program(CodingAdventures::GpuCore->saxpy_program(2.0, 3.0, 1.0));
  $core->run;
  print $core->registers->read(3);   # 7.0

=head1 DESCRIPTION

Simulates a single GPU processing element with a floating-point register file,
local scratchpad memory, and a pluggable instruction set.

=cut
