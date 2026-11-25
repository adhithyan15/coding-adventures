(* The generator algebra.

   Values are native OCaml values (bytes are [string], lists are lists, tuples
   are pairs and triples, integers are [Bigint.t] or native [int]); the engine
   reaches them only through each generator's [canon].

   Invalid generators fail closed: a constructor given invalid arguments returns
   a generator whose sampler raises [Argument_error]; the runner reports that as
   property-exhausted, so an invalid generator can never pass. *)

type 'a shrinker = Budget.t -> 'a -> 'a Shrink.falsifies -> int -> 'a * int

type 'a t = {
  sample : Budget.t option -> Prng.t -> 'a;
  shrink : 'a shrinker option;
  canon : 'a -> Val.t;
}

let sample ?ctx g rng = g.sample ctx rng

let shrink_nested b g v f max_steps =
  Types.check_steps max_steps;
  match g.shrink with None -> (v, 0) | Some s -> s b v f max_steps

let shrink ?(config = Types.default_config) ?parent g v falsifies max_steps =
  Types.check_steps max_steps;
  Budget.with_child config (Budget.resolve parent) ~max_steps (fun b ->
      shrink_nested b g v (Budget.charged_falsifies b falsifies) max_steps)

let custom ~canon ?shrink sample = { sample; shrink; canon }

let invalid ~canon msg =
  {
    sample = (fun _ _ -> raise (Error.Argument_error msg));
    shrink = None;
    canon;
  }

let constant ~canon v = { sample = (fun _ _ -> v); shrink = None; canon }
let int_val b = Val.Int b
let big = Bigint.of_int

(* The in-range point nearest zero: the integer shrink target. *)
let clamp_zero lo hi =
  if Bigint.sign lo > 0 then lo
  else if Bigint.sign hi < 0 then hi
  else Bigint.zero

let int_shrinker lo hi : Bigint.t shrinker =
 fun b v f max_steps ->
  Shrink.shrink_int_core (Budget.config b) (Some lo) (Some hi)
    (clamp_zero lo hi) v f max_steps

(* bounded(2) == 1.  Shrinks true to false. *)
let bool =
  {
    sample = (fun _ r -> Prng.bounded_u64 r 2L = 1L);
    shrink =
      Some
        (fun _ v f max_steps ->
          if v && max_steps > 0 then if f false then (false, 1) else (v, 0)
          else (v, 0));
    canon = (fun x -> Val.Bool x);
  }

(* [0, 255]: bounded(256). *)
let u8 =
  {
    sample = (fun _ r -> big (Int64.to_int (Prng.bounded_u64 r 256L)));
    shrink = Some (int_shrinker Bigint.zero (big 255));
    canon = int_val;
  }

let two64_m1 = Bigint.sub (Bigint.pow2 64) Bigint.one

(* A full 64-bit word: next_u64. *)
let u64 =
  {
    sample = (fun _ r -> Bigint.of_uint64 (Prng.next_u64 r));
    shrink = Some (int_shrinker Bigint.zero two64_m1);
    canon = int_val;
  }

let two127 = Bigint.pow2 127
let two128 = Bigint.pow2 128

(* Two stream words, high first, read as two's complement. *)
let sample_i128 r =
  let hi = Prng.next_u64 r in
  let lo = Prng.next_u64 r in
  let v =
    Bigint.add
      (Bigint.shift_left (Bigint.of_uint64 hi) 64)
      (Bigint.of_uint64 lo)
  in
  if Bigint.compare v two127 >= 0 then Bigint.sub v two128 else v

let i128 =
  {
    sample = (fun _ r -> sample_i128 r);
    shrink =
      Some (int_shrinker (Bigint.neg two127) (Bigint.sub two127 Bigint.one));
    canon = int_val;
  }

let bounded_int_big r lo hi =
  Bigint.add lo (Prng.bounded_big r (Bigint.add (Bigint.sub hi lo) Bigint.one))

(* A uniform integer over the inclusive range (VER05 7.2): the offset from lo is
   drawn by bounded_big.  Shrinks toward the in-range point nearest zero and
   never leaves the range.  A span wider than 16,384 bits is refused. *)
let integer lo hi =
  if Bigint.compare lo hi > 0 then
    invalid ~canon:int_val "integer needs min <= max"
  else if
    Bigint.bit_length (Bigint.add (Bigint.sub hi lo) Bigint.one)
    > Types.max_int_bits
  then invalid ~canon:int_val "integer span exceeds the width cap"
  else
    {
      sample = (fun _ r -> bounded_int_big r lo hi);
      shrink = Some (int_shrinker lo hi);
      canon = int_val;
    }

(* Native ints: the same draws and the same walk, converted at the boundary. *)
let int lo hi =
  let canon x = Val.Int (big x) in
  if lo > hi then invalid ~canon "int needs min <= max"
  else
    let g = integer (big lo) (big hi) in
    let to_native b =
      match Bigint.to_int_opt b with Some i -> i | None -> lo
    in
    {
      sample = (fun c r -> to_native (g.sample c r));
      shrink =
        Some
          (fun b v f steps ->
            let r, used =
              int_shrinker (big lo) (big hi) b (big v)
                (fun x -> f (to_native x))
                steps
            in
            (to_native r, used));
      canon;
    }

let check_lengths min_len max_len limit =
  if min_len < 0 || max_len < min_len then Some "needs 0 <= min_len <= max_len"
  else if max_len > limit then
    Some (Printf.sprintf "max_len %d exceeds %d" max_len limit)
  else None

(* A native length in [min_len, max_len]: min_len + bounded(span). *)
let draw_length r min_len max_len =
  min_len
  + Int64.to_int (Prng.bounded_u64 r (Int64.of_int (max_len - min_len + 1)))

let bytes_canon s = Val.Bytes s

(* L = min_len + bounded(span), then L draws of bounded(256).  Shrinks by
   sequence shrinking, respecting min_len. *)
let bytes min_len max_len =
  match check_lengths min_len max_len Types.max_input_bytes with
  | Some msg -> invalid ~canon:bytes_canon ("bytes " ^ msg)
  | None ->
      let sampler _ r =
        let n = draw_length r min_len max_len in
        let a = Array.make n 0 in
        for i = 0 to n - 1 do
          a.(i) <- Int64.to_int (Prng.bounded_u64 r 256L)
        done;
        String.init n (fun i -> Char.chr a.(i))
      in
      let shrinker b v f max_steps =
        (* the array conversion (and the string one on the way out) are copies too *)
        if max_steps = 0 || not (Budget.charge_copy b (2 * String.length v))
        then (v, 0)
        else
          let cfg = Budget.config b in
          let wrapped a =
            Budget.charge_copy b (Array.length a)
            && f (String.init (Array.length a) (fun i -> Char.chr a.(i)))
          in
          let a, steps =
            Shrink.shrink_sequence cfg b ~min_len
              (Some (Shrink.walk_byte cfg))
              wrapped max_steps
              (Array.init (String.length v) (fun i -> Char.code v.[i]))
          in
          (String.init (Array.length a) (fun i -> Char.chr a.(i)), steps)
      in
      { sample = sampler; shrink = Some shrinker; canon = bytes_canon }

(* Transforms generated values.  Drops shrinking: the function has no inverse. *)
let map ~canon fn g =
  { sample = (fun c r -> fn (g.sample c r)); shrink = None; canon }

(* Keeps only values satisfying the predicate.  After exactly max_retries
   rejected samples sampling raises Exhausted.  Shrinking is preserved: a
   candidate failing the predicate is simply not a valid input.  Every retry is
   CHARGED whenever a budget scope exists, so stacked filters cannot multiply
   uncharged work: in a NESTED check a retry is an attempt charged up the chain;
   at a top-level check root it is one element of work.  The predicate is charged
   BEFORE it runs: it is property code and may be costly. *)
let filter ?(max_retries = Types.default_max_retries) p g =
  if max_retries < 1 || max_retries > Types.max_retries_limit then
    invalid ~canon:g.canon
      (Printf.sprintf "max_retries %d outside [1, %d]" max_retries
         Types.max_retries_limit)
  else
    let sampler ctx r =
      let rec go k =
        if k = 0 then
          raise
            (Error.Exhausted
               (Printf.sprintf "filter rejected %d samples" max_retries))
        else (
          (match ctx with
          | None -> ()
          | Some b ->
              let ok =
                if Budget.is_nested b then Budget.charge b 1
                else Budget.charge_copy b 1
              in
              if not ok then
                raise
                  (Error.Exhausted
                     "the enclosing shrink budget was spent while filtering"));
          let v = g.sample ctx r in
          if p v then v else go (k - 1))
      in
      go max_retries
    in
    let shrinker b v f max_steps =
      shrink_nested b g v
        (fun c -> Budget.charge_copy b 1 && p c && f c)
        max_steps
    in
    { sample = sampler; shrink = Some shrinker; canon = g.canon }

(* Dependent generation: sample, then sample from the generator the function
   returns.  No shrinking. *)
let flat_map ~canon fn g =
  {
    sample =
      (fun c r ->
        let a = g.sample c r in
        (fn a).sample c r);
    shrink = None;
    canon;
  }

(* Picks one generator uniformly (bounded(len), then that arm's draws). *)
let one_of ~canon = function
  | [] -> invalid ~canon "one_of needs at least one generator"
  | arms ->
      let arr = Array.of_list arms in
      let n = Array.length arr in
      {
        sample = (fun c r -> arr.(Prng.bounded_int r n).sample c r);
        shrink = None;
        canon;
      }

(* A weighted choice: pick = bounded(total); arms are walked in order
   subtracting each weight, so a zero-weight arm can never be selected. *)
let frequency ~canon arms =
  if List.length arms = 0 then invalid ~canon "frequency needs at least one arm"
  else if List.exists (fun (w, _) -> w < 0) arms then
    invalid ~canon "frequency weights must be >= 0"
  else
    (* each weight is capped BEFORE adding, so the sum cannot wrap a 63-bit int *)
    let total =
      List.fold_left
        (fun a (w, _) ->
          if w > Types.max_frequency_total || a > Types.max_frequency_total then
            Types.max_frequency_total + 1
          else a + w)
        0 arms
    in
    if total = 0 then invalid ~canon "frequency total weight must be > 0"
    else if total > Types.max_frequency_total then
      invalid ~canon "frequency total weight exceeds 2^32"
    else
      let rec walk c r pick = function
        | [] -> invalid_arg "frequency: pick >= total"
        | (w, g) :: rest ->
            if pick < w then g.sample c r else walk c r (pick - w) rest
      in
      {
        sample = (fun c r -> walk c r (Prng.bounded_int r total) arms);
        shrink = None;
        canon;
      }

(* A list: L = min_len + bounded(span), then L elements in order.  Shrinks by
   sequence shrinking with the element generator's shrinker for the
   element-wise phase. *)
let list ?(min_len = 0) ?(max_len = 100) el =
  let canon xs = Val.List (List.map el.canon xs) in
  match check_lengths min_len max_len Types.max_list_len with
  | Some msg -> invalid ~canon ("list " ^ msg)
  | None ->
      let sampler c r =
        let n = draw_length r min_len max_len in
        let acc = ref [] in
        for _ = 1 to n do
          acc := el.sample c r :: !acc
        done;
        List.rev !acc
      in
      let shrinker b v f max_steps =
        let a, steps =
          Shrink.shrink_sequence (Budget.config b) b ~min_len
            (Some (fun x g s -> shrink_nested b el x g s))
            (fun a -> f (Array.to_list a))
            max_steps (Array.of_list v)
        in
        (Array.to_list a, steps)
      in
      { sample = sampler; shrink = Some shrinker; canon }

(* Samples each generator left to right and shrinks one coordinate at a time, in
   repeated passes until a pass makes no progress.  All passes share the one
   budget scope, so the repeat-until-no-progress loop cannot multiply the work. *)
let tuple2 ga gb =
  let sampler c r =
    let x = ga.sample c r in
    let y = gb.sample c r in
    (x, y)
  in
  let shrinker b (x0, y0) f max_steps =
    let rec pass x y steps =
      if steps >= max_steps then ((x, y), steps)
      else
        let x', u1 =
          shrink_nested b ga x (fun c -> f (c, y)) (max_steps - steps)
        in
        let steps1 = steps + u1 in
        let y', u2 =
          if max_steps - steps1 > 0 then
            shrink_nested b gb y (fun c -> f (x', c)) (max_steps - steps1)
          else (y, 0)
        in
        if u1 + u2 > 0 then pass x' y' (steps1 + u2) else ((x', y'), steps1)
    in
    pass x0 y0 0
  in
  {
    sample = sampler;
    shrink = Some shrinker;
    canon = (fun (x, y) -> Val.Tuple [ ga.canon x; gb.canon y ]);
  }

let tuple3 ga gb gc =
  let sampler c r =
    let x = ga.sample c r in
    let y = gb.sample c r in
    let z = gc.sample c r in
    (x, y, z)
  in
  let shrinker b (x0, y0, z0) f max_steps =
    let rec pass x y z steps =
      if steps >= max_steps then ((x, y, z), steps)
      else
        let x', u1 =
          shrink_nested b ga x (fun c -> f (c, y, z)) (max_steps - steps)
        in
        let s1 = steps + u1 in
        let y', u2 =
          if max_steps - s1 > 0 then
            shrink_nested b gb y (fun c -> f (x', c, z)) (max_steps - s1)
          else (y, 0)
        in
        let s2 = s1 + u2 in
        let z', u3 =
          if max_steps - s2 > 0 then
            shrink_nested b gc z (fun c -> f (x', y', c)) (max_steps - s2)
          else (z, 0)
        in
        if u1 + u2 + u3 > 0 then pass x' y' z' (s2 + u3) else ((x', y', z'), s2)
    in
    pass x0 y0 z0 0
  in
  {
    sample = sampler;
    shrink = Some shrinker;
    canon = (fun (x, y, z) -> Val.Tuple [ ga.canon x; gb.canon y; gc.canon z ]);
  }

(* ---------------------------------------------------------------------- *)
(* Corpus entry points (pbt-v1)                                             *)
(* ---------------------------------------------------------------------- *)

let check_count count =
  if count < 0 || count > Types.max_gen_count then
    raise (Error.Argument_error "count outside the generation limit")

let replicate count f =
  let acc = ref [] in
  for _ = 1 to count do
    acc := f () :: !acc
  done;
  List.rev !acc

(* count primitives of [ty] (bool, u8, u64, i128) from [seed]. *)
let gen_primitive (cfg : Types.config) seed ty count =
  check_count count;
  let rng = cfg.new_rng seed in
  match ty with
  | "bool" ->
      replicate count (fun () -> Val.Bool (Prng.bounded_u64 rng 2L = 1L))
  | "u8" ->
      replicate count (fun () ->
          Val.Int (big (Int64.to_int (Prng.bounded_u64 rng 256L))))
  | "u64" ->
      replicate count (fun () -> Val.Int (Bigint.of_uint64 (Prng.next_u64 rng)))
  | "i128" -> replicate count (fun () -> Val.Int (sample_i128 rng))
  | _ -> raise (Error.Argument_error "unknown primitive type")

(* count uniform integers over [lo, hi] from [seed].  Each draw is about as wide
   as the span, so count x width bounds what the returned list can hold. *)
let gen_bounded (cfg : Types.config) seed lo hi count =
  if Bigint.compare lo hi > 0 then raise (Error.Argument_error "need lo <= hi");
  check_count count;
  if Bigint.bit_length (Bigint.sub hi lo) > Types.max_int_bits then
    raise (Error.Argument_error "span exceeds the width cap");
  let width = max 1 ((Bigint.bit_length (Bigint.sub hi lo) + 7) / 8) in
  if count * width > 1_048_576 then
    raise (Error.Argument_error "count x span exceeds the generation budget");
  let rng = cfg.new_rng seed in
  replicate count (fun () -> Val.Int (bounded_int_big rng lo hi))

(* count byte strings with length in [min_len, max_len] from [seed]. *)
let gen_bytes (cfg : Types.config) seed min_len max_len count =
  (match check_lengths min_len max_len Types.max_input_bytes with
  | Some m -> raise (Error.Argument_error m)
  | None -> ());
  check_count count;
  if max_len > 0 && count > 1_048_576 / max_len then
    raise (Error.Argument_error "count x max_len exceeds the generation budget");
  let rng = cfg.new_rng seed in
  let g = bytes min_len max_len in
  replicate count (fun () -> Val.Bytes (g.sample None rng))
