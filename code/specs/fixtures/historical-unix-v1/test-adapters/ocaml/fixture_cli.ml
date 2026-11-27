(* Test-only JSON bridge from the shared replay helper to the pure OCaml API. *)

let quote text =
  let buffer = Buffer.create (String.length text + 2) in
  Buffer.add_char buffer '"';
  String.iter
    (function
      | '"' -> Buffer.add_string buffer "\\\""
      | '\\' -> Buffer.add_string buffer "\\\\"
      | '\n' -> Buffer.add_string buffer "\\n"
      | '\r' -> Buffer.add_string buffer "\\r"
      | '\t' -> Buffer.add_string buffer "\\t"
      | char when Char.code char < 0x20 -> Buffer.add_string buffer (Printf.sprintf "\\u%04x" (Char.code char))
      | char -> Buffer.add_char buffer char)
    text;
  Buffer.add_char buffer '"';
  Buffer.contents buffer

let string_array values =
  "[" ^ String.concat "," (List.map quote values) ^ "]"

let event_json event =
  let string key value = Printf.sprintf "%s:%s" (quote key) (quote value) in
  let integer key value = Printf.sprintf "%s:%d" (quote key) value in
  let error key value = string key (Unix_history.string_of_error value) in
  let fields values = "{" ^ String.concat "," values ^ "}" in
  match event with
  | Unix_history.Opened { path; inode; fd } ->
      fields [ string "call" "open"; string "path" path; integer "inode" inode; integer "fd" fd; integer "offset" 0 ]
  | Unix_history.Open_failed { path; error = code } ->
      fields [ string "call" "open"; string "path" path; error "error" code ]
  | Unix_history.Read { fd; inode; offset; bytes; next_offset } ->
      fields
        [ string "call" "read"; integer "fd" fd; integer "inode" inode;
          integer "offset" offset; integer "bytes" bytes; integer "nextOffset" next_offset ]
  | Unix_history.Read_failed { fd; error = code } ->
      fields [ string "call" "read"; integer "fd" fd; error "error" code ]
  | Unix_history.Closed { fd } -> fields [ string "call" "close"; integer "fd" fd ]
  | Unix_history.Close_failed { fd; error = code } ->
      fields [ string "call" "close"; integer "fd" fd; error "error" code ]
  | Unix_history.Listed { path; inode; names } ->
      fields [ string "call" "list"; string "path" path; integer "inode" inode;
        Printf.sprintf "%s:%s" (quote "names") (string_array names) ]
  | Unix_history.List_failed { path; error = code } ->
      fields [ string "call" "list"; string "path" path; error "error" code ]

let main = function
  | [ command ] ->
      let trace = Unix_history.Kernel.run_command (Unix_history.Kernel.create_demo ()) command in
      let events = String.concat "," (List.map event_json trace.events) in
      Printf.printf "{\"output\":%s,\"events\":[%s]}\n" (quote trace.output) events
  | _ -> prerr_endline "expected exactly one shell command"; exit 2

let () = main (List.tl (Array.to_list Sys.argv))
