open Asttypes
open Parsetree
module String_map = Map.Make (String)
module String_set = Set.Make (String)

module Capability = struct
  type t = { category : string; action : string; target : string }

  let make ~category ~action ~target = { category; action; target }
  let to_string value = value.category ^ ":" ^ value.action ^ ":" ^ value.target

  let compare left right =
    Stdlib.compare
      (left.category, left.action, left.target)
      (right.category, right.action, right.target)
end

type source_kind = Implementation | Interface

type detection = {
  file : string;
  line : int;
  column : int;
  capability : Capability.t;
  evidence : string;
}

type banned_construct = {
  file : string;
  line : int;
  column : int;
  construct : string;
  evidence : string;
  required_capability : Capability.t option;
  exemptible : bool;
}

type exception_declaration = {
  construct : string;
  language : string;
  justification : string;
}

type manifest = {
  package : string;
  capabilities : Capability.t list;
  exceptions : exception_declaration list;
}

type violation = {
  code : string;
  file : string;
  line : int;
  column : int;
  message : string;
}

type analysis_result = {
  dir : string;
  detected : detection list;
  banned : banned_construct list;
  declared : Capability.t list;
  violations : violation list;
}

let ( let* ) value next =
  match value with Ok result -> next result | Error _ as error -> error

let ( >>= ) value next =
  let* result = value in
  next result

let capability category action = Capability.make ~category ~action ~target:"*"

let valid_capability_pair category action =
  match (category, action) with
  | "fs", ("read" | "write" | "create" | "delete" | "list")
  | "net", ("connect" | "listen" | "dns")
  | "proc", ("exec" | "fork" | "signal")
  | "env", ("read" | "write")
  | "ffi", ("call" | "load")
  | "time", ("read" | "sleep")
  | "stdin", "read"
  | "stdout", "write" ->
      true
  | _ -> false

let duplicate values =
  let rec loop seen = function
    | [] -> None
    | value :: rest ->
        if String_set.mem value seen then Some value
        else loop (String_set.add value seen) rest
  in
  loop String_set.empty values

let validate_object_fields ~context ~allowed fields =
  let names = List.map fst fields in
  match duplicate names with
  | Some name -> Error (Printf.sprintf "%s has duplicate field %S" context name)
  | None -> (
      match List.find_opt (fun name -> not (List.mem name allowed)) names with
      | Some name ->
          Error (Printf.sprintf "%s has unknown field %S" context name)
      | None -> Ok ())

let required_field ~context name fields =
  match List.assoc_opt name fields with
  | Some value -> Ok value
  | None -> Error (Printf.sprintf "%s is missing field %S" context name)

let string_value ~context = function
  | `String value -> Ok value
  | _ -> Error (context ^ " must be a string")

let nonempty_string ~context value =
  let* value = string_value ~context value in
  if String.trim value = "" then Error (context ^ " must not be empty")
  else Ok value

let justification ~context value =
  let* value = string_value ~context value in
  if String.length (String.trim value) < 10 then
    Error (context ^ " must contain at least 10 characters")
  else Ok value

let object_value ~context = function
  | `Assoc fields -> Ok fields
  | _ -> Error (context ^ " must be an object")

let list_value ~context = function
  | `List values -> Ok values
  | _ -> Error (context ^ " must be an array")

let valid_package_name package =
  let prefix = "ocaml/" in
  let prefix_length = String.length prefix in
  String.length package > prefix_length
  && String.starts_with ~prefix package
  && (match package.[prefix_length] with
     | 'a' .. 'z' | '0' .. '9' -> true
     | _ -> false)
  && String.for_all
       (function 'a' .. 'z' | '0' .. '9' | '_' | '-' -> true | _ -> false)
       (String.sub package (prefix_length + 1)
          (String.length package - prefix_length - 1))

let parse_capability index value =
  let context = Printf.sprintf "capability %d" index in
  let* fields = object_value ~context value in
  let* () =
    validate_object_fields ~context
      ~allowed:[ "category"; "action"; "target"; "justification" ]
      fields
  in
  let* category =
    required_field ~context "category" fields
    >>= string_value ~context:(context ^ " category")
  in
  let* action =
    required_field ~context "action" fields
    >>= string_value ~context:(context ^ " action")
  in
  let* target =
    required_field ~context "target" fields
    >>= nonempty_string ~context:(context ^ " target")
  in
  let* _ =
    required_field ~context "justification" fields
    >>= justification ~context:(context ^ " justification")
  in
  if not (valid_capability_pair category action) then
    Error
      (Printf.sprintf "%s has invalid category/action pair %s:%s" context
         category action)
  else Ok (Capability.make ~category ~action ~target)

let parse_exception index value =
  let context = Printf.sprintf "banned construct exception %d" index in
  let* fields = object_value ~context value in
  let* () =
    validate_object_fields ~context
      ~allowed:[ "construct"; "language"; "justification" ]
      fields
  in
  let* construct =
    required_field ~context "construct" fields
    >>= nonempty_string ~context:(context ^ " construct")
  in
  let* language =
    required_field ~context "language" fields
    >>= string_value ~context:(context ^ " language")
  in
  let* justification =
    required_field ~context "justification" fields
    >>= justification ~context:(context ^ " justification")
  in
  if language <> "ocaml" then Error (context ^ " language must be ocaml")
  else Ok { construct; language; justification }

let map_indexed parser values =
  let rec loop index acc = function
    | [] -> Ok (List.rev acc)
    | value :: rest ->
        let* parsed = parser index value in
        loop (index + 1) (parsed :: acc) rest
  in
  loop 0 [] values

let parse_manifest text =
  try
    let* fields =
      Yojson.Safe.from_string text |> object_value ~context:"manifest"
    in
    let* () =
      validate_object_fields ~context:"manifest"
        ~allowed:
          [
            "$schema";
            "version";
            "package";
            "capabilities";
            "justification";
            "banned_construct_exceptions";
          ]
        fields
    in
    let* () =
      match List.assoc_opt "$schema" fields with
      | None -> Ok ()
      | Some value ->
          let* _ = string_value ~context:"manifest $schema" value in
          Ok ()
    in
    let* version = required_field ~context:"manifest" "version" fields in
    let* () =
      match version with
      | `Int 1 -> Ok ()
      | _ -> Error "manifest version must be integer 1"
    in
    let* package =
      required_field ~context:"manifest" "package" fields
      >>= string_value ~context:"manifest package"
    in
    let* () =
      if valid_package_name package then Ok ()
      else Error "manifest package must match ocaml/[a-z0-9_-]+"
    in
    let* capability_values =
      required_field ~context:"manifest" "capabilities" fields
      >>= list_value ~context:"manifest capabilities"
    in
    let* capabilities = map_indexed parse_capability capability_values in
    let* _ =
      required_field ~context:"manifest" "justification" fields
      >>= justification ~context:"manifest justification"
    in
    let* exception_values =
      match List.assoc_opt "banned_construct_exceptions" fields with
      | None -> Ok []
      | Some value ->
          list_value ~context:"manifest banned_construct_exceptions" value
    in
    let* exceptions = map_indexed parse_exception exception_values in
    let capability_keys = List.map Capability.to_string capabilities in
    let* () =
      match duplicate capability_keys with
      | Some value ->
          Error (Printf.sprintf "manifest duplicates capability %s" value)
      | None -> Ok ()
    in
    let exception_keys = List.map (fun value -> value.construct) exceptions in
    let* () =
      match duplicate exception_keys with
      | Some value ->
          Error (Printf.sprintf "manifest duplicates exception %s" value)
      | None -> Ok ()
    in
    Ok { package; capabilities; exceptions }
  with Yojson.Json_error message -> Error ("manifest JSON error: " ^ message)

type rule = {
  module_name : string;
  function_name : string;
  capabilities : Capability.t list;
}

let rule module_name function_name capabilities =
  { module_name; function_name; capabilities }

let rules =
  [
    rule "Stdlib" "open_in" [ capability "fs" "read" ];
    rule "Stdlib" "open_in_bin" [ capability "fs" "read" ];
    rule "In_channel" "open_text" [ capability "fs" "read" ];
    rule "In_channel" "open_bin" [ capability "fs" "read" ];
    rule "In_channel" "with_open_text" [ capability "fs" "read" ];
    rule "In_channel" "with_open_bin" [ capability "fs" "read" ];
    rule "Stdlib" "open_out" [ capability "fs" "write" ];
    rule "Stdlib" "open_out_bin" [ capability "fs" "write" ];
    rule "Stdlib" "open_out_gen" [ capability "fs" "write" ];
    rule "Out_channel" "open_text" [ capability "fs" "write" ];
    rule "Out_channel" "open_bin" [ capability "fs" "write" ];
    rule "Out_channel" "with_open_text" [ capability "fs" "write" ];
    rule "Out_channel" "with_open_bin" [ capability "fs" "write" ];
    rule "Sys" "file_exists" [ capability "fs" "list" ];
    rule "Sys" "is_directory" [ capability "fs" "list" ];
    rule "Sys" "read_directory" [ capability "fs" "list" ];
    rule "Sys" "remove" [ capability "fs" "delete" ];
    rule "Sys" "rename" [ capability "fs" "write" ];
    rule "Sys" "command" [ capability "proc" "exec" ];
    rule "Sys" "getenv" [ capability "env" "read" ];
    rule "Sys" "getenv_opt" [ capability "env" "read" ];
    rule "Sys" "time" [ capability "time" "read" ];
    rule "Unix" "openfile" [ capability "fs" "read"; capability "fs" "write" ];
    rule "Unix" "stat" [ capability "fs" "list" ];
    rule "Unix" "lstat" [ capability "fs" "list" ];
    rule "Unix" "realpath" [ capability "fs" "list" ];
    rule "Unix" "opendir" [ capability "fs" "list" ];
    rule "Unix" "readdir" [ capability "fs" "list" ];
    rule "Unix" "unlink" [ capability "fs" "delete" ];
    rule "Unix" "rmdir" [ capability "fs" "delete" ];
    rule "Unix" "rename" [ capability "fs" "write" ];
    rule "Unix" "mkdir" [ capability "fs" "create" ];
    rule "Unix" "chmod" [ capability "fs" "write" ];
    rule "Unix" "chown" [ capability "fs" "write" ];
    rule "Unix" "symlink" [ capability "fs" "create" ];
    rule "Unix" "socket" [ capability "net" "connect" ];
    rule "Unix" "socketpair" [ capability "net" "connect" ];
    rule "Unix" "connect" [ capability "net" "connect" ];
    rule "Unix" "shutdown" [ capability "net" "connect" ];
    rule "Unix" "send" [ capability "net" "connect" ];
    rule "Unix" "send_substring" [ capability "net" "connect" ];
    rule "Unix" "sendto" [ capability "net" "connect" ];
    rule "Unix" "recv" [ capability "net" "connect" ];
    rule "Unix" "recvfrom" [ capability "net" "connect" ];
    rule "Unix" "bind" [ capability "net" "listen" ];
    rule "Unix" "listen" [ capability "net" "listen" ];
    rule "Unix" "accept" [ capability "net" "listen" ];
    rule "Unix" "accept_non_intr" [ capability "net" "listen" ];
    rule "Unix" "getaddrinfo" [ capability "net" "dns" ];
    rule "Unix" "getnameinfo" [ capability "net" "dns" ];
    rule "Unix" "gethostbyname" [ capability "net" "dns" ];
    rule "Unix" "gethostbyaddr" [ capability "net" "dns" ];
    rule "Unix" "system" [ capability "proc" "exec" ];
    rule "Unix" "fork" [ capability "proc" "fork" ];
    rule "Unix" "kill" [ capability "proc" "signal" ];
    rule "Unix" "getenv" [ capability "env" "read" ];
    rule "Unix" "getenv_opt" [ capability "env" "read" ];
    rule "Unix" "putenv" [ capability "env" "write" ];
    rule "Unix" "time" [ capability "time" "read" ];
    rule "Unix" "gettimeofday" [ capability "time" "read" ];
    rule "Unix" "times" [ capability "time" "read" ];
    rule "Unix" "clock_gettime" [ capability "time" "read" ];
    rule "Unix" "sleep" [ capability "time" "sleep" ];
    rule "Unix" "sleepf" [ capability "time" "sleep" ];
    rule "Stdlib" "read_line" [ capability "stdin" "read" ];
    rule "Stdlib" "read_int" [ capability "stdin" "read" ];
    rule "Stdlib" "read_float" [ capability "stdin" "read" ];
    rule "Stdlib" "print_char" [ capability "stdout" "write" ];
    rule "Stdlib" "print_string" [ capability "stdout" "write" ];
    rule "Stdlib" "print_bytes" [ capability "stdout" "write" ];
    rule "Stdlib" "print_int" [ capability "stdout" "write" ];
    rule "Stdlib" "print_float" [ capability "stdout" "write" ];
    rule "Stdlib" "print_endline" [ capability "stdout" "write" ];
    rule "Stdlib" "print_newline" [ capability "stdout" "write" ];
    rule "Stdlib" "prerr_char" [ capability "stdout" "write" ];
    rule "Stdlib" "prerr_string" [ capability "stdout" "write" ];
    rule "Stdlib" "prerr_endline" [ capability "stdout" "write" ];
    rule "Stdlib" "prerr_newline" [ capability "stdout" "write" ];
    rule "Printf" "printf" [ capability "stdout" "write" ];
    rule "Printf" "eprintf" [ capability "stdout" "write" ];
    rule "Format" "printf" [ capability "stdout" "write" ];
    rule "Format" "eprintf" [ capability "stdout" "write" ];
    rule "Dynlink" "loadfile" [ capability "ffi" "load" ];
    rule "Dynlink" "loadfile_private" [ capability "ffi" "load" ];
  ]

let executable_prefixes = [ "exec"; "create_process"; "open_process" ]

let starts_with_any prefixes value =
  List.exists (fun prefix -> String.starts_with ~prefix value) prefixes

let extra_rule module_name function_name =
  if module_name = "Unix" && starts_with_any executable_prefixes function_name
  then [ capability "proc" "exec" ]
  else if
    module_name = "Unix" && String.starts_with ~prefix:"send" function_name
  then [ capability "net" "connect" ]
  else if
    module_name = "Unix" && String.starts_with ~prefix:"recv" function_name
  then [ capability "net" "connect" ]
  else []

let capabilities_for module_name function_name =
  let table =
    rules
    |> List.filter_map (fun entry ->
           if
             entry.module_name = module_name
             && entry.function_name = function_name
           then Some entry.capabilities
           else None)
    |> List.flatten
  in
  List.sort_uniq Capability.compare
    (table @ extra_rule module_name function_name)

type environment = {
  aliases : string list String_map.t;
  bound_modules : String_set.t;
  opens : string list list;
  bound_values : String_set.t;
}

let empty_environment =
  {
    aliases = String_map.empty;
    bound_modules = String_set.empty;
    opens = [];
    bound_values = String_set.empty;
  }

let canonical_modules =
  String_set.of_list
    [
      "Stdlib";
      "Sys";
      "Unix";
      "Dynlink";
      "Marshal";
      "Obj";
      "Printf";
      "Format";
      "In_channel";
      "Out_channel";
    ]

let rec longident_segments = function
  | Longident.Lident name -> Some [ name ]
  | Longident.Ldot (prefix, name) ->
      Option.map
        (fun segments -> segments @ [ name ])
        (longident_segments prefix)
  | Longident.Lapply _ -> None

let normalize_stdlib_prefix = function
  | "Stdlib" :: rest when rest <> [] -> rest
  | value -> value

let resolve_module_segments environment segments =
  let segments = normalize_stdlib_prefix segments in
  match segments with
  | [] -> None
  | root :: rest -> (
      match String_map.find_opt root environment.aliases with
      | Some alias -> Some (alias @ rest)
      | None ->
          if String_set.mem root environment.bound_modules then None
          else if String_set.mem root canonical_modules then Some segments
          else None)

let rec resolve_module_expression environment module_expression =
  match module_expression.pmod_desc with
  | Pmod_ident identifier ->
      Option.bind
        (longident_segments identifier.txt)
        (resolve_module_segments environment)
  | Pmod_constraint (body, _) -> resolve_module_expression environment body
  | _ -> None

let rec module_expression_mentions_sensitive environment module_expression =
  match module_expression.pmod_desc with
  | Pmod_ident identifier -> (
      match longident_segments identifier.txt with
      | Some segments ->
          Option.is_some (resolve_module_segments environment segments)
      | None -> false)
  | Pmod_apply (left, right) ->
      module_expression_mentions_sensitive environment left
      || module_expression_mentions_sensitive environment right
  | Pmod_apply_unit body | Pmod_constraint (body, _) ->
      module_expression_mentions_sensitive environment body
  | Pmod_functor (_, body) ->
      module_expression_mentions_sensitive environment body
  | Pmod_structure structure ->
      List.exists
        (fun item ->
          match item.pstr_desc with
          | Pstr_include declaration ->
              module_expression_mentions_sensitive environment
                declaration.pincl_mod
          | Pstr_open declaration ->
              module_expression_mentions_sensitive environment
                declaration.popen_expr
          | Pstr_module binding ->
              module_expression_mentions_sensitive environment binding.pmb_expr
          | Pstr_recmodule bindings ->
              List.exists
                (fun binding ->
                  module_expression_mentions_sensitive environment
                    binding.pmb_expr)
                bindings
          | _ -> false)
        structure
  | Pmod_unpack _ | Pmod_extension _ -> false

let pattern_names pattern =
  let names = ref String_set.empty in
  let iterator =
    {
      Ast_iterator.default_iterator with
      pat =
        (fun self current ->
          (match current.ppat_desc with
          | Ppat_var name | Ppat_alias (_, name) ->
              names := String_set.add name.txt !names
          | _ -> ());
          Ast_iterator.default_iterator.pat self current);
    }
  in
  iterator.pat iterator pattern;
  !names

let bind_patterns environment patterns =
  let names =
    patterns
    |> List.fold_left
         (fun result pattern -> String_set.union result (pattern_names pattern))
         String_set.empty
  in
  {
    environment with
    bound_values = String_set.union environment.bound_values names;
  }

let bind_module environment name alias =
  let aliases = String_map.remove name environment.aliases in
  let aliases =
    match alias with
    | Some path -> String_map.add name path aliases
    | None -> aliases
  in
  {
    environment with
    aliases;
    bound_modules = String_set.add name environment.bound_modules;
  }

let open_module environment path =
  { environment with opens = path :: environment.opens }

let last_two segments =
  match List.rev segments with
  | function_name :: module_name :: _ -> Some (module_name, function_name)
  | _ -> None

let standard_channel_function module_name function_name =
  let input_functions =
    [
      "input"; "input_char"; "input_line"; "really_input"; "really_input_string";
    ]
  in
  let output_functions =
    [ "output"; "output_char"; "output_string"; "output_bytes"; "flush" ]
  in
  (module_name = "Stdlib" || module_name = "In_channel")
  && List.mem function_name input_functions
  || (module_name = "Stdlib" || module_name = "Out_channel")
     && List.mem function_name output_functions

let resolves_rule module_name function_name =
  capabilities_for module_name function_name <> []
  || module_name = "Obj"
  || module_name = "Marshal"
     && String.starts_with ~prefix:"from_" function_name
  || standard_channel_function module_name function_name

let resolve_call environment identifier =
  match longident_segments identifier with
  | None -> None
  | Some [ function_name ] ->
      if String_set.mem function_name environment.bound_values then None
      else
        let candidates = environment.opens @ [ [ "Stdlib" ] ] in
        candidates
        |> List.find_map (fun path ->
               match List.rev path with
               | module_name :: _ when resolves_rule module_name function_name
                 ->
                   Some (module_name, function_name)
               | _ -> None)
  | Some segments ->
      let reversed = List.rev segments in
      let function_name = List.hd reversed in
      let module_segments = List.rev (List.tl reversed) in
      Option.bind (resolve_module_segments environment module_segments)
        (fun path ->
          match List.rev path with
          | module_name :: _ -> Some (module_name, function_name)
          | [] -> None)

let source_position location =
  let position = location.Location.loc_start in
  (position.pos_lnum, position.pos_cnum - position.pos_bol)

type analysis_context = {
  filename : string;
  detections : detection list ref;
  banned : banned_construct list ref;
  errors : string list ref;
}

let add_detection context location capability evidence =
  let line, column = source_position location in
  context.detections :=
    { file = context.filename; line; column; capability; evidence }
    :: !(context.detections)

let add_banned context location ?required_capability ~exemptible construct
    evidence =
  let line, column = source_position location in
  context.banned :=
    {
      file = context.filename;
      line;
      column;
      construct;
      evidence;
      required_capability;
      exemptible;
    }
    :: !(context.banned)

let add_error context location message =
  let line, column = source_position location in
  context.errors :=
    Printf.sprintf "%s:%d:%d: %s" context.filename line column message
    :: !(context.errors)

let evidence module_name function_name =
  if module_name = "Stdlib" then function_name ^ " call"
  else module_name ^ "." ^ function_name ^ " call"

let is_channel environment expression names =
  match expression.pexp_desc with
  | Pexp_ident identifier -> (
      match longident_segments identifier.txt with
      | Some [ name ] ->
          (not (String_set.mem name environment.bound_values))
          && List.mem name names
      | Some segments -> (
          match last_two (normalize_stdlib_prefix segments) with
          | Some (("In_channel" | "Out_channel"), name) -> List.mem name names
          | _ -> false)
      | None -> false)
  | _ -> false

let detect_standard_channel context environment location module_name
    function_name arguments =
  let first_argument =
    match arguments with (_, value) :: _ -> Some value | [] -> None
  in
  let input_functions =
    [
      "input"; "input_char"; "input_line"; "really_input"; "really_input_string";
    ]
  in
  let output_functions =
    [ "output"; "output_char"; "output_string"; "output_bytes"; "flush" ]
  in
  match first_argument with
  | Some argument
    when (module_name = "Stdlib" || module_name = "In_channel")
         && List.mem function_name input_functions
         && is_channel environment argument [ "stdin" ] ->
      add_detection context location
        (capability "stdin" "read")
        (evidence module_name function_name)
  | Some argument
    when (module_name = "Stdlib" || module_name = "Out_channel")
         && List.mem function_name output_functions
         && is_channel environment argument [ "stdout"; "stderr" ] ->
      add_detection context location
        (capability "stdout" "write")
        (evidence module_name function_name)
  | _ -> ()

let detect_resolved context environment location module_name function_name
    arguments =
  capabilities_for module_name function_name
  |> List.iter (fun detected ->
         add_detection context location detected
           (evidence module_name function_name));
  detect_standard_channel context environment location module_name function_name
    arguments;
  if arguments = [] && standard_channel_function module_name function_name then
    add_error context location
      (Printf.sprintf
         "first-class use of %s.%s is unsupported by capability analysis"
         module_name function_name);
  if module_name = "Obj" then
    let construct = "Obj." ^ function_name in
    add_banned context location ~exemptible:false construct
      (construct ^ " bypasses OCaml type safety")
  else if
    module_name = "Marshal"
    && List.mem function_name [ "from_channel"; "from_string"; "from_bytes" ]
  then
    let construct = "Marshal." ^ function_name in
    add_banned context location ~exemptible:false construct
      (construct ^ " reconstructs unchecked runtime values")
  else if
    module_name = "Dynlink"
    && List.mem function_name [ "loadfile"; "loadfile_private" ]
  then
    let construct = "Dynlink." ^ function_name in
    add_banned context location ~required_capability:(capability "ffi" "load")
      ~exemptible:true construct
      (construct ^ " loads native or bytecode modules at runtime")

let detect_call context environment expression function_expression arguments =
  match function_expression.pexp_desc with
  | Pexp_ident identifier -> (
      match resolve_call environment identifier.txt with
      | None -> ()
      | Some (module_name, function_name) ->
          detect_resolved context environment expression.pexp_loc module_name
            function_name arguments)
  | Pexp_extension _ ->
      add_error context expression.pexp_loc
        "extension node in call position is unsupported by capability analysis"
  | _ -> ()

let detect_reference context environment expression identifier =
  match resolve_call environment identifier with
  | None -> ()
  | Some (module_name, function_name) ->
      detect_resolved context environment expression.pexp_loc module_name
        function_name []

let add_external context location =
  let required_capability = capability "ffi" "call" in
  add_detection context location required_capability "external declaration";
  add_banned context location ~required_capability ~exemptible:true "external"
    "external declaration crosses an unchecked native boundary"

let detect_marshal_closures context environment expression identifier =
  match longident_segments identifier with
  | None -> ()
  | Some segments ->
      let segments = normalize_stdlib_prefix segments in
      let is_marshal =
        match segments with
        | [ "Closures" ] ->
            List.exists
              (fun path -> List.rev path |> List.hd = "Marshal")
              environment.opens
        | _ -> (
            match last_two segments with
            | Some (module_name, "Closures") -> (
                match resolve_module_segments environment [ module_name ] with
                | Some canonical -> List.rev canonical |> List.hd = "Marshal"
                | None -> false)
            | _ -> false)
      in
      if is_marshal then
        add_banned context expression.pexp_loc ~exemptible:false
          "Marshal.Closures" "Marshal.Closures serializes executable closures"

let rec source_iterator context environment =
  {
    Ast_iterator.default_iterator with
    expr = (fun _ expression -> walk_expression context environment expression);
    module_expr =
      (fun _ expression ->
        walk_module_expression context environment expression);
    structure =
      (fun _ structure -> ignore (walk_structure context environment structure));
    signature =
      (fun _ signature -> walk_signature context environment signature);
    value_description =
      (fun self description ->
        if description.pval_prim <> [] then
          add_external context description.pval_loc;
        Ast_iterator.default_iterator.value_description self description);
  }

and walk_expression context environment expression =
  (match expression.pexp_desc with
  | Pexp_apply (function_expression, arguments) ->
      detect_call context environment expression function_expression arguments
  | Pexp_ident identifier ->
      detect_reference context environment expression identifier.txt
  | Pexp_extension _ ->
      add_error context expression.pexp_loc
        "extension expression is unsupported by capability analysis"
  | Pexp_construct (identifier, _) ->
      detect_marshal_closures context environment expression identifier.txt
  | _ -> ());
  match expression.pexp_desc with
  | Pexp_let (recursive, bindings, body) ->
      let patterns = List.map (fun binding -> binding.pvb_pat) bindings in
      let recursive_environment =
        match recursive with
        | Recursive -> bind_patterns environment patterns
        | Nonrecursive -> environment
      in
      List.iter
        (fun binding ->
          walk_expression context recursive_environment binding.pvb_expr)
        bindings;
      walk_expression context (bind_patterns environment patterns) body
  | Pexp_function (parameters, _, body) -> (
      let function_environment =
        List.fold_left
          (fun current parameter ->
            match parameter.pparam_desc with
            | Pparam_val (_, default, pattern) ->
                Option.iter (walk_expression context current) default;
                bind_patterns current [ pattern ]
            | Pparam_newtype _ -> current)
          environment parameters
      in
      match body with
      | Pfunction_body body -> walk_expression context function_environment body
      | Pfunction_cases (cases, _, _) ->
          walk_cases context function_environment cases)
  | Pexp_match (scrutinee, cases) | Pexp_try (scrutinee, cases) ->
      walk_expression context environment scrutinee;
      walk_cases context environment cases
  | Pexp_for (pattern, first, last, _, body) ->
      walk_expression context environment first;
      walk_expression context environment last;
      walk_expression context (bind_patterns environment [ pattern ]) body
  | Pexp_letmodule (name, module_expression, body) ->
      walk_module_expression context environment module_expression;
      let body_environment =
        match name.txt with
        | None -> environment
        | Some name ->
            let resolved =
              resolve_module_expression environment module_expression
            in
            if
              Option.is_none resolved
              && module_expression_mentions_sensitive environment
                   module_expression
            then
              add_error context module_expression.pmod_loc
                "unresolved sensitive local module is unsupported by \
                 capability analysis";
            bind_module environment name resolved
      in
      walk_expression context body_environment body
  | Pexp_open (declaration, body) ->
      walk_module_expression context environment declaration.popen_expr;
      let body_environment =
        match resolve_module_expression environment declaration.popen_expr with
        | Some path -> open_module environment path
        | None ->
            if
              module_expression_mentions_sensitive environment
                declaration.popen_expr
            then
              add_error context declaration.popen_loc
                "unresolved sensitive local open is unsupported by capability \
                 analysis";
            environment
      in
      walk_expression context body_environment body
  | Pexp_apply (function_expression, arguments) ->
      (match function_expression.pexp_desc with
      | Pexp_ident _ -> ()
      | _ -> walk_expression context environment function_expression);
      List.iter
        (fun (_, argument) -> walk_expression context environment argument)
        arguments
  | Pexp_ident _ -> ()
  | _ ->
      Ast_iterator.default_iterator.expr
        (source_iterator context environment)
        expression

and walk_cases context environment cases =
  List.iter
    (fun case ->
      let case_environment = bind_patterns environment [ case.pc_lhs ] in
      Option.iter (walk_expression context case_environment) case.pc_guard;
      walk_expression context case_environment case.pc_rhs)
    cases

and walk_module_expression context environment expression =
  match expression.pmod_desc with
  | Pmod_ident _ -> ()
  | Pmod_structure structure ->
      ignore (walk_structure context environment structure)
  | Pmod_functor (_, body) -> walk_module_expression context environment body
  | Pmod_apply (left, right) ->
      walk_module_expression context environment left;
      walk_module_expression context environment right
  | Pmod_apply_unit body -> walk_module_expression context environment body
  | Pmod_constraint (body, _) -> walk_module_expression context environment body
  | Pmod_unpack body ->
      walk_expression context environment body;
      add_error context expression.pmod_loc
        "first-class module unpacking is unsupported by capability analysis"
  | Pmod_extension _ ->
      add_error context expression.pmod_loc
        "module extension is unsupported by capability analysis"

and walk_structure context environment structure =
  List.fold_left
    (fun current item ->
      match item.pstr_desc with
      | Pstr_eval (expression, _) ->
          walk_expression context current expression;
          current
      | Pstr_value (recursive, bindings) ->
          let patterns = List.map (fun binding -> binding.pvb_pat) bindings in
          let recursive_environment =
            match recursive with
            | Recursive -> bind_patterns current patterns
            | Nonrecursive -> current
          in
          List.iter
            (fun binding ->
              walk_expression context recursive_environment binding.pvb_expr)
            bindings;
          bind_patterns current patterns
      | Pstr_primitive description ->
          add_external context description.pval_loc;
          {
            current with
            bound_values =
              String_set.add description.pval_name.txt current.bound_values;
          }
      | Pstr_module binding -> (
          walk_module_expression context current binding.pmb_expr;
          match binding.pmb_name.txt with
          | None -> current
          | Some name -> (
              match resolve_module_expression current binding.pmb_expr with
              | Some alias -> bind_module current name (Some alias)
              | None ->
                  if
                    module_expression_mentions_sensitive current
                      binding.pmb_expr
                  then
                    add_error context binding.pmb_loc
                      "unresolved sensitive module alias is unsupported by \
                       capability analysis";
                  bind_module current name None))
      | Pstr_recmodule bindings ->
          List.iter
            (fun binding ->
              walk_module_expression context current binding.pmb_expr;
              if module_expression_mentions_sensitive current binding.pmb_expr
              then
                add_error context binding.pmb_loc
                  "unresolved sensitive recursive module is unsupported by \
                   capability analysis")
            bindings;
          List.fold_left
            (fun result binding ->
              match binding.pmb_name.txt with
              | None -> result
              | Some name -> bind_module result name None)
            current bindings
      | Pstr_open declaration -> (
          walk_module_expression context current declaration.popen_expr;
          match resolve_module_expression current declaration.popen_expr with
          | Some path -> open_module current path
          | None ->
              if
                module_expression_mentions_sensitive current
                  declaration.popen_expr
              then
                add_error context declaration.popen_loc
                  "unresolved sensitive open is unsupported by capability \
                   analysis";
              current)
      | Pstr_include declaration -> (
          walk_module_expression context current declaration.pincl_mod;
          match resolve_module_expression current declaration.pincl_mod with
          | Some path -> open_module current path
          | None ->
              if
                module_expression_mentions_sensitive current
                  declaration.pincl_mod
              then
                add_error context declaration.pincl_loc
                  "unresolved sensitive include is unsupported by capability \
                   analysis";
              current)
      | Pstr_extension _ ->
          add_error context item.pstr_loc
            "structure extension is unsupported by capability analysis";
          current
      | _ ->
          Ast_iterator.default_iterator.structure_item
            (source_iterator context current)
            item;
          current)
    environment structure

and walk_signature context environment signature =
  List.iter
    (fun item ->
      match item.psig_desc with
      | Psig_extension _ ->
          add_error context item.psig_loc
            "signature extension is unsupported by capability analysis"
      | _ ->
          Ast_iterator.default_iterator.signature_item
            (source_iterator context environment)
            item)
    signature

let compare_detection (left : detection) (right : detection) =
  Stdlib.compare
    ( left.file,
      left.line,
      left.column,
      Capability.to_string left.capability,
      left.evidence )
    ( right.file,
      right.line,
      right.column,
      Capability.to_string right.capability,
      right.evidence )

let compare_banned (left : banned_construct) (right : banned_construct) =
  Stdlib.compare
    (left.file, left.line, left.column, left.construct, left.evidence)
    (right.file, right.line, right.column, right.construct, right.evidence)

let parse_source ~filename kind source =
  let lexbuf = Lexing.from_string source in
  Location.init lexbuf filename;
  try
    match kind with
    | Implementation -> Ok (`Implementation (Parse.implementation lexbuf))
    | Interface -> Ok (`Interface (Parse.interface lexbuf))
  with exn ->
    Error
      (Printf.sprintf "%s: parse error: %s" filename (Printexc.to_string exn))

let analyze_source ~filename kind source =
  let* parsed = parse_source ~filename kind source in
  let context =
    { filename; detections = ref []; banned = ref []; errors = ref [] }
  in
  let attribute_iterator =
    {
      Ast_iterator.default_iterator with
      attribute =
        (fun _ attribute ->
          if not (String.starts_with ~prefix:"ocaml." attribute.attr_name.txt)
          then
            add_error context attribute.attr_loc
              (Printf.sprintf
                 "attribute %s is unsupported by capability analysis"
                 attribute.attr_name.txt));
    }
  in
  (match parsed with
  | `Implementation structure ->
      attribute_iterator.structure attribute_iterator structure;
      ignore (walk_structure context empty_environment structure)
  | `Interface signature ->
      attribute_iterator.signature attribute_iterator signature;
      walk_signature context empty_environment signature);
  match List.rev !(context.errors) with
  | first :: rest -> Error (String.concat "\n" (first :: rest))
  | [] ->
      Ok
        ( List.sort_uniq compare_detection !(context.detections),
          List.sort_uniq compare_banned !(context.banned) )

let capability_pair capability =
  capability.Capability.category ^ ":" ^ capability.action

let compare_violation (left : violation) (right : violation) =
  Stdlib.compare
    (left.file, left.line, left.column, left.code, left.message)
    (right.file, right.line, right.column, right.code, right.message)

let cap001 (detection : detection) =
  let capability = Capability.to_string detection.capability in
  {
    code = "CAP001";
    file = detection.file;
    line = detection.line;
    column = detection.column;
    message =
      Printf.sprintf
        "%s:%d:%d: [CAP001] undeclared capability: %s (%s) — add it to \
         required_capabilities.json"
        detection.file detection.line detection.column capability
        detection.evidence;
  }

let exception_exists (manifest : manifest) construct =
  List.exists
    (fun declaration ->
      declaration.language = "ocaml" && declaration.construct = construct)
    manifest.exceptions

let capability_exists (manifest : manifest) required =
  let required_pair = capability_pair required in
  List.exists
    (fun declared -> capability_pair declared = required_pair)
    manifest.capabilities

let cap002 (manifest : manifest) (finding : banned_construct) =
  let exception_present = exception_exists manifest finding.construct in
  let capability_present =
    match finding.required_capability with
    | None -> false
    | Some required -> capability_exists manifest required
  in
  let hint =
    if not finding.exemptible then "remove this hard-banned construct"
    else
      match
        (exception_present, capability_present, finding.required_capability)
      with
      | false, false, Some required ->
          Printf.sprintf "add an exact ocaml exception and declare %s"
            (Capability.to_string required)
      | false, true, _ -> "add an exact ocaml banned_construct_exceptions entry"
      | true, false, Some required ->
          Printf.sprintf "declare %s to accompany the exception"
            (Capability.to_string required)
      | _ -> "remove the construct"
  in
  {
    code = "CAP002";
    file = finding.file;
    line = finding.line;
    column = finding.column;
    message =
      Printf.sprintf "%s:%d:%d: [CAP002] banned construct: %s — %s" finding.file
        finding.line finding.column finding.construct hint;
  }

let evaluate ~dir ~(manifest : manifest) ~detections ~banned =
  let declared_pairs =
    List.fold_left
      (fun pairs capability ->
        String_set.add (capability_pair capability) pairs)
      String_set.empty manifest.capabilities
  in
  let _, capability_violations =
    List.fold_left
      (fun (seen, violations) detection ->
        let key = Capability.to_string detection.capability in
        if
          String_set.mem (capability_pair detection.capability) declared_pairs
          || String_set.mem key seen
        then (seen, violations)
        else (String_set.add key seen, cap001 detection :: violations))
      (String_set.empty, [])
      (List.sort compare_detection detections)
  in
  let remaining_banned, banned_violations =
    List.fold_left
      (fun (remaining, violations) finding ->
        let permitted =
          finding.exemptible
          && exception_exists manifest finding.construct
          && Option.fold ~none:false
               ~some:(capability_exists manifest)
               finding.required_capability
        in
        if permitted then (remaining, violations)
        else (finding :: remaining, cap002 manifest finding :: violations))
      ([], [])
      (List.sort compare_banned banned)
  in
  {
    dir;
    detected = List.sort compare_detection detections;
    banned = List.rev remaining_banned;
    declared = List.sort_uniq Capability.compare manifest.capabilities;
    violations =
      List.sort compare_violation
        (List.rev_append capability_violations banned_violations);
  }

let passed result = result.violations = []
let format_violation violation = violation.message
let max_manifest_bytes = 1024 * 1024
let max_source_file_bytes = 4 * 1024 * 1024
let max_total_source_bytes = 64 * 1024 * 1024
let max_source_files = 10_000
let max_directory_depth = 64

let dune_stanza_head_exists text expected =
  let length = String.length text in
  let is_space = function ' ' | '\t' | '\r' | '\n' -> true | _ -> false in
  let is_boundary index =
    index >= length
    ||
    match text.[index] with
    | ' ' | '\t' | '\r' | '\n' | ')' | '(' -> true
    | _ -> false
  in
  let rec skip_space index =
    if index < length && is_space text.[index] then skip_space (index + 1)
    else index
  in
  let expected_length = String.length expected in
  let rec search index =
    if index >= length then false
    else if text.[index] <> '(' then search (index + 1)
    else
      let head = skip_space (index + 1) in
      if
        head + expected_length <= length
        && String.sub text head expected_length = expected
        && is_boundary (head + expected_length)
      then true
      else search (index + 1)
  in
  search 0

let compact_dune text =
  let output = Buffer.create (String.length text) in
  let rec normal index =
    if index < String.length text then
      match text.[index] with
      | ' ' | '\t' | '\r' | '\n' -> normal (index + 1)
      | ';' -> comment (index + 1)
      | '"' ->
          Buffer.add_char output '"';
          quoted (index + 1)
      | character ->
          Buffer.add_char output character;
          normal (index + 1)
  and comment index =
    if index < String.length text then
      if text.[index] = '\n' then normal (index + 1) else comment (index + 1)
  and quoted index =
    if index < String.length text then
      match text.[index] with
      | '\\' when index + 1 < String.length text ->
          Buffer.add_char output text.[index];
          Buffer.add_char output text.[index + 1];
          quoted (index + 2)
      | '"' ->
          Buffer.add_char output '"';
          normal (index + 1)
      | character ->
          Buffer.add_char output character;
          quoted (index + 1)
  in
  normal 0;
  Buffer.contents output

let remove_dune_comments text =
  let output = Buffer.create (String.length text) in
  let rec normal index =
    if index < String.length text then
      match text.[index] with
      | ';' ->
          Buffer.add_char output ' ';
          comment (index + 1)
      | '"' ->
          Buffer.add_char output '"';
          quoted (index + 1)
      | character ->
          Buffer.add_char output character;
          normal (index + 1)
  and comment index =
    if index < String.length text then
      if text.[index] = '\n' then (
        Buffer.add_char output '\n';
        normal (index + 1))
      else comment (index + 1)
  and quoted index =
    if index < String.length text then
      match text.[index] with
      | '\\' when index + 1 < String.length text ->
          Buffer.add_char output text.[index];
          Buffer.add_char output text.[index + 1];
          quoted (index + 2)
      | '"' ->
          Buffer.add_char output '"';
          normal (index + 1)
      | character ->
          Buffer.add_char output character;
          quoted (index + 1)
  in
  normal 0;
  Buffer.contents output

let remove_substrings text needle =
  let output = Buffer.create (String.length text) in
  let needle_length = String.length needle in
  let rec copy index =
    if index < String.length text then
      if
        index + needle_length <= String.length text
        && String.sub text index needle_length = needle
      then copy (index + needle_length)
      else (
        Buffer.add_char output text.[index];
        copy (index + 1))
  in
  copy 0;
  Buffer.contents output

let safe_instrumentation_only ~allow_bisect contents =
  let compact = compact_dune contents in
  let without_safe =
    if allow_bisect then
      remove_substrings compact "(instrumentation(backendbisect_ppx))"
    else compact
  in
  not (dune_stanza_head_exists without_safe "instrumentation")

let unsafe_dune_construct ~allow_bisect contents =
  let comment_normalized = remove_dune_comments contents in
  let unsafe_stanzas =
    [
      "preprocess";
      "preprocessor_deps";
      "rule";
      "ocamllex";
      "menhir";
      "include";
      "copy_files";
      "copy_files#";
      "dynamic_include";
      "flags";
      "ocamlc_flags";
      "ocamlopt_flags";
      "foreign_stubs";
      "foreign_archives";
      "foreign_library";
      "ctypes";
      "extra_objects";
      "link_flags";
      "c_library_flags";
      "instrumentation.backend";
    ]
  in
  if not (safe_instrumentation_only ~allow_bisect contents) then
    Some "(instrumentation ...)"
  else
    match
      List.find_opt (dune_stanza_head_exists comment_normalized) unsafe_stanzas
    with
    | Some stanza -> Some ("(" ^ stanza ^ " ...)")
    | None -> None

let read_file ~max_bytes path =
  try
    In_channel.with_open_bin path (fun channel ->
        let length = In_channel.length channel in
        if length > Int64.of_int max_bytes then
          Error
            (Printf.sprintf "reading %s: input is %Ld bytes; limit is %d" path
               length max_bytes)
        else if length > Int64.of_int Sys.max_string_length then
          Error
            (Printf.sprintf "reading %s: input exceeds string capacity" path)
        else
          match
            In_channel.really_input_string channel (Int64.to_int length)
          with
          | Some source -> Ok source
          | None -> Error (Printf.sprintf "reading %s: input changed size" path))
  with Sys_error message ->
    Error (Printf.sprintf "reading %s: %s" path message)

let empty_manifest package = { package; capabilities = []; exceptions = [] }

let load_manifest directory =
  let path = Filename.concat directory "required_capabilities.json" in
  let expected_package = "ocaml/" ^ Filename.basename directory in
  match Unix.lstat path with
  | exception Unix.Unix_error (Unix.ENOENT, _, _) ->
      Ok (empty_manifest expected_package)
  | stats when stats.st_kind <> Unix.S_REG ->
      Error (path ^ " must be a regular, non-symlink file")
  | _ ->
      let* source = read_file ~max_bytes:max_manifest_bytes path in
      let* manifest = parse_manifest source in
      if manifest.package <> expected_package then
        Error
          (Printf.sprintf "manifest package %S must equal %S" manifest.package
             expected_package)
      else Ok manifest

let excluded_directory name =
  List.mem name [ "_build"; ".git"; "_opam"; "node_modules" ]

let source_kind path =
  if String.ends_with ~suffix:".mli" path then Some Interface
  else if String.ends_with ~suffix:".ml" path then Some Implementation
  else None

let normalize_relative path =
  if Filename.dir_sep = "/" then path
  else
    String.map
      (fun character -> if character = '\\' then '/' else character)
      path

let discover_sources root =
  let sources = ref [] in
  let source_count = ref 0 in
  let total_bytes = ref 0 in
  let rec walk depth relative directory =
    if depth > max_directory_depth then
      raise
        (Failure
           (Printf.sprintf "source directory depth exceeds limit %d at %s"
              max_directory_depth relative));
    let entries =
      try Sys.readdir directory |> Array.to_list |> List.sort String.compare
      with Sys_error message ->
        raise (Failure (Printf.sprintf "listing %s: %s" directory message))
    in
    List.iter
      (fun name ->
        let absolute = Filename.concat directory name in
        let child_relative =
          if relative = "" then name else Filename.concat relative name
        in
        let stats =
          try Unix.lstat absolute
          with Unix.Unix_error (error, _, _) ->
            raise
              (Failure
                 (Printf.sprintf "inspecting %s: %s" absolute
                    (Unix.error_message error)))
        in
        match stats.st_kind with
        | Unix.S_LNK ->
            raise
              (Failure
                 (Printf.sprintf "symlinked package input is not allowed: %s"
                    child_relative))
        | Unix.S_DIR when excluded_directory name -> ()
        | Unix.S_DIR -> walk (depth + 1) child_relative absolute
        | Unix.S_REG -> (
            match source_kind name with
            | Some kind ->
                if stats.st_size > max_source_file_bytes then
                  raise
                    (Failure
                       (Printf.sprintf
                          "source file %s is %d bytes; per-file limit is %d"
                          child_relative stats.st_size max_source_file_bytes));
                incr source_count;
                if !source_count > max_source_files then
                  raise
                    (Failure
                       (Printf.sprintf "source file count exceeds limit %d"
                          max_source_files));
                total_bytes := !total_bytes + stats.st_size;
                if !total_bytes > max_total_source_bytes then
                  raise
                    (Failure
                       (Printf.sprintf "total source bytes exceed limit %d"
                          max_total_source_bytes));
                sources :=
                  (normalize_relative child_relative, absolute, kind)
                  :: !sources
            | None ->
                if
                  String.ends_with ~suffix:".mll" name
                  || String.ends_with ~suffix:".mly" name
                then
                  raise
                    (Failure
                       (Printf.sprintf
                          "generated OCaml source input is unsupported: %s"
                          child_relative))
                else if name = "dune-workspace" then
                  raise
                    (Failure
                       (Printf.sprintf
                          "Dune workspace input is unsupported by capability \
                           analysis: %s"
                          child_relative))
                else if name = "dune" || name = "dune-project" then
                  let contents =
                    match read_file ~max_bytes:max_manifest_bytes absolute with
                    | Ok contents -> contents
                    | Error message -> raise (Failure message)
                  in
                  let normalized_child = normalize_relative child_relative in
                  let allow_bisect =
                    List.mem normalized_child
                      [ "bin/dune"; "src/dune"; "test/dune" ]
                  in
                  let unsupported =
                    unsafe_dune_construct ~allow_bisect contents
                  in
                  Option.iter
                    (fun stanza ->
                      raise
                        (Failure
                           (Printf.sprintf
                              "unsupported generated-source stanza %s in %s"
                              stanza child_relative)))
                    unsupported)
        | _ -> ())
      entries
  in
  try
    walk 0 "" root;
    Ok
      (List.sort
         (fun (left, _, _) (right, _, _) -> String.compare left right)
         !sources)
  with Failure message -> Error message

let analyze_directory directory =
  try
    let root = Unix.realpath directory in
    let* manifest = load_manifest root in
    let* sources = discover_sources root in
    let rec analyze detections banned = function
      | [] -> Ok (detections, banned)
      | (relative, absolute, kind) :: rest ->
          let* source = read_file ~max_bytes:max_source_file_bytes absolute in
          let* file_detections, file_banned =
            analyze_source ~filename:relative kind source
          in
          analyze
            (List.rev_append file_detections detections)
            (List.rev_append file_banned banned)
            rest
    in
    let* detections, banned = analyze [] [] sources in
    Ok (evaluate ~dir:root ~manifest ~detections ~banned)
  with
  | Unix.Unix_error (error, operation, path) ->
      Error
        (Printf.sprintf "%s %s: %s" operation path (Unix.error_message error))
  | Sys_error message -> Error message

let format_detection (detection : detection) =
  Printf.sprintf "%s:%d:%d: detected %s (%s)\n" detection.file detection.line
    detection.column
    (Capability.to_string detection.capability)
    detection.evidence

let run ~dir ~verbose ~stdout ~stderr =
  match analyze_directory dir with
  | Error message ->
      stderr ("coding-adventures-capability-analyzer: " ^ message ^ "\n");
      2
  | Ok result ->
      if verbose then
        List.iter
          (fun detection -> stdout (format_detection detection))
          result.detected;
      if passed result then (
        stdout
          (Printf.sprintf "coding-adventures-capability-analyzer: %s passed\n"
             result.dir);
        0)
      else (
        List.iter
          (fun violation -> stdout (format_violation violation ^ "\n"))
          result.violations;
        1)
