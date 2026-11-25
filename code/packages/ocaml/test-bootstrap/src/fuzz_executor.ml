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

   The deadline is WALL-clock time ([Unix.gettimeofday]; [unix] ships with the
   compiler) and the waiter sleeps with a backoff from 50 microseconds to 1 ms, so
   neither the waiter's own spinning nor a leaked spinner's CPU time can make an
   honest target time out.  A wall clock can be stepped by the system; a step
   forward is a false timeout, a step back a late one (fail closed either way).

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
type raw = Done | Sync of exn | Fatal of exn | Abandoned

let stale_kind = "stale-budget-sentinel"
let new_leaks () = Atomic.make 0

(* The leaked workers alive in the whole process (VER06r 7): incremented when
   one is given up on, decremented when it finally returns. *)
let global_leaks : int Atomic.t = Atomic.make 0

(* Polls the cell until it is filled or [ms] of time has passed, sleeping with a
   backoff (50 us up to 1 ms) instead of spinning.

   [Unix.gettimeofday] is not monotonic: a backward step would hang the wait for
   the length of the step, a forward step would end it early.  So elapsed time is
   ALSO accumulated from the sleeps themselves: each round adds the observed
   wall delta clamped to [delay, delay + 100 ms] (never less than the sleep that
   was requested, never more than a sleep plus a generous scheduling delay).  The
   wait ends when that accumulated time reaches the deadline; the raw wall clock
   is trusted too, but only while no step has been seen (a forward jump or a
   backward step marks it untrusted).  [now] and [sleep] are seams for a test. *)
let wait_with ~now ~sleep cell ms =
  let limit = float_of_int ms /. 1000. in
  let t0 = now () in
  let last = ref t0 and acc = ref 0. and stepped = ref false in
  let rec loop delay next =
    match Atomic.get cell with
    | Some _ as r -> r
    | None ->
        let t = now () in
        let d = t -. !last in
        last := t;
        if d < 0. || d > delay +. 0.1 then stepped := true;
        let add =
          if d < delay then delay
          else if d > delay +. 0.1 then delay +. 0.1
          else d
        in
        acc := !acc +. add;
        if !acc >= limit || ((not !stepped) && t -. t0 >= limit) then
          Atomic.get cell
        else (
          sleep next;
          loop next (if next >= 0.001 then 0.001 else next *. 2.))
  in
  loop 0. 0.00005

let wait cell ms = wait_with ~now:Unix.gettimeofday ~sleep:Unix.sleepf cell ms

let classify ex wall res =
  match ex with
  | Fuzz_handle_internal.Ex_steps -> Fuzz_types.Timeout
  | Fuzz_handle_internal.Ex_mem -> Fuzz_types.Oom
  | Fuzz_handle_internal.Ex_none -> (
      if wall then Fuzz_types.Timeout
      else
        match res with
        | None | Some (Fatal _) | Some Abandoned -> Fuzz_types.Timeout
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
  if Atomic.get global_leaks >= Fuzz_limits.max_global_leaked_workers then
    raise
      (Error.Engine_error
         (Printf.sprintf
            "%d leaked worker domains are still running in this process (cap \
             %d): refusing to start another execution"
            (Atomic.get global_leaks) Fuzz_limits.max_global_leaked_workers));
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
    (* a worker the engine already gave up on lost the race: it is no longer
       leaked, so the process-wide count drops *)
    if not (Atomic.compare_and_set cell None (Some r)) then
      Atomic.decr global_leaks
  in
  let dom =
    try Domain.spawn worker with
    | Failure m ->
        ignore (Fuzz_handle_internal.retire ih);
        raise
          (Error.Engine_error
             ("cannot spawn a worker domain (the runtime's domain ceiling, or \
               leaked workers still running): " ^ m))
    | e ->
        ignore (Fuzz_handle_internal.retire ih);
        raise e
  in
  let counted = ref false in
  let res, wall =
    (* an exception here (Sys.Break while waiting) retires the handle, so a
       cooperating worker unwinds instead of running on *)
    try
      match wait cell o.timeout_ms with
      | Some _ as r -> (r, false)
      | None -> (
          (* retire first: a cooperating target unwinds at its next handle call *)
          ignore (Fuzz_handle_internal.retire ih);
          let w =
            if Atomic.get leaks > 0 then leaked_kill_wait_ms else kill_wait_ms
          in
          match wait cell w with
          | Some _ as r -> (r, true)
          | None ->
              (* count it BEFORE the race: a worker that returns after our CAS
                 sees it failed and subtracts itself.  Both counters are raised
                 first and lowered again if the race is lost (or an exception
                 interrupts this window: the handler below reads the cell), so
                 neither can be left lost or drifted *)
              Atomic.incr leaks;
              Atomic.incr global_leaks;
              counted := true;
              if Atomic.compare_and_set cell None (Some Abandoned) then
                (None, true)
              else (
                counted := false;
                Atomic.decr global_leaks;
                Atomic.decr leaks;
                (Atomic.get cell, true)))
    with e ->
      ignore (Fuzz_handle_internal.retire ih);
      (* an exception between the increments and the end of the race: if the
         abandon marker is in the cell the leak stands, otherwise take it back *)
      if
        !counted
        && not
             (match Atomic.get cell with Some Abandoned -> true | _ -> false)
      then (
        Atomic.decr global_leaks;
        Atomic.decr leaks);
      raise e
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
