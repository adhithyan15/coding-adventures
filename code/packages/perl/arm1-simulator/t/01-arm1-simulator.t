use strict;
use warnings;
use Test2::V0;

use CodingAdventures::ARM1Simulator;

my $ARM1 = 'CodingAdventures::ARM1Simulator';

# ===========================================================================
# Construction and Reset
# ===========================================================================

subtest 'construction' => sub {
    my $cpu = $ARM1->new(1024);
    ok defined($cpu), 'creates simulator';
    is $cpu->get_pc(), 0, 'PC starts at 0';
    is $cpu->get_mode(), $ARM1->MODE_SVC, 'starts in SVC mode';
    ok $cpu->{regs}[15] & $ARM1->FLAG_I, 'IRQ disabled';
    ok $cpu->{regs}[15] & $ARM1->FLAG_F, 'FIQ disabled';
};

subtest 'reset' => sub {
    my $cpu = $ARM1->new(1024);
    $cpu->write_register(0, 42);
    $cpu->reset();
    is $cpu->read_register(0), 0, 'register cleared on reset';
    is $cpu->get_mode(), $ARM1->MODE_SVC, 'back to SVC after reset';
};

# ===========================================================================
# Memory
# ===========================================================================

subtest 'memory' => sub {
    my $cpu = $ARM1->new(4096);
    $cpu->write_word(0x100, 0xDEADBEEF);
    is $cpu->read_word(0x100), 0xDEADBEEF, 'word round-trip';

    $cpu->write_byte(0x10, 0xAB);
    is $cpu->read_byte(0x10), 0xAB, 'byte round-trip';

    # Little-endian check
    $cpu->write_word(0x200, 0x01020304);
    is $cpu->read_byte(0x200), 0x04, 'little-endian: LSB at lowest address';
    is $cpu->read_byte(0x201), 0x03, 'little-endian: next byte';
    is $cpu->read_byte(0x202), 0x02;
    is $cpu->read_byte(0x203), 0x01;
};

# ===========================================================================
# MOV immediate
# ===========================================================================

subtest 'MOV immediate' => sub {
    my $cpu = $ARM1->new(1024);
    $cpu->load_instructions([
        $ARM1->encode_mov_imm($ARM1->COND_AL, 0, 42),
        $ARM1->encode_halt(),
    ]);
    $cpu->run(100);
    is $cpu->read_register(0), 42, 'MOV R0, #42';
};

# ===========================================================================
# ADD
# ===========================================================================

subtest 'ADD' => sub {
    my $cpu = $ARM1->new(1024);
    $cpu->load_instructions([
        $ARM1->encode_mov_imm($ARM1->COND_AL, 0, 1),
        $ARM1->encode_mov_imm($ARM1->COND_AL, 1, 2),
        $ARM1->encode_alu_reg($ARM1->COND_AL, $ARM1->OP_ADD, 0, 2, 0, 1),
        $ARM1->encode_halt(),
    ]);
    $cpu->run(100);
    is $cpu->read_register(2), 3, 'ADD R2, R0, R1 (1+2=3)';
};

subtest 'ADD carry flag' => sub {
    my $cpu = $ARM1->new(1024);
    $cpu->write_register(0, 0xFFFFFFFF);
    $cpu->write_register(1, 1);
    $cpu->load_instructions([
        $ARM1->encode_alu_reg($ARM1->COND_AL, $ARM1->OP_ADD, 1, 2, 0, 1),
        $ARM1->encode_halt(),
    ]);
    $cpu->run(100);
    is $cpu->read_register(2), 0, 'ADD overflow wraps to 0';
    my $flags = $cpu->get_flags();
    is $flags->{c}, 1, 'carry set';
    is $flags->{z}, 1, 'zero set';
};

# ===========================================================================
# SUB
# ===========================================================================

subtest 'SUB' => sub {
    my $cpu = $ARM1->new(1024);
    $cpu->write_register(0, 10);
    $cpu->write_register(1, 3);
    $cpu->load_instructions([
        $ARM1->encode_alu_reg($ARM1->COND_AL, $ARM1->OP_SUB, 0, 2, 0, 1),
        $ARM1->encode_halt(),
    ]);
    $cpu->run(100);
    is $cpu->read_register(2), 7, 'SUB R2, R0, R1 (10-3=7)';
};

# ===========================================================================
# AND, ORR, EOR, MVN, BIC
# ===========================================================================

subtest 'AND' => sub {
    my $cpu = $ARM1->new(1024);
    $cpu->write_register(0, 0xFF0F);
    $cpu->write_register(1, 0x0FFF);
    $cpu->load_instructions([
        $ARM1->encode_alu_reg($ARM1->COND_AL, $ARM1->OP_AND, 0, 2, 0, 1),
        $ARM1->encode_halt(),
    ]);
    $cpu->run(100);
    is $cpu->read_register(2), 0x0F0F, 'AND';
};

subtest 'ORR' => sub {
    my $cpu = $ARM1->new(1024);
    $cpu->write_register(0, 0xFF00);
    $cpu->write_register(1, 0x00FF);
    $cpu->load_instructions([
        $ARM1->encode_alu_reg($ARM1->COND_AL, $ARM1->OP_ORR, 0, 2, 0, 1),
        $ARM1->encode_halt(),
    ]);
    $cpu->run(100);
    is $cpu->read_register(2), 0xFFFF, 'ORR';
};

subtest 'EOR (XOR with self = 0)' => sub {
    my $cpu = $ARM1->new(1024);
    $cpu->write_register(0, 0xABCDEF01);
    $cpu->load_instructions([
        $ARM1->encode_alu_reg($ARM1->COND_AL, $ARM1->OP_EOR, 0, 1, 0, 0),
        $ARM1->encode_halt(),
    ]);
    $cpu->run(100);
    is $cpu->read_register(1), 0, 'EOR R0, R0 = 0';
};

subtest 'MVN' => sub {
    my $cpu = $ARM1->new(1024);
    $cpu->write_register(0, 0xFFFFFF00);
    $cpu->load_instructions([
        $ARM1->encode_alu_reg($ARM1->COND_AL, $ARM1->OP_MVN, 0, 1, 0, 0),
        $ARM1->encode_halt(),
    ]);
    $cpu->run(100);
    is $cpu->read_register(1), 0x000000FF, 'MVN (NOT)';
};

subtest 'BIC' => sub {
    my $cpu = $ARM1->new(1024);
    $cpu->write_register(0, 0xFFFF);
    $cpu->write_register(1, 0x00FF);
    $cpu->load_instructions([
        $ARM1->encode_alu_reg($ARM1->COND_AL, $ARM1->OP_BIC, 0, 2, 0, 1),
        $ARM1->encode_halt(),
    ]);
    $cpu->run(100);
    is $cpu->read_register(2), 0xFF00, 'BIC bit clear';
};

# ===========================================================================
# Barrel Shifter
# ===========================================================================

subtest 'barrel shifter LSL' => sub {
    my $cpu = $ARM1->new(1024);
    $cpu->write_register(0, 3);
    # ADD R1, R0, R0, LSL #2  → R1 = R0 + (R0 << 2) = 3 + 12 = 15
    my $inst = $ARM1->encode_alu_reg_shift($ARM1->COND_AL, $ARM1->OP_ADD, 0, 1, 0, 0, $ARM1->SHIFT_LSL, 2);
    $cpu->load_instructions([$inst, $ARM1->encode_halt()]);
    $cpu->run(100);
    is $cpu->read_register(1), 15, 'ADD with LSL #2';
};

# ===========================================================================
# Condition Codes
# ===========================================================================

subtest 'EQ condition' => sub {
    my $cpu = $ARM1->new(1024);
    $cpu->write_register(0, 5);
    $cpu->write_register(1, 5);
    $cpu->write_register(2, 0);
    my $subs = $ARM1->encode_alu_reg($ARM1->COND_AL, $ARM1->OP_SUB, 1, 3, 0, 1);
    my $moveq = $ARM1->encode_mov_imm($ARM1->COND_EQ, 2, 99);
    my $movne = $ARM1->encode_mov_imm($ARM1->COND_NE, 2, 77);
    $cpu->load_instructions([$subs, $moveq, $movne, $ARM1->encode_halt()]);
    $cpu->run(100);
    is $cpu->read_register(2), 99, 'EQ executes when Z=1';
};

subtest 'NV never executes' => sub {
    my $cpu = $ARM1->new(1024);
    $cpu->write_register(0, 42);
    my $movnv = $ARM1->encode_mov_imm($ARM1->COND_NV, 0, 99);
    $cpu->load_instructions([$movnv, $ARM1->encode_halt()]);
    $cpu->run(100);
    is $cpu->read_register(0), 42, 'NV never executes, R0 unchanged';
};

subtest 'MI executes when N=1' => sub {
    my $cpu = $ARM1->new(1024);
    $cpu->write_register(0, 3);
    $cpu->write_register(1, 10);
    $cpu->write_register(2, 0);
    my $subs = $ARM1->encode_alu_reg($ARM1->COND_AL, $ARM1->OP_SUB, 1, 3, 0, 1);
    my $movmi = $ARM1->encode_mov_imm($ARM1->COND_MI, 2, 1);
    $cpu->load_instructions([$subs, $movmi, $ARM1->encode_halt()]);
    $cpu->run(100);
    is $cpu->read_register(2), 1, 'MI executes when negative';
};

# ===========================================================================
# Load/Store
# ===========================================================================

subtest 'STR/LDR round-trip' => sub {
    my $cpu = $ARM1->new(4096);
    $cpu->write_register(0, 0xCAFEBABE);
    $cpu->write_register(2, 0x100);
    $cpu->load_instructions([
        $ARM1->encode_str($ARM1->COND_AL, 0, 2, 0, 1),
        $ARM1->encode_ldr($ARM1->COND_AL, 1, 2, 0, 1),
        $ARM1->encode_halt(),
    ]);
    $cpu->run(100);
    is $cpu->read_register(1), 0xCAFEBABE, 'LDR loaded stored value';
};

# ===========================================================================
# Block Transfer
# ===========================================================================

subtest 'STMIA/LDMIA round-trip' => sub {
    my $cpu = $ARM1->new(4096);
    $cpu->write_register(0, 0x11111111);
    $cpu->write_register(1, 0x22222222);
    $cpu->write_register(2, 0x33333333);
    $cpu->write_register(13, 0x200);

    my $stm = $ARM1->encode_stm($ARM1->COND_AL, 13, 0x7, 0, 'IA');
    $cpu->load_instructions([
        $stm,
        $ARM1->encode_mov_imm($ARM1->COND_AL, 0, 0),
        $ARM1->encode_mov_imm($ARM1->COND_AL, 1, 0),
        $ARM1->encode_mov_imm($ARM1->COND_AL, 2, 0),
        $ARM1->encode_ldm($ARM1->COND_AL, 13, 0x7, 0, 'IA'),
        $ARM1->encode_halt(),
    ]);
    $cpu->run(100);
    is $cpu->read_register(0), 0x11111111, 'R0 restored';
    is $cpu->read_register(1), 0x22222222, 'R1 restored';
    is $cpu->read_register(2), 0x33333333, 'R2 restored';
};

# ===========================================================================
# Branch
# ===========================================================================

subtest 'B forward branch' => sub {
    my $cpu = $ARM1->new(4096);
    my $branch = $ARM1->encode_branch($ARM1->COND_AL, 0, 0);
    $cpu->load_instructions([
        $branch,
        $ARM1->encode_mov_imm($ARM1->COND_AL, 0, 99),
        $ARM1->encode_mov_imm($ARM1->COND_AL, 0, 42),
        $ARM1->encode_halt(),
    ]);
    $cpu->run(100);
    is $cpu->read_register(0), 42, 'jumped over first MOV, landed on second';
};

subtest 'BL sets link register' => sub {
    my $cpu = $ARM1->new(4096);
    my $bl = $ARM1->encode_branch($ARM1->COND_AL, 1, 4);
    $cpu->load_instructions([
        $ARM1->encode_mov_imm($ARM1->COND_AL, 0, 7),
        $bl,
        $ARM1->encode_halt(),
        $ARM1->encode_alu_reg($ARM1->COND_AL, $ARM1->OP_MOV, 0, 1, 0, 1),
        $ARM1->encode_alu_reg($ARM1->COND_AL, $ARM1->OP_ADD, 0, 0, 0, 0),
        $ARM1->encode_halt(),
    ]);
    $cpu->run(100);
    is $cpu->read_register(0), 14, 'BL subroutine doubled R0';
    ok $cpu->read_register(14) != 0, 'LR set by BL';
};

# ===========================================================================
# SWI / Halt
# ===========================================================================

subtest 'halt' => sub {
    my $cpu = $ARM1->new(1024);
    $cpu->load_instructions([$ARM1->encode_halt()]);
    my $traces = $cpu->run(100);
    ok $cpu->{halted}, 'CPU halted';
    is scalar(@$traces), 1, 'only 1 instruction executed';
};

# ===========================================================================
# End-to-End: sum 1..10 = 55
# ===========================================================================

subtest 'sum 1 to 10' => sub {
    my $cpu = $ARM1->new(4096);

    # SUBS R1, R1, #1
    my $sub_imm = ($ARM1->COND_AL << 28) | (1 << 25) | ($ARM1->OP_SUB << 21) | (1 << 20) | (1 << 16) | (1 << 12) | 1;
    $sub_imm &= 0xFFFFFFFF;

    my $bne = $ARM1->encode_branch($ARM1->COND_NE, 0, -8);

    $cpu->load_instructions([
        $ARM1->encode_mov_imm($ARM1->COND_AL, 0, 0),
        $ARM1->encode_mov_imm($ARM1->COND_AL, 1, 10),
        $ARM1->encode_alu_reg($ARM1->COND_AL, $ARM1->OP_ADD, 0, 0, 0, 1),
        $sub_imm,
        $bne,
        $ARM1->encode_halt(),
    ]);
    $cpu->run(1000);
    is $cpu->read_register(0), 55, 'sum 1..10 = 55';
};

# ===========================================================================
# Mode Banking
# ===========================================================================

subtest 'register banking' => sub {
    my $cpu = $ARM1->new(1024);
    # In SVC mode, write R13
    $cpu->write_register(13, 0xABCD);
    is $cpu->read_register(13), 0xABCD, 'SVC R13 = 0xABCD';

    # Switch to USR mode
    $cpu->{regs}[15] = ($cpu->{regs}[15] & ~0x3) & 0xFFFFFFFF;
    $cpu->write_register(13, 0x1234);

    # Back to SVC
    $cpu->{regs}[15] = ($cpu->{regs}[15] & ~0x3) | $ARM1->MODE_SVC;
    is $cpu->read_register(13), 0xABCD, 'SVC R13 still 0xABCD';

    # Back to USR
    $cpu->{regs}[15] &= ~0x3;
    is $cpu->read_register(13), 0x1234, 'USR R13 = 0x1234';
};

done_testing;
