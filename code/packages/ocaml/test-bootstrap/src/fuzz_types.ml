(* Options, outcomes, reports and verdicts of the fuzz engine (VER06r 5, 9).

   Every type that can hold a seed or a crash input has a [show_*] function that
   prints counts only: an input is PRIVATE (VER06 6.3) and must never reach a log
   through a printer. *)

type options = {
  max_iterations : int;
  seed : int64;
  max_input_len : int;
  base_energy : int;
  minimize_budget : int;
  step_budget : int;
  mem_budget : int;
  timeout_ms : int;
  ci : Types.ci_mode;
}

let default_options =
  {
    max_iterations = 1000;
    seed = 42L;
    max_input_len = 4096;
    base_energy = 10;
    minimize_budget = Fuzz_limits.default_minimize_budget;
    step_budget = Fuzz_limits.default_step_budget;
    mem_budget = Fuzz_limits.default_mem_budget;
    timeout_ms = Fuzz_limits.default_timeout_ms;
    ci = Types.Ci_unset;
  }

(* Out-of-range options raise [Argument_error] instead of producing a verdict. *)
let validate_options o =
  let range name v lo hi =
    if v < lo || v > hi then
      raise
        (Error.Argument_error
           (Printf.sprintf "%s must be in [%d, %d]" name lo hi))
  in
  range "max_iterations" o.max_iterations 0 Fuzz_limits.max_iterations;
  range "max_input_len" o.max_input_len 0 Fuzz_limits.max_input_bytes;
  range "base_energy" o.base_energy 0 Fuzz_limits.max_iterations;
  range "minimize_budget" o.minimize_budget 0 Fuzz_limits.max_minimize_budget;
  range "step_budget" o.step_budget 0 Fuzz_limits.max_step_budget;
  range "mem_budget" o.mem_budget Fuzz_limits.min_mem_budget
    Fuzz_limits.max_mem_budget;
  range "timeout_ms" o.timeout_ms 1 Fuzz_limits.max_timeout_ms

let show_options o =
  Printf.sprintf "FuzzOptions<max_iterations=%d>" o.max_iterations

(* A crash is its kind and frames (frames are always empty in this lane:
   OCaml exceptions carry no portable stack). *)
type outcome = Pass | Crash of string * string list | Timeout | Oom

type crash_record = {
  signature : string;
  kind : string;
  frames : string list;
  minimal_input : string;  (** PRIVATE *)
  hits : int;
  minimized : bool;
  input_dropped : bool;
}

let show_crash r =
  Printf.sprintf "CrashRecord<%s %s hits=%d>" r.signature r.kind r.hits

let tier_reported = "caller-reported"
let tier_black_box = "black-box-uninstrumented"

type report = {
  iterations : int;
  admitted_seeds : int;
  total_edges : int;
  crashes : crash_record list;
  timeouts : int;
  ooms : int;
  crashes_dropped : int;
  admissions_dropped : int;
  coverage_tier : string;
  seed : int64;
  corpus : string list;
  leaked_workers : int;
}

let unique_crashes r = List.length r.crashes + r.crashes_dropped

let show_report r =
  Printf.sprintf
    "FuzzReport<iterations=%d seeds=%d edges=%d crashes=%d timeouts=%d ooms=%d>"
    r.iterations r.admitted_seeds r.total_edges (unique_crashes r) r.timeouts
    r.ooms

(* Can this be read as "the campaign found nothing"?  Only if at least one
   mutation iteration ran and nothing crashed, timed out, ran out of memory,
   leaked a worker or was dropped.  ZERO EXECUTIONS IS NEVER A PASS. *)
let is_clean r =
  r.iterations > 0 && r.crashes = [] && r.timeouts = 0 && r.ooms = 0
  && r.crashes_dropped = 0 && r.admissions_dropped = 0 && r.leaked_workers = 0

type reverify = Reproduced | Flaky | Unverified

type crash_status = {
  status_signature : string;
  status_kind : string;
  status_frames : string list;
  status_hits : int;
  status : reverify;
}

type verdict = {
  passed : bool;
  reason : string option;
  lines : string list;
  crash_statuses : crash_status list;
  v_iterations : int;
  v_timeouts : int;
  v_ooms : int;
  v_seed : int64 option;
  witnesses : (string * string) list option;
}

let show_verdict v =
  Printf.sprintf "FuzzVerdict<passed=%b reason=%s lines=%d>" v.passed
    (match v.reason with Some r -> r | None -> "-")
    (List.length v.lines)
