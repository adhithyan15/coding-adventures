(** The generator algebra (VER05 3, VER05e 3, VER05q 6).

    DRAW ORDER IS THE COMPATIBILITY CONTRACT between language lanes: the pbt-v1
    vectors pin exact outputs per seed. A generator carries its own [canon], the
    explicit description of a value that the engine renders and measures, so the
    engine never inspects a user's value itself. *)

type 'a shrinker = Budget.t -> 'a -> 'a Shrink.falsifies -> int -> 'a * int
(** A shrinker JOINS the budget node it is handed (it never opens a scope). *)

type 'a t = {
  sample : Budget.t option -> Prng.t -> 'a;
  shrink : 'a shrinker option;
  canon : 'a -> Val.t;
}

val sample : ?ctx:Budget.t -> 'a t -> Prng.t -> 'a

val shrink :
  ?config:Types.config ->
  ?parent:Budget.t ->
  'a t ->
  'a ->
  'a Shrink.falsifies ->
  int ->
  'a * int
(** Public entry: opens a child scope and charges every evaluation. *)

val shrink_nested :
  Budget.t -> 'a t -> 'a -> 'a Shrink.falsifies -> int -> 'a * int
(** For combinators and custom shrinkers: JOINS the enclosing budget. *)

val custom :
  canon:('a -> Val.t) ->
  ?shrink:'a shrinker ->
  (Budget.t option -> Prng.t -> 'a) ->
  'a t

val constant : canon:('a -> Val.t) -> 'a -> 'a t
val bool : bool t
val u8 : Bigint.t t
val u64 : Bigint.t t
val i128 : Bigint.t t

val integer : Bigint.t -> Bigint.t -> Bigint.t t
(** Uniform over the inclusive range; a span wider than 16,384 bits is an
    invalid generator. *)

val int : int -> int -> int t
val bytes : int -> int -> string t
val map : canon:('b -> Val.t) -> ('a -> 'b) -> 'a t -> 'b t
val filter : ?max_retries:int -> ('a -> bool) -> 'a t -> 'a t
val flat_map : canon:('b -> Val.t) -> ('a -> 'b t) -> 'a t -> 'b t
val one_of : canon:('a -> Val.t) -> 'a t list -> 'a t
val frequency : canon:('a -> Val.t) -> (int * 'a t) list -> 'a t
val list : ?min_len:int -> ?max_len:int -> 'a t -> 'a list t
val tuple2 : 'a t -> 'b t -> ('a * 'b) t
val tuple3 : 'a t -> 'b t -> 'c t -> ('a * 'b * 'c) t
val gen_primitive : Types.config -> int64 -> string -> int -> Val.t list

val gen_bounded :
  Types.config -> int64 -> Bigint.t -> Bigint.t -> int -> Val.t list

val gen_bytes : Types.config -> int64 -> int -> int -> int -> Val.t list
