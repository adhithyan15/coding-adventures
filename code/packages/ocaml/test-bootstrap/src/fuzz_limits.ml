(* The CWE-400 ceilings of the fuzz engine (VER06r sections 2, 7 and 8).

   A fuzz campaign is attacker-reachable work (a CI job fed a hostile
   configuration or fixture), so every count and length that drives a loop or an
   allocation has a hard ceiling, and each one is checked BEFORE the allocation
   it bounds.  OCaml's native [int] has 63 bits and wraps silently, so no sum of
   two of these is ever formed without first checking it cannot pass a cap. *)

(* Most mutation iterations one campaign may run, and most steps one
   [mutate_sequence] call may take. *)
let max_iterations = 1_000_000

(* Largest input (a seed, a candidate, a sequence input): 1 MiB. *)
let max_input_bytes = 1_048_576

(* Distinct crash signatures kept; more are counted in [crashes_dropped]. *)
let max_crashes = 1000
let max_initial_corpus = 10_000
let max_step_budget = 100_000_000
let max_mem_budget = 1_073_741_824

(* Below this the engine's own allocation (the worker's startup, the locks)
   would make every execution an oom, so such budgets are refused. *)
let min_mem_budget = 65_536
let max_minimize_budget = 100_000
let default_minimize_budget = 2000
let default_step_budget = 200_000

(* 256 MiB of ALLOCATION per execution (VER06r section 6): the Gc counters
   measure total bytes allocated, not residency. *)
let default_mem_budget = 268_435_456
let max_timeout_ms = 60_000
let default_timeout_ms = 5000

(* CPU-time waits for a worker that was asked to stop (VER06r sections 4, 7). *)
let kill_wait_ms = 2000
let leaked_kill_wait_ms = 500
let max_leaked_workers = 4

(* Leaked workers alive in the WHOLE PROCESS, across campaigns.  OCaml allows about
   127 domains in all, so per-campaign caps alone would let a few dozen hostile
   campaigns exhaust them; at this cap the executor refuses to start another
   execution (VER06r 7).  A leaked worker that finally returns is subtracted. *)
let max_global_leaked_workers = 16

(* Corpus seeds plus crash inputs kept by one campaign: 256 MiB. *)
let max_retained_bytes = 268_435_456

(* Bytes one [mutate_sequence] call may copy (2^28) and the aggregate the corpus
   runner allows over all its cases (2^24). *)
let max_sequence_work = 268_435_456
let corpus_sequence_work = 16_777_216
let max_frames = 64
let max_kind_len = 64

(* The longest TAP target token, in BYTES (VER06 9.3). *)
let max_target_len = 128
let coverage_map_size = 65_536
