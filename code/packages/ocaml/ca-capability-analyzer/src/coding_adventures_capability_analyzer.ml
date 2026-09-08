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
  && String.for_all
       (function 'a' .. 'z' | '0' .. '9' | '_' | '-' -> true | _ -> false)
       (String.sub package prefix_length
          (String.length package - prefix_length))

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

let resolve_module_expression environment module_expression =
  match module_expression.pmod_desc with
  | Pmod_ident identifier ->
      Option.bind
        (longident_segments identifier.txt)
        (resolve_module_segments environment)
  | _ -> None

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

let detect_call context environment expression function_expression arguments =
  match function_expression.pexp_desc with
  | Pexp_ident identifier -> (
      match resolve_call environment identifier.txt with
      | None -> ()
      | Some (module_name, function_name) ->
          capabilities_for module_name function_name
          |> List.iter (fun detected ->
                 add_detection context expression.pexp_loc detected
                   (evidence module_name function_name));
          detect_standard_channel context environment expression.pexp_loc
            module_name function_name arguments;
          if module_name = "Obj" then
            let construct = "Obj." ^ function_name in
            add_banned context expression.pexp_loc ~exemptible:false construct
              (construct ^ " bypasses OCaml type safety")
          else if
            module_name = "Marshal"
            && List.mem function_name
                 [ "from_channel"; "from_string"; "from_bytes" ]
          then
            let construct = "Marshal." ^ function_name in
            add_banned context expression.pexp_loc ~exemptible:false construct
              (construct ^ " reconstructs unchecked runtime values")
          else if
            module_name = "Dynlink"
            && List.mem function_name [ "loadfile"; "loadfile_private" ]
          then
            let construct = "Dynlink." ^ function_name in
            add_banned context expression.pexp_loc
              ~required_capability:(capability "ffi" "load") ~exemptible:true
              construct
              (construct ^ " loads native or bytecode modules at runtime"))
  | Pexp_extension _ ->
      add_error context expression.pexp_loc
        "extension node in call position is unsupported by capability analysis"
  | _ -> ()

let add_external context location =
  let required_capability = capability "ffi" "call" in
  add_detection context location required_capability "external declaration";
  add_banned context location ~required_capability ~exemptible:true "external"
    "external declaration crosses an unchecked native boundary"

let detect_marshal_closures context environment expression identifier =
  match longident_segments identifier with
  | None -> ()
  | Some segments -> (
      match last_two (normalize_stdlib_prefix segments) with
      | Some (module_name, "Closures") ->
          let canonical =
            resolve_module_segments environment [ module_name ]
            |> Option.value ~default:[ module_name ]
          in
          if List.rev canonical |> List.hd = "Marshal" then
            add_banned context expression.pexp_loc ~exemptible:false
              "Marshal.Closures"
              "Marshal.Closures serializes executable closures"
      | _ -> ())

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
            bind_module environment name
              (resolve_module_expression environment module_expression)
      in
      walk_expression context body_environment body
  | Pexp_open (declaration, body) ->
      walk_module_expression context environment declaration.popen_expr;
      let body_environment =
        match resolve_module_expression environment declaration.popen_expr with
        | Some path -> open_module environment path
        | None -> environment
      in
      walk_expression context body_environment body
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
          | Some name ->
              bind_module current name
                (resolve_module_expression current binding.pmb_expr))
      | Pstr_recmodule bindings ->
          List.iter
            (fun binding ->
              walk_module_expression context current binding.pmb_expr)
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
          | None -> current)
      | Pstr_include declaration ->
          walk_module_expression context current declaration.pincl_mod;
          current
      | _ ->
          Ast_iterator.default_iterator.structure_item
            (source_iterator context current)
            item;
          current)
    environment structure

and walk_signature context environment signature =
  Ast_iterator.default_iterator.signature
    (source_iterator context environment)
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
  (match parsed with
  | `Implementation structure ->
      ignore (walk_structure context empty_environment structure)
  | `Interface signature -> walk_signature context empty_environment signature);
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

let read_file path =
  try Ok (In_channel.with_open_bin path In_channel.input_all)
  with Sys_error message ->
    Error (Printf.sprintf "reading %s: %s" path message)

let empty_manifest package = { package; capabilities = []; exceptions = [] }

let load_manifest directory =
  let path = Filename.concat directory "required_capabilities.json" in
  if not (Sys.file_exists path) then
    Ok (empty_manifest ("ocaml/" ^ Filename.basename directory))
  else
    let* source = read_file path in
    parse_manifest source

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
  let rec walk relative directory =
    let entries =
      try Sys.readdir directory |> Array.to_list |> List.sort String.compare
      with Sys_error message ->
        raise (Failure (Printf.sprintf "listing %s: %s" directory message))
    in
    List.fold_left
      (fun paths name ->
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
        | Unix.S_DIR when excluded_directory name -> paths
        | Unix.S_DIR -> paths @ walk child_relative absolute
        | Unix.S_REG -> (
            match source_kind name with
            | Some kind ->
                paths @ [ (normalize_relative child_relative, absolute, kind) ]
            | None -> paths)
        | _ -> paths)
      [] entries
  in
  try Ok (walk "" root) with Failure message -> Error message

let analyze_directory directory =
  try
    let root = Unix.realpath directory in
    let* manifest = load_manifest root in
    let* sources = discover_sources root in
    let rec analyze detections banned = function
      | [] -> Ok (detections, banned)
      | (relative, absolute, kind) :: rest ->
          let* source = read_file absolute in
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
