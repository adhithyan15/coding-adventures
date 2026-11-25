--- CWE-400 ceilings of the fuzz engine (VER06p section 6).
--
-- A fuzz campaign is attacker-reachable work (a CI job fed a hostile configuration
-- or fixture), so every count and length that drives a loop or an allocation has a
-- hard ceiling that is checked BEFORE the loop or the allocation.

local Limits = {
  MAX_ITERATIONS = 1000000,
  MAX_INPUT_BYTES = 1 << 20,
  MAX_CRASHES = 1000,
  MAX_INITIAL_CORPUS = 10000,
  MAX_STEP_BUDGET = 100000000,
  MAX_MEM_BUDGET = 1 << 30,
  MAX_MINIMIZE_BUDGET = 100000,
  DEFAULT_MINIMIZE_BUDGET = 2000,
  DEFAULT_STEP_BUDGET = 200000,
  DEFAULT_MEM_BUDGET = 64 << 20,
  -- Bytes of corpus seeds plus crash inputs one campaign keeps.
  MAX_RETAINED_BYTES = 256 << 20,
  -- Bytes one mutate_sequence call may copy (initial copy + one buffer per step).
  MAX_SEQUENCE_WORK = 1 << 28,
  -- The corpus runner's AGGREGATE of the same quantity, over all of its cases.
  MAX_CORPUS_SEQUENCE_WORK = 1 << 24,
  -- Stack levels inspected per crash, however deep the recursion.
  MAX_WALK = 4096,
  -- Bytes of chunk-name text one frame walk may read.
  MAX_WALK_TEXT = 1 << 20,
  -- Frames kept in a signature (the innermost survive).
  MAX_FRAMES = 64,
  MAX_BASENAME = 64,
  MAX_KIND_LEN = 64,
  MAX_TARGET_LEN = 128,
  -- VER06 section 4.1: 8,192 bytes = 65,536 bits = 65,536 edges.
  COVERAGE_MAP_SIZE = 65536,
  -- Leaked executions after which a campaign is aborted (VER06p 5.4).
  MAX_LEAKED = 4,
  -- Wall bound of one execution, in milliseconds of os.clock().
  DEFAULT_TIMEOUT_MS = 5000,
  MAX_TIMEOUT_MS = 60000,
  -- VM instructions of one execution (the count hook adds HOOK_COUNT per call).
  DEFAULT_INSTRUCTION_BUDGET = 100000000,
  MAX_INSTRUCTION_BUDGET = 10000000000,
  HOOK_COUNT = 1000,
  -- The poll never judges a memory budget below this (a bare coroutine is already KiBs).
  POLL_FLOOR = 1 << 20,
}

return Limits
