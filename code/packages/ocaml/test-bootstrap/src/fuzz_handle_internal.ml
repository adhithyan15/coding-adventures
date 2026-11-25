(* UNSAFE: the representation of the per-execution handle and its test seams
   (VER06r 5.1).  Targets receive the ABSTRACT [Fuzz_handle.t]; nothing in that
   interface gives back what is in here.  This module exists for the engine and
   for tests that must forge, clear or inspect a handle.  A hostile in-process
   target could call it; targets are trusted not to attack the harness.

   State: live or retired, the identity token, the step and memory budgets and
   counters, the sticky exhausted flag, the edge-call counter and the 65,536-bit
   edge map.  One [Mutex] guards it and is held only around integer arithmetic:
   nothing inside can raise and no target code runs under it, so a call can never
   leave the state half updated. *)

type token = { serial : int }
type which = Steps | Mem | Retired

exception Sentinel of token * which

type exhaustion = Ex_none | Ex_steps | Ex_mem

type state = {
  lock : Mutex.t;
  mutable live : bool;
  stored : token;
  step_budget : int;
  mem_budget : int;
  mutable steps : int;
  mutable reserved : int;
  mutable alloc : int; (* bytes allocated since [arm], latest reading *)
  mutable exhausted : exhaustion;
  mutable edge_calls : int;
  map : Bytes.t;
  mutable owner : Domain.id option;
  mutable base : float * float * float;
}

(* [claimed] is the token the CALLER holds; a call is honoured only if it is the
   stored one.  A forged handle shares the state but claims another token. *)
type t = { st : state; claimed : token }

let serials = Atomic.make 0
let mint_token () = { serial = Atomic.fetch_and_add serials 1 }

let create ~step_budget ~mem_budget =
  if step_budget < 0 || step_budget > Fuzz_limits.max_step_budget then
    raise (Error.Argument_error "step_budget out of range");
  if mem_budget < 0 || mem_budget > Fuzz_limits.max_mem_budget then
    raise (Error.Argument_error "mem_budget out of range");
  let tok = mint_token () in
  {
    claimed = tok;
    st =
      {
        lock = Mutex.create ();
        live = true;
        stored = tok;
        step_budget;
        mem_budget;
        steps = 0;
        reserved = 0;
        alloc = 0;
        exhausted = Ex_none;
        edge_calls = 0;
        map = Bytes.make (Fuzz_limits.coverage_map_size / 8) '\000';
        owner = None;
        base = (0., 0., 0.);
      };
  }

let token h = h.claimed
let same_token a b = a == b
let valid h = h.st.live && h.st.stored == h.claimed

(* ---- seams ------------------------------------------------------------ *)

(* The same state under another claimed token: inert. *)
let forge h tok = { h with claimed = tok }

(* Clears the sticky flag, as a hostile target that could reach the state
   would; the counters still tell the truth ([exhausted] cross-checks them). *)
let clear_exhausted h =
  Mutex.protect h.st.lock (fun () -> h.st.exhausted <- Ex_none)

(* ---- memory ------------------------------------------------------------ *)

(* Called by the worker domain at the start: the baseline of the allocation
   counters of THIS domain. *)
let arm h =
  let base = Gc.counters () in
  Mutex.protect h.st.lock (fun () ->
      h.st.owner <- Some (Domain.self ());
      h.st.base <- base)

let alloc_clamp = 1 lsl 60

(* Bytes allocated by the calling domain since [arm], or [None] when the caller
   is not the armed domain (a retained handle used elsewhere reads nothing). *)
let sample h =
  match h.st.owner with
  | Some d when d = Domain.self () ->
      let m, p, j = Gc.counters () in
      let m0, p0, j0 = h.st.base in
      let words = m -. m0 +. (j -. j0) -. (p -. p0) in
      let bytes = words *. 8. in
      if bytes <= 0. then Some 0
      else if bytes >= float_of_int alloc_clamp then Some alloc_clamp
      else Some (int_of_float bytes)
  | _ -> None

let mem_used s = s.reserved + s.alloc

(* A final reading, taken by the worker after the target returned, so a target
   that allocates a lot and never ticks is still judged. *)
let observe h =
  let a = sample h in
  Mutex.protect h.st.lock (fun () ->
      match a with Some a when a > h.st.alloc -> h.st.alloc <- a | _ -> ())

(* ---- operations -------------------------------------------------------- *)

(* A no-op on a retired or forged handle. *)
let edge h id =
  Mutex.protect h.st.lock (fun () ->
      if valid h then (
        h.st.edge_calls <- h.st.edge_calls + 1;
        let e = id land (Fuzz_limits.coverage_map_size - 1) in
        let k = e lsr 3 in
        Bytes.unsafe_set h.st.map k
          (Char.unsafe_chr
             (Char.code (Bytes.unsafe_get h.st.map k) lor (1 lsl (e land 7))))))

(* Shared by [tick] and [reserve]: sticky first (whichever budget breached),
   then the charge.  [charge] adds to the counter and returns the first budget
   found passed. *)
let spend h which n =
  let a = sample h in
  let r =
    Mutex.protect h.st.lock (fun () ->
        let s = h.st in
        if not (valid h) then Some Retired
        else (
          (match a with Some a when a > s.alloc -> s.alloc <- a | _ -> ());
          match s.exhausted with
          | Ex_steps -> Some Steps
          | Ex_mem -> Some Mem
          | Ex_none ->
              (* clamp so a counter can never wrap *)
              (match which with
              | Steps ->
                  let n = max 0 (min n (s.step_budget + 1)) in
                  s.steps <- s.steps + n
              | _ ->
                  let n = max 0 (min n (s.mem_budget + 1)) in
                  s.reserved <- s.reserved + n);
              let over_steps = s.steps > s.step_budget
              and over_mem = mem_used s > s.mem_budget in
              let hit =
                match which with
                | Steps ->
                    if over_steps then Some Steps
                    else if over_mem then Some Mem
                    else None
                | _ ->
                    if over_mem then Some Mem
                    else if over_steps then Some Steps
                    else None
              in
              (match hit with
              | Some Steps -> s.exhausted <- Ex_steps
              | Some _ -> s.exhausted <- Ex_mem
              | None -> ());
              hit))
  in
  match r with Some w -> raise (Sentinel (h.claimed, w)) | None -> ()

let tick h n = spend h Steps n
let reserve h n = spend h Mem n

(* Atomically retires; returns whether it was live. *)
let retire h =
  Mutex.protect h.st.lock (fun () ->
      let was = h.st.live in
      h.st.live <- false;
      was)

let is_retired h = Mutex.protect h.st.lock (fun () -> not h.st.live)

(* The sticky flag, cross-checked with the counters, so a target that clears the
   flag after a breach is still judged by steps > step_budget and
   mem > mem_budget.  [~cross_check:false] is the negative-control seam. *)
let exhausted ?(cross_check = true) h =
  Mutex.protect h.st.lock (fun () ->
      let s = h.st in
      match s.exhausted with
      | Ex_none when cross_check ->
          if s.steps > s.step_budget then Ex_steps
          else if mem_used s > s.mem_budget then Ex_mem
          else Ex_none
      | e -> e)

let edge_calls h = Mutex.protect h.st.lock (fun () -> h.st.edge_calls)
let steps_used h = Mutex.protect h.st.lock (fun () -> h.st.steps)
let mem_reserved h = Mutex.protect h.st.lock (fun () -> h.st.reserved)
let alloc_bytes h = Mutex.protect h.st.lock (fun () -> h.st.alloc)

(* The distinct edge ids set so far, ascending. *)
let edges h =
  Mutex.protect h.st.lock (fun () ->
      let out = ref [] in
      for e = Fuzz_limits.coverage_map_size - 1 downto 0 do
        if
          Char.code (Bytes.unsafe_get h.st.map (e lsr 3)) land (1 lsl (e land 7))
          <> 0
        then out := e :: !out
      done;
      !out)
