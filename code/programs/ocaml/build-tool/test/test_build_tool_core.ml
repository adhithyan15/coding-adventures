(** Native fixture consumption for the bounded OCaml build-tool core.

    The adapter in this file deliberately owns fixture discovery and JSON
    decoding. The production library receives only typed immutable values. *)

open Coding_adventures_build_tool
open Yojson.Safe.Util
module String_set = Set.Make (String)

let expected_cases =
  String_set.of_list
    [
      "diff-selection/exact-build-fronts";
      "diff-selection/forced-package";
      "diff-selection/known-unmatched-near-build";
      "diff-selection/match-work-at-limit";
      "diff-selection/match-work-over-limit";
      "diff-selection/package-prefix";
      "diff-selection/repository-boundary-reverse-index";
      "diff-selection/strict-glob-character-classes";
      "diff-selection/transitive-package-change";
      "diff-selection/unknown-path-all";
      "diff-selection/unknown-path-error";
      "graph/canonical-edge-order";
      "graph/chain";
      "graph/cycle";
      "graph/diamond";
      "graph/empty";
      "graph/isolated";
      "graph/multiple-components";
      "graph/partial-cycle-no-output";
    ]

let rec repository_root current =
  let fixture_root =
    Filename.concat current "code/specs/fixtures/build-tool-v1"
  in
  if Sys.file_exists fixture_root then current
  else
    let parent = Filename.dirname current in
    if String.equal parent current then failwith "repository root not found"
    else repository_root parent

let fixture_root () =
  Filename.concat
    (repository_root (Sys.getcwd ()))
    "code/specs/fixtures/build-tool-v1"

let max_fixture_bytes = 2_000_000
let max_json_depth = 64

let read_json_file path =
  (match (Unix.lstat path).st_kind with
  | Unix.S_REG -> ()
  | _ -> failwith ("fixture is not a regular in-tree file: " ^ path));
  let channel = open_in_bin path in
  Fun.protect
    ~finally:(fun () -> close_in_noerr channel)
    (fun () ->
      let length = in_channel_length channel in
      if length > max_fixture_bytes then
        failwith ("fixture exceeds byte limit: " ^ path);
      let json = Yojson.Safe.from_string (really_input_string channel length) in
      let rec check_depth depth = function
        | `Assoc fields ->
            if depth > max_json_depth then failwith "fixture JSON is too deep";
            List.iter (fun (_, value) -> check_depth (depth + 1) value) fields
        | `List values | `Tuple values ->
            if depth > max_json_depth then failwith "fixture JSON is too deep";
            List.iter (check_depth (depth + 1)) values
        | `Variant (_, Some value) -> check_depth (depth + 1) value
        | `Null | `Bool _ | `Int _ | `Intlit _ | `Float _ | `String _
        | `Variant (_, None) ->
            ()
      in
      check_depth 1 json;
      json)

let strings = function
  | `Null -> []
  | json -> json |> to_list |> List.map to_string

let edge json =
  match strings json with
  | [ prerequisite; dependent ] -> { prerequisite; dependent }
  | _ -> failwith "fixture edge must contain two package names"

let edges json = json |> to_list |> List.map edge

let package_spec json =
  let source_mode =
    match json |> member "source_mode" |> to_string with
    | "package_prefix" -> Package_prefix
    | "strict_globs" -> Strict_globs
    | value -> failwith ("unsupported source mode: " ^ value)
  in
  {
    name = json |> member "name" |> to_string;
    rel_path = json |> member "rel_path" |> to_string;
    source_mode;
    source_globs = json |> member "source_globs" |> strings;
  }

let applies_to json =
  {
    exact_roots = json |> member "exact_roots" |> strings;
    descendant_roots = json |> member "descendant_roots" |> strings;
    excluded_roots = json |> member "excluded_roots" |> strings;
  }

let boundary_input json =
  {
    path = json |> member "path" |> to_string;
    role = json |> member "role" |> to_string;
    generated_component =
      json |> member "generated_component" |> to_string_option;
  }

let boundary_rule json =
  {
    id = json |> member "id" |> to_string;
    input_origin = json |> member "input_origin" |> to_string;
    applies_to = json |> member "applies_to" |> applies_to;
    inputs = json |> member "inputs" |> to_list |> List.map boundary_input;
    reason = json |> member "reason" |> to_string;
    owner = json |> member "owner" |> to_string;
  }

let repository_boundary json =
  {
    schema_version = json |> member "schema_version" |> to_int;
    language_source_input_registry_sha256 =
      json |> member "language_source_input_registry_sha256" |> to_string;
    boundaries =
      json |> member "boundaries" |> to_list |> List.map boundary_rule;
  }

let diagnostic_code expected =
  expected |> member "diagnostics" |> to_list |> List.hd |> member "code"
  |> to_string

let check_edges message expected actual =
  let pairs values =
    List.map (fun edge -> (edge.prerequisite, edge.dependent)) values
  in
  Alcotest.(check (list (pair string string)))
    message (pairs expected) (pairs actual)

let check_levels message expected actual =
  Alcotest.(check (list (list string))) message expected actual

let check_graph fixture =
  let input = fixture |> member "input" |> member "options" in
  let expected = fixture |> member "expected" in
  let actual =
    evaluate_graph
      {
        packages = input |> member "packages" |> strings;
        edges = input |> member "edges" |> edges;
      }
  in
  match expected |> member "outcome" |> to_string with
  | "error" ->
      let error = Result.get_error actual in
      Alcotest.(check string)
        "graph error" (diagnostic_code expected) (error_code error)
  | "ok" ->
      let actual = Result.get_ok actual in
      let result = expected |> member "result" in
      check_edges "canonical edges"
        (result |> member "edges" |> edges)
        actual.edges;
      check_levels "deterministic levels"
        (result |> member "levels" |> to_list |> List.map strings)
        actual.levels
  | value -> failwith ("unsupported graph outcome: " ^ value)

let check_diff fixture boundary =
  let input = fixture |> member "input" in
  let options = input |> member "options" in
  let digest = options |> member "boundary_sha256" |> to_string_option in
  let expected = fixture |> member "expected" in
  let actual =
    evaluate_diff_selection
      {
        packages =
          options |> member "packages" |> to_list |> List.map package_spec;
        edges = options |> member "edges" |> edges;
        forced_packages = options |> member "forced_packages" |> strings;
        unknown_path_policy =
          (match options |> member "unknown_path_policy" |> to_string with
          | "all" -> All
          | "error" -> Error
          | value -> failwith ("unsupported unknown-path policy: " ^ value));
        changed_paths = input |> member "changed_paths" |> strings;
        boundary_sha256 = digest;
        boundary = Option.map (fun value -> value) boundary;
      }
  in
  match expected |> member "outcome" |> to_string with
  | "error" ->
      let error = Result.get_error actual in
      Alcotest.(check string)
        "diff error" (diagnostic_code expected) (error_code error)
  | "ok" ->
      let actual = Result.get_ok actual in
      let result = expected |> member "result" in
      Alcotest.(check (list string))
        "changed packages"
        (result |> member "changed_packages" |> strings)
        actual.changed_packages;
      Alcotest.(check (list string))
        "affected packages"
        (result |> member "affected_packages" |> strings)
        actual.affected_packages;
      Alcotest.(check (list string))
        "prerequisite packages"
        (result |> member "prerequisite_packages" |> strings)
        actual.prerequisite_packages
  | value -> failwith ("unsupported diff outcome: " ^ value)

let consumes_every_shared_fixture () =
  let root = fixture_root () in
  let boundary =
    read_json_file
      (Filename.concat root "repository-source-input-boundary.json")
    |> repository_boundary
  in
  let cases = Filename.concat root "cases" in
  let seen = ref String_set.empty in
  Sys.readdir cases |> Array.to_list |> List.sort String.compare
  |> List.iter (fun name ->
         if Filename.check_suffix name ".json" then
           let fixture = read_json_file (Filename.concat cases name) in
           match fixture |> member "domain" |> to_string with
           | "graph" ->
               seen :=
                 String_set.add (fixture |> member "id" |> to_string) !seen;
               check_graph fixture
           | "diff_selection" ->
               seen :=
                 String_set.add (fixture |> member "id" |> to_string) !seen;
               let digest =
                 fixture |> member "input" |> member "options"
                 |> member "boundary_sha256" |> to_string_option
               in
               check_diff fixture (Option.map (fun _ -> boundary) digest)
           | _ -> ());
  Alcotest.(check (list string))
    "exact shared case roster"
    (String_set.elements expected_cases)
    (String_set.elements !seen)

let error_code = function
  | Result.Error error -> Coding_adventures_build_tool.error_code error
  | Ok _ -> Alcotest.fail "expected an error"

let malformed_inputs_fail_closed () =
  let graph_error =
    evaluate_graph
      {
        packages = [ "fixture/a" ];
        edges =
          [ { prerequisite = "fixture/a"; dependent = "fixture/missing" } ];
      }
    |> error_code
  in
  Alcotest.(check string)
    "unknown graph endpoint" "GRAPH_EDGE_UNKNOWN" graph_error;
  let cycle_error =
    evaluate_diff_selection
      {
        packages =
          [
            {
              name = "fixture/a";
              rel_path = "a";
              source_mode = Package_prefix;
              source_globs = [];
            };
            {
              name = "fixture/b";
              rel_path = "b";
              source_mode = Package_prefix;
              source_globs = [];
            };
          ];
        edges =
          [
            { prerequisite = "fixture/a"; dependent = "fixture/b" };
            { prerequisite = "fixture/b"; dependent = "fixture/a" };
          ];
        forced_packages = [];
        unknown_path_policy = Error;
        changed_paths = [ "a/file" ];
        boundary_sha256 = None;
        boundary = None;
      }
    |> error_code
  in
  Alcotest.(check string) "cyclic diff graph" "DIFF_EDGE_CYCLE" cycle_error;
  let collision_error =
    evaluate_diff_selection
      {
        packages =
          [
            {
              name = "fixture/a";
              rel_path = "straße";
              source_mode = Package_prefix;
              source_globs = [];
            };
            {
              name = "fixture/b";
              rel_path = "strasse";
              source_mode = Package_prefix;
              source_globs = [];
            };
          ];
        edges = [];
        forced_packages = [];
        unknown_path_policy = Error;
        changed_paths = [ "strasse/file" ];
        boundary_sha256 = None;
        boundary = None;
      }
    |> error_code
  in
  Alcotest.(check string)
    "portable identity collision" "DIFF_PATH_INVALID" collision_error;
  let glob_error =
    evaluate_diff_selection
      {
        packages =
          [
            {
              name = "fixture/a";
              rel_path = "a";
              source_mode = Strict_globs;
              source_globs = [ "[z-a]" ];
            };
          ];
        edges = [];
        forced_packages = [];
        unknown_path_policy = Error;
        changed_paths = [ "a/z" ];
        boundary_sha256 = None;
        boundary = None;
      }
    |> error_code
  in
  Alcotest.(check string) "descending class" "DIFF_GLOB_INVALID" glob_error

let edge_cases () =
  let empty = evaluate_graph { packages = []; edges = [] } |> Result.get_ok in
  check_edges "empty graph edges" [] empty.edges;
  check_levels "empty graph levels" [] empty.levels;
  let partial_cycle =
    evaluate_graph
      {
        packages = [ "fixture/free"; "fixture/a"; "fixture/b" ];
        edges =
          [
            { prerequisite = "fixture/a"; dependent = "fixture/b" };
            { prerequisite = "fixture/b"; dependent = "fixture/a" };
          ];
      }
  in
  Alcotest.(check string)
    "partial cycle has no output" "GRAPH_CYCLE" (error_code partial_cycle);
  let spec =
    {
      name = "fixture/p";
      rel_path = "p";
      source_mode = Strict_globs;
      source_globs = [ "src/**/[a-c]*.txt"; "literal["; "emoji/[😀].txt" ];
    }
  in
  let run paths =
    match
      evaluate_diff_selection
        {
          packages = [ spec ];
          edges = [];
          forced_packages = [];
          unknown_path_policy = Error;
          changed_paths = paths;
          boundary_sha256 = None;
          boundary = None;
        }
    with
    | Ok result -> result
    | Result.Error error ->
        Alcotest.failf "unexpected diff error: %s"
          (Coding_adventures_build_tool.error_code error)
  in
  Alcotest.(check (list string))
    "strict portable glob" [ "fixture/p" ]
    (run [ "p/src/deep/bee.txt"; "p/BUILD_debug"; "p/BUILD" ]).changed_packages;
  Alcotest.(check (list string))
    "near BUILD path is not selected" []
    (run [ "p/src/deep/x.bin"; "p/BUILD_debug" ]).changed_packages;
  Alcotest.(check (list string))
    "unclosed class is literal" [ "fixture/p" ]
    (run [ "p/literal[" ]).changed_packages;
  Alcotest.(check (list string))
    "Unicode class" [ "fixture/p" ] (run [ "p/emoji/😀.txt" ]).changed_packages;
  let question_error =
    evaluate_diff_selection
      {
        packages = [ { spec with source_globs = [ "question/?.txt" ] } ];
        edges = [];
        forced_packages = [];
        unknown_path_policy = Error;
        changed_paths = [ "p/question/x.txt" ];
        boundary_sha256 = None;
        boundary = None;
      }
    |> error_code
  in
  Alcotest.(check string)
    "question-mark glob is forbidden" "DIFF_GLOB_INVALID" question_error

let check_error expected result =
  Alcotest.(check string) expected expected (error_code result)

let graph_validation_errors () =
  check_error "GRAPH_PACKAGE_LIMIT_EXCEEDED"
    (evaluate_graph
       {
         packages =
           List.init 4_097 (fun index -> Printf.sprintf "fixture/p%d" index);
         edges = [];
       });
  check_error "GRAPH_EDGE_LIMIT_EXCEEDED"
    (evaluate_graph
       {
         packages = [ "fixture/a"; "fixture/b" ];
         edges =
           List.init 16_385 (fun _ ->
               { prerequisite = "fixture/a"; dependent = "fixture/b" });
       });
  check_error "GRAPH_PACKAGE_INVALID"
    (evaluate_graph { packages = [ "invalid" ]; edges = [] });
  check_error "GRAPH_PACKAGE_INVALID"
    (evaluate_graph { packages = [ "" ]; edges = [] });
  check_error "GRAPH_PACKAGE_INVALID"
    (evaluate_graph
       { packages = [ "fixture/" ^ String.make 1 (Char.chr 0xff) ]; edges = [] });
  check_error "GRAPH_PACKAGE_INVALID"
    (evaluate_graph
       { packages = [ "fixture/" ^ String.make 1_000_000 'a' ]; edges = [] });
  check_error "GRAPH_PACKAGE_DUPLICATE"
    (evaluate_graph { packages = [ "fixture/a"; "fixture/a" ]; edges = [] });
  check_error "GRAPH_EDGE_SELF"
    (evaluate_graph
       {
         packages = [ "fixture/a" ];
         edges = [ { prerequisite = "fixture/a"; dependent = "fixture/a" } ];
       });
  check_error "GRAPH_EDGE_UNKNOWN"
    (evaluate_graph
       {
         packages = [ "fixture/a" ];
         edges =
           [ { prerequisite = "fixture/missing"; dependent = "fixture/a" } ];
       });
  check_error "GRAPH_EDGE_DUPLICATE"
    (evaluate_graph
       {
         packages = [ "fixture/a"; "fixture/b" ];
         edges =
           [
             { prerequisite = "fixture/a"; dependent = "fixture/b" };
             { prerequisite = "fixture/a"; dependent = "fixture/b" };
           ];
       })

let base_spec =
  {
    name = "fixture/a";
    rel_path = "a";
    source_mode = Strict_globs;
    source_globs = [ "src/**" ];
  }

let diff_input ?(packages = [ base_spec ]) ?(forced_packages = [])
    ?(changed_paths = [ "a/src/file.ml" ]) ?boundary_sha256 ?boundary () =
  {
    packages;
    edges = [];
    forced_packages;
    unknown_path_policy = Error;
    changed_paths;
    boundary_sha256;
    boundary;
  }

let diff_validation_errors () =
  check_error "DIFF_PACKAGE_INVALID"
    (evaluate_diff_selection
       (diff_input ~packages:[ { base_spec with name = "invalid" } ] ()));
  check_error "DIFF_PACKAGE_DUPLICATE"
    (evaluate_diff_selection (diff_input ~packages:[ base_spec; base_spec ] ()));
  check_error "DIFF_PATH_INVALID"
    (evaluate_diff_selection
       (diff_input
          ~packages:[ { base_spec with rel_path = "CON" } ]
          ~changed_paths:[ "CON/file" ] ()));
  check_error "DIFF_PATH_INVALID"
    (evaluate_diff_selection
       (diff_input
          ~packages:[ { base_spec with rel_path = "e\u{0301}" } ]
          ~changed_paths:[ "e\u{0301}/file" ] ()));
  check_error "DIFF_PATH_INVALID"
    (evaluate_diff_selection
       (diff_input ~changed_paths:[ String.make 1 (Char.chr 0xff) ] ()));
  check_error "DIFF_PATH_INVALID"
    (evaluate_diff_selection (diff_input ~changed_paths:[ "a/control\001" ] ()));
  check_error "DIFF_PATH_INVALID"
    (evaluate_diff_selection
       (diff_input
          ~packages:
            [
              base_spec;
              { base_spec with name = "fixture/b"; rel_path = "a/nested" };
            ]
          ()));
  List.iter
    (fun glob ->
      check_error "DIFF_GLOB_INVALID"
        (evaluate_diff_selection
           (diff_input
              ~packages:[ { base_spec with source_globs = [ glob ] } ]
              ())))
    [ "[z-a]"; "[a--b]"; "[a&&b]"; "[a~~b]"; "[a||b]" ];
  check_error "DIFF_GLOB_INVALID"
    (evaluate_diff_selection
       (diff_input
          ~packages:
            [
              {
                base_spec with
                source_globs = [ String.make 1 (Char.chr 0xff) ];
              };
            ]
          ()));
  check_error "DIFF_GLOB_INVALID"
    (evaluate_diff_selection
       (diff_input
          ~packages:[ { base_spec with source_globs = [ "src/**"; "src/**" ] } ]
          ()));
  check_error "DIFF_GLOB_INVALID"
    (evaluate_diff_selection
       (diff_input
          ~packages:
            [
              {
                base_spec with
                source_globs =
                  List.init 257 (fun index -> Printf.sprintf "p%d" index);
              };
            ]
          ()));
  check_error "DIFF_GLOB_INVALID"
    (evaluate_diff_selection
       (diff_input
          ~packages:
            [
              {
                base_spec with
                source_mode = Package_prefix;
                source_globs = [ "src/**" ];
              };
            ]
          ()));
  check_error "DIFF_FORCED_PACKAGE_INVALID"
    (evaluate_diff_selection
       (diff_input ~forced_packages:[ "fixture/a"; "fixture/a" ] ()));
  check_error "DIFF_FORCED_PACKAGE_INVALID"
    (evaluate_diff_selection
       (diff_input ~forced_packages:(List.init 4_097 (fun _ -> "fixture/a")) ()));
  check_error "DIFF_FORCED_PACKAGE_UNKNOWN"
    (evaluate_diff_selection
       (diff_input ~forced_packages:[ "fixture/missing" ] ()));
  check_error "DIFF_PATH_INVALID"
    (evaluate_diff_selection
       (diff_input ~changed_paths:[ "a/src/file.ml"; "a/src/file.ml" ] ()));
  check_error "DIFF_PATH_INVALID"
    (evaluate_diff_selection
       (diff_input
          ~changed_paths:
            (List.init 4_097 (fun index -> Printf.sprintf "a/p%d" index))
          ()));
  check_error "DIFF_BOUNDARY_DIGEST_MISMATCH"
    (evaluate_diff_selection
       (diff_input ~boundary_sha256:(String.make 64 '0') ()))

let boundary_and_digest () =
  let escaped =
    {
      schema_version = 1;
      language_source_input_registry_sha256 = String.make 64 '0';
      boundaries =
        [
          {
            id = "escaped";
            input_origin = "repository";
            applies_to =
              {
                exact_roots = [ "code/programs/ocaml/build-tool" ];
                descendant_roots = [];
                excluded_roots = [];
              };
            inputs =
              [
                {
                  path = "code/specs/shared-input.json";
                  role = "cross_package_exact";
                  generated_component = None;
                };
              ];
            reason =
              "back\bform\012line\nreturn\rtab\tcontrol\001quote\"slash\\";
            owner = "fixture";
          };
        ];
    }
  in
  Alcotest.(check string)
    "canonical JSON escaping"
    "4e68e52c1e17775785ad231d1be4157493d8768cada5063918dffd77a56451c9"
    (repository_boundary_digest escaped |> Result.get_ok);
  let boundary =
    {
      schema_version = 1;
      language_source_input_registry_sha256 = String.make 64 '0';
      boundaries =
        [
          {
            id = "shared";
            input_origin = "repository";
            applies_to =
              {
                exact_roots = [ "code/programs/ocaml/build-tool" ];
                descendant_roots = [];
                excluded_roots = [];
              };
            inputs =
              [
                {
                  path = "code/specs/shared-input.json";
                  role = "cross_package_exact";
                  generated_component = None;
                };
              ];
            reason = "test";
            owner = "fixture";
          };
        ];
    }
  in
  let digest =
    "cfb9c08af193487722b4aec4feda38eb0ec06cdc4f002249d81f6316e44303a4"
  in
  let result =
    evaluate_diff_selection
      (diff_input
         ~packages:
           [
             {
               name = "ocaml/build-tool";
               rel_path = "code/programs/ocaml/build-tool";
               source_mode = Package_prefix;
               source_globs = [];
             };
           ]
         ~changed_paths:[ "code/specs/shared-input.json" ]
         ~boundary_sha256:digest ~boundary ())
    |> Result.get_ok
  in
  Alcotest.(check (list string))
    "descendant boundary consumer" [ "ocaml/build-tool" ]
    result.changed_packages

let boundary_budget_guard () =
  let rule =
    {
      id = "bounded";
      input_origin = "repository";
      applies_to =
        {
          exact_roots = [ "code/programs/ocaml/build-tool" ];
          descendant_roots = [];
          excluded_roots = [];
        };
      inputs =
        [
          {
            path = "code/specs/shared-input.json";
            role = "cross_package_exact";
            generated_component = None;
          };
        ];
      reason = "test";
      owner = "fixture";
    }
  in
  let boundary =
    {
      schema_version = 1;
      language_source_input_registry_sha256 = String.make 64 '0';
      boundaries = List.init 257 (fun _ -> rule);
    }
  in
  check_error "DIFF_BOUNDARY_DIGEST_MISMATCH"
    (evaluate_diff_selection
       (diff_input ~boundary_sha256:(String.make 64 '0') ~boundary ()))

let () =
  Alcotest.run "OCaml build-tool graph and diff core"
    [
      ( "shared fixtures",
        [
          Alcotest.test_case "all 19 cases" `Quick consumes_every_shared_fixture;
        ] );
      ( "fail closed",
        [
          Alcotest.test_case "malformed typed input" `Quick
            malformed_inputs_fail_closed;
          Alcotest.test_case "graph validation codes" `Quick
            graph_validation_errors;
          Alcotest.test_case "diff validation codes" `Quick
            diff_validation_errors;
        ] );
      ( "focused edges",
        [
          Alcotest.test_case "cycle, glob, and Unicode behavior" `Quick
            edge_cases;
          Alcotest.test_case "boundary applicability and digest" `Quick
            boundary_and_digest;
          Alcotest.test_case "boundary work budgets" `Quick
            boundary_budget_guard;
        ] );
    ]
