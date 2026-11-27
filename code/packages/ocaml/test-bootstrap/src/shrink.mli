(** Integrated shrinking (VER05 4, VER05e 4, VER05q 1 and 5).

    The public shrinkers take a property that returns [true] when it HOLDS (the
    corpus convention) and return [(minimal, steps)], [steps] counting ACCEPTED
    moves only. Internally every shrinker works with a [falsifies] function
    ([true] while the candidate STILL FAILS) that the entry points have already
    wrapped so each call is charged one attempt to the budget tree. *)

type 'a falsifies = 'a -> bool
type 'a elem_shrink = 'a -> 'a falsifies -> int -> 'a * int

val in_bounds : Bigint.t option -> Bigint.t option -> Bigint.t -> bool

val int_walk :
  Types.config ->
  Bigint.t ->
  Bigint.t ->
  Bigint.t option ->
  Bigint.t option ->
  Bigint.t falsifies ->
  int ->
  Bigint.t * int

val shrink_int_core :
  Types.config ->
  Bigint.t option ->
  Bigint.t option ->
  Bigint.t ->
  Bigint.t ->
  Bigint.t falsifies ->
  int ->
  Bigint.t * int

val shrink_sequence :
  Types.config ->
  Budget.t ->
  min_len:int ->
  'a elem_shrink option ->
  'a array falsifies ->
  int ->
  'a array ->
  'a array * int

val walk_byte : Types.config -> int elem_shrink

val shrink_int :
  ?config:Types.config ->
  ?parent:Budget.t ->
  ?max_steps:int ->
  initial:Bigint.t ->
  target:Bigint.t ->
  lo:Bigint.t ->
  hi:Bigint.t ->
  (Bigint.t -> bool) ->
  Bigint.t * int

val shrink_list :
  ?config:Types.config ->
  ?parent:Budget.t ->
  ?max_steps:int ->
  initial:Bigint.t list ->
  elem_target:Bigint.t ->
  (Bigint.t list -> bool) ->
  Bigint.t list * int

val shrink_bytes :
  ?config:Types.config ->
  ?parent:Budget.t ->
  ?max_steps:int ->
  initial:string ->
  (string -> bool) ->
  string * int

val simulate_replay :
  ?config:Types.config ->
  ?parent:Budget.t ->
  int64 ->
  (Bigint.t -> bool) ->
  Bigint.t * Bigint.t
