(* Integrated shrinking.

   A property that fails on [913, -4, 77, 0, 2] has told you nothing; one that
   fails on [0] has told you what is wrong.  Shrinking walks from the first
   failing candidate toward the smallest one that still fails.  It is
   "integrated" because the generator that made the value knows its own
   bounds, so a shrunk [int 5 10] never becomes 0.

   A shrinker never re-charges an evaluation (the entry point's [falsifies]
   already did); it only charges the COPY of a candidate (work) before building
   it, so attempts equal evaluations. *)

type 'a falsifies = 'a -> bool
type 'a elem_shrink = 'a -> 'a falsifies -> int -> 'a * int

let ( <=! ) a b = Bigint.compare a b <= 0

let in_bounds lo hi v =
  (match lo with None -> true | Some l -> l <=! v)
  && match hi with None -> true | Some h -> v <=! h

(* Bisects then unit-steps [start] toward [target].  Each turn tries, in order,
   1. the bisection candidate current - floor(diff / 2) when |diff| > 1;
   2. the unit candidate current - sign diff.
   A candidate is accepted only if inside [lo, hi], different from current and
   still falsifying.  No acceptable candidate ends the walk.

   Worked example (corpus shrink_int/54321_le_100, property x <= 100):
   54321 -> 27161 -> 13581 -> ... -> 101 in 15 accepted moves; 100 would satisfy
   the property, so neither candidate from 101 is accepted.

   The division FLOORS on negatives (floor(-81 / 2) = -41), so from -81 toward 0
   the candidate is -81 - (-41) = -40; truncation would give -41. *)
let int_walk (cfg : Types.config) start target lo hi falsifies budget =
  let attempt cur cand =
    if (not (Bigint.equal cand cur)) && in_bounds lo hi cand && falsifies cand
    then Some cand
    else None
  in
  let one = Bigint.one in
  let rec go cur steps =
    if steps >= budget || Bigint.equal cur target then (cur, steps)
    else
      let diff = Bigint.sub cur target in
      let m1 =
        if Bigint.compare (Bigint.abs diff) one > 0 then
          attempt cur (cfg.bisect cur diff)
        else None
      in
      let m2 =
        match m1 with
        | Some _ -> m1
        | None -> attempt cur (cfg.unit_step cur diff)
      in
      match m2 with None -> (cur, steps) | Some c -> go c (steps + 1)
  in
  go start 0

(* The target probe followed by the walk (VER05 4.2): the target itself failing
   cannot be improved on and costs exactly one step. *)
let shrink_int_core cfg lo hi target initial falsifies max_steps =
  if max_steps = 0 then (initial, 0)
  else
    let probe =
      in_bounds lo hi target
      && (not (Bigint.equal target initial))
      && falsifies target
    in
    if probe then (target, 1)
    else int_walk cfg initial target lo hi falsifies max_steps

(* A byte shrunk toward 0 by the integer walk. *)
let walk_byte cfg w falsifies steps =
  let v, used =
    int_walk cfg (Bigint.of_int w) Bigint.zero (Some Bigint.zero)
      (Some (Bigint.of_int 255))
      (fun b ->
        match Bigint.to_int_opt b with Some i -> falsifies i | None -> false)
      steps
  in
  ((match Bigint.to_int_opt v with Some i -> i | None -> w), used)

let sub_arr a pos len = Array.sub a pos len

(* Shrinks a falsifying sequence (VER05 4.3).
   - probe: min_len = 0: does the empty list fail?  Then it is the answer, one
     step.  (min_len > 0: probe the length-min_len prefix and carry on from it.)
   - bisection: keep the left half, else the right half, while one fails.
   - deletion: single elements, right to left.
   - element-wise: left to right through the element shrinker, if any.
   Every candidate COPY is charged to the budget tree BEFORE it is built; once
   refused, every loop ends.  With max_steps = 0 or a spent budget the input is
   returned as is. *)
let shrink_sequence (cfg : Types.config) b ~min_len elem falsifies max_steps
    initial =
  if max_steps = 0 then (initial, 0)
  else
    let n0 = Array.length initial in
    if not (Budget.charge_copy b n0) then (initial, 0)
    else
      let floor_len = max 1 min_len in
      (* charge the copy, then build and evaluate; a refused charge is "does not falsify" *)
      let try_candidate cond len build =
        cond && Budget.charge_copy b len && falsifies (build ())
      in
      let probe =
        try_candidate (n0 > min_len) min_len (fun () ->
            sub_arr initial 0 min_len)
      in
      if probe && min_len = 0 then ([||], 1)
      else
        let cur = ref initial and steps = ref 0 in
        if probe then (
          cur := sub_arr initial 0 min_len;
          steps := 1);
        (* Phase 1: bisection, left half first, then the right half. *)
        let continue = ref true in
        while
          !continue && Array.length !cur > floor_len && !steps < max_steps
        do
          let n = Array.length !cur in
          let mid = n / 2 in
          if try_candidate (mid >= min_len) mid (fun () -> sub_arr !cur 0 mid)
          then (
            cur := sub_arr !cur 0 mid;
            incr steps)
          else if
            try_candidate
              (n - mid >= min_len)
              (n - mid)
              (fun () -> sub_arr !cur mid (n - mid))
          then (
            cur := sub_arr !cur mid (n - mid);
            incr steps)
          else continue := false
        done;
        (* Phase 2: single-element deletion, right to left. *)
        (if not cfg.skip_deletion then
           let i = ref (Array.length !cur - 1) in
           let continue = ref true in
           while
             !continue && !i >= 0
             && Array.length !cur > floor_len
             && !steps < max_steps
           do
             if not (Budget.room b) then continue := false
             else if not (Budget.charge_copy b (Array.length !cur - 1)) then
               continue := false
             else
               let len = Array.length !cur in
               let cand =
                 Array.append (sub_arr !cur 0 !i)
                   (sub_arr !cur (!i + 1) (len - !i - 1))
               in
               if falsifies cand then (
                 cur := cand;
                 incr steps;
                 i := min !i (Array.length cand - 1))
               else decr i
           done);
        (* Phase 3: element-wise, left to right. *)
        (match elem with
        | None -> ()
        | Some shrink_elem ->
            let size = Array.length !cur in
            let idx = ref 0 and continue = ref true in
            while !continue && !idx < size do
              let remaining = max_steps - !steps in
              if remaining <= 0 || not (Budget.room b) then continue := false
              else
                let with_element cand =
                  Budget.charge_copy b (Array.length !cur)
                  &&
                  let c = Array.copy !cur in
                  c.(!idx) <- cand;
                  falsifies c
                in
                let v, used = shrink_elem !cur.(!idx) with_element remaining in
                if used > 0 then (
                  let c = Array.copy !cur in
                  c.(!idx) <- v;
                  cur := c;
                  steps := !steps + used);
                incr idx
            done);
        (!cur, !steps)

(* Shrinks a falsifying integer toward [target] inside [lo, hi].  Raises
   Not_falsifying when [initial] satisfies the property and Argument_error for
   lo > hi, an initial outside the bounds or a budget outside [0, 10000].  A
   PUBLIC ENTRY POINT: opens a child of the parent (a fresh root without one)
   and charges every evaluation, the initial one included. *)
let shrink_int ?(config = Types.default_config) ?parent ?(max_steps = 1000)
    ~initial ~target ~lo ~hi prop =
  Types.check_steps max_steps;
  if Bigint.compare lo hi > 0 then
    raise (Error.Argument_error "min exceeds max");
  if not (in_bounds (Some lo) (Some hi) initial) then
    raise (Error.Argument_error "initial value must lie within [min, max]");
  Budget.with_child config (Budget.resolve parent) ~max_steps (fun b ->
      if not (Budget.charge b 1) then (initial, 0)
        (* starved by an enclosing shrink *)
      else (
        if prop initial then raise Error.Not_falsifying;
        let falsifies = Budget.charged_falsifies b (fun v -> not (prop v)) in
        shrink_int_core (Budget.config b) (Some lo) (Some hi) target initial
          falsifies max_steps))

(* A list of integers: empty probe, halves, single deletion, then each element
   toward [elem_target] (unbounded). *)
let shrink_list ?(config = Types.default_config) ?parent ?(max_steps = 1000)
    ~initial ~elem_target prop =
  Types.check_steps max_steps;
  Budget.with_child config (Budget.resolve parent) ~max_steps (fun b ->
      (* charged before the first copy and for the initial evaluation *)
      if
        not (Budget.charge_copy b (2 * List.length initial) && Budget.charge b 1)
      then (initial, 0)
      else (
        if prop initial then raise Error.Not_falsifying;
        let falsifies =
          Budget.charged_falsifies b (fun a -> not (prop (Array.to_list a)))
        in
        let walk v f steps =
          int_walk (Budget.config b) v elem_target None None f steps
        in
        let a, steps =
          shrink_sequence (Budget.config b) b ~min_len:0 (Some walk) falsifies
            max_steps (Array.of_list initial)
        in
        (Array.to_list a, steps)))

let bytes_of_ints a = String.init (Array.length a) (fun i -> Char.chr a.(i))

(* Falsifying bytes: sequence shrinking over byte values toward 0x00. *)
let shrink_bytes ?(config = Types.default_config) ?parent ?(max_steps = 1000)
    ~initial prop =
  Types.check_steps max_steps;
  Budget.with_child config (Budget.resolve parent) ~max_steps (fun b ->
      (* the copy, the array conversion and the string conversion back *)
      if
        not
          (Budget.charge_copy b (3 * String.length initial) && Budget.charge b 1)
      then (initial, 0)
      else (
        if prop initial then raise Error.Not_falsifying;
        (* each attempt also converts its candidate back: charge that copy too *)
        let falsifies a =
          Budget.charge_copy b (Array.length a)
          && Budget.charged_falsifies b
               (fun x -> not (prop x))
               (bytes_of_ints a)
        in
        let a, steps =
          shrink_sequence (Budget.config b) b ~min_len:0
            (Some (walk_byte (Budget.config b)))
            falsifies max_steps
            (Array.init (String.length initial) (fun i -> Char.code initial.[i]))
        in
        (bytes_of_ints a, steps)))

(* Replays a failure from nothing but [seed]: draw up to 1000 values
   bounded(1000); the first that falsifies is the initial failure, then shrunk
   toward 0 over [0, 1000].  Same seed, same pair, forever. *)
let simulate_replay ?(config = Types.default_config) ?parent seed prop =
  Budget.with_child config (Budget.resolve parent) ~max_steps:1000 (fun b ->
      let rng = (Budget.config b).new_rng seed in
      let thousand = Bigint.of_int 1000 in
      let rec draw k =
        if k = 0 || not (Budget.charge b 1) then None
        else
          let v = Prng.bounded rng thousand in
          if prop v then draw (k - 1) else Some v
      in
      match draw 1000 with
      | None ->
          raise
            (Error.Argument_error "property was not falsified within 1000 draws")
      | Some v ->
          let minimal, _ =
            shrink_int ~config:(Budget.config b) ~parent:b ~max_steps:1000
              ~initial:v ~target:Bigint.zero ~lo:Bigint.zero ~hi:thousand prop
          in
          (v, minimal))
