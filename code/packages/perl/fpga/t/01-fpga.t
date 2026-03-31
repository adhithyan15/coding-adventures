use strict;
use warnings;
use Test2::V0;

use CodingAdventures::FPGA::LUT;
use CodingAdventures::FPGA::Slice;
use CodingAdventures::FPGA::CLB;
use CodingAdventures::FPGA::SwitchMatrix;
use CodingAdventures::FPGA::IOBlock;
use CodingAdventures::FPGA::Fabric;
use CodingAdventures::FPGA::Bitstream;

# ===========================================================================
# LUT
# ===========================================================================

subtest 'LUT: construction' => sub {
    my $lut = CodingAdventures::FPGA::LUT->new(4);
    is $lut->{num_inputs}, 4, 'num_inputs = 4';
    is scalar(@{ $lut->{truth_table} }), 16, 'truth table size = 16';
    is $lut->{truth_table}[0], 0, 'default all zeros';
};

subtest 'LUT: configure AND gate' => sub {
    my $lut = CodingAdventures::FPGA::LUT->new(2);
    $lut->configure([0, 0, 0, 1]);
    is_deeply $lut->{truth_table}, [0, 0, 0, 1], 'AND truth table';
};

subtest 'LUT: evaluate AND' => sub {
    my $lut = CodingAdventures::FPGA::LUT->new(2);
    $lut->configure([0, 0, 0, 1]);
    is $lut->evaluate([0, 0]), 0, 'AND(0,0)=0';
    is $lut->evaluate([0, 1]), 0, 'AND(0,1)=0';
    is $lut->evaluate([1, 0]), 0, 'AND(1,0)=0';
    is $lut->evaluate([1, 1]), 1, 'AND(1,1)=1';
};

subtest 'LUT: evaluate OR' => sub {
    my $lut = CodingAdventures::FPGA::LUT->new(2);
    $lut->configure([0, 1, 1, 1]);
    is $lut->evaluate([0, 0]), 0, 'OR(0,0)=0';
    is $lut->evaluate([0, 1]), 1, 'OR(0,1)=1';
    is $lut->evaluate([1, 0]), 1, 'OR(1,0)=1';
    is $lut->evaluate([1, 1]), 1, 'OR(1,1)=1';
};

subtest 'LUT: evaluate XOR' => sub {
    my $lut = CodingAdventures::FPGA::LUT->new(2);
    $lut->configure([0, 1, 1, 0]);
    is $lut->evaluate([0, 0]), 0, 'XOR(0,0)=0';
    is $lut->evaluate([0, 1]), 1, 'XOR(0,1)=1';
    is $lut->evaluate([1, 0]), 1, 'XOR(1,0)=1';
    is $lut->evaluate([1, 1]), 0, 'XOR(1,1)=0';
};

subtest 'LUT: evaluate 4-input NAND' => sub {
    my $lut = CodingAdventures::FPGA::LUT->new(4);
    my @tt = map { $_ == 15 ? 0 : 1 } 0..15;
    $lut->configure(\@tt);
    is $lut->evaluate([0, 0, 0, 0]), 1, 'NAND4(0000)=1';
    is $lut->evaluate([1, 1, 1, 0]), 1, 'NAND4(1110)=1';
    is $lut->evaluate([1, 1, 1, 1]), 0, 'NAND4(1111)=0';
};

subtest 'LUT: rejects wrong truth table size' => sub {
    my $lut = CodingAdventures::FPGA::LUT->new(2);
    ok dies { $lut->configure([0, 0, 0]) }, 'wrong size dies';
};

subtest 'LUT: rejects wrong input count' => sub {
    my $lut = CodingAdventures::FPGA::LUT->new(2);
    $lut->configure([0, 0, 0, 1]);
    ok dies { $lut->evaluate([0, 0, 0]) }, 'wrong input count dies';
};

# ===========================================================================
# Slice
# ===========================================================================

subtest 'Slice: construction defaults' => sub {
    my $s = CodingAdventures::FPGA::Slice->new();
    is $s->{lut_a}{num_inputs}, 4, 'default lut_inputs=4';
    ok !$s->{use_ff_a}, 'use_ff_a default false';
    ok !$s->{carry_enable}, 'carry_enable default false';
};

subtest 'Slice: combinational (no FF, no carry)' => sub {
    my $s = CodingAdventures::FPGA::Slice->new(lut_inputs => 2);
    $s->configure({ lut_a => [0,0,0,1], lut_b => [0,1,1,0] });
    my ($out_a, $out_b, $carry) = $s->evaluate([1,1], [0,1], 0, 0);
    is $out_a, 1, 'AND(1,1)=1';
    is $out_b, 1, 'XOR(0,1)=1';
    is $carry, 0, 'no carry';
};

subtest 'Slice: carry chain' => sub {
    # lut_a = OR(a,b): with a=1,b=0 → 1; carry_in=1
    # out_a = 1 XOR 1 = 0; carry_mid = 1 AND 1 = 1
    # lut_b = AND(a,b): with a=0,b=0 → 0; carry_mid=1
    # out_b = 0 XOR 1 = 1; carry_out = 0 AND 1 = 0
    my $s = CodingAdventures::FPGA::Slice->new(lut_inputs => 2, carry_enable => 1);
    $s->configure({ lut_a => [0,1,1,1], lut_b => [0,0,0,1] });
    my ($out_a, $out_b, $carry_out) = $s->evaluate([1,0], [0,0], 0, 1);
    is $out_a,     0, 'sum_a = 1 XOR 1 = 0';
    is $out_b,     1, 'sum_b = 0 XOR 1 = 1';
    is $carry_out, 0, 'carry_out = 0 AND 1 = 0';
};

subtest 'Slice: flip-flop captures on rising edge' => sub {
    my $s = CodingAdventures::FPGA::Slice->new(lut_inputs => 2, use_ff_a => 1);
    $s->configure({ lut_a => [0,0,0,1] });  # AND

    # clock=0: LUT=AND(1,1)=1, but FF not capturing
    my ($out_a) = $s->evaluate([1,1], [0,0], 0, 0);
    is $out_a, 0, 'clock=0: FF holds 0';

    # clock=1: FF captures 1
    ($out_a) = $s->evaluate([1,1], [0,0], 1, 0);
    is $out_a, 1, 'clock=1: FF captured 1';
};

subtest 'Slice: flip-flop holds value between clocks' => sub {
    my $s = CodingAdventures::FPGA::Slice->new(lut_inputs => 2, use_ff_a => 1);
    $s->configure({ lut_a => [0,0,0,1] });

    $s->evaluate([1,1], [0,0], 1, 0);  # capture 1
    my ($out_a) = $s->evaluate([0,0], [0,0], 0, 0);  # clock low, AND=0
    is $out_a, 1, 'FF holds 1 when clock=0';
};

# ===========================================================================
# CLB
# ===========================================================================

subtest 'CLB: construction' => sub {
    my $clb = CodingAdventures::FPGA::CLB->new(2, 3);
    is $clb->{row}, 2, 'row=2';
    is $clb->{col}, 3, 'col=3';
};

subtest 'CLB: evaluate returns 4 outputs' => sub {
    my $clb = CodingAdventures::FPGA::CLB->new(0, 0, lut_inputs => 2);
    $clb->configure({
        slice_0 => { lut_a => [0,0,0,1], lut_b => [0,1,1,0] },
        slice_1 => { lut_a => [1,1,1,0], lut_b => [1,0,0,1] },
    });
    my $inputs = { s0_a => [1,1], s0_b => [0,1], s1_a => [1,1], s1_b => [0,0] };
    my ($outputs, $carry) = $clb->evaluate($inputs, 0, 0);
    is scalar(@$outputs), 4, '4 outputs';
    is $outputs->[0], 1, 'AND(1,1)=1';
    is $outputs->[1], 1, 'XOR(0,1)=1';
    is $outputs->[2], 0, 'NAND(1,1)=0';
    is $outputs->[3], 1, 'XNOR(0,0)=1';
};

# ===========================================================================
# SwitchMatrix
# ===========================================================================

subtest 'SwitchMatrix: construction' => sub {
    my $sm = CodingAdventures::FPGA::SwitchMatrix->new(4, 4);
    is $sm->{num_inputs},  4, 'num_inputs=4';
    is $sm->{num_outputs}, 4, 'num_outputs=4';
    is $sm->{input_names}[0],  'in_0',  'first input name';
    is $sm->{output_names}[0], 'out_0', 'first output name';
};

subtest 'SwitchMatrix: configure and route' => sub {
    my $sm = CodingAdventures::FPGA::SwitchMatrix->new(4, 4);
    $sm->configure({ out_0 => 'in_2' });
    my $result = $sm->route({ in_0 => 0, in_1 => 0, in_2 => 1, in_3 => 0 });
    is $result->{out_0}, 1, 'out_0 = in_2 = 1';
};

subtest 'SwitchMatrix: unconnected outputs are undef' => sub {
    my $sm = CodingAdventures::FPGA::SwitchMatrix->new(4, 4);
    $sm->configure({ out_0 => 'in_0' });
    my $result = $sm->route({ in_0 => 1, in_1 => 0, in_2 => 0, in_3 => 0 });
    ok !defined($result->{out_1}), 'out_1 undef (unconnected)';
    ok !defined($result->{out_2}), 'out_2 undef';
};

subtest 'SwitchMatrix: fan-out' => sub {
    my $sm = CodingAdventures::FPGA::SwitchMatrix->new(4, 4);
    $sm->configure({ out_0 => 'in_2', out_1 => 'in_2', out_2 => 'in_2' });
    my $result = $sm->route({ in_0 => 0, in_1 => 0, in_2 => 1, in_3 => 0 });
    is $result->{out_0}, 1, 'fan-out out_0';
    is $result->{out_1}, 1, 'fan-out out_1';
    is $result->{out_2}, 1, 'fan-out out_2';
};

subtest 'SwitchMatrix: rejects invalid ports' => sub {
    my $sm = CodingAdventures::FPGA::SwitchMatrix->new(4, 4);
    ok dies { $sm->configure({ out_99 => 'in_0' }) }, 'invalid output dies';
    ok dies { $sm->configure({ out_0  => 'in_99' }) }, 'invalid input dies';
};

# ===========================================================================
# IOBlock
# ===========================================================================

subtest 'IOBlock: input block' => sub {
    my $io = CodingAdventures::FPGA::IOBlock->new('pin_0', 'input');
    is $io->{direction}, 'input', 'direction=input';
    is $io->{output_enable}, 0, 'OE=0';

    $io->set_pin(1);
    is $io->read_fabric(), 1, 'read_fabric=1';
    is $io->read_pin(),    1, 'read_pin=1';
};

subtest 'IOBlock: output block' => sub {
    my $io = CodingAdventures::FPGA::IOBlock->new('pin_0', 'output');
    is $io->{output_enable}, 1, 'OE=1 for output block';

    $io->set_fabric(1);
    is $io->read_pin(),    1, 'read_pin=1';
    is $io->read_fabric(), 1, 'read_fabric=1';
};

subtest 'IOBlock: bidirectional OE=0 (input mode)' => sub {
    my $io = CodingAdventures::FPGA::IOBlock->new('pin_0', 'bidirectional');
    $io->set_pin(1);
    is $io->read_fabric(), 1, 'OE=0: read_fabric returns pin_value';
};

subtest 'IOBlock: bidirectional OE=1 (output mode)' => sub {
    my $io = CodingAdventures::FPGA::IOBlock->new('pin_0', 'bidirectional');
    $io->set_fabric(0);
    $io->set_output_enable(1);
    $io->set_fabric(1);
    is $io->read_pin(), 1, 'OE=1: read_pin returns fabric_value';
};

subtest 'IOBlock: rejects invalid direction' => sub {
    ok dies { CodingAdventures::FPGA::IOBlock->new('pin_0', 'invalid') }, 'invalid direction dies';
};

subtest 'IOBlock: input block rejects set_fabric' => sub {
    my $io = CodingAdventures::FPGA::IOBlock->new('pin_0', 'input');
    ok dies { $io->set_fabric(1) }, 'set_fabric on input dies';
};

subtest 'IOBlock: output block rejects set_pin' => sub {
    my $io = CodingAdventures::FPGA::IOBlock->new('pin_0', 'output');
    ok dies { $io->set_pin(1) }, 'set_pin on output dies';
};

subtest 'IOBlock: set_output_enable on non-bidirectional dies' => sub {
    my $io_in  = CodingAdventures::FPGA::IOBlock->new('pin_0', 'input');
    my $io_out = CodingAdventures::FPGA::IOBlock->new('pin_0', 'output');
    ok dies { $io_in->set_output_enable(1) },  'input dies';
    ok dies { $io_out->set_output_enable(0) }, 'output dies';
};

# ===========================================================================
# Bitstream
# ===========================================================================

subtest 'Bitstream: empty map' => sub {
    my $bs = CodingAdventures::FPGA::Bitstream->from_map({});
    ok !defined($bs->clb_config('0_0')),     'missing clb = undef';
    ok !defined($bs->routing_config('0_0')), 'missing routing = undef';
    ok !defined($bs->io_config('pin_0')),    'missing io = undef';
};

subtest 'Bitstream: parses CLB config' => sub {
    my $bs = CodingAdventures::FPGA::Bitstream->from_map({
        clbs => { '0_0' => { slice_0 => { lut_a => [0,0,0,1] } } },
    });
    my $cfg = $bs->clb_config('0_0');
    ok defined($cfg), 'config defined';
    is_deeply $cfg->{slice_0}{lut_a}, [0,0,0,1], 'lut_a correct';
};

subtest 'Bitstream: parses routing config' => sub {
    my $bs = CodingAdventures::FPGA::Bitstream->from_map({
        routing => { '0_0' => { out_0 => 'in_1' } },
    });
    my $cfg = $bs->routing_config('0_0');
    is $cfg->{out_0}, 'in_1', 'routing connection';
};

subtest 'Bitstream: parses IO config' => sub {
    my $bs = CodingAdventures::FPGA::Bitstream->from_map({
        io => { top_0 => { direction => 'input' } },
    });
    is $bs->io_config('top_0')->{direction}, 'input', 'IO direction';
};

# ===========================================================================
# Fabric
# ===========================================================================

subtest 'Fabric: construction 2x2' => sub {
    my $f = CodingAdventures::FPGA::Fabric->new(2, 2);
    is $f->{rows}, 2, 'rows=2';
    is $f->{cols}, 2, 'cols=2';
    is scalar(keys %{ $f->{clbs} }), 4, '4 CLBs';
    is scalar(keys %{ $f->{switch_matrices} }), 4, '4 switch matrices';
    # top 2, bottom 2, left 2, right 2 = 8 I/O blocks
    is scalar(keys %{ $f->{io_blocks} }), 8, '8 I/O blocks';
};

subtest 'Fabric: perimeter IO directions' => sub {
    my $f = CodingAdventures::FPGA::Fabric->new(2, 2);
    is $f->{io_blocks}{top_0}{direction},    'input',  'top=input';
    is $f->{io_blocks}{bottom_0}{direction}, 'output', 'bottom=output';
    is $f->{io_blocks}{left_0}{direction},   'input',  'left=input';
    is $f->{io_blocks}{right_0}{direction},  'output', 'right=output';
};

subtest 'Fabric: summary' => sub {
    my $f = CodingAdventures::FPGA::Fabric->new(2, 2);
    my $s = $f->summary();
    is $s->{rows},               2,  'rows';
    is $s->{cols},               2,  'cols';
    is $s->{clb_count},          4,  'clb_count';
    is $s->{lut_count},          16, 'lut_count';
    is $s->{ff_count},           16, 'ff_count';
    is $s->{switch_matrix_count}, 4, 'switch_matrix_count';
    is $s->{io_block_count},     8,  'io_block_count';
};

subtest 'Fabric: set_input' => sub {
    my $f = CodingAdventures::FPGA::Fabric->new(2, 2);
    $f->set_input('top_0', 1);
    is $f->{io_blocks}{top_0}->read_fabric(), 1, 'top_0 fabric = 1';
};

subtest 'Fabric: set_input unknown pin dies' => sub {
    my $f = CodingAdventures::FPGA::Fabric->new(2, 2);
    ok dies { $f->set_input('nonexistent', 1) }, 'unknown pin dies';
};

subtest 'Fabric: load_bitstream configures CLB' => sub {
    my $f = CodingAdventures::FPGA::Fabric->new(1, 1, lut_inputs => 2);
    my $bs = CodingAdventures::FPGA::Bitstream->from_map({
        clbs    => { '0_0' => { slice_0 => { lut_a => [0,0,0,1] } } },
        routing => {},
        io      => {},
    });
    $f->load_bitstream($bs);
    is_deeply $f->{clbs}{'0_0'}{slice_0}{lut_a}{truth_table}, [0,0,0,1], 'lut_a configured';
};

subtest 'Fabric: load_bitstream configures routing' => sub {
    my $f = CodingAdventures::FPGA::Fabric->new(1, 1, switch_size => 4);
    my $bs = CodingAdventures::FPGA::Bitstream->from_map({
        clbs    => {},
        routing => { '0_0' => { out_0 => 'in_1' } },
        io      => {},
    });
    $f->load_bitstream($bs);
    is $f->{switch_matrices}{'0_0'}{connections}{out_0}, 'in_1', 'routing configured';
};

subtest 'Fabric: load_bitstream reconfigures IO' => sub {
    my $f = CodingAdventures::FPGA::Fabric->new(1, 1);
    my $bs = CodingAdventures::FPGA::Bitstream->from_map({
        clbs    => {},
        routing => {},
        io      => { top_0 => { direction => 'bidirectional' } },
    });
    $f->load_bitstream($bs);
    is $f->{io_blocks}{top_0}{direction}, 'bidirectional', 'IO reconfigured';
};

subtest 'Fabric: evaluate runs without error' => sub {
    my $f = CodingAdventures::FPGA::Fabric->new(2, 2);
    $f->evaluate(0);
    $f->evaluate(1);
    ok 1, 'evaluate ran without dying';
};

# ===========================================================================
# End-to-End: AND gate programmed onto FPGA
# ===========================================================================

subtest 'E2E: AND gate on FPGA' => sub {
    my $f = CodingAdventures::FPGA::Fabric->new(1, 1, lut_inputs => 2);
    my $bs = CodingAdventures::FPGA::Bitstream->from_map({
        clbs    => { '0_0' => { slice_0 => { lut_a => [0,0,0,1] } } },
        routing => {},
        io      => {},
    });
    $f->load_bitstream($bs);

    my $lut = $f->{clbs}{'0_0'}{slice_0}{lut_a};
    is $lut->evaluate([0,0]), 0, 'AND(0,0)=0';
    is $lut->evaluate([0,1]), 0, 'AND(0,1)=0';
    is $lut->evaluate([1,0]), 0, 'AND(1,0)=0';
    is $lut->evaluate([1,1]), 1, 'AND(1,1)=1';
};

done_testing;
