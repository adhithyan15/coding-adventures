(* Exception classification (VER05q section 4).

   - Sys.Break (Ctrl-C) and Out_of_memory are NEVER turned into a verdict: the
     first is the user asking the process to stop, the second means the engine
     can no longer be trusted to allocate one.
   - Stack_overflow IS recoverable: it is deterministic misbehaviour of the
     property and the stack has unwound when the handler runs.
   - Classification is by constructor name only.  Printexc.to_string is never
     called: an exception can carry a payload, and a registered printer runs
     user code. *)

let recoverable f =
  match f () with
  | v -> Ok v
  | exception (Sys.Break as e) -> raise e
  | exception (Out_of_memory as e) -> raise e
  | exception Fun.Finally_raised ((Sys.Break | Out_of_memory) as e) -> raise e
  | exception e -> Error e

(* The constructor's name, escaped and clipped; never its payload. *)
let exn_name e =
  Escape.escape_text false (Escape.clip_text 200 (Printexc.exn_slot_name e))
