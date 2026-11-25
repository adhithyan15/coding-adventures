package CodingAdventures::TestBootstrap::Fuzz::Limits;

# CWE-400 ceilings of the fuzz engine (VER06q section 6).
#
# A fuzz campaign is attacker-reachable work (a CI job fed a hostile configuration
# or fixture), so every count and length that drives a loop or an allocation has a
# hard ceiling that is checked BEFORE the loop or the allocation.

use strict;
use warnings;

use constant {
    MAX_ITERATIONS          => 1_000_000,
    MAX_INPUT_BYTES         => 1 << 20,
    MAX_CRASHES             => 1000,
    MAX_INITIAL_CORPUS      => 10_000,
    MAX_STEP_BUDGET         => 100_000_000,
    MAX_MEM_BUDGET          => 1 << 30,
    MAX_MINIMIZE_BUDGET     => 100_000,
    DEFAULT_MINIMIZE_BUDGET => 2000,
    DEFAULT_STEP_BUDGET     => 200_000,
    DEFAULT_MEM_BUDGET      => 64 << 20,
    # Bytes of corpus seeds plus crash inputs one campaign keeps.
    MAX_RETAINED_BYTES => 256 << 20,
    # Bytes one mutate_sequence call may copy (initial copy + one buffer per step).
    MAX_SEQUENCE_WORK => 1 << 28,
    # The corpus runner's AGGREGATE of the same quantity, over all of its cases.
    MAX_CORPUS_SEQUENCE_WORK => 1 << 24,
    # Caller levels inspected per crash, however deep the recursion.
    MAX_WALK => 4096,
    # Frames kept in a signature (the innermost survive).
    MAX_FRAMES      => 64,
    MAX_BASENAME    => 64,
    MAX_KIND_LEN    => 64,
    MAX_TARGET_LEN  => 128,
    # VER06 section 4.1: 8,192 bytes = 65,536 bits = 65,536 edges.
    COVERAGE_MAP_SIZE => 65536,
    # Leaked executions after which a campaign is aborted (VER06q 5.4).
    MAX_LEAKED => 4,
    # Wall bound of one execution, in milliseconds.
    DEFAULT_TIMEOUT_MS => 5000,
    MAX_TIMEOUT_MS     => 60_000,
    # After a budget breach the alarm is re-armed this often (seconds), so a target
    # that swallowed the sentinel is interrupted again.
    REARM_SECONDS => 0.05,
    # Tamper checks on the timer tolerate this much clock noise (seconds).
    ALARM_MARGIN => 0.25,
};

1;
