(* Limits, failure vocabulary, options, verdicts and the sabotage seams
   (VER05 5.1, VER05q sections 5 and 7).  Every loop and allocation in the
   engine is finite (CWE-400) because of the caps below. *)

let max_cases = 100_000
let max_shrink_steps_limit = 10_000
let max_input_bytes = 1_048_576
let max_list_len = 100_000
let max_gen_count = 1_000_000
let max_frequency_total = 4_294_967_296
let max_shrink_work = 64 * 1024 * 1024
let max_retries_limit = 10_000
let default_max_retries = 100
let max_budget_depth = 64
let max_check_nesting = 64
let max_int_bits = 16_384

(* One attempt per property evaluation; 16 * max_steps + 64 per shrink. *)
let attempt_budget max_steps = (16 * max 0 max_steps) + 64

let check_steps n =
  if n < 0 || n > max_shrink_steps_limit then
    raise
      (Error.Argument_error
         (Printf.sprintf "max_steps %d outside [0, %d]" n max_shrink_steps_limit))

let failure_counterexample = "property-counterexample"
let failure_exhausted = "property-exhausted"
let failure_timeout = "property-timeout"
let failure_panic = "property-panic"

let failure_ids =
  [ failure_counterexample; failure_exhausted; failure_timeout; failure_panic ]

let is_failure_id s = List.mem s failure_ids

type ci_mode = Ci_on | Ci_off | Ci_unset

(* Pure: the caller reads the environment (Sys.getenv_opt "CI") and passes it. *)
let ci_of_env = function
  | None -> Ci_off
  | Some v ->
      let t = String.lowercase_ascii (String.trim v) in
      if t = "" || t = "0" || t = "false" then Ci_off else Ci_on

(* The seams a negative control can break: a changed bisection, a changed unit
   step, a skipped deletion phase, a shifted generator, a changed work cap.  A
   plain value passed down: there is no global mutable state. *)
type config = {
  bisect : Bigint.t -> Bigint.t -> Bigint.t;
      (** current -> diff -> candidate *)
  unit_step : Bigint.t -> Bigint.t -> Bigint.t;
  skip_deletion : bool;
  new_rng : int64 -> Prng.t;
  max_work : int;
}

let default_config =
  {
    bisect = (fun cur diff -> Bigint.sub cur (Bigint.fdiv2 diff));
    unit_step =
      (fun cur diff -> Bigint.sub cur (Bigint.of_int (Bigint.sign diff)));
    skip_deletion = false;
    new_rng = (fun seed -> Prng.create seed);
    max_work = max_shrink_work;
  }

type options = {
  num_tests : int;
  max_shrink_steps : int;
  ci : ci_mode;
  config : config;
}

let default_options =
  {
    num_tests = 100;
    max_shrink_steps = 1000;
    ci = Ci_unset;
    config = default_config;
  }

let validate_options o =
  if o.num_tests < 0 || o.num_tests > max_cases then
    raise
      (Error.Argument_error
         (Printf.sprintf "num_tests %d outside [0, %d]" o.num_tests max_cases));
  check_steps o.max_shrink_steps

type verdict = {
  passed : bool;
  failure_id : string option;
  message : string option;
  cases_run : int;
  iteration : int;
  shrink_steps : int;
  seed : int64 option;
  counterexample : string option;
  minimal : string option;
  digest : string option;
}
