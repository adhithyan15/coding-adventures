(* The closed fuzz failure vocabulary (VER06 6.2).  Nothing else is ever printed
   in a public failure line. *)

let crash = "fuzz-crash"
let timeout = "fuzz-timeout"
let oom = "fuzz-oom"
let leak = "fuzz-leak"
let panic = "fuzz-panic"
let all = [ crash; timeout; oom; leak; panic ]

(* Case and whitespace matter: "FUZZ-CRASH" and "fuzz-crash\n" are not ids. *)
let is_registered s = List.mem s all
