use strict;
use warnings;
use Test2::V0;

use CodingAdventures::ARM1Simulator;

# ============================================================
# Construction and Reset
# ============================================================

subtest 'construction and reset' => sub {
    my $cpu = CodingAdventures::ARM1Simulator->new(4096);
    is($cpu->get_mode(), CodingAdventures::ARM1Simulator::MODE_SVC, 'starts in SVC mode');
    is($cpu->{halted}, 0, 'starts not halted');
    is($cpu->get_pc(), 0, 'starts at PC=0');
    ok($cpu->{regs}[15] & CodingAdventures::ARM1Simulator::FLAG_I, 'IRQ disabled on reset');
    ok($cpu->{regs}[15] & CodingAdventures::ARM1Simulator::FLAG_F, 'FIQ disabled on reset');
};

# ============================================================
# Register Access
# ============================================================

subtest 'register access' => sub {
    my $cpu = CodingAdventures::ARM1Simulator->new(4096);
    # Force USR mode
    $cpu->{regs}[15] = ($cpu->{regs}[15] & ~CodingAdventures::ARM1Simulator::MODE_MASK) | CodingAdventures::ARM1Simulator::MODE_USR;

    for my $i (0..12) {
        $cpu->write_register($i, $i * 100);
    }
    for my $i (0..12) {
        is($cpu->read_register($i), $i * 100, "R$i round-trip");
    }

    # 32-bit masking — use arithmetic to avoid non-portable hex literal > 0xFFFFFFFF
    $cpu->write_register(0, 0xFFFFFFFF + 0x100000000);
    is($cpu->read_register(0), 0xFFFFFFFF, 'masks to 32 bits');
};

subtest 'FIQ banking' => sub {
    my $cpu = CodingAdventures::ARM1Simulator->new(4096);
    $cpu->{regs}[15] = ($cpu->{regs}[15] & ~CodingAdventures::ARM1Simulator::MODE_MASK) | CodingAdventures::ARM1Simulator::MODE_USR;
    $cpu->write_register(8, 0x1111);
    $cpu->write_register(9, 0x2222);

    $cpu->{regs}[15] = ($cpu->{regs}[15] & ~CodingAdventures::ARM1Simulator::MODE_MASK) | CodingAdventures::ARM1Simulator::MODE_FIQ;
    $cpu->write_register(8, 0xAAAA);
    $cpu->write_register(9, 0xBBBB);
    is($cpu->read_register(8), 0xAAAA, 'FIQ sees banked R8');

    $cpu->{regs}[15] = ($cpu->{regs}[15] & ~CodingAdventures::ARM1Simulator::MODE_MASK) | CodingAdventures::ARM1Simulator::MODE_USR;
    is($cpu->read_register(8), 0x1111, 'USR sees original R8');
};

subtest 'SVC banking' => sub {
    my $cpu = CodingAdventures::ARM1Simulator->new(4096);
    $cpu->{regs}[15] = ($cpu->{regs}[15] & ~CodingAdventures::ARM1Simulator::MODE_MASK) | CodingAdventures::ARM1Simulator::MODE_USR;
    $cpu->write_register(13, 0xDEAD);
    $cpu->write_register(14, 0xBEEF);

    $cpu->{regs}[15] = ($cpu->{regs}[15] & ~CodingAdventures::ARM1Simulator::MODE_MASK) | CodingAdventures::ARM1Simulator::MODE_SVC;
    $cpu->write_register(13, 0x1234);
    $cpu->write_register(14, 0x5678);
    is($cpu->read_register(13), 0x1234, 'SVC R13');
    is($cpu->read_register(14), 0x5678, 'SVC R14');

    $cpu->{regs}[15] = ($cpu->{regs}[15] & ~CodingAdventures::ARM1Simulator::MODE_MASK) | CodingAdventures::ARM1Simulator::MODE_USR;
    is($cpu->read_register(13), 0xDEAD, 'USR R13 preserved');
};

# ============================================================
# Memory
# ============================================================

subtest 'memory' => sub {
    my $cpu = CodingAdventures::ARM1Simulator->new(4096);
    $cpu->write_word(0x100, 0xDEADBEEF);
    is($cpu->read_word(0x100), 0xDEADBEEF, 'word round-trip');

    # Little-endian check
    is($cpu->read_byte(0x100), 0xEF, 'LE byte 0');
    is($cpu->read_byte(0x101), 0xBE, 'LE byte 1');
    is($cpu->read_byte(0x102), 0xAD, 'LE byte 2');
    is($cpu->read_byte(0x103), 0xDE, 'LE byte 3');

    $cpu->write_byte(0x200, 0xAB);
    is($cpu->read_byte(0x200), 0xAB, 'byte round-trip');

    is($cpu->read_word(0x1000), 0, 'OOB read returns 0');
};

# ============================================================
# Condition Evaluation
# ============================================================

subtest 'evaluate_condition' => sub {
    my $cpu = CodingAdventures::ARM1Simulator->new(4096);

    my $t = sub { {n=>$_[0], z=>$_[1], c=>$_[2], v=>$_[3]} };

    ok($cpu->evaluate_condition(CodingAdventures::ARM1Simulator::COND_EQ, $t->(0,1,0,0)), 'EQ: Z=1');
    ok(!$cpu->evaluate_condition(CodingAdventures::ARM1Simulator::COND_EQ, $t->(0,0,0,0)), 'EQ: Z=0 fails');
    ok($cpu->evaluate_condition(CodingAdventures::ARM1Simulator::COND_NE, $t->(0,0,0,0)), 'NE: Z=0');
    ok($cpu->evaluate_condition(CodingAdventures::ARM1Simulator::COND_CS, $t->(0,0,1,0)), 'CS: C=1');
    ok($cpu->evaluate_condition(CodingAdventures::ARM1Simulator::COND_CC, $t->(0,0,0,0)), 'CC: C=0');
    ok($cpu->evaluate_condition(CodingAdventures::ARM1Simulator::COND_MI, $t->(1,0,0,0)), 'MI: N=1');
    ok($cpu->evaluate_condition(CodingAdventures::ARM1Simulator::COND_PL, $t->(0,0,0,0)), 'PL: N=0');
    ok($cpu->evaluate_condition(CodingAdventures::ARM1Simulator::COND_VS, $t->(0,0,0,1)), 'VS: V=1');
    ok($cpu->evaluate_condition(CodingAdventures::ARM1Simulator::COND_VC, $t->(0,0,0,0)), 'VC: V=0');
    ok($cpu->evaluate_condition(CodingAdventures::ARM1Simulator::COND_HI, $t->(0,0,1,0)), 'HI: C=1,Z=0');
    ok(!$cpu->evaluate_condition(CodingAdventures::ARM1Simulator::COND_HI, $t->(0,1,1,0)), 'HI fails Z=1');
    ok($cpu->evaluate_condition(CodingAdventures::ARM1Simulator::COND_GE, $t->(0,0,0,0)), 'GE: N=V=0');
    ok($cpu->evaluate_condition(CodingAdventures::ARM1Simulator::COND_GE, $t->(1,0,0,1)), 'GE: N=V=1');
    ok($cpu->evaluate_condition(CodingAdventures::ARM1Simulator::COND_LT, $t->(1,0,0,0)), 'LT: N!=V');
    ok($cpu->evaluate_condition(CodingAdventures::ARM1Simulator::COND_GT, $t->(0,0,0,0)), 'GT: Z=0,N=V');
    ok($cpu->evaluate_condition(CodingAdventures::ARM1Simulator::COND_LE, $t->(0,1,0,0)), 'LE: Z=1');
    ok($cpu->evaluate_condition(CodingAdventures::ARM1Simulator::COND_AL, $t->(0,0,0,0)), 'AL');
    ok(!$cpu->evaluate_condition(CodingAdventures::ARM1Simulator::COND_NV, $t->(1,1,1,1)), 'NV never');
};

# ============================================================
# Barrel Shifter
# ============================================================

subtest 'barrel_shift' => sub {
    my $cpu = CodingAdventures::ARM1Simulator->new(4096);

    my ($r, $c) = $cpu->barrel_shift(0xDEAD, CodingAdventures::ARM1Simulator::SHIFT_LSL, 0, 1, 0);
    is($r, 0xDEAD, 'LSL #0 no change');
    is($c, 1, 'LSL #0 preserves carry');

    ($r, $c) = $cpu->barrel_shift(0x80000001, CodingAdventures::ARM1Simulator::SHIFT_LSL, 1, 0, 0);
    is($r, 2, 'LSL #1 result');
    is($c, 1, 'LSL #1 carry from bit 31');

    ($r, $c) = $cpu->barrel_shift(0x80000000, CodingAdventures::ARM1Simulator::SHIFT_LSR, 0, 0, 0);
    is($r, 0, 'LSR #0 imm = LSR #32');
    is($c, 1, 'LSR #32 carry from MSB');

    ($r, $c) = $cpu->barrel_shift(0x80000001, CodingAdventures::ARM1Simulator::SHIFT_LSR, 1, 0, 0);
    is($r, 0x40000000, 'LSR #1 result');
    is($c, 1, 'LSR #1 carry from bit 0');

    ($r, $c) = $cpu->barrel_shift(0x80000000, CodingAdventures::ARM1Simulator::SHIFT_ASR, 1, 0, 0);
    is($r, 0xC0000000, 'ASR #1 sign-extends');

    ($r, $c) = $cpu->barrel_shift(0x80000000, CodingAdventures::ARM1Simulator::SHIFT_ASR, 0, 0, 0);
    is($r, 0xFFFFFFFF, 'ASR #0 imm = all 1s');
    is($c, 1, 'ASR #0 imm carry');

    ($r, $c) = $cpu->barrel_shift(0x12345678, CodingAdventures::ARM1Simulator::SHIFT_ROR, 4, 0, 0);
    is($r, 0x81234567, 'ROR #4 result');

    ($r, $c) = $cpu->barrel_shift(3, CodingAdventures::ARM1Simulator::SHIFT_ROR, 0, 1, 0);
    is($r, 0x80000001, 'RRX result');
    is($c, 1, 'RRX carry out');
};

# ============================================================
# ALU
# ============================================================

subtest 'alu_execute' => sub {
    my $cpu = CodingAdventures::ARM1Simulator->new(4096);

    my $r = $cpu->alu_execute(CodingAdventures::ARM1Simulator::OP_AND, 0xFF, 0x0F, 0, 0, 0);
    is($r->{result}, 0x0F, 'AND result');
    is($r->{write_result}, 1, 'AND writes result');

    $r = $cpu->alu_execute(CodingAdventures::ARM1Simulator::OP_EOR, 0xFF, 0x0F, 0, 0, 0);
    is($r->{result}, 0xF0, 'EOR');

    $r = $cpu->alu_execute(CodingAdventures::ARM1Simulator::OP_MOV, 0, 0xABCD, 0, 0, 0);
    is($r->{result}, 0xABCD, 'MOV');

    $r = $cpu->alu_execute(CodingAdventures::ARM1Simulator::OP_MVN, 0, 0, 0, 0, 0);
    is($r->{result}, 0xFFFFFFFF, 'MVN');

    $r = $cpu->alu_execute(CodingAdventures::ARM1Simulator::OP_ADD, 0xFFFFFFFF, 1, 0, 0, 0);
    is($r->{result}, 0, 'ADD overflow wraps');
    is($r->{c}, 1, 'ADD carry set');
    is($r->{z}, 1, 'ADD Z set');

    $r = $cpu->alu_execute(CodingAdventures::ARM1Simulator::OP_SUB, 5, 3, 0, 0, 0);
    is($r->{result}, 2, 'SUB 5-3=2');
    is($r->{c}, 1, 'SUB no borrow = C=1');

    $r = $cpu->alu_execute(CodingAdventures::ARM1Simulator::OP_SUB, 3, 5, 0, 0, 0);
    is($r->{result}, 0xFFFFFFFE, 'SUB 3-5 wraps');
    is($r->{c}, 0, 'SUB borrow = C=0');

    $r = $cpu->alu_execute(CodingAdventures::ARM1Simulator::OP_TST, 0xFF, 0x0F, 0, 0, 0);
    is($r->{write_result}, 0, 'TST no write');

    $r = $cpu->alu_execute(CodingAdventures::ARM1Simulator::OP_CMP, 5, 5, 0, 0, 0);
    is($r->{z}, 1, 'CMP equal sets Z');
    is($r->{write_result}, 0, 'CMP no write');

    $r = $cpu->alu_execute(CodingAdventures::ARM1Simulator::OP_RSB, 3, 5, 0, 0, 0);
    is($r->{result}, 2, 'RSB 5-3=2');
};

# ============================================================
# Data Processing Instructions
# ============================================================

subtest 'data processing instructions' => sub {
    my $cpu = CodingAdventures::ARM1Simulator->new(4096);
    $cpu->load_instructions([
        $cpu->encode_mov_imm(CodingAdventures::ARM1Simulator::COND_AL, 0, 42),
        $cpu->encode_halt(),
    ]);
    $cpu->run(100);
    is($cpu->read_register(0), 42, 'MOV R0, #42');

    $cpu = CodingAdventures::ARM1Simulator->new(4096);
    $cpu->load_instructions([
        $cpu->encode_mov_imm(CodingAdventures::ARM1Simulator::COND_AL, 1, 10),
        $cpu->encode_mov_imm(CodingAdventures::ARM1Simulator::COND_AL, 2, 20),
        $cpu->encode_alu_reg(CodingAdventures::ARM1Simulator::COND_AL, CodingAdventures::ARM1Simulator::OP_ADD, 0, 0, 1, 2),
        $cpu->encode_halt(),
    ]);
    $cpu->run(100);
    is($cpu->read_register(0), 30, 'ADD R0,R1,R2');
};

subtest 'conditional skip' => sub {
    my $cpu = CodingAdventures::ARM1Simulator->new(4096);
    # MOVS R1, #0 sets Z=1; MOVNE R0, #99 should skip
    my $movs_0 = $cpu->encode_data_processing(
        CodingAdventures::ARM1Simulator::COND_AL, CodingAdventures::ARM1Simulator::OP_MOV, 1, 0, 1, (1 << 25)
    );
    my $movne = $cpu->encode_data_processing(
        CodingAdventures::ARM1Simulator::COND_NE, CodingAdventures::ARM1Simulator::OP_MOV, 0, 0, 0, (1 << 25) | 99
    );
    $cpu->load_instructions([$movs_0, $movne, $cpu->encode_halt()]);
    $cpu->run(100);
    is($cpu->read_register(0), 0, 'MOVNE skipped when Z=1');
};

# ============================================================
# Load/Store
# ============================================================

subtest 'load and store' => sub {
    my $cpu = CodingAdventures::ARM1Simulator->new(4096);
    $cpu->write_register(1, 0x200);
    $cpu->write_register(0, 0x1234);
    $cpu->load_instructions([
        $cpu->encode_str(CodingAdventures::ARM1Simulator::COND_AL, 0, 1, 0, 1),
        $cpu->encode_ldr(CodingAdventures::ARM1Simulator::COND_AL, 2, 1, 0, 1),
        $cpu->encode_halt(),
    ]);
    $cpu->run(100);
    is($cpu->read_register(2), 0x1234, 'STR/LDR round-trip');
};

# ============================================================
# Block Transfer
# ============================================================

subtest 'block transfer' => sub {
    my $cpu = CodingAdventures::ARM1Simulator->new(4096);
    $cpu->write_register(0, 10);
    $cpu->write_register(1, 20);
    $cpu->write_register(2, 30);
    $cpu->write_register(10, 0x400);

    # STMIA R10!, {R0, R1, R2}
    my $stm = $cpu->encode_stm(CodingAdventures::ARM1Simulator::COND_AL, 10, 0x7, 1, 'IA');
    $cpu->load_instructions([$stm, $cpu->encode_halt()]);
    $cpu->run(100);

    is($cpu->read_word(0x400), 10, 'STMIA stored R0');
    is($cpu->read_word(0x404), 20, 'STMIA stored R1');
    is($cpu->read_word(0x408), 30, 'STMIA stored R2');
};

# ============================================================
# Branch
# ============================================================

subtest 'branch' => sub {
    my $cpu = CodingAdventures::ARM1Simulator->new(4096);
    # addr 0: MOV R0, #1
    # addr 4: B +0 (skip MOV R0,#99 at addr 8, land at addr 12=halt)
    # addr 8: MOV R0, #99 — should be skipped
    # addr 12: HALT
    $cpu->load_instructions([
        $cpu->encode_mov_imm(CodingAdventures::ARM1Simulator::COND_AL, 0, 1),     # 0
        $cpu->encode_branch(CodingAdventures::ARM1Simulator::COND_AL, 0, 0),       # 4: B offset=0
        $cpu->encode_mov_imm(CodingAdventures::ARM1Simulator::COND_AL, 0, 99),    # 8
        $cpu->encode_halt(),                                                        # 12
    ]);
    $cpu->run(100);
    is($cpu->read_register(0), 1, 'branch skipped MOV #99');
};

# ============================================================
# SWI Halt
# ============================================================

subtest 'halt' => sub {
    my $cpu = CodingAdventures::ARM1Simulator->new(4096);
    $cpu->load_instructions([$cpu->encode_halt()]);
    $cpu->run(100);
    is($cpu->{halted}, 1, 'halted after SWI');
};

# ============================================================
# End-to-End: Sum 1..10 = 55
# ============================================================

subtest 'sum 1..10 = 55' => sub {
    my $cpu = CodingAdventures::ARM1Simulator->new(4096);

    # addr 0x00: MOV R0, #0  (accumulator)
    # addr 0x04: MOV R1, #1  (counter)
    # addr 0x08: MOV R2, #11 (limit)
    # addr 0x0C: ADD R0, R0, R1
    # addr 0x10: ADD R1, R1, #1
    # addr 0x14: CMP R1, R2
    # addr 0x18: BLT offset=-20 (target 0x0C; branch_base=0x18+4+4=0x20; 0x0C-0x20=-20)
    # addr 0x1C: HALT

    my $MOV_R0_0  = $cpu->encode_mov_imm(CodingAdventures::ARM1Simulator::COND_AL, 0, 0);
    my $MOV_R1_1  = $cpu->encode_mov_imm(CodingAdventures::ARM1Simulator::COND_AL, 1, 1);
    my $MOV_R2_11 = $cpu->encode_mov_imm(CodingAdventures::ARM1Simulator::COND_AL, 2, 11);
    my $ADD_R0    = $cpu->encode_alu_reg(CodingAdventures::ARM1Simulator::COND_AL, CodingAdventures::ARM1Simulator::OP_ADD, 0, 0, 0, 1);
    my $ADD_R1    = $cpu->encode_data_processing(CodingAdventures::ARM1Simulator::COND_AL, CodingAdventures::ARM1Simulator::OP_ADD, 0, 1, 1, (1 << 25) | 1);
    my $CMP_R1_R2 = $cpu->encode_data_processing(CodingAdventures::ARM1Simulator::COND_AL, CodingAdventures::ARM1Simulator::OP_CMP, 1, 1, 0, 2);
    my $BLT_LOOP  = $cpu->encode_branch(CodingAdventures::ARM1Simulator::COND_LT, 0, -20);
    my $HALT      = $cpu->encode_halt();

    $cpu->load_instructions([$MOV_R0_0, $MOV_R1_1, $MOV_R2_11, $ADD_R0, $ADD_R1, $CMP_R1_R2, $BLT_LOOP, $HALT]);

    # Rewrite BLT with correct offset: branch at 0x18, target 0x0C
    # branch_base = pc_after_advance + 4 = (0x18+4)+4 = 0x20
    # offset = 0x0C - 0x20 = -20
    $cpu->write_word(0x18, $cpu->encode_branch(CodingAdventures::ARM1Simulator::COND_LT, 0, -20));

    $cpu->run(10000);
    is($cpu->read_register(0), 55, 'sum 1..10 = 55');
};

done_testing;
