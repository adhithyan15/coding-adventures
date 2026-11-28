(* Native clock and host metadata are isolated behind a tiny checked C shim. *)
external now_ns : unit -> int64 = "caml_mb_monotonic_ns"
external os_name : unit -> string = "caml_mb_os_name"
external architecture : unit -> string = "caml_mb_architecture"
