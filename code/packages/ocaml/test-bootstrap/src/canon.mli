(** A stable, TOTAL text form of a value (VER05q section 7). *)

val max_depth : int
val max_nodes : int
val max_output : int
val max_bigint_bits : int
val unrenderable : string

val render : Val.t -> string
(** Total by construction: every traversal is capped and nothing user-supplied
    is called. *)

val render_with : ('a -> Val.t) -> 'a -> string
(** Applies the (untrusted) [canon] function inside the same safety net. *)

val same_val : Val.t -> Val.t -> bool
(** Kind-sensitive and bounded: at most [max_nodes] nodes, depth 64; [false]
    past either cap. *)
