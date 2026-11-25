(** The deterministic, budgeted coverage-guided campaign (VER06 5 and 9.1,
    VER06r 9). *)

type exec = string -> Fuzz_types.outcome * int list
(** The outcome and the (reduced) edges of one execution. *)

type config = {
  retained_cap : int;  (** retained bytes; default 256 MiB *)
  tier : (int -> string) option;  (** labels the coverage tier *)
  abort : (unit -> bool) option;  (** polled before EVERY execution *)
  leaked : (unit -> int) option;  (** leaked workers, for the report *)
}
(** Test seams and the executor's wiring; production uses {!default_config}. *)

val default_config : config
val same_crash : Fuzz_types.outcome -> string -> bool

val minimize_abortable :
  (unit -> bool) ->
  string ->
  string ->
  (string -> Fuzz_types.outcome) ->
  int ->
  string * int * bool
(** [minimize_abortable halt dat sig probe budget] gives
    [(bytes, executions_spent, finished)]. Every candidate, accepted or
    rejected, is counted and the budget is checked before each; [halt] is asked
    before each probe. *)

val minimize :
  string ->
  string ->
  (string -> Fuzz_types.outcome) ->
  int ->
  string * int * bool

val minimize_crashing_input :
  string -> string -> int -> (string -> Fuzz_types.outcome) -> string
(** Validated: budget in [0, 100000], input at most 1 MiB. *)

val run_outcomes_with :
  config -> string list -> Fuzz_types.options -> exec -> Fuzz_types.report

val run_outcomes :
  string list -> Fuzz_types.options -> exec -> Fuzz_types.report
