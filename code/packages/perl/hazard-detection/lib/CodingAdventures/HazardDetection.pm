package CodingAdventures::HazardDetection;

# ============================================================================
# CodingAdventures::HazardDetection — Pipeline Hazard Detection
# ============================================================================
#
# This module provides pure detection logic for CPU pipeline hazards.
# It has no pipeline state of its own — you pass in snapshots of the
# current pipeline stage contents and receive a HazardResult.
#
# THE THREE HAZARD TYPES:
#
# 1. DATA HAZARDS (RAW — Read After Write):
#
#      ADD R1, R2, R3   ; writes R1, completes WB at cycle 5
#      SUB R4, R1, R5   ; reads R1 at ID (cycle 3) — gets STALE value!
#
#    Solution A — Forwarding (bypassing):
#      Route ADD's result from EX/MEM directly to SUB's ALU input.
#      No stall needed if the value is computed (not loaded from memory).
#
#    Solution B — Stall:
#      If ADD is a LOAD, the value isn't ready until MEM (cycle 4).
#      SUB needs it in EX (cycle 4) — one cycle too early for forwarding.
#      Insert a bubble, delay SUB by 1 cycle.
#
# 2. CONTROL HAZARDS (branch misprediction):
#
#      BEQ R1, R2, target   ; resolved in EX (cycle 3)
#      ADD R3, R4, R5        ; fetched at cycle 2 — may be WRONG!
#      SUB R6, R7, R8        ; fetched at cycle 3 — may be WRONG!
#
#    If the branch is TAKEN, ADD and SUB must be flushed. The pipeline
#    redirects to `target` and starts fetching from there.
#
# 3. STRUCTURAL HAZARDS (resource conflicts):
#
#      Two instructions need the same hardware resource simultaneously.
#      Examples: unified cache (IF + MEM), limited write ports.
#
# MODULES PROVIDED:
#
#   PipelineSlot           — describes one stage's current instruction
#   HazardResult           — the detector's decision (action + value)
#   DataHazardDetector     — RAW hazard detection and forwarding
#   ControlHazardDetector  — branch misprediction detection
#   StructuralHazardDetector — resource conflict detection

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# PipelineSlot — snapshot of one pipeline stage
# ============================================================================

package CodingAdventures::HazardDetection::PipelineSlot;

sub new {
    my ($class, %opts) = @_;
    return bless {
        valid                  => exists $opts{valid}  ? $opts{valid}  : 1,
        pc                     => $opts{pc}                     // 0,
        dest_reg               => $opts{dest_reg}               // -1,
        dest_value             => $opts{dest_value}             // 0,
        source_regs            => $opts{source_regs}            // [],
        mem_read               => $opts{mem_read}               // 0,
        is_branch              => $opts{is_branch}              // 0,
        branch_taken           => $opts{branch_taken}           // 0,
        branch_predicted_taken => $opts{branch_predicted_taken} // 0,
        branch_target          => $opts{branch_target}          // 0,
    }, $class;
}

sub empty {
    my ($class) = @_;
    return $class->new(valid => 0);
}

# ============================================================================
# HazardResult — the detection result
# ============================================================================
#
# Possible actions:
#   "none"        — no hazard, proceed normally
#   "stall"       — freeze earlier stages, insert bubble
#   "flush"       — discard speculative instructions, redirect PC
#   "forward_ex"  — forward value from EX stage to ALU input
#   "forward_mem" — forward value from MEM stage to ALU input
#
# Priority: flush (4) > stall (3) > forward_ex (2) > forward_mem (1) > none (0)

package CodingAdventures::HazardDetection::HazardResult;

my %PRIORITY = (
    none        => 0,
    forward_mem => 1,
    forward_ex  => 2,
    stall       => 3,
    flush       => 4,
);

sub new {
    my ($class, %opts) = @_;
    return bless {
        action          => $opts{action}          // 'none',
        stall_cycles    => $opts{stall_cycles}    // 0,
        forwarded_value => $opts{forwarded_value} // 0,
        forwarded_from  => $opts{forwarded_from}  // '',
        flush_target    => $opts{flush_target}    // 0,
        reason          => $opts{reason}          // 'no hazard detected',
    }, $class;
}

sub _pick_higher_priority {
    my ($a, $b) = @_;
    my $pa = $PRIORITY{ $a->{action} } // 0;
    my $pb = $PRIORITY{ $b->{action} } // 0;
    return $pb > $pa ? $b : $a;
}

# ============================================================================
# DataHazardDetector
# ============================================================================
#
# Checks every source register that the ID-stage instruction reads against
# every destination register in EX and MEM. Returns the highest-priority
# hazard found.
#
# Algorithm:
#   For each source_reg in id_slot.source_regs:
#     1. EX has a LOAD writing source_reg → stall (load-use)
#     2. EX has non-LOAD writing source_reg → forward from EX
#     3. MEM writing source_reg → forward from MEM
#     4. Otherwise → no hazard for this register
#   Return highest-priority result.

package CodingAdventures::HazardDetection::DataHazardDetector;

use CodingAdventures::HazardDetection::HazardResult;

sub new { return bless {}, $_[0] }

sub detect {
    my ($self, $id_slot, $ex_slot, $mem_slot) = @_;
    my $HR = 'CodingAdventures::HazardDetection::HazardResult';

    unless ($id_slot->{valid}) {
        return $HR->new(reason => 'ID stage is empty (bubble)');
    }
    unless (@{ $id_slot->{source_regs} }) {
        return $HR->new(reason => 'instruction has no source registers');
    }

    my $worst = $HR->new(reason => 'no data dependencies detected');
    for my $src_reg (@{ $id_slot->{source_regs} }) {
        my $r = $self->_check_single_register($src_reg, $ex_slot, $mem_slot);
        $worst = CodingAdventures::HazardDetection::HazardResult::_pick_higher_priority($worst, $r);
    }
    return $worst;
}

sub _check_single_register {
    my ($self, $src_reg, $ex_slot, $mem_slot) = @_;
    my $HR = 'CodingAdventures::HazardDetection::HazardResult';

    # Load-use hazard: EX is a LOAD writing src_reg — must stall
    if ($ex_slot->{valid} && $ex_slot->{dest_reg} == $src_reg && $ex_slot->{mem_read}) {
        return $HR->new(
            action       => 'stall',
            stall_cycles => 1,
            reason       => sprintf(
                'load-use hazard: R%d is being loaded by instruction at PC=0x%04X — must stall 1 cycle',
                $src_reg, $ex_slot->{pc}
            ),
        );
    }

    # EX RAW hazard: forward from EX
    if ($ex_slot->{valid} && $ex_slot->{dest_reg} == $src_reg) {
        return $HR->new(
            action          => 'forward_ex',
            forwarded_value => $ex_slot->{dest_value},
            forwarded_from  => 'EX',
            reason          => sprintf(
                'RAW hazard on R%d: forwarding value %d from EX stage (PC=0x%04X)',
                $src_reg, $ex_slot->{dest_value}, $ex_slot->{pc}
            ),
        );
    }

    # MEM RAW hazard: forward from MEM
    if ($mem_slot->{valid} && $mem_slot->{dest_reg} == $src_reg) {
        return $HR->new(
            action          => 'forward_mem',
            forwarded_value => $mem_slot->{dest_value},
            forwarded_from  => 'MEM',
            reason          => sprintf(
                'RAW hazard on R%d: forwarding value %d from MEM stage (PC=0x%04X)',
                $src_reg, $mem_slot->{dest_value}, $mem_slot->{pc}
            ),
        );
    }

    return $HR->new(
        reason => sprintf('R%d has no pending writes in EX or MEM', $src_reg),
    );
}

# ============================================================================
# ControlHazardDetector
# ============================================================================

package CodingAdventures::HazardDetection::ControlHazardDetector;

sub new { return bless {}, $_[0] }

sub detect {
    my ($self, $ex_slot) = @_;
    my $HR = 'CodingAdventures::HazardDetection::HazardResult';

    unless ($ex_slot->{valid}) {
        return $HR->new(reason => 'EX stage is empty (bubble)');
    }
    unless ($ex_slot->{is_branch}) {
        return $HR->new(reason => 'EX stage instruction is not a branch');
    }
    if ($ex_slot->{branch_predicted_taken} == $ex_slot->{branch_taken}) {
        my $dir = $ex_slot->{branch_taken} ? 'taken' : 'not taken';
        return $HR->new(
            reason => sprintf('branch correctly predicted as %s at PC=0x%04X', $dir, $ex_slot->{pc})
        );
    }

    # Misprediction: flush and redirect
    my $target = $ex_slot->{branch_taken}
        ? $ex_slot->{branch_target}
        : $ex_slot->{pc} + 4;

    return $HR->new(
        action       => 'flush',
        flush_target => $target,
        reason       => sprintf(
            'branch mispredicted at PC=0x%04X: predicted %s but actually %s — redirecting to 0x%04X',
            $ex_slot->{pc},
            $ex_slot->{branch_predicted_taken} ? 'taken' : 'not taken',
            $ex_slot->{branch_taken}           ? 'taken' : 'not taken',
            $target
        ),
    );
}

# ============================================================================
# StructuralHazardDetector
# ============================================================================

package CodingAdventures::HazardDetection::StructuralHazardDetector;

sub new { return bless {}, $_[0] }

sub detect {
    my ($self, $mem_slot, $wb_slot, $has_split_cache) = @_;
    $has_split_cache //= 1;
    my $HR = 'CodingAdventures::HazardDetection::HazardResult';

    # Unified cache conflict: IF and MEM both need memory
    if (!$has_split_cache && $mem_slot->{valid} && $mem_slot->{mem_read}) {
        return $HR->new(
            action       => 'stall',
            stall_cycles => 1,
            reason       => 'structural hazard: unified cache — IF and MEM both need memory',
        );
    }

    # Register file write-port conflict
    if ($mem_slot->{valid} && $wb_slot->{valid} &&
        $mem_slot->{dest_reg} >= 0 && $wb_slot->{dest_reg} >= 0 &&
        $mem_slot->{dest_reg} == $wb_slot->{dest_reg}) {
        return $HR->new(
            action       => 'stall',
            stall_cycles => 1,
            reason       => sprintf(
                'structural hazard: both MEM and WB want to write R%d — need two write ports',
                $mem_slot->{dest_reg}
            ),
        );
    }

    return $HR->new(reason => 'no structural hazard detected');
}

# ============================================================================
# Top-level package
# ============================================================================

package CodingAdventures::HazardDetection;

1;
__END__

=head1 NAME

CodingAdventures::HazardDetection - Pipeline hazard detection and forwarding

=head1 SYNOPSIS

    use CodingAdventures::HazardDetection;

    my $Slot = 'CodingAdventures::HazardDetection::PipelineSlot';
    my $det  = CodingAdventures::HazardDetection::DataHazardDetector->new();

    my $ex_slot = $Slot->new(valid => 1, dest_reg => 1, dest_value => 42);
    my $id_slot = $Slot->new(valid => 1, source_regs => [1]);
    my $result  = $det->detect($id_slot, $ex_slot, $Slot->empty());

    print $result->{action};           # forward_ex
    print $result->{forwarded_value};  # 42

=cut
