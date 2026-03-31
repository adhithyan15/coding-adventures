use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::HazardDetection; 1 },
    'CodingAdventures::HazardDetection loads' );

my $Slot     = 'CodingAdventures::HazardDetection::PipelineSlot';
my $Result   = 'CodingAdventures::HazardDetection::HazardResult';
my $DataDet  = 'CodingAdventures::HazardDetection::DataHazardDetector';
my $CtrlDet  = 'CodingAdventures::HazardDetection::ControlHazardDetector';
my $StrucDet = 'CodingAdventures::HazardDetection::StructuralHazardDetector';

# ============================================================================
# PipelineSlot tests
# ============================================================================

subtest 'PipelineSlot: new() with defaults' => sub {
    my $s = $Slot->new();
    is($s->{valid},    1,  'valid defaults to true');
    is($s->{pc},       0,  'pc defaults to 0');
    is($s->{dest_reg}, -1, 'dest_reg defaults to -1');
    is($s->{mem_read}, 0,  'mem_read defaults to false');
    is($s->{is_branch},0,  'is_branch defaults to false');
};

subtest 'PipelineSlot: empty() creates invalid slot' => sub {
    my $s = $Slot->empty();
    is($s->{valid}, 0, 'empty slot is invalid');
};

subtest 'PipelineSlot: custom fields' => sub {
    my $s = $Slot->new(
        valid       => 1,
        pc          => 0x100,
        dest_reg    => 3,
        dest_value  => 42,
        source_regs => [1, 2],
        mem_read    => 1,
    );
    is($s->{pc},         0x100,  'pc=0x100');
    is($s->{dest_reg},   3,      'dest_reg=3');
    is($s->{dest_value}, 42,     'dest_value=42');
    is_deeply($s->{source_regs}, [1, 2], 'source_regs=[1,2]');
    is($s->{mem_read},   1,      'mem_read=1');
};

# ============================================================================
# HazardResult tests
# ============================================================================

subtest 'HazardResult: defaults to action=none' => sub {
    my $r = $Result->new();
    is($r->{action},          'none', 'action=none');
    is($r->{stall_cycles},    0,      'stall_cycles=0');
    is($r->{forwarded_value}, 0,      'forwarded_value=0');
    is($r->{forwarded_from},  '',     'forwarded_from=""');
};

# ============================================================================
# DataHazardDetector tests
# ============================================================================

subtest 'DataHazardDetector: no hazard for empty ID stage' => sub {
    my $det = $DataDet->new();
    my $r   = $det->detect($Slot->empty(), $Slot->empty(), $Slot->empty());
    is($r->{action}, 'none', 'action=none for empty ID');
};

subtest 'DataHazardDetector: no hazard for no source registers' => sub {
    my $det = $DataDet->new();
    my $id  = $Slot->new(valid => 1, source_regs => []);
    my $ex  = $Slot->new(valid => 1, dest_reg => 1, dest_value => 100);
    my $r   = $det->detect($id, $ex, $Slot->empty());
    is($r->{action}, 'none', 'action=none for no source regs');
};

subtest 'DataHazardDetector: no hazard when no dependency' => sub {
    my $det = $DataDet->new();
    my $id  = $Slot->new(valid => 1, source_regs => [1, 2]);
    my $ex  = $Slot->new(valid => 1, dest_reg => 5, dest_value => 0);  # writes R5
    my $mem = $Slot->new(valid => 1, dest_reg => 6, dest_value => 0);  # writes R6
    my $r   = $det->detect($id, $ex, $mem);
    is($r->{action}, 'none', 'no hazard when different registers');
};

subtest 'DataHazardDetector: forward_ex for EX-to-EX RAW hazard' => sub {
    my $det = $DataDet->new();
    my $id  = $Slot->new(valid => 1, source_regs => [1, 5]);
    my $ex  = $Slot->new(valid => 1, dest_reg => 1, dest_value => 42, mem_read => 0);
    my $r   = $det->detect($id, $ex, $Slot->empty());
    is($r->{action},          'forward_ex', 'action=forward_ex');
    is($r->{forwarded_value}, 42,           'forwarded_value=42');
    is($r->{forwarded_from},  'EX',         'forwarded_from=EX');
};

subtest 'DataHazardDetector: forward_mem for MEM-to-EX RAW hazard' => sub {
    my $det = $DataDet->new();
    my $id  = $Slot->new(valid => 1, source_regs => [3]);
    my $ex  = $Slot->new(valid => 1, dest_reg => 7, dest_value => 0);  # writes R7, not R3
    my $mem = $Slot->new(valid => 1, dest_reg => 3, dest_value => 77);
    my $r   = $det->detect($id, $ex, $mem);
    is($r->{action},          'forward_mem', 'action=forward_mem');
    is($r->{forwarded_value}, 77,            'forwarded_value=77');
    is($r->{forwarded_from},  'MEM',         'forwarded_from=MEM');
};

subtest 'DataHazardDetector: stall for load-use hazard' => sub {
    my $det = $DataDet->new();
    my $id  = $Slot->new(valid => 1, source_regs => [1, 4]);
    my $ex  = $Slot->new(valid => 1, dest_reg => 1, dest_value => 0, mem_read => 1);  # LOAD
    my $r   = $det->detect($id, $ex, $Slot->empty());
    is($r->{action},       'stall', 'action=stall for load-use');
    is($r->{stall_cycles}, 1,       'stall_cycles=1');
};

subtest 'DataHazardDetector: stall beats forward_mem (priority)' => sub {
    my $det = $DataDet->new();
    my $id  = $Slot->new(valid => 1, source_regs => [1, 2]);
    my $ex  = $Slot->new(valid => 1, dest_reg => 1, dest_value => 0, mem_read => 1);  # load-use on R1
    my $mem = $Slot->new(valid => 1, dest_reg => 2, dest_value => 55);                # forward on R2
    my $r   = $det->detect($id, $ex, $mem);
    is($r->{action}, 'stall', 'stall beats forward_mem');
};

subtest 'DataHazardDetector: forward_ex beats forward_mem' => sub {
    my $det = $DataDet->new();
    my $id  = $Slot->new(valid => 1, source_regs => [3]);
    my $ex  = $Slot->new(valid => 1, dest_reg => 3, dest_value => 10, mem_read => 0);
    my $mem = $Slot->new(valid => 1, dest_reg => 3, dest_value => 20);
    my $r   = $det->detect($id, $ex, $mem);
    is($r->{action},          'forward_ex', 'forward_ex beats forward_mem');
    is($r->{forwarded_value}, 10,           'uses EX value');
};

subtest 'DataHazardDetector: reason is non-empty' => sub {
    my $det = $DataDet->new();
    my $id  = $Slot->new(valid => 1, source_regs => [1]);
    my $ex  = $Slot->new(valid => 1, dest_reg => 1, dest_value => 5, mem_read => 0);
    my $r   = $det->detect($id, $ex, $Slot->empty());
    ok(length($r->{reason}) > 0, 'reason is non-empty');
};

# ============================================================================
# ControlHazardDetector tests
# ============================================================================

subtest 'ControlHazardDetector: none for empty EX stage' => sub {
    my $det = $CtrlDet->new();
    my $r   = $det->detect($Slot->empty());
    is($r->{action}, 'none', 'action=none for empty EX');
};

subtest 'ControlHazardDetector: none for non-branch instruction' => sub {
    my $det = $CtrlDet->new();
    my $ex  = $Slot->new(valid => 1, pc => 0x20, is_branch => 0);
    my $r   = $det->detect($ex);
    is($r->{action}, 'none', 'action=none for non-branch');
};

subtest 'ControlHazardDetector: none for correctly predicted not-taken' => sub {
    my $det = $CtrlDet->new();
    my $ex  = $Slot->new(
        valid                  => 1, pc => 0x20,
        is_branch              => 1,
        branch_taken           => 0,
        branch_predicted_taken => 0,
        branch_target          => 0x80,
    );
    my $r = $det->detect($ex);
    is($r->{action}, 'none', 'action=none for correct prediction');
};

subtest 'ControlHazardDetector: none for correctly predicted taken' => sub {
    my $det = $CtrlDet->new();
    my $ex  = $Slot->new(
        valid                  => 1, pc => 0x20,
        is_branch              => 1,
        branch_taken           => 1,
        branch_predicted_taken => 1,
        branch_target          => 0x80,
    );
    my $r = $det->detect($ex);
    is($r->{action}, 'none', 'action=none for correct taken prediction');
};

subtest 'ControlHazardDetector: flush when predicted not-taken but taken' => sub {
    my $det = $CtrlDet->new();
    my $ex  = $Slot->new(
        valid                  => 1, pc => 0x20,
        is_branch              => 1,
        branch_taken           => 1,
        branch_predicted_taken => 0,
        branch_target          => 0x80,
    );
    my $r = $det->detect($ex);
    is($r->{action},       'flush', 'action=flush on misprediction');
    is($r->{flush_target}, 0x80,   'flush_target = branch_target');
};

subtest 'ControlHazardDetector: flush when predicted taken but not taken' => sub {
    my $det = $CtrlDet->new();
    my $ex  = $Slot->new(
        valid                  => 1, pc => 0x20,
        is_branch              => 1,
        branch_taken           => 0,
        branch_predicted_taken => 1,
        branch_target          => 0x80,
    );
    my $r = $det->detect($ex);
    is($r->{action},       'flush', 'action=flush');
    is($r->{flush_target}, 0x24,   'flush_target = PC+4');
};

subtest 'ControlHazardDetector: reason mentions misprediction' => sub {
    my $det = $CtrlDet->new();
    my $ex  = $Slot->new(
        valid                  => 1, pc => 0x20,
        is_branch              => 1,
        branch_taken           => 1,
        branch_predicted_taken => 0,
        branch_target          => 0x80,
    );
    my $r = $det->detect($ex);
    like($r->{reason}, qr/mispredicted/, 'reason mentions misprediction');
};

# ============================================================================
# StructuralHazardDetector tests
# ============================================================================

subtest 'StructuralHazardDetector: none with split cache (default)' => sub {
    my $det = $StrucDet->new();
    my $mem = $Slot->new(valid => 1, dest_reg => 1, mem_read => 1);
    my $r   = $det->detect($mem, $Slot->empty(), 1);
    is($r->{action}, 'none', 'no hazard with split cache');
};

subtest 'StructuralHazardDetector: stall for unified cache conflict' => sub {
    my $det = $StrucDet->new();
    my $mem = $Slot->new(valid => 1, dest_reg => 1, mem_read => 1);
    my $r   = $det->detect($mem, $Slot->empty(), 0);  # unified cache
    is($r->{action}, 'stall', 'stall for unified cache conflict');
};

subtest 'StructuralHazardDetector: none when different write registers' => sub {
    my $det = $StrucDet->new();
    my $mem = $Slot->new(valid => 1, dest_reg => 1);
    my $wb  = $Slot->new(valid => 1, dest_reg => 2);
    my $r   = $det->detect($mem, $wb, 1);
    is($r->{action}, 'none', 'no hazard for different dest regs');
};

subtest 'StructuralHazardDetector: stall when both write same register' => sub {
    my $det = $StrucDet->new();
    my $mem = $Slot->new(valid => 1, dest_reg => 3);
    my $wb  = $Slot->new(valid => 1, dest_reg => 3);
    my $r   = $det->detect($mem, $wb, 1);
    is($r->{action}, 'stall', 'stall for write-port conflict');
};

subtest 'StructuralHazardDetector: none for empty stages' => sub {
    my $det = $StrucDet->new();
    my $r   = $det->detect($Slot->empty(), $Slot->empty(), 1);
    is($r->{action}, 'none', 'no hazard for empty stages');
};

done_testing;
