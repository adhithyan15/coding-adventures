(** The 65,536-edge execution map, admission and the power schedule. *)

type t

val empty : t

val reduce_edge : int -> int
(** [e land 0xFFFF]: the floored [mod 65536], also for negative ids. *)

val record : t -> int -> t
val has_edge : t -> int -> bool

val admit : t -> int list -> bool * int * t
(** ORs a trace into the cumulative map: [(has_new, count_new, map)] over the
    REDUCED ids. *)

val covered_edges : t -> int list
val count : t -> int

val raw_bitmap : t -> string
(** The 8,192-byte map: bit [e mod 8] of byte [e div 8]. *)

val corpus_admit : int64 list -> int64 list -> bool * int
(** Novelty over the UNREDUCED ids, counting unique novel edges. *)

val calculate_energy : int -> int -> int -> int -> int
(** [calculate_energy base seed_len new_edges total_edges]; half rounds up. *)
