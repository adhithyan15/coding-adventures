(* The OCaml coverage-guided fuzz engine (VER06r): the public facade.

     let report =
       Fuzz.run [ "\000\000" ] { Fuzz.default_options with max_iterations = 500 }
         (fun data h ->
           Fuzz_handle.edge h (Fuzz_handle.edge_id "entered");
           if data = "\xff\xff" then failwith "boom")

   Every execution of the target runs in its own domain with a fresh handle (see
   [Fuzz_executor]).  Coverage is what the target reports through
   [Fuzz_handle.edge] (VER06r 3: bisect_ppx is not used in the hot path). *)

include Fuzz_types

type target = Fuzz_executor.target
type exec = Fuzz_engine.exec

let corpus_admit = Fuzz_bitmap.corpus_admit
let calculate_energy = Fuzz_bitmap.calculate_energy
let tap_failures = Fuzz_signature.tap_failures
let format_tap_failure = Fuzz_signature.format_tap_failure
let crash_signature = Fuzz_signature.crash_signature
let run_outcomes = Fuzz_engine.run_outcomes

(* The outcome-level executor for [target]: one domain per call, each with a
   PRIVATE leak counter.  It does NOT enforce the leak cap across calls: use
   [verdict_for] (or [verdict_tracked] with [executor_tracked] on one shared
   counter). *)
let executor target o : exec =
 fun dat ->
  let r = Fuzz_executor.run target dat o in
  (r.outcome, r.edges)

let executor_tracked leaks target o : exec =
 fun dat ->
  let r = Fuzz_executor.run_tracked leaks target dat o in
  (r.outcome, r.edges)

(* Runs a campaign against [target].  Any synchronous exception escaping it is a
   crash.  The coverage tier is [caller-reported] if the target called [edge]
   during the campaign and [black-box-uninstrumented] otherwise. *)
let run initial_corpus o target =
  let calls = Atomic.make 0 in
  let leaks = Fuzz_executor.new_leaks () in
  let exec dat =
    let r = Fuzz_executor.run_tracked leaks target dat o in
    ignore (Atomic.fetch_and_add calls r.edge_calls);
    (r.outcome, r.edges)
  in
  let tier _ = if Atomic.get calls > 0 then tier_reported else tier_black_box in
  Fuzz_engine.run_outcomes_with
    {
      Fuzz_engine.default_config with
      tier = Some tier;
      abort = Some (fun () -> Atomic.get leaks >= Fuzz_limits.max_leaked_workers);
      leaked = Some (fun () -> Atomic.get leaks);
    }
    initial_corpus o exec

let reason (r : report) =
  if r.leaked_workers >= Fuzz_limits.max_leaked_workers then "workers-leaked"
  else if r.crashes <> [] then "crash"
  else if r.timeouts > 0 then "timeout"
  else if r.ooms > 0 then "oom"
  else if r.crashes_dropped > 0 then "crashes_dropped"
  else if r.admissions_dropped > 0 then "admissions_dropped"
  else if r.leaked_workers > 0 then "workers-leaked"
  else "no_iterations"

(* The honest verdict of a report (VER06r 9).  Before a crash is reported its
   minimal input is run once more through [exec]: [Reproduced] if the signature
   comes back, [Flaky] if not (it still fails the verdict, fail closed),
   [Unverified] when no input was kept or the leak cap is reached.  [passed] is
   true only for a clean report.  With [Ci_on] or [Ci_unset] the verdict carries
   no seed and no witness.  [leaks] is shared with the executor, so leaks during
   re-verification count against the same cap of 4. *)
let verdict_tracked leaks (report : report) (o : options) (exec : exec) target_id
    =
  let redact = o.ci <> Types.Ci_off in
  let clean = is_clean report in
  let reverify (r : crash_record) =
    let status =
      if
        r.input_dropped
        || report.leaked_workers + Atomic.get leaks
           >= Fuzz_limits.max_leaked_workers
      then Unverified
      else if Fuzz_engine.same_crash (fst (exec r.minimal_input)) r.signature then
        Reproduced
      else Flaky
    in
    {
      status_signature = r.signature;
      status_kind = r.kind;
      status_frames = r.frames;
      status_hits = r.hits;
      status;
    }
  in
  let rec first n = function
    | x :: rest when n > 0 -> x :: first (n - 1) rest
    | _ -> []
  in
  (* sequential and ordered: each re-run sees the leaks of the one before *)
  let statuses =
    List.rev
      (List.fold_left
         (fun acc r -> reverify r :: acc)
         []
         (first Fuzz_limits.max_crashes report.crashes))
  in
  {
    passed = clean;
    reason = (if clean then None else Some (reason report));
    lines = Fuzz_signature.tap_failures report target_id 1;
    crash_statuses = statuses;
    v_iterations = report.iterations;
    v_timeouts = report.timeouts;
    v_ooms = report.ooms;
    v_seed = (if redact then None else Some report.seed);
    witnesses =
      (if redact then None
       else
         Some
           (List.filter_map
              (fun (r : crash_record) ->
                if r.input_dropped then None
                else Some (r.signature, r.minimal_input))
              report.crashes));
  }

let verdict report o exec target_id =
  verdict_tracked (Fuzz_executor.new_leaks ()) report o exec target_id

(* [verdict] for a target, with the re-verification leak cap ENFORCED: the
   executor and the verdict share ONE leak counter.  This is the call to use. *)
let verdict_for target report o target_id =
  let leaks = Fuzz_executor.new_leaks () in
  verdict_tracked leaks report o (executor_tracked leaks target o) target_id
