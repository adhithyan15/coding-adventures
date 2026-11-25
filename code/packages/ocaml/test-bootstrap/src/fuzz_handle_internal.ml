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
  mutable calls : int;
      (* tick and reserve calls: paces the allocation sampling *)
  map : Bytes.t;
  mutable owner : Domain.id option;
  mutable base : float * float * float;
  mutable self_words : float;
      (* words one [Gc.counters] call allocates, calibrated at [arm] *)
  mutable samples : int; (* readings taken since the baseline *)
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
        calls = 0;
        map = Bytes.make (Fuzz_limits.coverage_map_size / 8) '\000';
        owner = None;
        base = (0., 0., 0.);
        self_words = 0.;
        samples = 0;
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

(* Words (minor + major - promoted) in a [Gc.counters] reading. *)
let words (m, p, j) = m +. j -. p

(* Called by the worker domain at the start: the baseline of the allocation
   counters of THIS domain, and the calibration of the sampler's own cost.

   [Gc.counters ()] allocates (a tuple and three boxed floats) on the calling
   domain, and a reading includes the allocation of every earlier call.  Two
   back-to-back calls measure that cost exactly: the second reading minus the
   first IS the first call's allocation.  The baseline is the SECOND reading, so
   reading number k (k >= 1) contains exactly k calls' worth of the sampler's own
   allocation (the calibration call and the k-1 earlier readings), and [sample]
   subtracts [k * self_words]: the sampler is invisible to the target. *)
let arm h =
  let first = Gc.counters () in
  let second = Gc.counters () in
  let self = words second -. words first in
  Mutex.protect h.st.lock (fun () ->
      h.st.owner <- Some (Domain.self ());
      h.st.base <- second;
      h.st.self_words <- (if self > 0. then self else 0.);
      h.st.samples <- 0)

let alloc_clamp = 1 lsl 60

(* [Gc.counters] allocates (a tuple and three boxed floats, about 150 bytes) on
   the worker domain, and that allocation is charged to the target.  So the
   per-call path takes a reading only on every 64th [tick]/[reserve] call (and on
   the first), which amortises the cost to a few bytes per call; the rest of the
   path is allocation-free: no closure, no option, no tuple (VER06r 6). *)
let sample_mask = 63

let owner_is_self s =
  match s.owner with
  | Some d -> (d :> int) = (Domain.self () :> int)
  | None -> false

(* Bytes allocated by the calling domain since [arm], or -1 when the caller is
   not the armed domain (a retained handle used elsewhere reads nothing). *)
let sample h =
  if owner_is_self h.st then (
    let m, p, j = Gc.counters () in
    let m0, p0, j0 = h.st.base in
    let k = h.st.samples + 1 in
    h.st.samples <- k;
    let own = float_of_int k *. h.st.self_words in
    let bytes = (m -. m0 +. (j -. j0) -. (p -. p0) -. own) *. 8. in
    if bytes <= 0. then 0
    else if bytes >= float_of_int alloc_clamp then alloc_clamp
    else int_of_float bytes)
  else -1

let mem_used s = s.reserved + s.alloc

(* A final reading, taken by the worker after the target returned, so a target
   that allocates a lot and never ticks is still judged. *)
let observe h =
  let a = sample h in
  Mutex.protect h.st.lock (fun () -> if a > h.st.alloc then h.st.alloc <- a)

(* ---- operations -------------------------------------------------------- *)

(* A no-op on a retired or forged handle.  No closure: lock, work that cannot
   raise, unlock (the match keeps the unlock exception-safe). *)
let edge h id =
  let s = h.st in
  Mutex.lock s.lock;
  match
    if valid h then (
      s.edge_calls <- s.edge_calls + 1;
      let e = id land (Fuzz_limits.coverage_map_size - 1) in
      let k = e lsr 3 in
      Bytes.unsafe_set s.map k
        (Char.unsafe_chr
           (Char.code (Bytes.unsafe_get s.map k) lor (1 lsl (e land 7)))))
  with
  | () -> Mutex.unlock s.lock
  | exception e ->
      Mutex.unlock s.lock;
      raise e

(* The locked part of [spend]; returns 0 (fine), 1 (steps), 2 (mem) or 3
   (retired).  Sticky first (whichever budget breached), then the charge,
   clamped so a counter can never wrap. *)
let spend_locked h which n a =
  let s = h.st in
  s.calls <- s.calls + 1;
  if not (valid h) then 3
  else (
    if a > s.alloc then s.alloc <- a;
    match s.exhausted with
    | Ex_steps -> 1
    | Ex_mem -> 2
    | Ex_none ->
        (match which with
        | Steps ->
            let cap = s.step_budget + 1 in
            let n = if n < 0 then 0 else if n > cap then cap else n in
            s.steps <- s.steps + n
        | _ ->
            let cap = s.mem_budget + 1 in
            let n = if n < 0 then 0 else if n > cap then cap else n in
            s.reserved <- s.reserved + n);
        let over_steps = s.steps > s.step_budget
        and over_mem = mem_used s > s.mem_budget in
        let hit =
          match which with
          | Steps -> if over_steps then 1 else if over_mem then 2 else 0
          | _ -> if over_mem then 2 else if over_steps then 1 else 0
        in
        if hit = 1 then s.exhausted <- Ex_steps
        else if hit = 2 then s.exhausted <- Ex_mem;
        hit)

let spend h which n =
  let s = h.st in
  let a = if s.calls land sample_mask = 0 then sample h else -1 in
  Mutex.lock s.lock;
  let code =
    match spend_locked h which n a with
    | c ->
        Mutex.unlock s.lock;
        c
    | exception e ->
        Mutex.unlock s.lock;
        raise e
  in
  if code <> 0 then
    raise
      (Sentinel
         ( h.claimed,
           if code = 1 then Steps else if code = 2 then Mem else Retired ))

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
