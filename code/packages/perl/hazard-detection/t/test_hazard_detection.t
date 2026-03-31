use strict;
use warnings;
use Test2::V0;
use lib '../lib';
use CodingAdventures::HazardDetection;

my $Slot       = 'CodingAdventures::HazardDetection::PipelineSlot';
my $DataDet    = 'CodingAdventures::HazardDetection::DataHazardDetector';
my $CtrlDet    = 'CodingAdventures::HazardDetection::ControlHazardDetector';
my $StructDet  = 'CodingAdventures::HazardDetection::StructuralHazardDetector';

sub empty_slot { $Slot->empty() }
sub slot { $Slot->new(@_) }

# ---------------------------------------------------------------------------
# DataHazardDetector
# ---------------------------------------------------------------------------

subtest 'DataHazard: no hazard — ID empty' => sub {
    my $det = $DataDet->new();
    my $r = $det->detect(empty_slot(), empty_slot(), empty_slot());
    is $r->{action}, 'none', 'action none';
};

subtest 'DataHazard: no hazard — ID has no source regs' => sub {
    my $det = $DataDet->new();
    my $id = slot(valid => 1, source_regs => []);
    my $r = $det->detect($id, empty_slot(), empty_slot());
    is $r->{action}, 'none', 'action none';
};

subtest 'DataHazard: no dependency' => sub {
    my $det = $DataDet->new();
    my $id  = slot(valid => 1, source_regs => [1, 2]);
    my $ex  = slot(valid => 1, dest_reg => 5);
    my $mem = slot(valid => 1, dest_reg => 6);
    my $r = $det->detect($id, $ex, $mem);
    is $r->{action}, 'none', 'action none';
};

subtest 'DataHazard: load-use stall' => sub {
    my $det = $DataDet->new();
    my $id  = slot(valid => 1, source_regs => [1], pc => 0x8);
    my $ex  = slot(valid => 1, dest_reg => 1, mem_read => 1, pc => 0x4);
    my $r = $det->detect($id, $ex, empty_slot());
    is $r->{action},       'stall', 'action stall';
    is $r->{stall_cycles}, 1,       'stall_cycles 1';
};

subtest 'DataHazard: load-use beats EX forward' => sub {
    my $det = $DataDet->new();
    my $id  = slot(valid => 1, source_regs => [1]);
    my $ex  = slot(valid => 1, dest_reg => 1, mem_read => 1);
    my $mem = slot(valid => 1, dest_reg => 1);
    my $r = $det->detect($id, $ex, $mem);
    is $r->{action}, 'stall', 'stall beats forward_mem';
};

subtest 'DataHazard: EX forwarding' => sub {
    my $det = $DataDet->new();
    my $id  = slot(valid => 1, source_regs => [3]);
    my $ex  = slot(valid => 1, dest_reg => 3, dest_value => 42);
    my $r = $det->detect($id, $ex, empty_slot());
    is $r->{action},          'forward_ex', 'action forward_ex';
    is $r->{forwarded_value}, 42,           'value 42';
    is $r->{forwarded_from},  'EX',         'from EX';
};

subtest 'DataHazard: EX beats MEM for same register' => sub {
    my $det = $DataDet->new();
    my $id  = slot(valid => 1, source_regs => [3]);
    my $ex  = slot(valid => 1, dest_reg => 3, dest_value => 10);
    my $mem = slot(valid => 1, dest_reg => 3, dest_value => 5);
    my $r = $det->detect($id, $ex, $mem);
    is $r->{action},          'forward_ex', 'EX beats MEM';
    is $r->{forwarded_value}, 10,           'value from EX';
};

subtest 'DataHazard: MEM forwarding' => sub {
    my $det = $DataDet->new();
    my $id  = slot(valid => 1, source_regs => [7]);
    my $ex  = slot(valid => 1, dest_reg => 9);
    my $mem = slot(valid => 1, dest_reg => 7, dest_value => 99);
    my $r = $det->detect($id, $ex, $mem);
    is $r->{action},          'forward_mem', 'action forward_mem';
    is $r->{forwarded_value}, 99,            'value 99';
};

subtest 'DataHazard: stall wins over forward_mem across source regs' => sub {
    my $det = $DataDet->new();
    # rs1=2 → MEM forward; rs2=4 → load-use stall
    my $id  = slot(valid => 1, source_regs => [2, 4]);
    my $ex  = slot(valid => 1, dest_reg => 4, mem_read => 1);
    my $mem = slot(valid => 1, dest_reg => 2, dest_value => 7);
    my $r = $det->detect($id, $ex, $mem);
    is $r->{action}, 'stall', 'stall beats forward_mem';
};

# ---------------------------------------------------------------------------
# ControlHazardDetector
# ---------------------------------------------------------------------------

subtest 'ControlHazard: empty EX stage' => sub {
    my $det = $CtrlDet->new();
    is $det->detect(empty_slot())->{action}, 'none', 'none';
};

subtest 'ControlHazard: not a branch' => sub {
    my $det = $CtrlDet->new();
    my $ex = slot(valid => 1, is_branch => 0);
    is $det->detect($ex)->{action}, 'none', 'none';
};

subtest 'ControlHazard: correctly predicted taken' => sub {
    my $det = $CtrlDet->new();
    my $ex = slot(valid => 1, is_branch => 1,
                  branch_taken => 1, branch_predicted_taken => 1);
    is $det->detect($ex)->{action}, 'none', 'none';
};

subtest 'ControlHazard: correctly predicted not-taken' => sub {
    my $det = $CtrlDet->new();
    my $ex = slot(valid => 1, is_branch => 1,
                  branch_taken => 0, branch_predicted_taken => 0);
    is $det->detect($ex)->{action}, 'none', 'none';
};

subtest 'ControlHazard: misprediction not-taken→taken' => sub {
    my $det = $CtrlDet->new();
    my $ex = slot(valid => 1, is_branch => 1,
                  branch_taken => 1, branch_predicted_taken => 0, pc => 0x10);
    my $r = $det->detect($ex);
    is $r->{action},      'flush', 'flush';
    is $r->{flush_count}, 2,       '2 stages flushed';
    like $r->{reason}, qr/misprediction/, 'reason mentions misprediction';
};

subtest 'ControlHazard: misprediction taken→not-taken' => sub {
    my $det = $CtrlDet->new();
    my $ex = slot(valid => 1, is_branch => 1,
                  branch_taken => 0, branch_predicted_taken => 1);
    my $r = $det->detect($ex);
    is $r->{action}, 'flush', 'flush';
};

# ---------------------------------------------------------------------------
# StructuralHazardDetector
# ---------------------------------------------------------------------------

subtest 'StructuralHazard: both empty' => sub {
    my $det = $StructDet->new();
    my $r = $det->detect(empty_slot(), empty_slot());
    is $r->{action}, 'none', 'none';
};

subtest 'StructuralHazard: different units' => sub {
    my $det = $StructDet->new();
    my $id = slot(valid => 1, uses_alu => 1, uses_fp => 0);
    my $ex = slot(valid => 1, uses_alu => 0, uses_fp => 1);
    is $det->detect($id, $ex)->{action}, 'none', 'none';
};

subtest 'StructuralHazard: ALU conflict (1 ALU)' => sub {
    my $det = $StructDet->new(num_alus => 1);
    my $id = slot(valid => 1, uses_alu => 1);
    my $ex = slot(valid => 1, uses_alu => 1);
    my $r = $det->detect($id, $ex);
    is $r->{action}, 'stall', 'stall';
};

subtest 'StructuralHazard: no ALU conflict (2 ALUs)' => sub {
    my $det = $StructDet->new(num_alus => 2);
    my $id = slot(valid => 1, uses_alu => 1);
    my $ex = slot(valid => 1, uses_alu => 1);
    is $det->detect($id, $ex)->{action}, 'none', 'none';
};

subtest 'StructuralHazard: split caches no memory conflict' => sub {
    my $det = $StructDet->new(split_caches => 1);
    my $if  = slot(valid => 1);
    my $mem = slot(valid => 1, mem_read => 1);
    my $r = $det->detect(empty_slot(), empty_slot(), if_stage => $if, mem_stage => $mem);
    is $r->{action}, 'none', 'none with split caches';
};

subtest 'StructuralHazard: unified cache memory conflict' => sub {
    my $det = $StructDet->new(split_caches => 0);
    my $if  = slot(valid => 1, pc => 0x100);
    my $mem = slot(valid => 1, pc => 0x50, mem_read => 1);
    my $r = $det->detect(empty_slot(), empty_slot(), if_stage => $if, mem_stage => $mem);
    is $r->{action}, 'stall', 'stall with unified cache';
};

done_testing;
