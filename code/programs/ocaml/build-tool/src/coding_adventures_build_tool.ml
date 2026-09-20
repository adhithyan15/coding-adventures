(** Pure graph and diff-selection decisions.

    The interesting boundary here is what the code does not know. Callers hand
    us package names, edges, changed paths, and an inert boundary registry. The
    module has no way to discover a file, ask Git a question, or execute a
    command. That makes the two decisions reproducible on every host. *)

type edge = { prerequisite : string; dependent : string }

type graph_input = { packages : string list; edges : edge list }

type graph_result = { edges : edge list; levels : string list list }

type source_mode = Package_prefix | Strict_globs

type unknown_path_policy = All | Error

type package_spec = {
  name : string;
  rel_path : string;
  source_mode : source_mode;
  source_globs : string list;
}

type applies_to = {
  exact_roots : string list;
  descendant_roots : string list;
  excluded_roots : string list;
}

type boundary_input = {
  path : string;
  role : string;
  generated_component : string option;
}

type boundary_rule = {
  id : string;
  input_origin : string;
  applies_to : applies_to;
  inputs : boundary_input list;
  reason : string;
  owner : string;
}

type repository_boundary = {
  schema_version : int;
  language_source_input_registry_sha256 : string;
  boundaries : boundary_rule list;
}

type diff_selection_input = {
  packages : package_spec list;
  edges : edge list;
  forced_packages : string list;
  unknown_path_policy : unknown_path_policy;
  changed_paths : string list;
  boundary_sha256 : string option;
  boundary : repository_boundary option;
}

type diff_selection_result = {
  changed_packages : string list;
  affected_packages : string list;
  prerequisite_packages : string list;
}

type error =
  | Graph_package_limit_exceeded
  | Graph_edge_limit_exceeded
  | Graph_package_invalid
  | Graph_package_duplicate
  | Graph_edge_unknown
  | Graph_edge_self
  | Graph_edge_duplicate
  | Graph_edge_invalid
  | Graph_invalid
  | Graph_cycle
  | Diff_edge_cycle
  | Diff_package_invalid
  | Diff_package_duplicate
  | Diff_path_invalid
  | Diff_glob_invalid
  | Diff_forced_package_invalid
  | Diff_forced_package_unknown
  | Diff_boundary_digest_mismatch
  | Diff_unknown_path
  | Diff_match_limit_exceeded

let error_code = function
  | Graph_package_limit_exceeded -> "GRAPH_PACKAGE_LIMIT_EXCEEDED"
  | Graph_edge_limit_exceeded -> "GRAPH_EDGE_LIMIT_EXCEEDED"
  | Graph_package_invalid -> "GRAPH_PACKAGE_INVALID"
  | Graph_package_duplicate -> "GRAPH_PACKAGE_DUPLICATE"
  | Graph_edge_unknown -> "GRAPH_EDGE_UNKNOWN"
  | Graph_edge_self -> "GRAPH_EDGE_SELF"
  | Graph_edge_duplicate -> "GRAPH_EDGE_DUPLICATE"
  | Graph_edge_invalid -> "GRAPH_EDGE_INVALID"
  | Graph_invalid -> "GRAPH_INVALID"
  | Graph_cycle -> "GRAPH_CYCLE"
  | Diff_edge_cycle -> "DIFF_EDGE_CYCLE"
  | Diff_package_invalid -> "DIFF_PACKAGE_INVALID"
  | Diff_package_duplicate -> "DIFF_PACKAGE_DUPLICATE"
  | Diff_path_invalid -> "DIFF_PATH_INVALID"
  | Diff_glob_invalid -> "DIFF_GLOB_INVALID"
  | Diff_forced_package_invalid -> "DIFF_FORCED_PACKAGE_INVALID"
  | Diff_forced_package_unknown -> "DIFF_FORCED_PACKAGE_UNKNOWN"
  | Diff_boundary_digest_mismatch -> "DIFF_BOUNDARY_DIGEST_MISMATCH"
  | Diff_unknown_path -> "DIFF_UNKNOWN_PATH"
  | Diff_match_limit_exceeded -> "DIFF_MATCH_LIMIT_EXCEEDED"

module String_set = Set.Make (String)

module Edge_set = Set.Make (struct
  type t = string * string

  let compare = compare
end)

module Directed = Coding_adventures_directed_graph.Make (struct
  type t = string

  let compare = String.compare
end)

let max_packages = 4_096
let max_edges = 16_384
let max_match_work = 50_000_000L

let build_names =
  String_set.of_list
    [ "BUILD"; "BUILD_windows"; "BUILD_mac"; "BUILD_linux";
      "BUILD_mac_and_linux" ]

let windows_reserved =
  String_set.of_list
    [ "CON"; "PRN"; "AUX"; "NUL"; "CONIN$"; "CONOUT$"; "CLOCK$";
      "COM1"; "COM2"; "COM3"; "COM4"; "COM5"; "COM6"; "COM7";
      "COM8"; "COM9"; "LPT1"; "LPT2"; "LPT3"; "LPT4"; "LPT5";
      "LPT6"; "LPT7"; "LPT8"; "LPT9"; "COM¹"; "COM²"; "COM³";
      "LPT¹"; "LPT²"; "LPT³" ]

let fail error = Error error

let ( let* ) value next = match value with Ok item -> next item | Error _ as e -> e

(** OCaml 5 exposes a strict UTF-8 decoder in the standard library. We keep
    decoding explicit so malformed bytes never reach Unicode tables. *)
let decode_utf8 value =
  let rec loop index scalars =
    if index = String.length value then Some (List.rev scalars)
    else
      let decoded = String.get_utf_8_uchar value index in
      if not (Uchar.utf_decode_is_valid decoded) then None
      else
        loop
          (index + Uchar.utf_decode_length decoded)
          (Uchar.utf_decode_uchar decoded :: scalars)
  in
  loop 0 []

let encode_utf8 scalars =
  let buffer = Buffer.create (List.length scalars) in
  List.iter (Buffer.add_utf_8_uchar buffer) scalars;
  Buffer.contents buffer

let normalize_nfc value =
  match decode_utf8 value with
  | None -> None
  | Some scalars ->
      let normalizer = Uunf.create `NFC in
      let output = ref [] in
      let rec drain input =
        match Uunf.add normalizer input with
        | `Uchar scalar ->
            output := scalar :: !output;
            drain `Await
        | `Await | `End -> ()
      in
      List.iter (fun scalar -> drain (`Uchar scalar)) scalars;
      drain `End;
      Some (encode_utf8 (List.rev !output))

let map_unicode mapper value =
  match decode_utf8 value with
  | None -> None
  | Some scalars ->
      let mapped =
        List.concat_map
          (fun scalar ->
            match mapper scalar with `Self -> [ scalar ] | `Uchars items -> items)
          scalars
      in
      Some (encode_utf8 mapped)

let full_casefold value = map_unicode Uucp.Case.Fold.fold value
let full_uppercase value = map_unicode Uucp.Case.Map.to_upper value

let scalar_count value =
  match decode_utf8 value with None -> None | Some scalars -> Some (List.length scalars)

let compare_unicode left right =
  match (decode_utf8 left, decode_utf8 right) with
  | Some left, Some right ->
      let rec compare_lists a b =
        match (a, b) with
        | [], [] -> 0
        | [], _ -> -1
        | _, [] -> 1
        | x :: xs, y :: ys ->
            let compared = Int.compare (Uchar.to_int x) (Uchar.to_int y) in
            if compared = 0 then compare_lists xs ys else compared
      in
      compare_lists left right
  | _ -> String.compare left right

let sort_strings values = List.sort compare_unicode values

let sort_edges values =
  List.sort
    (fun left right ->
      let compared = compare_unicode left.prerequisite right.prerequisite in
      if compared = 0 then compare_unicode left.dependent right.dependent
      else compared)
    values

let is_ascii_lower_or_digit character =
  (character >= 'a' && character <= 'z')
  || (character >= '0' && character <= '9')

let is_package_component component =
  let length = String.length component in
  length > 0
  && is_ascii_lower_or_digit component.[0]
  && String.for_all
       (fun character ->
         is_ascii_lower_or_digit character
         || character = '.' || character = '_' || character = '-')
       component

let is_package_name value =
  match scalar_count value with
  | None | Some 0 -> false
  | Some count ->
      count <= 240
      && String.contains value '/'
      && List.for_all is_package_component (String.split_on_char '/' value)

let contains value needle =
  let value_length = String.length value and needle_length = String.length needle in
  let rec search index =
    if index + needle_length > value_length then false
    else if String.sub value index needle_length = needle then true
    else search (index + 1)
  in
  needle_length = 0 || search 0

let starts_with value prefix =
  String.length value >= String.length prefix
  && String.sub value 0 (String.length prefix) = prefix

let ends_with value suffix =
  String.length value >= String.length suffix
  && String.sub value (String.length value - String.length suffix)
       (String.length suffix)
     = suffix

let first_dot_component segment =
  match String.index_opt segment '.' with
  | None -> segment
  | Some index -> String.sub segment 0 index

let is_reserved segment =
  match full_uppercase (first_dot_component segment) with
  | None -> true
  | Some upper -> String_set.mem upper windows_reserved

let has_forbidden ~path value =
  let forbidden = if path then "<>:\"|?*" else "<>:\"|?" in
  match decode_utf8 value with
  | None -> true
  | Some scalars ->
      List.exists
        (fun scalar ->
          let code = Uchar.to_int scalar in
          code < 32 || (code < 128 && String.contains forbidden (Char.chr code)))
        scalars

let has_drive_prefix value =
  String.length value >= 2
  && ((value.[0] >= 'A' && value.[0] <= 'Z')
     || (value.[0] >= 'a' && value.[0] <= 'z'))
  && value.[1] = ':'

let portable_segments value =
  List.for_all
    (fun segment ->
      String.length segment > 0
      && not (String.equal segment ".")
      && not (String.equal segment "..")
      && not (ends_with segment " ")
      && not (ends_with segment "."))
    (String.split_on_char '/' value)

let is_portable_path value =
  match (scalar_count value, normalize_nfc value) with
  | Some count, Some normalized ->
      count > 0 && count <= 512 && String.equal normalized value
      && not (starts_with value "/")
      && not (has_drive_prefix value)
      && not (String.contains value '\\')
      && not (contains value "//")
      && not (has_forbidden ~path:true value)
      && portable_segments value
      && List.for_all (fun segment -> not (is_reserved segment))
           (String.split_on_char '/' value)
  | _ -> false

let class_closing pattern opening =
  let length = Array.length pattern in
  let cursor = ref (opening + 1) in
  if !cursor < length && pattern.(!cursor) = Char.code '!' then incr cursor;
  if !cursor < length && pattern.(!cursor) = Char.code ']' then incr cursor;
  let closing = ref !cursor in
  while !closing < length && pattern.(!closing) <> Char.code ']' do
    incr closing
  done;
  if !closing = length then None else Some (!cursor, !closing)

let has_ambiguous_character_class segment =
  match decode_utf8 segment with
  | None -> true
  | Some scalars ->
      let values = Array.of_list (List.map Uchar.to_int scalars) in
      let rec scan index =
        if index >= Array.length values then false
        else if values.(index) <> Char.code '[' then scan (index + 1)
        else
          match class_closing values index with
          | None -> scan (index + 1)
          | Some (start, closing) ->
              let rec ambiguous member =
                if member >= closing - 1 then false
                else
                  let left = values.(member) and right = values.(member + 1) in
                  ((left = Char.code '-' && right = Char.code '-')
                  || (left = Char.code '&' && right = Char.code '&')
                  || (left = Char.code '~' && right = Char.code '~')
                  || (left = Char.code '|' && right = Char.code '|'))
                  || ambiguous (member + 1)
              in
              let rec descending member =
                if member + 2 >= closing then false
                else if values.(member + 1) = Char.code '-' then
                  values.(member) > values.(member + 2)
                  || descending (member + 3)
                else descending (member + 1)
              in
              ambiguous start || descending start || scan (closing + 1)
      in
      scan 0

let is_portable_glob value =
  match (scalar_count value, normalize_nfc value) with
  | Some count, Some normalized ->
      count > 0 && count <= 512 && String.equal normalized value
      && not (starts_with value "/")
      && not (has_drive_prefix value)
      && not (String.contains value '\\')
      && not (contains value "//")
      && not (has_forbidden ~path:false value)
      && portable_segments value
      && List.for_all
           (fun segment ->
             (String.exists
                (fun character ->
                  character = '*' || character = '[' || character = ']'
                  || character = '{' || character = '}')
                segment
             || not (is_reserved segment))
             && not (has_ambiguous_character_class segment))
           (String.split_on_char '/' value)
  | _ -> false

type validated_graph = {
  names : String_set.t;
  edges : edge list;
  forward : Directed.t;
  reverse : Directed.t;
}

let validate_graph packages edges =
  if List.length packages > max_packages then fail Graph_package_limit_exceeded
  else if List.length edges > max_edges then fail Graph_edge_limit_exceeded
  else
    let rec collect_names names = function
      | [] -> Ok names
      | name :: rest ->
          if not (is_package_name name) then fail Graph_package_invalid
          else if String_set.mem name names then fail Graph_package_duplicate
          else collect_names (String_set.add name names) rest
    in
    let* names = collect_names String_set.empty packages in
    let forward = Directed.create () and reverse = Directed.create () in
    String_set.iter
      (fun name ->
        Directed.add_node forward name;
        Directed.add_node reverse name)
      names;
    let rec collect_edges seen = function
      | [] -> Ok ()
      | edge :: rest ->
          if
            (not (String_set.mem edge.prerequisite names))
            || not (String_set.mem edge.dependent names)
          then fail Graph_edge_unknown
          else if String.equal edge.prerequisite edge.dependent then
            fail Graph_edge_self
          else if Edge_set.mem (edge.prerequisite, edge.dependent) seen then
            fail Graph_edge_duplicate
          else
            let seen = Edge_set.add (edge.prerequisite, edge.dependent) seen in
            let* () =
              match Directed.add_edge forward edge.prerequisite edge.dependent with
              | Ok () -> Ok ()
              | Error _ -> fail Graph_edge_invalid
            in
            let* () =
              match Directed.add_edge reverse edge.dependent edge.prerequisite with
              | Ok () -> Ok ()
              | Error _ -> fail Graph_edge_invalid
            in
            collect_edges seen rest
    in
    let* () = collect_edges Edge_set.empty edges in
    Ok { names; edges = sort_edges edges; forward; reverse }

let evaluate_graph input =
  let* graph = validate_graph input.packages input.edges in
  match Directed.independent_groups graph.forward with
  | Error Directed.Cycle -> fail Graph_cycle
  | Error _ -> fail Graph_invalid
  | Ok levels -> Ok { edges = graph.edges; levels = List.map sort_strings levels }

let inside path root =
  String.equal path root || starts_with path (root ^ "/")

let relative path root =
  if String.equal path root then ""
  else String.sub path (String.length root + 1)
         (String.length path - String.length root - 1)

let basename path =
  match String.rindex_opt path '/' with
  | None -> path
  | Some index ->
      String.sub path (index + 1) (String.length path - index - 1)

let class_matches pattern start finish value =
  let negated = start < finish && pattern.(start) = Char.code '!' in
  let rec loop index matched =
    if index >= finish then if negated then not matched else matched
    else if index + 2 < finish && pattern.(index + 1) = Char.code '-' then
      loop (index + 3)
        (matched || (value >= pattern.(index) && value <= pattern.(index + 2)))
    else loop (index + 1) (matched || value = pattern.(index))
  in
  loop (if negated then start + 1 else start) false

let segment_matches pattern value =
  match (decode_utf8 pattern, decode_utf8 value) with
  | Some pattern, Some value ->
      let pattern = Array.of_list (List.map Uchar.to_int pattern)
      and value = Array.of_list (List.map Uchar.to_int value) in
      let memo = Hashtbl.create 32 in
      let rec matches pattern_index value_index =
        match Hashtbl.find_opt memo (pattern_index, value_index) with
        | Some result -> result
        | None ->
            let result =
              if pattern_index = Array.length pattern then
                value_index = Array.length value
              else if pattern.(pattern_index) = Char.code '*' then
                matches (pattern_index + 1) value_index
                || (value_index < Array.length value
                   && matches pattern_index (value_index + 1))
              else if pattern.(pattern_index) = Char.code '[' then
                (match class_closing pattern pattern_index with
                | None ->
                    value_index < Array.length value
                    && pattern.(pattern_index) = value.(value_index)
                    && matches (pattern_index + 1) (value_index + 1)
                | Some (start, closing) ->
                    value_index < Array.length value
                    && class_matches pattern start closing value.(value_index)
                    && matches (closing + 1) (value_index + 1))
              else if pattern.(pattern_index) = Char.code '?' then
                value_index < Array.length value
                && matches (pattern_index + 1) (value_index + 1)
              else
                value_index < Array.length value
                && pattern.(pattern_index) = value.(value_index)
                && matches (pattern_index + 1) (value_index + 1)
            in
            Hashtbl.add memo (pattern_index, value_index) result;
            result
      in
      matches 0 0
  | _ -> false

let glob_matches pattern path =
  let patterns = Array.of_list (String.split_on_char '/' pattern)
  and paths = Array.of_list (String.split_on_char '/' path) in
  let memo = Hashtbl.create 32 in
  let rec matches pattern_index path_index =
    match Hashtbl.find_opt memo (pattern_index, path_index) with
    | Some result -> result
    | None ->
        let result =
          if pattern_index = Array.length patterns then
            path_index = Array.length paths
          else if String.equal patterns.(pattern_index) "**" then
            matches (pattern_index + 1) path_index
            || (path_index < Array.length paths
               && matches pattern_index (path_index + 1))
          else
            path_index < Array.length paths
            && segment_matches patterns.(pattern_index) paths.(path_index)
            && matches (pattern_index + 1) (path_index + 1)
        in
        Hashtbl.add memo (pattern_index, path_index) result;
        result
  in
  matches 0 0

let json_string value =
  let output = Buffer.create (String.length value + 2) in
  Buffer.add_char output '"';
  String.iter
    (fun character ->
      match character with
      | '"' -> Buffer.add_string output "\\\""
      | '\\' -> Buffer.add_string output "\\\\"
      | '\b' -> Buffer.add_string output "\\b"
      | '\012' -> Buffer.add_string output "\\f"
      | '\n' -> Buffer.add_string output "\\n"
      | '\r' -> Buffer.add_string output "\\r"
      | '\t' -> Buffer.add_string output "\\t"
      | character when Char.code character < 32 ->
          Buffer.add_string output
            (Printf.sprintf "\\u%04x" (Char.code character))
      | character -> Buffer.add_char output character)
    value;
  Buffer.add_char output '"';
  Buffer.contents output

let json_array encode values =
  "[" ^ String.concat "," (List.map encode values) ^ "]"

let canonical_applies value =
  "{\"descendant_roots\":"
  ^ json_array json_string value.descendant_roots
  ^ ",\"exact_roots\":" ^ json_array json_string value.exact_roots
  ^ ",\"excluded_roots\":" ^ json_array json_string value.excluded_roots
  ^ "}"

let canonical_boundary_input value =
  let generated =
    match value.generated_component with
    | None -> ""
    | Some component -> "\"generated_component\":" ^ json_string component ^ ","
  in
  "{" ^ generated ^ "\"path\":" ^ json_string value.path ^ ",\"role\":"
  ^ json_string value.role ^ "}"

let canonical_boundary_rule value =
  "{\"applies_to\":" ^ canonical_applies value.applies_to ^ ",\"id\":"
  ^ json_string value.id ^ ",\"input_origin\":" ^ json_string value.input_origin
  ^ ",\"inputs\":" ^ json_array canonical_boundary_input value.inputs
  ^ ",\"owner\":" ^ json_string value.owner ^ ",\"reason\":"
  ^ json_string value.reason ^ "}"

let canonical_boundary value =
  "{\"boundaries\":" ^ json_array canonical_boundary_rule value.boundaries
  ^ ",\"language_source_input_registry_sha256\":"
  ^ json_string value.language_source_input_registry_sha256
  ^ ",\"schema_version\":" ^ string_of_int value.schema_version ^ "}"

let u64_big_endian value =
  let value = Int64.of_int value in
  let output = Bytes.create 8 in
  for index = 0 to 7 do
    let shift = (7 - index) * 8 in
    Bytes.set output index
      (Char.chr
         (Int64.to_int (Int64.logand 0xffL (Int64.shift_right_logical value shift))))
  done;
  Bytes.unsafe_to_string output

let repository_boundary_digest boundary =
  let encoded = canonical_boundary boundary in
  Digestif.SHA256.digest_string
    ("coding-adventures/build-tool-repository-source-input-boundary/v1\000"
    ^ u64_big_endian (String.length encoded) ^ encoded)
  |> Digestif.SHA256.to_hex

let lowercase_hex_digest value =
  String.length value = 64
  && String.for_all
       (fun character ->
         (character >= '0' && character <= '9')
         || (character >= 'a' && character <= 'f'))
       value

let applies_to applies root =
  List.exists (String.equal root) applies.exact_roots
  || ((not (List.exists (String.equal root) applies.excluded_roots))
     && List.exists (fun ancestor -> starts_with root (ancestor ^ "/"))
          applies.descendant_roots)

let add_consumer consumers path package =
  let current =
    Option.value ~default:String_set.empty (Hashtbl.find_opt consumers path)
  in
  Hashtbl.replace consumers path (String_set.add package current)

let closure graph seeds =
  String_set.fold
    (fun seed result ->
      match Directed.transitive_closure graph seed with
      | Error _ -> result
      | Ok reachable ->
          List.fold_left (fun values item -> String_set.add item values) result
            reachable)
    seeds seeds

let unique values =
  let rec loop seen = function
    | [] -> true
    | item :: rest ->
        if String_set.mem item seen then false
        else loop (String_set.add item seen) rest
  in
  loop String_set.empty values

let portable_identity value =
  match normalize_nfc value with
  | None -> None
  | Some normalized -> full_casefold normalized

let validate_packages packages =
  let table = Hashtbl.create (List.length packages) in
  let rec loop identities = function
    | [] -> Ok table
    | spec :: rest ->
        if not (is_package_name spec.name) then fail Diff_package_invalid
        else if Hashtbl.mem table spec.name then fail Diff_package_duplicate
        else if not (is_portable_path spec.rel_path) then fail Diff_path_invalid
        else if List.length spec.source_globs > 256 || not (unique spec.source_globs)
        then fail Diff_glob_invalid
        else if not (List.for_all is_portable_glob spec.source_globs) then
          fail Diff_glob_invalid
        else if spec.source_mode = Package_prefix && spec.source_globs <> [] then
          fail Diff_glob_invalid
        else
          match portable_identity spec.rel_path with
          | None -> fail Diff_path_invalid
          | Some identity ->
              if
                List.exists
                  (fun root ->
                    String.equal identity root || starts_with identity (root ^ "/")
                    || starts_with root (identity ^ "/"))
                  identities
              then fail Diff_path_invalid
              else (
                Hashtbl.add table spec.name spec;
                loop (identity :: identities) rest)
  in
  loop [] packages

let validate_match_work packages changed_paths =
  let remaining = ref max_match_work in
  let rec packages_loop = function
    | [] -> Ok ()
    | spec :: rest when spec.source_mode = Package_prefix -> packages_loop rest
    | spec :: rest ->
        let pattern_factor =
          List.fold_left
            (fun total pattern ->
              match scalar_count pattern with
              | None -> total
              | Some count -> Int64.add total (Int64.of_int (count + 1)))
            0L spec.source_globs
        in
        let rec paths_loop = function
          | [] -> packages_loop rest
          | path :: paths when not (inside path spec.rel_path) -> paths_loop paths
          | path :: paths ->
              let relative_path = relative path spec.rel_path in
              if String_set.mem (basename relative_path) build_names then
                paths_loop paths
              else
                let path_factor =
                  match scalar_count relative_path with
                  | None -> 0L
                  | Some count -> Int64.of_int (count + 1)
                in
                if
                  pattern_factor <> 0L
                  && Int64.compare path_factor (Int64.div !remaining pattern_factor)
                     > 0
                then fail Diff_match_limit_exceeded
                else (
                  remaining :=
                    Int64.sub !remaining (Int64.mul pattern_factor path_factor);
                  paths_loop paths)
        in
        paths_loop changed_paths
  in
  packages_loop packages

let evaluate_diff_selection input =
  let* graph = validate_graph (List.map (fun spec -> spec.name) input.packages) input.edges in
  if Directed.has_cycle graph.forward then fail Diff_edge_cycle
  else
    let* package_table = validate_packages input.packages in
    if
      List.length input.forced_packages > max_packages
      || not (unique input.forced_packages)
    then fail Diff_forced_package_invalid
    else if
      List.length input.changed_paths > max_packages
      || not (unique input.changed_paths)
      || not (List.for_all is_portable_path input.changed_paths)
    then fail Diff_path_invalid
    else if
      not
        (List.for_all
           (fun name -> Hashtbl.mem package_table name)
           input.forced_packages)
    then fail Diff_forced_package_unknown
    else
      let consumers = Hashtbl.create 32 in
      let* () =
        match (input.boundary_sha256, input.boundary) with
        | None, None -> Ok ()
        | Some digest, Some boundary
          when lowercase_hex_digest digest
               && String.equal digest (repository_boundary_digest boundary) ->
            List.iter
              (fun spec ->
                List.iter
                  (fun rule ->
                    if applies_to rule.applies_to spec.rel_path then
                      List.iter
                        (fun boundary_input ->
                          add_consumer consumers boundary_input.path spec.name)
                        rule.inputs)
                  boundary.boundaries)
              input.packages;
            Ok ()
        | _ -> fail Diff_boundary_digest_mismatch
      in
      let* () = validate_match_work input.packages input.changed_paths in
      let changed =
        ref
          (List.fold_left
             (fun values name -> String_set.add name values)
             String_set.empty input.forced_packages)
      in
      let unknown = ref false in
      List.iter
        (fun path ->
          let boundary_consumers =
            Option.value ~default:String_set.empty (Hashtbl.find_opt consumers path)
          in
          changed := String_set.union !changed boundary_consumers;
          let known = ref (not (String_set.is_empty boundary_consumers)) in
          List.iter
            (fun spec ->
              if inside path spec.rel_path then (
                known := true;
                let relative_path = relative path spec.rel_path in
                if
                  spec.source_mode = Package_prefix
                  || String_set.mem (basename relative_path) build_names
                  || List.exists
                       (fun pattern -> glob_matches pattern relative_path)
                       spec.source_globs
                then changed := String_set.add spec.name !changed))
            input.packages;
          if not !known then unknown := true)
        input.changed_paths;
      if !unknown && input.unknown_path_policy = Error then fail Diff_unknown_path
      else (
        if !unknown then
          List.iter
            (fun spec -> changed := String_set.add spec.name !changed)
            input.packages;
        let affected = closure graph.forward !changed in
        let prerequisites = String_set.diff (closure graph.reverse affected) affected in
        Ok
          { changed_packages = sort_strings (String_set.elements !changed);
            affected_packages = sort_strings (String_set.elements affected);
            prerequisite_packages =
              sort_strings (String_set.elements prerequisites) })
