package CodingAdventures::HazardDetection;
# =============================================================================
# CodingAdventures::HazardDetection — pipeline hazard detectors
# =============================================================================
#
# Packages defined here:
#   CodingAdventures::HazardDetection::PipelineSlot             — one stage view
#   CodingAdventures::HazardDetection::HazardResult             — detector output
#   CodingAdventures::HazardDetection::DataHazardDetector       — RAW hazards
#   CodingAdventures::HazardDetection::ControlHazardDetector    — branch misprediction
#   CodingAdventures::HazardDetection::StructuralHazardDetector — resource conflicts

use strict;
use warnings;

our $VERSION = '0.01';

1;

# =============================================================================

package CodingAdventures::HazardDetection::PipelineSlot;
# -----------------------------------------------------------------------------
# ISA-agnostic view of one pipeline stage, used by all detectors.
# -----------------------------------------------------------------------------

use strict;
use warnings;

sub new {
    my ($class, %opts) = @_;
    return bless {
        valid                  => $opts{valid}  // 0,
        pc                     => $opts{pc}     // 0,
        source_regs            => $opts{source_regs}            // [],
        dest_reg               => $opts{dest_reg}               // -1,
        dest_value             => $opts{dest_value}             // 0,
        mem_read               => $opts{mem_read}               // 0,
        mem_write              => $opts{mem_write}              // 0,
        is_branch              => $opts{is_branch}              // 0,
        branch_taken           => $opts{branch_taken}           // 0,
        branch_predicted_taken => $opts{branch_predicted_taken} // 0,
        uses_alu               => $opts{uses_alu}               // 0,
        uses_fp                => $opts{uses_fp}                // 0,
    }, $class;
}

sub empty {
    my ($class) = @_;
    return $class->new(valid => 0);
}

1;

# =============================================================================

package CodingAdventures::HazardDetection::HazardResult;
# -----------------------------------------------------------------------------
# Output of a hazard detector.
# action: 'none' | 'stall' | 'flush' | 'forward_ex' | 'forward_mem'
# -----------------------------------------------------------------------------

use strict;
use warnings;

sub new {
    my ($class, %opts) = @_;
    return bless {
        action          => $opts{action}          // 'none',
        stall_cycles    => $opts{stall_cycles}    // 0,
        flush_count     => $opts{flush_count}     // 0,
        forwarded_value => $opts{forwarded_value} // 0,
        forwarded_from  => $opts{forwarded_from}  // '',
        reason          => $opts{reason}          // '',
    }, $class;
}

my %PRIORITY = (none => 0, forward_mem => 1, forward_ex => 2, stall => 3, flush => 4);

sub _priority { $PRIORITY{ $_[0]->{action} } // 0 }

sub _pick_higher_priority {
    my ($a, $b) = @_;
    return $b->_priority > $a->_priority ? $b : $a;
}

1;

# =============================================================================

package CodingAdventures::HazardDetection::DataHazardDetector;
# -----------------------------------------------------------------------------
# Detects RAW (Read After Write) hazards.
# Priority: stall > forward_ex > forward_mem > none
# -----------------------------------------------------------------------------

use strict;
use warnings;

my $Slot   = 'CodingAdventures::HazardDetection::PipelineSlot';
my $Result = 'CodingAdventures::HazardDetection::HazardResult';

sub new { bless {}, $_[0] }

sub detect {
    my ($self, $id_slot, $ex_slot, $mem_slot) = @_;
    unless ($id_slot->{valid}) {
        return $Result->new(reason => 'ID stage is empty (bubble)');
    }
    my @srcs = @{ $id_slot->{source_regs} };
    unless (@srcs) {
        return $Result->new(reason => 'instruction has no source registers');
    }

    my $worst = $Result->new(reason => 'no data dependencies detected');
    for my $src (@srcs) {
        my $r = $self->_check_single_register($src, $ex_slot, $mem_slot);
        $worst = $worst->_pick_higher_priority($r);
    }
    return $worst;
}

sub _check_single_register {
    my ($self, $src, $ex_slot, $mem_slot) = @_;

    # Load-use hazard: EX is a load, ID needs the result
    if ($ex_slot->{valid} && $ex_slot->{dest_reg} == $src && $ex_slot->{mem_read}) {
        return $Result->new(
            action       => 'stall',
            stall_cycles => 1,
            reason       => sprintf(
                'load-use hazard: R%d is being loaded by instruction at PC=0x%04X — must stall 1 cycle',
                $src, $ex_slot->{pc}),
        );
    }

    # EX forwarding
    if ($ex_slot->{valid} && $ex_slot->{dest_reg} == $src) {
        return $Result->new(
            action          => 'forward_ex',
            forwarded_value => $ex_slot->{dest_value},
            forwarded_from  => 'EX',
            reason          => sprintf(
                'RAW hazard on R%d: forwarding value %d from EX (PC=0x%04X)',
                $src, $ex_slot->{dest_value}, $ex_slot->{pc}),
        );
    }

    # MEM forwarding
    if ($mem_slot->{valid} && $mem_slot->{dest_reg} == $src) {
        return $Result->new(
            action          => 'forward_mem',
            forwarded_value => $mem_slot->{dest_value},
            forwarded_from  => 'MEM',
            reason          => sprintf(
                'RAW hazard on R%d: forwarding value %d from MEM (PC=0x%04X)',
                $src, $mem_slot->{dest_value}, $mem_slot->{pc}),
        );
    }

    return $Result->new(reason => sprintf 'R%d has no pending writes', $src);
}

1;

# =============================================================================

package CodingAdventures::HazardDetection::ControlHazardDetector;
# -----------------------------------------------------------------------------
# Detects branch mispredictions and emits flush signals.
# -----------------------------------------------------------------------------

use strict;
use warnings;

my $Result2 = 'CodingAdventures::HazardDetection::HazardResult';

sub new { bless {}, $_[0] }

sub detect {
    my ($self, $ex_slot) = @_;
    unless ($ex_slot->{valid}) {
        return $Result2->new(reason => 'EX stage is empty (bubble)');
    }
    unless ($ex_slot->{is_branch}) {
        return $Result2->new(reason => 'EX stage instruction is not a branch');
    }
    if ($ex_slot->{branch_predicted_taken} == $ex_slot->{branch_taken}) {
        my $dir = $ex_slot->{branch_taken} ? 'taken' : 'not taken';
        return $Result2->new(
            reason => sprintf('branch at PC=0x%04X correctly predicted %s',
                $ex_slot->{pc}, $dir),
        );
    }
    # Misprediction
    my $dir = $ex_slot->{branch_taken}
        ? 'predicted not-taken, actually taken'
        : 'predicted taken, actually not-taken';
    return $Result2->new(
        action      => 'flush',
        flush_count => 2,
        reason      => sprintf(
            'branch misprediction at PC=0x%04X: %s — flushing IF and ID stages',
            $ex_slot->{pc}, $dir),
    );
}

1;

# =============================================================================

package CodingAdventures::HazardDetection::StructuralHazardDetector;
# -----------------------------------------------------------------------------
# Detects resource conflicts: ALU/FP unit, shared memory port.
# -----------------------------------------------------------------------------

use strict;
use warnings;

my $Result3 = 'CodingAdventures::HazardDetection::HazardResult';

sub new {
    my ($class, %opts) = @_;
    return bless {
        num_alus     => $opts{num_alus}     // 1,
        num_fp_units => $opts{num_fp_units} // 1,
        split_caches => $opts{split_caches} // 1,
    }, $class;
}

sub detect {
    my ($self, $id_slot, $ex_slot, %opts) = @_;
    my $exec_result = $self->_check_execution_unit($id_slot, $ex_slot);
    return $exec_result if $exec_result->{action} ne 'none';

    if (defined $opts{if_stage} && defined $opts{mem_stage}) {
        return $self->_check_memory_port($opts{if_stage}, $opts{mem_stage});
    }

    return $Result3->new(reason => 'no structural hazards — all resources available');
}

sub _check_execution_unit {
    my ($self, $id_slot, $ex_slot) = @_;
    unless ($id_slot->{valid} && $ex_slot->{valid}) {
        return $Result3->new(reason => 'one or both stages are empty (bubble)');
    }
    if ($id_slot->{uses_alu} && $ex_slot->{uses_alu} && $self->{num_alus} < 2) {
        return $Result3->new(
            action       => 'stall',
            stall_cycles => 1,
            reason       => sprintf(
                'structural hazard: both ID (PC=0x%04X) and EX (PC=0x%04X) need the ALU, but only %d ALU available',
                $id_slot->{pc}, $ex_slot->{pc}, $self->{num_alus}),
        );
    }
    if ($id_slot->{uses_fp} && $ex_slot->{uses_fp} && $self->{num_fp_units} < 2) {
        return $Result3->new(
            action       => 'stall',
            stall_cycles => 1,
            reason       => sprintf(
                'structural hazard: both ID and EX need the FP unit, but only %d FP unit available',
                $self->{num_fp_units}),
        );
    }
    return $Result3->new(reason => 'no execution unit conflict');
}

sub _check_memory_port {
    my ($self, $if_slot, $mem_slot) = @_;
    if ($self->{split_caches}) {
        return $Result3->new(reason => 'split caches — no memory port conflict');
    }
    if ($if_slot->{valid} && $mem_slot->{valid}
            && ($mem_slot->{mem_read} || $mem_slot->{mem_write})) {
        my $type = $mem_slot->{mem_read} ? 'load' : 'store';
        return $Result3->new(
            action       => 'stall',
            stall_cycles => 1,
            reason       => sprintf(
                'structural hazard: IF (PC=0x%04X) and MEM (%s at PC=0x%04X) both need the shared memory bus',
                $if_slot->{pc}, $type, $mem_slot->{pc}),
        );
    }
    return $Result3->new(reason => 'no memory port conflict');
}

1;
