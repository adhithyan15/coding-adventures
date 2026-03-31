use strict;
use warnings;
use Test2::V0;

use CodingAdventures::FPGA::LUT;
use CodingAdventures::FPGA::Slice;
use CodingAdventures::FPGA::CLB;
use CodingAdventures::FPGA::SwitchMatrix;
use CodingAdventures::FPGA::IOBlock;
use CodingAdventures::FPGA::Bitstream;
use CodingAdventures::FPGA::Fabric;

# ============================================================
# LUT
# ============================================================

subtest 'LUT' => sub {
    my $lut = CodingAdventures::FPGA::LUT->new(4);
    is($lut->{num_inputs}, 4, '4-input LUT');
    is(scalar @{$lut->{truth_table}}, 16, '16 entries');

    $lut = CodingAdventures::FPGA::LUT->new(2);
    $lut->configure([0, 0, 0, 1]);
    is($lut->evaluate([0,0]), 0, 'AND 00');
    is($lut->evaluate([0,1]), 0, 'AND 01');
    is($lut->evaluate([1,0]), 0, 'AND 10');
    is($lut->evaluate([1,1]), 1, 'AND 11');

    # OR
    $lut->configure([0, 1, 1, 1]);
    is($lut->evaluate([0,0]), 0, 'OR 00');
    is($lut->evaluate([1,1]), 1, 'OR 11');

    # XOR
    $lut->configure([0, 1, 1, 0]);
    is($lut->evaluate([0,1]), 1, 'XOR 01');
    is($lut->evaluate([1,1]), 0, 'XOR 11');

    # MSB-first: [1,0,1] = address 5
    my $lut3 = CodingAdventures::FPGA::LUT->new(3);
    my @tt = (0) x 8;
    $tt[5] = 1;
    $lut3->configure(\@tt);
    is($lut3->evaluate([1,0,1]), 1, 'MSB-first address');
    is($lut3->evaluate([1,0,0]), 0, 'other address');

    # Error cases
    ok(dies { $lut->configure([0,1]) }, 'wrong tt size dies');
    ok(dies { $lut->evaluate([0,1,0]) }, 'wrong input count dies');
};

# ============================================================
# Slice
# ============================================================

subtest 'Slice' => sub {
    my $sl = CodingAdventures::FPGA::Slice->new(lut_inputs => 2);
    $sl->configure({lut_a => [0,0,0,1], lut_b => [0,1,1,0]});

    my ($a, $b, $c) = $sl->evaluate([1,1], [0,1], 0, 0);
    is($a, 1, 'AND(1,1)=1');
    is($b, 1, 'XOR(0,1)=1');
    is($c, 0, 'no carry');

    # Flip-flop
    my $sl_ff = CodingAdventures::FPGA::Slice->new(lut_inputs => 2, use_ff_a => 1);
    $sl_ff->configure({lut_a => [0,0,0,1]});

    # Clock=0: FF doesn't capture, stays 0
    ($a) = $sl_ff->evaluate([1,1], [0,0], 0, 0);
    is($a, 0, 'FF holds initial value');

    # Clock=1: FF captures
    ($a) = $sl_ff->evaluate([1,1], [0,0], 1, 0);
    is($a, 1, 'FF captures on rising edge');

    # Change inputs, no clock: FF holds
    ($a) = $sl_ff->evaluate([0,0], [0,0], 0, 0);
    is($a, 1, 'FF holds value');

    # Carry chain
    my $sl_c = CodingAdventures::FPGA::Slice->new(lut_inputs => 2, carry_enable => 1);
    $sl_c->configure({lut_a => [1,1,1,1], lut_b => [0,0,0,0]});
    # LUT_A=1 always: out_a = 1 XOR 1 = 0; carry_mid = 1 AND 1 = 1
    # LUT_B=0 always: out_b = 0 XOR 1 = 1; carry_out = 0 AND 1 = 0
    my ($ra, $rb, $rc) = $sl_c->evaluate([0,0], [0,0], 0, 1);
    is($ra, 0, 'carry XOR: 1 XOR 1 = 0');
    is($rb, 1, 'carry XOR: 0 XOR 1 = 1');
    is($rc, 0, 'carry out: 0 AND 1 = 0');
};

# ============================================================
# CLB
# ============================================================

subtest 'CLB' => sub {
    my $clb = CodingAdventures::FPGA::CLB->new(0, 0, lut_inputs => 2);
    is($clb->{row}, 0, 'row=0');
    is($clb->{col}, 0, 'col=0');

    $clb->configure({
        slice_0 => {lut_a => [0,0,0,1], lut_b => [0,1,1,0]},
        slice_1 => {lut_a => [1,1,1,0], lut_b => [1,0,0,1]},
    });

    my $inputs = {s0_a=>[1,1], s0_b=>[0,1], s1_a=>[1,1], s1_b=>[0,1]};
    my ($outputs, $carry) = $clb->evaluate($inputs, 0, 0);
    is(scalar @$outputs, 4, '4 outputs');
    is($outputs->[0], 1, 'slice0 AND(1,1)');
    is($outputs->[1], 1, 'slice0 XOR(0,1)');
};

# ============================================================
# SwitchMatrix
# ============================================================

subtest 'SwitchMatrix' => sub {
    my $sm = CodingAdventures::FPGA::SwitchMatrix->new(4, 4);
    is($sm->{input_names}[0],  'in_0',  'in_0');
    is($sm->{output_names}[3], 'out_3', 'out_3');

    $sm->configure({'out_0' => 'in_2', 'out_1' => 'in_0'});
    my $out = $sm->route({in_0=>1, in_1=>0, in_2=>1, in_3=>0});
    is($out->{'out_0'}, 1, 'out_0 from in_2=1');
    is($out->{'out_1'}, 1, 'out_1 from in_0=1');
    ok(!defined $out->{'out_2'}, 'out_2 unconnected');

    # Fan-out
    my $sm2 = CodingAdventures::FPGA::SwitchMatrix->new(2, 3);
    $sm2->configure({'out_0'=>'in_1', 'out_1'=>'in_1', 'out_2'=>'in_0'});
    $out = $sm2->route({in_0=>0, in_1=>1});
    is($out->{'out_0'}, 1, 'fan-out out_0');
    is($out->{'out_1'}, 1, 'fan-out out_1');
    is($out->{'out_2'}, 0, 'in_0=0');

    # Invalid port
    ok(dies { $sm->configure({'out_99' => 'in_0'}) }, 'bad output port');
    ok(dies { $sm->configure({'out_0' => 'in_99'}) }, 'bad input port');
};

# ============================================================
# IOBlock
# ============================================================

subtest 'IOBlock' => sub {
    my $io = CodingAdventures::FPGA::IOBlock->new('p', 'input');
    $io->set_pin(1);
    is($io->read_fabric(), 1, 'input read_fabric');

    $io = CodingAdventures::FPGA::IOBlock->new('p', 'output');
    is($io->{output_enable}, 1, 'output OE=1');
    $io->set_fabric(0);
    is($io->read_pin(), 0, 'output read_pin');

    $io = CodingAdventures::FPGA::IOBlock->new('p', 'bidirectional');
    $io->set_pin(1);
    $io->set_output_enable(0);
    is($io->read_fabric(), 1, 'bidi OE=0: read_fabric=pin');

    $io->set_fabric(0);
    $io->set_output_enable(1);
    is($io->read_pin(), 0, 'bidi OE=1: read_pin=fabric');

    ok(dies { CodingAdventures::FPGA::IOBlock->new('p', 'bad') }, 'bad direction');
    ok(dies {
        my $out = CodingAdventures::FPGA::IOBlock->new('p', 'output');
        $out->set_pin(1);
    }, 'set_pin on output');
};

# ============================================================
# Bitstream
# ============================================================

subtest 'Bitstream' => sub {
    my $bs = CodingAdventures::FPGA::Bitstream->from_map({
        clbs => {
            '0,0' => { slice_0 => { lut_a => [0,0,0,1] } },
        },
        routing => {
            '1,2' => { 'out_0' => 'in_1' },
        },
    });

    my $cfg = $bs->clb_config('0,0');
    ok(defined $cfg, 'CLB config retrieved');

    my $rc = $bs->routing_config('1,2');
    is($rc->{'out_0'}, 'in_1', 'routing config');

    ok(!defined $bs->clb_config('99,99'), 'unknown key returns undef');
    ok(!defined $bs->io_config('top_0'), 'missing io returns undef');
};

# ============================================================
# Fabric
# ============================================================

subtest 'Fabric' => sub {
    my $fab = CodingAdventures::FPGA::Fabric->new(2, 2);
    is($fab->{rows}, 2, 'rows=2');
    is($fab->{cols}, 2, 'cols=2');
    ok(defined $fab->{io_blocks}{'top_0'}, 'top_0 exists');
    ok(defined $fab->{io_blocks}{'bottom_1'}, 'bottom_1 exists');
    ok(defined $fab->{io_blocks}{'left_0'}, 'left_0 exists');
    ok(defined $fab->{io_blocks}{'right_1'}, 'right_1 exists');
    is($fab->{io_blocks}{'top_0'}{direction}, 'input', 'top is input');
    is($fab->{io_blocks}{'bottom_0'}{direction}, 'output', 'bottom is output');

    # set_input and evaluate
    $fab->set_input('top_0', 1);
    $fab->evaluate(0);  # No crash

    my $s = $fab->summary();
    like($s, qr/FPGA/i, 'summary mentions FPGA');

    # load_bitstream
    my $fab2 = CodingAdventures::FPGA::Fabric->new(1, 1, lut_inputs => 2);
    my $bs = CodingAdventures::FPGA::Bitstream->from_map({
        clbs => {
            '0,0' => { slice_0 => { lut_a => [0,0,0,1] } },
        },
    });
    $fab2->load_bitstream($bs);
    is(scalar @{$fab2->{clbs}[0][0]{slice_0}{lut_a}{truth_table}}, 4, 'bitstream loaded LUT');
};

# ============================================================
# End-to-End: LUT-based AND gate
# ============================================================

subtest 'end-to-end AND gate' => sub {
    my $lut = CodingAdventures::FPGA::LUT->new(2);
    $lut->configure([0,0,0,1]);
    is($lut->evaluate([0,0]), 0, 'AND 00');
    is($lut->evaluate([0,1]), 0, 'AND 01');
    is($lut->evaluate([1,0]), 0, 'AND 10');
    is($lut->evaluate([1,1]), 1, 'AND 11');
};

# ============================================================
# End-to-End: Carry-chain adder
# ============================================================

subtest 'end-to-end carry chain' => sub {
    my $sl = CodingAdventures::FPGA::Slice->new(lut_inputs => 2, carry_enable => 1);
    $sl->configure({
        lut_a => [0,1,1,0],  # XOR(A,B)
        lut_b => [0,0,0,0],  # unused
    });

    # A=1, B=0, Cin=0: XOR=1; sum=1 XOR 0=1; carry_mid=1 AND 0=0
    my ($s, undef, $c) = $sl->evaluate([1,0], [0,0], 0, 0);
    is($s, 1, 'sum A=1,B=0,Cin=0');
    is($c, 0, 'carry A=1,B=0,Cin=0');

    # A=1, B=0, Cin=1: XOR=1; sum=1 XOR 1=0; carry_mid=1 AND 1=1
    ($s, undef, $c) = $sl->evaluate([1,0], [0,0], 0, 1);
    is($s, 0, 'sum A=1,B=0,Cin=1');
    is($c, 1, 'carry A=1,B=0,Cin=1');
};

done_testing;
