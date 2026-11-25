(* Crash signatures, kind normalisation and the one public failure line (VER06
   5.2, 6 and 9.3; VER06r 9 and 10).

   Nothing in here reads an exception MESSAGE: a kind comes from the exception's
   constructor name. *)

let crash_signature kind frames =
  let h = Prng.fnv1a64 (kind ^ ":" ^ String.concat ":" frames) in
  Printf.sprintf "%08Lx" (Int64.logand h 0xFFFFFFFFL)

(* Frames are NOT escaped (VER06 9.4 item 6): one frame "x:y" and two frames "x",
   "y" hash alike.  This lane never produces a frame at all. *)
let duplicate_crash ka fa kb fb = crash_signature ka fa = crash_signature kb fb

(* The target token, exactly as VER06 9.3 and every other lane: truncated to 128
   BYTES first, then every BYTE outside [A-Za-z0-9._/-] (a control character, a
   space, '#', a newline, each byte of a multi-byte character) becomes '_'. *)
let tap_token target =
  let n = min (String.length target) Fuzz_limits.max_target_len in
  String.init n (fun i ->
      match target.[i] with
      | ('0' .. '9' | 'A' .. 'Z' | 'a' .. 'z' | '.' | '_' | '/' | '-') as c -> c
      | _ -> '_')

let is_hex8 s =
  String.length s = 8
  && String.for_all (function '0' .. '9' | 'a' .. 'f' -> true | _ -> false) s

(* The only public failure line: [not ok N - token [failure-id:hex8]].  It
   carries no input bytes, seed, message or stack trace (VER00 7.3).  The
   registered-grammar fields are CHECKED, not escaped. *)
let format_tap_failure n target failure_id sig_hash =
  if n < 1 || n > 2_147_483_647 then
    raise (Error.Argument_error "test_num must be in [1, 2147483647]");
  if not (Fuzz_failure_ids.is_registered failure_id) then
    raise
      (Error.Argument_error
         "failure id is not one of the registered fuzz failure ids");
  if not (is_hex8 sig_hash) then
    raise
      (Error.Argument_error "sig hash must be exactly 8 lowercase hex digits");
  Printf.sprintf "not ok %d - %s [%s:%s]" n (tap_token target) failure_id
    sig_hash

(* A report as public, opaque failure lines (none for a clean one): one per
   crash signature, then timeouts, ooms and leaked workers, whose hash is the
   signature of the bare kind (they have no stack to hash). *)
let tap_failures (r : Fuzz_types.report) target first_test =
  let crash_lines =
    List.mapi
      (fun i (c : Fuzz_types.crash_record) ->
        format_tap_failure (first_test + i) target Fuzz_failure_ids.crash
          c.signature)
      r.crashes
  in
  let extras =
    (if r.timeouts > 0 then [ (Fuzz_failure_ids.timeout, "timeout") ] else [])
    @ (if r.ooms > 0 then [ (Fuzz_failure_ids.oom, "oom") ] else [])
    @
    if r.leaked_workers > 0 then
      [ (Fuzz_failure_ids.timeout, "workers-leaked") ]
    else []
  in
  let base = first_test + List.length crash_lines in
  crash_lines
  @ List.mapi
      (fun i (fid, kind) ->
        format_tap_failure (base + i) target fid (crash_signature kind []))
      extras

(* Every byte outside [A-Za-z0-9._<>$/-] becomes '_', capped at 64 bytes; empty
   input is "<unreadable>". *)
let sanitize_kind s =
  if s = "" then "<unreadable>"
  else
    let n = min (String.length s) Fuzz_limits.max_kind_len in
    String.init n (fun i ->
        match s.[i] with
        | ( '0' .. '9' | 'A' .. 'Z' | 'a' .. 'z' | '.' | '_' | '<' | '>' | '$'
          | '/' | '-' ) as c ->
            c
        | _ -> '_')

(* The crash kind of a caught exception: its constructor name and nothing else
   (not its message, not any argument; Printexc.to_string is never called because
   it runs registered printers, i.e. user code). *)
let kind_of e =
  match Printexc.exn_slot_name e with
  | s -> sanitize_kind s
  | exception _ -> "<unreadable>"
