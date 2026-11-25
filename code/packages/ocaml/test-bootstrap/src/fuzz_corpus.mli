(** The production runner for the shared fuzz-v1 corpus (VER06r 11). It strictly
    decodes every case, evaluates it with this package's own engine functions
    (through {!Fuzz_hooks}) and returns human-readable problems; an empty list
    means "conforms". It does no file I/O. *)

type report = Corpus.report = { evaluated : int; problems : string list }

val fuzz_v1_case_count : int
val max_corpus_cases : int
val max_problems : int

val parse_corpus : string -> (Strict_json.t list, string) result
(** Exactly [{"schema_version": 1, "cases": [...]}]. *)

val evaluate :
  ?hooks:Fuzz_hooks.t -> ?expected:int option -> Strict_json.t list -> report
(** [expected] defaults to [Some 25]; [None] skips the case-count check. Every
    problem about a case starts with its escaped id. *)

val evaluate_text : ?hooks:Fuzz_hooks.t -> string -> report
