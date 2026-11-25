(** Crash signatures, kind normalisation and the one public failure line. *)

val crash_signature : string -> string list -> string
(** The low 32 bits of [fnv1a64 (kind ^ ":" ^ frames joined by ":")] as 8
    lowercase hex digits. Frames are not escaped (VER06 9.4 item 6). *)

val duplicate_crash : string -> string list -> string -> string list -> bool

val tap_token : string -> string
(** The first 128 BYTES of the target, every byte outside [A-Za-z0-9._/-] as
    ['_'] (VER06 9.3). *)

val format_tap_failure : int -> string -> string -> string -> string
(** [format_tap_failure n target failure_id sig]. Raises [Error.Argument_error]
    for [n] outside [1, 2^31-1], an unregistered id or a signature that is not
    eight lowercase hex digits. *)

val tap_failures : Fuzz_types.report -> string -> int -> string list

val kind_of : exn -> string
(** The constructor name only, sanitised; never raises. *)

val sanitize_kind : string -> string
