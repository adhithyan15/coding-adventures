use strict;
use warnings;
use Test2::V0;

require CodingAdventures::GpuCore;

my $pkg = 'CodingAdventures::GpuCore';

# ============================================================================
# Instruction constructors
# ============================================================================

subtest 'instruction constructors' => sub {
  my $i = $pkg->fadd(1, 2, 3);
  is($i->{opcode}, 'FADD', 'fadd opcode');
  is($i->{rd}, 1, 'fadd rd');
  is($i->{rs1}, 2, 'fadd rs1');
  is($i->{rs2}, 3, 'fadd rs2');

  my $j = $pkg->ffma(0, 1, 2, 3);
  is($j->{opcode}, 'FFMA', 'ffma opcode');
  is($j->{rs3}, 3, 'ffma rs3');

  my $l = $pkg->limm(5, 3.14);
  is($l->{opcode}, 'LIMM', 'limm opcode');
  is($l->{rd}, 5, 'limm rd');
  ok(abs($l->{immediate} - 3.14) < 1e-6, 'limm immediate');

  my $h = $pkg->halt;
  is($h->{opcode}, 'HALT', 'halt opcode');

  my $b = $pkg->beq(0, 1, -3);
  is($b->{opcode}, 'BEQ', 'beq opcode');
  is($b->{immediate}, -3, 'beq offset');

  my $jmp = $pkg->jmp(7);
  is($jmp->{opcode}, 'JMP', 'jmp opcode');
  is($jmp->{immediate}, 7, 'jmp target');
};

# ============================================================================
# FPRegisterFile
# ============================================================================

subtest 'FPRegisterFile' => sub {
  my $rf = CodingAdventures::GpuCore::FPRegisterFile->new(num_registers => 8);

  # All zeros initially
  for my $i (0..7) {
    is($rf->read($i), 0.0, "register $i initialized to 0");
  }

  $rf->write(3, 3.14);
  ok(abs($rf->read(3) - 3.14) < 1e-6, 'write and read');

  is($rf->size, 8, 'size');

  $rf->reset;
  is($rf->read(3), 0.0, 'reset clears registers');

  ok( dies { $rf->read(-1) },  'read -1 dies' );
  ok( dies { $rf->read(8) },   'read 8 dies' );
  ok( dies { $rf->write(8, 1.0) }, 'write 8 dies' );
};

# ============================================================================
# LocalMemory
# ============================================================================

subtest 'LocalMemory' => sub {
  my $mem = CodingAdventures::GpuCore::LocalMemory->new(size => 16);

  is($mem->load(0), 0.0, 'unwritten address returns 0.0');

  $mem->store(5, 2.718);
  ok(abs($mem->load(5) - 2.718) < 1e-6, 'store and load');

  $mem->reset;
  is($mem->load(5), 0.0, 'reset clears memory');

  ok( dies { $mem->load(-1) }, 'load -1 dies' );
  ok( dies { $mem->load(16) }, 'load out-of-bounds dies' );
  ok( dies { $mem->store(16, 1.0) }, 'store out-of-bounds dies' );
};

# ============================================================================
# GenericISA — arithmetic
# ============================================================================

subtest 'GenericISA arithmetic' => sub {
  my $isa = CodingAdventures::GpuCore::GenericISA->new;
  my $rf  = CodingAdventures::GpuCore::FPRegisterFile->new(num_registers => 8);
  my $mem = CodingAdventures::GpuCore::LocalMemory->new(size => 64);

  # FADD
  $rf->write(1, 3.0); $rf->write(2, 4.0);
  $isa->execute($pkg->fadd(0, 1, 2), $rf, $mem);
  ok(abs($rf->read(0) - 7.0) < 1e-6, 'FADD');

  # FSUB
  $rf->write(1, 10.0); $rf->write(2, 3.0);
  $isa->execute($pkg->fsub(0, 1, 2), $rf, $mem);
  ok(abs($rf->read(0) - 7.0) < 1e-6, 'FSUB');

  # FMUL
  $rf->write(1, 3.0); $rf->write(2, 5.0);
  $isa->execute($pkg->fmul(0, 1, 2), $rf, $mem);
  ok(abs($rf->read(0) - 15.0) < 1e-6, 'FMUL');

  # FFMA: R3 = R0 * R1 + R2 = 2 * 3 + 1 = 7
  $rf->write(4, 2.0); $rf->write(5, 3.0); $rf->write(6, 1.0);
  $isa->execute($pkg->ffma(7, 4, 5, 6), $rf, $mem);
  ok(abs($rf->read(7) - 7.0) < 1e-6, 'FFMA');

  # FNEG
  $rf->write(1, 5.0);
  $isa->execute($pkg->fneg(0, 1), $rf, $mem);
  ok(abs($rf->read(0) - (-5.0)) < 1e-6, 'FNEG');

  # FABS
  $rf->write(1, -7.5);
  $isa->execute($pkg->fabs(0, 1), $rf, $mem);
  ok(abs($rf->read(0) - 7.5) < 1e-6, 'FABS');

  # MOV
  $rf->write(2, 9.9);
  $isa->execute($pkg->mov(0, 2), $rf, $mem);
  ok(abs($rf->read(0) - 9.9) < 1e-6, 'MOV');

  # LIMM
  $isa->execute($pkg->limm(3, 3.14159), $rf, $mem);
  ok(abs($rf->read(3) - 3.14159) < 1e-5, 'LIMM');
};

# ============================================================================
# GenericISA — memory
# ============================================================================

subtest 'GenericISA memory' => sub {
  my $isa = CodingAdventures::GpuCore::GenericISA->new;
  my $rf  = CodingAdventures::GpuCore::FPRegisterFile->new(num_registers => 8);
  my $mem = CodingAdventures::GpuCore::LocalMemory->new(size => 64);

  # STORE
  $rf->write(0, 0.0);  $rf->write(1, 42.0);
  $isa->execute($pkg->store(0, 1, 5), $rf, $mem);
  ok(abs($mem->load(5) - 42.0) < 1e-6, 'STORE writes to memory');

  # LOAD
  $mem->store(10, 3.14);
  $rf->write(1, 5.0);
  $isa->execute($pkg->load(0, 1, 5), $rf, $mem);
  ok(abs($rf->read(0) - 3.14) < 1e-5, 'LOAD reads from memory');

  # LOAD from unwritten address
  $rf->write(1, 0.0);
  $isa->execute($pkg->load(0, 1, 20), $rf, $mem);
  is($rf->read(0), 0.0, 'LOAD unwritten returns 0.0');
};

# ============================================================================
# GenericISA — control flow
# ============================================================================

subtest 'GenericISA control flow' => sub {
  my $isa = CodingAdventures::GpuCore::GenericISA->new;
  my $rf  = CodingAdventures::GpuCore::FPRegisterFile->new(num_registers => 8);
  my $mem = CodingAdventures::GpuCore::LocalMemory->new(size => 64);

  # BEQ taken
  $rf->write(0, 5.0); $rf->write(1, 5.0);
  my $r = $isa->execute($pkg->beq(0, 1, 3), $rf, $mem);
  is($r->{next_pc_offset}, 3, 'BEQ taken');

  # BEQ not taken
  $rf->write(0, 5.0); $rf->write(1, 6.0);
  $r = $isa->execute($pkg->beq(0, 1, 3), $rf, $mem);
  is($r->{next_pc_offset}, 0, 'BEQ not taken');

  # BLT taken
  $rf->write(0, 2.0); $rf->write(1, 5.0);
  $r = $isa->execute($pkg->blt(0, 1, -2), $rf, $mem);
  is($r->{next_pc_offset}, -2, 'BLT taken');

  # BLT not taken
  $rf->write(0, 5.0); $rf->write(1, 3.0);
  $r = $isa->execute($pkg->blt(0, 1, -2), $rf, $mem);
  is($r->{next_pc_offset}, 0, 'BLT not taken');

  # BNE taken
  $rf->write(0, 1.0); $rf->write(1, 2.0);
  $r = $isa->execute($pkg->bne(0, 1, 4), $rf, $mem);
  is($r->{next_pc_offset}, 4, 'BNE taken');

  # BNE not taken
  $rf->write(0, 3.0); $rf->write(1, 3.0);
  $r = $isa->execute($pkg->bne(0, 1, 4), $rf, $mem);
  is($r->{next_pc_offset}, 0, 'BNE not taken');

  # JMP
  $r = $isa->execute($pkg->jmp(10), $rf, $mem);
  is($r->{jmp_target}, 10, 'JMP target');

  # NOP
  $r = $isa->execute($pkg->nop, $rf, $mem);
  ok(!$r->{halted}, 'NOP not halted');
  is($r->{next_pc_offset}, 0, 'NOP offset=0');

  # HALT
  $r = $isa->execute($pkg->halt, $rf, $mem);
  ok($r->{halted}, 'HALT sets halted');

  # Unknown opcode
  ok( dies { $isa->execute({opcode=>'BOGUS', rd=>0, rs1=>0, rs2=>0, rs3=>0, immediate=>0}, $rf, $mem) },
      'unknown opcode dies' );
};

# ============================================================================
# GPUCore
# ============================================================================

subtest 'GPUCore basic' => sub {
  my $core = CodingAdventures::GpuCore::GPUCore->new;
  ok(!$core->halted, 'starts not halted');
  is($core->pc, 0, 'starts at pc=0');
  is($core->cycle, 0, 'starts at cycle=0');

  $core->load_program([$pkg->halt]);
  is($core->pc, 0, 'load_program resets pc');

  my $trace = $core->step;
  ok(ref $trace eq 'HASH', 'step returns hash-ref');
  ok($trace->{halted}, 'HALT trace is halted');
};

subtest 'GPUCore LIMM and registers' => sub {
  my $core = CodingAdventures::GpuCore::GPUCore->new;
  $core->load_program([
    $pkg->limm(0, 42.0),
    $pkg->halt,
  ]);
  $core->step;   # LIMM
  $core->step;   # HALT
  ok(abs($core->registers->read(0) - 42.0) < 1e-6, 'LIMM stored value');
  ok($core->halted, 'core is halted after HALT');
};

subtest 'GPUCore run returns traces' => sub {
  my $core = CodingAdventures::GpuCore::GPUCore->new;
  $core->load_program([$pkg->nop, $pkg->halt]);
  my $traces = $core->run;
  is(scalar @$traces, 2, '2 traces for nop+halt');
  ok($traces->[-1]{halted}, 'last trace is halted');
};

subtest 'GPUCore run stops at HALT' => sub {
  my $core = CodingAdventures::GpuCore::GPUCore->new;
  $core->load_program([$pkg->halt, $pkg->nop]);
  my $traces = $core->run;
  is(scalar @$traces, 1, 'stops after first HALT');
};

subtest 'GPUCore empty program halts' => sub {
  my $core = CodingAdventures::GpuCore::GPUCore->new;
  $core->load_program([]);
  my $trace = $core->step;
  ok($trace->{halted}, 'empty program halts');
};

subtest 'GPUCore reset' => sub {
  my $core = CodingAdventures::GpuCore::GPUCore->new;
  $core->load_program([$pkg->limm(0, 5.0), $pkg->halt]);
  $core->run;
  $core->reset;
  is($core->registers->read(0), 0.0, 'reset clears registers');
  is($core->pc,    0, 'reset clears pc');
  ok(!$core->halted, 'reset clears halted');
  is($core->cycle, 0, 'reset clears cycle');
};

subtest 'GPUCore max_steps limit' => sub {
  # Infinite loop
  my $core = CodingAdventures::GpuCore::GPUCore->new;
  $core->load_program([$pkg->jmp(0)]);
  my $traces = $core->run(5);
  is(scalar @$traces, 5, 'max_steps limits execution');
  ok(!$traces->[-1]{halted}, 'not halted when stopped by max_steps');
};

# ============================================================================
# Complete programs
# ============================================================================

subtest 'SAXPY: 2.0 * 3.0 + 1.0 = 7.0' => sub {
  my $core = CodingAdventures::GpuCore::GPUCore->new;
  $core->load_program($pkg->saxpy_program(2.0, 3.0, 1.0));
  $core->run;
  ok(abs($core->registers->read(3) - 7.0) < 1e-6, 'SAXPY result');
};

subtest 'SAXPY: 1.5 * 4.0 + 0.5 = 6.5' => sub {
  my $core = CodingAdventures::GpuCore::GPUCore->new;
  $core->load_program($pkg->saxpy_program(1.5, 4.0, 0.5));
  $core->run;
  ok(abs($core->registers->read(3) - 6.5) < 1e-6, 'SAXPY result 2');
};

subtest 'dot product [1,2,3]·[4,5,6] = 32' => sub {
  my $core = CodingAdventures::GpuCore::GPUCore->new;
  $core->load_program($pkg->dot_product_program);
  $core->run;
  ok(abs($core->registers->read(6) - 32.0) < 1e-6, 'dot product result');
};

subtest 'loop: sum 1+2+3+4 = 10' => sub {
  my $core = CodingAdventures::GpuCore::GPUCore->new;
  $core->load_program([
    $pkg->limm(0, 0.0),    # R0 = sum
    $pkg->limm(1, 1.0),    # R1 = i
    $pkg->limm(2, 1.0),    # R2 = step
    $pkg->limm(3, 5.0),    # R3 = limit
    $pkg->fadd(0, 0, 1),   # sum += i
    $pkg->fadd(1, 1, 2),   # i += 1
    $pkg->blt(1, 3, -2),   # if i < 5: back 2
    $pkg->halt,
  ]);
  $core->run;
  ok(abs($core->registers->read(0) - 10.0) < 1e-6, 'loop sum = 10');
};

subtest 'FABS: |-3.14| = 3.14' => sub {
  my $core = CodingAdventures::GpuCore::GPUCore->new;
  $core->load_program([
    $pkg->limm(0, -3.14),
    $pkg->fabs(1, 0),
    $pkg->halt,
  ]);
  $core->run;
  ok(abs($core->registers->read(1) - 3.14) < 1e-5, 'FABS result');
};

subtest 'JMP skips instructions' => sub {
  my $core = CodingAdventures::GpuCore::GPUCore->new;
  $core->load_program([
    $pkg->jmp(2),           # jump to index 2
    $pkg->limm(0, 99.0),    # skipped
    $pkg->limm(1, 7.0),
    $pkg->halt,
  ]);
  $core->run;
  ok(abs($core->registers->read(0) - 0.0) < 1e-6, 'JMP skips R0=99');
  ok(abs($core->registers->read(1) - 7.0) < 1e-6, 'JMP lands on R1=7');
};

subtest 'memory store-load roundtrip' => sub {
  my $core = CodingAdventures::GpuCore::GPUCore->new;
  $core->load_program([
    $pkg->limm(0, 0.0),
    $pkg->limm(1, 3.14),
    $pkg->store(0, 1, 10),
    $pkg->load(2, 0, 10),
    $pkg->halt,
  ]);
  $core->run;
  ok(abs($core->registers->read(2) - 3.14) < 1e-5, 'memory roundtrip');
};

subtest 'BEQ branches on equality' => sub {
  my $core = CodingAdventures::GpuCore::GPUCore->new;
  $core->load_program([
    $pkg->limm(0, 5.0),
    $pkg->limm(1, 5.0),
    $pkg->beq(0, 1, 1),     # skip PC 3
    $pkg->limm(2, 99.0),    # should be skipped
    $pkg->halt,
  ]);
  $core->run;
  ok(abs($core->registers->read(2) - 0.0) < 1e-6, 'BEQ skips instruction');
};

subtest 'trace records cycle and register changes' => sub {
  my $core = CodingAdventures::GpuCore::GPUCore->new;
  $core->load_program([$pkg->limm(2, 5.0), $pkg->halt]);
  my $t1 = $core->step;
  my $t2 = $core->step;
  is($t1->{cycle}, 0, 'first trace cycle=0');
  is($t2->{cycle}, 1, 'second trace cycle=1');
  ok(abs($t1->{registers_changed}{2} - 5.0) < 1e-6, 'trace records register change');
  ok(ref($t1->{description}) || length($t1->{description}) > 0, 'trace has description');
};

done_testing;
