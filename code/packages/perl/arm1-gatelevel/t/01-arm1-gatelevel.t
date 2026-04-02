use strict;
use warnings;
use Test2::V0;

require CodingAdventures::ARM1Gatelevel;
require CodingAdventures::ARM1Simulator;

# Aliases for brevity
my $GL   = 'CodingAdventures::ARM1Gatelevel';
my $ARM1 = 'CodingAdventures::ARM1Simulator';

# Helper: build a CPU with instructions loaded at address 0
sub make_cpu {
    my ($instrs, $mem) = @_;
    $mem //= 4096;
    my $cpu = $GL->new($mem);
    $cpu->load_instructions(0, $instrs);
    return $cpu;
}

# =========================================================================
# int_to_bits / bits_to_int
# =========================================================================

subtest 'int_to_bits — zero' => sub {
    my @bits = $GL->can('int_to_bits')->(0, 32);
    is(scalar @bits, 32, '32 bits');
    is($_, 0, "bit[${\($_ + 0)}] = 0") for @bits;
};

subtest 'int_to_bits — 1 (LSB only)' => sub {
    my @bits = $GL->can('int_to_bits')->(1, 32);
    is($bits[0], 1,  'bits[0] = 1');
    is($bits[1], 0,  'bits[1] = 0');
    is($bits[31], 0, 'bits[31] = 0');
};

subtest 'int_to_bits — 0x80000000 (MSB only)' => sub {
    my @bits = $GL->can('int_to_bits')->(0x80000000, 32);
    is($bits[31], 1, 'bits[31] = 1 (MSB)');
    is($bits[0],  0, 'bits[0]  = 0 (LSB)');
};

subtest 'bits_to_int roundtrip' => sub {
    for my $v (0, 1, 0xFF, 0x1234, 0xDEADBEEF, 0xFFFFFFFF) {
        my @bits = $GL->can('int_to_bits')->($v, 32);
        my $back = $GL->can('bits_to_int')->(\@bits);
        is($back, $v & 0xFFFFFFFF, "roundtrip 0x" . sprintf('%X', $v));
    }
};

# =========================================================================
# Gate-level barrel shifter
# =========================================================================

subtest 'barrel_shift — LSL #0 no change' => sub {
    my $cpu = $GL->new(256);
    my @bits = $GL->can('int_to_bits')->(0xABCDEF01, 32);
    my ($r_ref, $cout) = $cpu->gate_barrel_shift(\@bits, $GL->SHIFT_LSL, 0, 0, 0);
    is($GL->can('bits_to_int')->($r_ref), 0xABCDEF01, 'value unchanged');
    is($cout, 0, 'carry=0');
};

subtest 'barrel_shift — LSL #1' => sub {
    my $cpu = $GL->new(256);
    my @bits = $GL->can('int_to_bits')->(1, 32);
    my ($r_ref, $cout) = $cpu->gate_barrel_shift(\@bits, $GL->SHIFT_LSL, 1, 0, 0);
    is($GL->can('bits_to_int')->($r_ref), 2, 'result = 2');
    is($cout, 0, 'carry = 0');
};

subtest 'barrel_shift — LSL #1 carry from MSB' => sub {
    my $cpu = $GL->new(256);
    my @bits = $GL->can('int_to_bits')->(0x80000001, 32);
    my ($r_ref, $cout) = $cpu->gate_barrel_shift(\@bits, $GL->SHIFT_LSL, 1, 0, 0);
    is($GL->can('bits_to_int')->($r_ref), 2, 'result = 2');
    is($cout, 1, 'carry = 1 (MSB shifted out)');
};

subtest 'barrel_shift — LSR #1' => sub {
    my $cpu = $GL->new(256);
    my @bits = $GL->can('int_to_bits')->(0x80000000, 32);
    my ($r_ref, $cout) = $cpu->gate_barrel_shift(\@bits, $GL->SHIFT_LSR, 1, 0, 0);
    is($GL->can('bits_to_int')->($r_ref), 0x40000000, 'result = 0x40000000');
};

subtest 'barrel_shift — LSR #1 carry from LSB' => sub {
    my $cpu = $GL->new(256);
    my @bits = $GL->can('int_to_bits')->(3, 32);
    my ($r_ref, $cout) = $cpu->gate_barrel_shift(\@bits, $GL->SHIFT_LSR, 1, 0, 0);
    is($GL->can('bits_to_int')->($r_ref), 1, 'result = 1');
    is($cout, 1, 'carry = 1 (LSB shifted out)');
};

subtest 'barrel_shift — ASR #1 negative' => sub {
    my $cpu = $GL->new(256);
    my @bits = $GL->can('int_to_bits')->(0x80000000, 32);
    my ($r_ref, $cout) = $cpu->gate_barrel_shift(\@bits, $GL->SHIFT_ASR, 1, 0, 0);
    is($GL->can('bits_to_int')->($r_ref), 0xC0000000, 'sign bit preserved');
};

subtest 'barrel_shift — ROR #1' => sub {
    my $cpu = $GL->new(256);
    my @bits = $GL->can('int_to_bits')->(1, 32);
    my ($r_ref, $cout) = $cpu->gate_barrel_shift(\@bits, $GL->SHIFT_ROR, 1, 0, 0);
    is($GL->can('bits_to_int')->($r_ref), 0x80000000, 'bit 0 rotates to MSB');
    is($cout, 1, 'carry = MSB of result');
};

subtest 'barrel_shift — RRX (ROR #0 immediate)' => sub {
    my $cpu = $GL->new(256);
    # carry_in=1, value=3 (bits[0]=1)
    # carry_out = bits[0] = 1; MSB = carry_in = 1
    my @bits = $GL->can('int_to_bits')->(3, 32);
    my ($r_ref, $cout) = $cpu->gate_barrel_shift(\@bits, $GL->SHIFT_ROR, 0, 1, 0);
    is($cout, 1, 'carry_out = old LSB');
    is($GL->can('bits_to_int')->($r_ref), 0x80000001, 'carry_in becomes MSB');
};

# =========================================================================
# Gate-level ALU — all 16 operations
# =========================================================================

sub alu {
    my ($op, $a, $b, $cin, $sc, $ov) = @_;
    my $cpu = CodingAdventures::ARM1Gatelevel->new(256);
    return $cpu->gate_alu_execute($op, $a, $b, $cin//0, $sc//0, $ov//0);
}

subtest 'ALU AND' => sub {
    my $r = alu($GL->OP_AND, 0xF0, 0xFF);
    is($r->{result}, 0xF0, 'AND result');
    is($r->{write_result}, 1, 'write_result=1');
};

subtest 'ALU EOR' => sub {
    my $r = alu($GL->OP_EOR, 0xFF, 0x0F);
    is($r->{result}, 0xF0, 'EOR result');
};

subtest 'ALU SUB 5-3=2' => sub {
    my $r = alu($GL->OP_SUB, 5, 3);
    is($r->{result}, 2, 'SUB result');
    is($r->{n}, 0, 'N=0');
    is($r->{z}, 0, 'Z=0');
};

subtest 'ALU SUB 0-0=0 (Z)' => sub {
    my $r = alu($GL->OP_SUB, 0, 0);
    is($r->{result}, 0, 'result=0');
    is($r->{z}, 1, 'Z=1');
};

subtest 'ALU RSB b-a' => sub {
    my $r = alu($GL->OP_RSB, 5, 10);
    is($r->{result}, 5, 'RSB result');
};

subtest 'ALU ADD 3+4=7' => sub {
    my $r = alu($GL->OP_ADD, 3, 4);
    is($r->{result}, 7, 'ADD result');
    is($r->{c}, 0, 'no carry');
};

subtest 'ALU ADD carry out' => sub {
    my $r = alu($GL->OP_ADD, 0xFFFFFFFF, 1);
    is($r->{result}, 0, 'wraps to 0');
    is($r->{c}, 1, 'carry=1');
    is($r->{z}, 1, 'Z=1');
};

subtest 'ALU ADC 3+4+1=8' => sub {
    my $r = alu($GL->OP_ADC, 3, 4, 1);
    is($r->{result}, 8, 'ADC result');
};

subtest 'ALU SBC 5-3-0=2 (carry=1 means no borrow)' => sub {
    my $r = alu($GL->OP_SBC, 5, 3, 1);
    is($r->{result}, 2, 'SBC result');
};

subtest 'ALU RSC b-a-NOT(carry)' => sub {
    my $r = alu($GL->OP_RSC, 3, 5, 1);
    is($r->{result}, 2, 'RSC result');
};

subtest 'ALU TST no write' => sub {
    my $r = alu($GL->OP_TST, 0xFF, 0x0F);
    is($r->{write_result}, 0, 'write_result=0 for TST');
    is($r->{z}, 0, 'non-zero result');
};

subtest 'ALU TEQ EOR no write' => sub {
    my $r = alu($GL->OP_TEQ, 5, 5);
    is($r->{write_result}, 0, 'write_result=0 for TEQ');
    is($r->{z}, 1, 'Z=1 (0 XOR 0 via equal values)');
};

subtest 'ALU CMP no write' => sub {
    my $r = alu($GL->OP_CMP, 5, 5);
    is($r->{write_result}, 0, 'write_result=0 for CMP');
    is($r->{z}, 1, 'Z=1');
};

subtest 'ALU CMN no write' => sub {
    my $r = alu($GL->OP_CMN, 0xFFFFFFFF, 1);
    is($r->{write_result}, 0, 'write_result=0 for CMN');
    is($r->{z}, 1, 'Z=1');
    is($r->{c}, 1, 'C=1');
};

subtest 'ALU ORR' => sub {
    my $r = alu($GL->OP_ORR, 0xF0, 0x0F);
    is($r->{result}, 0xFF, 'ORR result');
};

subtest 'ALU MOV' => sub {
    my $r = alu($GL->OP_MOV, 0, 0x12345678);
    is($r->{result}, 0x12345678, 'MOV result');
};

subtest 'ALU BIC a AND NOT(b)' => sub {
    my $r = alu($GL->OP_BIC, 0xFF, 0x0F);
    is($r->{result}, 0xF0, 'BIC result');
};

subtest 'ALU MVN NOT(b)' => sub {
    my $r = alu($GL->OP_MVN, 0, 0);
    is($r->{result}, 0xFFFFFFFF, 'MVN of 0 = 0xFFFFFFFF');
};

subtest 'ALU N flag (negative result)' => sub {
    my $r = alu($GL->OP_MOV, 0, 0x80000000);
    is($r->{n}, 1, 'N=1 for negative result');
};

subtest 'ALU Z flag (zero result)' => sub {
    my $r = alu($GL->OP_MOV, 0, 0);
    is($r->{z}, 1, 'Z=1 for zero result');
};

subtest 'ALU V flag (signed overflow ADD)' => sub {
    # 0x7FFFFFFF + 1: both positive, result negative → overflow
    my $r = alu($GL->OP_ADD, 0x7FFFFFFF, 1);
    is($r->{v}, 1, 'V=1 on signed overflow');
};

# =========================================================================
# Full simulation — MOV, ADD
# =========================================================================

subtest 'MOV R0, #42' => sub {
    my $cpu = make_cpu([
        $ARM1->can('encode_mov_imm')->($GL->COND_AL, 0, 42),
        $ARM1->can('encode_halt')->(),
    ]);
    $cpu->run(10);
    is($cpu->read_register(0), 42, 'R0 = 42');
};

subtest 'ADD R2, R0, R1' => sub {
    my $cpu = make_cpu([
        $ARM1->can('encode_mov_imm')->($GL->COND_AL, 0, 10),
        $ARM1->can('encode_mov_imm')->($GL->COND_AL, 1, 20),
        $ARM1->can('encode_alu_reg')->($GL->COND_AL, $GL->OP_ADD, 0, 2, 0, 1),
        $ARM1->can('encode_halt')->(),
    ]);
    $cpu->run(10);
    is($cpu->read_register(2), 30, 'R2 = 30');
};

subtest 'gate_ops increases' => sub {
    my $cpu = make_cpu([
        $ARM1->can('encode_mov_imm')->($GL->COND_AL, 0, 1),
        $ARM1->can('encode_mov_imm')->($GL->COND_AL, 1, 2),
        $ARM1->can('encode_halt')->(),
    ]);
    is($cpu->{gate_ops}, 0, 'starts at 0');
    $cpu->step();
    ok($cpu->{gate_ops} > 0, 'gate_ops > 0 after first step');
    my $ops1 = $cpu->{gate_ops};
    $cpu->step();
    ok($cpu->{gate_ops} > $ops1, 'gate_ops increases after second step');
};

# =========================================================================
# Conditional execution
# =========================================================================

subtest 'MOVEQ executes when Z=1, MOVNE skipped' => sub {
    # MOV R0, #0; CMP R0, R0 (Z=1); MOVEQ R1, #99; MOVNE R2, #77; HALT
    my $cpu = make_cpu([
        $ARM1->can('encode_mov_imm')->($GL->COND_AL, 0, 0),
        # CMP R0, R0: encode_data_processing(cond, CMP, s=1, rn=R0, rd=0, operand2=R0)
        $ARM1->can('encode_data_processing')->($GL->COND_AL, $GL->OP_CMP, 1, 0, 0, 0),
        $ARM1->can('encode_mov_imm')->($GL->COND_EQ, 1, 99),
        $ARM1->can('encode_mov_imm')->($GL->COND_NE, 2, 77),
        $ARM1->can('encode_halt')->(),
    ]);
    $cpu->run(20);
    is($cpu->read_register(1), 99, 'R1 = 99 (MOVEQ executed)');
    is($cpu->read_register(2), 0,  'R2 = 0  (MOVNE skipped)');
};

# =========================================================================
# Integration — sum 1..10 = 55
# =========================================================================

subtest 'sum 1 to 10 = 55' => sub {
    #
    # Layout:
    #   0x00  MOV R0, #1       ; counter = 1
    #   0x04  MOV R1, #0       ; sum = 0
    #   0x08  ADD R1, R1, R0   ; sum += counter   (loop_top)
    #   0x0C  ADD R0, R0, #1   ; counter++
    #   0x10  CMP R0, #11
    #   0x14  BLT -20          ; branch to 0x08 if counter < 11
    #   0x18  SWI HALT_SWI
    #
    # BLT at 0x14: branch_base = (0x14+4)+4 = 0x1C; target=0x08; offset = 0x08-0x1C = -20

    my $COND_AL  = $GL->COND_AL;
    my $COND_LT  = $GL->COND_LT;
    my $OP_ADD   = $GL->OP_ADD;
    my $OP_CMP   = $GL->OP_CMP;

    # ADD R0, R0, #1 (immediate): operand2 = (1<<25) | imm8
    my $add_r0_1 = $ARM1->can('encode_data_processing')->(
        $COND_AL, $OP_ADD, 0, 0, 0, (1 << 25) | 1
    );
    # CMP R0, #11 (immediate, s=1): operand2 = (1<<25) | 11
    my $cmp_r0_11 = $ARM1->can('encode_data_processing')->(
        $COND_AL, $OP_CMP, 1, 0, 0, (1 << 25) | 11
    );

    my $cpu = CodingAdventures::ARM1Gatelevel->new(4096);
    $cpu->load_instructions(0, [
        $ARM1->can('encode_mov_imm')->($COND_AL, 0, 1),                     # 0x00
        $ARM1->can('encode_mov_imm')->($COND_AL, 1, 0),                     # 0x04
        $ARM1->can('encode_alu_reg')->($COND_AL, $OP_ADD, 0, 1, 1, 0),    # 0x08 ADD R1,R1,R0
        $add_r0_1,                                                           # 0x0C ADD R0,R0,#1
        $cmp_r0_11,                                                          # 0x10 CMP R0,#11
        $ARM1->can('encode_branch')->($COND_LT, 0, -20),                   # 0x14 BLT -20
        $ARM1->can('encode_halt')->(),                                       # 0x18
    ]);
    $cpu->run(500);

    is($cpu->read_register(1), 55, 'R1 = 55 (sum 1..10)');
    ok($cpu->{gate_ops} > 0, 'gate_ops is positive');
};

# =========================================================================
# Gate-level vs behavioral equivalence
# =========================================================================

subtest 'gate-level matches behavioral for short sequence' => sub {
    my @prog = (
        $ARM1->can('encode_mov_imm')->($GL->COND_AL, 0, 7),
        $ARM1->can('encode_mov_imm')->($GL->COND_AL, 1, 3),
        $ARM1->can('encode_alu_reg')->($GL->COND_AL, $GL->OP_SUB, 0, 2, 0, 1),
        $ARM1->can('encode_halt')->(),
    );

    my $cpu_gl  = CodingAdventures::ARM1Gatelevel->new(4096);
    my $cpu_beh = CodingAdventures::ARM1Simulator->new(4096);

    $cpu_gl->load_instructions(0, \@prog);
    $cpu_beh->load_instructions(0, \@prog);

    $cpu_gl->run(10);
    $cpu_beh->run(10);

    for my $i (0 .. 13) {
        is(
            $cpu_gl->read_register($i),
            $cpu_beh->read_register($i),
            "R$i matches"
        );
    }
};

done_testing();
