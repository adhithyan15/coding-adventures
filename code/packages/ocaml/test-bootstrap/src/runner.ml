(* The honest runner.

   - zero cases evaluated is property-exhausted, never a pass;
   - a generator error is property-exhausted;
   - a property that raises is property-panic (Sys.Break and Out_of_memory are
     never swallowed: see Safe);
   - the reported minimal witness is RE-VERIFIED: if the shrinker raises,
     returns something that no longer fails, a value the property raises on, or
     a nonsensical step count, the original candidate is reported with 0 steps;
   - redacted mode never emits the witness or the seed;
   - the call always returns a verdict (invalid options raise instead).

   There is no wall-clock timeout: property-timeout is registered and never
   produced.  A caller that needs one runs [check] under its own watchdog, which
   works because Sys.Break propagates. *)

open Types

let fail_v fid msg cases iter =
  {
    passed = false;
    failure_id = Some fid;
    message = Some msg;
    cases_run = cases;
    iteration = iter;
    shrink_steps = 0;
    seed = None;
    counterexample = None;
    minimal = None;
    digest = None;
  }

let rec take n = function x :: xs when n > 0 -> x :: take (n - 1) xs | _ -> []

(* A text or byte string counts its length; a collection counts its length
   plus, one level deeper, the length of each child that is itself a collection
   or string, visiting at most 10,000 children. *)
let val_work v =
  let shallow = function
    | Val.Bytes s | Val.Text s -> String.length s
    | Val.List ys | Val.Tuple ys -> List.length (take 10_000 ys)
    | _ -> 0
  in
  let collection xs =
    let kids = take 10_000 xs in
    List.length kids + List.fold_left (fun a k -> a + shallow k) 0 kids
  in
  match v with
  | Val.Bytes s | Val.Text s -> String.length s
  | Val.List xs | Val.Tuple xs -> collection xs
  | _ -> 0

let work_of canon v =
  match Safe.recoverable (fun () -> val_work (canon v)) with
  | Ok n -> n
  | Error _ -> 0

let spent i =
  fail_v failure_exhausted
    (Printf.sprintf "the enclosing shrink budget was spent at iteration %d" i)
    (max 0 (i - 1))
    i

let exhausted e i =
  let text =
    match e with
    | Error.Exhausted _ ->
        Printf.sprintf
          "filter precondition rejected all candidates at iteration %d" i
    | _ ->
        Printf.sprintf "generator error at iteration %d (%s)" i
          (Safe.exn_name e)
  in
  fail_v failure_exhausted text (i - 1) i

let panic seed redact e i =
  let msg =
    Printf.sprintf "property evaluation raised at iteration %d (%s)" i
      (Safe.exn_name e)
  in
  let base = fail_v failure_panic msg i i in
  if redact then base else { base with seed = Some seed }

let check ?parent ?shrink_override ~seed (gen : 'a Gen.t) opts prop =
  validate_options opts;
  let redact = opts.ci <> Ci_off in
  let num_tests = opts.num_tests
  and max_steps = opts.max_shrink_steps
  and cfg = opts.config in
  let parent = Budget.resolve parent in
  let nested = Option.is_some parent in
  if num_tests = 0 then
    fail_v failure_exhausted
      "no cases were evaluated (num_tests is 0); a property that never ran is \
       not a pass"
      0 0
  else if match parent with Some p -> not (Budget.charge p 1) | None -> false
  then spent 0
  else
    (* the WHOLE check, case loop included, runs in one scope *)
    let run scope =
      let rng = (Budget.config scope).new_rng seed in
      let finish () =
        if nested || not (Budget.is_refused scope) then
          {
            passed = true;
            failure_id = None;
            message = None;
            cases_run = num_tests;
            iteration = 0;
            shrink_steps = 0;
            seed = None;
            counterexample = None;
            minimal = None;
            digest = None;
          }
        else
          fail_v failure_exhausted
            "a nested check was starved by the shrink budget; the property was \
             not fully evaluated"
            num_tests num_tests
      in
      (* True while the candidate still falsifies.  A candidate on which the
         property raises is a different failure, never a smaller counterexample. *)
      let falsifies v =
        match Safe.recoverable (fun () -> prop v) with
        | Ok b -> not b
        | Error _ -> false
      in
      let falsified cand i =
        let shrunk =
          Safe.recoverable (fun () ->
              Budget.with_child cfg (Some scope) ~max_steps (fun sc ->
                  let charged = Budget.charged_falsifies sc falsifies in
                  match shrink_override with
                  | Some s -> s sc cand charged max_steps
                  | None -> Gen.shrink_nested sc gen cand charged max_steps))
        in
        let minimal, steps =
          match shrunk with
          | Ok (v, used) when used >= 0 && used <= max_steps ->
              (* Re-verify: the witness we report must really fail the property.
               One attempt is charged to the check scope; if the chain refuses it
               the shrunk value is not adopted. *)
              if Budget.charge scope 1 && falsifies v then (v, used)
              else (cand, 0)
          | _ -> (cand, 0)
        in
        let minimal_text = Canon.render_with gen.canon minimal in
        if redact then
          let preimage =
            Printf.sprintf "iter:%d;steps:%d;shrunk:%s" i steps minimal_text
          in
          let digest = Printf.sprintf "%016Lx" (Prng.fnv1a64 preimage) in
          let msg =
            Printf.sprintf
              "property falsified after %d iterations (shrunk in %d steps, \
               repro-fnv: %s)"
              i steps digest
          in
          {
            (fail_v failure_counterexample msg i i) with
            shrink_steps = steps;
            digest = Some digest;
          }
        else
          let initial_text = Canon.render_with gen.canon cand in
          let msg =
            Printf.sprintf
              "property falsified at iteration %d (seed=%Lu): initial=%s, \
               minimal=%s after %d shrink steps"
              i seed initial_text minimal_text steps
          in
          {
            (fail_v failure_counterexample msg i i) with
            shrink_steps = steps;
            seed = Some seed;
            counterexample = Some initial_text;
            minimal = Some minimal_text;
          }
      in
      let rec run_cases i =
        if i > num_tests then finish ()
        else if nested && i > 1 && not (Budget.charge scope 1) then spent i
        else run_case i
      and run_case i =
        match Safe.recoverable (fun () -> gen.sample (Some scope) rng) with
        | Error e -> exhausted e i
        | Ok cand -> (
            if nested && not (Budget.charge_copy scope (work_of gen.canon cand))
            then spent i
            else
              match Safe.recoverable (fun () -> prop cand) with
              | Error e -> panic seed redact e i
              | Ok true -> run_cases (i + 1)
              | Ok false -> falsified cand i)
      in
      run_cases 1
    in
    match Budget.with_check cfg parent ~num_tests ~max_steps run with
    | Some v -> v
    | None ->
        fail_v failure_exhausted
          "check nesting is too deep; nothing was evaluated" 0 0
