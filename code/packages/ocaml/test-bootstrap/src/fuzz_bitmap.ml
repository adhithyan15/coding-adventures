(* The 65,536-edge execution map, admission and the power schedule (VER06 4,
   VER06r 2).

   The map is a VALUE: a set of edge ids reduced with [land 0xFFFF] (which equals
   the floored [mod 65536], so -1 is edge 65535; collisions are the known price
   of a fixed-size map, AFL pays the same one). *)

module IS = Set.Make (Int)
module LS = Set.Make (Int64)

type t = IS.t

let empty = IS.empty
let reduce_edge e = e land (Fuzz_limits.coverage_map_size - 1)
let record t e = IS.add (reduce_edge e) t
let has_edge t e = IS.mem (reduce_edge e) t

(* ORs a trace into the cumulative map: (has_new, count_new, map), over the
   REDUCED ids. *)
let admit t trace =
  let novel =
    List.fold_left
      (fun acc e ->
        let r = reduce_edge e in
        if IS.mem r t then acc else IS.add r acc)
      IS.empty trace
  in
  let n = IS.cardinal novel in
  (n > 0, n, IS.union t novel)

let covered_edges t = IS.elements t
let count t = IS.cardinal t

(* The 8,192-byte map: bit (e mod 8) of byte (e div 8). *)
let raw_bitmap t =
  let b = Bytes.make (Fuzz_limits.coverage_map_size / 8) '\000' in
  IS.iter
    (fun e ->
      let k = e lsr 3 in
      Bytes.set b k
        (Char.chr (Char.code (Bytes.get b k) lor (1 lsl (e land 7)))))
    t;
  Bytes.to_string b

(* Pure novelty check over the UNREDUCED u64 ids, counting UNIQUE novel edges.
   Ids are compared for equality only, so signed [int64] order is irrelevant. *)
let corpus_admit existing trace =
  let have = LS.of_list existing in
  let novel = LS.diff (LS.of_list trace) have in
  (not (LS.is_empty novel), LS.cardinal novel)

(* Mutation slots for a seed: max 1 (round_half_up (base * (1 + new/total) /
   sqrt len)).  The edge bonus is 1.0 while nothing is covered.  Rounding is half
   AWAY FROM ZERO (floor (x + 0.5)); banker's rounding would give 2 for 2.5.

   Worked example: base 10, len 4, 3 new of 6 total: 10 * 1.5 / 2 = 7.5, so 8. *)
let calculate_energy base_energy seed_len new_edges total_edges =
  let base = float_of_int (max 1 base_energy) in
  let bonus =
    if total_edges > 0 then
      1.0 +. (float_of_int new_edges /. float_of_int total_edges)
    else 1.0
  in
  let slots =
    Float.floor ((base *. bonus /. sqrt (float_of_int (max 1 seed_len))) +. 0.5)
  in
  max 1 (int_of_float slots)
