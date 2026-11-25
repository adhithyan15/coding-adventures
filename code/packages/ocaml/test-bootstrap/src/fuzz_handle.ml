(* The abstract handle a target receives (VER06r 5).  Everything is delegated to
   [Fuzz_handle_internal]; the interface hides the representation. *)

module I = Fuzz_handle_internal

type t = I.t
type token = I.token
type which = I.which = Steps | Mem | Retired

exception Sentinel = I.Sentinel

let wrap h = h
let token = I.token
let edge_id s = Int64.to_int (Prng.fnv1a64 s)
let edge = I.edge
let tick = I.tick
let reserve = I.reserve
