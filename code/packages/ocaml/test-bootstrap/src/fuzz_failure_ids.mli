(** The closed fuzz failure vocabulary (VER06 6.2). *)

val crash : string
val timeout : string
val oom : string
val leak : string
val panic : string
val all : string list

val is_registered : string -> bool
(** Exact match: ["FUZZ-CRASH"] and ["fuzz-crash\n"] are not ids. *)
