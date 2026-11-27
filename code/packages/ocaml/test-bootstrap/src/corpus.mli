(** The production runner for the shared pbt-v1 corpus (VER05 section 6, VER05q
    section 7). It strictly decodes every case, evaluates it with this package's
    own public engine functions and returns human-readable problems. An empty
    list means "conforms". It does no file I/O. *)

type report = { evaluated : int; problems : string list }

val pbt_v1_case_count : int
val max_corpus_cases : int
val max_problems : int

val parse_corpus : string -> (Strict_json.t list, string) result
(** Exactly [{"schema_version": 1, "cases": [...]}]. *)

val evaluate :
  ?config:Types.config -> ?expected:int option -> Strict_json.t list -> report
(** [expected] defaults to [Some 25]; [None] skips the case-count check. *)

val evaluate_text : ?config:Types.config -> string -> report
(** [parse_corpus] then [evaluate]; a parse failure is a single problem. *)
