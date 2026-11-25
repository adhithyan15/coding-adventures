(** The budget of a shrink, as a TREE of scopes (VER05q section 5). *)

type t

val with_root : Types.config -> max_attempts:int -> (t -> 'a) -> 'a
(** A fresh root with the given attempt cap and the configuration's work cap. *)

val with_child : Types.config -> t option -> max_steps:int -> (t -> 'a) -> 'a
(** The scope of a public entry point that shrinks: a CHILD of the parent with a
    fresh local cap, or a fresh root for [None]. At depth 64 the call JOINS the
    parent instead (nothing is opened or closed). *)

val with_check :
  Types.config ->
  t option ->
  num_tests:int ->
  max_steps:int ->
  (t -> 'a) ->
  'a option
(** The scope of a whole check. [None], WITHOUT running the body and after
    marking the tree refused, when the nesting caps are exceeded. *)

val charge : t -> int -> bool
(** One attempt over a candidate of [n] elements (at least 1) charged to this
    node and every ancestor; [false] and NOTHING charged when any node cannot
    afford it. Callers charge BEFORE building the candidate. *)

val charge_copy : t -> int -> bool
(** Work only (no attempt). *)

val room : t -> bool
val attempts : t -> int
val work : t -> int

val is_closed : t -> bool
(** The node's own flag. *)

val is_spent : t -> bool
(** Its own flag or any ancestor's. *)

val root : t -> t
(** The outermost node of the tree. *)

val is_refused : t -> bool
val mark_refused : t -> unit
val is_nested : t -> bool
val depth : t -> int
val checks : t -> int
val config : t -> Types.config
val max_attempts : t -> int
val max_work : t -> int
val charged_falsifies : t -> ('a -> bool) -> 'a -> bool

val current : unit -> t option
(** The ambient scope of the calling domain (inherited by spawned domains). It
    is per domain, not per thread or fibre: concurrent checks in different
    threads or fibres of one domain must pass [?parent] explicitly. Closed
    (spent) scopes are skipped, so a stale ambient never poisons a later check.
*)

val resolve : t option -> t option
(** [resolve (Some p) = Some p]; [resolve None = current ()]. *)
