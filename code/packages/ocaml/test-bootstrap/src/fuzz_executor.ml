(* One target execution in its OWN domain (VER06r 4).

   A fresh domain per execution: the engine regains control at the deadline even
   if the target never calls [tick], and the target's domain-local state and
   allocation counters start clean.  OCaml has NO safe asynchronous interruption
   of another domain, so the bounds are cooperative: [tick], [reserve] and
   RETIREMENT (a worker the engine gave up on gets the retired sentinel at its
   next handle call).  A tight loop that never calls the handle cannot be
   stopped in-process: it is reported [Timeout], its domain is LEAKED (never
   joined) and counted; the campaign aborts after [max_leaked_workers].  The CI
   job timeout is the backstop; the out-of-process runner (PT-F14) is the fix.

   The deadline is process CPU time ([Sys.time]): the standard library has no
   wall clock.  The waiting domain spins, so the CPU clock advances at least as
   fast as the wall clock.

   After the worker has finished (or been given up on) the engine (1) retires
   the handle, (2) reads the sticky exhausted state cross-checked with the
   counters, and only then (3) looks at how the worker ended.  A target that
   swallowed the budget sentinel and went on is still a timeout or an oom, never
   a pass and never a crash. *)

type target = string -> Fuzz_handle.t -> unit

type exec_result = {
  outcome : Fuzz_types.outcome;
  edges : int list;
  edge_calls : int;
  leaked : bool;
}

type leaks = int Atomic.t

(* How the worker ended.  [Fatal] is Sys.Break or Out_of_memory: never a
   verdict, re-raised in the engine's own domain. *)
type raw = Done | Sync of exn | Fatal of exn

let stale_kind = "stale-budget-sentinel"
let new_leaks () = Atomic.make 0

(* Polls the cell until it is filled or [ms] of process CPU time has passed. *)
let wait cell ms =
  let deadline = Sys.time () +. (float_of_int ms /. 1000.) in
  let n = ref 0 in
  let rec loop () =
    match Atomic.get cell with
    | Some _ as r -> r
    | None ->
        Domain.cpu_relax ();
        incr n;
        if !n land 1023 = 0 && Sys.time () >= deadline then Atomic.get cell
        else loop ()
  in
  loop ()

let classify ex wall res =
  match ex with
  | Fuzz_handle_internal.Ex_steps -> Fuzz_types.Timeout
  | Fuzz_handle_internal.Ex_mem -> Fuzz_types.Oom
  | Fuzz_handle_internal.Ex_none -> (
      if wall then Fuzz_types.Timeout
      else
        match res with
        | None | Some (Fatal _) -> Fuzz_types.Timeout
        | Some Done -> Fuzz_types.Pass
        | Some (Sync (Fuzz_handle_internal.Sentinel _)) ->
            Fuzz_types.Crash (stale_kind, [])
        | Some (Sync e) -> Fuzz_types.Crash (Fuzz_signature.kind_of e, []))

(* [on_handle] is a test seam called with the fresh handle before the target
   starts; [cross_check:false] is the negative-control seam. *)
let run_tracked ?(cross_check = true) ?on_handle
    ?(kill_wait_ms = Fuzz_limits.kill_wait_ms)
    ?(leaked_kill_wait_ms = Fuzz_limits.leaked_kill_wait_ms) (leaks : leaks)
    (target : target) input (o : Fuzz_types.options) =
  let ih =
    Fuzz_handle_internal.create ~step_budget:o.step_budget
      ~mem_budget:o.mem_budget
  in
  (match on_handle with Some f -> f ih | None -> ());
  let cell : raw option Atomic.t = Atomic.make None in
  let worker () =
    let r =
      try
        Fuzz_handle_internal.arm ih;
        match
          Safe.recoverable (fun () -> target input (Fuzz_handle.wrap ih))
        with
        | Ok () -> Done
        | Error e -> Sync e
      with e -> Fatal e
    in
    Fuzz_handle_internal.observe ih;
    Atomic.set cell (Some r)
  in
  let dom =
    try Domain.spawn worker
    with e ->
      ignore (Fuzz_handle_internal.retire ih);
      raise e
  in
  let res, wall =
    match wait cell o.timeout_ms with
    | Some _ as r -> (r, false)
    | None ->
        (* retire first: a cooperating target unwinds at its next handle call *)
        ignore (Fuzz_handle_internal.retire ih);
        let w =
          if Atomic.get leaks > 0 then leaked_kill_wait_ms else kill_wait_ms
        in
        let r2 = wait cell w in
        if r2 = None then Atomic.incr leaks;
        (r2, true)
  in
  (* a worker that answered is joined (instant); one that did not is leaked *)
  if res <> None then Domain.join dom;
  ignore (Fuzz_handle_internal.retire ih);
  let ex = Fuzz_handle_internal.exhausted ~cross_check ih in
  (match res with Some (Fatal e) -> raise e | _ -> ());
  {
    outcome = classify ex wall res;
    edges = Fuzz_handle_internal.edges ih;
    edge_calls = Fuzz_handle_internal.edge_calls ih;
    leaked = wall && res = None;
  }

let run ?cross_check ?on_handle ?kill_wait_ms ?leaked_kill_wait_ms target input
    o =
  run_tracked ?cross_check ?on_handle ?kill_wait_ms ?leaked_kill_wait_ms
    (new_leaks ()) target input o
