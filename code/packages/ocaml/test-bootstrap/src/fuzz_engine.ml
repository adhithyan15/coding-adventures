(* The deterministic, budgeted coverage-guided campaign (VER06 5 and 9.1, VER06r
   9).

   A campaign is a function of (corpus, options, exec): triage every seed, then
   repeatedly pick a seed, give it an energy-scaled number of mutated children,
   run each child and either admit it (new coverage), record it (crash, timeout,
   oom) or drop it (redundant).  The engine itself reads no clock, file or
   environment; only the executor spawns domains.

   [exec] is a function from the input to its outcome and the (reduced) edges of
   that execution.

   Per iteration the stream draws: [bounded (len corpus)] for the seed; per
   energy slot [1 + bounded 4] steps, one [next_u64] sub-seed, then, if there is
   more than one seed, [bounded 4] and on 0 a donor index and two split draws.
   That order is the cross-lane contract. *)

open Fuzz_types

type exec = string -> outcome * int list

type config = {
  retained_cap : int;
  tier : (int -> string) option;
  abort : (unit -> bool) option;
  leaked : (unit -> int) option;
}

let default_config =
  {
    retained_cap = Fuzz_limits.max_retained_bytes;
    tier = None;
    abort = None;
    leaked = None;
  }

(* -------------------------------------------------------------------- *)
(* Minimisation (VER06 5.3)                                              *)
(* -------------------------------------------------------------------- *)

let same_crash out sig_hash =
  match out with
  | Crash (kind, frames) -> Fuzz_signature.crash_signature kind frames = sig_hash
  | _ -> false

(* A copy of [s] with [len] bytes removed from [at]. *)
let cut s at len =
  String.sub s 0 at ^ String.sub s (at + len) (String.length s - at - len)

(* Shrinks [dat] while it keeps the EXACT signature.  Returns
   (bytes, executions_spent, finished).  EVERY candidate execution, accepted or
   rejected, is counted and the budget is checked before each one, so no stream
   of rejected candidates can run past it.

   Pass 1 deletes blocks, halving the block size.  It starts at
   [max 1 (len / 2)] (not [len / 2]), which is what makes the empty input
   reachable from a 1-byte crasher; the chunk sizes tried for a 9-byte input are
   4, 2, 1.  Pass 2 zeroes individual non-essential bytes.  [halt] is asked
   before EVERY probe: once it says true the pass ends at once, unfinished, so a
   campaign aborted for leaked workers stops probing. *)
let minimize_abortable halt dat sig_hash probe budget =
  if dat = "" then (dat, 0, true)
  else
    let rec delete_pass cur chunk offset spent =
      if chunk = 0 then zero_pass cur 0 spent
      else
        let size = String.length cur in
        if offset >= size then delete_pass cur (chunk / 2) 0 spent
        else if spent >= budget || halt () then (cur, spent, false)
        else
          let candidate = cut cur offset (min chunk (size - offset)) in
          if same_crash (probe candidate) sig_hash then
            (* same offset: the tail slid left *)
            delete_pass candidate chunk offset (spent + 1)
          else delete_pass cur chunk (offset + chunk) (spent + 1)
    and zero_pass cur i spent =
      if i >= String.length cur then (cur, spent, true)
      else if cur.[i] = '\000' then zero_pass cur (i + 1) spent
      else if spent >= budget || halt () then (cur, spent, false)
      else
        let candidate =
          String.sub cur 0 i ^ "\000"
          ^ String.sub cur (i + 1) (String.length cur - i - 1)
        in
        if same_crash (probe candidate) sig_hash then
          zero_pass candidate (i + 1) (spent + 1)
        else zero_pass cur (i + 1) (spent + 1)
    in
    delete_pass dat (max 1 (String.length dat / 2)) 0 0

let minimize dat sig_hash probe budget =
  minimize_abortable (fun () -> false) dat sig_hash probe budget

let minimize_crashing_input dat sig_hash budget probe =
  if budget < 0 || budget > Fuzz_limits.max_minimize_budget then
    raise
      (Error.Argument_error
         (Printf.sprintf "budget must be in [0, %d]"
            Fuzz_limits.max_minimize_budget));
  if String.length dat > Fuzz_limits.max_input_bytes then
    raise
      (Error.Argument_error
         (Printf.sprintf "data exceeds %d bytes" Fuzz_limits.max_input_bytes));
  let out, _, _ = minimize dat sig_hash probe budget in
  out

(* -------------------------------------------------------------------- *)
(* The campaign                                                          *)
(* -------------------------------------------------------------------- *)

type entry = { data : string; new_edges : int; generation : int }

(* The retained copy: charged by the caller (VER06r 8). *)
let copy s = String.sub s 0 (String.length s)

let run_outcomes_with cfg initial_corpus (o : options) (exec : exec) =
  validate_options o;
  if List.compare_length_with initial_corpus Fuzz_limits.max_initial_corpus > 0
  then
    raise
      (Error.Argument_error
         (Printf.sprintf "initial corpus exceeds %d seeds"
            Fuzz_limits.max_initial_corpus));
  List.iter
    (fun s ->
      if String.length s > Fuzz_limits.max_input_bytes then
        raise
          (Error.Argument_error
             (Printf.sprintf "an initial corpus seed exceeds %d bytes"
                Fuzz_limits.max_input_bytes)))
    initial_corpus;
  let rng = Prng.create o.seed in
  let seeds = if initial_corpus = [] then [ "\000" ] else initial_corpus in
  let cum = ref Fuzz_bitmap.empty in
  let corpus : entry array ref = ref [||] in
  let size = ref 0 in
  let crashes : (string, crash_record) Hashtbl.t = Hashtbl.create 16 in
  let order = ref [] in
  let timeouts = ref 0 and ooms = ref 0 and dropped = ref 0 in
  let adm_dropped = ref 0 and min_left = ref o.max_iterations in
  let retained = ref 0 in
  let aborted () = match cfg.abort with Some f -> f () | None -> false in
  let add_entry e =
    if !size = Array.length !corpus then (
      let bigger = Array.make (max 8 (2 * !size)) e in
      Array.blit !corpus 0 bigger 0 !size;
      corpus := bigger);
    !corpus.(!size) <- e;
    incr size
  in
  (* Puts a passing input in the corpus iff it covered a new edge.  The first
     passing seed is always admitted, even with no coverage, so there is
     something to mutate in black-box mode. *)
  let admit_seed dat generation es =
    let has_new, count_new, c = Fuzz_bitmap.admit !cum es in
    cum := c;
    let first_seed = generation = 0 && !size = 0 in
    if has_new || first_seed then
      if String.length dat > cfg.retained_cap - !retained then incr adm_dropped
      else (
        add_entry { data = copy dat; new_edges = count_new; generation };
        retained := !retained + String.length dat)
  in
  let repeat_crash sig_hash (r : crash_record) dat =
    if (not r.input_dropped) && String.length dat < String.length r.minimal_input
    then (
      let saved = String.length r.minimal_input - String.length dat in
      Hashtbl.replace crashes sig_hash
        { r with hits = r.hits + 1; minimal_input = copy dat };
      retained := !retained - saved)
    else Hashtbl.replace crashes sig_hash { r with hits = r.hits + 1 }
  in
  let novel_crash sig_hash kind frames dat =
    if Hashtbl.length crashes >= Fuzz_limits.max_crashes then incr dropped
    else
      let allowance = min o.minimize_budget !min_left in
      (* The minimiser's re-run: coverage is discarded and nothing is counted;
         once the campaign is aborted (leaked workers) nothing more runs. *)
      let probe d = if aborted () then Pass else fst (exec d) in
      let kept, spent, finished =
        minimize_abortable aborted dat sig_hash probe allowance
      in
      let over = String.length kept > cfg.retained_cap - !retained in
      let kept', is_dropped =
        if over then ("", true)
        else (
          retained := !retained + String.length kept;
          (copy kept, false))
      in
      Hashtbl.replace crashes sig_hash
        {
          signature = sig_hash;
          kind;
          frames;
          minimal_input = kept';
          hits = 1;
          minimized = finished && not is_dropped;
          input_dropped = is_dropped;
        };
      order := sig_hash :: !order;
      min_left := !min_left - spent
  in
  let file_outcome out dat =
    match out with
    | Timeout -> incr timeouts
    | Oom -> incr ooms
    | Pass -> ()
    | Crash (kind, frames) -> (
        let sig_hash = Fuzz_signature.crash_signature kind frames in
        match Hashtbl.find_opt crashes sig_hash with
        | Some r -> repeat_crash sig_hash r dat
        | None -> novel_crash sig_hash kind frames dat)
  in
  (* 1. Triage: run every seed once. *)
  List.iter
    (fun dat ->
      if not (aborted ()) then
        let out, es = exec dat in
        match out with Pass -> admit_seed dat 0 es | _ -> file_outcome out dat)
    seeds;
  (* 2. The mutation loop. *)
  let iterations = ref 0 in
  let stop = ref false in
  while !iterations < o.max_iterations && !size > 0 && not !stop do
    if aborted () then stop := true
    else
      let idx = Prng.bounded_int rng !size in
      let entry = !corpus.(idx) in
      let energy =
        Fuzz_bitmap.calculate_energy o.base_energy (String.length entry.data)
          entry.new_edges (Fuzz_bitmap.count !cum)
      in
      let slot = ref 0 in
      while !slot < energy && !iterations < o.max_iterations && not !stop do
        if aborted () then stop := true
        else (
          let n = Prng.bounded_int rng 4 in
          let sub = Prng.next_u64 rng in
          let candidate0 =
            Fuzz_mutate.mutate_sequence entry.data (Bigint.of_uint64 sub)
              (Bigint.of_int (1 + n))
              Fuzz_limits.max_input_bytes
          in
          let candidate1 =
            if !size > 1 && Prng.bounded_int rng 4 = 0 then (
              let di = Prng.bounded_int rng !size in
              let donor = !corpus.(di).data in
              let t_split = Prng.bounded_int rng (String.length candidate0 + 1) in
              let d_split = Prng.bounded_int rng (String.length donor + 1) in
              Fuzz_mutate.splice candidate0 donor (Bigint.of_int t_split)
                (Bigint.of_int d_split))
            else candidate0
          in
          let candidate =
            if String.length candidate1 > o.max_input_len then
              String.sub candidate1 0 o.max_input_len
            else candidate1
          in
          let out, es = exec candidate in
          (match out with
          | Pass -> admit_seed candidate (entry.generation + 1) es
          | _ -> file_outcome out candidate);
          incr slot;
          incr iterations)
      done
  done;
  let total = Fuzz_bitmap.count !cum in
  let leaked = match cfg.leaked with Some f -> f () | None -> 0 in
  let tier =
    match cfg.tier with
    | Some f -> f total
    | None -> if total > 0 then tier_reported else tier_black_box
  in
  {
    iterations = !iterations;
    admitted_seeds = !size;
    total_edges = total;
    crashes =
      List.filter_map (fun s -> Hashtbl.find_opt crashes s) (List.rev !order);
    timeouts = !timeouts;
    ooms = !ooms;
    crashes_dropped = !dropped;
    admissions_dropped = !adm_dropped;
    coverage_tier = tier;
    seed = o.seed;
    corpus = List.init !size (fun i -> !corpus.(i).data);
    leaked_workers = leaked;
  }

let run_outcomes initial_corpus o exec =
  run_outcomes_with default_config initial_corpus o exec
